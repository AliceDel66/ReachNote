# ReachNote Next Phase PRD: Platform Matrix, Templates, Multi-Destination Sync, Onboarding, Shortcuts

> **Status**: Draft
> **Date**: 2026-07-01
> **Owner**: Yaocheng review; implementation owner assigned after PRD approval
> **Audience**: Claude review, Codex implementation, product/engineering planning
> **Source Inputs**: `memory/README.md`, `memory/frontend-progress.md`, `memory/backend-progress.md`, `memory/integration-progress.md`, `memory/design-source.md`, `memory/desktop-qa.md`, `plans/prds/20260630-1906-reachnote-mvp-reset.prd.md`, current code under `src/`, `src-tauri/`, `crates/core/`, local `agent-reach doctor --json`, local `agent-reach --help`, user-provided Agent-Reach platform screenshot.

## AI Quick-Read Card

| Field | Content |
| --- | --- |
| Product | ReachNote |
| Phase Name | Next Phase: 从 `Article -> Notion` 升级到 `Multi-source -> Template -> Multi-destination` |
| Thesis | ReachNote 下一阶段的核心不是“堆平台按钮”，而是建立一套可检测、可降级、可验证的能力框架：Agent-Reach 负责看到互联网，ReachNote 负责把来源转成模板化研究卡，并同步到用户选择的目标系统。 |
| Current Verified Baseline | 当前已跑通 `Article URL -> SQLite queue -> AgentReachWebReader(Jina/GitHub fallback) -> Claude/Codex/OpenAI-compatible -> AnalysisResult -> Notion -> Queue UI`。 |
| Primary Users | 开发者、AI 工具研究者、需要持续采集 GitHub/网页/视频/RSS/平台内容并沉淀到工作系统的个人用户。 |
| Primary Outcome | 用户首次启动完成环境检测和目标系统配置后，可以用手动输入、剪贴板、快捷键把支持来源加入队列，选择模板分析，并同步到 Notion、飞书、企业微信或钉钉中的一种。 |
| First Slice | 首次启动引导 + settings 持久化 + 环境能力检测。 |
| Hard Non-goals | 不在 ReachNote 内重写社交平台抓取；不承诺所有 Agent-Reach 平台一次性达到同等深度；不做团队协作知识库；不做双向同步；不把登录态/Cookie 风险隐藏成“零配置”。 |
| Validation Bar | 每个切片必须有本地测试、Tauri desktop runtime 检查、memory 更新；用户可见功能不能只靠浏览器或 mock 宣称 PASS。 |

---

## 审查决议 (Claude Review Gate · 2026-07-01)

> 审查人：Claude。方式：只读代码核对 + P1/P2/P3。基线声称已逐条对照当前 `src-tauri/src/{lib,notion,reader,provider,store}.rs` 与 `crates/core/src/{task,analysis,notion}.rs` 核实**属实**（`NotionClient`、`notion_settings` 表、`recover_interrupted_tasks`、`sync_pending_analyzed_tasks` 均已存在并测试）。

**Gate: PASS with conditions（方向正确、切片基本可独立发布；下列 P0/P1 须在对应切片开工前解决）**

**P0 findings（开工前必须闭环）**

- **P0-1 · Webhook 密钥落盘顺序自相矛盾。** PRD 一边说「Webhook URLs count as secrets」「add keychain abstraction before adding more plaintext credentials」（L435/L990），一边 Slice 5 无 keychain 依赖，且 `destinations.config_json`（L641）会明文存 webhook URL。这会新增一个明文密钥面。**决议：Slice 5 必须依赖一个 keychain 切片（新增 Slice 4.5 或把 `SecretStore` 落地并入 Slice 4）。** webhook secret 一律走 `credential_ref` → `SecretStore`，`config_json` 只存非敏感字段（如消息模板、目标名）。现存 Notion token 在 SQLite 明文是**既有债务**，本阶段顺带迁移到 `SecretStore`，不作为「明文可接受」的先例扩大。（对应 Open Q1，答案收紧为「必须」而非「recommended」）

**P1 findings（切片设计必须写清，否则实现会跑偏）**

- **P1-1 · `pending_destination` 是幻影状态。** 当前 `TaskStatus` 只有 `Queued/Reading/Analyzing/Analyzed/Syncing/Synced/Failed`（见 `crates/core/src/task.rs`），无 `pending_destination`。**决议：不新增状态。** 复用已有的 `Analyzed`——它天然就是「本地研究卡就绪、尚未同步」。「未配置目的地」= 停在 `Analyzed` 且不触发 sync（不是 `Failed`）。Open Q2 的两可答案就此定为「用 `Analyzed`」。全文 `pending_destination` 字样应删除或替换。
- **P1-2 · 启动时 `sync_pending_analyzed_tasks` 与多目的地/无目的地会冲突。** 现有该函数是 Notion 专用、且会在启动扫描**所有** `Analyzed` 任务自动补同步。多目的地后必须：① 按 task 的 `destination_id` 路由；② `destination_id` 为空的任务**跳过**（保持 `Analyzed`），不得当失败；③ Notion 专用逻辑收进 adapter。Slice 4 验收要显式覆盖「无目的地任务不被后台扫描误改」。
- **P1-3 · Agent-Reach `doctor` 归一化缺映射规格 → 无法实现。** L117-124 只给状态示例，没给 `doctor --json` 原始结构 → `PlatformAvailability` 枚举的确定映射规则。**Slice 2 必须先钉一个真实 `agent-reach doctor --json` 的 fixture 文件**（脱敏后入库到 `crates/core/src/testdata/` 或 tests），并据此写归一化纯函数 + 单测（ready/warn/off 三类各一例，见 L970）。否则 codex 只能猜 JSON 形状。
- **P1-4 · 组件拆分要提前，不能等「文件膨胀阻碍清晰」再做。** 本阶段给 `App.tsx`（单文件）叠加 onboarding 4 步 + settings 5 段 + 平台矩阵 + 目的地表单，必然失控。**决议：把 App.tsx 拆分列为 Slice 1 的硬性交付**（至少拆出 `onboarding/`、`settings/`、`capture/`、`queue/` 目录），而非 Risk 表里的「if」。

