# Desktop QA

Last updated: 2026-07-01

## Current Snapshot

状态：Preliminary / Orphan analyzed task auto-synced in Tauri dev, Computer Use blocked

旧桌面实现清空后，本轮已恢复 `pnpm tauri dev` 可运行的 Tauri dev app。命令已编译并运行 `target/debug/reachnote-app`。最新桌面验证确认用户截图中的 `OpenCLI` orphan analyzed 任务已在真实 Tauri dev 窗口 reload 后自动补同步到 Notion：队列 UI 从 `已分析` 变为 `已完成`，右侧出现 `Notion` 按钮；SQLite 写入 `notion_page_id`。Computer Use 仍不能绑定 debug app，因此本轮桌面验证仍是 AX fallback。

注意：Computer Use 本轮对 `reachnote-app` 和 debug binary 完整路径仍返回 Invalid app，因此仍不能给出 Computer Use PASS。macOS Accessibility 可读取 dev 窗口但后续 `click at` 返回 `“osascript”不允许辅助访问 (-25211)`；本轮隐藏点击与 Dock reopen 只能标为 Blocked。

## Verification Status

- `pnpm tauri dev`：通过，Tauri dev runtime 已启动；曾遇到旧 Vite 占用 `5173`，清理旧进程后可启动。
- Accessibility smoke：通过，窗口标题 `ReachNote`、进程 `reachnote-app`；采集页空 URL / 非法 URL CTA disabled；合法 URL 创建本地任务；重启后队列页仍显示 `https://openai.com/index/gpt-4o / openai.com / 排队中 / Claude CLI`。
- DB persistence：通过，`~/Library/Application Support/com.reachnote.app/reachnote.db` 存在，`tasks` 表 1 行，仓库内无 `reachnote.db`。
- Worker failure smoke：通过，`REACHNOTE_CLAUDE_CMD=__missing_claude__ pnpm tauri dev` 启动真实 Tauri 窗口后，采集合法 URL，SQLite 记录写入 `failed/provider_unavailable`，`error_message` 为 Claude CLI 缺失提示；队列页显示 `失败` badge、行内失败原因、`重试` 按钮和最小 worker banner。
- Structured analysis smoke：通过，`REACHNOTE_CLAUDE_CMD=/tmp/reachnote-fake-claude-success REACHNOTE_AI_TIMEOUT_SECS=10 pnpm tauri dev` 启动真实 Tauri 窗口后，采集合法 URL，验证时 SQLite 记录写入 `analyzed`，包含 `analysis_json`；队列页显示 `已分析`、结构化标题、4 星评分和 fake model。
- Agent-Reach reader smoke：通过，`REACHNOTE_AGENT_REACH_WEB_READER_BASE_URL=http://127.0.0.1:18089 REACHNOTE_READER_TIMEOUT_SECS=5 REACHNOTE_CLAUDE_CMD=/tmp/reachnote-fake-claude-reader-check REACHNOTE_AI_TIMEOUT_SECS=10 pnpm tauri dev` 启动真实 Tauri 窗口后，验证时队列 UI 显示 `Reader Content OK / 已分析 / fake-claude-reader-check`；验证时 SQLite `analysis_json.summary` 显示“基于 Agent-Reach web route 读取正文后生成”。
- Notion settings startup：通过；app data SQLite `~/Library/Application Support/com.reachnote.app/reachnote.db` 的 `notion_settings` singleton 为 configured，版本 `2022-06-28`，未打印 token。
- 真实后端 E2E：通过；ignored Rust test 用 `AliceDel66/fe-fidelity-kit` 真实数据、GitHub API/README fallback、real Claude CLI、real Notion API 创建 page `390c9b0c-3c3c-81d2-b04d-f0cd5b8859bb`。
- Tauri dev UI 自动提交 smoke URL：通过 AX fallback。Computer Use 不能绑定 debug app；改用 `System Events` 聚焦 `process "reachnote-app"`，用 `try click at` 避免点击抛错中断脚本，向采集页输入非敏感 `https://example.com/reachnote-notion-sync-smoke-*`，点击 CTA 后队列页验证时显示 `已完成` 和 `Notion` 链接。SQLite 验证时该记录为 `synced`，带 `notion_page_id` / `synced_at`；Notion API 读取 page HTTP 200，Title/URL/Status/Score/Source Type/Tags/AI Model 均匹配。
- Queue recovery smoke：通过 AX fallback。向 `~/Library/Application Support/com.reachnote.app/reachnote.db` 插入测试 `reading` row（`updated_at` 早于当前 1000 秒），聚焦 `process "reachnote-app"` 并 `Cmd+R` reload；验证时 DB 行变为 `failed/read_failed`，队列页截图显示红色 `失败`、阶段性失败原因和 `重试` 按钮。测试 row 已删除。
- Orphan analyzed auto-sync smoke：通过 AX fallback。聚焦 `process "reachnote-app"` 并 `Cmd+R` reload 后，用户截图中的第一行 `OpenCLI:把网站变成 CLI 供 AI 使用` 显示 `已完成` 和 `Notion`；DB 验证 `task-1782878357-900342000-78578-1` 为 `synced`，`notion_page_id=390c9b0c-3c3c-81b7-8332-e4a8b4413cb6`。
- Header icon visual check：Preliminary pass。真实 Tauri dev 窗口截图确认右上按钮改为轻量 `Search` / `Settings2` / `Minimize2`，设置按钮 active 态、按钮边框和尺寸正常；该截图包含本地 Notion 设置预览，不提交仓库、不外传。
- Shrink-to-background click：Blocked。尝试用 AX 点击右上 `隐藏到系统菜单栏` 按钮时，macOS 返回 `“osascript”不允许辅助访问 (-25211)`；因此未能用真实点击证明窗口隐藏、进程常驻和 Dock reopen 恢复。代码检查确认 `set_compact_mode(true)` 调用 `window.hide()`，`RunEvent::Reopen { has_visible_windows: false }` 调用 `restore_main_window_from_app`，恢复时只 `show/unminimize/set_focus`，不强制改窗口几何。Tauri capability 已补 `core:window:allow-hide`，debug bundle 构建通过。
- QA screenshot：`/tmp/reachnote-worker-provider-unavailable.png`，用于本轮人工视觉核对；截图不提交仓库。
- QA screenshot：`/tmp/reachnote-analysis-success.png`，用于本轮结构化分析成功路径视觉核对；截图不提交仓库。
- QA screenshot：`/tmp/reachnote-notion-synced-queue.png`，用于本轮 Notion sync UI smoke 视觉核对；截图不提交仓库。
- QA screenshot：`/tmp/reachnote-recovery-queue-after-nav.png`，用于本轮 stale recovery UI smoke 视觉核对；截图不提交仓库。
- QA screenshot：`/tmp/reachnote-opencli-autosynced.png`，用于本轮 orphan analyzed auto-sync 视觉核对；截图不提交仓库。
- Computer Use：Blocked；`mcp__computer_use.get_app_state` 对 `reachnote-app` 和 debug binary 绝对路径均返回 `Invalid app`，`list_apps` 也不列出 debug app，不作为最终 PASS。

