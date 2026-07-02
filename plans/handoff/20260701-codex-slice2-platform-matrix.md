# Codex 开发 Prompt — ReachNote Slice 2:Agent-Reach 平台能力矩阵

> 整段作为 prompt 交给 Codex。自洽:含前置状态(已核实)、边界、真实 doctor fixture 规格、映射规则、验收、禁止项。
> 生成日期:2026-07-01 · 真源:`plans/prds/20260701-1447-reachnote-next-phase-platform-template-destinations-onboarding-shortcuts.prd.md`(含审查决议段)
> 对应 PRD **Slice 2: Agent-Reach Platform Matrix**,以及审查批注 **P1-3**(doctor 归一化规格)、**P2-2**(doctor 缓存策略)。

---

## 你的角色与目标

你是 ReachNote 仓库的实现 agent。Slice 1(settings + 首启动引导 + App.tsx 拆分)**已完成并提交**:`app_settings` 表、`get_app_settings`/`save_app_settings`/`get_environment_status` 命令、`AiProviderStatus`/`AgentReachStatus` 已在;前端已拆成 `src/{onboarding,settings,capture,queue,templates,components}/`。**但平台能力矩阵尚未接入**——`get_environment_status` 目前只检测 `agent-reach` 是否存在 + 版本,不跑 doctor,`EnvironmentStatus` 里没有平台字段,onboarding/settings 也不展示平台状态。

本阶段目标:把 **Agent-Reach 15 平台能力矩阵**接进来——跑 `agent-reach doctor --json`,归一化成结构化平台状态,持久化最近快照,并在 Settings 渲染成能力矩阵。

**一句话验收**:Settings 里出现「Agent-Reach 平台能力」区块,点「刷新」跑一次 doctor,15 个平台各显示一行(名称 / 可用性 / active backend / 建议动作);`agent-reach` 缺失或 doctor JSON 异常时显示可读错误、不崩;`ok`/`warn`/`off` 三类平台都能无溢出渲染。

> **本阶段不改采集流程**(除了在 Capture 展示「检测到的来源 + 是否可读」的只读提示)。真实按平台路由读取是 Slice 7。**不接任何登录态平台的真实抓取。**

---

## 前置状态(已核实,在其上扩展)

用 `.codegraph/` 或 targeted read 确认:

- `src-tauri/src/lib.rs`:
  - `struct EnvironmentStatus { ai_providers, agent_reach, recommended_provider_id, ... }`(**目前无 `source_platforms` 字段** —— 本阶段加)。
  - `fn get_environment_status(store) -> EnvironmentStatus`(已注册)。
  - `fn detect_agent_reach_status() -> AgentReachStatus`:已 `resolve_command("agent-reach")` + 取版本。**复用它的 command 解析,不要重写。**
  - 已注册命令:`shell_status/create_capture_task/list_capture_tasks/recover_interrupted_tasks/run_capture_task/retry_capture_task/sync_pending_analyzed_tasks/sync_capture_task/get_app_settings/save_app_settings/get_environment_status/get_notion_settings/save_notion_settings/test_notion_connection/set_compact_mode`。
- `src-tauri/src/store.rs`:`app_settings` 表 + `get/save_app_settings`;`AppSettings` 有 `last_environment_check_json`(可存快照)。migrate 用 `CREATE TABLE IF NOT EXISTS`。
- `crates/core/src/`:纯逻辑放这里,不依赖 Tauri/reqwest/进程。`task.rs`/`analysis.rs`/`notion.rs` 已在。
- 前端 `src/types.ts`:有 `EnvironmentStatus`/`AiProviderStatus` TS 类型;`src/settings/SettingsView.tsx`、`src/onboarding/OnboardingView.tsx` 已渲染 AI provider,平台区块是本阶段新增。

**已为你备好真实 fixture(直接用,不要臆测 JSON 形状)**:
`crates/core/src/testdata/agent_reach_doctor.sample.json` —— 本机 `agent-reach doctor --json` v1.5.0 的真实脱敏输出,15 平台。

---

## doctor JSON 真实结构(实测,权威)

顶层是 **平台 key → 条目** 的扁平 map(**无** `platforms`/`checks` 包装层)。每条目:

```json
{
  "github": {
    "status": "ok",                 // "ok" | "warn" | "off"
    "name": "GitHub 仓库和代码",
    "message": "完整可用（读取、搜索、Fork、Issue、PR 等）",  // 可能多行,含安装指引
    "tier": 0,                       // int
    "backends": ["gh CLI"],          // string[]
    "active_backend": "gh CLI"       // string 或 null
  }
}
```

