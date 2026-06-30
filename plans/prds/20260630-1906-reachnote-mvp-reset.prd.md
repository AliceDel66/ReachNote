# ReachNote MVP Reset PRD

> **Status**: Draft  
> **Date**: 2026-06-30  
> **Owner**: TBD  
> **Source Inputs**: `README.md`, `README.zh-CN.md`, `docs/discussions/mvp-prd-information-architecture.md`, `docs/adr/0001-tech-stack.md`, `memory/design-source.md`, CodeGraph audit before implementation reset

## AI Quick-Read Card

| Field | Content |
| --- | --- |
| Product | ReachNote |
| Thesis | ReachNote 是开发者和 AI 工具研究者的本地优先桌面采集工作台，把值得研究的链接转成可追踪、可评分、可同步到 Notion 的结构化研究卡。 |
| First User | 开发者、AI 工具研究者、需要持续跟踪工具/项目/技术文章的个人用户。 |
| First Version | 桌面窗口里的 `队列 / 采集 / 模板 / 设置`，完成一条 `文章 URL -> 本地队列 -> AI 研究卡 -> Notion 同步 -> 队列状态` 的真实闭环。 |
| Default Screen | `队列`，因为用户最需要知道采集任务是否成功、失败在哪里、能否重试。 |
| Hard Non-goals | 不做社交平台深采、不做完整知识库、不做 RAG、不做团队协作、不做复杂模板编辑器、不自建云端中转服务。 |
| Validation Bar | 真实运行、真实页面、真实队列状态、真实 Notion 写入；不能只靠 mock 或构建通过。 |

## Direction Framing

### Thesis

ReachNote 的价值不是“收藏链接”，而是让用户看到内容后的下一步变轻：一条链接进入本地队列，被可靠读取、结构化分析、同步到 Notion，并留下可重试的处理记录。

### High-Level Direction

第一版应做成桌面工作台，不是 marketing page，也不是旧版小 popover。新版 UI 的主形态是一个宽桌面窗口，围绕队列、采集、模板、设置四个区域工作。产品优先级应从“任务是否处理完成”倒推，而不是从“支持多少平台”发散。

### Bold Takes

- `队列` 是默认入口，不是附属页面。
- 第一版必须跑通 Notion 同步，否则 ReachNote 只是一个本地 AI 摘要器。
- 模板第一版只做系统模板，不做自定义模板编辑。
- Agent-Reach 是外部读取能力，不在 ReachNote 内重写网页/社交平台抓取。
- 本地优先是产品承诺，凭证必须进入 OS keychain，队列必须在本地持久化。

### What Not To Do

- 不把 ReachNote 做成又一个收藏夹。
- 不把 Notion 写作、双向同步、团队知识库、RAG 搜索塞进第一版。
- 不把 Twitter/X、小红书、Reddit、B 站深度采集作为首版主路径。
- 不为“看起来完整”先铺满所有模块。
- 不在没有真实 runtime 验证时宣称功能完成。

### First Proof Point

用户粘贴一篇技术文章 URL，点击“分析并生成研究卡”，任务进入队列，状态从处理中到已完成，生成标题、摘要、关键要点、标签、下一步行动、评分，并写入用户绑定的 Notion database。

### Falsifier

如果第一版不能稳定展示失败原因和重试入口，或不能把至少一种稳定来源写入 Notion，产品方向需要收缩。

## Project Boundary

### Problem

目标用户在浏览 GitHub、技术文章、AI 工具更新、视频教程和 RSS 时，经常“先收藏，后遗忘”。当前痛点有三类：

1. 收藏入口轻，但整理成本高。
2. AI 能总结，但用户需要手动复制正文、粘贴 prompt、再整理字段。
3. Notion 适合长期沉淀，但手动建页、填字段、打标签过重。

### Product Role

ReachNote 是 `Capture -> Read -> Analyze -> Structure -> Sync to Notion` 的本地桌面工具。

- Agent-Reach 负责“读到内容”：渠道选择、安装体检、读取能力。
- ReachNote 负责“沉淀成研究资产”：采集、队列、AI 分析、结构化字段、Notion 同步、失败可见和重试。
- Notion 负责“长期知识库”：用户自己的 database、检索、归档和后续管理。

### First Version Must Complete

第一版必须完成以下闭环：

- 桌面 app 可启动，展示新版 UI 的 `队列 / 采集 / 模板 / 设置`。
- 默认进入 `队列`，可看到任务状态：全部、处理中、已完成、失败。
- `采集` 支持手动 URL 和从剪贴板粘贴 URL。
- 支持补充说明，限制长度并做 validation。
- 支持 Claude CLI 作为默认 AI provider。
- 支持 Agent-Reach doctor 或等价本地读取能力检查。
- 支持 Notion 连接状态，并能把生成的研究卡写入指定 database。
- 本地队列持久化，失败可见、可重试。
- 至少完成一种稳定来源的真实闭环：`Article URL`。
- 输出结构化研究卡：标题、摘要、关键要点、标签、下一步行动、评分、模型。

