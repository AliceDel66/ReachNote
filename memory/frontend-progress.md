# Frontend Progress

Last updated: 2026-07-01

## Current Snapshot

状态：Done / Slice 3 template registry UI

旧 `src/` 前端实现已清空后，本轮重新建立 React 18 + Vite + HeroUI 依赖基线，并实现新版 UI 静态壳。当前默认进入 `队列`，导航为 `采集 / 队列 / 模板 / 设置`。队列页已从 mock 常量切到真实 Tauri commands：加载和轮询前先 `invoke("recover_interrupted_tasks")`，再 `invoke("sync_pending_analyzed_tasks")`，最后 `invoke("list_capture_tasks")`；采集页 CTA 先 `create_capture_task`，再后台触发后端完整 `run_capture_task`，分析成功后的 Notion 同步不再依赖前端追加调用。Slice 1 已把 `App.tsx` 拆成页面组件和 shared types/utils，并新增首次启动 onboarding、环境检测状态、provider 持久化和 settings 默认项管理。Slice 2 已在 Settings 接入 Agent-Reach 平台能力矩阵，在 onboarding 首次自动触发一次平台检测，在 Capture 页根据最近快照显示只读来源提示；不改变真实采集读取路由。Slice 3 已把模板从静态展示升级为系统模板选择地基：`TEMPLATES` 使用 PRD canonical IDs，模板页可设默认模板，采集页可选择模板并显示 URL 推荐模板，队列表新增模板列并按 `task.template_id` 显示中文 label。最新 UI/桌面切片优化顶部右侧搜索/设置/隐藏三个 icon button，并把“缩小”从 React 伪导航条改为 `invoke("set_compact_mode", { compact: true })` 隐藏主窗口；Tauri 启动层已创建原生 macOS status item，后台保留 Tauri 进程并提供恢复入口。

## Changed Files

- `package.json`
- `pnpm-workspace.yaml`
- `index.html`
- `tsconfig.json`
- `vite.config.ts`
- `src/main.tsx`
- `src/App.tsx`
- `src/components/AppHeader.tsx`
- `src/components/StatusBar.tsx`
- `src/capture/CaptureView.tsx`
- `src/queue/QueueView.tsx`
- `src/templates/TemplatesView.tsx`
- `src/settings/SettingsView.tsx`
- `src/onboarding/OnboardingView.tsx`
- `src/types.ts`
- `src/constants.ts`
- `src/utils.ts`
- `src/styles.css`
- `src/vite-env.d.ts`

## Verification Status