## Future Baseline

后续用户可见桌面切片必须验证：

- 真实窗口打开和导航。
- 队列、采集、模板、设置关键交互。
- loading、empty、error、disabled、validation、permission、network failure。
- 文本溢出、资源加载、console error。
- macOS 和 Windows 差异如果进入发布范围，需要分别验证。

## Progress Log

### 2026-06-30

- 已验证 `pnpm tauri dev` 可启动 dev app。
- 已发现本机旧安装版 ReachNote 会干扰 Computer Use 定位；后续若要完成桌面 PASS，应先改 dev app identifier/productName，或临时移除/隔离旧安装版，再用 Computer Use 绑定正确窗口。
- 完成本地队列 Tauri dev 冒烟：`create_capture_task` 写入 SQLite，`list_capture_tasks` 在重启后仍能让队列页显示同一条任务。
- 提交并推送当前基线：`c49c530`，中文 commit `实现本地队列闭环与静态桌面壳`，已推送 `origin/main`。
- 完成 provider unavailable 桌面冒烟：真实 Tauri 窗口内触发采集，worker 将任务推进到 `Failed/provider_unavailable`；队列 UI 可见失败原因且无明显文本重叠。
- 当前仍是 Preliminary：Computer Use 目标识别问题未解决，最终 PASS 仍需后续恢复 Computer Use 绑定或安装版验证。

