# Integration Progress

Last updated: 2026-07-01

## Current Snapshot

状态：Done / Slice 3 template registry and selection integrated

当前仓库已从 PRD-only 推进到可运行桌面壳，并完成本地队列、provider 失败路径、Agent-Reach web / GitHub repo 正文读取、结构化分析成功路径、Notion 最小同步、队列 in-progress 恢复、重试调度、orphan analyzed 补同步、Slice 1 首启动/settings 地基、Slice 2 平台能力矩阵和 Slice 3 模板注册/选择地基：`App startup -> get_app_settings + get_environment_status(轻量检测 + 最近平台快照) -> onboarding gate or Queue -> Settings/Onboarding run_agent_reach_doctor -> agent-reach doctor --json -> core normalize_doctor_output -> source_capability_snapshots -> Settings matrix / Capture source hint`。原采集链路升级为 `Article URL -> selected/default/recommended templateId -> create_capture_task(provider_id, template_id) -> SQLite tasks(template_id) -> recover_interrupted_tasks -> sync_pending_analyzed_tasks -> run_capture_task -> Reading -> AgentReachWebReader(Jina Reader / GitHub API fallback) -> AnalysisRequest(content_text/content_reader/template_id) -> template registry prompt profile -> Analyzing -> provider adapter -> AnalysisResult JSON validation -> Syncing -> NotionClient -> Synced/Failed -> 队列页显示结果/失败原因/template label`。`Analyzed` 只作为内部过渡/历史补偿状态，不再是用户可见链路会静默停住的终态。后台自动调度器、OS keychain、OAuth、真实按平台路由读取和登录态平台抓取仍未接入。

最新 PRD：

- `plans/prds/20260630-1906-reachnote-mvp-reset.prd.md`

## Verification Status

- 本地队列 runtime 可验证：`pnpm tauri dev` 已启动 dev app。
- 端到端本地队列冒烟：空库 `tasks` 为 0；采集页空 URL / `abc` 时 CTA disabled；合法 URL `https://openai.com/index/gpt-4o` 创建 `queued` 任务；重启 `pnpm tauri dev` 后队列仍显示该任务，DB count 为 1。
- 最小 worker 失败路径冒烟：`REACHNOTE_CLAUDE_CMD=__missing_claude__ pnpm tauri dev` 下采集合法 URL，验证时 SQLite 记录为 `failed/provider_unavailable`，`error_message` 写入 Claude CLI 缺失提示；队列页行内显示失败原因和重试按钮。
- 结构化分析成功路径冒烟：fake Claude CLI 返回合法 JSON；Tauri dev 真实窗口采集后，验证时 SQLite 记录为 `analyzed`，包含 title、score、model、analysis_json；队列 UI 显示 `已分析`、标题、评分和模型。
- Agent-Reach 内容读取冒烟：本地 mock Jina Reader 返回固定正文；fake Claude CLI 检查 prompt 是否包含该正文；Tauri dev 真实窗口验证时记录为 `analyzed/Reader Content OK`，`analysis_json.summary` 为“基于 Agent-Reach web route 读取正文后生成”。
- Provider adapter tests：Claude/Codex fake CLI 和 OpenAI-compatible 本地 mock HTTP 均通过，不触外网、不使用真实 token。
- Notion settings：app data SQLite `notion_settings` 已配置，`sync_capture_task` 从本地 settings 读取 token/database，不再读 `NOTION_TOKEN` / `NOTION_DATABASE_ID` 环境变量。
- 真实 E2E：`AliceDel66/fe-fidelity-kit` 真实数据已通过 GitHub API/README fallback -> real Claude CLI -> real Notion page，输出 `REAL_E2E_PAGE_ID=390c9b0c-3c3c-81d2-b04d-f0cd5b8859bb`。
- 桌面 UI 自动提交 smoke URL：通过 AX fallback；Computer Use 仍 Invalid app。采集页提交非敏感 `example.com` smoke URL 后，验证时该任务完成 `synced`，队列页显示 `已完成` 和 `Notion` 链接，Notion API 读取 page 返回 200 且字段匹配。
- 队列 stale recovery：通过 AX fallback。手动向 app data SQLite 插入超过 300 秒未更新的 `reading` 测试任务，reload 真实 Tauri 窗口后，前端队列加载调用 `recover_interrupted_tasks`，SQLite 行变为 `failed/read_failed`，UI 显示失败原因和 `重试`；测试 row 已删除。
- Orphan analyzed auto-sync：通过 AX fallback。用户截图中的 `OpenCLI` 任务原始 DB 状态为 `analyzed`、`notion_page_id` 为空；修复后 reload 真实 Tauri 窗口，前端调用 `sync_pending_analyzed_tasks`，DB 变为 `synced` 并写入 Notion page id `390c9b0c-3c3c-81b7-8332-e4a8b4413cb6`，队列 UI 显示 `已完成` 和 `Notion`。
- Slice 1 setup/settings：通过本地命令、Tauri dev smoke 和 QA installed smoke。新 SQLite store 单测证明空库 `onboarding_completed=false`；现有 app data 迁移为 `onboarding_completed=1`，默认 provider/template/destination/shortcut 均写入；环境快照包含 Claude CLI、Codex CLI、agent-reach/version。Vite dev server 服务的新源码命中 onboarding/settings 文案与 settings commands。隔离 QA app `com.reachnote.qa` 已通过 Computer Use 验证首启动 onboarding、Settings 空 Notion、provider 跨重启持久化。
- Slice 2 platform matrix：通过本地命令、Tauri dev 启动和 QA installed smoke。core fixture 单测覆盖真实 doctor JSON 15 平台；Tauri fake doctor 测试覆盖 `REACHNOTE_AGENT_REACH_CMD` 注入和 JSON 异常；QA app 首启动自动写入快照，Settings 手动刷新后快照 count 增加，矩阵显示 15 平台和 ready/needs_install/needs_login 三态；Capture 页 YouTube URL 只读提示显示需安装/暂不支持。QA 数据库最终 tasks=0、notion_settings=0，只保留 source capability snapshots。
- Slice 3 template registry：通过本地命令和 QA installed smoke。core 测试证明模板 profile 注入 prompt、旧 `article` alias canonical 为 `web_article`；Tauri tests 证明注册模板 ID 可存 task、未知模板被拒、默认模板可存 settings；QA app 中模板页设默认 GitHub 后 DB `app_settings.default_template_id=github_project`，采集公开 GitHub URL 后 DB `tasks.template_id=github_project`，队列新增模板列并在重启后仍显示 `GitHub 项目分析`。该公开 GitHub URL 任务失败在 reader/network，不影响模板持久化验证。