**P2 findings（建议改进，不阻断）**

- **P2-1 · `create_capture_task` 签名「向后兼容」表述有误。** Tauri command 在 `invoke` 边界本就是命名参数对象（现前端已传 `{url,note,providerId}`）。加 `templateId/destinationId/sourceType` 只要都是 `Option<T>` 就**天然兼容**，无需保留「旧参数形式」双轨（L730-734 可简化）。
- **P2-2 · `doctor` 不应每次启动同步运行。** 15 平台网络探测慢。启动读 `source_capability_snapshots` 最近快照即可，doctor 仅在首启动与 Settings「刷新」按钮时跑（带超时）。请在 Slice 2 写明读取/刷新 TTL 策略。
- **P2-3 · 模板本阶段共用 `research_card_v1`。** 五个模板输出焦点不同（L365-369），但当前 `AnalysisResult`（title/summary/key_points/tags/score/next_action/model）是唯一 schema。**明确本阶段模板只改 prompt、不改输出结构**（L262 的「template-specific extension」推迟），否则会诱导 codex 提前分裂 schema。
- **P2-4 · 切片依赖图未画。** 「每片独立可发布」总体成立,但 Slice 7 依赖 2（矩阵）+3（模板路由）;Slice 3 改 `AnalysisRequest`/`create_capture_task`;Slice 4 迁移 task 字段。建议加一张依赖图：`1 → 2 → 3 → 4 →(4.5 keychain)→ 5`,`6` 独立,`7` 依赖 `2+3`。
- **P2-5 · 全局快捷键缺剪贴板/权限前置。** `capture_from_clipboard` 读剪贴板需 `tauri-plugin-clipboard-manager`;macOS 全局快捷键可能需辅助功能授权。Slice 6 scope 应补这两项依赖与授权引导。

**审查确认的强项（保留,勿改）**：加法式迁移不 rename、保留 `notion_page_id`/`analysis_json`；平台矩阵拒绝伪造通用 `read`；webhook 明确定位「消息同步」而非数据库；retry 不重跑分析；Falsifier 具体；`cargo test --manifest-path src-tauri/Cargo.toml` 纳入验收(补齐了前几刀只 `cargo check` 的缺口)。

**开工前须先落的改动**：① 把 P0-1 的 keychain 顺序写进切片(新增 Slice 4.5 或并入 Slice 4);② 全文删 `pending_destination`、改用 `Analyzed`(P1-1);③ Slice 2 补 doctor fixture 要求(P1-3);④ Slice 1 scope 加 App.tsx 组件拆分(P1-4)。下方对应位置有内联批注 `[审查批注 Pn-n]`。

---

## Direction Framing

### Thesis

ReachNote 的下一阶段要从“能把一篇文章写进 Notion”升级为“用户的本地采集控制台”。真正的产品护城河不是支持列表很长，而是每个来源、模板、同步目标都有清晰状态、失败原因和可重试路径。

### High-Level Direction

把功能拆成三类 connector：

```text
SourceConnector -> TemplateRegistry -> DestinationConnector
```

- `SourceConnector`：封装 Agent-Reach 能力、当前机器可用状态、读取策略和登录态提示。
- `TemplateRegistry`：封装不同来源的分析 prompt、输出字段、默认评分逻辑和目标字段映射。
- `DestinationConnector`：封装 Notion、飞书、企业微信、钉钉等目标系统的配置、测试连接、写入、失败分类。

### Bold Takes

- 全平台支持必须先表现为“能力矩阵 + 可用/需配置/不可用状态”，再逐个做真实读取闭环。
- 首次启动引导是下一阶段第一刀，因为默认 AI provider、Agent-Reach 状态和 destination 选择都需要持久化。
- 模板系统优先做系统模板和模板选择，不先做复杂自定义编辑器。
- 飞书、企业微信、钉钉第一版应走 webhook/机器人消息路径，不直接承诺完整知识库数据库写入。
- 快捷键必须建立在后台常驻能力上，不能只做前端按钮。

### What Not To Do

- 不把 Twitter/X、小红书、Reddit、Facebook、Instagram 等登录态平台伪装成“安装即用”。
- 不把所有平台塞进同一个 `read_article(url)`。
- 不把 Notion 字段模型硬编码到通用 `Task`，否则多目的地会继续被 Notion 命名污染。
- 不把 AI provider 选择只放在 React state，必须进入本地 settings。
- 不把 webhook token、Notion token、Cookie、API key 写入日志、memory 或测试快照。

### First Proof Point

首次启动时，ReachNote 自动检测本机 `claude`、`codex`、OpenAI-compatible 配置和 `agent-reach doctor --json`，推荐一个默认 AI provider；用户选择 Notion 或飞书 webhook 并测试连接；进入系统后按快捷键从剪贴板采集一个 GitHub repo 或网页 URL，选择对应模板，队列显示完成并同步到所选目标。

### Falsifier

如果下一阶段仍然只能处理 `source_type=article`、`template_id=article`、`NotionClient`，或用户重启后 provider/destination 选择丢失，则该阶段没有真正支撑后续扩展。

## Current State Evidence

### Verified Local Chain

当前仓库已经验证：

```text
Article URL
-> create_capture_task(provider_id)
-> SQLite tasks
-> recover_interrupted_tasks
-> sync_pending_analyzed_tasks
-> run_capture_task
-> AgentReachWebReader(Jina Reader / GitHub API fallback)
-> AnalysisRequest(content_text/content_reader)
-> ProviderRunner
-> AnalysisResult JSON validation
-> Syncing
-> NotionClient
-> Synced/Failed
-> Queue UI
```

