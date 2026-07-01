# Backend Progress

Last updated: 2026-07-01

## Current Snapshot

状态：Done / Analysis-to-Notion auto-sync hardened

旧 Rust core 与 Tauri command 实现已清空后，当前已恢复 Rust workspace、`reachnote-core` crate、Tauri 2 app shell，并完成本地队列、结构化分析、GitHub repo 真实读取 fallback、Notion 同步地基、队列 in-progress 恢复和已分析任务补同步：core 任务领域类型、URL/domain 校验、AnalysisResult JSON 契约、Notion property 映射、SQLite `tasks` / `notion_settings` 表、`create_capture_task` / `list_capture_tasks` / `recover_interrupted_tasks` / `run_capture_task` / `retry_capture_task` / `sync_pending_analyzed_tasks` / `sync_capture_task` / `get_notion_settings` / `save_notion_settings` / `test_notion_connection` Tauri commands。`run_capture_task` 分析成功后已在后端继续同步 Notion；队列加载也会补扫 `Analyzed + analysis_json + no notion_page_id` 遗留任务。当前仍没有后台自动调度器或 OS keychain。

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
- `src-tauri/src/notion.rs`
- `src-tauri/src/store.rs`
- `src-tauri/tauri.conf.json`
- `src-tauri/capabilities/default.json`

## Verification Status

- `cargo test -p reachnote-core`：通过，20 个测试；覆盖 task status snake_case 序列化、URL 校验、domain 提取、shell status、provider id、AnalysisResult JSON 宽松解析、读取正文/未读取正文两种 prompt、Notion property 映射和 token mask。
- `cargo test --manifest-path src-tauri/Cargo.toml`：通过，32 passed / 1 ignored；覆盖 Claude/Codex fake CLI adapter、stdin prompt、OpenAI-compatible 本地 mock HTTP adapter、reader endpoint、GitHub repo URL 解析、NotionClient mock HTTP、SQLite `notion_settings` round-trip、stale processing task recovery、active processing retry rejection、带 `analysis_json` 的失败任务同步重试路径和 orphan `Analyzed` 补同步路径。
- `cargo check --manifest-path src-tauri/Cargo.toml`：通过。
- 真实 E2E：`REACHNOTE_CLAUDE_CMD=/opt/homebrew/bin/claude REACHNOTE_AI_TIMEOUT_SECS=240 REACHNOTE_READER_TIMEOUT_SECS=60 cargo test --manifest-path src-tauri/Cargo.toml real_e2e_fe_fidelity_kit_claude_to_notion -- --ignored --nocapture --test-threads=1`：通过，1 passed，真实创建 Notion page `390c9b0c-3c3c-81d2-b04d-f0cd5b8859bb`。
- Tauri dev Notion UI smoke：通过。使用本地 mock reader、fake Claude CLI 和 SQLite `notion_settings`，从采集页提交非敏感 `example.com` smoke URL 后，验证时该任务写为 `synced`，包含 `notion_page_id` / `synced_at`；Notion API 读取 page 返回 HTTP 200，Title/URL/Status/Score/Source Type/Tags/AI Model 均匹配。
- Tauri dev stale recovery smoke：通过。向 app data SQLite 插入超过 300 秒未更新的测试 `reading` 任务后刷新真实 Tauri 窗口，`recover_interrupted_tasks` 将其写为 `failed/read_failed`，队列页显示恢复错误原因和 `重试` 按钮；测试 row 已删除。

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
- 新增本地 Notion 配置：`TaskStore` 建表 `notion_settings`，`get_notion_settings` / `save_notion_settings` singleton round-trip；Tauri 设置页命令返回 masked `NotionSettingsView`，不回传明文 token。
- `src-tauri/src/notion.rs`：`NotionClient::from_env()` 改为 `NotionClient::from_settings(NotionSettings)`；新增 `test_connection` 读 `/v1/databases/{id}`；`create_page` 继续走 `2022-06-28 + parent.database_id`。
- `sync_capture_task`：只同步已分析或带 `analysis_json` 的失败任务；状态写 `Syncing`，从 SQLite local settings 读 token/database，成功写 `Synced/notion_page_id/synced_at`，失败保留本地研究卡并写 `error_kind/error_message`。
- 真实数据修复：Jina Reader 对 `github.com` 真实返回 451，因此 `AgentReachWebReader` 对 GitHub repo URL 改走 GitHub API metadata + README raw fallback，reader 标记为 `GitHub API / README`。
- 真实 E2E 验证：`AliceDel66/fe-fidelity-kit` 通过 GitHub fallback 读取真实内容，真实 Claude CLI 生成非 fake `AnalysisResult`，真实 Notion API 创建 page `390c9b0c-3c3c-81d2-b04d-f0cd5b8859bb`。
- 未闭环风险：Notion token 当前按用户本轮要求存 SQLite，尚未迁移 OS keychain；Computer Use 插件仍不能绑定 debug app，桌面 PASS 依赖 macOS Accessibility fallback，见 `desktop-qa.md`。
- Notion 最小同步收尾：`sync_capture_task` 已在 Tauri dev 真实窗口中由采集页自动触发，完成 `Analyzed -> Syncing -> Synced`；失败路径由 NotionClient 分类为 `notion_unauthorized` / `schema_mismatch` / `network_failed` 并保留本地研究卡。
- 本轮验证：`pnpm typecheck`、`pnpm build`、`cargo test -p reachnote-core`、`cargo test --manifest-path src-tauri/Cargo.toml`、`cargo check --manifest-path src-tauri/Cargo.toml`、`git diff --check` 全部通过。
- 第七刀实现队列 in-progress 恢复与重试调度：`TaskStore::recover_stale_processing_tasks` 恢复 stale `Reading/Analyzing/Syncing`，Tauri command `recover_interrupted_tasks` 默认阈值 300 秒（可用 `REACHNOTE_STALE_TASK_SECS` 覆盖），`retry_capture_task` 统一处理 queued、failed、analyzed、synced 和 active processing 任务。
- 恢复策略：中断任务不清空 `analysis_json` / `notion_page_id` / `synced_at`；`reading/analyzing/syncing` 均落为 `failed/read_failed` 并写入面向用户的阶段性错误文案，方便队列页直接提示下一步。
- 本轮验证：`cargo fmt`、`cargo test --manifest-path src-tauri/Cargo.toml`、`cargo test -p reachnote-core`、`pnpm typecheck`、`pnpm build`、`cargo check --manifest-path src-tauri/Cargo.toml` 均通过；Tauri dev + AX fallback stale recovery smoke 通过。
- Bugfix：修复 `run_capture_task` 分析成功后只写 `Analyzed`、同步依赖前端继续发起导致 reload/HMR/关闭后任务停在 `已分析` 的问题。后端新增 `run_and_sync_capture_task_blocking` 和 `sync_pending_analyzed_tasks`，`TaskStore::list_pending_sync_tasks` 只选 `Analyzed + analysis_json + no notion_page_id`。
- 验证：截图中的 `task-1782878357-900342000-78578-1` 经 Tauri dev reload 补同步为 `synced`，写入 Notion page id `390c9b0c-3c3c-81b7-8332-e4a8b4413cb6`。
