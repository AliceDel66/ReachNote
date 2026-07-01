# ReachNote Memory

Last updated: 2026-07-01

## Current Snapshot

状态：Done / Analyzed-to-Notion auto-sync hardened

用户要求清空旧实现后，当前已恢复 Tauri 2 + React 18 + HeroUI + Rust core 最小桌面壳，并推进到本地 SQLite 队列与最小 worker 地基。

当前事实：

- 旧前端、Tauri、Rust core、构建配置、旧 QA 和旧 progress memory 已从工作树删除；后续实现均来自 reset 后的新代码。
- `src/`、`src-tauri/`、`crates/core/` 当前已有新脚手架和本地队列实现，不能再按“无源码”假设工作。
- 新版 UI 设计图登记在 `memory/design-source.md`。
- 新 PRD 输出在 `plans/prds/20260630-1906-reachnote-mvp-reset.prd.md`。
- 最新代码链路：`Article URL -> create_capture_task(provider_id) -> SQLite tasks -> recover_interrupted_tasks -> run_capture_task -> AgentReachWebReader(Jina Reader / GitHub API fallback) -> AnalysisRequest(content_text/content_reader) -> ProviderRunner -> AnalysisResult JSON validation -> Analyzed -> sync_capture_task -> local notion_settings -> NotionClient -> Synced/Failed -> Queue UI`。
- 当前 worker 支持 `claude_cli`、`codex_cli`、`openai_compatible` 三种 provider。Claude/Codex 走本地 CLI，OpenAI-compatible 读取 `REACHNOTE_OPENAI_BASE_URL` / `REACHNOTE_OPENAI_MODEL` / 可选 `REACHNOTE_OPENAI_API_KEY`。Agent-Reach web route 默认以 Jina Reader 作为文章正文读取入口，GitHub repo 走 GitHub API fallback；Notion 同步已接入本地 settings。
- Notion settings 已接 SQLite `notion_settings` singleton，前端设置页调用 `get_notion_settings` / `save_notion_settings` / `test_notion_connection`，同步不再读取 `NOTION_TOKEN` / `NOTION_DATABASE_ID` 环境变量。
- GitHub repo URL 真实读取已补 `GitHub API / README` fallback；原因是 Jina Reader 对 `github.com` 真实返回 451，导致用户给的 `AliceDel66/fe-fidelity-kit` 无法通过默认 Jina 路径读取。
- 真实后端 E2E 已通过：`AliceDel66/fe-fidelity-kit` -> GitHub API/README -> real Claude CLI -> real Notion page，输出 `REAL_E2E_PAGE_ID=390c9b0c-3c3c-81d2-b04d-f0cd5b8859bb`。这不是 fake/smoke 数据。
- Tauri dev 桌面 UI smoke 已通过 AX fallback：采集页提交非敏感 `example.com` smoke URL，fake reader + fake Claude 生成结构化卡，`sync_capture_task` 写入真实 Notion page；验证时该 smoke 任务为 `synced`，Notion API `GET /v1/pages/{id}` 返回 200 且 Title/URL/Status/Score/Source Type/Tags/AI Model 均匹配。
- 队列 in-progress 恢复已接入：前端加载/轮询队列前会调用 `recover_interrupted_tasks`，默认恢复超过 300 秒未更新的 `reading/analyzing/syncing` 为 `failed/read_failed`；失败行的 `重试` 统一调用 `retry_capture_task`，后端根据 `analysis_json` 决定重跑分析或只重试 Notion 同步。
- `Analyzed` 不再是会静默停住的终态：`run_capture_task` 后端命令分析成功后会继续 `sync_capture_task_blocking`；队列加载/刷新也会调用 `sync_pending_analyzed_tasks`，补同步历史遗留的 `Analyzed + analysis_json + no notion_page_id` 任务。
- 顶部右侧 icon 已按用户反馈收敛为更轻的 `Search` / `Settings2` / `Minimize2`；缩小目标已澄清为 macOS 系统菜单栏语义：隐藏主窗口、后台静默运行、后续用 Dock Reopen/快捷键唤回，不再渲染伪 compact bar 小窗口。
- Computer Use 仍是 Blocked：对 `ReachNote`、`reachnote-app` 和 debug binary 完整路径均返回 `Invalid app`。既有队列/同步桌面验证可记为 Tauri dev + macOS Accessibility fallback PASS；最新缩小隐藏点击因 `osascript` 辅助访问被拒只能记为 Blocked，不能记为 Computer Use PASS。

## Rules

- 任何后续实现必须先以最新 PRD 为 source of truth。
- 声称功能已实现前，必须回到当前代码和验证结果确认。
- 若 README 与 PRD/代码冲突，以最新 PRD 和当前代码为准，并同步修正文档。
- 不记录真实 API key、token、Cookie、账号信息或未脱敏用户内容。

## Files

