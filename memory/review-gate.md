# Review Gate

Last updated: 2026-07-01

## Current Snapshot

状态：PASS / Analyzed-to-Notion auto-sync bugfix passed Claude gate

历史上多个队列/worker/provider/reader/Notion 切片的 Claude CLI 只读 review gate 曾 timeout 或 blocked。最新 `Analyzed -> Notion` 自动同步 bugfix 已完成 Claude gate：无 P0/P1，1 个 P2（批量补同步遇到极低概率 DB 错误会提前退出，后续 refresh 可恢复），Gate = PASS。

## Gate Rules

- Review 重点：行为回归、测试缺口、权限/安全问题、数据契约破坏、部署风险、未清理调试代码。
- 有 P0/P1 finding 时不得 ship。
- 需要 AI review 时，另一个 agent/model 只读审阅，不直接改文件。

## Progress Log

### 2026-06-30

- Review attempt 1：Claude CLI 只读审阅返回 FAIL；有效风险点已修复，明显错读事实未采纳。
- Review attempt 2：`claude -p --output-format text --disable-slash-commands --safe-mode --model sonnet --tools "" --no-session-persistence` 使用缩小 packet，180 秒 timeout，无输出。
- 当前 gate：Blocked。实现验证通过，但没有可采信的 Claude PASS。
- 提交并推送当前基线：`c49c530`，中文 commit `实现本地队列闭环与静态桌面壳`，已推送 `origin/main`。
- Review attempt 3：针对最小 worker/provider_unavailable 切片，使用精简 packet（目标、验证结果、`provider.rs`、`src-tauri/src/lib.rs`、`src-tauri/src/store.rs`、`src/App.tsx`、`src/styles.css` diff），命令为 `timeout 180 claude -p --output-format text --disable-slash-commands --safe-mode --model sonnet --tools "" --no-session-persistence "$REVIEW_PROMPT"`；结果 180 秒超时，退出码 124，无输出。
- 当前 gate：Blocked。`pnpm typecheck`、`pnpm build`、`cargo test -p reachnote-core`、`cargo check --manifest-path src-tauri/Cargo.toml`、Tauri dev provider_unavailable smoke 均通过，但没有可采信的 Claude PASS。

### 2026-07-01

