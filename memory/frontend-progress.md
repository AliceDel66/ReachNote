# Frontend Progress

Last updated: 2026-06-30

## Current Snapshot

状态：Done / Local queue UI wired to minimal worker

旧 `src/` 前端实现已清空后，本轮重新建立 React 18 + Vite + HeroUI 依赖基线，并实现新版 UI 静态壳。当前默认进入 `队列`，导航为 `采集 / 队列 / 模板 / 设置`。队列页已从 mock 常量切到 `invoke("list_capture_tasks")`，采集页 CTA 已接 `invoke("create_capture_task")`；成功创建任务后自动回到队列并触发 `invoke("run_capture_task")`。缺少 Claude CLI 时，队列页显示 `失败`、错误原因和重试按钮。模板页和设置页已按用户截图反馈收缩到更合理的桌面密度，顶部搜索/设置按钮可点击。

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
- Tauri dev Accessibility smoke：通过；空 URL CTA disabled，输入 `abc` 后 CTA 仍 disabled；输入 `https://openai.com/index/gpt-4o` 后 CTA enabled，点击后队列页显示真实 `queued` 任务。
- Tauri dev provider_unavailable smoke：通过；使用 `REACHNOTE_CLAUDE_CMD=__missing_claude__ pnpm tauri dev`，采集合法 URL 后队列页显示 `失败`、行内 Claude CLI 缺失原因和 `重试` 按钮。
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
