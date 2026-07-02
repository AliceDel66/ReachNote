# Desktop QA

Last updated: 2026-07-01

## Current Snapshot

状态：Done / Slice 3 template registry verified in isolated QA app

旧桌面实现清空后，本轮已恢复 `pnpm tauri dev` 可运行的 Tauri dev app。命令已编译并运行 `target/debug/reachnote-app`。最新桌面验证确认原生 ReachNote status item 已进入 Tauri 启动层并通过 debug app 打包；用户截图中的 `OpenCLI` orphan analyzed 任务此前也已在真实 Tauri dev 窗口 reload 后自动补同步到 Notion：队列 UI 从 `已分析` 变为 `已完成`，右侧出现 `Notion` 按钮；SQLite 写入 `notion_page_id`。Slice 2 已通过隔离 `ReachNote QA.app` 验证 Agent-Reach 平台矩阵、手动刷新、快照持久化和 Capture 只读提示。Slice 3 已通过同一 QA app 验证模板默认选择、采集页模板下拉、队列模板列和重启 reload。Computer Use 仍不绑定裸 dev app，用户可见 PASS 继续走 QA app。

注意：旧问题的根因是 `pnpm tauri dev` 运行裸二进制 `target/debug/reachnote-app`，Computer Use 不能稳定按裸二进制绑定；按 `ReachNote` 名称会启动 `target/debug/bundle/macos/ReachNote.app`。当前修复是新增隔离安装版 smoke 路径：`ReachNote QA.app` / `com.reachnote.qa` / 独立 app data。后续用户可见桌面切片优先用 QA app 做 Computer Use PASS，再用正式 app 做发布前检查。

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
- Native status item：Preliminary pass / visual blocked。`src-tauri/Cargo.toml` 已启用 `tauri/tray-icon`，`src-tauri/src/lib.rs` 在 app setup 阶段创建 `reachnote-status-item`；左键恢复主窗口，右键菜单提供 `显示 ReachNote` / `隐藏窗口` / `退出 ReachNote`。验证：`cargo check --manifest-path src-tauri/Cargo.toml`、`pnpm typecheck`、`pnpm build`、`pnpm tauri build --debug --bundles app` 通过；直接运行 debug bundle 内二进制可保持 app 运行。阻塞：`mcp__computer_use.get_app_state` 对 `com.reachnote.app` 被 MCP 拒绝，`screencapture` 报 `could not create image from display`，`open ReachNote.app` 报 LaunchServices `kLSNoExecutableErr`，因此不能声明菜单栏目视 PASS。
- Slice 1 settings/onboarding smoke：Preliminary pass。`pnpm tauri dev` 编译并运行 `target/debug/reachnote-app`；当前 Vite `127.0.0.1:5173` 源码命中 `首次启动检查`、`重新检测`、`4. 快捷键与隐私`、`get_app_settings`、`get_environment_status`、`save_app_settings`。app data SQLite `app_settings` 验证为 `1|claude_cli|article|notion|CommandOrControl+Shift+R|0|570`，环境 JSON 含 Claude CLI、Codex CLI 和 `Agent Reach v1.5.0`。Computer Use 对 `ReachNote` 会启动旧 debug bundle，不能声明当前 dev UI PASS。
- QA installed smoke：PASS。`scripts/desktop-smoke-qa.sh --reset-data` 构建 `target/debug/bundle/macos/ReachNote QA.app`；`Info.plist` 验证 `CFBundleIdentifier=com.reachnote.qa`、`CFBundleName=ReachNote QA`。Computer Use 绑定该 `.app` 后窗口标题 `ReachNote QA`，首屏显示 `首次启动检查`，环境检测显示 Claude CLI / Codex CLI / Agent-Reach，OpenAI-compatible 缺失。
- QA settings smoke：PASS。进入 Settings 后可见 AI provider、Agent-Reach v1.5.0、Notion 空配置、快捷键占位；SQLite `~/Library/Application Support/com.reachnote.qa/reachnote.db` 验证 `notion_settings` count 为 0、`tasks` count 为 0、`app_settings` 环境快照长度 570。正式 `~/Library/Application Support/com.reachnote.app/reachnote.db` 未被重置。
- QA provider persistence smoke：PASS。在 QA Settings 中选择 Codex CLI 后，`app_settings.default_provider_id=codex_cli`；退出并重新打开 `ReachNote QA.app` 后直接进入 Queue，底部状态栏显示 `AI Codex CLI`。
- Notion autofill hardening：PASS。第一次 QA click path 曾因 WebView/password manager 自动填充或页面切换后的二次点击把正式 Notion 配置复制到 QA DB；已清除 `com.reachnote.qa` 隔离数据，并为 Notion token/database inputs 增加 `autoComplete` 与 password-manager ignore attributes。重测后 QA `notion_settings` 保持 0。
- QA updater isolation：PASS。Claude review 提到 QA build 可能继承 updater 并弹窗；真实仓库当前无 updater plugin/endpoints，但 QA config 已显式设置 `plugins.updater.active=false`。复建 QA app 后 Computer Use 仍直接显示 onboarding，无更新弹窗。
- Slice 2 QA platform matrix：PASS。`scripts/desktop-smoke-qa.sh --reset-data` 构建 `ReachNote QA.app`；Computer Use 首次打开时 onboarding 显示 Agent-Reach 自动平台检测 loading，完成后写入 `source_capability_snapshots`。Settings 显示「Agent-Reach 平台能力」15 行，手动点击「刷新平台」按钮进入 disabled/loading，完成后上次检测时间更新、快照 count 增加到 2+。本机真实状态：GitHub ready/read_content、B站 ready/search、RSS ready/read_content、Web ready/read_content、雪球 needs_login，其余多为 needs_install；V2EX/Exa 当前受本机网络/配置影响不是 ready。矩阵在默认 QA 窗口无横向溢出，长 message 省略。
- Slice 2 Capture hint：Partial PASS。Capture 页粘贴 YouTube URL 后显示「检测到：YouTube 视频和字幕 · 需安装 · 暂不支持」，不改 CTA 可点击规则。GitHub URL 的桌面输入路径因 Computer Use/WebView 粘贴反复触发实际提交风险，未继续第三次尝试；GitHub `ready/read_content` 已由 Settings 矩阵和 SQLite normalized snapshot 验证。
- Slice 2 QA cleanup：PASS。桌面验证期间两次误触发的 QA-only capture task 均失败在 reader/network，未写 Notion；验证后已删除所有 QA tasks，`~/Library/Application Support/com.reachnote.qa/reachnote.db` 最终 `tasks=0`、`notion_settings=0`，仅保留平台快照。
- Slice 3 QA template registry：PASS。先停止旧 QA 进程，再跑 `scripts/desktop-smoke-qa.sh --reset-data` 构建隔离 app；SQLite 验证新 QA DB `tasks=0`、`notion_settings=0`、`app_settings.default_template_id=web_article`。Computer Use 打开 `ReachNote QA.app` 后，模板页显示 5 个系统模板，默认 `网页文章笔记`；点击 GitHub「设为默认」后 UI 显示默认，SQLite `default_template_id=github_project`。
- Slice 3 Capture/Queue template path：PASS。采集页模板下拉恢复为 `GitHub 项目分析`，输入公开 `https://github.com/AliceDel66/ReachNote` 后推荐模板显示 GitHub，CTA 可提交。队列页新增「模板」列，任务行显示 `GitHub 项目分析`；SQLite 最新 task 为 `template_id=github_project|source_type=article|provider_id=claude_cli|status=failed|url=https://github.com/AliceDel66/ReachNote`。重启 QA app 后队列仍显示模板列和 `GitHub 项目分析`。
- Slice 3 QA known failure：GitHub QA task 失败在 `GitHub reader 请求失败: error sending request for url (https://api.github.com/repos/AliceDel66/ReachNote)`；这是本机网络/API reader 层失败，不是模板选择、持久化或 reload 问题。验证未配置 Notion，未写第三方 Notion 数据。
- QA screenshot：`/tmp/reachnote-worker-provider-unavailable.png`，用于本轮人工视觉核对；截图不提交仓库。
- QA screenshot：`/tmp/reachnote-analysis-success.png`，用于本轮结构化分析成功路径视觉核对；截图不提交仓库。
- QA screenshot：`/tmp/reachnote-notion-synced-queue.png`，用于本轮 Notion sync UI smoke 视觉核对；截图不提交仓库。
- QA screenshot：`/tmp/reachnote-recovery-queue-after-nav.png`，用于本轮 stale recovery UI smoke 视觉核对；截图不提交仓库。
- QA screenshot：`/tmp/reachnote-opencli-autosynced.png`，用于本轮 orphan analyzed auto-sync 视觉核对；截图不提交仓库。
- Computer Use：QA PASS；`mcp__computer_use.get_app_state` 对 `target/debug/bundle/macos/ReachNote QA.app` 可稳定绑定并返回 screenshot/accessibility tree。裸二进制 `target/debug/reachnote-app` 仍不作为 Computer Use 目标。

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
- Header/缩小切片状态：顶部右侧 icon 优化已在真实 Tauri dev 窗口中人工截图核对；缩小目标按用户澄清改为“隐藏主窗口 + 后台静默运行 + 原生 status item / Dock Reopen 恢复”，不再使用伪 compact bar。最终交互验证受 Computer Use / 截屏 / AX 权限阻断，仍需在权限恢复后补一次真实点击 PASS。
- Status item 补丁：用户反馈“最小化后不像 claude-mem 出现在导航栏，而是直接消失”。根因是旧实现只调用 `window.hide()`，没有创建 macOS status item。现已通过 Tauri tray API 创建系统菜单栏图标，并保留后台常驻进程入口，为下一版全局快捷键一键收录做准备。
- Claude gate：上一轮顶部 icon / 隐藏窗口切片第一轮 FAIL 的 P1 已修复，第二轮 conditional PASS 的 runtime 条件已满足（handler 注册 + `core:window:allow-hide`），因此上一切片 gate 记为 PASS。当前 native status item 切片的 Claude gate 返回 `FailedToOpenSocket`，本切片 gate 记为 Blocked。
- Bugfix 桌面状态：用户截图中停在 `已分析` 的 `OpenCLI` 行已通过 Tauri dev reload 自动补同步为 `已完成`，UI 与 SQLite 一致。Computer Use 仍 `Invalid app`，不能声明 Computer Use PASS。
- Slice 1 桌面状态：当前 dev 二进制可启动，现有安装未被 onboarding 阻断，SQLite settings migration 和环境快照已写入；空库 onboarding 由 `store::tests::new_store_starts_with_onboarding_required` 覆盖。Computer Use 绑定旧 debug bundle 的问题仍需后续通过清理 debug bundle、改 dev bundle id，或安装版验证解决。
- 桌面验证隔离修复：已新增 QA app 配置与脚本。推荐后续命令：`scripts/desktop-smoke-qa.sh --reset-data` 构建干净 QA 安装版；用 Computer Use 绑定 `/Users/yaocheng/Desktop/nexus/rearchnote/target/debug/bundle/macos/ReachNote QA.app`；需要保留 QA 状态时不要加 `--reset-data`。验证后 QA app 已退出，QA 数据目录当前无 Notion 凭证、无任务。
