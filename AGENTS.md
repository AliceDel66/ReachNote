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
3. 当前开发记忆：`memory/README.md` 与 `memory/*-progress.md`。
4. UI 设计源：`memory/design-source.md` 中登记的用户提供设计图、截图、Figma 导出或设计说明。
5. 产品边界：`docs/discussions/mvp-prd-information-architecture.md`。
6. 技术选型：`docs/adr/0001-tech-stack.md`。
7. 当前代码：`src/`、`src-tauri/`、`crates/core/`。
8. `README.md` / `README.zh-CN.md`。

注意：README 中有 planned / in progress 内容。声称功能已实现前，必须回到代码或运行结果验证。

## Memory Protocol

为了避免长开发周期后产生幻觉，任何 agent 开始非平凡任务前必须先读：

- `memory/README.md`
- 与任务相关的 `memory/frontend-progress.md`、`memory/backend-progress.md`、`memory/integration-progress.md`
- 涉及 UI 时必须读 `memory/design-source.md`

每完成一个可验证开发切片，必须在同一轮更新相关 memory 文件，至少记录：

- 当前状态：Done / In Progress / Blocked / Planned。
- 实际改动文件。
- 已验证命令或人工检查结果。
- 未闭环风险、阻塞和下一步入口。
- 日期使用 `YYYY-MM-DD`。

Memory 文件只记录事实和可追踪决策，不记录猜测、完整私密正文、真实 token、API key、Cookie、个人账号信息或未验证的外部状态。

如果 memory 与代码冲突，以当前代码和验证结果为准，并立即修正 memory。

## Design Fidelity Protocol

后续用户提供前端 UI 设计图后，设计图是前端实现的强约束，不允许自行重设计。

- 先把设计来源登记到 `memory/design-source.md`，包括文件路径、日期、覆盖页面、视口尺寸和开放问题。
- 实现前先拆解视觉规格：布局、间距、字号、颜色、圆角、阴影、图标、状态、响应式规则。
- 前端实现必须优先匹配设计图，再考虑代码便利性。
- 与设计图不一致的地方必须在 `memory/design-source.md` 写明原因和用户确认状态。
- 用户可见 UI 改动必须实际渲染验证，检查桌面和窄屏、console error、文字溢出、重叠、断图和关键交互状态。

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
- 最终呈现形态是桌面导航栏 / 菜单栏常驻应用，不是网页应用；所有用户可见功能必须以 Tauri 桌面运行结果为准。
- 优先复用 HeroUI + Tailwind 现有风格。不要引入新 UI 框架、设计语言或装饰型 landing page，除非任务明确要求。
- 常用页面要覆盖 loading、empty、success、error、disabled、retry 等状态。
- 前端文字不能溢出按钮、卡片、表格或窄屏容器。
- 桌面可见体验变更，需要实际运行 Tauri app 并检查真实窗口 / 菜单栏 / 托盘 / 系统权限表现；仅静态读代码、headless、Vite 网页或截图不算完成。
- 每个用户可见功能开发完成后，必须主动调用 `@电脑`（`plugin://computer-use@openai-bundled`）直接控制本机桌面应用完成交互验证；浏览器/Vite 只能作为开发期辅助验证，不能作为最终 PASS 依据。
- 通过 Computer Use 操作本机 UI 时，若下一步会安装软件、创建/授权账号、上传文件、改权限、删除数据、写入第三方服务或传输敏感数据，必须按 `computer-use` 技能要求在动作发生前向用户确认。

## Verification

按改动风险选择验证。常用入口：

- 前端类型检查：`pnpm typecheck`。
- 前端构建：`pnpm build`。
- 前端本地预览：`pnpm dev`，仅作辅助 UI 调试，不作为最终验收。
- Tauri 开发运行：`pnpm tauri dev`，可作桌面 runtime 预检。
- 桌面安装/发布验证：使用 Tauri bundler 产物或已安装 app；最终验收必须测试安装后的桌面结果。
- Rust core 测试：`cargo test -p reachnote-core`。
- Rust core 快速检查：`cargo check -p reachnote-core`。
- Tauri crate 检查：`cargo check --manifest-path src-tauri/Cargo.toml`。
- 桌面真实验证：`@电脑`（`plugin://computer-use@openai-bundled`）操作本机 Tauri app / 已安装 app，检查菜单栏/托盘入口、窗口打开关闭、tab/导航、表单输入、快捷入口、错误态、文本溢出、重叠、断图和系统级权限/弹窗。