## Progress Log

### 2026-06-30

- 已按用户要求清空旧实现，当前无可运行端到端链路。
- 新 PRD 明确下一条目标链路：`Article URL -> Local queue -> Claude CLI analysis -> Notion page -> Queue status`。
- Claude CLI PRD 起草尝试超时，采用 Codex fallback。
- 第一刀实现最小可运行静态壳：Tauri 2 + React 18 + HeroUI + Rust core。
- 根据用户反馈完成 UI 尺度修正：默认窗口 `1180x780`，采集页控件和字体改为更紧凑的桌面工具密度。
- 第二刀实现本地队列数据流：core 任务类型和 URL 校验、Tauri SQLite store、创建/列表命令、前端真实 invoke。
- SQLite DB 路径：`~/Library/Application Support/com.reachnote.app/reachnote.db`；仓库内未发现 `reachnote.db`。
- 下一步入口：在现有 `TaskStatus` 和 `tasks` 表基础上实现 worker/Claude CLI provider，把 `Queued` 推进到 `Analyzing` / `Failed`，仍先不接 Notion。
- Review gate：Claude 第一轮 FAIL 中的有效风险点已修复；第二轮 180 秒 timeout 无输出，当前 gate 记录为 Blocked，不能声明 Claude PASS。
- 提交并推送当前基线：`c49c530`，中文 commit `实现本地队列闭环与静态桌面壳`，已推送 `origin/main`。
- 第三刀实现最小本地 worker：新增 `run_capture_task`，前端采集成功后自动触发；worker 只检测 Claude CLI 可执行文件，缺失时写回 `Failed/provider_unavailable/error_message`。
- 本轮不接 Notion、不接 Agent-Reach、不读取网页、不调用 Claude 进程；验证通过的失败路径完全发生在本地 SQLite 和 Tauri command 内。
- Claude review gate：本轮 worker 切片精简 packet 仍 180 秒 timeout，无输出；当前 gate 继续 Blocked，不能声明 Claude PASS。
- 下一步入口：实现 Claude CLI provider 的真实结构化分析成功路径，把 `Analyzing` 推进到可渲染 research card；仍先不接 Notion。

