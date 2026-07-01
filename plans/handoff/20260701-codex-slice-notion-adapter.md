# Codex 开发 Prompt — ReachNote 第 6 步:Notion Adapter(Analyzed → Synced)

> 整段作为 prompt 交给 Codex。自洽:含前置状态、边界、设计决策、字段映射、验收、禁止项。
> 生成日期:2026-07-01 · 真源:`plans/prds/20260630-1906-reachnote-mvp-reset.prd.md`、`README.md`「Notion Database Schema」
> 对应 PRD「Small-Step Rule」**第 6 步:Notion adapter**。这是 First Vertical Slice 的最后一块:完成后 `Article URL → 队列 → AI 研究卡 → Notion → 队列状态` 全链路闭环。

---

## 你的角色与目标

你是 ReachNote 仓库的实现 agent。前五刀已完成并验证:本地队列 + 三 provider 结构化分析。当前 `run_capture_task` 把任务推进到 **`Analyzed`**(本地研究卡,存在 `Task.analysis_json`),**到此为止——从不写 Notion**。

本阶段目标:实现 **Notion adapter**,把任务从 `Analyzed` 推进到 **`Synced`**——把研究卡写入用户的 Notion database,落 `notion_page_id` 和 `synced_at`。

**一句话验收**:`pnpm tauri dev` 下采集一个真实 URL → 队列走到「已分析」→ 触发同步 → 状态变「已完成」→ **去 Notion database `ReachNote Research Inbox` 能看到一条新 page**,字段(标题/URL/摘要/标签/评分/来源类型…)填对;用错误凭证时该任务变「失败」并显示可读错误、可重试同步。

> 测试凭证、database、连通性**已就绪**(上一轮 `scripts/notion-smoke.sh` 已端到端跑通)。本阶段**不接 OAuth、不接 keychain、不引入任何新 crate**。

---

## 前置状态(已存在,在其上扩展,不要重写)

用 `.codegraph/` 或 targeted read 确认:

- `crates/core/src/analysis.rs`:
  - **`AnalysisResult { title:String, summary:String, key_points:Vec<String>, tags:Vec<String>, score:u8, next_action:String, model:String }`** —— 这是要映射到 Notion 的研究卡。`score` 约束 `1..=5`。
  - `ProviderId`、`AnalysisRequest`、`parse_analysis_result`、`build_analysis_prompt`。
- `crates/core/src/task.rs`:
  - `TaskStatus`:`Queued/Reading/Analyzing/Analyzed/Syncing/Synced/Failed` —— **`Syncing`/`Synced` 已定义但未使用,本阶段启用它们**。
  - `ErrorKind`:含 **`NotionUnauthorized`**、`SchemaMismatch`、`NetworkFailed` —— 已为 Notion 预留,直接用。
  - `Task`:含 `provider_id`、`note`、`analysis_json`、**`notion_page_id:Option<String>`**、`synced_at:Option<String>` —— 字段全预留好,初始 `None`。
- `src-tauri/src/store.rs`:`update_task` 已 UPDATE 全字段含 `notion_page_id`(?13)、`synced_at`(?18)、`status`(?5);`get_task` 可用。**直接复用,无需改 store(除非加测试)。**
- `src-tauri/src/lib.rs`:commands `shell_status/create_capture_task/list_capture_tasks/run_capture_task`。`run_capture_task` 流程 `Reading →(reader)→ Analyzing →(ProviderRunner)→ Analyzed`,成功时写 `title/score/model/analysis_json`。**保持它止于 `Analyzed` 不变。**
- `src-tauri/src/provider.rs`:**`analyze_openai_compatible` 已用 `reqwest::blocking::Client`**(`.timeout()`/`.json()`/`.bearer_auth()`/`.send()`/`.status()`/`.text()`)——**Notion adapter 照这个写**。`from_env()` + `required_env(env_key,label)->ProviderError` 是本仓库的配置约定,照搬。
- `src-tauri/Cargo.toml`:已有 `reqwest = { version = "0.12", default-features = false, features = ["blocking","json","rustls-tls"] }`、`serde_json`。**这就是你全部需要的,别加新依赖。**

环境/凭证(已就绪,本机已验证):
- 测试 database **`ReachNote Research Inbox`** 已建,13 字段(`Title`/`URL`/`Source Type`/`Summary`/`Key Points`/`Tags`/`Status`/`Score`/`Captured At`/`Synced At`/`AI Model`/`Template`/`Next Action`),已 share 给 integration。
- 凭证在仓库根 **`.env.notion`**(gitignored):`NOTION_TOKEN`、`NOTION_DATABASE_ID`、`NOTION_VERSION=2022-06-28`。`scripts/notion-smoke.sh` 已用它端到端跑通(POST /v1/pages 成功)。
- 字段映射真源:`README.md`「Notion Database Schema」;测试前置说明:`plans/handoff/20260701-notion-adapter-prerequisites.md`。

