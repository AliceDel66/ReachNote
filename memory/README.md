# ReachNote Memory

Last updated: 2026-06-30

## Current Snapshot

状态：In Progress / Local queue worker foundation

用户要求清空旧实现后，当前已恢复 Tauri 2 + React 18 + HeroUI + Rust core 最小桌面壳，并推进到本地 SQLite 队列与最小 worker 地基。

当前事实：

- 旧前端、Tauri、Rust core、构建配置、旧 QA 和旧 progress memory 已从工作树删除；后续实现均来自 reset 后的新代码。
- `src/`、`src-tauri/`、`crates/core/` 当前已有新脚手架和本地队列实现，不能再按“无源码”假设工作。
- 新版 UI 设计图登记在 `memory/design-source.md`。
- 新 PRD 输出在 `plans/prds/20260630-1906-reachnote-mvp-reset.prd.md`。
- 最新代码链路：`Article URL -> create_capture_task -> SQLite tasks -> run_capture_task -> Claude CLI availability check -> Analyzing/Failed -> Queue UI`。
- 当前 worker 只检查 Claude CLI 是否可用；不调用 Claude、不接 Agent-Reach、不请求网页、不写 Notion。缺少 Claude CLI 时写入 `Failed/provider_unavailable/error_message`。

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
| `frontend-progress.md` | 前端当前状态；队列/采集 UI 已接本地任务和失败展示 |
| `backend-progress.md` | 后端当前状态；SQLite tasks、TaskStatus、最小 worker 已接 |
| `integration-progress.md` | 端到端状态；本地 queue + provider_unavailable 失败路径已通 |
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