### Current Code Boundaries

| Area | Current File/Surface | Constraint |
| --- | --- | --- |
| Frontend app shell | `src/App.tsx` | Single-file React app; nav = `采集 / 队列 / 模板 / 设置`; templates are static constants; provider select is session-only. |
| Core task model | `crates/core/src/task.rs` | `Task` has `source_type`, `template_id`, `provider_id`, `analysis_json`, `notion_page_id`; validation still assumes article/template article in store layer. |
| AI provider | `src-tauri/src/provider.rs` | Supports `claude_cli`, `codex_cli`, `openai_compatible`; OpenAI-compatible still reads env vars. |
| Reader | `src-tauri/src/reader.rs` | `AgentReachWebReader` uses Jina Reader by default and GitHub API/README fallback; no generalized Agent-Reach source router yet. |
| Notion | `src-tauri/src/notion.rs` and `src-tauri/src/lib.rs` | Notion settings are in local SQLite; sync path is hard-coded to Notion semantics. |
| Desktop runtime | `src-tauri/src/lib.rs`, `src-tauri/tauri.conf.json` | Hide-to-background exists; global shortcut plugin is not installed yet. |
| Memory/QA | `memory/*.md` | Tauri dev + AX fallback validated; Computer Use remains blocked for debug app. |

### Agent-Reach Local Snapshot

Local command used:

```bash
agent-reach doctor --json
agent-reach --help
agent-reach check-update
```

Observed facts on 2026-07-01:

- Agent-Reach version: `v1.5.0`.
- `agent-reach --help` exposes `setup/install/configure/doctor/uninstall/skill/format/transcribe/check-update/watch/version`.
- There is no generic `agent-reach read <url>` command in this local version.
- `agent-reach doctor --json` is the best machine-readable source for platform availability and `active_backend`.
- Local doctor snapshot reported 15 platform keys: `github`, `twitter`, `youtube`, `reddit`, `facebook`, `instagram`, `bilibili`, `xiaohongshu`, `linkedin`, `xiaoyuzhou`, `v2ex`, `xueqiu`, `rss`, `exa_search`, `web`.
- Local doctor snapshot status examples:
  - `github`: ok, active backend `gh CLI`.
  - `web`: ok, active backend `Jina Reader`.
  - `rss`: ok, active backend `feedparser`.
  - `exa_search`: ok, active backend `Exa via mcporter`.
  - `bilibili`: ok, active backend `B站搜索 API`, with full functionality recommending `bili-cli`.
  - `twitter`, `v2ex`, `xueqiu`: warn on this machine.
  - `youtube`, `reddit`, `facebook`, `instagram`, `xiaohongshu`, `linkedin`, `xiaoyuzhou`: off on this machine.
- `agent-reach check-update` reported current version is latest.

> **[审查批注 P1-3]** 这些是人读的状态示例,不是机器映射规格。Slice 2 实现前必须先落一个**真实 `agent-reach doctor --json` 输出的脱敏 fixture**(存入 tests/testdata),并据此定义 `doctor JSON 字段 → PlatformAvailability` 的确定映射(如 `ok→ready`、`warn→needs_login|needs_config`、`off→needs_install|blocked`——具体依 doctor 实际字段而非猜测)。归一化写成 core 纯函数 + 单测(ready/warn/off 各一例)。没有 fixture,codex 只能臆测 JSON 形状。

## Goals

### G1: Agent-Reach Supported Platform Matrix

ReachNote should reflect all Agent-Reach supported platforms in UI/settings as capability rows, with each row showing:

- platform name
- source type
- current availability: `ready`, `needs_install`, `needs_login`, `needs_config`, `blocked`, `unknown`
- active backend from `agent-reach doctor --json`
- what ReachNote can do now: `capture_url`, `search`, `read_content`, `transcribe`, `metadata_only`, `not_supported_yet`
- configuration guidance, without collecting credentials unless the connector needs it

The goal is not equal-depth support for all platforms in one release. The goal is truthful platform visibility and a staged path to real ingestion.

### G2: Template Foundation

ReachNote should persist and apply built-in templates:

- GitHub 项目分析
- 网页文章笔记
- YouTube/B站视频笔记
- RSS 简报
- 平台讨论摘要

Each template should define:

- compatible source types
- prompt profile
- output fields
- default destination mapping
- display label and description
- enabled state

### G3: Multi-Destination Sync

ReachNote should support user-selected destinations:

- Notion: keep current database-page path, then prepare keychain migration.
- 飞书: webhook/机器人 message MVP first. Full document/wiki/table sync is not in this phase.
- 企业微信: group robot webhook message MVP first. Full app message/approval/workbench integration is not in this phase.
- 钉钉: robot webhook message MVP first. Full DingTalk document/table integration is not in this phase.

Each destination should support:

- settings form
- save config
- masked credential display
- test connection
- sync structured research card
- error classification
- retry from queue

### G4: First Launch Onboarding

On first install or after reset, ReachNote should guide users through:

1. AI provider detection.
2. Agent-Reach doctor check.
3. Destination selection and connection test.
4. Finish and enter Queue.

The onboarding must persist choices so the app behaves consistently after restart.

### G5: Global Capture Shortcut

ReachNote should support a configurable global shortcut for one-key capture:

- Default shortcut: `CommandOrControl+Shift+R`.
- First action: read clipboard text, validate URL, create queued task using default provider/template/destination.
- If clipboard is invalid: show main window on Capture page with validation error.
- If app is hidden: keep process alive and update queue in background.

## Non-Goals