fixture 里 15 平台的真实 status 分布:
- **ok(6)**:`github, bilibili, v2ex, rss, exa_search, web`(`active_backend` 非空)
- **warn(2)**:`twitter, xueqiu`(`active_backend=null`,message 含安装/登录指引)
- **off(7)**:`youtube, reddit, facebook, instagram, xiaohongshu, linkedin, xiaoyuzhou`(`active_backend=null`)

平台 key 全集(15):`github, twitter, youtube, reddit, facebook, instagram, bilibili, xiaohongshu, linkedin, xiaoyuzhou, v2ex, xueqiu, rss, exa_search, web`。

---

## 归一化映射规格(P1-3,照此实现)

在 **`crates/core/src/platform.rs`**(新建,`lib.rs` 加 `pub mod platform;`)实现纯函数归一化。

### 类型

```rust
#[serde(rename_all = "snake_case")]
pub enum PlatformAvailability { Ready, NeedsInstall, NeedsLogin, NeedsConfig, Blocked, Unknown }

#[serde(rename_all = "snake_case")]
pub enum PlatformAction { CaptureUrl, ReadContent, Search, Transcribe, MetadataOnly, NotSupportedYet }

pub struct SourcePlatformStatus {
    pub key: String,                 // agent-reach 平台 key
    pub name: String,                // doctor 的 name
    pub availability: PlatformAvailability,
    pub active_backend: Option<String>,
    pub action: PlatformAction,      // ReachNote 现在能做什么
    pub message: String,             // doctor message,截断见下
    pub raw_status: String,          // 原始 "ok"/"warn"/"off",便于排错
}
```

### status → availability 映射(实测确定,不要猜)

- `"ok"` → **`Ready`**
- `"off"` → **`NeedsInstall`**(fixture 里 off 全是「后端未安装」;message 通常含 `pip install`/`install --channels`)
- `"warn"`:
  - message 含「登录 / 登录态 / login / cookie」→ **`NeedsLogin`**
  - 否则(未安装 CLI 等)→ **`NeedsInstall`**
- 未知 status 字符串 / 条目缺 `status` → **`Unknown`**
- (`Blocked`/`NeedsConfig` 本阶段保留枚举值但 fixture 不产生;归一化里可先不主动产出,除非 message 明确「网络封禁 / 需配置」)

> 判定用**小写包含匹配**,把中英文关键词都覆盖(`登录`、`登录态`、`cookie`、`login`)。

### availability → action 映射(ReachNote 当前真实能力,别乐观)

- `github` ready → `ReadContent`(已有 GitHub API/README fallback)
- `web` ready → `ReadContent`(已有 Jina Reader)
- `rss` ready → `ReadContent`
- `exa_search` ready → `Search`
- `bilibili` ready(仅搜索后端)→ `Search`
- 其余 ready → `MetadataOnly`(保守:矩阵可见但 Slice 7 才做真实读取)
- 任何非 ready → `NotSupportedYet`

> 这套 action 表体现「诚实可见」:平台可见 ≠ 现在就能读。宁可标 `NotSupportedYet`/`MetadataOnly`,不要假装 `capture_url`。

### message 截断

`message` 可能多行长指令。归一化**保留完整 message**(前端负责显示/截断),但**额外**给一个 `summary`(首行或前 ~120 字符)供矩阵行紧凑显示 —— 或前端自己截断,二选一并说明。

### 纯函数签名 + 单测(P1-3 硬要求)

```rust
pub fn normalize_doctor_output(doctor_json: &str) -> Result<Vec<SourcePlatformStatus>, PlatformParseError>;
```

- 解析失败(非法 JSON)→ `PlatformParseError`(可映射到 `ErrorKind::ParseFailed`)。
- **单测直接读 fixture**:`include_str!("testdata/agent_reach_doctor.sample.json")`,断言:
  1. 返回 15 条,key 集合完全匹配。
  2. `github`/`web` → `Ready` + `ReadContent` + active_backend 非空。
  3. `twitter` → `warn` 源、`NeedsLogin`(twitter message 提「未安装」还是「登录」?按 fixture 实际:twitter 是「CLI 未安装」→ 实际会落 `NeedsInstall`;**以 fixture 真实 message 为准写断言**,别硬套)。
  4. `youtube` → `off` → `NeedsInstall` + `NotSupportedYet`。
  5. 至少 ready/warn/off 各一条被覆盖(PRD L970)。
