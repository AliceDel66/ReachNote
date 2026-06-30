# Codex 开发 Prompt — ReachNote 下一阶段:本地队列数据流 + 现存问题修正

> 把本文件**整段**作为 prompt 交给 Codex。它是自洽的:包含背景、硬约束、修正项、实现范围、验收命令和禁止事项。
> 生成日期:2026-06-30 · 来源真源:`plans/prds/20260630-1906-reachnote-mvp-reset.prd.md`

---

## 你的角色与目标

你是 ReachNote 仓库的实现 agent。仓库已有一个**已验证通过的 P0 静态桌面壳**(Tauri 2 + React 18 + HeroUI + Rust core),前端队列/采集/模板/设置四个页面渲染正常,但**数据全是前端 mock,没有任何本地持久化和真实数据流**。

本阶段目标 = PRD「Implementation Discipline / Small-Step Rule」的**第 2~4 步**,外加修正三个现存问题:

```
2. 领域类型 + 测试 (crates/core)
3. SQLite 本地队列 (src-tauri)
4. Tauri 命令,把前端队列从 mock 切到真实本地持久化
```

**一句话验收**:用户在采集页填一个合法 URL 点击 CTA → 在本地 SQLite 落一条 `Queued` 任务 → 队列页从数据库读取并显示这条真实任务(不再是 mock 常量),刷新 app 后任务仍在。

> 本阶段**不接** Claude CLI、不接 Notion、不接 Agent-Reach、不接网络。那是后续切片。原因见下方「为什么是这个边界」。

---

## 必读上下文(开工前按序读完)

仓库根目录 = `/Users/yaocheng/Desktop/nexus/rearchnote`。

1. `AGENTS.md` — 仓库协作规则与 Source Of Truth 优先级。**必须遵守**。
2. `plans/prds/20260630-1906-reachnote-mvp-reset.prd.md` — 产品真源。重点读:
   - `## First Vertical Slice`(included/excluded/acceptance)
   - `## P1 Architecture Map`(各 crate 的 owns / must-not-own 边界)
   - `### State Model`(任务状态机与必填字段)
   - `## Implementation Discipline`(小步规则、每条用户路径必须覆盖的状态)
3. `docs/adr/0001-tech-stack.md` — 技术决策(SQLite 用 rusqlite/sqlx、core 不依赖 Tauri、AiProvider trait 形态)。
4. `memory/README.md` 和 `memory/backend-progress.md`、`memory/frontend-progress.md`、`memory/integration-progress.md` — 当前进度事实。
5. `memory/design-source.md` — 新版 UI 设计源(队列列定义、状态样式)。涉及 UI 改动时**不得自行重设计**。

读代码先用仓库内 `.codegraph/`(若 MCP 工具可用,先 `codegraph_explore`),再 `rg` / targeted read。

---

## 现存问题修正(本阶段必须一并修掉)

这三项是上一轮 `/check` 审查发现的真实问题,已用命令验证。**在动新功能之前先修掉前两个**,第三个随手做。

### 修正 1 [HIGH · 一打开就破图] — 品牌 logo 在 dev 和打包后都 404

- **位置**:`src/App.tsx` 第 ~200 行 `AppHeader` 里:
  ```tsx
  <img className="brand-mark"
       src="/assets/reachnote_brand_assets/png/icon/reachnote-symbol-transparent-64.png"
       alt="" />
  ```
- **根因**:Vite 把 `/` 开头的绝对路径解析到 `publicDir`(默认 `./public`),但 `public/` 为空,真实 PNG 在 `assets/reachnote_brand_assets/png/icon/` 下,Vite 不会从那里 serve。`vite.config.ts` 也没设 `publicDir`。已验证 `public/` 下 0 个 PNG。
- **要求修法**:用 Vite 资源 import,让打包器处理 hash 路径,且只暴露用到的文件:
  ```tsx
  import brandMark from "../assets/reachnote_brand_assets/png/icon/reachnote-symbol-transparent-64.png";
  // ...
  <img className="brand-mark" src={brandMark} alt="ReachNote" />
  ```
  顺手把 `alt=""` 改成有意义的 `alt="ReachNote"`(空 alt 让破图连占位都没有)。