- No cloud relay/server.
- No full custom template editor in this phase.
- No Notion OAuth in this phase unless internal token becomes a blocker.
- No Feishu/WeCom/DingTalk full database or document schema integration in this phase.
- No social platform write operations: posting, liking, commenting, following.
- No automated scraping around platform anti-bot protections.
- No storing raw Cookie/login sessions inside ReachNote.
- No browser extension in this phase.
- No full-text local search beyond current queue search.
- No RAG or vector database.

## User Stories

### U1: First-Time User

As a new user, I open ReachNote for the first time and see a guided setup:

1. ReachNote detects `claude`, `codex`, and OpenAI-compatible configuration.
2. ReachNote recommends a default AI provider.
3. ReachNote runs Agent-Reach doctor and shows source platform status.
4. I choose Notion or a webhook destination.
5. I test the connection.
6. I enter the app with a clear ready/needs-config state.

Acceptance:

- The app does not silently enter Capture with no usable provider or destination.
- If no AI provider is ready, Capture is disabled and Settings explains the missing dependency.
- If no destination is ready, tasks may analyze locally but sync is marked `failed` or `pending_destination` with a readable message.

> **[审查批注 P1-1]** `pending_destination` 不存在于当前 `TaskStatus` 枚举,且没必要新增。**改为**:无目的地时任务停在 **`Analyzed`**(已有状态,语义正好是「本地研究卡就绪、未同步」),**不标 `failed`**,不触发 sync。UI 用 `Analyzed` + 「未配置目的地」提示。全文 `pending_destination` 一律替换为「停在 `Analyzed`」。

### U2: Returning User

As a returning user, my provider, destination, and shortcut choices persist after restart.

Acceptance:

- The default provider is not reset to Claude CLI on every launch.
- Destination settings display masked credentials.
- Queue still shows previous task status from SQLite.

### U3: Platform-Aware Capture

As a researcher, I paste a GitHub repo, article URL, YouTube link, RSS URL, or B站 video/search URL and ReachNote suggests compatible templates and shows whether the source can be read now.

Acceptance:

- GitHub repo routes to GitHub template by default.
- Web article routes to article template by default.
- YouTube/B站 routes to video template if transcript/search backend is available.
- RSS URL routes to RSS template if feed parser is available.
- Unsupported or login-required sources do not pretend to be ready.

### U4: Template-Based Analysis

As a user, I choose a template before analysis so the generated card matches the content type.

Acceptance:

- Template choice is saved on the task.
- AI prompt includes template intent.
- Output still validates against a shared core schema or template-specific extension.
- Queue row shows template label.

### U5: Multi-Destination Sync

As a user, I choose where the research card goes: Notion, 飞书, 企业微信, or 钉钉.

Acceptance:

- Destination choice is explicit.
- Test connection runs before marking a destination ready.
- Sync errors name the target system and action.
- Queue can retry sync without re-running analysis when `analysis_json` exists.

### U6: One-Key Capture

As a user, I copy a URL in any app and press the global shortcut. ReachNote queues the capture without forcing me to open the main window first.

Acceptance:

- Valid clipboard URL creates a task.
- Invalid clipboard content opens Capture with an error.
- Shortcut conflict is detected and explained.
- Hidden app remains alive and queue continues processing.

## Source Platform Matrix

### Platform Status Model

ReachNote should normalize Agent-Reach doctor output into:

```ts
type PlatformAvailability =
  | "ready"
  | "needs_install"
  | "needs_login"
  | "needs_config"
  | "blocked"
  | "unknown";

type PlatformAction =
  | "capture_url"
  | "read_content"
  | "search"
  | "transcribe"
  | "metadata_only"
  | "not_supported_yet";
```

### Planned Platform Coverage

| Platform | Agent-Reach Key | Initial ReachNote Action | Phase | Notes |
| --- | --- | --- | --- | --- |
| Web page | `web` | `read_content` via Jina Reader | Phase 2A | Already partially implemented. |
| GitHub | `github` | repo metadata + README, later issues/PR/search | Phase 2A | Already has GitHub API/README fallback. |
| RSS/Atom | `rss` | feed URL read + article list summary | Phase 2B | Use as stable source after web/GitHub. |
| Exa Search | `exa_search` | search result capture and summary | Phase 2B | Must avoid claiming full web crawl. |
| YouTube | `youtube` | transcript-based video note | Phase 2B | Depends on `yt-dlp` or transcript availability. |
| B站 | `bilibili` | search result or transcript when available | Phase 2B | Current machine has search API only. |
| V2EX | `v2ex` | hot/topic capture | Phase 2C | Public API may fail by network; mark degraded. |
| 雪球 | `xueqiu` | stock/community snapshot | Phase 2C | Usually needs login cookie; not P0. |
| 小宇宙播客 | `xiaoyuzhou` | audio transcription summary | Phase 2C | Depends on ffmpeg/transcribe backend. |
| Twitter/X | `twitter` | search/tweet/thread capture | Phase 2D | Login/backend required; no silent support. |
| Reddit | `reddit` | post/comment capture | Phase 2D | Login/backend required. |
| Facebook | `facebook` | page/group/feed capture | Phase 2D | OpenCLI/login required. |
| Instagram | `instagram` | profile/post capture | Phase 2D | OpenCLI/login required. |
| 小红书 | `xiaohongshu` | note/search capture | Phase 2D | OpenCLI or MCP/login required. |
| LinkedIn | `linkedin` | profile/company/job capture | Phase 2D | MCP/login required. |

### Platform UX Rules

- Ready platforms appear as selectable.
- Needs-login platforms appear disabled with setup guidance.
- Off platforms do not disappear; they explain what is missing.
- The app must not ask for platform cookies directly unless a future approved connector explicitly owns that credential flow.
- Source platform UI should use the exact `agent-reach doctor` message when helpful, but truncate long shell instructions for layout.

## Template System Requirements

### Template Data Model

Recommended core template record:

```ts
interface ResearchTemplate {
  id: string;
  name: string;
  description: string;
  compatible_source_types: string[];
  prompt_profile: string;
  output_schema: "research_card_v1";
  destination_mappings: Record<string, string>;
  enabled: boolean;
  system: boolean;
  created_at: string;
  updated_at: string;
}
```

### Built-In Templates

| Template ID | Name | Compatible Sources | Required Output Focus |
| --- | --- | --- | --- |
| `github_project` | GitHub 项目分析 | `github_repo` | 项目定位、核心能力、技术栈、适用场景、风险、下一步验证。 |
| `web_article` | 网页文章笔记 | `article`, `web` | 摘要、关键论点、可复用观点、标签、下一步行动。 |
| `video_note` | 视频笔记 | `youtube_video`, `bilibili_video`, `podcast` | 主题、章节/片段、关键观点、可执行动作。 |
| `rss_digest` | RSS 简报 | `rss_feed`, `rss_item` | 多条来源聚合、趋势、重点链接、待读优先级。 |
| `platform_discussion` | 平台讨论摘要 | `twitter`, `reddit`, `v2ex`, `xiaohongshu`, `facebook`, `instagram`, `linkedin` | 讨论共识、分歧、代表性观点、可信度提醒。 |

### Template Scope For This Phase

Must have:

- Built-in templates persisted locally or registered from core constants.
- Capture page can select template.
- Default template auto-selected by URL/source type.
- Task stores `template_id`.
- Prompt changes based on template.
- Queue row shows template label or source type.

May have:

- Enable/disable template.
- Template preview.

Must not have:

- Full custom template editor.
- Arbitrary user prompt injection into system prompt without validation.
- Template marketplace/import/export.

## Multi-Destination Requirements

### Destination Model

Recommended destination record:

```ts
type DestinationKind =
  | "notion"
  | "feishu_webhook"
  | "wecom_webhook"
  | "dingtalk_webhook";

interface DestinationSettings {
  id: string;
  kind: DestinationKind;
  label: string;
  configured: boolean;
  enabled: boolean;
  credential_ref: string | null;
  config_json: string;
  last_tested_at: string | null;
  last_test_status: "passed" | "failed" | null;
  created_at: string;
  updated_at: string;
}
```

### Destination MVP Behavior

| Destination | Phase Behavior | Test Connection | Sync Output |
| --- | --- | --- | --- |
| Notion | Keep current database page creation; migrate credential storage later. | Read database or equivalent current Notion API check. | Structured properties + page link. |
| 飞书 | Webhook/robot message only. | Send test message to configured webhook. | Markdown/text card message. |
| 企业微信 | Group robot webhook only. | Send test message to configured webhook. | Markdown/text card message. |
| 钉钉 | Robot webhook only. | Send test message to configured webhook. | Markdown/text card message. |

### Destination Security Rules

- Credentials must be masked in UI.
- Credentials must not be printed in logs or memory.
- Existing Notion SQLite token storage is accepted as current baseline but next phase should introduce a keychain abstraction before adding more destination secrets.
- Webhook URLs count as secrets.
- Test connection should send a minimal non-sensitive message.

### Destination Error Kinds

Extend `ErrorKind` or add destination-specific classification:

- `destination_unauthorized`
- `destination_not_configured`
- `destination_schema_mismatch`
- `destination_rate_limited`
- `destination_network_failed`
- `destination_payload_rejected`

Do not reuse `NotionUnauthorized` for 飞书/企业微信/钉钉.

## First Launch Onboarding Requirements

### Onboarding States

```ts
type OnboardingStep =
  | "environment_check"
  | "ai_provider"
  | "source_capabilities"
  | "destination"
  | "finish";
```

### Environment Check

ReachNote should expose a Tauri command:

```ts
interface EnvironmentStatus {
  ai_providers: AiProviderStatus[];
  agent_reach: AgentReachStatus;
  source_platforms: SourcePlatformStatus[];
  destinations: DestinationSummary[];
  recommended_provider_id: string | null;
}
```

Provider detection:

- Claude CLI: check executable `REACHNOTE_CLAUDE_CMD` or `PATH`.
- Codex CLI: check executable `REACHNOTE_CODEX_CMD` or `PATH`.
- OpenAI-compatible: check base URL/model config and optional key availability without printing key.

Default provider selection:

1. If Claude CLI ready, recommend `claude_cli`.
2. Else if Codex CLI ready, recommend `codex_cli`.
3. Else if OpenAI-compatible config ready, recommend `openai_compatible`.
4. Else no default; onboarding shows provider unavailable.

Agent-Reach detection:

- Check `agent-reach` executable.
- Run `agent-reach doctor --json` with timeout.
- Parse platform status.
- If command missing, show install guidance, but do not install automatically.

### Destination Setup

Onboarding destination choice:

- Notion
- 飞书 webhook
- 企业微信 webhook
- 钉钉 webhook
- Skip for now

If user skips:

- App can enter Queue.
- Capture can analyze locally.
- Sync status becomes `failed` or `pending_destination` with a clear action: configure destination.

### Completion Criteria

Onboarding is complete when:

- `app_settings.onboarding_completed = true`
- default AI provider saved
- default template saved
- default destination saved or explicit `no_default_destination` saved
- last environment check snapshot saved

## Global Shortcut Requirements

### Technical Approach

Use Tauri 2 global shortcut plugin.

Expected dependencies:

- JS package: `@tauri-apps/plugin-global-shortcut`
- Rust plugin: `tauri-plugin-global-shortcut`

Implementation must use the repo's Tauri plugin installation path instead of manual config when possible:

```bash
pnpm tauri add global-shortcut
```

### Default Shortcut

Default:

```text
CommandOrControl+Shift+R
```

Reason:

- Avoids common browser capture shortcuts.
- Memorable for ReachNote.
- Works cross-platform with `CommandOrControl`.

### Shortcut Behavior

On shortcut:

1. Read clipboard text.
2. If clipboard is a valid supported URL:
   - detect source type
   - choose default provider/template/destination
   - create task
   - run capture in background
   - optionally show lightweight notification or bring Queue forward, depending on app setting
3. If clipboard is invalid:
   - show main window
   - navigate to Capture
   - display validation error

### Shortcut Settings

Settings page should show:

- current shortcut
- enabled/disabled toggle
- registration status
- conflict/error message

Do not build a full shortcut recorder in the first shortcut slice unless required by Tauri plugin constraints. A small preset selector is enough:

- `CommandOrControl+Shift+R`
- `CommandOrControl+Shift+N`
- disabled

## Data Model Changes

### Existing Task Migration

Current `Task` has:

- `source_type`
- `template_id`
- `provider_id`
- `analysis_json`
- `notion_page_id`

Next phase should avoid a breaking migration by adding fields rather than renaming immediately.

Recommended additions:

```text
tasks.source_backend TEXT NULL
tasks.reader_label TEXT NULL
tasks.destination_id TEXT NULL
tasks.destination_kind TEXT NULL
tasks.destination_record_id TEXT NULL
tasks.destination_url TEXT NULL
tasks.raw_content_ref TEXT NULL
tasks.template_version TEXT NULL
```

Keep `notion_page_id` for backward compatibility until `destination_record_id` is stable.

### New Tables

Recommended tables:

```text
app_settings(
  id TEXT PRIMARY KEY CHECK (id = 'singleton'),
  onboarding_completed INTEGER NOT NULL,
  default_provider_id TEXT NULL,
  default_template_id TEXT NULL,
  default_destination_id TEXT NULL,
  global_shortcut TEXT NULL,
  global_shortcut_enabled INTEGER NOT NULL,
  last_environment_check_json TEXT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
)
```

```text
destinations(
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL,
  label TEXT NOT NULL,
  enabled INTEGER NOT NULL,
  credential_ref TEXT NULL,
  config_json TEXT NOT NULL,
  last_tested_at TEXT NULL,
  last_test_status TEXT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
)
```

```text
templates(
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT NOT NULL,
  compatible_source_types_json TEXT NOT NULL,
  prompt_profile TEXT NOT NULL,
  output_schema TEXT NOT NULL,
  destination_mappings_json TEXT NOT NULL,
  enabled INTEGER NOT NULL,
  system INTEGER NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
)
```

```text
source_capability_snapshots(
  id TEXT PRIMARY KEY,
  agent_reach_version TEXT NULL,
  doctor_json TEXT NOT NULL,
  normalized_json TEXT NOT NULL,
  created_at TEXT NOT NULL
)
```

### Secrets Storage

Recommended keychain abstraction:

```rust
trait SecretStore {
    fn set_secret(&self, key: &str, value: &str) -> Result<(), SecretError>;
    fn get_secret(&self, key: &str) -> Result<Option<String>, SecretError>;
    fn delete_secret(&self, key: &str) -> Result<(), SecretError>;
}
```

First implementation may keep current Notion SQLite storage only as migration source. New destination credentials should prefer the keychain abstraction. If keychain is not ready, do not add more plaintext secret surfaces without explicit approval.

> **[审查批注 P0-1]** 此处「should prefer / if not ready」太软,与 Slice 5 无 keychain 依赖组合会直接产生明文 webhook URL。**收紧为硬约束**:webhook 类目的地的 secret(webhook URL 视为 secret)**必须**经 `SecretStore` 存取,`destinations.config_json` 只允许非敏感字段。`SecretStore` 落地是 Slice 5 的前置(见新增 Slice 4.5)。现存 Notion token 明文存储在本阶段一并迁移到 `SecretStore`,不作为扩大明文的先例。

## Public Command/API Changes

### New Tauri Commands

Recommended:

```ts
get_app_settings(): AppSettingsView
save_app_settings(input): AppSettingsView
get_environment_status(): EnvironmentStatus
run_agent_reach_doctor(): SourcePlatformStatus[]
list_templates(): ResearchTemplate[]
save_default_template(template_id: string): AppSettingsView
list_destinations(): DestinationSummary[]
save_destination(input): DestinationSummary
test_destination_connection(destination_id: string): string
set_default_destination(destination_id: string | null): AppSettingsView
register_global_capture_shortcut(shortcut: string): ShortcutStatus
unregister_global_capture_shortcut(): ShortcutStatus
capture_from_clipboard(): Task
```

### Existing Commands To Extend

```ts
create_capture_task(url, note, providerId)
```

should become:

```ts
create_capture_task({
  url,
  note,
  providerId,
  templateId,
  destinationId,
  sourceType
})
```

Backward compatibility:

- Keep the old argument form only if current frontend/tests still depend on it.
- Default missing `templateId` to `web_article`.
- Default missing `destinationId` to app settings.

## UI Requirements

### Onboarding UI

New first-launch screen should be a product workflow, not a marketing page.

Required sections:

1. `环境检测`
   - AI provider cards
   - Agent-Reach availability
   - platform count summary: ready / needs config / unavailable
2. `选择默认 AI`
   - recommended provider selected by default
   - unavailable providers disabled with reason
3. `连接目标`
   - Notion / 飞书 / 企业微信 / 钉钉 cards
   - config form for selected destination
   - test connection button
4. `完成`
   - summary of selected provider/template/destination
   - enter Queue

### Settings UI

Settings should become the place to revisit onboarding:

- AI provider section:
  - status, default selector, test button where possible
- Agent-Reach section:
  - doctor run button
  - platform matrix
  - setup hints
- Destination section:
  - list configured destinations
  - default selector
  - add/update/test destination
- Shortcut section:
  - enable toggle
  - shortcut selector
  - registration status
