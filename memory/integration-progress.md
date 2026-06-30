# Integration Progress

Last updated: 2026-06-30

## Current Snapshot

状态：In Progress / Local queue worker failure path complete

当前仓库已从 PRD-only 推进到可运行桌面壳，并完成本地队列与最小 worker 失败路径：`Article URL -> create_capture_task -> SQLite tasks -> run_capture_task -> Analyzing -> Claude CLI availability -> Failed/provider_unavailable -> 队列页显示失败原因`。这不是完整 PRD 闭环；Claude CLI 内容分析、Agent-Reach 读取、Notion 写入、后台调度器仍未接入。

最新 PRD：

- `plans/prds/20260630-1906-reachnote-mvp-reset.prd.md`

## Verification Status

- 本地队列 runtime 可验证：`pnpm tauri dev` 已启动 dev app。
- 端到端本地队列冒烟：空库 `tasks` 为 0；采集页空 URL / `abc` 时 CTA disabled；合法 URL `https://openai.com/index/gpt-4o` 创建 `queued` 任务；重启 `pnpm tauri dev` 后队列仍显示该任务，DB count 为 1。
- 最小 worker 失败路径冒烟：`REACHNOTE_CLAUDE_CMD=__missing_claude__ pnpm tauri dev` 下采集合法 URL，SQLite 最新记录为 `failed/provider_unavailable`，`error_message` 写入 Claude CLI 缺失提示；队列页行内显示失败原因和重试按钮。
- 完整 capture/analyze/sync 仍未实现，不可声明 Notion 或 AI 链路完成。

## Progress Log

### 2026-06-30

- 已按用户要求清空旧实现，当前无可运行端到端链路。
- 新 PRD 明确下一条目标链路：`Article URL -> Local queue -> Claude CLI analysis -> Notion page -> Queue status`。
- Claude CLI PRD 起草尝试超时，采用 Codex fallback。
- 第一刀实现最小可运行静态壳：Tauri 2 + React 18 + HeroUI + Rust core。
- 根据用户反馈完成 UI 尺度修正：默认窗口 `1180x780`，采集页控件和字体改为更紧凑的桌面工具密度。
- 第二刀实现本地队列数据流：core 任务类型和 URL 校验、Tauri SQLite store、创建/列表命令、前端真实 invoke。
- SQLite DB 路径：`~/Library/Application Support/com.reachnote.app/reachnote.db`；仓库内未发现 `reachnote.db`。
- 下一步入口：在现有 `TaskStatus` 和 `tasks` 表基础上实现 worker/Claude CLI provider，把 `Queued` 推进到 `Analyzing` / `Failed`，仍先不接 Notion。
- Review gate：Claude 第一轮 FAIL 中的有效风险点已修复；第二轮 180 秒 timeout 无输出，当前 gate 记录为 Blocked，不能声明 Claude PASS。
- 提交并推送当前基线：`c49c530`，中文 commit `实现本地队列闭环与静态桌面壳`，已推送 `origin/main`。
- 第三刀实现最小本地 worker：新增 `run_capture_task`，前端采集成功后自动触发；worker 只检测 Claude CLI 可执行文件，缺失时写回 `Failed/provider_unavailable/error_message`。
- 本轮不接 Notion、不接 Agent-Reach、不读取网页、不调用 Claude 进程；验证通过的失败路径完全发生在本地 SQLite 和 Tauri command 内。
- Claude review gate：本轮 worker 切片精简 packet 仍 180 秒 timeout，无输出；当前 gate 继续 Blocked，不能声明 Claude PASS。
- 下一步入口：实现 Claude CLI provider 的真实结构化分析成功路径，把 `Analyzing` 推进到可渲染 research card；仍先不接 Notion。