- `pnpm install`：通过；本机 pnpm supply-chain policy 要求批准 `esbuild` build script，已用 `pnpm approve-builds --all` 批准当前 pending build。
- `pnpm typecheck`：通过。
- `pnpm build`：通过。
- Slice 1 Tauri dev smoke：通过启动；当前 dev server 服务的新源码包含 `首次启动检查`、`重新检测`、`4. 快捷键与隐私`、`save_app_settings` / `get_environment_status` 调用。app data SQLite 已写入 `app_settings` 和环境快照。裸二进制仍不适合作为 Computer Use 目标，后续 UI PASS 改走隔离 `ReachNote QA.app`。
- Slice 2 QA installed platform matrix smoke：通过。`ReachNote QA.app` 首启动自动 doctor 后写入平台快照；Settings 显示「Agent-Reach 平台能力」15 行，手动点击「刷新平台」进入 loading 并刷新上次检测时间；ready/needs_install/needs_login 均可见，无横向溢出。Capture 页 YouTube URL 显示「检测到：YouTube 视频和字幕 · 需安装 · 暂不支持」只读提示；GitHub ready/read_content 在 Settings 矩阵和 normalized snapshot 中验证。
- Slice 3 QA installed template smoke：通过。`ReachNote QA.app` 模板页显示 5 个系统模板，默认 `网页文章笔记`；点击 GitHub 模板「设为默认」后 UI 改为默认且 DB 写入 `github_project`。采集页模板下拉恢复为 `GitHub 项目分析`，输入 GitHub URL 后推荐模板也显示 GitHub。提交公开 GitHub URL 后队列新增「模板」列，任务行显示 `GitHub 项目分析`；重启 QA app 后队列仍显示该模板 label。该任务失败在 GitHub reader 网络请求，不影响模板 UI/持久化结论。
- `cargo check --manifest-path src-tauri/Cargo.toml`：通过。
- `cargo test --manifest-path src-tauri/Cargo.toml`：上一轮完整通过，32 passed / 1 ignored；本轮 status item 复跑受沙盒禁止本地 mock server 监听影响，4 个 HTTP mock 用例 `Operation not permitted`，28 passed / 1 ignored。
- `pnpm tauri build --debug --bundles app --no-sign`：通过；Tauri capability schema 接受 `core:window:allow-hide`，产物在 `target/debug/bundle/macos/ReachNote.app`。
- Tauri dev Accessibility smoke：通过；空 URL CTA disabled，输入 `abc` 后 CTA 仍 disabled；输入 `https://openai.com/index/gpt-4o` 后 CTA enabled，点击后队列页显示真实 `queued` 任务。
- Tauri dev provider_unavailable smoke：通过；使用 `REACHNOTE_CLAUDE_CMD=__missing_claude__ pnpm tauri dev`，采集合法 URL 后队列页显示 `失败`、行内 Claude CLI 缺失原因和 `重试` 按钮。
- Tauri dev structured analysis smoke：通过；使用 fake Claude CLI，采集后队列页显示 `已分析`、结构化标题、4 星评分和 fake model，无明显文本重叠。
- Tauri dev Agent-Reach reader smoke：通过；使用本地 mock Jina Reader + fake Claude CLI，验证时队列行显示 `Reader Content OK`、`已分析`、`fake-claude-reader-check`，与验证时 SQLite row 一致。
- Tauri dev Notion backend startup：通过；`pnpm tauri dev` 可启动真实窗口，app data SQLite `notion_settings` 显示 configured / `2022-06-28`。
- Tauri dev Notion UI smoke：通过 AX fallback；采集页提交非敏感 `example.com` smoke URL 后，前端自动调用 `run_capture_task` 和 `sync_capture_task`，验证时队列行显示 `已完成`、4 星评分、fake model 和 `Notion` 链接。
- Tauri dev stale recovery smoke：通过 AX fallback；真实窗口 reload 后，队列加载触发 `recover_interrupted_tasks`，测试 `reading` row 显示为 `失败`，包含阶段性失败原因和 `重试` 按钮。
- Tauri dev orphan analyzed sync smoke：通过 AX fallback；真实窗口 reload 后，截图中的 `OpenCLI` 行从 `已分析` 变为 `已完成` 并显示 `Notion` 按钮，DB 行 `task-1782878357-900342000-78578-1` 写入 `notion_page_id`。
- 真实后端 E2E：通过；ignored Rust test 使用真实 `AliceDel66/fe-fidelity-kit`、真实 Claude CLI、真实 Notion API，创建 page `390c9b0c-3c3c-81d2-b04d-f0cd5b8859bb`。
- Computer Use：QA installed PASS。`target/debug/bundle/macos/ReachNote QA.app` / `com.reachnote.qa` 可由 Computer Use 稳定绑定；已验证首启动 onboarding、Settings 空 Notion 配置、provider 持久化重启恢复。
- Tauri dev header visual check：Preliminary；真实窗口截图确认右上三个 icon 已替换为更轻的 `Search` / `Settings2` / `Minimize2`，按钮尺寸和描边收敛。截图包含本地 Notion 设置预览，不提交仓库、不外传。
- Shrink-to-background UI click：Blocked；本轮后续 `osascript click` 被 macOS 拒绝辅助访问 `(-25211)`，因此未能完成真实点击隐藏的最终桌面 PASS。代码层已改为调用 Tauri `set_compact_mode(true)` 隐藏主窗口。
- Native status item：Preliminary pass / visual blocked；`tauri/tray-icon` feature 已启用，Tauri setup 创建 `reachnote-status-item`，左键恢复主窗口，右键菜单提供显示/隐藏/退出。`cargo check --manifest-path src-tauri/Cargo.toml`、`pnpm typecheck`、`pnpm build`、`pnpm tauri build --debug --bundles app` 通过；Computer Use / 截屏权限阻断菜单栏目视 PASS。
- Claude review gate：当前 status item 切片 Blocked；Claude CLI 返回 `API Error: Unable to connect to API (FailedToOpenSocket)`。上一轮顶部 icon / 隐藏窗口切片为 PASS，P1（恢复窗口时强制重置尺寸/位置）已修。
- Browser/CDP visual smoke：模板页和设置页在 `1180x780` / `900x680` 无横向溢出；设置页首屏高度合理。注意普通浏览器没有 Tauri invoke 环境，数据流最终以 Tauri dev 为准。

## Progress Log

### 2026-06-30