规则：

- 只改 docs 可不跑全量构建，但要至少检查文件存在、链接/命令没有明显错写。
- 改 `crates/core/` 必须跑 `cargo test -p reachnote-core`。
- 改 `src/` 必须跑 `pnpm typecheck`；用户可见 UI 改动还要用 `@电脑` 在 Tauri 桌面 app 中验证。
- 改 `src-tauri/` 必须跑对应 Cargo check；涉及 command 契约还要跑前端 typecheck，并用 `@电脑` 验证桌面 runtime 行为。
- 改 external adapters、secrets、queue、Notion sync 时，要补针对失败路径的测试或可重复验证命令。
- 任一用户可见功能切片完成后，必须在进入 Claude review gate 前完成 `@电脑` 桌面验证，并把 app 形态（`pnpm tauri dev` / 已安装 app / 打包产物）、操作路径、截图/观察结果、系统弹窗、权限状态和失败信息写入相关 `memory/*-progress.md` 或 `desktop-qa.md`。如果 Computer Use 不可用，必须把该状态标为 Blocked，而不是当作验证通过。
- 如果只能完成浏览器/Vite 验证，结果只能标为 Preliminary，不能标为 PASS。

## Claude Read-only Review Gate

每个开发板块 / phase / 可验证切片完成后，必须进入 Claude CLI 只读审阅 gate。Codex 负责实现和修复，Claude 只负责审阅，不改文件。

Gate 顺序：

1. Codex 完成实现，更新相关 `memory/`。
2. Codex 跑本切片要求的本地验证。
3. Codex 主动调用 `@电脑` 做桌面应用验证；用户可见功能至少检查菜单栏/托盘入口、窗口导航、关键交互、系统弹窗、安装后运行状态和视觉布局。
4. Codex 准备 review packet：任务目标、改动文件、验证结果、`@电脑` 桌面验证结果、`git diff --stat`、相关 diff、风险说明。
5. 调用 Claude CLI 做只读审阅。
6. Claude 输出 P0 / P1 / P2 findings 和最终 gate verdict。
7. 如果有 P0 或 P1，Gate = FAIL，Codex 必须修复后重新验证、重新审阅。
8. 如果没有 P0/P1，Gate = PASS；P2 只记录为后续优化，不阻塞当前切片。

Severity 规则：

- P0：数据丢失、凭证泄漏、安全/隐私违规、构建或核心链路不可用、会阻止用户完成主流程的严重问题。必须修。
- P1：must fix / gate blocker；会造成错误行为、可靠复现的崩溃、契约破坏、关键状态丢失、明显 UI 阻塞或测试缺口。必须修。
- P2：非阻塞改进、可维护性、轻微 UX、命名、局部重构建议。记录即可，不阻塞 PASS。

Claude CLI 默认只读命令形态：

```bash
claude -p --output-format text --disable-slash-commands --safe-mode --model sonnet --tools "" --no-session-persistence "$REVIEW_PROMPT"
```

要求：

- 不给 Claude 写权限，不让它执行 edit/commit/push。
- 优先把 diff 和必要上下文放入 prompt；如果上下文太大，缩小到当前切片文件。
- Claude timeout、no output、输出无法判断 gate verdict，都不是 PASS，必须报告为 blocked 并重试或缩小 review packet。
- Review 结果必须写入 `memory/review-gate.md` 和相关 progress memory。
- Codex 修复 P0/P1 后必须重新跑验证和 Claude gate，直到 PASS 或明确阻塞。

## Git And Release Hygiene

- 提交前检查 `git status --short --branch` 和 intentional diff。
- 不使用 `git add .`，除非本次任务明确要纳入全部变更。
- commit 只在用户明确要求时做；给这个用户提交时使用中文 commit message。
- push 只在用户明确要求时做。`AGENTS.md` 和 `memory/` 可以先作为本地工作协议持续完善，不默认推送。
- push 前确认 remote、branch、secret scan 和验证结果。不要根据旧记忆假设 GitHub remote 已绑定或已推送。

## Reporting

完成报告优先说明：

- 结论。
- 实际改了什么文件。
- 为什么这样切。
- 跑了哪些验证，哪些没有跑。
- 剩余风险或下一刀。只有存在真实未闭环点时才写 `下一刀`。
