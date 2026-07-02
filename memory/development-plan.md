# Development Plan

Last updated: 2026-06-30

## Current Context

状态：P0 context calibrated / implementation reset.

当前 `src/`、`src-tauri/`、`crates/core/` 无实现源码。`src-tauri/target/` 里仍有历史构建产物，但它们不是当前源码，也不能作为可运行实现依据。CodeGraph 本轮对 `src` / `src-tauri` / `crates/core` 未找到现行实现符号；文件系统排除 `target/` 后也未列出源码文件。

UI source of truth 已切到仓库内 `assets/ui/`：

- `assets/ui/ChatGPT Image 2026年6月30日 19_12_36 (2).png`：队列。
- `assets/ui/ChatGPT Image 2026年6月30日 19_12_34 (1).png`：采集。
- `assets/ui/ChatGPT Image 2026年6月30日 19_12_36 (3).png`：模板。
- `assets/ui/ChatGPT Image 2026年6月30日 19_12_36 (4).png`：设置。

## Official Docs Calibration

- Tauri 2 可以用 `create-tauri-app` 建 React/TypeScript 项目，也可以先建 Vite 前端再 `pnpm tauri init` 生成 `src-tauri/`。
- Tauri + Vite 配置应保持 `devUrl: "http://localhost:5173"`、`frontendDist: "../dist"`；Vite `server.port` 要固定 `5173`、`strictPort: true`，并忽略监听 `src-tauri`。
- HeroUI v2 Vite 文档要求 Vite 2+、React 18+、Tailwind CSS v4、Framer Motion 11.9+；安装后需要在应用根部包 `HeroUIProvider`。文档页同时提示新项目推荐 HeroUI v3，若后续仍坚持 v2，应在 ADR 中显式保留这个选择。
- Tauri clipboard manager JS API 可用：`readText(): Promise<string>` 可读取剪贴板纯文本，适合采集页“从剪贴板粘贴”。
- Tauri global shortcut 插件可用：通过 `pnpm tauri add global-shortcut` 安装，JS 侧可 `register("CommandOrControl+Shift+C", handler)`；P0 不应抢先实现，可作为 P1/P2。
- Notion public connection 使用 OAuth 2.0：跳转授权 URL，回调带 `code`，再 `POST /v1/oauth/token` 换 `access_token` / `refresh_token`，凭证必须进入 OS keychain。
- Notion 创建页面接口是 `POST https://api.notion.com/v1/pages`；当前文档强调 parent 可为 `page_id` 或 `data_source`，写入 data source 时 `properties` key 必须匹配父 data source 的属性。实现前不要只按旧 `database_id` 术语硬编码。

## First Slice

第一刀只恢复工程骨架，不实现完整业务闭环：

1. 用 Tauri 2 + React 18 + TypeScript + Vite + HeroUI 建立最小可运行桌面 app。
2. 恢复目录边界：`src/` 前端壳、`src-tauri/` Tauri 壳、`crates/core/` 空 core crate 与基础测试。
3. 实现静态新版 UI 壳：`队列 / 采集 / 模板 / 设置`、默认进入 `队列`、空队列和不可用状态文案。
4. 仅接入安全的本地能力探针：剪贴板按钮可读取纯文本并填入 URL 输入框；Notion、Agent-Reach、Claude CLI 只显示未连接/待检测状态，不调用真实外部服务。

## Verification Commands

第一刀完成后最低验收命令：

```bash
pnpm install
pnpm typecheck
pnpm build
cargo test -p reachnote-core
cargo check --manifest-path src-tauri/Cargo.toml
pnpm tauri dev
```

用户可见 UI 还必须按项目规则做桌面 runtime 验证：真实打开 Tauri 窗口，检查队列默认页、四个导航 tab、采集页剪贴板按钮、空/disabled/error 状态、console error、文字溢出和布局重叠。

## Next Phase PRD

Last updated: 2026-07-01

状态：Draft PRD ready for Claude review.

新增下一阶段 PRD：

- `plans/prds/20260701-1447-reachnote-next-phase-platform-template-destinations-onboarding-shortcuts.prd.md`

PRD 范围：

1. Agent-Reach 支持平台能力矩阵：以 `agent-reach doctor --json` 为 truth surface，覆盖 `github/twitter/youtube/reddit/facebook/instagram/bilibili/xiaohongshu/linkedin/xiaoyuzhou/v2ex/xueqiu/rss/exa_search/web`，但按 ready / needs_config / needs_login / off 分批接入，不承诺所有平台同等深度一次完成。
2. 模板系统：先做系统模板和模板选择，模板驱动 prompt 与目标字段映射，不做复杂自定义编辑器。
3. 多目的地同步：Notion 保留为首个 adapter；飞书、企业微信、钉钉第一版限定为 webhook/机器人消息，不承诺完整数据库/文档 API 集成。
4. 首次启动引导：检测 Claude CLI、Codex CLI、OpenAI-compatible 配置和 Agent-Reach doctor，推荐默认 AI provider，引导选择并测试 destination。
5. 全局快捷键：基于 Tauri global-shortcut，从剪贴板 URL 一键入队，依赖后台常驻/隐藏窗口能力。

第一刀建议：`settings/onboarding/environment check`。理由是当前 provider 选择仍在 React session，模板和 destination 也没有持久化；不先建立 settings 地基，后续平台、模板、多目标和快捷键都会互相阻塞。
