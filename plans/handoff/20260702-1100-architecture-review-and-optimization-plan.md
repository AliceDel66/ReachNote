<!-- /autoplan restore point: /Users/yaocheng/.gstack/projects/AliceDel66-ReachNote/main-autoplan-restore-20260702-112602.md -->
# ReachNote 架构审查与优化计划

> **日期**: 2026-07-02
> **审查人**: Claude（架构师视角，只读审查 + 全量测试基线核实）
> **用途**: 前半部分是给 Yaocheng 的优化报告；后半部分是可直接切片交给 Codex 的开发计划。
> **真源**: 当前工作树（Slice 1-3 已完成未提交）、`plans/prds/20260701-1447-*.prd.md`（含审查决议）、`memory/*.md`
> **验证基线**: `pnpm typecheck` PASS；`cargo test -p reachnote-core` 26 passed；`cargo test --manifest-path src-tauri/Cargo.toml` 41 passed / 1 ignored。审查结论建立在全绿基线上。

---

## Part 1 · 架构审查报告

### 1.1 系统现状图（P1）

**边界**: 本地优先桌面应用，无服务端。三层：

```text
src/           React 18 + Vite + Tailwind    前端壳（App.tsx 544 行中枢 + 6 个视图目录）
src-tauri/     Tauri 2 shell (Rust)          lib.rs 1456 行（17 个 command + 全部编排）
                                             store.rs 1189 行（SQLite: tasks/notion_settings/
                                             app_settings/source_capability_snapshots）
                                             provider.rs / reader.rs / notion.rs（外部 IO 适配）
crates/core/   纯逻辑 crate（无 IO 依赖）      task/analysis/notion/platform/template
```

**运行时主链路**（P2 已逐行核实）:

```text
CaptureView.onSubmit
  → invoke create_capture_task(url, note, providerId, templateId)     [lib.rs:69]
  → SQLite insert (status=queued)
  → 前端 handleRunTask → invoke run_capture_task                       [App.tsx:383 → lib.rs:299]
  → spawn_blocking: run_and_sync_capture_task_blocking                 [lib.rs:410]
      Reading  → AgentReachWebReader (Jina / GitHub API fallback)      [lib.rs:344]
      Analyzing→ ProviderRunner (claude/codex/openai, stdin+超时+三线程管道) [lib.rs:380]
      Analyzed → sync_capture_task_blocking → NotionClient.create_page [lib.rs:428]
      Synced / Failed
  → 前端 1.2s 轮询: recover_interrupted_tasks + sync_pending_analyzed_tasks
    + list_capture_tasks 三连击                                        [App.tsx:224-266]
```

**环境/能力面**: `get_environment_status` 读最近快照（不跑 doctor）；`run_agent_reach_doctor` 仅手动/首启动触发，带超时与 fake 注入（`REACHNOTE_AGENT_REACH_CMD`）。15 个 `REACHNOTE_*` 环境变量构成完整测试注入面。

### 1.2 总评

**架构方向是健康的，不需要推倒。** 值得点名保留的强项：

- core（纯逻辑）/ shell（IO）/ 前端三层边界干净，core 无 Tauri/reqwest 依赖，单测便宜。
- 所有长操作已走 `async command + spawn_blocking`，UI 永不冻结（这是修过的坑，有注释守护）。
- `run_process` 三线程管道模式解决了 64KB pipe 死锁；codex `--output-last-message` 解决了 stdout 噪音——这两处注释是真实事故的疫苗，勿动。
- 能力矩阵"诚实可见"（doctor fixture 驱动、保守 action 映射）、加法式迁移、错误分类落库可重试——这些设计决策都正确。
- 测试注入面完备（fake CLI / mock HTTP / fixture），67 个测试不触外网。

**真正的债务不在分层，在执行模型**：**任务 runner 与任务创建者耦合**——只有前端提交流（`handleCaptureSubmit → handleRunTask`）会驱动任务，后端没有独立执行者。精确表述：隐藏窗口本身不杀 JS 定时器（`set_compact_mode` 只是 `window.hide()`），但 ① 快捷键（Rust 侧）创建的任务没有任何 runner 会领取；② app 退出后 `Queued` 任务重启永久孤儿（F1a）。因此 PRD Slice 6 的验收（后台采集持续处理）在当前模型下不成立。这是可靠性债务而非市场护城河——修它的理由是两个在产 bug + Slice 6 的硬前置，不多也不少。

### 1.3 发现清单（按严重度）

#### F1 · P0 — 前端驱动执行模型：孤儿任务卡死 + 重复同步竞态

三个相互关联的实证问题，根因同一个——**没有后端任务 worker**：

**F1a · Queued 孤儿任务永久卡死（确定性 bug）。** 任务只在前端 `handleRunTask` 调用时才运行。如果创建任务后 app 在 `run_capture_task` invoke 前退出：
- `recover_stale_processing_tasks` 只恢复 `Reading|Analyzing|Syncing`（store.rs:746-751），不含 `Queued`；
- 启动补偿 `sync_pending_analyzed_tasks` 只处理 `Analyzed`；
- 队列 UI 重试按钮只在 `failed` 时出现（QueueView.tsx:107），而 `queued` 被归入 processing 显示转圈（utils.ts:39-41）。

结果：该任务显示"排队中"转圈**到永远**，无任何 UI 或后台路径能推进它。

**F1b · 重复同步竞态（小概率，产生重复 Notion page）。** `task_can_sync` 允许 `Syncing` 状态重入（lib.rs:694-698，为中断恢复而设），且状态推进是"读-改-写"而非 CAS。当 `run_and_sync` 刚写入 `Analyzed`、尚未进入 sync 时，1.2s 轮询触发的 `sync_pending_analyzed_tasks` 可同时选中同一任务——两条路径都通过 `task_can_sync` 检查，各自 `create_page`，产生两个 Notion page。Mutex 只序列化 DB 操作，不覆盖 HTTP 段。窗口是毫秒级，但每次采集都会开一次窗。

**F1c · 轮询三连击浪费。** 有处理中任务时前端每 1.2s 依次 invoke `recover_interrupted_tasks` + `sync_pending_analyzed_tasks` + `list_capture_tasks`（App.tsx:224-266）。recover 和 sync_pending 是启动补偿语义，放进高频轮询既无必要又放大 F1b 竞态；后端明知每次状态变更却无法推送（Tauri 事件系统完全未用，grep 零命中）。

**10x 视角**: 任务量放大十倍时，最先坏的是"UI 必须开着任务才会跑"这个隐含假设——快捷键后台采集（PRD Slice 6 验收明确要求"Hidden-to-background app continues to process task"）直接不成立。

#### F2 · P1 — 模板与路由逻辑双源漂移

- `crates/core/src/template.rs` 的 `BUILT_IN_TEMPLATES` 与 `src/constants.ts` 的 `TEMPLATES` 是两份手工维护的模板表（id/名称/描述/promptProfile 全部重复）。
- 后端 `list_templates` command **已存在但前端零调用**（grep 确认）——真源命令做好了没人用。
- URL→模板路由同样两份：Rust `suggest_template_id_for_url` vs TS `templateForSourcePlatformKey` + `sourcePlatformKeyForUrl`。
- 平台可用性/action 的中文文案也在 CaptureView 和 SettingsView 各自维护。

Slice 4/5 要加 destination 映射时，这个漂移面会再翻一倍。改一处忘一处的第一现场会是"模板页文案与分析 prompt 对不上"。

#### F3 · P1 — 工程流缺口：无 CI + review gate 反复超时

- `.github/workflows/` 只有 `release.yml`。PRD 规定的 6 条验收命令（typecheck/build/两套 cargo test/check/diff --check）每个切片都靠 Codex 自跑自报，**没有独立机器验证**。"声称通过"和"真的通过"之间没有防线——这对"交给 Codex 执行"的工作流是最划算的一道保险。
- `memory/review-gate.md`: Claude CLI 只读 review 以 180s 超时反复 Blocked（Slice 1、Slice 3 两次）。gate 名存实亡时，实际防线只剩测试。既然本机 `codex exec` 链路已修通（provider.rs 注释可证），跨模型 review 可以作为 gate 的可靠 fallback。

#### F4 · P2 — 若干小项（不阻塞，择机顺手修）

| # | 问题 | 位置 | 建议时机 |
| --- | --- | --- | --- |
| F4a | `command_error` 把 `ErrorKind` 压成 `"message (kind)"` 字符串，前端无法程序化区分错误类别 | lib.rs:678-680 | Slice 4 destination 错误分类时一并结构化为 `{kind, message}` |
| F4b | 迁移判断靠 schema 字符串包含（`!schema.contains("note TEXT")`） | store.rs:622 | Slice 4 加表时改用 `PRAGMA user_version` |
| F4c | `get_environment_status` 每次调用都写 `last_environment_check_json` 快照 | lib.rs:203-211 | 任意后续切片顺手修（一行判断），不绑定 Slice A |
| F4d | lib.rs 1456 行混装 command 注册 + 状态机 + doctor 进程 + 环境检测 | src-tauri/src/lib.rs | Slice A 抽 pipeline 时自然拆出 `worker.rs`，不做独立重构 |
| F4e | Notion token 明文 SQLite | 已知债务 | PRD Slice 4.5 已锁定，不提前 |

