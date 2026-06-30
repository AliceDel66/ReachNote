# ReachNote MVP PRD + 信息架构讨论稿

日期：2026-06-30

## 立项结论

建议将项目定位为：

> ReachNote 是一款 Mac 端 AI 信息采集与 Notion 知识沉淀工具。用户在浏览 GitHub、网页、视频、社区内容时，一键收集链接，由本地 Agent 获取内容、调用 AI 总结分析，并自动写入用户绑定的 Notion 数据库，形成可持续更新的个人研究库。

项目边界：

- Agent-Reach 负责“看见互联网”：渠道选型、安装、体检、路由和内容读取。
- ReachNote 负责“沉淀到个人 Notion 知识库”：捕获、分析、结构化、同步、状态管理。
- 首版不做知识库产品、不做 Notion 写作工具、不做完整 RAG 系统。

核心链路：

```text
Capture -> Read -> Analyze -> Structure -> Sync to Notion
```

## 产品定位

产品名称：`ReachNote`

英文副标题：

> AI-powered web capture for Notion

中文介绍：

> ReachNote 是一款 Mac 桌面端 AI 信息采集工具。它基于 Agent-Reach 的跨平台读取能力，帮助用户从 GitHub、网页、视频和 RSS 中提取有效内容，自动总结、分类、打标签，并同步到个人 Notion 知识库。

首版不应定位成“又一个收藏夹”，而应定位成：

> 把互联网内容变成可复用的 Notion 研究资产。

## 目标用户

MVP 先服务开发者 / AI 工具研究者。

原因：

- GitHub repo、技术博客、YouTube 教程、RSS 更新是高频需求。
- 这些来源相对稳定，登录态和平台风控风险较低。
- 输出结构天然适合 Notion database：项目分析、技术栈、价值判断、是否跟进。

后续用户可以扩展到：

- 产品经理 / 创业者：竞品、市场反馈、用户痛点。
- 内容创作者 / 研究型个人用户：文章、视频、播客、社区帖子。
- 投研 / 行业观察者：RSS、GitHub trending、社媒关键词日报或周报。

## 核心问题

ReachNote 解决三个断点：

1. 用户看到好内容，但收藏后不再整理。
2. AI 能总结内容，但缺少稳定的跨平台读取能力。
3. Notion 是长期知识库，但手动录入、分类、打标签太重。

## MVP Job To Be Done

> 当我看到一个值得研究的链接时，我希望不用手动整理，只需快捷键保存，它就能自动生成一张结构化 Notion 研究卡，方便以后检索、比较和跟进。

## MVP 成功标准

1. 用户可以在 2 分钟内完成 Notion 绑定和默认 database 设置。
2. 用户可以用菜单栏、剪贴板 URL 或快捷键捕获链接。
3. GitHub repo、普通网页、YouTube 字幕、RSS 文章可以稳定进入处理队列。
4. 每条内容生成结构化摘要、标签、价值评分和下一步建议。
5. 成功写入 Notion；失败时本地可见、可重试、可复制错误。

## MVP 范围

### 应该做

- Mac 菜单栏常驻。
- Notion OAuth 绑定。
- 选择或创建 Notion database。
- URL 捕获：剪贴板、手动输入、快捷键。
- Agent-Reach 读取内容。
- AI 总结、分析、打标签。
- Notion 写入。
- 本地任务队列、历史记录、失败重试。
- 模板化输出。
- 简单设置：AI Key、默认模板、默认 database。

### 暂时不做

- Twitter/X、小红书、Reddit、B站深度采集。
- 社交平台大规模采集。
- 团队协作知识库。
- 自建云端内容数据库。
- 复杂 RAG 搜索系统。
- 完整浏览器自动化。
- Notion 双向同步。
- 复杂模板编辑器。

## 首版支持平台

P0 支持：

- GitHub repo
- 普通网页
- YouTube 字幕
- RSS / 文章链接

暂不把小红书、Twitter/X、Reddit 作为首版主路径，因为它们涉及登录态、Cookie、风控和账号风险。即使 Agent-Reach 支持这些方向，产品首版也应先用稳定内容源打磨体验。

## 主链路

```text
Capture URL
-> Detect Source Type
-> Read via Agent-Reach
-> Clean Content
-> Analyze with AI Template
-> Map to Notion Fields
-> Sync to Notion
-> Show Local Status
```

关键判断：

- 核心不是“能抓很多平台”，而是每条内容都能稳定变成一张可用的 Notion 研究卡。
- 首版要优先打磨 `GitHub/网页/YouTube -> AI 模板 -> Notion database` 的闭环。

## 信息架构

