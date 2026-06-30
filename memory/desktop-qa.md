# Desktop QA

Last updated: 2026-06-30

## Current Snapshot

状态：Preliminary / Tauri dev local queue smoke, Computer Use target blocked

旧桌面实现清空后，本轮已恢复 `pnpm tauri dev` 可运行的 Tauri dev app。命令已编译并运行 `target/debug/reachnote-app`。

注意：Computer Use 本轮对 `ReachNote`、`reachnote-app`、`com.reachnote.app` 和 debug binary 完整路径均返回 Invalid app，因此仍不能给出 Computer Use PASS。macOS Accessibility 可识别 dev 窗口 `process "reachnote-app"`，本轮用它完成真实 Tauri 窗口冒烟；浏览器端仅作视觉辅助。

## Verification Status

- `pnpm tauri dev`：通过，Tauri dev runtime 已启动。
- Accessibility smoke：通过，窗口标题 `ReachNote`、进程 `reachnote-app`；采集页空 URL / 非法 URL CTA disabled；合法 URL 创建本地任务；重启后队列页仍显示 `https://openai.com/index/gpt-4o / openai.com / 排队中 / Claude CLI`。
- DB persistence：通过，`~/Library/Application Support/com.reachnote.app/reachnote.db` 存在，`tasks` 表 1 行，仓库内无 `reachnote.db`。
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