- Privacy section:
  - local queue path
  - credential storage status
  - clear local settings action only with explicit confirmation in a future slice

### Capture UI

Capture page should add:

- source type/platform detected from URL
- template selector
- destination selector
- “平台不可用/需配置” warning before submit

CTA rules:

- disabled if URL invalid
- disabled if selected provider unavailable
- allowed if destination missing only when user explicitly chooses local-only analysis
- disabled or warning if selected source requires login/config and is unavailable

### Queue UI

Queue row should display:

- title
- source platform
- template
- destination
- status
- time
- score
- model
- action link based on destination kind

For webhook destinations, action link may be absent unless the platform response returns a URL.

## Implementation Slices

Each slice must be independently mergeable and leave the app usable.

### Slice 1: Settings and First-Launch Foundation

Scope:

- Add `app_settings`.
- Add environment detection command.
- Persist default provider/template/destination placeholders.
- Show onboarding when `onboarding_completed=false`.
- Keep current Notion sync path working.

> **[审查批注 P1-4]** Slice 1 scope 追加一条硬性交付:**拆分 `src/App.tsx`**。本阶段要叠加 onboarding(4 步)+ settings(5 段)+ 平台矩阵 + 目的地表单,单文件必然失控。至少拆出 `src/onboarding/`、`src/settings/`、`src/capture/`、`src/queue/` 与共享类型模块。这不是 Risk 表里的「if」,是 Slice 1 的验收项(拆分后 `pnpm typecheck`/`pnpm build` 仍过、UI 无回归)。

Acceptance:

- New install opens onboarding.
- Existing install can continue to Queue if settings migrate with sensible defaults.
- Provider selection persists after restart.
- `pnpm typecheck`, `pnpm build`, `cargo test -p reachnote-core`, `cargo test --manifest-path src-tauri/Cargo.toml`, `cargo check --manifest-path src-tauri/Cargo.toml` pass.
- Tauri dev desktop smoke verifies onboarding and settings.

### Slice 2: Agent-Reach Platform Matrix

Scope:

- Add `run_agent_reach_doctor` command.
- Normalize 15 platform statuses.
- Render platform matrix in Settings.
- Store last snapshot.
- Do not change capture flow yet except showing detected source warnings.

Acceptance:

- Missing `agent-reach` shows actionable error.
- Doctor JSON parse failure shows readable error.
- Local doctor snapshot with ready/warn/off platforms renders without overflow.
- No platform is marked ready unless doctor says usable or local fallback is implemented.

### Slice 3: Template Registry

Scope:

- Add built-in templates.
- Add `templates` table or core static registry with view command.
- Capture page selects template.
- `AnalysisRequest` prompt changes based on template.
- Task stores template metadata.

Acceptance:

- Web URL defaults to `web_article`.
- GitHub URL defaults to `github_project`.
- Template survives task reload.
- AI prompt unit tests prove template profile is included.
- Queue displays template label.

### Slice 4: Multi-Destination Connector Foundation

Scope:

- Add destination abstraction.
- Keep Notion as first destination through adapter.
- Introduce neutral task fields: `destination_id`, `destination_kind`, `destination_record_id`, `destination_url`.
- Migrate current Notion tasks without data loss.

Acceptance:

- Existing Notion flow still passes.
- Failed-with-analysis retry still syncs without reanalysis.
- Notion-specific errors no longer leak into generic destination UI except Notion adapter.

> **[审查批注 P1-2]** Slice 4 必须显式处理 `sync_pending_analyzed_tasks`(启动补同步)与多目的地的交互:① 按 task `destination_id` 路由到对应 adapter;② `destination_id` 为空的任务**跳过**、保持 `Analyzed`,**不得**标 `Failed`;③ 现有崩溃恢复 `recover_interrupted_tasks` 要能处理 `destination_id=NULL` 的存量任务。验收补一条:「一个无目的地的 `Analyzed` 任务,重启后仍是 `Analyzed`,未被后台扫描改写」。

### Slice 4.5: Secret Store (keychain abstraction)

> **[审查批注 P0-1 · 新增切片,Slice 5 的前置]**

Scope:

- 实现 `SecretStore` trait(见 Data Model / Secrets Storage),首选 OS keychain(`keyring` crate)。
- 提供 SQLite fallback 仅用于「无 keychain 的开发环境」,且必须在设置页明示当前 credential storage 状态(keychain / 本地明文)。
- 把现有 Notion token 从 `notion_settings` 迁移到 `SecretStore`(保留读旧值的一次性迁移,迁移后清除明文)。

Acceptance:

- `SecretStore` set/get/delete 有单测(mock 或临时 keychain namespace)。
- Notion 同步在迁移后仍通过真实 E2E(token 走 SecretStore)。
- 设置页显示凭证存储位置;无明文 secret 出现在日志/memory/快照。
- 若目标机无可用 keychain,fallback 行为有明确告警,不静默明文存储新 secret。

### Slice 5: Webhook Destinations

Scope:

- Add 飞书 webhook.
- Add 企业微信 webhook.
- Add 钉钉 webhook.
- Save/test/sync structured text cards.

> **[审查批注 P0-1]** Slice 5 **依赖 Slice 4.5**。所有 webhook secret(URL 视为 secret)经 `credential_ref` → `SecretStore` 存取;`config_json` 只存非敏感字段。未先完成 4.5 不得开工 Slice 5。

Acceptance:

- Each destination can be configured and tested with a non-sensitive test message.
- Sync payload includes title, summary, key points, tags, score, source URL, model.
- Failed webhook response is visible in queue.
- Webhook URLs are masked and never printed.

### Slice 6: Global Capture Shortcut

Scope:

- Install Tauri global-shortcut plugin.
- Register default shortcut.
- Add shortcut settings.
- Implement `capture_from_clipboard`.