- **不要**用 `publicDir: "assets"` 的方案——那会把整个品牌资产目录暴露到 web root。
- **注意**:`src-tauri/tauri.conf.json` 里 bundle icon 的 `../assets/...` 路径是**对的**(Tauri 构建期读取),不要动它,别和前端 `<img>` 混为一谈。
- **验证**:`pnpm tauri dev` 打开后,header 左上角 logo 正常显示,无破图,console 无 404。

### 修正 2 [MEDIUM · 真源会丢] — `.gitignore` 吞掉了 `docs/`,但它是正式真源

- **位置**:`.gitignore` 第 1 行 `docs/`。
- **根因**:`docs/adr/0001-tech-stack.md` 和 `docs/discussions/mvp-prd-information-architecture.md` 被 `AGENTS.md` 的 Source Of Truth 列为第 5、6 条权威来源,但整个 `docs/` 被 git 忽略 → clone 下来就丢,协作者按 AGENTS.md 找会扑空。已用 `git check-ignore` 确认。
- **要求修法**:把 `.gitignore` 第 1 行的 `docs/` **删掉**(ADR 和讨论稿是正式产物,必须进版本库)。删掉后 `git status` 应能看到 `docs/adr/` 和 `docs/discussions/` 变为可跟踪的未跟踪文件——**不需要你 commit**(commit 由用户决定),只要让它们不再被忽略即可。
- **验证**:`git check-ignore docs/adr/0001-tech-stack.md` 不再返回该路径(退出码非 0)。

### 修正 3 [LOW · 随手做] — 多余依赖 + 缺注释的版本锁

- `src-tauri/Cargo.toml` 第 17 行 `serde_json = "1"`:当前 `src-tauri/src/` 里**完全没用到**(已 `grep` 确认)。但本阶段你**会**用到它(Tauri command 返回结构化数据 / 错误序列化),所以**保留它即可,无需删**。
- `src-tauri/Cargo.toml` 第 19 行 `time = "=0.3.41"`:这是**有据的 workaround**(`cookie 0.18.1` 与 `time 0.3.52` API 不兼容,见 `memory/backend-progress.md`),**不要删、不要升级**。但它缺一行说明,半年后没人知道为什么钉死。**要求**:在该行上方加一行注释:
  ```toml
  # Pin: cookie 0.18.1 与 time 0.3.52 API 不兼容,锁定 0.3.41 以让 Tauri 依赖图通过 cargo check。勿升级。
  time = "=0.3.41"
  ```

---

## 本阶段实现范围

严格遵守 PRD 的 P1 边界:**core 不依赖 Tauri,不碰 OS 路径,不碰 UI**;**src-tauri 拥有 SQLite 连接、app data 路径、command**;**前端只渲染和调 command,不碰持久化逻辑**。

### 第 2 步 — `crates/core` 领域类型 + 测试

在 `crates/core/src/` 新增任务领域模型(建议 `crates/core/src/task.rs`,在 `lib.rs` 用 `pub mod task;` 导出)。依据 PRD `### State Model`:

- **状态枚举** `TaskStatus`:`Queued / Reading / Analyzing / Syncing / Synced / Failed`。`derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)`,用 `#[serde(rename_all = "snake_case")]` 让前端拿到稳定字符串。
- **错误分类枚举** `ErrorKind`:至少覆盖 PRD「Error Paths」里的 `invalid_url / read_failed / provider_unavailable / parse_failed / notion_unauthorized / schema_mismatch / network_failed`。本阶段不会真触发它们,但类型要先立住(PRD 10x failure mode:状态和错误分类必须最先建立,否则未来每个 source/provider 都重复造)。
- **任务结构** `Task`,字段严格对齐 PRD「Required task fields」:`id, url, source_type, template_id, status, title, source_domain, score, model, notion_page_id, error_kind, error_message, created_at, updated_at, synced_at`。本阶段用不到的字段用 `Option<...>` 并允许为空。
- **URL 校验纯函数** `validate_article_url(input: &str) -> Result<ValidatedUrl, ErrorKind>`:trim、要求 `http(s)://`、能解析出 host。**这是纯函数,必须有单元测试**(合法 URL、空串、非 URL、非 http scheme 各一例)。把校验放 core 而不是前端,是为了让校验逻辑可单测、且未来 CLI/后台 worker 复用(对应 PRD「testability seam」)。
- **`source_domain` 提取**也做成纯函数并测试(从 URL 取 host,如 `openai.com`)。