### 1.4 明确不做（反过度工程）

- **不引入状态管理库 / 不拆 App.tsx hooks**——544 行中枢在事件化（Slice A）后自然瘦身（删掉轮询编排），等 destinations UI 真正落地再评估。
- **不动 `Mutex<Connection>` 单连接**——当前规模下串行化正确且简单，长操作已在锁间隙释放。
- **不做任务并发执行**——worker 第一版并发度=1（顺序循环），这就是"最小一致改动"；并发是将来有实证需求再开的旋钮。
- **不重构 provider/reader/notion 适配层**——它们边界正确、测试充分。
- **不提前做 SecretStore / OAuth / 后台调度 cron**——按 PRD 既定切片走。
- **不做 single-instance 防护**（Eng F-A5 显式化）——双开 app = 双 worker 共享 SQLite；CAS claim 保证不会双跑同一任务，代价只是重复扫描。tauri-plugin-single-instance 留到有实证需求再加。
- **不做彻底 Notion 去重**（按 URL 查询既有 page）——外部查询成本与新失败面不划算；crash window 残留由 finalization 状态机缩到最小并显式声明。

---

## Part 2 · 后续开发计划（交 Codex 执行）

### 2.1 切片顺序与依赖

```text
C0 (CI 防线 + landing protocol + Slice 1-3 落盘, ~1 天)
 └→ A (后端任务 worker + 事件推送) ── 修复 F1a/F1b/F1c，Slice 6 的硬前置
     └→ B (模板/路由单一真源, 半天) ── 修复 F2
         └→ PRD Slice 6 (全局快捷键, Notion-only) ── 前移：PRD First Proof Point
             └→ PRD Slice 4 (destination abstraction, 接入 worker 的 sync 缝)
                 └→ PRD Slice 4.5 (SecretStore)
                     └→ PRD Slice 5 (webhooks)
                         └→ PRD Slice 7 (平台扩展)
```

**为什么 A 在 Slice 4 之前**: Slice 4 的审查批注 P1-2 本来就要求重写 `sync_pending_analyzed_tasks`（按 destination_id 路由、空目的地跳过）。若先做 4 再做 A，同一段代码要动两次。先做 A 把执行模型定住——worker 暴露一个 `sync(task)` 缝，Slice 4 只需往缝里换 adapter 路由。同时 A 立即修复两个在产 bug，不用等。

**为什么 Slice 6 前移到 4/4.5/5 之前（审查修正 R2，跨模型共识）**: PRD 审查批注 P2-4 的依赖图明确判定 **`6` 独立**（只依赖 Slice 1 的 app_settings 默认值，源码核实成立：默认 provider/template 已持久化，Notion 链路已通）。本计划初版把它线性排到 webhook 之后没有任何依赖论证。而快捷键采集是 PRD First Proof Point 的核心、是这个产品唯一的差异化时刻（复制 URL → 按快捷键 → 研究卡进 Notion）；飞书/企微/钉钉 webhook 消息推送是次级需求。Slice 6 的唯一硬前置是 A（后台执行者），A 完成后立即做 6，第一版 destination 就是现有 Notion 路径，不等 destination abstraction。

**F1a 不出 hotfix 的显式决策（审查修正 R6）**: 存在 ~10 行 hotfix（启动时顺带领取 Queued）可立即止血，但 A 就是第二刀、天级可达，hotfix 与 worker 改同一段领取逻辑，做两次不划算。**显式接受 F1a 存续到 A 落地**；若 A 因故延期超过一周，此决策失效、先出 hotfix。

### 2.2 Slice C0：CI 验收防线 + landing protocol

**目标**: 每次 push/PR 自动跑 PRD 全套验收命令，让任何实现者（Codex/人）的"声称通过"可被独立验证。**审查修正（R1）**：CI 只在 push 时生效，而旧切片协议禁止 Codex commit——防线接不上执行流。C0 同时落 landing protocol，否则 CI 是装饰品。

**前置（硬条件，执行分工写死）**: 当前工作树的 Slice 1-3 改动（30+ 文件）必须先 commit 落盘——**此步由 Yaocheng 执行**（在别人 dirty tree 上做 hunk 级分块风险过高，不交给 Codex）。理想是 3 个逻辑 commit（Slice 1 settings/onboarding、Slice 2 平台矩阵、Slice 3 模板注册），但 lib.rs/store.rs/App.tsx 同文件混合三片改动，hunk 级分离需要交互式 `git add -p`；**无法干净分离时允许单 commit 落盘**（消灭 dirty tree 优先于 commit 美学）。dirty tree 上无法干净验收 C0，且巨型未提交工作树本身就是当前最大的交付风险。

**Scope**:
- 新增 `.github/workflows/ci.yml`：`macos-latest`（理由是**与 release 目标环境对齐**——Tauri 桌面产物最终跑在 macOS/Windows；不是因为 Linux 依赖难装），触发 `push` 到 main + 所有 PR。
- 步骤：checkout（**`fetch-depth: 0`**——diff 范围检查需要 base 可达，默认 depth 1 会 `bad revision`）→ **`pnpm/action-setup@v4`（读 `packageManager` 字段）+ `actions/setup-node`（`node-version: 24`，与 release.yml 一致，`cache: pnpm`）**——不用 corepack（9947a2e 在 release 工作流踩过 corepack/Node 版本坑，release.yml 现行配方就是 action-setup + Node 24，照抄）→ Rust stable + cargo cache → `pnpm install --frozen-lockfile` → `pnpm typecheck` → `pnpm build` → `cargo test -p reachnote-core` → `cargo test --manifest-path src-tauri/Cargo.toml` → `cargo check --manifest-path src-tauri/Cargo.toml` → 空白检查。
- **硬顺序约束（不可拆 job）**：`tauri::generate_context!` 编译期嵌入 `frontendDist: ../dist`——**`pnpm build` 产出 dist/ 是所有 src-tauri cargo 步骤的硬前置**。将来任何人把 node/rust 拆成并行 job 都会神秘编译失败，此处写明防患。
- **空白检查语义修正（跨模型共识）**：`git diff --check` 在 CI 干净 checkout 上是 no-op（工作树永远无 diff）。PR 用 `git diff --check origin/$BASE...HEAD`；push 用 `git diff-tree --check --root -r HEAD`。本地验收命令照搬 CI 而不适配，与"防线要真"的立意矛盾。
- **权限收敛**：ci.yml 显式 `permissions: contents: read`（测试起 localhost TcpListener + 临时 fake CLI，fork PR 也会跑，GITHUB_TOKEN 降权是零成本防线）；严禁 `pull_request_target`。
- pnpm store 与 cargo registry/target 缓存（macos runner 分钟贵，缓存是必须项不是优化项）。
- **landing protocol（写进本切片交付的协作规则，后续所有切片遵守）**: 每个切片在分支上开发 → commit → push → PR → CI 绿 + 跨模型 review 通过 → 合 main。旧禁令"不 commit/push"作废，改为"**不直接推 main、不自行合并**"。
- **跨模型 review gate 常规化（R9，硬验收项）**: Claude CLI review 超时（>180s 无输出）时 fallback 到 `codex exec` 只读 review。本切片内实跑一次 codex review 留档输出格式；gate 结论必须标注实际 reviewer 身份。这不是可选附带——review-gate.md 记录 9+ 次 Claude gate Blocked，codex fallback 就是主防线。

**验收**:
- Slice 1-3 已 commit（3 个逻辑 commit 或单 commit fallback），`git status` 干净。
- **触发验证 = 开 PR**（ci.yml 触发条件是 push main + PR；推普通分支不触发任何 run——`gh pr create` 后观察）。一次绿色 run；在 PR 分支临时加一个 `assert!(false)` commit 验证会红，随后 `git revert` 该 commit。
- **flake 检查（R8）**: 同一 commit `gh run rerun` 3 次全绿。15 个 `REACHNOTE_*` fake 注入与 mock HTTP listener 测试此前在本地沙盒出过 `Operation not permitted`——GitHub runner 上的行为必须实证，不能假设。若有用例在 runner 上不稳定，用 `#[ignore]` 属性隔离 + 开 issue 记录（不是 `--skip` 参数），不许静默重试掩盖。
- codex exec 只读 review 实跑一次：对 C0 的 PR diff 跑 `codex exec "<只读 review prompt>" -s read-only`，通过标准 = 输出含结论段且无 P0/P1 finding；输出格式与 reviewer 身份留档进 `memory/review-gate.md`。
- **执行分工**：ci.yml 编写、PR 创建、弄红/撤销由 Codex 执行（gh auth 已具备——见 §1.3 F3 仓库权限已确认）；Slice 1-3 落盘由 Yaocheng 执行（见前置）；最终合并由 Yaocheng 执行。

