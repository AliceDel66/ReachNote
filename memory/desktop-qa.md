# Desktop QA

Last updated: 2026-06-30

## Current Snapshot

状态：Preliminary / Tauri dev worker failure smoke, Computer Use target blocked

旧桌面实现清空后，本轮已恢复 `pnpm tauri dev` 可运行的 Tauri dev app。命令已编译并运行 `target/debug/reachnote-app`。最新桌面冒烟使用 `REACHNOTE_CLAUDE_CMD=__missing_claude__ pnpm tauri dev` 验证 provider unavailable 失败路径。

注意：Computer Use 本轮对 `ReachNote`、`reachnote-app`、`com.reachnote.app` 和 debug binary 完整路径均返回 Invalid app，因此仍不能给出 Computer Use PASS。macOS Accessibility 可识别 dev 窗口 `process "reachnote-app"`，本轮用它完成真实 Tauri 窗口冒烟；浏览器端仅作视觉辅助。

## Verification Status

- `pnpm tauri dev`：通过，Tauri dev runtime 已启动。
- Accessibility smoke：通过，窗口标题 `ReachNote`、进程 `reachnote-app`；采集页空 URL / 非法 URL CTA disabled；合法 URL 创建本地任务；重启后队列页仍显示 `https://openai.com/index/gpt-4o / openai.com / 排队中 / Claude CLI`。
- DB persistence：通过，`~/Library/Application Support/com.reachnote.app/reachnote.db` 存在，`tasks` 表 1 行，仓库内无 `reachnote.db`。
- Worker failure smoke：通过，`REACHNOTE_CLAUDE_CMD=__missing_claude__ pnpm tauri dev` 启动真实 Tauri 窗口后，采集合法 URL，SQLite 记录写入 `failed/provider_unavailable`，`error_message` 为 Claude CLI 缺失提示；队列页显示 `失败` badge、行内失败原因、`重试` 按钮和最小 worker banner。
- QA screenshot：`/tmp/reachnote-worker-provider-unavailable.png`，用于本轮人工视觉核对；截图不提交仓库。
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
