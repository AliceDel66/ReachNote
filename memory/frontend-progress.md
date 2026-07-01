# Frontend Progress

Last updated: 2026-07-01

## Current Snapshot

状态：Done / Queue auto-synces orphan analyzed tasks

旧 `src/` 前端实现已清空后，本轮重新建立 React 18 + Vite + HeroUI 依赖基线，并实现新版 UI 静态壳。当前默认进入 `队列`，导航为 `采集 / 队列 / 模板 / 设置`。队列页已从 mock 常量切到真实 Tauri commands：加载和轮询前先 `invoke("recover_interrupted_tasks")`，再 `invoke("sync_pending_analyzed_tasks")`，最后 `invoke("list_capture_tasks")`；采集页 CTA 先 `create_capture_task`，再后台触发后端完整 `run_capture_task`，分析成功后的 Notion 同步不再依赖前端追加调用。最新 UI 切片优化顶部右侧搜索/设置/隐藏三个 icon button，并把“缩小”从 React 伪导航条改为 `invoke("set_compact_mode", { compact: true })` 隐藏主窗口，后台保留 Tauri 进程。

## Changed Files

- `package.json`
- `pnpm-workspace.yaml`
- `index.html`
- `tsconfig.json`
- `vite.config.ts`
- `src/main.tsx`
- `src/App.tsx`
- `src/styles.css`
- `src/vite-env.d.ts`

## Verification Status

- `pnpm install`：通过；本机 pnpm supply-chain policy 要求批准 `esbuild` build script，已用 `pnpm approve-builds --all` 批准当前 pending build。
- `pnpm typecheck`：通过。
- `pnpm build`：通过。
- `cargo check --manifest-path src-tauri/Cargo.toml`：通过。
- `cargo test --manifest-path src-tauri/Cargo.toml`：通过，32 passed / 1 ignored。
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
- Computer Use：Blocked；插件无法绑定 debug app。本轮可见 UI 验证依赖 macOS Accessibility fallback，不是 Computer Use PASS。
- Tauri dev header visual check：Preliminary；真实窗口截图确认右上三个 icon 已替换为更轻的 `Search` / `Settings2` / `Minimize2`，按钮尺寸和描边收敛。截图包含本地 Notion 设置预览，不提交仓库、不外传。
- Shrink-to-background UI click：Blocked；本轮后续 `osascript click` 被 macOS 拒绝辅助访问 `(-25211)`，因此未能完成真实点击隐藏的最终桌面 PASS。代码层已改为调用 Tauri `set_compact_mode(true)` 隐藏主窗口。
- Claude review gate：PASS；第一轮 P1（恢复窗口时强制重置尺寸/位置）已修，第二轮 conditional PASS 的两个条件已满足：`set_compact_mode` 已在 `generate_handler!` 注册，`src-tauri/capabilities/default.json` 已包含 `core:window:allow-hide`。
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