**禁止**: 不改任何应用代码；不加 release 流程外的签名/发布步骤。

### 2.3 Slice A：后端任务 worker + 事件推送（本计划核心刀）

**目标**: 任务生命周期由后端 worker 驱动，前端从"编排者"降级为"观察者"。修复 F1a（孤儿 Queued）、F1b（重复同步）、F1c（轮询三连击），为 Slice 6 隐藏窗口采集铺底。

**Scope（后端）**:

1. **新建 `src-tauri/src/worker.rs`**，把 lib.rs 中的 `run_capture_task_blocking` / `run_and_sync_capture_task_blocking` / `sync_capture_task_blocking` / `sync_pending_analyzed_tasks_blocking` / `retry_capture_task_blocking` 迁入（纯搬家 + 下述改动，lib.rs 只留 command 注册与轻量查询）。
2. **CAS 状态领取（不变量：所有状态迁移写入都带 from-status 谓词）**：store.rs 新增 `claim_next_queued_task(now)` / `claim_next_pending_sync_task(now)`——单条 `UPDATE tasks SET status=?, updated_at=? WHERE id = (SELECT id FROM tasks WHERE status=? ... ORDER BY CAST(created_at AS INTEGER) ASC, id ASC LIMIT 1)` 按 **FIFO** 领取（执行顺序不复用 UI 的 `created_at DESC` 排序，防旧任务饥饿——跨模型审查 Codex-3）；另有 `claim_task(id, from, to)` 供 retry/手动命令用，affected-rows 判定成功。**retry 的状态重置同样走 CAS**（`Failed→Queued` / `Failed→Analyzed` 带 WHERE status='failed'，affected=0 则返回当前任务不动作）——否则双击重试的盲写会复活 F1b（Eng F-B2）。`task_can_sync` 中允许 `Syncing` 重入的分支删除。
   **幂等 finalization 状态机（双模型 P0 共识，替代初版"幂等守卫"）**：sync 路径的执行序固定为三步——**`create_page` 成功 → 立即 CAS 写 `notion_page_id`（状态仍 Syncing）→ 写 `synced_at` + 置 Synced**；任何一步之后 crash 都由 finalization 收敛。初版只写"入口守卫"会与选取谓词打架——`task_needs_auto_sync` 要求 `notion_page_id IS NULL`（store.rs:772-776），page_id 写回后 crash → stale recovery 置 Failed → retry 重置 Analyzed → **永远不被 pending-sync 选中**，任务卡死在 Analyzed（本切片自己引入的 F1a 翻版）。正确设计是把"已有 page_id 的未完成任务收敛"做成一等路径：
   - **finalization 谓词（写死，Codex 不自行设计）**：`status IN ('analyzed','failed','syncing') AND notion_page_id IS NOT NULL`——命中即收敛：补写 `synced_at`（若空）并 CAS 置 Synced，零次 create_page。**注意这是语义变更**：`Failed + page_id 非空` 由 worker 自动收敛为 Synced，不再等用户手动重试——这是有意的（该状态只可能来自 crash window，页面已实际创建）。
   - **普通 pending-sync 谓词**：`status='analyzed' AND analysis_json IS NOT NULL AND notion_page_id IS NULL`，claim 到 Syncing 后走完整 sync。
   - 两个谓词各自实现为 `claim_next_finalization_task(now)` / `claim_next_pending_sync_task(now)`，worker 每轮先 finalization 后普通 sync，均 FIFO（`ORDER BY CAST(created_at AS INTEGER) ASC, id ASC LIMIT 1`）。
   - 既有孤儿类 `Analyzed && analysis_json IS NULL` 由 worker 判定为 `Failed(ParseFailed)`，不再装看不见。
   - 残留窗口（create HTTP 成功后、写 page_id 前进程死）仍会重复 page，显式声明接受；此窗口频率与队列长度正相关（队列越长用户中途 quit 概率越大），验收声明如实写。
2b. **worker 日志与故障可见性**：每次状态推进、claim 失败、emit 失败都 log 一行结构化日志（task id + from→to + 原因）。**Mutex 毒化防护（Eng F-A4）**：store 的 `lock_connection` 改用 `unwrap_or_else(|p| p.into_inner())` 消毒（SQLite 单语句原子，安全），防 worker catch_unwind + continue 把毒化 Mutex 变成"每 30s 记错、UI 永久 stale"的静默僵尸；worker 对 StoreError 连续失败 ≥3 次时 emit `worker:error` 事件让 UI 可见。
2c. **stale 阈值不变量（Eng F-A5）**：生效 stale 阈值 = `max(REACHNOTE_STALE_TASK_SECS, REACHNOTE_AI_TIMEOUT_SECS, REACHNOTE_READER_TIMEOUT_SECS, REACHNOTE_NOTION_TIMEOUT_SECS 的最大值 + 60s)`（默认 300/120/30/30 → 生效 300s），防用户调大 AI 超时后周期 recovery 误杀活体分析中的任务。"不做 single-instance 防护"（双开 app = 双 worker）写进 1.4 不做清单，靠 CAS 保证正确性、接受重复扫描。
2d. **worker panic 策略**：外壳每轮 `catch_unwind(|| worker_tick(...))`——panic 记 `[worker] panic:` 日志后 continue 下一轮（Mutex 已消毒不会毒化连锁）；连续 panic/StoreError ≥3 次 emit `worker:error`。**前端接收**：App.tsx `listen("worker:error")` → 顶部 `app-error-banner` 显示"后台任务处理异常，请重启应用或点任务行手动处理"（事件不得发进真空——没有接收方的可见性承诺等于没有）。
3. **worker 循环（执行基底写死，不留白——双模型 P0 共识）**：worker 的**单轮迭代必须是纯同步函数**：
   ```rust
   enum TickOutcome { Processed, Idle }   // Processed=本轮领到并处理了一个任务；Idle=无可领任务
   fn worker_tick(store: &TaskStore, emit: &dyn Fn(&Task) -> Result<(), String>) -> TickOutcome
   ```
   事件经闭包注入（返回 Result 使 emit 失败可被 tick 记日志），不直接持 AppHandle——这是可测性的硬前提：src-tauri 无直接 tokio dev-dep，普通 `#[test]` 起不了 async 循环也构造不出 AppHandle；tick 抽成同步函数后，孤儿领取/claim/finalization 全部在现有测试风格（in-memory store 直调）下可测。**drain 契约**：外壳循环在每次唤醒后连续调 `worker_tick` 直到返回 `Idle` 才回到等待——单测 #8 测的就是这个外壳契约（多条 Queued、一次唤醒、循环 tick 至 Idle 后断言全部处理）。
   外壳**采用 (b)**：独立 `std::thread` + `std::sync::mpsc::Receiver::recv_timeout(30s)` 当唤醒面（与现有全阻塞代码风格一致，不引 tokio 依赖，不碰 async 线程纪律）；`Sender` 经 `app.manage()` 注入，`create_capture_task`/`retry_capture_task` 写库成功后 `send(())`。（曾考虑 (a) async 循环 + spawn_blocking + Notify——需要显式加 tokio dev-dep 且 AppHandle 不可测，已否决。）
   - 启动先跑一次现有 `recover_stale_processing_tasks`（复用，不重写）；冒烟时用 `REACHNOTE_STALE_TASK_SECS` 注入小值（见验收）。
   - 每轮扫描：FIFO claim 一个 `Queued` 跑完整链路；无 Queued 则 claim 一个 pending-sync/finalization 任务。**drain 不变量：每次唤醒后循环处理直到无可领任务，才回到等待**——Notify 的 permit 合并/mpsc 缓冲语义下，这是无 lost-wakeup 的前提。
   - 兜底周期 30s 醒一次扫描（注入面 `REACHNOTE_WORKER_IDLE_SECS`，与既有 15 个环境变量模式一致），覆盖 stale recovery 周期化。
   - pending-sync 批处理：单任务 store 错误 **continue + log**，不得 `?` 短路整批（现 `sync_pending_analyzed_tasks_blocking` 的行为，迁入时修正）。
   - Notion 未配置时 Analyzed 任务置 Failed 一次即离开 pending 集合（现有语义保留），worker 不得每 30s 无限重试同一任务。
4. **事件推送**：worker 每次写回任务状态后 `app_handle.emit("task:updated", &task)`。失败不致命（log 即可），因为有兜底轮询。
5. **命令面收缩**：`run_capture_task` 命令**保留但内部改走 claim**（幂等：claim 失败返回当前任务状态，不报错）；前端提交流不再 invoke 它，改为 create 后由 worker 自动领取（notify 唤醒）。`retry_capture_task` 改为按现有规则重置状态（无 analysis 的 Failed→Queued；有 analysis 的 Failed→Analyzed）后 notify，不再前端等待全程结果；`recover_interrupted_tasks` / `sync_pending_analyzed_tasks` 命令保留但仅供手动兜底，前端不再周期调用。

