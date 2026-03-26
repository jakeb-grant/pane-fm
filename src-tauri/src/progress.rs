use crate::error::AppError;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

static CANCEL_FLAG: AtomicBool = AtomicBool::new(false);
static LAST_EMIT_MS: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Serialize)]
pub struct ProgressPayload {
    pub processed: u64,
    pub total: u64,
}

pub fn reset() {
    CANCEL_FLAG.store(false, Ordering::Relaxed);
    LAST_EMIT_MS.store(0, Ordering::Relaxed);
}

pub fn cancel() {
    CANCEL_FLAG.store(true, Ordering::Relaxed);
}

pub fn is_cancelled() -> bool {
    CANCEL_FLAG.load(Ordering::Relaxed)
}

/// Check cancellation, returning `std::io::Error` (for use inside `Read`/`Write` impls).
pub fn check_cancelled() -> Result<(), std::io::Error> {
    if is_cancelled() {
        Err(std::io::Error::other("Cancelled"))
    } else {
        Ok(())
    }
}

/// Check cancellation, returning `AppError::Cancelled` directly.
pub fn check_cancelled_err() -> Result<(), AppError> {
    if is_cancelled() {
        Err(AppError::Cancelled)
    } else {
        Ok(())
    }
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Emit progress, throttled to at most once per 50ms (always emits the final event).
pub fn emit(app: &AppHandle, processed: u64, total: u64) {
    let now = now_ms();
    let prev = LAST_EMIT_MS.load(Ordering::Relaxed);
    if processed < total && now.saturating_sub(prev) < 50 {
        return;
    }
    LAST_EMIT_MS.store(now, Ordering::Relaxed);
    let _ = app.emit(
        "operation-progress",
        ProgressPayload { processed, total },
    );
}