- 建立最小 React/Vite 前端入口。
- 实现静态页面：队列、采集、模板、设置。
- 根据用户截图反馈收缩默认窗口和 UI 全局尺度：header、导航、表格、按钮、输入框、预览区均从 1448 图稿直译尺寸降到更接近桌面工具的中等密度。
- 采集页改用 native input/textarea/button，以避免 HeroUI Input slot 导致 icon、placeholder 和中文按钮文本错位。
- 修复 logo 前端路径：从 web root 绝对路径改为 Vite PNG import，避免 dev/build 后 404；header alt 改为 `ReachNote`。
- 队列页改为真实本地任务读取，覆盖 loading、empty、error、success；搜索按钮打开本地队列搜索，设置按钮跳转设置页。
- 采集页 CTA 按 URL validity / submitting 状态 disabled；成功创建本地 `Queued` 任务后刷新队列。
- 提交并推送当前基线：`c49c530`，中文 commit `实现本地队列闭环与静态桌面壳`，已推送 `origin/main`。
- 采集页 CTA 成功创建任务后改为触发 `run_capture_task`；worker 返回的 task 会 upsert 到本地 React state，并最终刷新 SQLite 列表。
- 队列行新增 `error_kind/error_message` 展示：失败原因显示在标题下方，`失败` 状态行展示 `重试` 按钮，搜索可命中错误分类和错误文案。
- 队列页说明 banner 改为“最小 worker 已启用；当前仅检查 Claude CLI 可用性，不读取网页、不同步 Notion”。
- 未闭环风险：当前没有真实分析结果和 Notion 写入，队列行 title 仍以 URL fallback 显示；普通浏览器预览因无 Tauri invoke 会进入错误态；Claude CLI 可用时任务会停在 `Analyzing` 等待后续分析切片。

### 2026-07-01

- 采集页 AI 提供方从静态 button 改为可选择的 native select：`Claude CLI`、`Codex CLI`、`OpenAI-compatible API`。
- 设置页 AI 提供方从“计划中”静态行改为可点击 radio-like 行，并与采集页共享当前会话 provider 状态。
- 底部状态栏显示当前 provider。
- 队列页新增 `analyzed` 状态支持，显示为绿色 `已分析`；Done filter 同时包含 `analyzed` 和 `synced`。
- 未闭环风险：provider 选择当前只保存在 React session state，尚未持久化到 app data/config；OpenAI-compatible API key 不在 UI 输入，当前只读环境变量，后续应接 OS keychain。
- 队列页说明 banner 改为 Agent-Reach web 读取已启用；设置页 Agent-Reach 状态文案标记 `Jina Reader` / `Agent-Reach web route`，与当前后端 reader 入口一致。
- 采集页分析成功后自动接 `sync_capture_task`；`synced` 行显示 Notion 链接，失败行重试逻辑根据是否已有 `analysis_json` 决定重跑分析或重试同步。
- 设置页 Notion 卡从计划态改为真实本地配置：加载 masked token/database、保存 token/database、测试连接；首次保存 token 必填，之后 token 留空表示保留旧 token。
- 当前 UI 自动化风险：Computer Use 对 debug app 仍 `Invalid app`；AX `click at` 会执行但抛错，需要 `try` 包裹点击才能继续键盘输入和 CTA。该 fallback 已完成本轮 Notion smoke，但后续发布前仍应恢复 Computer Use 或安装版验证。
- 队列恢复切片：`loadTasks` / `refreshTasks` 在读取任务列表前调用 `recover_interrupted_tasks`，处理中任务轮询保持 1.2 秒节奏；失败行 `重试` 统一调用 `retry_capture_task`，避免前端复制“重跑分析 vs 重试同步”的业务判断。
- 当前可见状态：stale `reading/analyzing/syncing` 会在刷新队列后变成红色 `失败` 行，行内显示后端返回的阶段性恢复原因并保留 `重试` 入口；队列 banner 已改为“处理中任务超时会恢复为失败并保留可重试入口”。
- 顶部栏优化：右上搜索/设置/隐藏按钮改为统一 22px icon 尺寸、细描边渐变按钮和 hover/focus 状态；隐藏按钮 aria/title 改为 `隐藏到系统菜单栏`。
- 缩小行为修正：移除 React `CompactBar` 和相关 compact CSS，缩小不再渲染黑色小窗口；前端只触发 Tauri command，真实隐藏/恢复由 `src-tauri/src/lib.rs` 负责。Tauri capability 已补 `core:window:allow-hide`。
- Bugfix：队列加载/静默刷新新增 `sync_pending_analyzed_tasks`；`handleRunTask` 不再在前端二次调用 `sync_capture_task`，因为后端 `run_capture_task` 已负责 `Analyzed -> Syncing -> Synced/Failed`。这修复了窗口 reload/HMR 后任务停在 `已分析` 的问题。
- Native status item 补丁：用户指出隐藏后没有像 claude-mem 一样出现在 macOS 右上角，而是直接消失。根因是旧实现只 `window.hide()`，没有注册原生 status item；现已在 Tauri setup 阶段创建 `reachnote-status-item`，用于后台常驻恢复入口。
- Slice 1 前端拆分：`App.tsx` 从 1200+ 行页面实现收敛为 setup/load/persist/queue/capture 编排；`QueueView`、`CaptureView`、`TemplatesView`、`SettingsView`、`OnboardingView` 和 header/status bar 拆到独立目录；共享类型、模板常量、provider label、URL 校验、queue row mapper 拆到 `types/constants/utils`。
- Slice 1 onboarding/settings：启动时先读取 `get_app_settings` 与 `get_environment_status`；`onboarding_completed=false` 渲染首次启动检查页；既有安装正常进入 Queue。Settings 页展示 AI provider 可用性、Agent-Reach 安装/版本、Notion 连接、快捷键占位；provider 选择通过 `save_app_settings` 持久化，采集页沿用该 provider。
- Desktop QA 修复：新增 QA bundle 后，Settings 页 Notion token/database 输入加 `autoComplete` 与 password-manager ignore attributes，防止 WebView 自动填充把正式 Notion 凭证带进隔离 smoke 数据。重测 QA `notion_settings` count 为 0。
- Slice 2 Settings：新增平台矩阵 loading/empty/error/success 状态；`run_agent_reach_doctor` 成功后重新拉取 `get_environment_status` 读取最新快照。矩阵行显示 name/key、availability pill、active backend、action 标签和截断 message，完整 message 放在 title。
- Slice 2 Capture：新增 `sourcePlatformKeyForUrl` 与只读提示，不强制 disable CTA，不改变现有 `create_capture_task -> run_capture_task` 链路。桌面 QA 中 YouTube 提示已验证；GitHub 输入在 Computer Use 粘贴路径上反复触发提交风险，因此以 Settings 矩阵和 snapshot 验证 GitHub `ready/read_content`，未再冒险做第三次提交。
- Slice 3 Templates/Capture/Queue：新增 `TemplateId`、`normalizeTemplateId`、`templateForSourcePlatformKey` 和 `templateLabel`；`App.tsx` 持有 `selectedTemplateId`，用户选择模板时调用 `save_app_settings(defaultTemplateId)`，创建任务时传 `templateId`。Capture 页使用 URL key 显示推荐模板；Queue 行新增模板列并把模板 label 纳入搜索。

