# Integration Progress

Last updated: 2026-07-01

## Current Snapshot

状态：In Progress / Agent-Reach content enters structured analysis

当前仓库已从 PRD-only 推进到可运行桌面壳，并完成本地队列、provider 失败路径、Agent-Reach web 正文读取和结构化分析成功路径：`Article URL -> create_capture_task(provider_id) -> SQLite tasks -> run_capture_task -> Reading -> AgentReachWebReader(Jina Reader) -> AnalysisRequest(content_text/content_reader) -> Analyzing -> provider adapter -> AnalysisResult JSON validation -> Analyzed/Failed -> 队列页显示结果或失败原因`。这不是完整 PRD 闭环；Notion 写入和后台调度器仍未接入。

最新 PRD：

- `plans/prds/20260630-1906-reachnote-mvp-reset.prd.md`

## Verification Status

- 本地队列 runtime 可验证：`pnpm tauri dev` 已启动 dev app。
- 端到端本地队列冒烟：空库 `tasks` 为 0；采集页空 URL / `abc` 时 CTA disabled；合法 URL `https://openai.com/index/gpt-4o` 创建 `queued` 任务；重启 `pnpm tauri dev` 后队列仍显示该任务，DB count 为 1。
- 最小 worker 失败路径冒烟：`REACHNOTE_CLAUDE_CMD=__missing_claude__ pnpm tauri dev` 下采集合法 URL，SQLite 最新记录为 `failed/provider_unavailable`，`error_message` 写入 Claude CLI 缺失提示；队列页行内显示失败原因和重试按钮。
- 结构化分析成功路径冒烟：fake Claude CLI 返回合法 JSON；Tauri dev 真实窗口采集后 SQLite 最新记录为 `analyzed`，包含 title、score、model、analysis_json；队列 UI 显示 `已分析`、标题、评分和模型。
- Agent-Reach 内容读取冒烟：本地 mock Jina Reader 返回固定正文；fake Claude CLI 检查 prompt 是否包含该正文；Tauri dev 真实窗口最新记录为 `analyzed/Reader Content OK`，`analysis_json.summary` 为“基于 Agent-Reach web route 读取正文后生成”。
- Provider adapter tests：Claude/Codex fake CLI 和 OpenAI-compatible 本地 mock HTTP 均通过，不触外网、不使用真实 token。
- 完整 capture/analyze/sync 仍未实现，不可声明 Notion 写入完成。

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

### 2026-07-01

- 第四刀实现结构化分析成功路径：`run_capture_task` 调用 `ProviderRunner`，成功后写 `Analyzed`，并把结构化研究卡保存在 `analysis_json`。
- provider 范围扩展：`claude_cli`、`codex_cli`、`openai_compatible` 已共享 core `AnalysisResult` 契约。Claude/Codex 缺失、超时或执行失败会写标准错误；OpenAI-compatible 使用环境变量配置。
- 本轮明确没有接 Agent-Reach，因此 prompt 要求模型不得声称已读取网页正文，只能根据 URL 和补充说明生成待复核研究卡。
- Claude review gate：本轮结构化分析 provider 切片精简 packet 仍 180 秒 timeout，无输出；当前 gate 继续 Blocked，不能声明 Claude PASS。
- 下一步入口：接 Agent-Reach/内容读取，把 `AnalysisRequest` 从 URL-only 升级为 URL + normalized content，再保留同一 provider 契约。
- 第五刀实现 Agent-Reach 内容读取：`src-tauri/src/reader.rs` 使用 Agent-Reach web route / Jina Reader 读取 URL 正文，`run_capture_task` 先写 `Reading`，读取成功后再进入 `Analyzing`。
- `AnalysisRequest` 已从 URL-only 升级为 URL + normalized content：`content_text` 进入统一 prompt，`content_reader` 标记来源；读取失败直接写 `Failed/read_failed` 或 `network_failed`。
- Tauri dev 冒烟使用本地 mock reader 与 fake Claude CLI，验证正文确实进入 provider prompt：SQLite 最新任务 `task-1782837428-477655000-32681-0` 为 `analyzed`，标题 `Reader Content OK`，model `fake-claude-reader-check`。
- 未闭环风险：当前不持久化全文，仅把本轮读取正文送入 provider；真实网络读取可能受目标站点/Jina Reader 可用性影响；Notion 同步仍未接入。
