# Codex 开发 Prompt — ReachNote 下一阶段:Claude CLI Provider(Queued → 本地研究卡)

> 把本文件**整段**作为 prompt 交给 Codex。它自洽:含前置状态、边界、设计决策、实现范围、验收命令、禁止事项。
> 生成日期:2026-06-30 · 真源:`plans/prds/20260630-1906-reachnote-mvp-reset.prd.md`
> 对应 PRD「Implementation Discipline / Small-Step Rule」**第 5 步:Claude CLI provider**。

---

## 你的角色与目标

你是 ReachNote 仓库的实现 agent。上一阶段(本地队列闭环)**已完成并验证通过**:`crates/core` 有任务领域类型与校验,`src-tauri` 有 SQLite 队列和 `create_capture_task` / `list_capture_tasks`,前端已用 `invoke` 渲染真实队列。当前一条任务被创建后**停在 `Queued`,不会前进**——没有任何 AI 分析。

本阶段目标:**让任务从 `Queued` 真正推进到「AI 生成的本地研究卡」**,实现 PRD 的 `AiProvider` 抽象和第一个 provider `ClaudeCliProvider`,并把研究卡结果落库、回显到队列和采集页预览。

**一句话验收**:用户在采集页贴合法 URL 点 CTA → 队列里该任务状态从「处理中」推进 → 几秒后变「已完成」,该行显示**真实的 AI 生成标题、评分、模型**,采集页右侧预览从骨架变成**真实研究卡**(标题/摘要/要点/标签/下一步行动/评分);失败时该行显示**可读错误**且**重试按钮可用**。

> **本阶段不接 Notion、不接 agent-reach 真实读取、不引入任何新 crate。** 理由见下方「为什么是这个边界」。

---

## 前置状态(已存在,不要重写,只在其上扩展)

开工前用 `.codegraph/`(优先 `codegraph_explore`)或 targeted read 确认以下现状:

- `crates/core/src/task.rs`:`TaskStatus`(`queued/reading/analyzing/syncing/synced/failed`,snake_case 序列化,有 `as_str`/`from_str`)、`ErrorKind`(7 类,含 `provider_unavailable`/`parse_failed`/`schema_mismatch`/`network_failed`)、`Task`(15 字段)、`validate_article_url`、`source_domain`。**复用这些,不要改其定义。**
- `crates/core/src/lib.rs`:已 `pub mod task;`。
- **`crates/core/src/ai/` 是已存在的空目录** —— 本阶段的 AI provider 模块就放这里。
- `src-tauri/src/store.rs`:`TaskStore`(`Mutex<Connection>`),已有 `insert_task` / `list_tasks` / `get_task`(get_task 现标了 `#[allow(dead_code)]`,本阶段会用到它,去掉该标注)。**目前没有 `update_task`,你要新增。**
- `src-tauri/src/lib.rs`:3 个 command（`shell_status` / `create_capture_task` / `list_capture_tasks`);`create_capture_task` 构造 `Task{ status: Queued, model: Some("Claude CLI"), ... }` 后 `insert_task` 返回。DB 在 `app_data_dir()/reachnote.db`。
- `src/App.tsx`:已 `import { invoke }`;`type TaskStatus` 与 Rust 对齐;状态分组已就绪(`queued/reading/analyzing/syncing → 处理中`,`synced → 已完成`,`failed → 失败`);**采集页右侧是骨架预览;队列失败行已有「重试」按钮 UI,但后端还没有 retry command,本阶段要补上让它生效。**

环境事实(本机已验证):
- `claude` 已装(`/opt/homebrew/bin/claude`,v2.1.185),支持 `-p/--print`、`--output-format json`、`--permission-mode`。✅ 本阶段可真实端到端验证。
- `agent-reach` 已装,但其子命令是 `{setup, doctor, configure, format, transcribe, skill, ...}` —— **没有「给 URL 返回正文」的直接命令**。所以本阶段**不**用 agent-reach 做内容读取(见决策 3)。

---

## 必读上下文

