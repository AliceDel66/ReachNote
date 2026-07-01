# 第 6 步(Notion Adapter)测试前置条件

> 对应 PRD「Small-Step Rule」**第 6 步:Notion adapter**。这份清单不是 codex 的实现 prompt,而是**你本人**在让 codex 写 adapter 代码**之前**要完成的真实凭证准备 + 验证。
> 生成日期:2026-07-01 · 真源:`plans/prds/20260630-1906-reachnote-mvp-reset.prd.md`、`README.md`「Notion Database Schema」

---

## 为什么需要这份准备

Notion adapter 是第一垂直切片里**唯一无法纯本地验证**的环节——它需要真实的 Notion 凭证和一个真实 database,而这些**只有你能提供**。先把「凭证通不通」独立验证掉,这样 adapter 阶段一旦出问题,你能立刻区分是**凭证问题**还是**代码问题**,而不是两者纠缠在一起调试。

**完成标志**:`bash scripts/notion-smoke.sh` 通过(能读 database + 能写一条测试 page)+ 下方「需要你裁决的 3 个决策」有答案 = 第 6 步前置就绪。

---

## 安全红线(先读)

- 真实 `NOTION_TOKEN` **绝不**写进任何被 git 跟踪的文件,**绝不**贴进聊天 / PR / memory。
- 凭证只放 **`.env.notion`**(已加进 `.gitignore`)。
- 已加固 `.gitignore` 保护 `.env*` / `*.token` / `config/secrets.*`;模板 `.env.notion.example` 例外、可提交。
- ⚠️ 注意:`config/mcporter.json` **已被 git 跟踪**——**不要**把 Notion token 放进 `config/` 下任何文件。token 走 `.env.notion`(测试)/ OS keychain(产品)。

---

## 你需要做的 6 件事

### 1. 建 internal integration,拿 token
打开 https://www.notion.so/my-integrations → **New integration** → 类型选 **Internal** → 关联你的 workspace → 创建后复制 **Internal Integration Secret**(`ntn_` 开头)。
> 本阶段用 **internal token**(BYOK,最简单)。README 里写的 OAuth 是面向**公开分发**的未来形态,测试与个人使用不需要 —— 见下方决策 3。

### 2. 建测试 database,配字段
在 Notion 新建一个 database(可命名 `ReachNote Research Inbox`),按下方 **Schema 规格** 配 property。
> 可以**先配最小 6 字段**跑通闭环,完整 13 字段后续补 —— 见决策 2。