---

## 已拍板的决策(照此执行)

1. **Score 口径**:内部与前端保持 `1-5`(`AnalysisResult.score` 不动);**写 Notion 时 `score * 20 → 0-100`**(README 的 Number 0-100)。映射在 core 完成。
2. **零新 crate**:`reqwest::blocking` + `serde_json` 已在,全部够用。**禁止** OAuth 库、keychain 库、新 HTTP 栈、chrono 等。
3. **认证 = internal token**,从 `NOTION_TOKEN` 环境变量读(不是 OAuth)。**变量名不带 `REACHNOTE_` 前缀**(要和 `.env.notion`/自检脚本一致,测试时 `source .env.notion` 即生效)。
4. **parent = `{"database_id": NOTION_DATABASE_ID}`** + `Notion-Version: 2022-06-28`(自检已验证此路径)。不走 2025-09-03/data_source_id(留作未来)。
5. **同步是独立 command,不并进 `run_capture_task`**:本地分析(`Analyzed`)与 Notion 同步(`Synced`)解耦,Notion 失败不抹掉已生成的本地研究卡,且可独立重试。前端编排「`run` 成功后自动接 `sync`」。

---

## 实现范围

严守 PRD P1 边界:**core 拥有 Notion 字段映射逻辑(纯函数,不依赖 reqwest/Tauri)**;**src-tauri 拥有 HTTP 调用 + command + SQLite 写回**;**前端只触发与渲染**。

### 第 1 步 — core:研究卡 → Notion properties 映射(`crates/core/src/notion.rs`)

新建 `crates/core/src/notion.rs`,`lib.rs` 加 `pub mod notion;`。纯函数,不碰网络。

- `pub fn build_notion_properties(task: &Task, analysis: &AnalysisResult, synced_at_iso: &str) -> serde_json::Value`:
  产出 Notion `properties` object(`serde_json::json!`),字段严格对齐已建 database 的 property 名与类型(见下方映射表)。
- **Select 值映射**(必须命中 database 已有 option,否则 Notion 报 validation_error):
  - `source_type_to_select(&str) -> &str`:`"article" → "Article"`(其余 `github→GitHub`/`video→Video`/`rss→RSS`/`social→Social`,未知给 `"Article"`)。
  - `template_to_select(&str) -> &str`:`"article" → "Article Reading Note"`(`github→GitHub Project Analysis`/`video→Video Note`/`rss→RSS Digest`,未知给 `"Article Reading Note"`)。
  - `Status` 固定初始值 `"Inbox"`。
- **Score**:`(analysis.score as i64) * 20`(→ Number 0-100)。
- `synced_at_iso` 由调用方(src-tauri)传入 RFC3339 字符串,core 不算时间(保持纯)。`Captured At` 同理:把 `task.created_at`(unix 秒字符串)在 src-tauri 转成 RFC3339 再传入,或由 core 接收一个 `captured_at_iso` 参数。
- **core 单测(≥3)**:① Score 1→20、5→100;② `source_type`/`template` 映射命中预期字符串;③ 生成的 properties 含 `Title.title`/`URL.url`/`Tags.multi_select`/`Score.number` 且结构正确。不调真实 Notion。

> Notion property JSON 形态(照此构造):
> - title: `{"Title":{"title":[{"text":{"content": <title>}}]}}`
> - rich_text: `{"Summary":{"rich_text":[{"text":{"content": <text>}}]}}`(`Key Points` 用 `key_points.join("\n")`,`AI Model`/`Next Action` 同理)
> - url: `{"URL":{"url": <url>}}`
> - select: `{"Source Type":{"select":{"name": <value>}}}`(`Status`/`Template` 同)
> - multi_select: `{"Tags":{"multi_select":[{"name": <tag>}, …]}}`
> - number: `{"Score":{"number": <score*20>}}`
> - date: `{"Captured At":{"date":{"start": <iso>}}}`(`Synced At` 同)
> rich_text/title 的 content 有 2000 字符上限,超长要截断。

### 第 2 步 — src-tauri:NotionClient(`src-tauri/src/notion.rs`)

仿 `provider.rs::analyze_openai_compatible` 用 `reqwest::blocking`。

- `pub struct NotionClient { token, database_id, version, timeout }`,`pub fn from_env() -> Result<Self, NotionError>`:
  - `NOTION_TOKEN`、`NOTION_DATABASE_ID` 必填(缺失 → `NotionError{ kind: NotionUnauthorized, message: "未配置 NOTION_TOKEN/…" }`,照搬 `required_env` 风格)。
  - `NOTION_VERSION` 默认 `"2022-06-28"`。timeout 可复用 `REACHNOTE_AI_TIMEOUT_SECS` 或固定 30s。
