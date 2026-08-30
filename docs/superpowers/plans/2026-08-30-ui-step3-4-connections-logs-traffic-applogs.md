# UI Step 3 + 4 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace stubs `connections.rs:1`, `logs.rs:1`, `dashboard.rs:39` with connections table (WS `?interval=1000`, sorting, filter, close), logs streaming (WS `/logs?level=`, levels/search/autoscroll) plus app-wide logger, and dashboard traffic graph (egui_plot 60 points WS `/traffic`).

**Architecture:** Keep `RClashApp` + `poll-promise::Promise::spawn_thread` + `reqwest::blocking` idiom from `app.rs:122-170`. Streaming via `tokio-tungstenite` WS readers in background threads writing to `mpsc` channels drained in `update()`. Graph via `egui_plot 0.32` full-width card under 3 columns in `dashboard.rs:88`. App logger via `log` + `fern` file + in-memory `VecDeque` for UI.

**Tech Stack:** Rust 1.77, `eframe 0.32.3`/`egui 0.32.3`, `egui_plot 0.32`, `tokio-tungstenite 0.24` + `futures-util 0.3`, `poll-promise 0.3`, `reqwest 0.12` (rustls), `serde_json`, `log 0.4` + `fern 0.6`.

**Spec:** `.agents/todo.md:25-26`, `.agents/architecture.md:30,57-59`, `mihomo/hub/route/server.go:124,371,482` + `connections.go:16,36`, `manager.go:124`, `tracker.go:24`.

## Global Constraints

- `cargo fmt --check && cargo check --all-targets && cargo clippy --all-targets -- -D warnings && cargo test` green before done (`.agents/commands.md:24`).
- UI text Russian, identifiers/commits English (`.agents/rules.md:9`).
- 6 tabs fixed `app.rs:8`, no new tabs without approval.
- Config via `dirs::config_dir()/RClash`, no DB.
- No patch to `mihomo/constant/*.go`, ldflags only.
- `manifest.json` contract unchanged.

---

## File Structure

**Modify:**
- `rclash/Cargo.toml` — add `egui_plot`, `tokio-tungstenite`, `futures-util`, `log`, `fern`
- `rclash/crates/rclash-core-manager/Cargo.toml` — add `tokio-tungstenite`, `futures-util` if needed (else only in root)
- `rclash/crates/rclash-core-manager/src/api.rs:15-258` — extend `TrafficInfo`, add `Snapshot/TrackerInfo/LogEntry`, `format_bytes`, `format_duration`
- `rclash/src/app.rs:41-384` — add fields `connections_*`, `logs_*`, `traffic_*`, methods `poll_*`, `start_*_stream`, `update()` calls
- `rclash/src/ui/connections.rs:1-9` — full table WS 1s
- `rclash/src/ui/logs.rs:1-11` — streaming + controls + App/Core tabs
- `rclash/src/ui/dashboard.rs:1-102` — graph + real up/down values
- `rclash/src/ui/settings.rs:109` — log level combo wired to `AppConfig`+logger
- `rclash/src/main.rs` — `logger::init()`
- `rclash/crates/rclash-config/src/lib.rs` — add `LogLevel` + `AppConfig.log_level`

**Create:**
- `rclash/src/logger.rs` — `AppLogger` impl `log::Log`, file + ring buffer, `init`, `drain_app_logs`

---

### Task 1: Dependencies + CoreApi models

**Files:**
- Modify: `rclash/Cargo.toml`, `rclash/crates/rclash-core-manager/Cargo.toml`, `rclash/crates/rclash-core-manager/src/api.rs`

**Interfaces:**
- Consumes: `eframe 0.32`
- Produces: `TrafficInfo {up,down,up_total,down_total}`, `Snapshot {downloadTotal,uploadTotal,connections,memory}`, `TrackerInfo {id,metadata,upload,download,start,chains,rule,rulePayload}`, `LogEntry {level,payload}`, `format_bytes(u64)->String`, `format_duration` for later tasks.