1. `AGENTS.md` — 协作规则、Source Of Truth 优先级、Design Fidelity Protocol。
2. `plans/prds/20260630-1906-reachnote-mvp-reset.prd.md` — 重点:`### State Model`、`## P1 Architecture Map`(core 拥有 AI provider 契约、parse/validation;src-tauri 拥有 subprocess wiring;**core 不依赖 Tauri**)、`### Error Paths`、`## Implementation Discipline`。
3. `docs/adr/0001-tech-stack.md` — 「关键设计:AI Provider 抽象」一节给了 trait 形态与三 provider 路由意图。
4. `memory/backend-progress.md` / `frontend-progress.md` / `integration-progress.md` — 当前进度事实。

---

## 本阶段实现范围

严格守 PRD P1 边界:**core 拥有 provider 契约 / 研究卡类型 / JSON 解析与校验,且不依赖 Tauri、不 spawn 进程**;**src-tauri 拥有 `claude` 子进程编排、command、SQLite 写回**;**前端只渲染与调 command**。

### 第 1 步 — core:AiProvider 契约 + 研究卡类型 + 解析校验(放 `crates/core/src/ai/`)

在空的 `crates/core/src/ai/` 下建模块(如 `mod.rs` 暴露子项),并在 `lib.rs` 加 `pub mod ai;`。

- **研究卡结果类型** `AnalysisResult`(`Serialize, Deserialize, Clone, Debug, PartialEq`):
  ```rust
  pub struct AnalysisResult {
      pub title: String,
      pub summary: String,
      pub key_points: Vec<String>,
      pub tags: Vec<String>,
      pub next_actions: Vec<String>,
      pub score: u8,        // 1..=5
      pub model: String,
  }
  ```
- **分析请求** `AnalysisRequest { url: String, note: Option<String>, content: Option<String> }`。本阶段 `content` 恒为 `None`(无真实读取);保留该字段是为下一阶段 agent-reach 填入正文留扩展点。
- **错误类型** `AiError { kind: ErrorKind, message: String }`,复用 `task::ErrorKind`。
- **provider trait**(**同步**,不要引入 `async_trait`,理由见决策 2):
  ```rust
  pub trait AiProvider {
      fn analyze(&self, req: &AnalysisRequest) -> Result<AnalysisResult, AiError>;
      fn id(&self) -> &'static str;
  }
  ```