### Explicit Non-goals

- 不做完整知识库产品。
- 不做通用 Notion 写作工具。
- 不做完整 RAG 系统。
- 不做团队协作、权限管理、共享工作区。
- 不做社交平台大规模采集或登录态渠道。
- 不做 Notion 双向同步。
- 不做复杂模板编辑器。
- 不做云端内容数据库或中间服务器。
- 不把 Codex CLI、OpenAI-compatible API 作为第一版必须完成项；可在 UI 中标记计划中。

## Target Users

### Primary

开发者和 AI 工具研究者。

典型场景：

- 研究新的 AI coding 工具、模型发布、SDK、agent 框架。
- 追踪 GitHub repo、技术博客、产品更新、论文/报告摘要。
- 需要把“看到的信息”变成可比较、可回看的 Notion research card。

### Secondary

- 产品经理 / 创业者：竞品和产品更新追踪。
- 内容创作者 / 研究者：文章、视频、RSS 的素材沉淀。
- 投研 / 行业观察者：行业报告和工具动态的轻量归档。

第一版不为 secondary 用户扩展专门 workflow。

## Core Workflow

```text
Open ReachNote
-> Queue is default screen
-> User goes to Capture
-> Paste or input Article URL
-> Optional note
-> Select Claude CLI
-> Click Analyze and Generate Research Card
-> Local task created
-> Read content through Agent-Reach or approved read adapter
-> Build AI prompt from template
-> Claude CLI returns JSON
-> Validate and normalize research card
-> Sync to Notion database
-> Queue row updates to Completed or Failed
-> User can retry failed task or open the synced Notion page
```

## Success Criteria

### Product Success

- 新用户能在 5 分钟内完成本地依赖检查、Notion 连接，并成功处理第一条文章 URL。
- 完成任务在队列中可见，包含标题、来源、状态、时间、评分、模型。
- 失败任务展示人能理解的原因，并提供重试入口。
- 至少一种稳定来源能真实写入 Notion，不依赖 mock。
- 用户无需理解内部 prompt 或 Notion API，也能得到结构化卡片。

### Engineering Success

- 第一垂直切片有真实数据流和本地持久化，不只停留在 React state。
- 所有外部能力都有错误分类：Agent-Reach 缺失、AI provider 不可用、Notion 未授权、schema 不匹配、network failure。
- Secrets 不出现在 repo、日志、memory、测试快照。
- 关键契约有测试：task status、AI JSON parse、Notion mapping。
- 前端完成真实页面检查：loading、empty、error、validation、disabled、text overflow、console error。

## Data Sources

### P0

| Source | First Version Status | Reason |
| --- | --- | --- |
| Article URL | Must complete | 最稳定，最适合验证 `read -> analyze -> Notion` 闭环。 |
| Clipboard URL | Must complete as capture method | 与新版 UI 的“从剪贴板粘贴”一致。 |
| GitHub repo | Planned after Article is stable | 目标用户高频，但字段更复杂。 |
| RSS article/link | Planned | 可与文章共用部分读取和模板。 |
| YouTube transcript | Planned | 需要字幕可用性和失败提示。 |

### Out of Scope for V1

Twitter/X、小红书、Reddit、B 站深度采集、需要登录态或 Cookie 风控的平台、大规模监控。

## Runtime Environment

当前仓库已清空实现，以下为目标技术栈和必须重新建立的运行方式。

### Target Stack

| Layer | Decision |
| --- | --- |
| Desktop shell | Tauri 2 |
| Frontend | React 18 + TypeScript + Vite |
| UI | HeroUI + Tailwind CSS, plus icons from existing library choice |
| Core | Rust `reachnote-core`, no Tauri dependency |
| App layer | `src-tauri`, commands, tray/window, app data path |
| Persistence | SQLite local queue |
| Secrets | OS keychain |
| External reading | Agent-Reach or read adapter selected from doctor output |
| AI provider | Claude CLI first, Codex CLI and OpenAI-compatible API later |
| Sync | Notion API |

### Target Directory Structure

```text
rearchnote/
├─ plans/prds/                         # PRD source of truth
├─ memory/                             # factual project progress and design source
├─ docs/
│  ├─ adr/                             # architecture decisions
│  └─ discussions/                     # historical product discussion
├─ src/                                # React frontend, after implementation restarts
├─ src-tauri/                          # Tauri app layer, after implementation restarts
├─ crates/core/                        # Rust core domain logic, after implementation restarts
└─ README.md / README.zh-CN.md          # product overview, must be updated after PRD approval
```