### 2026-07-02

- Slice A queue observer：`App.tsx` 启动任务列表时先注册 `listen<Task>("task:updated")` 和 `listen<string>("worker:error")`，再调用一次 `list_capture_tasks`。已删除前端启动/刷新中的 `recover_interrupted_tasks`、`sync_pending_analyzed_tasks` 和 1.2 秒三 invoke 轮询；现在只保留 30 秒 `list_capture_tasks` 兜底和 window focus refresh。
- Slice A submit/retry 行为：采集提交后只 `create_capture_task` 并切回 Queue，不再 `handleRunTask(createdTask.id)`；`retry_capture_task` 只接收后端 CAS reset 返回并等待 worker event。Queued 行新增 `立即处理` inline fallback，调用 `run_capture_task`，claim 失败由后端返回当前任务而非前端报错。
- Slice A merge 规则：`upsertTask` / `mergeTaskList` 按 `id` 合并，并用 `updated_at` 新旧决定是否覆盖；queued 状态文案改为“等待处理”。
- Slice A review P2 fix：`handleRetryTask` 改为 functional state update，不再闭包依赖 `tasks`；`handleRunTask` 点击“立即处理”后对 queued 行乐观显示 `reading`，等待后端 CAS/worker event 校正。
- Slice A 桌面 QA：隔离 `ReachNote QA.app` / `com.reachnote.qa` 中，Computer Use 创建公开 `https://example.com/reachnote-slice-a-worker-smoke` 任务。队列先显示 `读取中`，随后显示 `分析中`，最终显示红色 `失败`、行内 Notion 未配置原因、`重试` 按钮；DB 同步显示 `failed/notion_unauthorized` 且 `analysis_json IS NOT NULL`。点击重试后仍由 worker 回到同一失败态。
- Slice A 验证：`pnpm typecheck`、`pnpm build` 通过；`scripts/desktop-smoke-qa.sh --reset-data` 构建 QA app 通过，Computer Use 可绑定 QA app。