| File | Purpose |
| --- | --- |
| `design-source.md` | 新版 UI 设计源和产品界面约束 |
| `development-plan.md` | P0 上下文校准、官方文档确认、第一刀范围和验证命令 |
| `frontend-progress.md` | 前端当前状态；队列/采集/设置已接 provider 选择和分析结果展示 |
| `backend-progress.md` | 后端当前状态；SQLite tasks、TaskStatus、AnalysisResult、多 provider worker 已接 |
| `integration-progress.md` | 端到端状态；本地 queue + structured analysis + provider failure 路径已通 |
| `review-gate.md` | 后续 review/gate 规则基线 |
| `desktop-qa.md` | 桌面验证基线；Tauri dev 冒烟通过，Computer Use 仍 blocked |

## Progress Log

### 2026-06-30

- 清空旧实现进度：删除旧前端、Tauri、Rust core、构建配置、旧 QA 和旧 progress memory。
- 清理旧 Tauri 生成 schema 残留：`src-tauri/gen/schemas/*`。
- 登记新版 UI：以用户提供的 2026-06-30 18:50 四张设计图为后续 UI source of truth。
- 产出轻量 PRD：`plans/prds/20260630-1906-reachnote-mvp-reset.prd.md`。
- 修正 README 状态：中英文 README 均标明当前为 PRD reset，旧实现不再代表当前状态。
- Claude PRD 起草命令 60 秒超时无输出，本轮 PRD 由 Codex fallback 编写和校验。
- P0 上下文校准：确认当前 `src/`、`src-tauri/`、`crates/core/` 排除 `target/` 后无实现源码；UI 源路径改为仓库内 `assets/ui/`；官方 Tauri / HeroUI / Notion 文档结论记录到 `memory/development-plan.md`。
- 第一刀实现：恢复 Tauri 2 + React 18 + HeroUI + Rust core 最小脚手架，实现新版 UI 静态壳；默认进入 `队列`，导航为 `采集 / 队列 / 模板 / 设置`。
- 根据用户截图反馈收缩 UI 尺度并修复采集页 input、textarea、粘贴按钮、CTA、预览标签换行问题。
- 验证：`pnpm typecheck`、`pnpm build`、`cargo test -p reachnote-core`、`cargo check --manifest-path src-tauri/Cargo.toml`、`pnpm tauri dev` 均通过；Browser smoke 无 console error/warn。Computer Use 直接桌面验证因旧安装版 `/Applications/ReachNote.app` 与 dev binary 目标冲突而标为 Blocked。
- 第二刀实现：本地队列数据流完成。新增 core task 类型/校验、Tauri SQLite store、`create_capture_task` / `list_capture_tasks`、前端真实 invoke；合法 URL 可创建 `Queued` 任务，重启 app 后队列仍从 SQLite 显示该任务。
- 修复项：前端 logo 改 Vite import，`.gitignore` 不再忽略正式 `docs/`，`time = "=0.3.41"` 加 workaround 注释。
- 提交并推送当前基线：`c49c530`，中文 commit `实现本地队列闭环与静态桌面壳`，已推送 `origin/main`。
- 第三刀实现：新增本地最小 worker `run_capture_task`，任务先写 `Analyzing`，再做本地 Claude CLI 可执行文件检测；缺少 CLI 时写回 `Failed/provider_unavailable/error_message`。前端创建任务后自动触发 worker，队列页展示失败原因并提供重试按钮。
- 验证：`pnpm typecheck`、`pnpm build`、`cargo test -p reachnote-core`、`cargo check --manifest-path src-tauri/Cargo.toml` 通过；`REACHNOTE_CLAUDE_CMD=__missing_claude__ pnpm tauri dev` 真实桌面冒烟通过，SQLite latest tasks 显示 `failed/provider_unavailable`，队列 UI 可见失败原因。

### 2026-07-01