### Target Commands

These commands are acceptance targets for the first implementation slice. They do not currently run because implementation has been cleared.

```bash
pnpm install
pnpm typecheck
pnpm build
cargo test -p reachnote-core
cargo check --manifest-path src-tauri/Cargo.toml
pnpm tauri dev
```

## UI Requirements From New Design

### Shell

- Desktop tool window, white background, large working canvas, subtle borders.
- Header includes ReachNote logo, search icon, settings icon.
- Bottom status bar includes local-first state, Pre-alpha state, AI provider.

### Queue

- Default screen.
- Filter chips: 全部、处理中、已完成、失败。
- Table columns: 标题、来源、状态、时间、评分、模型。
- Status styles: processing spinner/pill, completed green, failed red.
- Empty state required when no tasks.
- Failed row must expose retry and readable error in row detail or expanded panel.

### Capture

- Left panel: URL input, clipboard paste button, optional note with character count, provider select, primary CTA, Notion disconnected card.
- Right panel: research card preview. Before generation, show skeleton matching title, summary, key points, tags, next action, rating.
- CTA disabled when URL invalid, provider unavailable, or active capture is running.

### Templates

- Show built-in templates: GitHub 项目分析、文章阅读笔记、视频笔记、RSS 简报。
- First version only displays template direction and supports default template selection if required by capture.
- No custom editing or save.

### Settings

- AI provider: Claude CLI default, Codex CLI planned, OpenAI-compatible API planned.
- Agent-Reach: command path and doctor status.
- Notion: disconnected/connected state and connect action.
- Privacy and storage: local-first explanation, keychain requirement.

## P1 Architecture Map

### System Boundaries

| Boundary | Owns | Must Not Own |
| --- | --- | --- |
| `src/` frontend | UI rendering, form state, local interaction state, command calls, visible error states | Secrets, direct Notion API, scraping, business orchestration |
| `src-tauri/` app layer | Tauri commands, app lifecycle, window/tray, SQLite connection, OS keychain, subprocess wiring | AI prompt domain decisions, UI styling |
| `crates/core/` | Task domain, source type, AI provider contract, card draft, mapping logic, parse/validation | Tauri APIs, OS path, UI state |
| Agent-Reach adapter | Doctor/read capability detection and normalized content return | Long-term storage, Notion sync |
| AI provider adapter | Claude CLI first, JSON output and error classification | UI decisions, Notion mapping side effects |
| Notion adapter | OAuth/token use, database schema detection, page write | AI analysis, external reading |
| SQLite store | Local queue, task states, retry metadata, Notion page id, timestamps | Raw secrets, unbounded full content by default |

### State Model

Processing status:

```text
Queued -> Reading -> Analyzing -> Syncing -> Synced
                         \-> Failed
```

Required task fields:

- `id`
- `url`
- `source_type`
- `template_id`
- `status`
- `title`
- `source_domain`
- `score`
- `model`
- `notion_page_id`
- `error_kind`
- `error_message`
- `created_at`
- `updated_at`
- `synced_at`

### Configuration

- Normal preferences: local config file under app data directory.
- Secrets: OS keychain only.
- Required config: Claude CLI command, Agent-Reach command, Notion database id, default template.

### Third-Party Services

- Agent-Reach local CLI or upstream selected tools.
- Claude CLI local subprocess.
- Notion API.

### Build and Deploy

- Build target: Tauri desktop app.
- First validation target: local dev runtime.
- Ship target: signed/bundled app only after local runtime and Notion smoke test pass.

## P2 Concrete Trace

### Repo Reality Before Reset

CodeGraph audit before clearing implementation found the old chain:

```text
src/App.tsx handleCapture
-> invoke("capture")
-> src-tauri/src/lib.rs capture
-> detect_source
-> optional AgentReach::read
-> build_provider
-> AiProvider::analyze
-> AnalysisResult
-> frontend research card
```

That chain is no longer current code. It is useful only as a warning: it lacked persistent queue, real Notion sync, real settings, robust Agent-Reach alignment, and desktop QA.

### Target Trace For First Slice

```text
User enters Article URL
-> frontend validates URL and note length
-> frontend invokes create_capture_task
-> Tauri command creates SQLite task: Queued
-> worker/read pipeline marks Reading
-> Agent-Reach/read adapter returns content or readable error
-> core builds Article template prompt
-> Claude CLI analyzes and returns JSON
-> core parses and validates card draft
-> app marks Syncing
-> Notion adapter writes page to selected database
-> SQLite saves notion_page_id and Synced status
-> Queue UI refreshes row as Completed
```

