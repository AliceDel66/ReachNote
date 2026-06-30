# ReachNote Agent Rules

本文件是 ReachNote 仓库的项目级 agent.md。当前用户请求和更高优先级系统规则优先；本文件用于约束本仓库内的开发、评审、文档和验证工作。

## 默认协作方式

- 默认使用中文沟通，技术名词保持英文。
- 目标是完成具体工程任务：先读真实代码和文档，再做最小 coherent change，最后验证并报告实际状态。
- 非平凡工程工作必须先做 P1/P2/P3：架构地图、真实链路追踪、设计决策。小任务可内化，架构/bug/共享契约/安全/部署相关任务要显式报告。
- 仓库存在 `.codegraph/` 时，理解或定位代码必须先用 CodeGraph，再用 `rg` 或 targeted read 补充。
- 允许工作树是 dirty。改动前先看 `git status --short --branch`，不要回滚用户或其他 agent 的改动。
- 手工编辑文件用 patch-style edits，避免无关格式化、依赖升级和元数据 churn。

## Source Of Truth

优先级从高到低：

1. 用户当前明确请求。
2. 本文件和 repo-local docs。
3. 产品边界：`docs/discussions/mvp-prd-information-architecture.md`。
4. 技术选型：`docs/adr/0001-tech-stack.md`。
5. 当前代码：`src/`、`src-tauri/`、`crates/core/`。
6. `README.md` / `README.zh-CN.md`。

注意：README 中有 planned / in progress 内容。声称功能已实现前，必须回到代码或运行结果验证。

## Product Boundary

ReachNote 的定位是：

```text
Capture -> Read -> Analyze -> Structure -> Sync to Notion
```

它是本地优先的桌面 AI 信息采集工具，把互联网内容转成可复用的 Notion research cards。

必须保持的边界：

- Agent-Reach 负责“看见互联网”：渠道选型、安装、体检、路由和内容读取。
- ReachNote 负责“沉淀到个人 Notion 知识库”：捕获、分析、结构化、同步、状态管理。
- 首版目标用户优先服务开发者 / AI 工具研究者。
- P0 内容源优先 GitHub repo、普通网页、YouTube 字幕、RSS / 文章链接。
- 首屏产品方向应优先 History / Queue，而不是 dashboard 或 marketing landing page。

不要把 ReachNote 改写成：

- 又一个收藏夹。
- 完整知识库产品。
- 通用 Notion 写作工具。
- 完整 RAG 系统。
- 大规模社交平台采集工具。
- 云端内容数据库或团队协作知识库。

首版暂不把 Twitter/X、小红书、Reddit、B 站深度采集作为主路径；这些来源涉及登录态、Cookie、风控和账号风险。

## Architecture Boundary

当前技术栈已定板：

- App shell：Tauri 2。
- Frontend：React 18 + TypeScript + Vite。
- UI：HeroUI + Tailwind CSS。
- Core：Rust `reachnote-core` crate，保持无 Tauri 依赖。
- Capabilities：spawn `agent-reach`、`claude`、`codex`，或调用 OpenAI-compatible API。
- Planned persistence：SQLite 本地队列、历史记录、失败重试。
- Planned secret storage：OS keychain，不明文裸存 token 或 API key。

当前 P0 竖切：

```text
src/App.tsx handleCapture
-> invoke("capture", args)
-> src-tauri/src/lib.rs capture
-> detect_source / optional AgentReach::read
-> build_provider
-> AiProvider::analyze
-> AnalysisResult
-> UI rendered research card
```

修改这条链路时，必须同时检查前端 `AnalysisResult`、Tauri `CaptureArgs`、Rust `AnalysisRequest` / `AnalysisResult` 的契约是否一致。

## Module Boundaries

- `src/`：只负责 UI、用户输入、本地交互状态和调用 typed Tauri commands。不要在前端直接保存 secrets、直接调 Notion API，或塞入核心业务编排。
- `src-tauri/`：负责 Tauri commands、托盘/菜单栏、OS 能力、权限、应用生命周期，以及把 UI 请求接到 `reachnote-core`。
- `crates/core/`：负责 AI provider、Agent-Reach adapter、结构化分析类型、模板/prompt、未来的 Notion mapping 和 queue 领域逻辑。保持可独立 `cargo test -p reachnote-core`。
- 外部系统 adapter 必须隔离：Agent-Reach、AI provider、Notion API、SQLite/keyring 不要互相泄漏实现细节。
- 错误信息必须面向用户可读，尤其是缺少 CLI、PATH 错误、provider JSON 解析失败、Notion 权限失败、队列重试失败。