**Scope（前端）**（Eng F-H1 三项竞态防护为硬要求，不是可选优化）:

6. App.tsx：`listen<Task>("task:updated")` → `upsertTask`；删除 1.2s 三连击定时器与 `refreshTasks` 中的 recover/sync_pending 调用；保留一个**低频**（30s）`list_capture_tasks` 兜底轮询 + 窗口重获焦点时刷新一次。三个竞态防护：
   - **先 listen 后快照**：必须 `await listen()` 注册成功后再拉首次 `list_capture_tasks`——worker 在 setup 即开跑（启动就领孤儿），注册前 emit 的事件静默丢失；
   - **StrictMode cleanup**：`listen()` 返回 `Promise<UnlistenFn>`，effect cleanup 必须正确 await/cancelled-flag 处理，防 React 18 StrictMode 双挂载积累双监听器；
   - **单调合并**：兜底轮询的 `setTasks` 不得全量盲替换——poll 响应在途时事件先到，旧快照后到会把 UI 状态回滚一拍；按 task id 合并且 `updated_at` 较新者胜（改造 `upsertTask`）。
7. QueueView：`queued` 状态显示"等待处理"而非无限转圈；同时给 `queued` 行加"立即处理"操作——invoke 保留的 `run_capture_task`，其语义钉死为 **claim + inline 执行全链**（不是只 notify worker：worker 死了按钮必须仍然有效，否则兜底承诺落空）；claim 失败（已被 worker 持有）返回当前任务状态不报错。worker 异常时 queued 任务必须有用户可点的兜底入口，否则保留的手动命令没有 UI 入口等于不存在（Eng F-H2）。

**验收**:

```bash
pnpm typecheck && pnpm build
cargo test -p reachnote-core
cargo test --manifest-path src-tauri/Cargo.toml   # 新增测试见下方清单
cargo check --manifest-path src-tauri/Cargo.toml
git diff --check
pnpm tauri dev   # 人工冒烟，硬性证据
```

**新增单测清单（worker_tick 同步可测，双模型审查补齐）**：
1. claim CAS：双线程（`Arc<TaskStore>` + 2×`std::thread`）对同一任务领取，恰好一个成功。
2. claim 负路径表驱动：id 不存在 → false；from_status 不匹配 → false；FIFO：两条 Queued 先创建者先领取。
3. Queued 孤儿被 `worker_tick` 领取推进（fake 注入下到终态）。
4. Syncing 无法重入（`task_can_sync` 收紧后）。
5. **finalization 穿谓词端到端**：`Failed + analysis_json + notion_page_id 存在` → retry → 终态 Synced、`synced_at` 补写、**零次** create_page（不是只测守卫函数——测选取谓词真的放行）。
6. early-write crash 恢复：`Syncing + notion_page_id 存在` → stale recovery → finalization → Synced，不重复 create。
7. retry 并发：两路对同一 Failed 任务并发 retry（CAS 化后），恰好一次进入 sync。
8. drain 语义：绕过唤醒直接 insert 多条 Queued，一次唤醒后全部处理完。
9. Notion 未配置：Analyzed 任务置 Failed 一次后不再被 pending 集合选中。
10. `Analyzed && analysis_json IS NULL` 孤儿 → worker 判定 Failed(ParseFailed)。
11. worker panic：注入会 panic 的 fake，断言外壳 catch_unwind 后继续处理下一任务、连续 3 次后 emit `worker:error`（emit 闭包可注入计数）。

`pnpm tauri dev` 冒烟场景（由 maintainer 执行；Codex 交付冒烟清单与预期结果，逐条报告实际结果）:
1. 向 **QA 数据目录**插入一条 `queued` 任务后启动。注意裸 `pnpm tauri dev` 读 `tauri.conf.json` 的 `com.reachnote.app`——QA 隔离需 `pnpm tauri dev --config src-tauri/tauri.qa.conf.json`（identifier `com.reachnote.qa`）或直接走 `scripts/desktop-smoke-qa.sh` 流程。插入配方：
   ```bash
   sqlite3 "$HOME/Library/Application Support/com.reachnote.qa/reachnote.db" \
     "INSERT INTO tasks (id, url, source_type, template_id, status, provider_id, model, source_domain, created_at, updated_at)
      VALUES ('smoke-orphan-1', 'https://example.com/a', 'article', 'web_article', 'queued', 'claude_cli', 'Claude CLI', 'example.com',
              strftime('%s','now'), strftime('%s','now'));"
   ```
   启动 app → **无需任何点击**，任务自动推进（fake provider 注入下走到 analyzed/synced 或按注入失败）。这是 F1a 的直接回归验证。
2. 采集一条任务，处理中途 `set_compact_mode` 隐藏窗口 → 任务在后台继续完成（查 DB 终态）。
3. 采集过程中观察 devtools：不再出现 1.2s 级的 `recover_interrupted_tasks` / `sync_pending_analyzed_tasks` invoke；UI 状态由事件驱动实时变化。
4. 分析中途强杀 app → **以 `REACHNOTE_STALE_TASK_SECS=5` 注入重启** → stale 任务恢复为 failed 且可重试（默认 300s 阈值下刚强杀的任务不满足 stale 条件，不注入会看到"未恢复"的假失败——这是脚本化冒烟的已知坑，不是 bug）。
5. Notion 未配置时，analyzed 任务按现有语义处理，不被 worker 无限重试（失败即离开 pending 集合，原有行为保持）。

**禁止**:
- ❌ 不做并发 worker（并发度=1，顺序循环）；不引入 job queue crate；不引入日志 crate（用 `eprintln!` 前缀 `[worker]`）。
- ❌ 不改 `TaskStatus` 枚举、不加新状态（PRD P1-1 决议）。
- ❌ 不动 provider.rs / reader.rs / notion.rs 的注释与管道模式（历史事故疫苗）。
- ❌ 不删 `recover_interrupted_tasks` / `sync_pending_analyzed_tasks` 命令本身（保留手动兜底面）。
- ❌ 不在本刀里做 destination 路由（Slice 4 的事）。
- ❌ 不直接推 main、不自行合并（landing protocol：分支 → commit → push → PR → CI 绿 + review 通过后由 maintainer 合并）。

**收尾（landing protocol + 如实报告）**：分支 commit → push → 开 PR（贴 CI run URL）→ codex/claude 只读 review 结论留档。任一验收命令失败 → 修复后重跑全套；无法修复 → 停止并如实报告失败点与已尝试项。`pnpm tauri dev` 冒烟由 maintainer 执行（Codex 交付时提供冒烟清单与预期结果，无冒烟证据不得声称切片完成）。同轮更新 `memory/backend-progress.md`、`memory/frontend-progress.md`、`memory/integration-progress.md`（改动文件、验证结果、残留风险、日期）。

### 2.4 Slice B：模板/路由单一真源

**目标**: 消除 F2 的 TS/Rust 双源。**审查修正（R4）**：初版"注释同步 fixture"方案恰是 F2 抱怨的漂移机制（靠 convention 不靠 contract），且 Rust 单测无法断言 TS 行为、AD8 又拒绝 vitest——验收不可证伪。改为**路由规则数据化**，逻辑单源在 Rust。

**Scope**:
- **模板数据单源**：前端启动时（`loadSetup` 并行）`invoke("list_templates")`，TemplatesView / Capture 下拉 / `templateLabel` 全部改用后端数据。**字段映射契约**：Rust `ResearchTemplate { id, name, description, prompt_profile, ... }`（snake_case，serde 原样序列化）→ TS 侧新建 `BackendTemplate` 类型逐字对齐 snake_case（`name` 就是 `name`，不改叫 `title`——TS 消费处改字段名）；`src/constants.ts` 的 `TEMPLATES` 缩减为 UI-only 展示映射 `{ id → { icon, chips } }`；`compatibleSourceTypes`/`state` 字段废弃（后端 `compatible_source_types`/`enabled` 为准）。旧 `article` 别名归一化（TS `normalizeTemplateId` × Rust `canonical_template_id` 是第三份重复逻辑）一并数据化：alias 映射随规则表下发，TS 手写 alias 判断删除。
- **路由规则数据化**：`crates/core` 新增平台匹配规则表，schema 与匹配语义写死：
  ```rust
  struct PlatformRule { platform_key: String, exact_hosts: Vec<String>, host_suffixes: Vec<String>, path_keywords: Vec<String> }
  ```
  **匹配优先级：精确域名 > 域名后缀 > 路径关键词；同优先级内按规则表顺序先命中先赢；全不中 → `web`**（与现 `sourcePlatformKeyForUrl` 行为对齐：`youtu.be` 归 exact_hosts，`rss/feed` 归 path_keywords）。`suggest_template_id_for_url` 改为查表实现；规则表 + platform→template 映射 + alias 映射**并入 `list_templates` 返回**（一次 invoke 全拿，不新增 command）。前端改为数据驱动匹配（启动拿一次规则本地闭包匹配），TS 手写规则逻辑删除。Rust 表驱动单测：github/youtube(含 youtu.be)/bilibili/rss/twitter/普通网页/畸形 URL 各一。