```text
ReachNote
├─ Menu Bar
│  ├─ Capture Current URL
│  ├─ Capture Clipboard URL
│  ├─ Paste URL Manually
│  ├─ Processing Queue
│  └─ Open History
├─ Inbox / History
│  ├─ All Captures
│  ├─ Processing
│  ├─ Synced
│  ├─ Failed
│  └─ Retry / Open in Notion
├─ Capture Detail
│  ├─ Source Preview
│  ├─ Read Status
│  ├─ AI Output
│  ├─ Notion Mapping
│  └─ Error / Retry
├─ Templates
│  ├─ GitHub Project Analysis
│  ├─ Article Reading Note
│  ├─ Video Note
│  └─ RSS Brief
└─ Settings
   ├─ Notion Connection
   ├─ Database Mapping
   ├─ AI Provider / Model
   ├─ Agent-Reach Doctor
   └─ Privacy / Local Storage
```

首屏建议直接展示 `History / Queue`，不要做 dashboard。用户最关心的是刚捕获的链接处理到哪里、有没有成功进入 Notion。

## 关键页面

### 1. Onboarding

目标：让用户完成第一次可用闭环。

流程：

1. 连接 Notion。
2. 选择或创建 database。
3. 配置 AI Key。
4. 运行 Agent-Reach doctor。
5. 完成一次测试 capture。

### 2. Menu Bar Capture

能力：

- 显示当前剪贴板 URL。
- 捕获剪贴板 URL。
- 手动粘贴 URL。
- 展示最近 3 条任务。
- 快速打开历史记录。
- 快速打开 Notion。

### 3. History / Queue

能力：

- 展示所有 capture 记录。
- 按状态筛选：Processing、Synced、Failed。
- 每条记录展示标题、来源、状态、时间。
- 支持 retry、open URL、open Notion page。
- 失败原因必须人能看懂。

### 4. Capture Detail

能力：

- 展示原链接。
- 展示读取结果。
- 展示 AI 结构化输出。
- 展示 Notion 字段映射。
- 展示写入结果。
- 支持手动重试。

## Notion 数据库结构

默认 database 名称建议：

> ReachNote Research Inbox

字段：

- `Title`
- `URL`
- `Source Type`：GitHub / Article / Video / RSS / Social
- `Summary`
- `Key Points`
- `Tags`
- `Status`
- `Score`
- `Captured At`
- `Synced At`
- `AI Model`
- `Template`
- `Raw Content`
- `Next Action`

`Status` 首版建议只保留：

```text
Inbox / Reviewing / Follow-up / Archived
```

不要首版就设计太多状态，否则用户会把它当任务管理器。

## AI 模板

### GitHub 项目分析

输出：

- 项目定位
- 核心功能
- 技术栈
- 适用场景
- 亮点
- 风险
- 是否值得跟进

### 文章精读

输出：

- 一句话摘要
- 关键观点
- 证据
- 可复用结论
- 相关标签

### 视频笔记

输出：

- 主题
- 章节摘要
- 关键观点
- 行动项
- 适合谁看

### RSS Brief

输出：

- 更新摘要
- 为什么值得看
- 归类标签
- 是否需要后续阅读

## 优先级

### P0

- Notion OAuth
- Database 选择 / 创建
- 手动 URL / 剪贴板 URL 捕获
- GitHub / 网页 / YouTube / RSS 读取
- AI 结构化输出
- Notion 写入
- 本地任务状态
- 失败重试

### P1

- 全局快捷键
- 当前浏览器 URL 捕获
- 模板选择
- Agent-Reach doctor 可视化
- 简单批量处理

### P2

- RSS 定时监控
- GitHub repo watch
- 周报生成
- 自定义 Notion 字段映射
- 社交平台登录态渠道

## 商业化方向

### 免费版

- 每月有限条捕获。
- 支持 GitHub / 网页 / YouTube。
- 用户自带 AI Key 或使用本地模型。

### Pro 版

- 无限捕获。
- 自定义 Notion 模板。
- 批量处理。
- 定时 RSS / GitHub 监控。
- 多模型总结。
- 周报 / 月报自动生成。

### 团队版

团队版后置：

- 共享 Notion workspace。
- 统一标签体系。
- 团队情报流。
- 竞品 / 技术雷达。

## 主要风险

### 平台读取稳定性

Agent-Reach 的价值正是处理渠道读取问题，但社交平台仍有登录态、Cookie、封号和风控风险。首版应优先稳定源。

### Notion 写入权限体验

OAuth 授权、page picker、database 选择和字段映射必须顺滑，否则用户第一步就会流失。

### AI 输出质量

单纯摘要不够有差异化。应提供固定模板：`GitHub 项目分析`、`文章精读`、`竞品分析`、`视频笔记`、`行动项提取`。

## 下一刀

建议切 `MVP 用户流程 + 首版界面草图`。

理由是 PRD 边界已经可以收敛，下一处瓶颈是把 `首次绑定 Notion -> 捕获第一条 GitHub repo -> 写入 Notion` 这条最小闭环画清楚，避免后面技术架构围绕过大的信息架构发散。

入口是：

- `Onboarding`
- `Menu Bar Capture`
- `History / Queue`
- `Capture Detail`
