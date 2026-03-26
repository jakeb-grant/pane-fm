use crate::error::AppError;
use crate::progress;
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};

static SEARCH_CANCEL: AtomicBool = AtomicBool::new(false);

const MAX_RESULTS: usize = 500;
const BATCH_SIZE: usize = 20;
const EMIT_THROTTLE_MS: u64 = 50;

#[derive(Debug, Serialize, Clone)]
pub struct SearchResult {
    pub name: String,
    pub path: String,
    pub relative_path: String,
    pub is_dir: bool,
    pub is_symlink: bool,
}

#[derive(Clone, Serialize)]
struct SearchBatch {
    results: Vec<SearchResult>,
    done: bool,
    gen: u64,
}

fn is_glob(query: &str) -> bool {
    query.bytes().any(|b| b == b'*' || b == b'?' || b == b'[')
}

fn glob_matches(pattern: &[u8], text: &[u8]) -> bool {
    let (mut pi, mut ti) = (0, 0);
    let (mut star_p, mut star_t) = (usize::MAX, 0);

    while ti < text.len() {
        if pi < pattern.len()
            && (pattern[pi] == b'?' || pattern[pi].eq_ignore_ascii_case(&text[ti]))
        {
            pi += 1;
            ti += 1;
        } else if pi < pattern.len() && pattern[pi] == b'[' {
            if let Some(end) = pattern[pi..].iter().position(|&b| b == b']') {
                let class = &pattern[pi + 1..pi + end];
                let matched = class.iter().any(|b| b.eq_ignore_ascii_case(&text[ti]));
                if matched {
                    pi += end + 1;
                    ti += 1;
                } else if star_p != usize::MAX {
                    pi = star_p + 1;
                    star_t += 1;
                    ti = star_t;
                } else {
                    return false;
                }
            } else if pattern[pi].eq_ignore_ascii_case(&text[ti]) {
                pi += 1;
                ti += 1;
            } else if star_p != usize::MAX {
                pi = star_p + 1;
                star_t += 1;
                ti = star_t;
            } else {
                return false;
            }
        } else if pi < pattern.len() && pattern[pi] == b'*' {
            star_p = pi;
            star_t = ti;
            pi += 1;
        } else if star_p != usize::MAX {
            pi = star_p + 1;
            star_t += 1;
            ti = star_t;
        } else {
            return false;
        }
    }

    while pi < pattern.len() && pattern[pi] == b'*' {
        pi += 1;
    }

    pi == pattern.len()
}

struct SearchState {
    results: Vec<SearchResult>,
    count: usize,
    last_emit_ms: u64,
    gen: u64,
}

impl SearchState {
    fn new(gen: u64) -> Self {
        Self {
            results: Vec::new(),
            count: 0,
            last_emit_ms: 0,
            gen,
        }
    }

    fn is_stale(&self) -> bool {
        SEARCH_CANCEL.load(Ordering::Relaxed)
    }

    fn flush(&mut self, app: &AppHandle, done: bool) {
        if self.is_stale() {
            return;
        }
        if self.results.is_empty() && !done {
            return;
        }
        let _ = app.emit(
            "search-results",
            SearchBatch {
                results: std::mem::take(&mut self.results),
                done,
                gen: self.gen,
            },
        );
        self.last_emit_ms = progress::now_ms();
    }

    fn maybe_flush(&mut self, app: &AppHandle) {
        let should_flush = self.results.len() >= BATCH_SIZE
            || (progress::now_ms().saturating_sub(self.last_emit_ms) >= EMIT_THROTTLE_MS
                && !self.results.is_empty());
        if should_flush {
            self.flush(app, false);
        }
    }

    fn push(&mut self, result: SearchResult, app: &AppHandle) -> bool {
        self.count += 1;
        self.results.push(result);
        self.maybe_flush(app);
        self.count >= MAX_RESULTS
    }
}

fn walk_search(
    dir: &Path,
    base: &Path,
    query: &str,
    glob_mode: bool,
    show_hidden: bool,
    state: &mut SearchState,
    app: &AppHandle,
) -> Result<(), AppError> {
    if state.is_stale() {
        return Err(AppError::Cancelled);
    }

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    for entry in entries {
        if state.is_stale() {
            return Err(AppError::Cancelled);
        }

        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };

        if !ft.is_file() && !ft.is_dir() && !ft.is_symlink() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') && !show_hidden {
            continue;
        }

        let path_buf = entry.path();

        let matched = if glob_mode {
            glob_matches(query.as_bytes(), name.as_bytes())
        } else {
            name.to_ascii_lowercase().contains(query)
        };

        if matched {
            let relative = path_buf
                .strip_prefix(base)
                .unwrap_or(&path_buf)
                .to_string_lossy()
                .to_string();

            let hit_limit = state.push(
                SearchResult {
                    name,
                    path: path_buf.to_string_lossy().to_string(),
                    relative_path: relative,
                    is_dir: ft.is_dir() || (ft.is_symlink() && path_buf.is_dir()),
                    is_symlink: ft.is_symlink(),
                },
                app,
            );

            if hit_limit {
                return Ok(());
            }
        }

        let is_dir = ft.is_dir() || (ft.is_symlink() && path_buf.is_dir());
        if is_dir {
            walk_search(&path_buf, base, query, glob_mode, show_hidden, state, app)?;
            if state.count >= MAX_RESULTS {
                return Ok(());
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn search_files(
    app: AppHandle,
    root: String,
    query: String,
    show_hidden: bool,
    gen: u64,
) -> Result<(), AppError> {
    SEARCH_CANCEL.store(false, Ordering::Relaxed);

    if query.is_empty() {
        let _ = app.emit(
            "search-results",
            SearchBatch {
                results: vec![],
                done: true,
                gen,
            },
        );
        return Ok(());
    }

    let root_path = std::path::PathBuf::from(&root);
    let glob_mode = is_glob(&query);
    let query_lower = query.to_ascii_lowercase();

    tokio::task::spawn_blocking(move || {
        let mut state = SearchState::new(gen);
        let _ = walk_search(
            &root_path,
            &root_path,
            &query_lower,
            glob_mode,
            show_hidden,
            &mut state,
            &app,
        );
        state.flush(&app, true);
    })
    .await
    .map_err(|e| AppError::Io {
        message: format!("Search task failed: {e}"),
        path: None,
    })?;

    Ok(())
}

#[tauri::command]
pub fn cancel_search() {
    SEARCH_CANCEL.store(true, Ordering::Relaxed);
}
