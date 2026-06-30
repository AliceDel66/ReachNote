# Backend Progress

Last updated: 2026-07-01

## Current Snapshot

状态：Done / Agent-Reach web content reader feeds AnalysisRequest

旧 Rust core 与 Tauri command 实现已清空后，当前已恢复 Rust workspace、`reachnote-core` crate、Tauri 2 app shell，并完成本地队列和结构化分析地基：core 任务领域类型、URL/domain 校验、AnalysisResult JSON 契约、SQLite `tasks` 表、`create_capture_task` / `list_capture_tasks` / `run_capture_task` Tauri commands。最新切片支持 `claude_cli`、`codex_cli`、`openai_compatible` 三种 provider，并已把 Agent-Reach web route / Jina Reader 读取到的正文写入 `AnalysisRequest.content_text`。当前仍没有 Notion 同步、后台调度器或 keychain。

## Changed Files

- `Cargo.toml`
- `Cargo.lock`
- `crates/core/Cargo.toml`
- `crates/core/src/analysis.rs`
- `crates/core/src/lib.rs`
- `crates/core/src/task.rs`
- `src-tauri/Cargo.toml`
- `src-tauri/build.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/main.rs`
- `src-tauri/src/provider.rs`
- `src-tauri/src/reader.rs`
- `src-tauri/src/store.rs`
- `src-tauri/tauri.conf.json`
- `src-tauri/capabilities/default.json`

## Verification Status

- `cargo test -p reachnote-core`：通过，13 个测试；覆盖 task status snake_case 序列化、URL 校验、domain 提取、shell status、provider id、AnalysisResult JSON 校验，以及读取正文/未读取正文两种 prompt。
- `cargo test --manifest-path src-tauri/Cargo.toml`：通过，15 个测试；覆盖 Claude/Codex fake CLI adapter、OpenAI-compatible 本地 mock HTTP adapter、reader endpoint 拼接和本地 mock reader 成功读取。
- `cargo check --manifest-path src-tauri/Cargo.toml`：通过。
- `REACHNOTE_AGENT_REACH_WEB_READER_BASE_URL=http://127.0.0.1:18089 REACHNOTE_CLAUDE_CMD=/tmp/reachnote-fake-claude-reader-check REACHNOTE_AI_TIMEOUT_SECS=10 pnpm tauri dev`：通过，Tauri dev 编译并运行 `target/debug/reachnote-app`；采集合法 URL 后最新任务写入 `analyzed`，标题为 `Reader Content OK`，证明 reader 正文进入 provider prompt。

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

### 2026-07-01

- 新增 `crates/core/src/analysis.rs`：定义 `ProviderId`、`AnalysisRequest`、`AnalysisResult`、`parse_analysis_result` 和 `build_analysis_prompt`，三种 provider 共用同一 JSON 契约。
- `TaskStatus` 新增 `Analyzed`，避免在未接 Notion 前把分析完成误标为 `Synced`。
- `Task` 新增 `provider_id`、`note`、`analysis_json`；SQLite schema 新增对应列，旧表自动 rebuild 迁移，保留旧任务并默认 `claude_cli`。
- `src-tauri/src/provider.rs` 改为 `ProviderRunner`：Claude/Codex 走本地 CLI，带 timeout；OpenAI-compatible 走 `{REACHNOTE_OPENAI_BASE_URL}/chat/completions`，使用 `REACHNOTE_OPENAI_MODEL` 和可选 `REACHNOTE_OPENAI_API_KEY`。
- `run_capture_task` 从 provider 返回 JSON 后进行 core 校验，成功写 `Analyzed/title/score/model/analysis_json`，失败写标准 `error_kind/error_message`。
- 未闭环风险：当前 prompt 只基于 URL 和补充说明生成“待复核”研究卡，不读取网页正文；Notion、Agent-Reach、OS keychain、持久化 provider 设置仍未接入。
- 新增 `src-tauri/src/reader.rs`：`AgentReachWebReader` 默认使用 Jina Reader (`https://r.jina.ai`) 作为 Agent-Reach web route 的文章正文读取入口，也可通过 `REACHNOTE_AGENT_REACH_WEB_READER_BASE_URL` 指向本地 mock。
- `AnalysisRequest` 新增 `content_text` / `content_reader`；`build_analysis_prompt` 在读取成功时要求基于正文生成研究卡，在未读取时要求明确待复核，正文输入截断到 12000 字符。
- `run_capture_task` 状态推进变为 `Queued -> Reading -> Analyzing -> Analyzed/Failed`；reader 失败会写 `ReadFailed` 或 `NetworkFailed`，不会继续调用 provider。
- 未闭环风险：Agent-Reach CLI 本机实际没有 `read` 子命令，本轮按 agent-reach skill 的 web route/Jina Reader 事实接入；尚未保存原文摘录到 SQLite，也未接 Notion、OS keychain 或后台调度器。
