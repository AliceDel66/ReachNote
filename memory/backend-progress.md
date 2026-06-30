# Backend Progress

Last updated: 2026-06-30

## Current Snapshot

状态：Done / Local queue worker foundation

旧 Rust core 与 Tauri command 实现已清空后，当前已恢复 Rust workspace、`reachnote-core` crate、Tauri 2 app shell，并完成本地队列地基：core 任务领域类型、URL/domain 校验、SQLite `tasks` 表、`create_capture_task` / `list_capture_tasks` Tauri commands。最新切片新增最小本地 worker：`run_capture_task` 先把任务写为 `Analyzing`，再做 Claude CLI 本地可执行文件检测；缺少 CLI 时写回 `Failed/provider_unavailable/error_message`。当前仍没有 Agent-Reach 读取、Claude 内容分析、Notion 同步或 keychain。

## Changed Files

- `Cargo.toml`
- `crates/core/Cargo.toml`
- `crates/core/src/lib.rs`
- `crates/core/src/task.rs`
- `src-tauri/Cargo.toml`
- `src-tauri/build.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/main.rs`
- `src-tauri/src/provider.rs`
- `src-tauri/src/store.rs`
- `src-tauri/tauri.conf.json`
- `src-tauri/capabilities/default.json`

## Verification Status

- `cargo test -p reachnote-core`：通过，7 个测试；覆盖 task status snake_case 序列化、URL 校验、domain 提取和 shell status。
- `cargo check --manifest-path src-tauri/Cargo.toml`：通过。
- `REACHNOTE_CLAUDE_CMD=__missing_claude__ pnpm tauri dev`：通过，Tauri dev 编译并运行 `target/debug/reachnote-app`；采集合法 URL 后最新任务写入 `failed/provider_unavailable`，`error_message` 为 Claude CLI 缺失提示。

## Progress Log

### 2026-06-30

- 恢复 Rust workspace 和 Tauri 2 最小 app shell。
- 新增 `shell_status` Tauri command，仅用于验证前后端/core 边界可编译。
- 因上游 `cookie 0.18.1` 与最新 `time 0.3.52` API 不兼容，`src-tauri/Cargo.toml` 固定 `time = "=0.3.41"`，使 Tauri 依赖图可通过 `cargo check`。
- 新增 `crates/core/src/task.rs`：`TaskStatus`、`ErrorKind`、`Task`、`ValidatedUrl`、`validate_article_url`、`source_domain`。
- 新增 `src-tauri/src/store.rs`：用 `rusqlite 0.32` bundled SQLite 初始化 app data 下的 `reachnote.db`，建表 `tasks`，提供 `insert_task`、`list_tasks`、`get_task`。
- 新增 Tauri commands：`create_capture_task(url, note)` 合法 URL 创建 `Queued` 任务；`list_capture_tasks()` 按 `created_at` 倒序读取本地任务。
- `.gitignore` 删除 `docs/` 忽略规则；`docs/adr/0001-tech-stack.md` 现在不再被 `git check-ignore` 命中。`src-tauri/Cargo.toml` 已给 `time = "=0.3.41"` 加 workaround 注释。
- 提交并推送当前基线：`c49c530`，中文 commit `实现本地队列闭环与静态桌面壳`，已推送 `origin/main`。
- 新增 `src-tauri/src/provider.rs`：只扫描 `REACHNOTE_CLAUDE_CMD` 或 `PATH` 中的可执行文件，不启动 Claude 进程，不发网络请求。
- 新增 `run_capture_task(id)` Tauri command：读取 SQLite task，写 `Analyzing`；若 Claude CLI 不可用，写 `Failed`、`provider_unavailable`、面向用户的 `error_message`。
- 新增 `TaskStore::update_task`，用于原位更新 status/error fields；`get_task` 正式进入 worker 读取路径。
- 未闭环风险：Claude CLI 可用时当前只停在 `Analyzing`，尚未调用 Claude 做结构化分析；Agent-Reach、Notion、retry queue 调度器和 keychain 仍未实现。
