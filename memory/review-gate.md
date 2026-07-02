# Review Gate

Last updated: 2026-07-01

## Current Snapshot

状态：Blocked / Slice 3 template registry Claude review timed out

历史上多个队列/worker/provider/reader/Notion 切片的 Claude CLI 只读 review gate 曾 timeout 或 blocked。`Analyzed -> Notion` 自动同步 bugfix 已完成 Claude gate：无 P0/P1，1 个 P2（批量补同步遇到极低概率 DB 错误会提前退出，后续 refresh 可恢复），Gate = PASS。Slice 1 settings/onboarding/App.tsx 拆分本地验证通过，但 Claude CLI 只读 review 运行约 3 分钟无有效输出，终止时仅返回 `Execution error`，因此该切片 gate = Blocked。桌面 QA 隔离修复已完成 Claude gate：最终 Gate = PASS。Slice 2 Agent-Reach 平台能力矩阵已完成 Claude gate：无 P0/P1，Gate = PASS。最新 Slice 3 模板注册/模板选择地基本地验证与桌面 QA 均通过，但 Claude CLI 只读 review 两次超时无输出，当前 gate = Blocked，不能声明 Claude PASS。

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
- Review attempt 12：针对 macOS native status item / tray 切片，packet 只包含目标、根因、`tauri/tray-icon` feature、`setup_reachnote_tray` 行为、验证结果和桌面验证阻塞项；命令为 `claude -p --output-format text --disable-slash-commands --safe-mode --model sonnet --tools "" --no-session-persistence "$REVIEW_PROMPT"`；结果为 `API Error: Unable to connect to API (FailedToOpenSocket)`，退出码 1，无可采信 review findings。当前 gate：Blocked。`cargo check --manifest-path src-tauri/Cargo.toml`、`pnpm typecheck`、`pnpm build`、`pnpm tauri build --debug --bundles app`、`git diff --check` 均通过；`cargo test --manifest-path src-tauri/Cargo.toml` 受沙盒禁止本地 mock server 监听影响，4 个 HTTP mock 用例 `Operation not permitted`，28 passed / 1 ignored。
- Review attempt 13：针对 Slice 1 settings/onboarding/App.tsx 拆分，packet 包含目标、关键行为、验证矩阵和已知桌面限制；命令为 `claude -p --output-format text --disable-slash-commands --safe-mode --model sonnet --tools "" --no-session-persistence "$REVIEW_PROMPT"`；约 3 分钟无输出后终止，仅返回 `Execution error`，无可采信 findings。当前 gate：Blocked。`pnpm typecheck`、`pnpm build`、`cargo test -p reachnote-core`、`cargo test --manifest-path src-tauri/Cargo.toml`、`cargo check --manifest-path src-tauri/Cargo.toml`、`git diff --check` 均通过；Tauri dev settings smoke 为 Preliminary，Computer Use 会绑定旧 debug bundle而非当前 dev binary。
- Review attempt 14：针对桌面验证隔离 / installed QA smoke 修复，packet 包含 `tauri.qa.conf.json`、`desktop-smoke-qa.sh`、Notion input autofill hardening、Computer Use 验证和数据隔离结果；Claude 返回 Gate = PASS。Claude 提到 P1：QA config 可能继承 `plugins.updater` 干扰 onboarding；本仓库真实 `src-tauri/tauri.conf.json` 未配置 updater，`rg` 也未发现 tauri updater plugin/endpoints，因此该 P1 是基于不存在配置的误报。为降低未来风险，已在 `src-tauri/tauri.qa.conf.json` 显式设置 `plugins.updater.active=false` 并复跑 `scripts/desktop-smoke-qa.sh --reset-data`，Tauri build 通过，Computer Use 打开 QA app 后仍显示 onboarding，无 updater 弹窗。P2 中 macOS guard 已修复；其它 P2（QA icon 与生产相同、app path 与 productName 绑定）记录为后续 polish，不阻塞。
- Review attempt 15：针对 Slice 2 Agent-Reach 平台能力矩阵，packet 包含目标、验证矩阵、`platform.rs`、store 快照、Tauri doctor command、frontend Settings/Capture 摘录；命令为 `claude -p --output-format text --disable-slash-commands --safe-mode --model sonnet --tools "" --no-session-persistence "$REVIEW_PROMPT"`。Claude 返回 Gate = PASS，无 P0/P1。P2：保留但暂不产出的 enum/action 变体、`NotSupportedYet` 语义偏保守、`get_environment_status` 写环境快照失败会放大到启动错误、`last_environment_check_json` 与 snapshot 双存可能未来漂移、summary 双遍历。Claude 还提到 async command blocking、Capture helper 未完整审阅、URL key mapping 未完整审阅；其中 blocking 是误判（command 入口已 `spawn_blocking`），后两项来自 packet 摘录截断，源码已覆盖。P2 不阻塞本切片。
- Review attempt 16：针对 Slice 3 模板注册/模板选择地基，packet 包含范围、验收标准、core/Tauri/frontend 行为摘要、验证矩阵和 Computer Use QA 结果；命令为 `/opt/homebrew/bin/timeout 240 claude -p --output-format text --disable-slash-commands --safe-mode --model sonnet --tools "" --no-session-persistence "$REVIEW_PROMPT"`。结果：退出码 124，超时无输出，不能作为 PASS。
- Review attempt 17：同一 Slice 3 使用更小 packet 重试，仅保留目标、关键改动和验证矩阵；命令为 `/opt/homebrew/bin/timeout 180 claude -p --output-format text --disable-slash-commands --safe-mode --model sonnet --tools "" --no-session-persistence "$REVIEW_PROMPT"`。结果：退出码 124，超时无输出。当前 gate：Blocked。已通过 `pnpm typecheck`、`pnpm build`、`cargo test -p reachnote-core`、`cargo test --manifest-path src-tauri/Cargo.toml`、`cargo check --manifest-path src-tauri/Cargo.toml`、`git diff --check`、`scripts/desktop-smoke-qa.sh --reset-data` 和 Computer Use QA；但没有可采信的 Claude review PASS。
