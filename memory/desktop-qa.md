# Desktop QA

Last updated: 2026-07-01

## Current Snapshot

状态：Preliminary / Tauri dev Agent-Reach reader smoke, Computer Use target blocked

旧桌面实现清空后，本轮已恢复 `pnpm tauri dev` 可运行的 Tauri dev app。命令已编译并运行 `target/debug/reachnote-app`。最新桌面冒烟使用本地 mock Jina Reader + fake Claude CLI 验证 Agent-Reach 正文读取进入 `AnalysisRequest`。

注意：Computer Use 本轮对 `ReachNote`、`reachnote-app`、`com.reachnote.app` 和 debug binary 完整路径均返回 Invalid app，因此仍不能给出 Computer Use PASS。macOS Accessibility 可识别 dev 窗口 `process "reachnote-app"`，本轮用它完成真实 Tauri 窗口冒烟；浏览器端仅作视觉辅助。

## Verification Status

- `pnpm tauri dev`：通过，Tauri dev runtime 已启动。
- Accessibility smoke：通过，窗口标题 `ReachNote`、进程 `reachnote-app`；采集页空 URL / 非法 URL CTA disabled；合法 URL 创建本地任务；重启后队列页仍显示 `https://openai.com/index/gpt-4o / openai.com / 排队中 / Claude CLI`。
- DB persistence：通过，`~/Library/Application Support/com.reachnote.app/reachnote.db` 存在，`tasks` 表 1 行，仓库内无 `reachnote.db`。
- Worker failure smoke：通过，`REACHNOTE_CLAUDE_CMD=__missing_claude__ pnpm tauri dev` 启动真实 Tauri 窗口后，采集合法 URL，SQLite 记录写入 `failed/provider_unavailable`，`error_message` 为 Claude CLI 缺失提示；队列页显示 `失败` badge、行内失败原因、`重试` 按钮和最小 worker banner。
- Structured analysis smoke：通过，`REACHNOTE_CLAUDE_CMD=/tmp/reachnote-fake-claude-success REACHNOTE_AI_TIMEOUT_SECS=10 pnpm tauri dev` 启动真实 Tauri 窗口后，采集合法 URL，SQLite 最新记录写入 `analyzed`，包含 `analysis_json`；队列页显示 `已分析`、结构化标题、4 星评分和 fake model。
- Agent-Reach reader smoke：通过，`REACHNOTE_AGENT_REACH_WEB_READER_BASE_URL=http://127.0.0.1:18089 REACHNOTE_READER_TIMEOUT_SECS=5 REACHNOTE_CLAUDE_CMD=/tmp/reachnote-fake-claude-reader-check REACHNOTE_AI_TIMEOUT_SECS=10 pnpm tauri dev` 启动真实 Tauri 窗口后，队列 UI 最新行显示 `Reader Content OK / 已分析 / fake-claude-reader-check`；SQLite 最新 `analysis_json.summary` 显示“基于 Agent-Reach web route 读取正文后生成”。
- QA screenshot：`/tmp/reachnote-worker-provider-unavailable.png`，用于本轮人工视觉核对；截图不提交仓库。
- QA screenshot：`/tmp/reachnote-analysis-success.png`，用于本轮结构化分析成功路径视觉核对；截图不提交仓库。
- Computer Use：Blocked；当前工具无法识别 dev app，不作为最终 PASS。

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