### 2026-07-01

- 准备第 6 步(Notion adapter）测试前置条件，与第 5 步（Claude provider 真实分析）并行，不阻塞。
- 交付物:`plans/handoff/20260701-notion-adapter-prerequisites.md`(用户清单 + schema + 开放决策)、`.env.notion.example`(凭证模板)、`scripts/notion-smoke.sh`(连通性自检)。
- 安全加固:`.gitignore` 新增 secret 段(`.env*` / `*.token` / `config/secrets.*`,放行 `*.example`)。已验证 `.env.notion` 被忽略、模板可提交。**注意 `config/mcporter.json` 已被 git 跟踪,Notion token 不得放 `config/`,只走 `.env.notion`(测试)/ keychain(产品)。**
- Notion API 事实:2025-09-03 起 `database` 是容器、create page 的 `parent` 用 `data_source_id`;`2022-06-28` + `database_id` 对单 data source 的新建 database 仍可用,定为 MVP 起步路径。来源:developers.notion.com/docs/upgrade-guide-2025-09-03。
- **待用户裁决的开放决策(影响 adapter)**:① Score 口径冲突 —— README schema `Number 0-100` vs 现有代码/前端/Claude prompt `1-5 星`,必须统一;② 测试字段范围(最小 6 字段 vs 全 13);③ 认证 internal token(推荐)vs OAuth。
- 验证状态:`notion-smoke.sh` 已端到端跑通(2026-07-01):token 有效、database 已 share、可写入测试 page。详见本日志末尾「Notion 测试前置闭环」。
- 下一步入口:用户完成 6 步准备 + 跑通自检 + 定 3 个决策 → 据此写第 6 步 Notion adapter 的 codex 实现 prompt。
- 第四刀实现结构化分析成功路径：`run_capture_task` 调用 `ProviderRunner`，成功后写 `Analyzed`，并把结构化研究卡保存在 `analysis_json`。
- provider 范围扩展：`claude_cli`、`codex_cli`、`openai_compatible` 已共享 core `AnalysisResult` 契约。Claude/Codex 缺失、超时或执行失败会写标准错误；OpenAI-compatible 使用环境变量配置。
- 本轮明确没有接 Agent-Reach，因此 prompt 要求模型不得声称已读取网页正文，只能根据 URL 和补充说明生成待复核研究卡。
- Claude review gate：本轮结构化分析 provider 切片精简 packet 仍 180 秒 timeout，无输出；当前 gate 继续 Blocked，不能声明 Claude PASS。
- 下一步入口：接 Agent-Reach/内容读取，把 `AnalysisRequest` 从 URL-only 升级为 URL + normalized content，再保留同一 provider 契约。
- 第五刀实现 Agent-Reach 内容读取：`src-tauri/src/reader.rs` 使用 Agent-Reach web route / Jina Reader 读取 URL 正文，`run_capture_task` 先写 `Reading`，读取成功后再进入 `Analyzing`。
- `AnalysisRequest` 已从 URL-only 升级为 URL + normalized content：`content_text` 进入统一 prompt，`content_reader` 标记来源；读取失败直接写 `Failed/read_failed` 或 `network_failed`。
- Tauri dev 冒烟使用本地 mock reader 与 fake Claude CLI，验证正文确实进入 provider prompt：SQLite 最新任务 `task-1782837428-477655000-32681-0` 为 `analyzed`，标题 `Reader Content OK`，model `fake-claude-reader-check`。
- 未闭环风险：当前不持久化全文，仅把本轮读取正文送入 provider；真实网络读取可能受目标站点/Jina Reader 可用性影响；Notion 同步仍未接入。
- 第六刀实现 Notion 本地设置与同步：`store.rs` 新增 `notion_settings`，`src-tauri/src/notion.rs` 改用 `NotionSettings` 构造并新增 `test_connection`，`lib.rs` 注册 `get_notion_settings` / `save_notion_settings` / `test_notion_connection` / `sync_capture_task`。
- `sync_capture_task` 启用 `Syncing/Synced`：同步成功写 `notion_page_id` / `synced_at`，同步失败保留 `analysis_json` 并允许独立重试。
- 真实 GitHub 数据问题：Jina Reader 对 `https://github.com/AliceDel66/fe-fidelity-kit` 返回 HTTP 451 SecurityCompromiseError；为保证 GitHub repo P0 来源真实可用，新增 GitHub API metadata + README raw fallback。
- 真实 E2E 验证：显式 ignored test `real_e2e_fe_fidelity_kit_claude_to_notion` 使用 `.env.notion`、`REACHNOTE_CLAUDE_CMD=/opt/homebrew/bin/claude`、真实 GitHub repo、真实 Notion API 跑通，1 passed，page id `390c9b0c-3c3c-81d2-b04d-f0cd5b8859bb`。
- 本轮 Tauri dev：`pnpm tauri dev` 可启动窗口和 Vite `127.0.0.1:5173`；使用 macOS Accessibility `try click` fallback 后，采集页 URL input 和 CTA 可触发完整同步。Computer Use 仍无法绑定 debug app，因此不能声明 Computer Use PASS。
- 本机队列数据状态：此前清空过 `tasks` 并保留 `notion_settings`；本轮 Notion smoke 重新写入测试任务。验证时该 smoke 任务为 `synced`，带 `notion_page_id` 和 `synced_at`。当前 app data 可能被仍在运行的 dev session 继续改写；前端队列仍通过 `list_capture_tasks` 从 SQLite 读取，当前没有代码级 seed 数据。
- 本轮验证矩阵：`pnpm typecheck`、`pnpm build`、`cargo test -p reachnote-core`、`cargo test --manifest-path src-tauri/Cargo.toml`、`cargo check --manifest-path src-tauri/Cargo.toml`、`git diff --check` 全部通过。
- 第七刀队列恢复与重试调度：`recover_interrupted_tasks` 将 stale `reading/analyzing/syncing` 恢复为 `failed/read_failed`，`retry_capture_task` 统一重试入口；`Queued/Failed(no analysis)` 走读取+分析，`Analyzed/Failed(with analysis)` 走 Notion 同步，`Synced` 幂等返回，active processing 直接返回“仍在处理中”错误。
- 集成验证：Tauri dev 中插入 stale `reading` row 后 reload，恢复链路从前端加载触发、写入 SQLite、队列页展示失败原因和重试按钮全程可见；`cargo test --manifest-path src-tauri/Cargo.toml` 新增覆盖 stale recovery、active retry rejection 和 failed-with-analysis sync retry path。
- Bugfix：修复分析成功后同步依赖前端 promise 导致 reload/HMR/关闭时任务停在 `已分析` 的链路缺口。`run_capture_task` 后端现在自动继续同步，队列加载再补扫历史 `Analyzed` 遗留任务；截图中的 `OpenCLI` 已完成真实 Notion 同步。
- Release CI 修复：`v0.1.0` 首次 tag run `28496282107` 已触发但失败于 `Setup Node`，根因是 workflow 使用 Node 20，而 `pnpm@11.5.0` 需要 Node >= 22.13 并依赖 `node:sqlite`。已将 `.github/workflows/release.yml` 的 `actions/setup-node` 改为 Node 24；本地验证 `node -v` 为 `v24.15.0`、`pnpm -v` 为 `11.5.0`，`git diff --check` 和 `pnpm build` 通过。下一步是提交修复、移动 `v0.1.0` tag 到修复 commit 并重新触发 release workflow。
- Release CI 二次修复：第二次 tag run `28496402431` 已越过 Node setup，但在 `pnpm install --frozen-lockfile` 失败，具体错误为 `[ERR_PNPM_IGNORED_BUILDS] Ignored build scripts: @heroui/shared-utils@2.1.12`。根因是 `pnpm-workspace.yaml` 中 `@heroui/shared-utils` 的 `allowBuilds` 仍是占位值；已改为 `true`，并验证 `pnpm install --frozen-lockfile --force`、`pnpm build`、`git diff --check` 通过。下一步提交修复、再次移动 `v0.1.0` tag 触发 release workflow。
- Release CI 三次修复：第三次 tag run `28496668063` 中 macOS universal build 已通过，Windows build 失败于 `src-tauri/src/lib.rs:490`，错误为 `no variant named Reopen found for enum RunEvent`。根因是 `tauri::RunEvent::Reopen` 只适合作为 macOS Dock reopen 路径，Windows target 下不可用；已将该匹配加 `#[cfg(target_os = "macos")]`，并在非 macOS 分支显式消费 closure 参数。为避免混入当前工作树其它未提交 tray 改动，修复 hunk 在 index 中单独 staged；临时 clean worktree 验证 `pnpm install --frozen-lockfile`、`pnpm build`、`cargo check --manifest-path src-tauri/Cargo.toml` 通过。下一步提交修复、再次移动 `v0.1.0` tag 触发 release workflow。
- Release 发布完成：第四次 tag run `28497116955` 已成功，macOS universal 与 Windows jobs 均为 success。`v0.1.0` 已从 draft 发布为正式 release，URL 为 `https://github.com/AliceDel66/ReachNote/releases/tag/v0.1.0`，打包资产包括 `ReachNote_0.1.0_universal.dmg`、`ReachNote_universal.app.tar.gz`、`ReachNote_0.1.0_x64-setup.exe`、`ReachNote_0.1.0_x64_en-US.msi`。发布 tag 指向 `75b6bf0`。
- Slice 1 集成：启动链路新增 settings/env gate，不破坏原 `Article -> Queue -> Analysis -> Notion` 主链路。既有安装继续直接进入 Queue，新安装由 `app_settings.onboarding_completed=false` 触发首启动引导；provider 选择会持久化并继续传给 `create_capture_task(providerId)`。
- Slice 1 验证矩阵：`pnpm typecheck`、`pnpm build`、`cargo test -p reachnote-core`、`cargo test --manifest-path src-tauri/Cargo.toml`、`cargo check --manifest-path src-tauri/Cargo.toml`、`git diff --check` 均通过。`pnpm tauri dev` 启动通过；用户可见 UI PASS 改走 `ReachNote QA.app`。
- 桌面验证隔离修复：新增 `src-tauri/tauri.qa.conf.json` 和 `scripts/desktop-smoke-qa.sh`，用 `com.reachnote.qa` 隔离 app identity/data。`scripts/desktop-smoke-qa.sh --reset-data` 构建 debug app 后，Computer Use 绑定 `/Users/yaocheng/Desktop/nexus/rearchnote/target/debug/bundle/macos/ReachNote QA.app`，首启动、Settings、空队列和 provider persistence 已验证。正式 app data `com.reachnote.app` 未被 reset。
- Slice 2 集成：`get_environment_status` 不再同步跑 doctor，只读最近平台快照；`run_agent_reach_doctor` 是唯一 doctor 执行入口。当前矩阵只是能力可见化和后续路由地基，未把 `run_capture_task` 改成按 platform action 路由，也未接 OpenCLI/登录态平台真实抓取。
- Slice 3 集成：模板选择已经跨前端、Tauri command、SQLite task、core prompt 和队列展示形成闭环；本阶段仍保持 `source_type=article`，只让模板先驱动 prompt intent 和可见 label。后续 source_type/destination adapter 切片可在此基础上迁移，不需要重做模板选择 UI。

#### Notion 测试前置闭环(2026-07-01)

- 用户提供的 `AIOps` 是内容 page(30 段落/17 代码块/11 列表)、非 database;经用户同意,用 Notion API 在该 page 下创建测试 database **`ReachNote Research Inbox`**,13 字段:`Title`(title)/`URL`/`Source Type`(select)/`Summary`/`Key Points`/`Tags`(multi_select)/`Status`(select)/`Score`(number)/`Captured At`(date)/`Synced At`(date)/`AI Model`/`Template`(select)/`Next Action`。
- `scripts/notion-smoke.sh` 端到端跑通:读 database + 创建测试 page 均 HTTP 200。配置在 `.env.notion`(gitignored,**不进版本库**):`NOTION_DATABASE_ID`=新 db(前缀 `38fc9b0c…`)、`NOTION_VERSION=2022-06-28`、`NOTION_DATA_SOURCE_ID` 留空 → create page 走 `parent.database_id`。token 仅在 `.env.notion`,产品阶段迁移到 OS keychain。
- 脚本修正:create page 的 `parent` 类型改为**只由 `NOTION_VERSION` 决定**(原逻辑「data_source_id 非空即用」会在 `2022-06-28` 下拼出不兼容组合)。
- 第 6 步(Notion adapter)前置就绪。剩 3 个开放决策待裁决后即可写 adapter prompt:① Score 口径 `0-100`(README)vs `1-5`(现有 `Task.score`/前端星级/第四刀 `AnalysisResult`)—— 真实冲突,必须统一;② 测试字段范围(最小 6 vs 全 13);③ 认证 internal token(推荐)vs OAuth。