- **F2 第 4 面（平台文案）显式不做**：CaptureView/SettingsView 各自维护的平台可用性中文文案不在本刀收敛（它依赖 doctor message 语义，等 Slice 7 平台扩展时随真实路由一并处理）——记录进 1.4 不做清单语义。

**验收**（完整命令块，B 单发时自带）:
```bash
pnpm typecheck && pnpm build
cargo test -p reachnote-core          # 含规则表表驱动单测
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
git diff --check
pnpm tauri dev                        # 冒烟由 maintainer 执行
```
- 模板页文案与 `crates/core/src/template.rs` 一致的机械断言：`grep -rn "description\|promptProfile" src/constants.ts` 零命中文字性字段。
- 改一处 Rust 模板描述、重启后 UI 跟随变化（验证后撤销）；改一条 Rust 路由规则、Capture 来源检测跟随变化（验证后撤销）。
- **证伪面（精确 grep）**：`grep -rn "github\.com\|youtube\.com\|bilibili\.com\|xiaohongshu" src/ --include='*.ts' --include='*.tsx'` 仅允许命中 `src/utils.ts` 的 `sourcePlatformFallbackName` 显示名映射（无域名）与测试文件；`sourcePlatformKeyForUrl` 中的手写域名分支必须为零。
- **收尾同 Slice A**：分支 → commit → push → PR → CI 绿 → review 留档；失败即停并如实报告；memory 四件套更新。

**禁止**: 不改模板 id 集合、不加自定义模板能力（PRD 非目标）；不动 prompt 构建逻辑；不引入前端测试框架（规则逻辑已单源在 Rust，TS 只剩数据匹配胶水）；不动平台可用性文案（见上）。

### 2.5 之后：回 PRD 主线

Slice 6（前移，见 2.1）→ 4 → 4.5 → 5 → 7 按 `plans/prds/20260701-1447-*.prd.md` 及其审查决议执行，不在此重复。接缝提示写给未来的切片 prompt：

- **Slice 6（A 之后立即做）** 的 `capture_from_clipboard` 只需"建任务 + notify worker"，隐藏窗口处理能力由 Slice A 保证——这正是 A 先行的回报。第一版 destination 就是现有 Notion 路径（app_settings 默认值已就绪），不等 Slice 4 abstraction。按 PRD 批注 P2-5 补 clipboard-manager 插件与 macOS 辅助功能授权区分。
- **Slice 4** 的 destination 路由实现在 worker 的 sync 缝上，但有**结构约束（R10）**：worker.rs 只准 dispatch 到独立 `src-tauri/src/destinations/` 模块（adapter trait + per-kind 文件），worker.rs 本体不得增长 adapter 代码——App.tsx 的 God-file 教训（PRD P1-4）不在 worker.rs 上重演。同刀落 F4a（结构化错误 `{kind, message}`）与 F4b（`PRAGMA user_version` 迁移）。
- **Slice 7 前补一条风险测试**：Agent-Reach 是 source 侧单点依赖（无通用 read 命令、doctor JSON 形状仅由一份 fixture 固化）。`normalize_doctor_output` 需补"字段变形/新增未知字段/条目缺字段"的降级用例——UI 退化为 unknown 而非崩（R7）。

### 2.6 产品侧待议（跨模型审查提出，超出本计划边界，留给 PRD 下一轮修订）

两个模型独立审查后在产品层各自提出了超出"架构优化"边界的质疑，如实记录、不擅自裁决：

- **Codex（P0 级质疑）**: ① 用户 wedge 过宽——15 平台矩阵 + 多模板 + 多目的地并行推进，没有一个尖锐场景先闭环；建议收窄为 `GitHub/web URL → 后台 worker → 本地持久研究卡 → Notion → 快捷键采集` 单链路打穿。② 平台矩阵对用户可能是"限制清单"而非"能力感"（大量 needs_login/not_supported），建议只暴露能端到端闭环的 2-3 个来源，其余收进 diagnostics。③ webhook 三件套是高成本低留存面（为浅层消息推送建 SecretStore/error taxonomy/settings 全家桶）。④ 缺 adoption metrics（time-to-first-card、7 日重复采集、失败恢复率）；PRD falsifier 是架构状态而非产品状态。⑤ 本地知识资产缺位（source snapshot/dedupe/card history）——没有它，产品是"URL 转发到 Notion 的任务器"。
- **Claude subagent（与 Codex 共识）**: shortcut 是唯一差异化时刻，竞品（Raycast AI / Readwise / Obsidian & Notion clipper）按周迭代采集入口，顺序让位等于让出窗口——已采纳为 R2 前移。Agent-Reach 可被任何人 wrap，护城河只在 UX 完成度与迭代速度。
- **本计划的处理**: Slice 6 前移（已改 2.1）是架构计划权限内能做的最大产品修正；wedge 收窄、平台矩阵瘦身、webhook 降级、metrics 补齐属于 PRD 层决策，需要 Yaocheng 在 PRD 下一轮修订裁决，本计划不越权改 PRD 既定切片的存废。

---

## Part 3 · 决策依据备忘（P3）

- **保留的核心不变量**: 加法式迁移不 rename；`analysis_json` 存在时重试不重跑分析；能力矩阵不高估真实能力；所有长操作不占 UI 主线程；测试不触外网。以上任何切片不得破坏。
- **最小一致改动的取舍**: worker 并发度=1 而不是线程池——当前瓶颈是"有没有后端执行者"，不是吞吐；CAS claim 而不是全局任务锁——它同时是并发防护和未来多执行者的地基，成本只是一条 UPDATE。
- **第一个会坏的假设**: "任务 runner 与创建者耦合（只有前端提交流会驱动任务）"。Slice A 之后，这个假设从系统中移除。

<!-- AUTONOMOUS DECISION LOG -->
## 审查附录（/autoplan 产出）

> 阅读顺序说明：A1-A6 为 CEO 阶段产出，A8-A10 为 ENG 阶段（AD22-30），A11-A13 为 DX 阶段（AD31-39），A7 为 CEO 决策表（AD1-21，置于文末）。编号按阶段分配、非文内顺序，均连续有效。附录是审查证据与决策档案；**给 Codex 派发切片时不要携带附录**——按 §2 各切片自带的 scope/验收/禁止/收尾派发（DX 审查裁定 AD39：执行 prompt 与审查档案分离）。

### A1 · CEO 双声部共识表

```text
CEO DUAL VOICES — CONSENSUS TABLE
═════════════════════════════════════════════════════════════════════
  维度                        Claude subagent   Codex        共识
  ─────────────────────────── ───────────────── ──────────── ─────────
  1. 前提有效？                部分（R5 论据错）  部分（W3）    DISAGREE→已修正表述
  2. 是对的问题？               是（排序错）       是（量纲错）  CONFIRMED（方向对）
  3. Scope 校准正确？           否（6 应前移）     否（6 应前移）CONFIRMED→已前移
  4. 备选方案探索充分？          否（R4/R6）       否（W6）     CONFIRMED→已补 AD13-16
  5. 竞争/市场风险覆盖？         否（R7 竞品窗口）  否（W1 wedge）CONFIRMED→2.6 待议
  6. 6 个月轨迹健全？           否（R1 管线断）    否（W5 webhook）DISAGREE→R1 已修，W5 待议
═════════════════════════════════════════════════════════════════════
```

跨模型独立一致（高置信信号）：① Slice 6 前移；② 验证/交付管线是比执行模型更早的瓶颈；③ Agent-Reach 可被任何人 wrap，护城河在 UX 迭代速度。分歧：Codex 主张砍 webhook/矩阵（产品层），Claude subagent 认为属 PRD 权限——按 AD21 记录待议，不在本计划裁决。

### A2 · What already exists（子问题 → 既有代码映射）

| 子问题 | 既有代码 | 本计划复用方式 |
| --- | --- | --- |
| 中断恢复 | `recover_stale_processing_tasks`（store.rs:370） | worker 启动先跑一次，周期兜底，不重写 |
| 补同步 | `sync_pending_analyzed_tasks_blocking`（lib.rs:536） | 迁入 worker.rs，改走 claim |
| 超时/管道安全的子进程执行 | `run_process` 三线程模式（provider.rs:252） | 原样保留，禁动 |
| 状态机全链路 | `run_and_sync_capture_task_blocking`（lib.rs:410） | 纯搬家进 worker.rs + claim 化 |
| 模板真源 | `BUILT_IN_TEMPLATES` + `list_templates`（已存在，前端零调用） | Slice B 接线，不新建 |
| 测试注入面 | 15 个 `REACHNOTE_*` 环境变量 + fake CLI/mock HTTP | CI 直接复用，须验 runner 兼容（AD18） |
| CI 构建配方 | release.yml 的 pnpm/Rust 步骤 | ci.yml 抄配方去掉签名/打包 |