- [ ] Step 1: Add deps in `rclash/Cargo.toml` after `poll-promise`:
```toml
egui_plot = "0.32"
tokio-tungstenite = { version = "0.24", features = ["rustls-tls-webpki-roots"] }
futures-util = "0.3"
log = "0.4"
fern = { version = "0.6", features = ["colored"] }
```
- [ ] Step 2: Extend `api.rs` TrafficInfo to include `up_total/down_total` alias, add Snapshot/TrackerInfo/LogEntry + helpers + tests (see plan body).
- [ ] Step 3: `cargo check --all-targets` PASS
- [ ] Step 4: Commit

### Task 2: RClashApp WS state

**Files:**
- Modify: `rclash/src/app.rs`

- [ ] Step 1: Add fields: `connections_promise/data/error/filter/sort/close_promise`, `logs_buf/level/filter/search/autoscroll/rx/handle`, `traffic_up_buf/down_buf/total_up/down/rx`
- [ ] Step 2: Add methods `fetch_connections`, `poll_connections`, `close_connection_async`, `close_all_connections_async`, `start_traffic_stream`, `poll_traffic`, `start_logs_stream`, `poll_logs` following `app.rs:122-170` pattern, with 1s WS `?interval=1000` for connections
- [ ] Step 3: Wire `update()` to call `poll_*` + `request_repaint_after`
- [ ] Step 4: `cargo check` PASS

### Task 3: Connections UI table (WS interval 1000)

**Files:**
- Modify: `rclash/src/ui/connections.rs`

Consumes `RClashApp.connections_*`, Produces table with sort/filter/close.

- [ ] Step 1: Implement `show(ui, app, ctx)` with `section_header`, horizontal filter+ComboBox+Refresh+CloseAll, `ScrollArea` + striped `Grid` 7 cols, row close button `❌` calling `close_connection_async`, `empty_state` when none.
- [ ] Step 2: Verify `cargo check`, manual run shows table, close works, filter/sort work.

### Task 4: Logs WS + App logs tabs

**Files:**
- Modify: `rclash/src/app.rs`, `rclash/src/ui/logs.rs`

- [ ] Step 1: Add `LogsTab {Core, App}` state, `mpsc` WS reader for `ws://127.0.0.1:9090/logs?level=info` with `Authorization` header, buffer 500
- [ ] Step 2: UI with level ComboBox, filter/search TextEdit, autoscroll checkbox, Clear/Copy, `ScrollArea::stick_to_bottom`, tab selector Core/App, color per level
- [ ] Step 3: `cargo check` PASS, manual WS streams, level switch restarts stream

### Task 5: Dashboard traffic graph egui_plot 60pts

**Files:**
- Modify: `rclash/src/app.rs`, `rclash/src/ui/dashboard.rs`

- [ ] Step 1: Add `traffic_up_buf/down_buf: VecDeque<f64>` 60 cap, `traffic_stream` WS `ws://127.0.0.1:9090/traffic` 1/sec
- [ ] Step 2: In `dashboard.rs` add full-width `widgets::card` below 3 columns with `Plot::new("traffic").height(120.0).legend` + two `Line` with `PlotPoints`, real `↑/↓` values via `format_bytes`
- [ ] Step 3: `cargo check` PASS, manual graph slides

### Task 6: App-wide logger

**Files:**
- Create: `rclash/src/logger.rs`
- Modify: `rclash/src/main.rs`, `rclash/src/ui/settings.rs`, `rclash/crates/rclash-config/src/lib.rs`, `rclash/src/ui/logs.rs`

- [ ] Step 1: `logger.rs` impl `log::Log` with file `RClash/logs/app.log` rotation 5MB, in-memory `Mutex<VecDeque>`, `init(LevelFilter::Info)`
- [ ] Step 2: Wire `main.rs` early init, `settings.rs` ComboBox to `AppConfig.log_level` + `logger::set_level` + `save_app_config`
- [ ] Step 3: Logs tab App pane shows app logs with same controls, button `Открыть папку логов`
- [ ] Step 4: `cargo check/clippy/test` PASS

### Task 7: Verification

- [ ] `cargo fmt --check && cargo check --all-targets && cargo clippy --all-targets -- -D warnings && cargo test`
- [ ] Manual `cargo run` check all tabs
- [ ] Update `.agents/todo.md` marks done, record decision