## External Capability Rules

### Agent-Reach

- 不要在 ReachNote 内重新实现通用网页/社交平台抓取。
- `agent-reach doctor` 是 Onboarding / Settings 的体检入口。
- `agent-reach read <url>` 目前是占位假设；对齐真实 Agent-Reach CLI 前，不要把该命令写成已确认事实。

### AI Providers

- 三种 provider 必须共享同一套结构化输出契约：`title`、`summary`、`key_points`、`tags`、`score`、`next_action`、`model`。
- Claude CLI / Codex CLI / OpenAI-compatible API 的差异只允许存在于 adapter 层。
- 模型输出必须按 JSON 解析和校验；不要依赖 Markdown 片段或自然语言兜底作为长期方案。
- 不要把用户 prompt、正文、API key、token 写入日志、README、issue 或测试快照。

### Notion

- 默认方向是写入用户绑定的 Notion database，字段映射来自 PRD。
- Notion OAuth、database selection、field mapping、retry/idempotency 要按本地任务生命周期设计。
- 首版不做 Notion 双向同步，不做复杂模板编辑器。

## Security And Privacy Boundary

- 本项目默认 local-first。除用户配置的 AI provider、Agent-Reach 读取目标和 Notion API 外，不引入中间服务器。
- 真实 Notion token、OpenAI-compatible API key、Claude/Codex session、Cookie、个人内容样本不得提交。
- 配置样例只能使用 fake placeholder。
- 涉及 secrets 的实现优先 OS keychain；临时开发配置必须进 `.gitignore` 覆盖的本地文件或环境变量。
- 调试日志要可定位问题，但不能包含完整正文、凭证或个人私密内容。

## UI And Product Quality

- UI 是工具型桌面产品：密度清晰、层级克制、适合反复处理队列和失败重试。
- 优先复用 HeroUI + Tailwind 现有风格。不要引入新 UI 框架、设计语言或装饰型 landing page，除非任务明确要求。
- 常用页面要覆盖 loading、empty、success、error、disabled、retry 等状态。
- 前端文字不能溢出按钮、卡片、表格或窄屏容器。
- 浏览器或 Tauri 可见体验变更，需要实际渲染检查；仅静态读代码不算完成。

## Verification

按改动风险选择验证。常用入口：

- 前端类型检查：`pnpm typecheck`。
- 前端构建：`pnpm build`。
- 前端本地预览：`pnpm dev`。
- Tauri 开发运行：`pnpm tauri dev`。
- Rust core 测试：`cargo test -p reachnote-core`。
- Rust core 快速检查：`cargo check -p reachnote-core`。
- Tauri crate 检查：`cargo check --manifest-path src-tauri/Cargo.toml`。

规则：

- 只改 docs 可不跑全量构建，但要至少检查文件存在、链接/命令没有明显错写。
- 改 `crates/core/` 必须跑 `cargo test -p reachnote-core`。
- 改 `src/` 必须跑 `pnpm typecheck`；用户可见 UI 改动还要实际渲染。
- 改 `src-tauri/` 必须跑对应 Cargo check；涉及 command 契约还要跑前端 typecheck。
- 改 external adapters、secrets、queue、Notion sync 时，要补针对失败路径的测试或可重复验证命令。

## Git And Release Hygiene

- 提交前检查 `git status --short --branch` 和 intentional diff。
- 每次开始新的更新或迭代前，先提交当前已验证状态，commit message 使用中文，避免后续改动覆盖可回退基线。
- 不使用 `git add .`，除非本次任务明确要纳入全部变更。
- commit 只在用户要求时做；给这个用户提交时使用中文 commit message。
- push 前确认 remote、branch、secret scan 和验证结果。不要根据旧记忆假设 GitHub remote 已绑定或已推送。

## Reporting

完成报告优先说明：

- 结论。
- 实际改了什么文件。
- 为什么这样切。
- 跑了哪些验证，哪些没有跑。
- 剩余风险或下一刀。只有存在真实未闭环点时才写 `下一刀`。