- **`ContentReader` trait** 只定义不实现(为 agent-reach 阶段留扩展点),签名如 `fn read(&self, url: &str) -> Result<String, AiError>`。本阶段不写实现、不接线。
- **纯函数:从 claude 文本里解析研究卡** `parse_analysis_result(raw: &str) -> Result<AnalysisResult, AiError>`:
  - 从 `raw` 中提取 JSON 对象(claude 可能在 JSON 前后包裹解释文字 / ```json 围栏,要能容错地截取第一个 `{` 到最后一个 `}`)。
  - serde 反序列化到 `AnalysisResult`;反序列化失败 → `ErrorKind::ParseFailed`。
  - **字段校验**:`title` 非空、`score ∈ 1..=5`、`key_points` 非空;不满足 → `ErrorKind::SchemaMismatch`。
  - 失败时 `message` 可含**截断**的原始片段(≤200 字符)辅助排查,**绝不**写入任何 token/key。
- **prompt 构造纯函数** `build_article_prompt(req: &AnalysisRequest) -> String`:产出要求 claude **只返回严格 JSON**(给出字段 schema 和一个示例),包含 URL 和可选 note。语言中文、字段为上面 7 项。

**core 单测(≥3,全部不依赖真实 claude,用 fixture 字符串)**:
1. 合法 JSON(含 ```json 围栏包裹）→ 正确解析出 `AnalysisResult`。
2. 缺字段 / 非法 JSON → `ParseFailed`。
3. `score = 9` 越界 → `SchemaMismatch`。
保留现有所有测试。

### 第 2 步 — src-tauri:ClaudeCliProvider(`std::process::Command` spawn `claude`)

新建 `src-tauri/src/provider.rs`(或 `ai_provider.rs`),实现 `reachnote_core::ai::AiProvider`:

- spawn:`claude -p "<build_article_prompt 的输出>" --output-format json`。用**标准库 `std::process::Command`**,不要引入 tokio/async runtime 到这一层。
- **claude 的 `--output-format json` 返回的是 claude 自身的信封 JSON**(包含 `result` / `is_error` 等字段,不是研究卡本身)。你要:先解析信封取出最终文本(`result` 字段),再把该文本交给 core 的 `parse_analysis_result`。**实现前先实跑一次** `claude -p "返回 {\"ok\":1} 这个 JSON" --output-format json` 确认信封结构,按实际字段取值,不要照搬假设。
- **超时**:spawn 设上限(~120s)。用 `std::process::Child` + 简单看门狗线程,或在 command 层用 `tauri::async_runtime::spawn_blocking` 包裹并配合超时。超时 → `AiError{ kind: ProviderUnavailable, message: "Claude CLI 分析超时" }`(或 `NetworkFailed`,二选一,保持一致)。
- **错误分类**(对齐 PRD Error Paths):
  - `claude` 不存在 / spawn 失败(`io::ErrorKind::NotFound`)→ `ProviderUnavailable`,message 给安装/PATH 提示。
  - 退出码非 0 / 信封 `is_error` → `ProviderUnavailable`(或按内容判 `NetworkFailed`)。
  - 输出非合法 JSON / 字段缺失 / 越界 → 交由 `parse_analysis_result` 返回 `ParseFailed` / `SchemaMismatch`。
- `id()` 返回 `"claude-cli"`。
- **不要**在这里写 SQLite 或状态机逻辑——provider 只「输入请求 → 输出结果/错误」。

### 第 3 步 — src-tauri:store 增加 `update_task` + 状态机推进

- 在 `store.rs` 加 `pub fn update_task(&self, task: &Task) -> Result<(), StoreError>`:按 `id` UPDATE 全字段(status/title/source_domain/score/model/notion_page_id/error_kind/error_message/updated_at/synced_at)。
- 去掉 `get_task` 的 `#[allow(dead_code)]`(本阶段会用)。

### 第 4 步 — src-tauri:推进与重试 command

新增两个 command 并注册进 `invoke_handler`:

- `run_capture_task(store, id: String) -> Result<Task, String>`:
  1. `get_task(id)`,不存在 → `Err`。
  2. 置 `status = Analyzing`、刷新 `updated_at`、清空上一次的 `error_kind/error_message`,`update_task` 落库。
  3. 构造 `AnalysisRequest{ url: task.url, note: ..., content: None }`,调 `ClaudeCliProvider.analyze`(用 `tauri::async_runtime::spawn_blocking` 跑,避免阻塞 UI 线程;command 用 `async fn`)。
  4. **成功**:把 `title/summary…/score/model` 写回 task;`status = Synced`、`synced_at = now`、`notion_page_id = None`(本阶段的「本地完成」语义,见决策 4);`update_task`;返回。
     - 研究卡的 `summary/key_points/tags/next_actions` 在 `Task` 里没有专属列 → **本阶段把完整研究卡 JSON 存进一个新增的可空列**(给 `tasks` 表加一列 `card_json TEXT`,migrate 用 `ALTER TABLE ... ADD COLUMN`,并在 `CREATE TABLE` 里也加上;`Task` 结构加 `pub card_json: Option<String>`)。`title/score` 仍冗余存主列以便队列表直接显示。
  5. **失败**:`status = Failed`、写 `error_kind/error_message`、刷新 `updated_at`;`update_task`;**仍返回 `Ok(task)`**(失败是任务的正常终态,不是 command 调用错误)——或返回 `Err` 也可,但要保证前端能拿到可读错误并刷新队列。二选一并与前端约定一致。
- `retry_capture_task(store, id: String) -> Result<Task, String>`:对 `Failed` 任务,重置为 `Queued` 后复用 `run_capture_task` 的推进逻辑(抽一个内部 async helper,两个 command 都调它)。

> **保持 `create_capture_task` 不变**(仍只创建 `Queued`)。是否在创建后自动 `run` 由前端决定(见第 5 步),后端两件事解耦。

### 第 5 步 — 前端 `src/App.tsx`:触发分析 + 真实研究卡 + 重试

- **采集页 CTA**:`create_capture_task` 成功拿到 `Queued` 任务后,**接着 `invoke("run_capture_task", { id })`**;跳到队列页。期间该任务在队列里走「处理中」(已有状态映射),完成后刷新为「已完成」或「失败」。
  - CTA 在 URL 非法 / 正在提交时 **disabled**(已有规则则复用)。
- **队列**:`run`/`retry` 返回后刷新列表(复用现有 `list_capture_tasks` 加载)。「处理中」行转圈、「已完成」行显示真实 `title`/`score`/`model`、「失败」行显示 `error_message`。
- **失败行重试按钮**(已存在 UI):接 `invoke("retry_capture_task", { id })`,点击后该行回到「处理中」再推进。
- **采集页右侧预览**:任务**完成后**用研究卡真实数据替换骨架——解析 `card_json`(或后端额外返回结构化字段)渲染 标题/摘要/要点/标签/下一步/评分。分析进行中保持骨架 + loading 态;失败显示可读错误。
- **Required States 覆盖**(PRD):loading(分析中)、success(研究卡)、error(可读失败 + 重试)、empty(无任务)、disabled(CTA)。
- 前端 `Task` interface 加 `card_json: string | null` 与 Rust 对齐。**不重设计 UI**,沿用现有 `styles.css` 与组件。

---

## 关键设计决策(照此执行,不要自行改变)

1. **零新增 crate**。用标准库 `std::process::Command` spawn claude,用已有的 `serde_json` 解析。**不引入** `tokio`(直接依赖)、`async-trait`、`reqwest`、`jsonschema`、HTTP 客户端、Notion SDK——那些属于后续阶段。需要异步只用 Tauri 自带的 `tauri::async_runtime`。
2. **provider trait 同步**。本阶段单 provider、单次调用,无并发需求,`async_trait` 是过度设计。子进程阻塞调用放进 `spawn_blocking`,UI 不卡。等真有并发 worker 需求再升级为 async。
3. **内容读取本阶段不接 agent-reach**。它无直接 read 命令,接口需专门研究,会撑爆本阶段。改为把 URL+note 交给 claude,由 claude 决定是否联网读取。**因此本阶段验收只看「claude 返回合法研究卡 JSON 并正确解析、落库、回显」,不验收研究卡内容的事实准确性**——内容质量由下一阶段真实读取解决。`ContentReader` trait 只留扩展点。
4. **成功终态用 `Synced` + `notion_page_id = None`**,这是本阶段对 `Synced` 的**临时语义**(「本地研究卡就绪,尚未同步 Notion」),与前端现有 `synced → 已完成` 映射吻合。**下一阶段接 Notion 时收紧**:`Synced` 将要求 `notion_page_id` 非空,中间经 `Syncing`,本阶段这些任务届时按 `notion_page_id IS NULL` 识别为「待同步」。把这条权衡写进 `memory/integration-progress.md`,避免后人误解 Synced 语义。
5. **claude 调用细节先实测再编码**:`--output-format json` 的信封结构、`-p` 模式下能否联网读 URL、默认权限模式是否放行工具——**都先手动跑一次** `claude -p ... --output-format json` 看真实输出,按实际结果实现,不要照假设写。本阶段不要求 `--dangerously-skip-permissions`(链路在 claude 仅凭 URL 字符串生成的情况下也应能跑通)。

---

## 为什么是这个边界(不要扩大)

- PRD「Small-Step Rule」第 5 步就是 Claude CLI provider,第 6 步才是 Notion adapter。本阶段 = 第 5 步。
- **不接 Notion**:Notion 需真实 token + database + schema 映射 + keychain,几乎无法本地自动验证,且会让本阶段跨 4 个新边界,违反小步原则与 PRD「Validation Bar:不能只靠 mock 或构建通过」。
- **本阶段建立的 `AiProvider` 抽象是后续 Codex CLI / OpenAI-compatible API 的复用地基**(ADR 关键设计),且 `claude` 本机已装 → **100% 可本地端到端验证**。
- **明确不做**(留后续):Notion 连接与写入、agent-reach 真实读取、`Reading` 状态的真实内容获取、Codex/OpenAI provider、模板编辑、全文搜索、打包签名。

---

## 验收命令(全部通过,粘贴真实输出)

```bash
pnpm typecheck
pnpm build
cargo test -p reachnote-core                          # 含本阶段新增 ≥3 个 ai 解析/校验单测
cargo check --manifest-path src-tauri/Cargo.toml
pnpm tauri dev
```

`pnpm tauri dev` **人工冒烟**(本阶段硬性证据):

1. 贴合法 URL(如 `https://openai.com/index/hello-gpt-4o`)+ 可选补充说明 → 点 CTA。
2. 队列里该任务出现并走「处理中」→ 数秒后变「**已完成**」,该行显示 **claude 生成的真实标题、评分、模型**(不是 mock、不是空)。
3. 切回采集页(或完成时)→ 右侧预览显示**真实研究卡**(标题/摘要/要点/标签/下一步/评分),非骨架。
4. **失败用例**:临时把 `claude` 移出 PATH(或重命名),新建任务 → 该行变「**失败**」并显示**可读错误**(provider_unavailable 提示),**重试按钮可点**;恢复 claude 后点重试 → 任务重新跑并完成。
5. 关闭 app 再 `pnpm tauri dev` 重开 → 已完成任务及其研究卡**仍在**(落了 SQLite)。
6. console 无 error/warn;logo 正常(上一阶段已修,勿回退)。

> 如实报告:claude `-p` 是否真的联网读了 URL、研究卡内容是真实抓取还是基于 URL 推断、哪一层没验到。**不要在没有 `tauri dev` 冒烟的情况下声称端到端完成**(PRD 与 AGENTS 都禁止未验证即声称完成)。

---

## 收尾:更新 memory(PRD「Documentation Requirements」强制,同一轮完成)

- `memory/backend-progress.md`:新增 `ai` 模块 + `ClaudeCliProvider` + `update_task` + `run/retry` command;`card_json` 列;改动文件、验证结果、残留风险。
- `memory/frontend-progress.md`:CTA 触发 run、采集页真实研究卡、重试接线、覆盖的 loading/success/error 状态。
- `memory/integration-progress.md`:链路从「本地队列(Queued)」推进到「**Queued → Analyzing → Synced(本地研究卡)**」;**明确记录决策 4 的 `Synced` 临时语义**和「仍未接 Notion / agent-reach」;下一刀 = Notion adapter(第 6 步)。
- 每文件记:状态、实际改动文件、已验证命令、未闭环风险、下一步入口、日期。

---

## 硬性禁止(违反任一即视为偏离任务)

- ❌ 不接 Notion / 不接 agent-reach 真实读取 / 不发 HTTP 请求(本阶段范围外)。
- ❌ 不引入任何新 crate(`tokio` 直接依赖、`async-trait`、`reqwest`、`jsonschema`、Notion SDK 全部禁止);只用标准库 + 已有 `serde_json` + `tauri::async_runtime`。
- ❌ 不修改 `task.rs` 里 `TaskStatus`/`ErrorKind`/`Task` 现有字段定义的语义(可**新增** `card_json` 字段);不改状态机枚举取值。
- ❌ 不动 `create_capture_task` 的「只创建 Queued」行为;不动上一阶段的 logo import / `.gitignore` / `time` 注释(勿回退)。
- ❌ 不自行重设计 UI;沿用 `memory/design-source.md` 与现有 `styles.css`。
- ❌ 不把 token/key/cookie/claude 会话内容写进 repo、日志、memory、测试快照、error_message。
- ❌ 不 commit / push(由用户决定);只改工作树并跑验证。
- ❌ 不在缺少 `tauri dev` 人工冒烟时声称端到端完成;claude 联网与否要如实说明。