- 第四刀实现：结构化分析成功路径完成。新增 `crates/core/src/analysis.rs`，定义 `ProviderId`、`AnalysisRequest`、`AnalysisResult`、JSON 校验和统一 prompt。
- 扩展 provider：`claude_cli`、`codex_cli`、`openai_compatible` 共享同一套结构化 JSON 契约；CLI 调用增加 timeout，OpenAI-compatible 使用本地配置的 base/model/api key。
- 扩展 SQLite `tasks`：新增 `provider_id`、`note`、`analysis_json`，新增 `analyzed` 状态；旧表会自动重建迁移，旧任务默认 `claude_cli`。
- 前端扩展：采集页和设置页可选择 AI provider；队列页支持 `已分析` 状态、标题、评分和模型展示；底部状态栏显示当前 provider。
- 验证：`pnpm typecheck`、`pnpm build`、`cargo test -p reachnote-core`、`cargo test --manifest-path src-tauri/Cargo.toml`、`cargo check --manifest-path src-tauri/Cargo.toml` 均通过；Tauri dev 使用 fake Claude CLI 完成真实窗口冒烟，SQLite 最新任务为 `analyzed` 且写入 `analysis_json`。
- 第五刀实现：Agent-Reach 内容读取接入 `AnalysisRequest` 正文输入。新增 `src-tauri/src/reader.rs`，通过 `REACHNOTE_AGENT_REACH_WEB_READER_BASE_URL`（默认 `https://r.jina.ai`）读取文章正文，成功后把 `content_text` / `content_reader` 注入统一 provider prompt；读取失败会写回 `Failed/read_failed` 或 `network_failed`。
- 验证：`cargo test -p reachnote-core` 13 个测试通过，`cargo test --manifest-path src-tauri/Cargo.toml` 15 个测试通过；Tauri dev 使用本地 mock Jina Reader + fake Claude CLI 完成真实窗口冒烟，SQLite 最新任务为 `analyzed/Reader Content OK`，`analysis_json.summary` 证明正文已进入 provider prompt。
- 第六刀实现：Notion 后端接入本地配置。`store.rs` 新增 `notion_settings` 表和 `get_notion_settings` / `save_notion_settings`；`src-tauri/src/notion.rs` 改为 `NotionSettings` 构造并新增 `test_connection`；`lib.rs` 注册 `get_notion_settings` / `save_notion_settings` / `test_notion_connection` / `sync_capture_task`，同步读取 SQLite settings。
- 真实数据修复：Jina Reader 对 `github.com` 返回 451，新增 GitHub repo direct reader fallback，通过 GitHub API metadata + README raw 构造正文。
- 真实 E2E：显式 ignored test `real_e2e_fe_fidelity_kit_claude_to_notion` 已用 `.env.notion`、`/opt/homebrew/bin/claude`、真实 GitHub repo 和真实 Notion API 跑通，返回 page id `390c9b0c-3c3c-81d2-b04d-f0cd5b8859bb`。
- 验证：`pnpm typecheck`、`pnpm build`、`cargo test -p reachnote-core`、`cargo test --manifest-path src-tauri/Cargo.toml`、`cargo check --manifest-path src-tauri/Cargo.toml` 全部通过；真实 E2E ignored test 1 passed。
- 本机队列数据状态：此前按用户要求清空过 `tasks` 表并保留 `notion_settings`；本轮 Notion UI smoke 又写入测试任务。验证时该 smoke 任务为 `synced`，带 `notion_page_id` 和 `synced_at`。当前 app data 可能被仍在运行的 dev session 继续改写；队列数据源仍为 `list_capture_tasks` -> SQLite，源码中未发现 seed/default row。
- 第六刀收尾验证：`pnpm typecheck`、`pnpm build`、`cargo test -p reachnote-core`、`cargo test --manifest-path src-tauri/Cargo.toml`、`cargo check --manifest-path src-tauri/Cargo.toml`、`git diff --check` 均通过；`pnpm tauri dev` 通过 AX fallback 触发真实 Notion 同步 smoke。
- 第七刀实现：队列 in-progress 状态恢复与重试调度。新增 `recover_interrupted_tasks` 和 `retry_capture_task`；恢复阈值默认 300 秒，可用 `REACHNOTE_STALE_TASK_SECS` 调整；恢复时保留本地研究卡和 Notion page 信息，只写失败状态与可读错误。
- 验证：`cargo test --manifest-path src-tauri/Cargo.toml` 30 passed / 1 ignored，`cargo test -p reachnote-core` 20 passed，`pnpm typecheck`、`pnpm build`、`cargo check --manifest-path src-tauri/Cargo.toml` 通过。Tauri dev + AX fallback 插入 stale `reading` 测试行后 reload，DB 验证恢复为 `failed/read_failed`，队列 UI 显示失败原因和 `重试`；测试 row 已删除。
- Bugfix：修复用户截图中的 `OpenCLI` 任务停在 `已分析` 不同步 Notion。根因是同步由前端在 `run_capture_task` 返回后继续发起，窗口 reload/HMR/关闭会让后续 `sync_capture_task` 丢失；修复后后端 `run_capture_task` 自己完成 `Analyzed -> Syncing -> Synced/Failed`，队列加载还会补同步遗留 `Analyzed`。
- 验证：`pnpm typecheck`、`pnpm build`、`cargo test -p reachnote-core`、`cargo test --manifest-path src-tauri/Cargo.toml` 32 passed / 1 ignored、`cargo check --manifest-path src-tauri/Cargo.toml` 通过；Tauri dev + AX fallback reload 后，截图中的 `task-1782878357-900342000-78578-1` 已从 `analyzed` 变为 `synced`，Notion page id `390c9b0c-3c3c-81b7-8332-e4a8b4413cb6`，队列 UI 显示 `已完成` 和 `Notion`。
- Release CI 修复：`main` 和 `v0.1.0` 已通过本机 GitHub SSH key + `127.0.0.1:7890` HTTP CONNECT 代理推到 `AliceDel66/ReachNote`；首次 release run `28496282107` 失败于 `Setup Node`，原因是 Node 20 不满足 `pnpm@11.5.0` 的 Node >= 22.13 / `node:sqlite` 要求。已将 `.github/workflows/release.yml` 改为 Node 24，并通过 `git diff --check`、`pnpm build` 验证；下一步提交修复并移动 `v0.1.0` tag 重新触发 draft release。