### A3 · Dream state delta

```text
CURRENT                          本计划后                        12 个月理想
前端驱动、无后端执行者            worker 常驻、CAS 领取、事件推送     多来源自动采集管线（cron/watch）
Codex 自跑自报、gate 反复超时     CI + landing protocol + codex     全绿主干、每片独立可回滚
                                 review 常规化
模板/路由 TS+Rust 双源           Rust 单源、数据驱动路由            模板含 destination 映射单源
采集=打开窗口手动贴 URL           （Slice 6 前移后）快捷键后台采集    浏览器/多入口采集
差距：本计划补齐执行模型与工程流地基，让 PRD Slice 4-7 在其上安全叠加；
不解决（留给 PRD 层）：wedge 收窄、本地知识资产、adoption metrics。
```

### A4 · Error & Rescue Registry（Slice A 新增/变更路径）

| 路径 | 可能失败 | 错误类别 | 挽救动作 | 用户可见 |
| --- | --- | --- | --- | --- |
| `claim_task` UPDATE | DB 锁/IO 错误 | StoreError | 返回 Err，worker log 后跳过本轮，30s 兜底重扫 | 任务暂留原状态，队列可见 |
| claim 失败（已被持有） | 竞态正常路径 | — | 返回 false，直接跳过 | 无感知 |
| worker 循环内 panic | 逻辑 bug | — | **GAP→规定**: 循环体 catch_unwind 或错误分支 continue，禁止整个 worker 死亡；log 必留 | 任务停滞→stale recovery 兜底 |
| `emit("task:updated")` 失败 | webview 未就绪/序列化 | tauri::Error | log 一行（AD 2b），30s 兜底轮询覆盖 | 状态延迟 ≤30s |
| create_page 成功后写库失败 | crash window | — | finalization 状态机（AD22，取代 AD13）：create→立即 CAS 写 page_id→置 Synced；已有 page_id 的未完成态被 finalization 谓词收敛 | 极小概率重复 page（残留窗口显式声明） |
| notify 唤醒丢失 | 逻辑竞态 | — | 30s 兜底扫描保证最终执行 | 最坏延迟 30s |
| Notion 未配置 | 用户未配置 | NotionUnauthorized | 现有语义保留：置 Failed 可重试，不无限重试 | 失败原因行内可见 |

### A5 · Failure Modes Registry

| 路径 | 失败模式 | 挽救? | 测试? | 用户可见? | 日志? |
| --- | --- | --- | --- | --- | --- |
| Queued 孤儿（F1a） | app 退出后无 runner | Y（worker 领取） | Y（验收#1） | Y（自动推进） | Y |
| 双执行者同步（F1b 活体） | 轮询与主链竞态 | Y（CAS） | Y（双领取单测） | N/A | Y |
| crash window 重复 page（F1b 残留） | create 后写 page_id 前死 | 部分（finalization 缩窗，AD22） | Y（单测#6/#7） | 显式声明 | Y |
| worker 死循环/panic | 逻辑 bug | Y（catch_unwind+continue+消毒） | Y（单测#11） | ≥3 次 emit worker:error→banner | Y |
| CI flake | runner 沙盒差异 | Y（3 次 re-run 验收+隔离协议） | Y（验收内置） | 红勾可见 | Y |
| doctor JSON 变形（R7） | 上游 breaking change | Y（降级 unknown） | Slice 7 前补 | 矩阵显示 unknown | Y |
| **无 CRITICAL GAP**（原 worker panic 与 emit 失败两项静默风险已在 scope 中规定为必须可见） | | | | | |

### A6 · CEO Completion Summary

```text
+====================================================================+
|            MEGA PLAN REVIEW — COMPLETION SUMMARY (CEO)             |
+====================================================================+
| Mode                 | SELECTIVE EXPANSION（autoplan 固定）          |
| System Audit         | 12 commits/30d；无 stash；无 TODO 注释；      |
|                      | 热点=lib.rs/App.tsx/memory；dirty tree 30+ 文件|
| Step 0 premises      | 用户确认 4 前提（D1=A）                       |
| 0C-bis alternatives  | worker 方案 A/B/C 已比较（AD3）+hotfix 组合(AD16)|
| Sec 1 (Arch)         | 3 issues: R3 幂等缺失/R10 God-file/R5 表述    |
| Sec 2 (Errors)       | 7 路径映射, 2 GAP→已规定为 scope（A4）        |
| Sec 3 (Security)     | 无新面（无新 secret/endpoint；token 债务在 4.5）|
| Sec 4 (Data/UX edge) | queued 显示语义 1 项（已在 A scope#7）        |
| Sec 5 (Quality)      | R4 注释同步反模式→已改数据化                  |
| Sec 6 (Tests)        | 新增 claim/幂等/孤儿领取 3 类；flake 验收（AD18）|
| Sec 7 (Perf)         | 30s 兜底轮询替代 1.2s 三连击；无新热点        |
| Sec 8 (Observ)       | 2 gap→AD6/2b（worker 日志+emit 失败可见）     |
| Sec 9 (Deploy)       | landing protocol 落 C0（AD11）；CI=部署面     |
| Sec 10 (Trajectory)  | 可逆性 4/5（worker 可禁用回旧命令）；R10 防债  |
| Sec 11 (Design)      | SKIPPED（无 UI scope，探测 0 命中）           |
+--------------------------------------------------------------------+
| NOT in scope         | 1.4 节 5 项 + 2.6 产品待议 5 项               |
| What already exists  | A2（7 项映射）                               |
| Dream state delta    | A3                                          |
| Error/rescue registry| A4（7 路径, 0 未处理 GAP）                    |
| Failure modes        | A5（6 项, 0 CRITICAL）                       |
| Outside voices       | codex + claude subagent 双声部均完成          |
| 决策审计             | AD1-21（下表）                               |
+====================================================================+
```


### A8 · ENG 双声部共识表 + 架构图

```text
ENG DUAL VOICES — CONSENSUS TABLE
═════════════════════════════════════════════════════════════════════
  维度                     Claude subagent      Codex           共识
  ──────────────────────── ──────────────────── ─────────────── ─────────
  1. 架构健全？             骨架对,基底留白(F-A1)  同(P0-2)        CONFIRMED→已写死
  2. 测试覆盖充分？          否(6 缺口+可测性F-A2)  否(7 漏测)      CONFIRMED→清单已补
  3. 性能风险覆盖？          是(10x=排队变长)      LIFO 饥饿(P1-3)  DISAGREE→FIFO 已采纳
  4. 安全威胁覆盖？          CI 4 项加固           CI 2 项(重叠)    CONFIRMED→已补
  5. 错误路径处理？          F-B1 谓词卡死(HIGH)   P0-1 同一发现    CONFIRMED→finalization
  6. 部署风险可控？          dist 硬顺序+Node 钉版  同(P1-6)        CONFIRMED→已写明
═════════════════════════════════════════════════════════════════════
```

跨模型独立一致（高置信）：① **幂等守卫与 `task_needs_auto_sync` 谓词冲突会造成任务永久卡死 Analyzed**——两个模型在互不知情下发现同一条 P0/HIGH（初版计划自己引入的 F1a 翻版），已改为 finalization 状态机；② worker 执行基底不能留白（async 直跑阻塞违反仓库纪律）；③ `git diff --check` 在 CI 是 no-op；④ dist/ 是 cargo 步骤硬前置；⑤ 前端 listen 竞态 + StrictMode。Codex 独有：FIFO claim 防饥饿。Claude 独有：Mutex 毒化僵尸、stale×超时不变量、可测性结构（worker_tick 抽取）。

**Slice A 架构图（审查后终版）**：