- 加一个**畸形 JSON** 用例 → 返回 `PlatformParseError`。

> ⚠️ 上面第 3 点提醒:`twitter` 的 message 实际是「Twitter CLI 未安装」,按规格该落 `NeedsInstall` 而非 `NeedsLogin`。**写断言前先读 fixture 里每条 message,让断言匹配真实数据**,不要照抄我举的例子。`reddit`(off,message 明说「必须用登录态」)是 `NeedsLogin` 语义的更好例子 —— 但它 status=off,按 status 优先规则先落 `NeedsInstall`;**若你认为 off+登录态该是 `NeedsLogin`,在 PR 说明你的映射选择并保持一致**。

---

## src-tauri:doctor 执行 + 快照持久化

### 命令 `run_agent_reach_doctor`

新增并注册:

```rust
#[tauri::command]
async fn run_agent_reach_doctor(store) -> Result<Vec<SourcePlatformStatus>, String>
```

- 复用 `resolve_command("agent-reach")`;缺失 → `Err`,message 含安装引导(不自动安装,PRD Open Q3)。
- `spawn_blocking` 里跑 `agent-reach doctor --json`,**带超时**(复用/新增 `REACHNOTE_DOCTOR_TIMEOUT_SECS`,默认 60)。子进程输出捕获用现成模式(参考 `provider.rs` 的 `run_process`,避免管道死锁 —— 该问题历史上修过)。
- 拿到 stdout → `core::platform::normalize_doctor_output`。JSON 异常 → 可读 `Err`(不崩)。
- **成功后写快照**(见下),返回归一化结果。
- **测试注入**:支持用环境变量覆盖 doctor 命令(如 `REACHNOTE_AGENT_REACH_CMD`),让冒烟能用一个打印 fixture 的 fake 脚本,不依赖真实 agent-reach。参考 provider 的 `REACHNOTE_CLAUDE_CMD` 注入模式。

### 快照持久化(P2-2)

- 新表 `source_capability_snapshots`(PRD 已给 schema):`id / agent_reach_version / doctor_json / normalized_json / created_at`。migrate 加建表。
- store 加 `save_capability_snapshot(&self, ...)` 和 `get_latest_capability_snapshot(&self) -> Option<...>`。
- **`get_environment_status` 改为读最近快照**(不在这里同步跑 doctor —— doctor 慢,不能每次启动/进设置就跑 15 平台网络探测)。给 `EnvironmentStatus` 加 `source_platforms: Vec<SourcePlatformStatus>`(来自最近快照,无快照则空数组 + 一个「尚未检测」标记)。
- doctor 只在:① 用户点 Settings/onboarding 的「刷新平台」;② 首启动 onboarding 主动跑一次。**不随每次 `get_environment_status` 跑。**

---

## 前端:平台矩阵 UI

### Settings(`src/settings/SettingsView.tsx`)

新增「Agent-Reach 平台能力」区块:

- 「刷新检测」按钮 → `invoke("run_agent_reach_doctor")`,期间 loading。
- 15 平台各一行:`name` · 可用性 pill(ready 绿 / needs_* 黄 / blocked 红 / unknown 灰)· `active_backend`(无则「—」)· action 标签(如「可读取」「仅搜索」「暂不支持」)。
- 长 message 截断显示,可展开或 title 悬浮看全文(截断长 shell 指令,PRD L337)。
- **状态覆盖**:loading(检测中)、success(矩阵)、error(agent-reach 缺失 / JSON 异常的可读提示)、empty(从未检测过)。
- `off`/不可用平台**不消失**,而是解释缺什么(PRD Platform UX Rules)。

### Capture(只读提示,不改流程)

- 从 URL 检测来源类型后,显示「检测到:GitHub / 网页 / …」+ 该来源当前是否可读(来自最近快照)。
- 「平台不可用/需配置」时在 CTA 上方给**警告**(本阶段仅提示,不强制 disable —— 真实路由是 Slice 7)。

### 前端类型

- `src/types.ts` 加 `PlatformAvailability`/`PlatformAction`/`SourcePlatformStatus`,与 Rust snake_case 逐字对齐;`EnvironmentStatus` 加 `source_platforms`。
- **不重设计 UI**,沿用现有 `styles.css` 与组件风格。

---

## 为什么是这个边界