- `pub fn create_page(&self, properties: serde_json::Value) -> Result<String, NotionError>`:
  - `POST https://api.notion.com/v1/pages`,header `Authorization: Bearer {token}`、`Notion-Version: {version}`、`Content-Type: application/json`,body `{"parent":{"database_id": self.database_id}, "properties": properties}`。
  - 成功(2xx):解析响应 `id` 字段返回(即 `notion_page_id`)。无 `id` → `ParseFailed`。
  - **错误分类**:`401/403 → NotionUnauthorized`;`404 → NotionUnauthorized`(database 没 share/不存在,message 提示去 share);`400 → SchemaMismatch`(property/option 不匹配,把 Notion 的 `message` 截断附上);其他非 2xx → `NetworkFailed`;连接/超时 → `NetworkFailed`。
  - `NotionError { kind: ErrorKind, message: String }`,与 `ProviderError` 同形。**绝不把 token 写进 message。**
- `NotionError` 用 `reachnote_core::task::ErrorKind`。

### 第 3 步 — src-tauri:`sync_capture_task` command(`lib.rs`)

新增并注册进 `invoke_handler`:

- `#[tauri::command] fn sync_capture_task(store, id: String) -> Result<Task, String>`:
  1. `get_task(id)`;要求 `status` 为 `Analyzed`(或 `Failed` 且 `analysis_json` 非空 → 允许重试同步)。否则返回可读 `Err`。
  2. 从 `task.analysis_json` 反序列化出 `AnalysisResult`(失败 → `ParseFailed`)。
  3. `status = Syncing`、刷新 `updated_at`、清空旧 `error_*`,`update_task`。
  4. 在 src-tauri 计算 `synced_at` 与 `captured_at` 的 RFC3339 字符串(见下「时间」),调 `build_notion_properties`,再 `NotionClient::from_env()?.create_page(props)`。
  5. **成功**:`status=Synced`、`notion_page_id=Some(page_id)`、`synced_at=Some(<unix 秒>)`、清空 `error_*`,`update_task`,返回 task。
  6. **失败**:`status=Failed`、写 `error_kind/error_message`、刷新 `updated_at`,`update_task`,返回 task(同步失败是任务终态,不是 command 调用错误)。
- **保持 `run_capture_task` 不变**(仍止于 `Analyzed`)。

**时间**:Notion date 要 RFC3339。`Task.synced_at`/`created_at` 是 unix 秒字符串。在 src-tauri 转 RFC3339:用已在依赖的 `time` crate(给 `Cargo.toml` 的 `time` 行加 `features = ["formatting"]`,**不改版本 pin `=0.3.41`**),`OffsetDateTime::from_unix_timestamp(secs)?.format(&Rfc3339)`。
> 若 `time` formatting 引入任何麻烦:`Captured At`/`Synced At` 这两个 **Date 字段可先省略**(标 `// TODO`),核心闭环不依赖它们——其余 11 个字段必须写。不要为时间格式化卡住主链路。

### 第 4 步 — 前端 `src/App.tsx`:触发同步 + Notion 链接

- **编排**:`run_capture_task` 返回 `analyzed` 后,**自动 `invoke("sync_capture_task", { id })`**;期间状态走 `syncing`(已映射为「处理中」),完成后刷新为 `synced`(「已完成」)或 `failed`。
- **Synced 行**:显示「已完成」,并给一个打开 Notion 的链接 —— `https://www.notion.so/${notion_page_id.replaceAll('-','')}`。
- **失败行**:显示 `error_message`;「重试」按钮对同步失败的任务 `invoke("sync_capture_task", { id })`(对分析失败的仍走 `run_capture_task`——按 `status`/`error_kind` 区分,或统一一个「重试」入口按当前 status 决定调哪个 command)。
- 覆盖 PRD Required States:syncing(loading)、synced(success + 链接)、error(可读 + 重试)。**不重设计 UI**,沿用现有 `styles.css` 与状态映射(`syncing/synced` 已存在)。
- 前端 `Task` interface 已有 `notion_page_id`;确认渲染逻辑用到它。

---

## 字段映射表(research card + task → Notion `ReachNote Research Inbox`)

| Notion property | 类型 | 来源 |
| --- | --- | --- |
| `Title` | title | `analysis.title` |
| `URL` | url | `task.url` |
| `Source Type` | select | `task.source_type` → `source_type_to_select`(`article→Article`) |
| `Summary` | rich_text | `analysis.summary` |
| `Key Points` | rich_text | `analysis.key_points.join("\n")` |
| `Tags` | multi_select | `analysis.tags` → `[{name}]` |
| `Status` | select | 固定 `"Inbox"` |
| `Score` | number | `analysis.score * 20`(1-5 → 0-100) |
| `Captured At` | date | `task.created_at`(unix→RFC3339)·可选 |
| `Synced At` | date | now(RFC3339)·可选 |
| `AI Model` | rich_text | `analysis.model` |
| `Template` | select | `task.template_id` → `template_to_select`(`article→Article Reading Note`) |
| `Next Action` | rich_text | `analysis.next_action` |