```text
┌─ Frontend (React) ─────────────────────────────────────────────┐
│ CaptureView.submit → invoke create_capture_task ──────┐        │
│ QueueView.retry    → invoke retry_capture_task ───────┤        │
│ QueueView.queued 行"立即处理" → run_capture_task(claim)─┤        │
│ listen("task:updated") ←──(先注册,后拉快照)──┐          │        │
│ 30s 兜底 list_capture_tasks (updated_at 合并)│          │        │
└──────────────────────────────────────────────┼──────────┼───────┘
                                               │ emit     │ invoke
┌─ Tauri shell (Rust) ─────────────────────────┼──────────┼───────┐
│ commands: create/retry ──写库(CAS 重置)──→ notify ──┐   │       │
│                                                     ▼   ▼       │
│ ┌─ worker（外壳: std::thread+mpsc 或 async+spawn_blocking）──┐  │
│ │ loop: recv_timeout(REACHNOTE_WORKER_IDLE_SECS=30s)        │  │
│ │   → worker_tick(store, emit_fn)  [纯同步,可单测]           │  │
│ │      ① recover_stale (阈值=max(stale, 超时+60s))           │  │
│ │      ② FIFO claim Queued → read→analyze→sync 全链         │  │
│ │      ③ FIFO claim pending-sync/finalization                │  │
│ │      ④ drain until empty → 回到等待                        │  │
│ └────────────────────────────────────────────────────────────┘  │
│ store: claim_next_*(FIFO) / claim_task(id,from,to) [全部 CAS]   │
│        lock_connection 毒化消毒 into_inner                       │
│ 状态机: Queued→Reading→Analyzing→Analyzed→Syncing→Synced        │
│         └Failed(可重试)  finalization: *+page_id→Synced          │
└──────────────────────────────────────────────────────────────────┘
```

### A9 · ENG Completion Summary

```text
+====================================================================+
|              ENG PLAN REVIEW — COMPLETION SUMMARY                  |
+====================================================================+
| Scope challenge      | 通过：A2 已映射 7 项复用；worker 是缺失能力    |
|                      | 而非重复建设；复杂度检查=8 文件级,justified    |
| Sec 1 (Architecture) | 3 HIGH: 执行基底留白/可测性缺失/谓词冲突      |
|                      | → 全部写死进 scope；ASCII 图见 A8            |
| Sec 2 (Code Quality) | 2: pending批处理?短路→continue; 盲写→CAS化    |
| Sec 3 (Test Review)  | 6 缺口→10 条单测清单落验收；并发测试可行性    |
|                      | 已核实(Arc+std::thread, 无需 tokio)          |
| Sec 4 (Performance)  | LIFO饥饿→FIFO claim; 队头阻塞→显式接受并声明  |
| Failure modes        | +3: Mutex毒化僵尸/stale误杀/listen竞态→已防护 |
| Test plan artifact   | 写盘 ~/.gstack/projects/.../test-plan        |
| Deferred (TODOS)     | single-instance / Notion 彻底去重 → 1.4 不做  |
+====================================================================+
```

### A10 · ENG 阶段决策增补

| # | Phase | Decision | Classification | Principle | Rationale | Rejected |
|---|-------|----------|----------------|-----------|-----------|----------|
| AD22 | ENG-voices | 幂等守卫升级为 finalization 状态机；`task_needs_auto_sync` 谓词放行 page_id 非空 | Mechanical | P1 | 双模型同发现 P0：early-write × 谓词 `notion_page_id IS NULL` 冲突 → 任务永久卡 Analyzed | 仅入口守卫（测试全绿但谓词不放行） |
| AD23 | ENG-voices | worker_tick 抽为纯同步函数，事件经闭包注入；外壳推荐 std::thread+mpsc | Mechanical | P5 | 无 tokio dev-dep 且 AppHandle 不可测——不抽 tick 则验收测试写不出 | async 循环直跑阻塞 |
| AD24 | ENG-voices | claim 改 FIFO（created_at ASC），执行序与 UI 序分离 | Mechanical | P1 | `list_tasks` 是 DESC，复用则持续输入下旧任务饥饿（Codex 独有发现） | 复用 UI 排序 |
| AD25 | ENG-voices | retry 重置 CAS 化；"所有状态迁移带 from-status 谓词"升为切片不变量 | Mechanical | P1 | 双击重试盲写会复活 F1b（换入口） | 仅两入口 CAS |
| AD26 | ENG-voices | lock_connection 毒化消毒（into_inner）+ worker 连续失败≥3 emit worker:error | Mechanical | P1 | catch_unwind+continue 会把毒化 Mutex 变永久静默僵尸 | 无限 continue |
| AD27 | ENG-voices | stale 阈值=max(配置, 超时最大值+60s)；不做 single-instance 写进不做清单 | Mechanical | P3 | 用户调大 AI 超时后周期 recovery 误杀活体任务；双实例靠 CAS 兜底 | 内存记 current-task |
| AD28 | ENG-voices | CI 四项加固：Node 钉版/diff --check 范围化/dist 硬顺序注记/permissions 收敛 | Mechanical | P1 | diff --check 在干净 checkout 是 no-op；9947a2e 踩过 Node 版本坑 | 原样照搬本地命令 |
| AD29 | ENG-voices | 前端三竞态防护（先 listen 后快照/StrictMode cleanup/updated_at 单调合并）写为硬要求 | Mechanical | P1 | worker setup 即跑，注册前事件全丢；StrictMode 双挂载是 Tauri 高频事故 | 事后修 |
| AD30 | ENG-voices | queued 行加"立即处理"入口 | Mechanical | P2 | worker 异常时 queued 无任何用户可点路径；保留的手动命令无 UI 入口=不存在 | 仅改文案 |

### A11 · DX 双声部共识表（阶段 3.5，开发者=Codex + maintainer）

```text
DX DUAL VOICES — CONSENSUS TABLE
═════════════════════════════════════════════════════════════════════
  维度                        Claude subagent     Codex         共识
  ─────────────────────────── ─────────────────── ───────────── ─────────
  1. 拿到即可开工（<5min）？    否(C2 主体/凭据)     否(同)        CONFIRMED→已补执行分工
  2. 指令无歧义可执行？         否(C3-C9)           否(9 处留白)   CONFIRMED→已钉死
  3. 验收机械可查？            半数不可查           同             CONFIRMED→已改写
  4. 失败协议存在？            仅 C0 flake 有       同             CONFIRMED→统一收尾段
  5. 内部一致（新旧无并存）？   否(C1/C7 5 处)       否(5 处 stale) CONFIRMED→已清理
  6. 达到 slice2 house style？ 否(缺件表)           否(建议拆 3 文件) CONFIRMED→见裁定
═════════════════════════════════════════════════════════════════════
```

双模型独立同发现（高置信）：① Slice A 禁止项"不 commit/push"与 C0 landing protocol 直接冲突（P0 级 DX 矛盾——恰好复刻本计划自己诊断的 R1"防线接不上执行流"）；② sync 执行序（early-write 是否保留）在 finalization 改写后失去正面定义，Codex 可能读成"不写 page_id"——crash window 原样回归且单测全绿发现不了；③ pending-sync/finalization 选取谓词留白在本计划自己标记的 P0 事故区；④ A4/AD13 旧设计残文与新设计并存；⑤ QA 目录冒烟与裸 `pnpm tauri dev` 配置不匹配。全部已修（见 AD31-38）。

**DX 裁定（拆文件建议）**：两个模型都建议把 C0/A/B 抽成三个独立 slice2 风格 handoff 文件、本文档降级为真源引用。**采纳方向、推迟执行**——派发前由派发者（Yaocheng 或下一轮 Claude 会话）按本文档 §2 各切片逐段抽取成独立 prompt；本轮不再新建三个文件（避免同一内容四处维护，且 §2 已按"每片自带 scope/验收/禁止/收尾"补齐到可抽取粒度）。

### A12 · DX 阶段决策增补

| # | Phase | Decision | Classification | Principle | Rationale | Rejected |
|---|-------|----------|----------------|-----------|-----------|----------|
| AD31 | DX-voices | C1: 删除 Slice A"不 commit/push"禁止项，A/B 各加 landing 收尾段（分支→PR→CI→review→maintainer 合并） | Mechanical | P1 | 双模型 P0：与 C0 landing protocol 直接冲突，Codex 按字面执行则 CI 永不触发 | 保留旧禁令 |
| AD32 | DX-voices | C3: sync 执行序正面写死（create→立即 CAS 写 page_id→synced_at+Synced），finalization 谓词 SQL 级钉死 | Mechanical | P1 | finalization 收敛的前提是 early-write 存在；改写后只剩"被否决初版"口吻，Codex 可能读反 | 留给实现者推断 |
| AD33 | DX-voices | C2: C0 补执行分工（Slice 1-3 落盘=Yaocheng、允许单 commit fallback；PR/弄红/re-run=Codex）；触发验证改"开 PR" | Mechanical | P3 | "推分支触发 run"与自定义 trigger 矛盾；30+ 文件 hunk 级分块不可交给 agent | 全交 Codex |
| AD34 | DX-voices | C6: CI 配方改抄 release.yml 现行（pnpm/action-setup@v4 + Node 24），弃 corepack；补 fetch-depth: 0 | Mechanical | P4 | 三处安装口径互相打架；diff 范围检查在 depth=1 下 bad revision | corepack |
| AD35 | DX-voices | C5: 冒烟写死 `--config src-tauri/tauri.qa.conf.json` + sqlite3 插入配方全列 | Mechanical | P1 | 裸 tauri dev 读 com.reachnote.app，照做必假失败或误伤真实 DB | 留给执行者变通 |
| AD36 | DX-voices | C7/C8: 清理全部陈旧引用（A4/A5/AD13 幂等守卫措辞、store.rs:746、Part 3 旧表述、F4c 时机、AD9 语义钉死为 claim+inline、TickOutcome 定义、emit 签名带 Result、worker:error 前端接收、panic 策略上移+单测#11、日志用 eprintln 不引 crate） | Mechanical | P5 | 新旧设计并存加剧歧义；引用不解析违反自洽 prompt 标准 | 留档不改 |
| AD37 | DX-voices | C9: Slice B 补字段映射契约（snake_case 逐字对齐）、PlatformRule schema、匹配优先级（精确>后缀>路径，表序先中先赢）、alias 数据化、F2 第 4 面显式不做、完整验收命令块、精确 grep 证伪面 | Mechanical | P1 | 三种匹配无优先级会产出第三份行为；"逐字一致"人眼比对不可机械查 | 保持描述级 |
| AD38 | DX-voices | 统一失败协议进 A/B 收尾：任一命令红→修复重跑全套；修不动→停止+如实报告；冒烟=maintainer 执行，无证据不得声称完成；memory 四件套同轮更新 | Mechanical | P1 | slice2 的诚实护栏一条都没继承，对"防 Codex 声称通过"主题是退步 | 静默假设成功 |
| AD39 | DX-voices | 拆三个独立 handoff 文件：采纳方向、推迟到派发时执行 | **Taste→gate** | P3 | 避免同一内容四处维护；§2 已补齐到可抽取粒度 | 本轮立即拆 |