- 修订版 PRD 依赖图:`1 → 2 → 3 → …`,Slice 2 是矩阵,给后续「按来源路由 / 模板按来源选」打底。
- **本阶段只读展示 + 快照**,不做真实按平台路由读取(Slice 7)、不做任何登录态平台抓取。矩阵的价值是「诚实可见 + 分级路径」(PRD G1),不是一次性全平台等深支持。
- **明确不做**:改采集读取路由、模板系统(Slice 3)、目的地(Slice 4/5)、快捷键(Slice 6)、从 UI 触发 agent-reach install/configure(Open Q3)。

---

## 验收命令(全部通过,粘贴真实输出)

```bash
pnpm typecheck
pnpm build
cargo test -p reachnote-core                          # 含 normalize_doctor_output 单测(读 fixture,≥ ready/warn/off 三类 + 畸形 JSON)
cargo test --manifest-path src-tauri/Cargo.toml       # 含快照存取 / doctor 命令(用 fake 注入)测试
cargo check --manifest-path src-tauri/Cargo.toml
git diff --check
pnpm tauri dev
```

`pnpm tauri dev` **人工冒烟**(本阶段硬性证据):

1. 进 Settings →「Agent-Reach 平台能力」→ 点「刷新检测」→ 15 平台渲染,`github`/`web` 显示 ready + backend,`youtube` 等显示 needs_install/暂不支持,无横向溢出、无文字重叠(`1180x780` 和 `900x680` 都测)。
2. **agent-reach 缺失用例**:`PATH` 去掉 agent-reach(或用 `REACHNOTE_AGENT_REACH_CMD=__missing__`)→ 点刷新 → 可读错误 + 安装引导,不崩、不白屏。
3. **JSON 异常用例**:用 fake 命令返回 `not json` → 可读 parse 错误。
4. 关闭 app 重开 → Settings 直接显示**上次快照**(未自动重跑 doctor),说明快照持久化生效。
5. Capture 页贴一个 GitHub URL → 显示「检测到:GitHub · 可读取」;贴一个 youtube URL → 显示「需安装/暂不支持」提示。
6. console 无 error/warn;无 secret 泄漏(doctor message 里的安装指引不是 secret,但确认没有意外打印 token/env)。

> 如实报告:doctor 是真跑了还是 fake 注入、哪些平台在**你本机**的真实状态、快照 TTL 行为。**不要在没有 `tauri dev` 冒烟的情况下声称完成**。

---

## 收尾:更新 memory(同一轮)

- `memory/backend-progress.md`:新增 `core::platform` 归一化 + `run_agent_reach_doctor` + `source_capability_snapshots` 表 + `get_environment_status` 读快照;改动文件、验证结果(含 fixture 单测)、残留风险。
- `memory/frontend-progress.md`:Settings 平台矩阵、Capture 来源提示;覆盖的 loading/empty/error 状态。
- `memory/integration-progress.md`:能力矩阵接入(读展示 + 快照),明确**未做**真实按平台路由(Slice 7)、未接登录态平台;下一刀 Slice 3(模板注册)。
- `memory/desktop-qa.md`:平台矩阵在 `1180x780`/`900x680` 的渲染与三态(ready/warn/off)冒烟结果;Computer Use / AX fallback 状态。
- 记录:状态、改动文件、已验证命令、未闭环风险、日期。

---

## 硬性禁止(违反任一即偏离任务)

- ❌ 不臆造 doctor JSON 结构 —— 用 `crates/core/src/testdata/agent_reach_doctor.sample.json` 作权威 fixture;归一化单测必须读它。
- ❌ 不在 `get_environment_status` / 每次启动同步跑 doctor(慢);只读快照,doctor 仅手动刷新 / 首启动。
- ❌ 不做真实按平台路由读取(Slice 7)、不接任何登录态平台抓取、不从 UI 触发 agent-reach install/configure。
- ❌ 不把平台标成比真实能力更乐观的 action(可见 ≠ 能读;保守用 `MetadataOnly`/`NotSupportedYet`)。
- ❌ 不改 `task.rs` 现有 `TaskStatus`/`Task` 字段定义;不改采集读取/分析/同步现有流程(除 Capture 只读提示)。
- ❌ 不引入不必要的新 crate(JSON 用已在的 `serde_json`;子进程用标准库,参考 `provider.rs`)。
- ❌ 不把 token/key/cookie/env 值写进 repo、日志、memory、快照、测试。
- ❌ 不 commit / push(由用户决定);只改工作树并跑验证。
- ❌ 不在缺少 `tauri dev` 冒烟证据时声称完成。