**测试要求**:`crates/core` 至少新增 5 个单元测试,覆盖状态序列化字符串、URL 校验四个分支、domain 提取。保留现有 `shell_status` 测试。

### 第 3 步 — `src-tauri` SQLite 本地队列

- 依赖:用 **`rusqlite`**(ADR 给的选项,bundled feature 避免系统 sqlite 依赖):在 `src-tauri/Cargo.toml` 加 `rusqlite = { version = "0.32", features = ["bundled"] }`(版本以当前 crates.io 稳定版为准,你来定)。这是本阶段**唯一允许新增的重依赖**,因为它直接服务于 SQLite 持久化目标——除此之外不要加任何新依赖。
- DB 文件落在 **app data 目录**(用 Tauri 的 `app.path().app_data_dir()`),不要落在仓库内、不要落在 cwd。文件名如 `reachnote.db`。
- 建表 migration(首启动 `CREATE TABLE IF NOT EXISTS tasks (...)`),列对齐 `Task` 字段。时间戳用 ISO-8601 文本或 unix 秒,自己定但要一致。
- 封装一个 store 模块(`src-tauri/src/store.rs`),提供:`insert_task`、`list_tasks`、`get_task`。SQLite 连接用 `Mutex<Connection>` 放进 Tauri `State`(本阶段单连接够用,不要提前上连接池)。
- **错误处理**:store 函数返回 `Result`,错误映射到 core 的 `ErrorKind`,不要 `unwrap()` 在用户路径上。

### 第 4 步 — Tauri 命令,前端切到真实数据

新增命令(`src-tauri/src/lib.rs`),用 `serde` 序列化 core 类型回前端:

- `create_capture_task(url: String, note: Option<String>) -> Result<Task, String>`:
  - 调 core 的 `validate_article_url`,非法直接返回 `Err`(对应 PRD「Invalid URL: frontend validation, no task created」——前端也要挡一道,但 command 是权威校验)。
  - note 长度上限 500(与前端现有 `slice(0,500)` 一致),超限截断或报错,二选一并保持和前端一致。
  - 合法则构造 `Task { status: Queued, source_type: "article", template_id: "article", created_at: now, ... }`,`insert_task`,返回该 `Task`。
  - **本阶段到 `Queued` 为止**。不要在 command 里 spawn 任何读取/AI/Notion,不要推进到 `Reading` 之后。
- `list_capture_tasks() -> Result<Vec<Task>, String>`:从 SQLite 读全部任务,按 `created_at` 倒序。
- 在 `invoke_handler` 注册这两个命令(连同已有的 `shell_status`)。

**前端改造**(`src/App.tsx`):

- 用 `@tauri-apps/api/core` 的 `invoke`(依赖已在 `package.json`,但代码里**从未调用过** —— 已确认 `src/` 无任何 `invoke`)。
- **队列页**:把硬编码的 `QUEUE_ITEMS` 常量换成开机 `invoke("list_capture_tasks")` 拉取的真实数据,用 `useEffect` + `useState` 加载。
  - **必须覆盖这些状态**(PRD「Required States」):`loading`(加载中)、`empty`(无任务时的空态,已有 `.queue-empty` 样式可复用)、`error`(invoke 失败时的可读错误,不要静默)。
- **采集页 CTA**:点击「分析并生成研究卡」时,先做前端 URL 合法性预检(非法则 disable 或提示,对应 PRD CTA disabled 规则),合法则 `invoke("create_capture_task", { url, note })`;成功后跳到队列页并刷新列表,让用户看到新任务出现在队列里。
  - CTA 在 URL 为空/非法、或正在提交时 **disabled**(PRD:CTA disabled when URL invalid / active capture running)。
- 前端类型:为 `Task` / `TaskStatus` 定义 TS interface,与 Rust 序列化字段**逐字对齐**(snake_case)。现有 `QueueItem` 可保留给 mock 或直接替换,但渲染表格的列必须仍是设计源定义的 `标题/来源/状态/时间/评分/模型`。

> UI 视觉**不要重设计**:沿用现有 `styles.css` 类名和布局,只把数据来源从常量换成真实 invoke。状态样式(processing spinner / done 绿 / failed 红)保持现有 `StatusPill`。