### A13 · DX Scorecard（对象：本计划作为 Codex/maintainer 的开发接口）

| 维度 | 初始 | 修订后 | 说明 |
|---|---|---|---|
| Time-to-first-action | 3/10 | 8/10 | C0 曾缺主体/凭据/触发定义；现执行分工+配方齐 |
| 指令无歧义 | 4/10 | 9/10 | 9 处留白（TickOutcome/谓词/执行序/QA 配置…）全部钉死 |
| 验收可机械核查 | 5/10 | 9/10 | grep 证伪面精确化、单测 11 条、冒烟含配方与注入 |
| 失败协议 | 2/10 | 8/10 | 统一收尾三行 + flake 隔离 + review fallback 判定标准 |
| 内部一致性 | 4/10 | 9/10 | C1/C7 等 10 处 stale/冲突清理；附录加阅读顺序说明 |
| house-style 契合 | 4/10 | 7/10 | 收尾/禁止/命令块补齐；拆文件推迟（AD39） |
| **总分** | **3.7/10** | **8.3/10** | 派发前按 AD39 抽取成独立 prompt 可到 9+ |

### A7 · Decision Audit Trail（CEO 阶段 AD1-21）

| # | Phase | Decision | Classification | Principle | Rationale | Rejected |
|---|-------|----------|----------------|-----------|-----------|----------|
| AD1 | CEO-intake | 跳过 /office-hours 前置 | Mechanical | P6 | PRD（含审查决议）+ 架构报告已充当 design doc，问题定义清晰 | 运行 office-hours |
| AD2 | CEO-intake | UI scope=NO，DX scope=YES | Mechanical | P3 | UI 仅 QueueView 状态标签文案变化；DX 命中 13 词且 CI/注入面/handoff 是本计划的开发者接口 | — |
| AD3 | CEO-0C-bis | Slice A 选方案 A（worker+CAS+事件），拒绝 B（最小修补）与 C（job queue） | Mechanical | P1,P5 | B 无法满足 Slice 6 隐藏窗口硬前置（治标不治本）；C 对桌面单用户是过度工程 | B / C |
| AD4 | CEO-0D | E2 结构化错误 {kind,message} 不提前，保持 Slice 4 | Mechanical | P3 | 与 destination 错误分类同刀处理，提前会双改同一面 | 提前到 A |
| AD5 | CEO-0D | E3 PRAGMA user_version 不提前，保持 Slice 4 | Mechanical | P3 | Slice 4 加表时一并切换，避免独立迁移刀 | 提前到 A |
| AD6 | CEO-0D | E4 worker 日志约定纳入 Slice A scope | Mechanical | P2 | 爆炸半径内（worker.rs 单文件）<1d；修复"事件 emit 失败仅静默"的可见性缺口 | 推迟 |
| AD7 | CEO-0D | E5 CI 不加 Windows job | Mechanical | P3 | release.yml 已在 tag 时覆盖 Windows；PR 级双 runner 时长翻倍不划算 | 纳入 C0 |
| AD8 | CEO-0D | E6 不引入前端测试框架（vitest） | Mechanical | P2,P4 | 新基建超出爆炸半径；Slice A 前端 delta 小（监听+删轮询），冒烟覆盖 | 纳入 A |
| AD9 | CEO-0E | run_capture_task 语义：保留命令但内部走 claim；提交流不再 invoke 它 | **Taste→gate** | P5 | 消除双执行者语义；保留手动兜底面；解决计划中"二选一"留白 | 删除命令 / 保留直跑双轨 |
| AD10 | CEO-learnings | 跳过 cross-project learnings 询问 | Mechanical | P3 | LEARNINGS=0，询问无信息增益 | — |
| AD11 | CEO-voices | R1: C0 扩为 CI + landing protocol + Slice 1-3 先落盘；废除"不 commit"禁令改为"不合 main" | Mechanical | P1 | 双模型共识：CI 触发在 push，旧协议禁 push，防线接不上执行流；dirty tree 30+ 文件是最大交付风险 | 保持 CI-only 叙事 |
| AD12 | CEO-voices | R2: Slice 6 前移到 A→B 之后、Slice 4 之前（Notion-only） | **Taste→gate** | P2,P6 | 双模型共识：PRD P2-4 判定 6 独立；shortcut 是 First Proof Point 与唯一差异化时刻；初版线性化无依赖论证 | 保持 4→4.5→5→6 原序 |
| AD13 | CEO-voices | R3: Slice A 加幂等守卫（先写 notion_page_id 再置 Synced），显式声明残留 crash window | Mechanical **[superseded by AD22]** | P1 | "CAS 单独消灭 F1b"是过度承诺；create_page 非幂等，重试即重复 page。ENG 阶段发现此守卫与选取谓词冲突，升级为 finalization 状态机 | 完全去重（查询 Notion）过度工程 |
| AD14 | CEO-voices | R4: Slice B 改为路由规则数据化（Rust 单源 + 前端数据驱动匹配），废弃注释同步 fixture 方案 | Mechanical | P4,P5 | 原方案验收不可证伪（Rust 测不了 TS 且无 vitest）；注释同步正是 F2 抱怨的漂移机制 | 注释 fixture / 引入 vitest |
| AD15 | CEO-voices | R5: 修正 1.2 节因果表述——"runner 与创建者耦合"替代"UI 开着才会跑" | Mechanical | P5 | 隐藏窗口不杀 JS 定时器；结论对但论据不精确，实证滑坡损害整体可信度 | — |
| AD16 | CEO-voices | R6: 显式接受 F1a 存续到 A 落地（不出 hotfix），附带一周失效条款 | **Taste→gate** | P3 | A 天级可达且 hotfix 与 worker 改同一段逻辑；但这是"接受在产 bug 存续"的决策，须过 gate | hotfix 先行 |
| AD17 | CEO-voices | R7: Slice 7 前补 doctor JSON 变形降级测试；风险清单补 Agent-Reach 单点依赖 | Mechanical | P1 | source 侧整体押在上游 CLI，一份 fixture 固化的形状假设无变形用例 | — |
| AD18 | CEO-voices | R8: CI runner 理由改为"发布环境对齐"；验收补 3 次 re-run flake 检查 | Mechanical | P5 | 原 webkit 依赖理由站不住；本地沙盒已出过 mock listener 被拒案例，runner 兼容性必须实证 | Linux runner（放弃：与发布目标不一致） |
| AD19 | CEO-voices | R9: codex review fallback 从"可选附带"升格为 C0 硬验收项 | Mechanical | P1 | 9+ 次 Claude gate Blocked，codex 就是主防线组成部分，权重错配 | — |
| AD20 | CEO-voices | R10: Slice 4 加结构约束——destination adapter 独立模块，worker.rs 禁止增长 adapter 代码 | Mechanical | P5 | 防 worker.rs 成为下一个 1456 行 God-file；App.tsx 教训（PRD P1-4）刚兑现 | — |
| AD21 | CEO-voices | Codex 产品级质疑（wedge 收窄/矩阵瘦身/webhook 降级/metrics）记录为 2.6 待议，不越权改 PRD | **User Challenge→gate** | P6 | 超出架构计划边界；PRD 切片存废是 Yaocheng 的产品决策 | 直接砍 Slice 5/7 |