### Error Paths

- Invalid URL: frontend validation, no task created.
- Clipboard empty or not URL: visible disabled/error state.
- Claude CLI missing: task fails with provider error and setup hint.
- Agent-Reach missing: task fails with doctor/setup hint.
- AI JSON parse failure: task fails with model output parse error, no Notion write.
- Notion disconnected: task can produce local card but sync status is blocked; CTA and queue must show connection requirement.
- Notion schema mismatch: task fails with mapping error and required field names.
- Network failure: retryable failure, no silent drop.

### Final Side Effect

The first slice is complete only when:

- local SQLite has a durable task record,
- queue UI displays final status,
- Notion contains a created page for a real article URL,
- failure state can be reproduced and retried.

## P3 Design Decision

### Why This Shape Exists

- Product intent: turn incoming links into durable Notion research assets, not just summaries.
- Security: local-first and BYOK require local queue and keychain.
- Reliability: queue-first UI makes async failures visible and recoverable.
- Scope control: Article URL first reduces platform-reading risk while proving the full value chain.
- Compatibility: Tauri + React preserves HeroUI and cross-platform desktop path.

### Decision

Restart implementation from a single end-to-end article capture slice:

```text
Article URL -> Local queue -> Claude CLI analysis -> Notion page -> Queue status
```

This is the smallest coherent slice because it touches every product-critical boundary once: UI, command contract, local persistence, external reading, AI provider, Notion sync, and visible state.

### Tradeoff

This delays GitHub/RSS/YouTube support and template customization, but prevents the product from becoming a collection of disconnected screens.

### 10x Failure Mode

If task state and error classification are not established first, every future source, provider, and template will duplicate failure handling and retry logic.

## First Vertical Slice

### Slice Name

`Article Capture to Notion`

### Included

- Recreate minimal Tauri + React + Rust workspace.
- Implement queue default screen from new UI.
- Implement capture form from new UI.
- Implement local SQLite task model.
- Implement Claude CLI provider.
- Implement one Article template.
- Implement Notion connection and write to existing database.
- Implement retry for failed tasks.
- Implement settings display for Claude CLI, Agent-Reach command, Notion status.

### Excluded

- GitHub-specific analysis.
- Video/RSS-specific analysis.
- Custom template editing.
- Search beyond current queue table.
- Global shortcut.
- Current browser URL capture.
- Packaging/signing.

### Acceptance Tests

- `pnpm typecheck` passes.
- `pnpm build` passes.
- `cargo test -p reachnote-core` passes.
- `cargo check --manifest-path src-tauri/Cargo.toml` passes.
- `pnpm tauri dev` opens desktop window.
- Manual smoke: paste valid article URL, generate card, queue shows completed, Notion page exists.
- Manual failure: disconnect Notion or use missing Claude command, queue shows failed with readable error and retry.
- Visual QA: queue, capture, settings states match new UI enough to avoid layout drift, text overflow, broken icons, console errors.

## Implementation Discipline

### Small-Step Rule

Each implementation step should change one goal:

1. Scaffold workspace.
2. Domain types and tests.
3. SQLite queue.
4. Tauri commands.
5. Claude CLI provider.
6. Notion adapter.
7. Queue UI.
8. Capture UI.
9. Settings UI.
10. Desktop QA and review gate.

No unrelated framework swaps, broad refactors, or hidden dependency upgrades.

### Required States

Every user-facing path must cover:

- loading
- empty
- success
- error
- validation
- disabled
- permission/auth blocked
- network failure

## Review, Gate, and Ship

### Review Gate

Before ship or commit, run an engineering review focused on:

- behavior regressions,
- missing tests,
- security/privacy leaks,
- data contract breakage,
- Notion permission and schema risk,
- debug code,
- unverified runtime claims.

If using AI review, another agent/model should review read-only and report P0/P1/P2 findings. P0/P1 blocks ship.

### Ship Rule

Ship only after verification passes. Before commit:

- inspect `git diff`,
- inspect `git status`,
- check no secrets are present,
- ensure README/memory/handoff reflect actual state.

After deployment or packaging, verify the real app path, not just build success.

## Documentation Requirements

Every completed slice must update reusable project records:

- `memory/*-progress.md` or successor progress file,
- README if commands or product status changed,
- handoff or development progress note with changed files, reason, verification, residual risk,
- interface matrix if command/API contracts changed.

## Next Bottleneck After This PRD

The next real bottleneck is not more product prose. It is recreating the first runnable workspace scaffold and proving the queue-first shell can launch.

Entry point after PRD approval:

```text
Scaffold Tauri 2 + React 18 + HeroUI + Rust core
-> create queue/domain types
-> run typecheck/build/cargo checks
```

