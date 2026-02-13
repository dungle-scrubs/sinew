## Review Issues

### Critical

- Bar updates call every module's update synchronously on the UI thread
  every 500ms. Several modules run blocking shell commands inside update,
  which can stall rendering and make popups feel laggy. Examples include
  DiskModule running `df` in update. Files: `src/gpui_app/bar.rs:265`,
  `src/gpui_app/modules/disk.rs:35`.

### High

- Multiple modules spawn background threads with infinite loops and no
  shutdown path. On config reload or module rebuild, those threads can
  leak and multiply, degrading performance over time. Examples include
  CpuModule and TemperatureModule. Files: `src/gpui_app/modules/cpu.rs:31`,
  `src/gpui_app/modules/temperature.rs:31`.
- PopupHostView enforces a fixed height via `min_h` + `h` even when
  content is smaller. This guarantees extra empty space and can force
  scrolling when it is not needed. File:
  `src/gpui_app/modules/popup_host.rs:331`.
- Popup tracing writes to `/tmp` from render/update paths when
  `SINEW_TRACE_POPUP` is set, which can block the UI thread. Examples:
  `src/gpui_app/modules/popup_host.rs:160` and `src/gpui_app/bar.rs:274`.

### Medium

- NewsModule spawns a new thread on every refresh without guarding
  in-flight requests. Concurrent refreshes can race to update the same
  data, increasing CPU and network load. File:
  `src/gpui_app/modules/news.rs:182`.
- ApiUsageModule runs multiple `sh -c` commands without timeouts. If a
  command blocks (network stall or auth prompt), the worker thread can
  hang indefinitely. File: `src/gpui_app/modules/api_usage.rs:101`.
- The global module registry is `RwLock<Option<ModuleRegistry>>` with
  per-module `RwLock`. This increases lock contention on render, which is
  especially noticeable when popup rendering also reads module state.
  Files: `src/gpui_app/modules/mod.rs:513`,
  `src/gpui_app/modules/popup_host.rs:177`.

### Low

- NewsModule swallows parsing errors and may remain in a "Loading..."
  state without surfacing failures, which makes missing content harder
  to diagnose. File: `src/gpui_app/modules/news.rs:231`.