### 2026-07-01

- 完成 fake Claude CLI 结构化分析成功路径冒烟：真实 Tauri 窗口内触发采集，worker 将任务推进到 `Analyzed`；队列 UI 可见标题、绿色 `已分析`、评分和模型。
- DB 验证：`~/Library/Application Support/com.reachnote.app/reachnote.db` 最新任务包含 `provider_id=claude_cli`、`model=fake-claude-success`、`analysis_json`。
- Codex CLI 真实未配置 fake command 时触发过 timeout 失败记录，队列可见错误原因；adapter 参数错误已修复，并由 fake Codex CLI 单元测试覆盖。
- 当前仍是 Preliminary：Computer Use 目标识别问题未解决，本轮使用 macOS Accessibility + 截图 + SQLite 作为替代验证。
- 完成 Agent-Reach reader 桌面冒烟：真实 Tauri 窗口内最新队列行可见 `Reader Content OK`、绿色 `已分析` 和 `fake-claude-reader-check`；SQLite 最新任务 `analysis_json` 证明 mock reader 正文进入 provider prompt。
- Computer Use 插件本轮对 `ReachNote`、`reachnote-app`、`com.reachnote.app` 和 debug binary 路径均返回 `Invalid app`；macOS Accessibility 可读取 `process "reachnote-app"` 的窗口和控件树，因此本轮继续标为 Preliminary 而不是最终 PASS。
- 第六刀桌面状态：`pnpm tauri dev` 可启动，Notion settings 已进入 app data；真实后端 E2E PASS。Computer Use 仍 `Invalid app`，但 AX fallback 已完成采集页提交 smoke URL、队列页显示 `已完成/Notion`、SQLite `synced` 和 Notion API page 读取验证。
- 第七刀桌面状态：继续使用现有 `pnpm tauri dev` 进程验证；Computer Use 仍 `Invalid app`，但 AX fallback 已完成 stale `reading` 恢复验证。恢复后队列 UI 可见失败原因和 `重试`，SQLite 验证结果与 UI 一致；用于 QA 的 synthetic row 已清理。
- Header/缩小切片状态：顶部右侧 icon 优化已在真实 Tauri dev 窗口中人工截图核对；缩小目标按用户澄清改为“隐藏主窗口 + 后台静默运行 + Dock Reopen 恢复”，不再使用伪 compact bar。最终交互验证受 AX 权限阻断，仍需在 Computer Use 恢复或手动授权 `osascript` 后补一次真实点击 PASS。
- Claude gate：本切片第一轮 FAIL 的 P1 已修复；第二轮 conditional PASS 的 runtime 条件已满足（handler 注册 + `core:window:allow-hide`），因此 gate 记为 PASS。剩余桌面风险只在真实点击验证仍被 AX 权限阻断。
- Bugfix 桌面状态：用户截图中停在 `已分析` 的 `OpenCLI` 行已通过 Tauri dev reload 自动补同步为 `已完成`，UI 与 SQLite 一致。Computer Use 仍 `Invalid app`，不能声明 Computer Use PASS。