> **[审查批注 P2-5]** Slice 6 scope 补两项依赖前置:① `capture_from_clipboard` 读剪贴板需 `tauri-plugin-clipboard-manager`(或等价 capability);② macOS 全局快捷键在部分系统需辅助功能(Accessibility)授权,注册失败要区分「快捷键冲突」与「缺少系统授权」并各自给引导。另核实 `tauri.conf.json` 的 hide-to-background 设置确实使进程在隐藏后存活(Acceptance 已要求,scope 要点名)。

Acceptance:

- Shortcut registers on app start when enabled.
- Valid clipboard URL creates task using defaults.
- Invalid clipboard opens Capture with error.
- Hidden-to-background app continues to process task.
- Shortcut conflict is handled and shown.

### Slice 7: First Platform Expansion

Scope:

- Enable real source routing beyond article for GitHub/RSS/YouTube/B站 where local doctor supports it.
- Do not include login-heavy platforms yet.

Acceptance:

- GitHub repo still passes real E2E.
- RSS feed can create summary task from a controlled test feed.
- YouTube/B站 gracefully fail when transcript backend is missing and tell user what to install.

## Verification Plan

### Required Commands For Code Slices

```bash
pnpm typecheck
pnpm build
cargo test -p reachnote-core
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
git diff --check
```

### Required Desktop QA

For each user-visible slice:

- Run `pnpm tauri dev`.
- Verify real Tauri window.
- Check onboarding/settings/capture/queue navigation.
- Check text overflow at `1180x780` and min size `900x680`.
- Check loading, empty, error, disabled, retry states.
- Check no raw secrets are visible in screenshots or console.
- If Computer Use remains blocked, record AX fallback status in `memory/desktop-qa.md`.

### Required External QA

For destination slices:

- Use fake/local mock tests for automated CI-like coverage.
- Use one real manual smoke per enabled real destination only when credentials are configured by the user.
- Never commit or paste real credentials.

For Agent-Reach:

- Use `agent-reach doctor --json` as setup truth.
- Test missing command path.
- Test malformed JSON path.
- Test at least one ready, one warn, one off platform row.

## Rollback Plan

Each slice should be reversible:

- `app_settings` can default to Queue and current provider if onboarding has a bug.
- Platform matrix is read-only until source routing is enabled.
- Templates can fall back to `web_article`.
- Destination abstraction can keep Notion as default and ignore webhook destinations.
- Shortcut can be disabled by setting `global_shortcut_enabled=false`.

Database migrations should be additive in this phase. Do not delete existing `notion_page_id` or `analysis_json`.

## Risks

| Risk | Impact | Mitigation |
| --- | --- | --- |
| Agent-Reach lacks generic read command | Platform routing cannot be a single CLI call. | Use doctor as capability truth and maintain per-source adapters. |
| Login-heavy platforms fail due to cookies/risk controls | User sees “supported” but cannot capture. | Mark as needs login/config; defer real ingestion until explicit backend is available. |
| More destinations increase secret handling risk | Webhook URLs and tokens can leak. | Add keychain abstraction before adding more plaintext credentials. |
| Webhook MVP is weaker than full database sync | Users may expect searchable structured database. | Label as “消息同步” and reserve full API/database sync for later. |
| Shortcut conflicts with OS/app shortcuts | Capture silently fails. | Registration status and preset selector; disabled fallback. |
| Frontend remains single giant `App.tsx` | More settings/onboarding UI becomes hard to maintain. | Split components as part of Slice 1 or 2 if file growth blocks clarity. |

## Open Questions

These do not block Slice 1, but Claude should challenge them during review:

1. Should webhook destinations be allowed before OS keychain is implemented? Recommended answer: only if stored through a `SecretStore`; otherwise keep them planned.
2. Should tasks support local-only analysis when no destination is configured? Recommended answer: yes, but status must be `analyzed` or `pending_destination`, not `synced`.
3. Should Agent-Reach install/configure run from ReachNote UI? Recommended answer: not in this phase; show guidance and doctor output first.
4. Should custom templates ship in this phase? Recommended answer: no; system templates first.
5. Should global shortcut capture browser current URL instead of clipboard? Recommended answer: not in this phase; clipboard first, browser integration later.

> **[审查裁决]** 逐条给出终审答案:
> 1. **收紧为「必须」**:webhook 不得在 `SecretStore` 之前落地(P0-1)。已新增 Slice 4.5 作前置。
> 2. **定为 `Analyzed`**,删除 `pending_destination`(P1-1)。`analyzed`=本地就绪未同步,是终态之一,不是失败。
> 3. **同意**:本阶段只 doctor + 引导,不从 UI 触发 install/configure。
> 4. **同意**:仅系统模板;且本阶段模板只改 prompt、共用 `research_card_v1`,不分裂 schema(P2-3)。
> 5. **同意**:先剪贴板。补充:`capture_from_clipboard` 需 `tauri-plugin-clipboard-manager`,macOS 全局快捷键可能需辅助功能授权(P2-5)。

## Claude Review Instructions

Please review this PRD as a read-only product/engineering gate.

Focus on:

- P0/P1 blockers that would make this plan unsafe or non-mergeable.
- Whether the phase slicing is independently shippable.
- Whether the data model preserves existing Notion tasks and retry behavior.
- Whether the platform matrix truthfully handles Agent-Reach v1.5.0 without inventing a generic `read` command.
- Whether multi-destination webhook MVP is too shallow or correctly scoped.
- Whether onboarding should block capture when AI provider or destination is missing.
- Whether any secret-handling step risks leaking webhook URLs, Notion tokens, API keys, cookies, or user content.

Expected output:

```text
Gate: PASS | FAIL
P0 findings:
P1 findings:
P2 findings:
Recommended changes before implementation:
```

Do not modify files during review.