### 3. 把 database share 给 integration  ←最容易漏
打开 database 的**整页**视图 → 右上 **⋯** → **Connections**(或 Add connections）→ 选你第 1 步建的 integration。
> 漏了这步,自检会报 `object_not_found`。

### 4. 拿 Database ID
database 整页 URL:`https://www.notion.so/<workspace>/<DATABASE_ID>?v=...`,中间那段 32 位十六进制就是。

### 5.(可选)拿 Data Source ID
仅当你打算用 `Notion-Version: 2025-09-03`,或该 database 有**多个 data source** 时需要:
Database settings → **Manage Data Sources** → **⋯** → **Copy data source ID**。
> 单 data source 的新建 database 不需要这步,留空即可。

### 6. 填凭证并自检
```bash
cp .env.notion.example .env.notion      # 然后编辑 .env.notion 填入 token + database id
bash scripts/notion-smoke.sh
```

---

## 运行自检:期望与排错

`scripts/notion-smoke.sh` 会:① GET database(验证 token + share,并打印 database 的真实字段供你核对 schema)→ ② POST 一条测试 page(验证能写)。

**成功**输出:`✅ database 可访问` + `✅ 成功创建测试 page` + `🎉 凭证就绪`。然后到 Notion 删掉那条 `ReachNote smoke test` page。

**失败对照**:

| Notion 错误 | 原因 | 解法 |
| --- | --- | --- |
| `unauthorized` | token 错 / 失效 | 重查第 1 步的 secret |
| `object_not_found` | database 没 share,或 ID 错 | 做第 3 步;核对第 4 步 ID |
| `validation_error` | database 含多个 data source,或 title 字段异常 | 改 `NOTION_VERSION=2025-09-03` 并填 `NOTION_DATA_SOURCE_ID`(第 5 步) |

---

## Notion API 版本与 data source(给 codex 的实现约束)

Notion 在 **2025-09-03** 版本引入了**破坏性变更**:`database` 现在是**容器**,可含多个 **data source**(旧称的 "database" = 现在的 "data source")。影响 create page:

- **旧版 `2022-06-28`**:`parent` 用 `{"database_id": "..."}`。**对单 data source 的 database 仍可用** —— 这正是测试场景,所以是**最简起步路径**。
- **新版 `2025-09-03`**:endpoint 仍是 `POST /v1/pages`,但 `parent` 改为 `{"type":"data_source_id","data_source_id":"..."}`。一旦某 database 加了**第二个** data source,旧版 `database_id` 调用会 `validation_error`。

**给 adapter 的建议**:
1. MVP 锁 **`Notion-Version: 2022-06-28` + `parent.database_id`** 起步(README schema 与 `[notion] database_id` config 模型直接可用)。
2. 代码里**预留一个发现步骤**:GET `/v1/databases/{id}` 读 `data_sources[0].id` 并缓存,方便日后切到 `2025-09-03`。自检脚本已演示这条发现逻辑。
3. 错误分类对齐 `task::ErrorKind`:`unauthorized → NotionUnauthorized`、字段类型不匹配 → `SchemaMismatch`、网络 → `NetworkFailed`。

来源:[Notion 官方 Upgrade guide 2025-09-03](https://developers.notion.com/docs/upgrade-guide-2025-09-03)

---

## Schema 规格(测试 database 字段)

来自 `README.md`「Notion Database Schema」(default database `ReachNote Research Inbox`):

| Field | Notion 类型 | 测试最小子集 | 说明 |
| --- | --- | :---: | --- |
| `Title` | Title | ✅ | 内容标题(title property,名称随意,脚本自动探测) |
| `URL` | URL | ✅ | 原始链接 |
| `Source Type` | Select | | `GitHub` / `Article` / `Video` / `RSS` / `Social` |
| `Summary` | Text (rich_text) | ✅ | AI 摘要 |
| `Key Points` | Text | | 关键要点 |
| `Tags` | Multi-select | ✅ | 自动标签 |
| `Status` | Select | ✅ | `Inbox` / `Reviewing` / `Follow-up` / `Archived` |
| `Score` | Number | ✅ | 价值评分 —— **口径见决策 1** |
| `Captured At` | Date | | 采集时间 |
| `Synced At` | Date | | 写入时间 |
| `AI Model` | Text | | 实际使用的 model / provider |
| `Template` | Select | | 使用的分析模板 |
| `Raw Content` | Text | | 清洗后的正文 |
| `Next Action` | Text | | 建议的下一步 |

> 自检脚本只写 Title 即可创建 page(其余 property 可空),所以**最小子集足够先跑通连通性**。

---

## 需要你裁决的 3 个决策(影响 adapter 实现)

### 决策 1 · Score 口径冲突 ⚠️ 真实不一致
- `README` schema:`Score = Number 0-100`
- 现有代码 `Task.score: Option<u8>` + 前端 **5 星** + 上一阶段 Claude provider prompt:**1-5**

二者必须统一,否则映射出错。**建议**:内部 + UI 保持 **1-5**(星级直观),写 Notion 时映射 `score * 20 → 0-100`。或把 Notion 字段也改 1-5。**请定一个。**

### 决策 2 · 测试字段范围
adapter 先写**最小 6 字段**(Title/URL/Summary/Tags/Status/Score)还是**全 13 字段**?**建议**先 6 字段跑通 `Article URL → Notion page` 闭环,再补全 —— 符合 PRD 小步原则。

### 决策 3 · 认证方式
**internal token**(推荐,测试 / 个人 BYOK)vs **OAuth**(README 设想的公开分发形态)。**建议** MVP 用 internal token,OAuth 留到面向多用户分发时再做。

---

## 完成后

`notion-smoke.sh` 通过 + 上面 3 个决策有答案 → 告诉我,我据此写**第 6 步 Notion adapter 的 codex 实现 prompt**(沿用前两份 handoff 的结构:边界、状态机 `Analyzing → Syncing → Synced`、错误分类、keychain 注入、验收命令)。