- Review attempt 4：针对结构化分析成功路径与三 provider adapter 切片，使用精简 packet（目标、验证结果、`crates/core/src/analysis.rs`、`src-tauri/src/provider.rs`、`src-tauri/src/lib.rs`、`src-tauri/src/store.rs`、`src/App.tsx`、`src/styles.css` diff），命令为 `timeout 180 claude -p --output-format text --disable-slash-commands --safe-mode --model sonnet --tools "" --no-session-persistence "$REVIEW_PROMPT"`；结果 180 秒超时，退出码 124，无输出。
- 当前 gate：Blocked。`pnpm typecheck`、`pnpm build`、`cargo test -p reachnote-core`、`cargo test --manifest-path src-tauri/Cargo.toml`、`cargo check --manifest-path src-tauri/Cargo.toml`、Tauri dev fake Claude structured analysis smoke 均通过，但没有可采信的 Claude PASS。
- Review attempt 5：针对 Agent-Reach web/Jina Reader 内容读取进入 `AnalysisRequest` 切片，packet 路径 `/tmp/reachnote-reader-review-packet.md`，包含目标、验证结果、`crates/core/src/analysis.rs`、`src-tauri/src/reader.rs`、`src-tauri/src/lib.rs` diff、`src/App.tsx` diff；命令为 `timeout 180 claude -p --output-format text --disable-slash-commands --safe-mode --model sonnet --tools "" --no-session-persistence "$(cat /tmp/reachnote-reader-review-packet.md)"`；结果 180 秒超时，退出码 124，无输出。
- 当前 gate：Blocked。`pnpm typecheck`、`pnpm build`、`cargo test -p reachnote-core`、`cargo test --manifest-path src-tauri/Cargo.toml`、`cargo check --manifest-path src-tauri/Cargo.toml`、Tauri dev 本地 mock Jina Reader + fake Claude reader smoke 均通过，但没有可采信的 Claude PASS。
- Review attempt 6：针对 Notion 本地 settings、`sync_capture_task`、GitHub repo reader fallback、真实 E2E 切片，packet 路径 `/tmp/reachnote-notion-review-packet.md`，包含目标、验证结果、`src-tauri/src/store.rs`、`src-tauri/src/notion.rs`、`src-tauri/src/reader.rs`、`src-tauri/src/lib.rs`、`crates/core/src/notion.rs` 关键 diff；命令为 `/opt/homebrew/bin/timeout 180 claude -p --output-format text --disable-slash-commands --safe-mode --model sonnet --tools "" --no-session-persistence "$(cat /tmp/reachnote-notion-review-packet.md)"`；结果 180 秒超时，退出码 124，无输出。
- 当前 gate：Blocked。实现验证和真实 E2E 均通过，但没有可采信的 Claude review PASS。
- Review attempt 7：针对 Notion adapter 最小同步收尾（`Analyzed -> Syncing -> Synced/Failed`、Tauri dev AX smoke、Notion API page 验证）切片，packet 路径 `/tmp/reachnote-notion-sync-review-packet.md`，包含目标、验证结果、`git diff --stat` 和本切片核心 diff；命令为 `timeout 180 claude -p --output-format text --disable-slash-commands --safe-mode --model sonnet --tools "" --no-session-persistence "$(cat /tmp/reachnote-notion-sync-review-packet.md)"`；结果 180 秒超时，退出码 124，无输出。
- 当前 gate：Blocked。`pnpm typecheck`、`pnpm build`、`cargo test -p reachnote-core`、`cargo test --manifest-path src-tauri/Cargo.toml`、`cargo check --manifest-path src-tauri/Cargo.toml`、`git diff --check`、Tauri dev AX Notion sync smoke 和 Notion API page 读取均通过，但没有可采信的 Claude review PASS。
- Review attempt 8：针对队列 in-progress 状态恢复与重试调度切片，packet 只包含目标、验证结果、`git diff --stat`、`src-tauri/src/store.rs` stale recovery 相关函数、`src-tauri/src/lib.rs` `recover_interrupted_tasks` / `retry_capture_task` / tests、`src/App.tsx` 加载/轮询/重试调用；命令为 `/opt/homebrew/bin/timeout 180 claude -p --output-format text --disable-slash-commands --safe-mode --model sonnet --tools "" --no-session-persistence "$REVIEW_PROMPT"`；结果 180 秒超时，退出码 124，无输出。
- 当前 gate：Blocked。`cargo test --manifest-path src-tauri/Cargo.toml`、`cargo test -p reachnote-core`、`pnpm typecheck`、`pnpm build`、`cargo check --manifest-path src-tauri/Cargo.toml`、`git diff --check`、Tauri dev AX stale recovery smoke 均通过，但没有可采信的 Claude review PASS。
- Review attempt 9：针对顶部右侧 icon 优化与“隐藏到 macOS 系统菜单栏/后台运行”切片，Claude CLI 只读 review 返回 FAIL；有效 P1 为 `restore_main_window` 强制 `set_size(1180,780)` 和 `center()` 会丢失用户窗口几何。已修复为恢复时只 `show/unminimize/set_focus`。P2 中统一 icon 尺寸、将隐藏 icon 改为 `Minimize2`、移除无用窗口属性重置均已处理。
- Review attempt 10：同一切片二次 Claude gate 返回 conditional PASS；条件为确认 `set_compact_mode` 在 `generate_handler!` 中注册、`src-tauri/capabilities/default.json` 包含 `core:window:allow-hide`。已验证 `src-tauri/src/lib.rs` 命中 `set_compact_mode` 定义和 handler 注册，capability 命中 `core:window:allow-hide`；`pnpm tauri build --debug --bundles app --no-sign` 通过。因此本切片 gate = PASS。真实点击隐藏仍因 `osascript` 辅助访问被拒未完成最终桌面交互 PASS。
- Review attempt 11：针对用户截图中任务停在 `已分析` 未同步 Notion 的 bugfix，Claude CLI 只读 review 返回 PASS，无 P0/P1。唯一 P2：`sync_pending_analyzed_tasks_blocking` 在批量循环中遇到 DB 级 `Err` 会提前退出，后续任务在当次调用中跳过；正常 Notion 失败走 `fail_sync_task -> Ok(Failed task)`，不触发此问题，且下一轮 refresh 可恢复。验证矩阵：`pnpm typecheck`、`pnpm build`、`cargo test -p reachnote-core`、`cargo test --manifest-path src-tauri/Cargo.toml`、`cargo check --manifest-path src-tauri/Cargo.toml`、`git diff --check`、Tauri dev AX orphan analyzed auto-sync smoke 均通过。