---

## 为什么是这个边界(不要扩大范围)

- PRD「Small-Step Rule」明确每步只改一个目标,步骤 2→3→4 是有序的;一次性写到 Notion(步骤 6)会跨 5 个边界,且 **Notion/Claude CLI 需要真实 token 和外部进程,本地无法验证**,违反 PRD「Validation Bar:不能只靠 mock 或构建通过」。
- 本阶段选 2~4 的理由:它**建立了所有未来 source/provider 都依赖的状态机与持久化地基**(PRD「10x Failure Mode:若不先建立 task state 和 error classification,未来每个 source/provider/template 都会重复 failure/retry 逻辑」),且**100% 可本地验证**,不依赖任何外部凭证或网络。
- **明确不做**(留给后续切片):Claude CLI provider、Agent-Reach 读取、Notion 连接与写入、重试的真实执行(本阶段只建 `Failed` 状态和字段,不实现 retry 动作)、模板编辑、全文搜索、全局快捷键、打包签名。

---

## 验收命令(全部必须通过,粘贴真实输出)

```bash
# 前端
pnpm typecheck
pnpm build

# Rust
cargo test -p reachnote-core              # 含新增 ≥5 个 core 单测
cargo check --manifest-path src-tauri/Cargo.toml

# 桌面运行时(最关键的端到端证据)
pnpm tauri dev
```

`pnpm tauri dev` 打开后**人工冒烟**(PRD Acceptance + Required States):

1. 默认进入「队列」,首次无数据时显示**空态**(不是报错、不是 mock 数据)。
2. 切到「采集」,URL 框为空时 CTA **disabled**;填非法字符串(如 `abc`)CTA 仍 disabled 或给出校验提示。
3. 填合法 URL(如 `https://openai.com/index/gpt-4o`),点 CTA → 自动回到「队列」,**看到这条真实任务**出现,状态为「处理中/Queued 对应的 pill」。
4. **关闭 app 再 `pnpm tauri dev` 重开** → 队列里那条任务**仍在**(证明落了 SQLite,不是 React state)。
5. logo 正常显示(修正 1),console 无 error/warn/404。

> 验证不可只凭"编译通过"或"读代码推断"。`tauri dev` 的人工冒烟是本阶段的硬性证据。若某项无法验证,**如实说明哪一层没验到**,不要声称完成。

---

## 收尾:更新 memory(PRD「Documentation Requirements」强制)

完成后**在同一轮**更新这些文件(只记事实,不记猜测、不记 token/key):

- `memory/backend-progress.md`:新增 core 类型 + SQLite store + 两个 command;列出改动文件、验证命令结果、残留风险。状态从 `Done / Minimal` 推进描述。
- `memory/frontend-progress.md`:队列页改为真实 invoke、采集页 CTA 接 create_capture_task;记录覆盖的 loading/empty/error 状态。
- `memory/integration-progress.md`:端到端链路从 `静态壳` 推进到 `本地队列闭环(Queued 落库 + 队列读取持久化)`;明确**仍未接** Claude/Notion/Agent-Reach。
- 每个文件记录:状态(Done/In Progress/Blocked)、实际改动文件、已验证命令、未闭环风险、下一步入口、日期 `2026-06-30`(或你执行当天)。

---

## 硬性禁止(违反任一即视为偏离任务)

- ❌ 不接 Notion / Claude CLI / Codex / Agent-Reach / 任何网络请求(本阶段范围外)。
- ❌ 不删 `time = "=0.3.41"`、不升级它(是 workaround,只加注释)。
- ❌ 不引入 `rusqlite` 以外的新依赖;不做无关的框架替换或大范围重构(PRD「No unrelated framework swaps, broad refactors, or hidden dependency upgrades」)。
- ❌ 不自行重设计 UI;沿用 `memory/design-source.md` 的设计源和现有 `styles.css`。
- ❌ 不把 secrets / token / key / cookie 写进 repo、日志、memory、测试。
- ❌ 不 commit / push(由用户决定);你只改工作树并跑验证。
- ❌ 不在没有 `tauri dev` 人工冒烟的情况下声称端到端完成。
- ❌ 不把 DB 文件落在仓库内或 cwd;必须用 app data 目录,且确认它不会被 git 跟踪。