---

## 为什么是这个边界

- PRD 第 6 步就是 Notion adapter,且这是 First Vertical Slice 的最后一块——完成即闭环 `Article URL → Notion`。
- **不做**(留后续):OAuth、OS keychain、设置页里的 Notion 连接 UI(MVP 用 `.env.notion` 环境变量)、字段自定义映射、双向同步、把 database_id 持久化进 app config、`Status` 的后续流转。
- 之所以独立 `sync` command 而非并进 `run`:本地优先——`Analyzed` 的研究卡已经是有价值的本地产物,Notion 同步是可失败、可重试的下游副作用,解耦才能独立重试且不互相污染状态。

---

## 验收命令(全部通过,粘贴真实输出)

```bash
pnpm typecheck
pnpm build
cargo test -p reachnote-core                          # 含本阶段新增 ≥3 个 notion 映射单测
cargo check --manifest-path src-tauri/Cargo.toml

# 真实端到端(本阶段硬性证据):让 .env.notion 进入 dev 进程环境
set -a; source .env.notion; set +a
pnpm tauri dev
```

`pnpm tauri dev`(已 source `.env.notion`)**人工冒烟**:

1. 采集真实 URL(如 `https://openai.com/index/hello-gpt-4o`)→ 队列走「处理中→已分析→处理中(同步)→**已完成**」。
2. **打开 Notion database `ReachNote Research Inbox`,确认多出一条 page**:Title/URL/Summary/Tags/Score(0-100)/Source Type=Article/Status=Inbox/AI Model/Next Action 都填对。
3. 队列该行的 Notion 链接能打开这条 page。
4. **失败用例**:临时 `export NOTION_TOKEN=ntn_invalid` 再 `pnpm tauri dev`,采集 → 任务走到「失败」,错误为 `notion_unauthorized` 的可读提示,**重试**按钮可点;恢复正确 token 后重试 → 变「已完成」。
5. 关闭 app 重开 → 已 `synced` 任务仍在,`notion_page_id` 保留。

> 如实报告:Notion 写入是否真的发生(给出新 page 在 Notion 的可见证据)、哪些字段没映射(如 Date 字段若省略)。**不要在没有真实 Notion 写入验证的情况下声称闭环完成**(PRD Validation Bar:不能只靠 mock 或构建通过)。

---

## 收尾:更新 memory(同一轮)

- `memory/backend-progress.md`:新增 `core::notion` 映射 + `src-tauri::NotionClient` + `sync_capture_task`;状态机启用 `Syncing/Synced`;改动文件、验证结果、残留风险。
- `memory/frontend-progress.md`:run 后自动接 sync、Notion 链接、同步失败重试;覆盖的 syncing/synced/error 状态。
- `memory/integration-progress.md`:链路从 `Analyzed` 推进到 **完整闭环 `Article URL → 队列 → AI 研究卡 → Notion page → Synced`**;记录测试用 database/凭证来自 `.env.notion`;**First Vertical Slice 是否达成 PRD「First Proof Point」**;下一刀(OAuth/keychain/设置页 Notion 连接、或 Agent-Reach 真实读取)。
- 记录:状态、改动文件、已验证命令(含真实 Notion 写入证据)、未闭环风险、日期。

---

## 硬性禁止(违反任一即偏离任务)

- ❌ 不引入任何新 crate(只用已在的 `reqwest::blocking`/`serde_json`/`time`);`time` 可加 `features`,但**不改版本 pin `=0.3.41`**。
- ❌ 不做 OAuth、不接 OS keychain、不在本阶段加设置页 Notion 连接 UI(MVP 用 `.env.notion` 环境变量)。
- ❌ 不改 `task.rs` 的 `TaskStatus`/`ErrorKind`/`Task` 字段定义(用已有的 `Syncing/Synced/notion_page_id/synced_at/NotionUnauthorized`)。
- ❌ 不改 `create_capture_task`/`run_capture_task` 的现有行为(`run` 仍止于 `Analyzed`;同步是新 command)。
- ❌ 不自行重设计 UI;沿用 `memory/design-source.md` 与现有 `styles.css`。
- ❌ 不把 `NOTION_TOKEN`/任何 token 写进 repo、日志、memory、`analysis_json`、`error_message`、测试快照。
- ❌ 不 commit / push(由用户决定);只改工作树并跑验证。
- ❌ 不在缺少「真实 Notion 写入」证据时声称闭环完成。
