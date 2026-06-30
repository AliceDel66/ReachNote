# Backend Progress

Last updated: 2026-06-30

## Current Snapshot

状态：Done / Local queue foundation

旧 Rust core 与 Tauri command 实现已清空后，当前已恢复 Rust workspace、`reachnote-core` crate、Tauri 2 app shell，并完成本地队列地基：core 任务领域类型、URL/domain 校验、SQLite `tasks` 表、`create_capture_task` / `list_capture_tasks` Tauri commands。当前仍没有 Agent-Reach、Claude 分析、Notion 同步、keychain 或后台 worker。

## Changed Files

- `Cargo.toml`
- `crates/core/Cargo.toml`
- `crates/core/src/lib.rs`
- `crates/core/src/task.rs`
- `src-tauri/Cargo.toml`
- `src-tauri/build.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/main.rs`
- `src-tauri/src/store.rs`
- `src-tauri/tauri.conf.json`
- `src-tauri/capabilities/default.json`

## Verification Status

- `cargo test -p reachnote-core`：通过，7 个测试；覆盖 task status snake_case 序列化、URL 校验、domain 提取和 shell status。
- `cargo check --manifest-path src-tauri/Cargo.toml`：通过。
- `pnpm tauri dev`：通过，Tauri dev 编译并运行 `target/debug/reachnote-app`；首次启动在 app data 目录创建 `~/Library/Application Support/com.reachnote.app/reachnote.db`。

## Progress Log

### 2026-06-30

- 恢复 Rust workspace 和 Tauri 2 最小 app shell。
- 新增 `shell_status` Tauri command，仅用于验证前后端/core 边界可编译。
- 因上游 `cookie 0.18.1` 与最新 `time 0.3.52` API 不兼容，`src-tauri/Cargo.toml` 固定 `time = "=0.3.41"`，使 Tauri 依赖图可通过 `cargo check`。
- 新增 `crates/core/src/task.rs`：`TaskStatus`、`ErrorKind`、`Task`、`ValidatedUrl`、`validate_article_url`、`source_domain`。
- 新增 `src-tauri/src/store.rs`：用 `rusqlite 0.32` bundled SQLite 初始化 app data 下的 `reachnote.db`，建表 `tasks`，提供 `insert_task`、`list_tasks`、`get_task`。
- 新增 Tauri commands：`create_capture_task(url, note)` 合法 URL 创建 `Queued` 任务；`list_capture_tasks()` 按 `created_at` 倒序读取本地任务。
- `.gitignore` 删除 `docs/` 忽略规则；`docs/adr/0001-tech-stack.md` 现在不再被 `git check-ignore` 命中。`src-tauri/Cargo.toml` 已给 `time = "=0.3.41"` 加 workaround 注释。
- 未闭环风险：当前只落 `Queued` 任务，不推进 Reading/Analyzing/Syncing/Synced；Claude CLI、Agent-Reach、Notion、retry worker 和 keychain 仍未实现。
