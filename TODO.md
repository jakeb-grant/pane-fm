# Preview & Caching Performance TODO

Optimizations identified from comparative analysis with yazi's caching system.

## Done

- [x] JPEG thumbnails instead of PNG (5-10x smaller cache files)
- [x] Image dimension/memory guards (reject >10000x10000 or >512MB decoded)
- [x] Fix preview cache size estimates (images/PDFs tracked at 500KB, not URL string length)
- [x] Raise preview cache limit from 5MB to 20MB (matches corrected estimates)
- [x] Cap directory cache at 200 entries with LRU eviction
- [x] Tree-shake highlight.js (44 languages instead of 190, ~70% smaller worker bundle)

## High Impact

- [x] **Cancellation via generation counter** — `AtomicU64` preview generation counter synced from JS. Rust checks staleness before `image::open()` and before JPEG encode. Stale thumbnail/PDF operations return `Cancelled` early.

- [x] **Skip decode when image fits within max_dim** — Header-only dimension check: if `max(w, h) <= max_dim`, return original file path directly without any decode/resize/cache.

- [x] **Streaming directory reads** — New `stream_directory` command emits batches (500 entries / 100ms) via Tauri events. Frontend accumulates unsorted during streaming, sorts on completion. Cancellable via `AtomicBool` when navigating away. Existing `list_directory` kept for prefetch.

## Medium Impact

- [x] **EXIF orientation** — Uses `ImageReader` + `ImageDecoder::orientation()` to read EXIF metadata, applies flip/rotate transforms before resizing. Phone photos now display correctly.

- [x] **Configurable limits** — New `[preview]` section in `config.toml`: `image_quality` (50-90, default 75), `max_dimension` (default 10000), `max_alloc_mb` (default 512).

- [ ] **Priority queues for preloading** — Current prefetch treats adjacent entries equally. Yazi uses LOW/NORMAL/HIGH priority so the currently-viewed file always takes precedence over prefetch work.

## Low Impact

- [ ] **Rust-native syntax highlighting (syntect)** — Would enable per-line cancellation and eliminate the JS worker overhead. Major effort, low urgency since tree-shaking already cut the bundle significantly.

- [ ] **Atomic cache writes** — Write to temp file then rename, preventing corrupt cache files on crash. Current `File::create` + write can leave partial files.
