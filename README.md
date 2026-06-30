<h1 align="center">ReachNote</h1>

<p align="center"><b>AI-powered web capture for Notion</b></p>
<p align="center">把互联网内容变成可复用的 Notion 研究资产</p>

<p align="center">
  <a href="https://github.com/AliceDel66/ReachNote/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-green.svg" alt="License: MIT"></a>
  <img src="https://img.shields.io/badge/platform-macOS-black.svg" alt="Platform: macOS">
  <img src="https://img.shields.io/badge/status-early%20development-orange.svg" alt="Status: Early Development">
  <img src="https://img.shields.io/badge/PRs-welcome-blueviolet.svg" alt="PRs Welcome">
  <a href="https://github.com/AliceDel66/ReachNote/stargazers"><img src="https://img.shields.io/github/stars/AliceDel66/ReachNote?style=social" alt="GitHub stars"></a>
</p>

> ReachNote 是一款 **Mac 桌面端 AI 信息采集工具**。你在浏览 GitHub、网页、视频、RSS 时一键收藏链接，本地 Agent 自动读取内容、调用 AI 总结分析，并写入你绑定的 Notion 数据库，形成一个可持续更新、可检索、可比较的个人研究库。

> [!NOTE]
> **项目状态：早期开发中（Pre-Alpha）。** 产品定位、信息架构与技术方案已收敛，核心代码正在开发。本仓库当前以公开 README 为主，内部讨论稿暂不上传；欢迎通过 Issue / Discussion 参与讨论。下文中标注「拟定 / 设计中」的部分代表目标形态，尚未完全落地。

---

## 目录

- [ReachNote 是什么](#reachnote-是什么)
- [核心特性](#核心特性)
- [核心链路](#核心链路)
- [系统架构](#系统架构)
- [AI 分析：三种 Provider](#ai-分析三种-provider)
- [捕获方式](#捕获方式)
- [任务生命周期](#任务生命周期)
- [快速开始](#快速开始)
- [配置说明](#配置说明)
- [Notion 数据库结构](#notion-数据库结构)
- [内置 AI 模板](#内置-ai-模板)
- [信息架构](#信息架构)
- [路线图](#路线图)
- [隐私与数据](#隐私与数据)
- [技术选型（待定）](#技术选型待定)
- [参与贡献](#参与贡献)
- [License](#license)

---

## ReachNote 是什么

收藏一时爽，整理火葬场。ReachNote 想解决信息消费链路上的三个断点：

1. **看到好内容，但收藏后再也不整理。** 收藏夹变成数字垃圾场。
2. **AI 能总结内容，但缺少稳定的跨平台读取能力。** 复制粘贴正文太累，很多页面还读不到。
3. **Notion 是理想的长期知识库，但手动录入、分类、打标签太重。**

ReachNote 把这条链路自动化。它要做的不是「又一个收藏夹」，而是：

> **当我看到一个值得研究的链接时，我不用手动整理 —— 只需一个快捷键，它就自动生成一张结构化的 Notion 研究卡，方便我以后检索、比较和跟进。**

首版聚焦**开发者 / AI 工具研究者**：GitHub 仓库、技术博客、YouTube 教程、RSS 更新是高频且稳定的内容源，输出天然适合落进 Notion database（项目分析、技术栈、价值判断、是否跟进）。

---

## 核心特性

| 特性 | 说明 |
| --- | --- |
| 🖱️ **一键捕获** | 菜单栏常驻，支持剪贴板 URL、手动粘贴、全局快捷键（规划中） |
| 🌐 **多源读取** | 通过 Agent-Reach 读取 GitHub / 普通网页 / YouTube 字幕 / RSS，正文提取 + 去噪 |
| 🤖 **AI 模板化分析** | 不只是摘要，而是按内容类型输出结构化字段：定位、技术栈、关键观点、价值评分、下一步建议 |
| 🔌 **三种 AI Provider** | 本地 **Claude CLI** / 本地 **Codex CLI** / 任意 **OpenAI 兼容 API**，自带算力、自带 Key |
| 🗂️ **直写 Notion** | OAuth 授权，自动映射字段并写入你选定的 database |
| 🔁 **本地队列与重试** | 任务持久化在本地，失败原因人类可读、可一键重试 |
| 🔐 **本地优先 / 隐私友好** | 内容只流经「你的机器 → 你选的 AI → 你的 Notion」，无中间服务器 |
| 🆓 **开源 / BYOK** | MIT 协议，无云端账号、无订阅，自带 Key 即用 |

---

## 核心链路

ReachNote 的全部价值，是把每一个链接稳定地变成一张可用的 Notion 研究卡：

```mermaid
flowchart LR
    classDef io fill:#374151,stroke:#d1d5db,stroke-width:2px,color:#fff
    classDef capture fill:#1e40af,stroke:#bfdbfe,stroke-width:2px,color:#fff
    classDef read fill:#0f766e,stroke:#99f6e4,stroke-width:2px,color:#fff
    classDef ai fill:#5b21b6,stroke:#ddd6fe,stroke-width:2px,color:#fff
    classDef sync fill:#047857,stroke:#a7f3d0,stroke-width:2px,color:#fff

    URL((URL)):::io
    URL --> Capture(["1 · Capture<br/>剪贴板 / 快捷键 / 手动"]):::capture
    Capture --> Detect(["2 · Detect Source<br/>GitHub · 网页 · 视频 · RSS"]):::capture
    Detect --> Read(["3 · Read<br/>Agent-Reach 读取"]):::read
    Read --> Clean(["4 · Clean<br/>正文提取 · 去噪"]):::read
    Clean --> Analyze(["5 · Analyze<br/>AI 模板结构化"]):::ai
    Analyze --> Map(["6 · Map<br/>映射 Notion 字段"]):::sync
    Map --> Sync(["7 · Sync<br/>写入 Notion"]):::sync
    Sync --> Done((Done)):::sync
```

> 设计原则：核心不是「能抓多少平台」，而是 `GitHub / 网页 / YouTube → AI 模板 → Notion database` 这条闭环每一条都稳。

---

## 系统架构

ReachNote 是一个**本地优先**的桌面应用，分四层：

```mermaid
flowchart TB
    classDef ui fill:#1e40af,stroke:#bfdbfe,stroke-width:2px,color:#fff
    classDef core fill:#5b21b6,stroke:#ddd6fe,stroke-width:2px,color:#fff
    classDef cap fill:#0f766e,stroke:#99f6e4,stroke-width:2px,color:#fff
    classDef ext fill:#374151,stroke:#d1d5db,stroke-width:2px,color:#fff

    subgraph UI["桌面层 · macOS App（用户交互）"]
        direction LR
        Menu(["菜单栏 Capture"]):::ui ~~~ History(["History / Queue"]):::ui ~~~ Detail(["Capture Detail"]):::ui ~~~ Settings(["Settings"]):::ui
    end

    subgraph Core["核心引擎 · Local Engine（本地编排）"]
        direction LR
        CaptureSvc(["Capture Service"]):::core --> Queue(["Task Queue<br/>持久化 · 重试"]):::core --> Tmpl(["Template Engine"]):::core --> Mapper(["Notion Mapper"]):::core
    end

    subgraph Cap["能力层 · Capabilities（外部对接）"]
        direction LR
        Reach(["Agent-Reach<br/>内容读取"]):::cap ~~~ AIRouter(["AI Provider<br/>路由"]):::cap ~~~ NotionSync(["Notion Sync"]):::cap
    end

    subgraph Ext["外部服务 · External"]
        direction LR
        Sites(["GitHub · 网页<br/>YouTube · RSS"]):::ext ~~~ AIBackends(["Claude CLI · Codex CLI<br/>OpenAI 兼容 API"]):::ext ~~~ NotionDB[("Notion Database")]:::ext
    end

    UI ==> Core ==> Cap ==> Ext

    style UI fill:none,stroke:#3b82f6,stroke-width:2px,stroke-dasharray:5 5,color:#3b82f6
    style Core fill:none,stroke:#8b5cf6,stroke-width:2px,stroke-dasharray:5 5,color:#8b5cf6
    style Cap fill:none,stroke:#14b8a6,stroke-width:2px,stroke-dasharray:5 5,color:#14b8a6
    style Ext fill:none,stroke:#9ca3af,stroke-width:2px,stroke-dasharray:5 5,color:#9ca3af
```

- **桌面层**：菜单栏捕获入口、History / Queue 列表、Capture Detail 详情、Settings。首屏直接展示 History / Queue，让你随时看到「刚收的链接处理到哪了」。
- **核心引擎**：本地任务编排。捕获服务接单 → 任务队列持久化与重试 → 模板引擎组装 prompt → Notion 映射器把 AI 输出对齐到 database 字段。
- **能力层**：三个对外适配器 —— Agent-Reach（读内容）、AI Provider 路由（做分析）、Notion Sync（写卡片）。
- **外部服务**：内容源、AI 后端、你的 Notion 数据库，全部由你掌控。

---

## AI 分析：三种 Provider

这是 ReachNote 的核心设计之一：**自带算力（Bring Your Own Compute）**。AI 分析不绑定任何单一厂商，你可以根据隐私、成本、效果自由选择三种接入方式之一：

```mermaid
flowchart TB
    classDef io fill:#374151,stroke:#d1d5db,stroke-width:2px,color:#fff
    classDef router fill:#c2410c,stroke:#fed7aa,stroke-width:2px,color:#fff
    classDef local fill:#5b21b6,stroke:#ddd6fe,stroke-width:2px,color:#fff
    classDef api fill:#1e40af,stroke:#bfdbfe,stroke-width:2px,color:#fff
    classDef out fill:#047857,stroke:#a7f3d0,stroke-width:2px,color:#fff

    Req(["内容正文 + 模板 Prompt"]):::io
    Req --> Router{{"AI Provider 路由<br/>读取用户配置"}}:::router

    Router -->|本地 CLI| Claude(["Claude CLI<br/>claude -p"]):::local
    Router -->|本地 CLI| Codex(["Codex CLI<br/>codex exec"]):::local
    Router -->|HTTP API| OpenAI(["OpenAI 兼容 API<br/>base_url + key + model"]):::api

    Claude --> Parse(["解析 · 校验<br/>JSON Schema"]):::out
    Codex --> Parse
    OpenAI --> Parse

    Parse --> Result(["结构化结果<br/>Summary · Key Points<br/>Tags · Score · Next Action"]):::out
```

| Provider | 调用方式 | 适合场景 | 你需要准备 |
| --- | --- | --- | --- |
| **Claude CLI** | 本地子进程 `claude -p` | 已在用 Claude Code，想复用登录态与额度 | 安装并登录 [Claude Code CLI](https://claude.com/claude-code) |
| **Codex CLI** | 本地子进程 `codex exec` | 已在用 OpenAI Codex CLI | 安装并登录 [Codex CLI](https://github.com/openai/codex) |
| **OpenAI 兼容 API** | HTTP 请求 | 想直连官方 / 兼容服务（如自部署、第三方代理、本地推理） | `base_url` + `api_key` + `model` |

> **为什么支持本地 CLI？** 很多开发者已经装好了 Claude / Codex CLI 并完成登录。ReachNote 直接以子进程方式复用它们，你**无需再单独配置 API Key**，内容也不经过第三方中转。OpenAI 兼容模式则覆盖一切剩余场景 —— 包括指向本地推理服务（如 Ollama、LM Studio 的 OpenAI 兼容端点）实现完全离线。

无论走哪条路，ReachNote 都向模型请求**同一套结构化输出**，并按 JSON Schema 校验，确保最终能稳定映射到 Notion 字段。配置示例见 [配置说明](#配置说明)。

---

## 捕获方式

| 方式 | 说明 | 优先级 |
| --- | --- | --- |
| 📋 剪贴板 URL | 菜单栏识别当前剪贴板里的链接，一键捕获 | P0 |
| ⌨️ 手动粘贴 | 在菜单栏弹窗里粘贴任意 URL | P0 |
| 🔥 全局快捷键 | 任意 App 下按快捷键，捕获当前/剪贴板链接 | P1 |
| 🌍 当前浏览器 URL | 抓取前台浏览器正在浏览的页面 | P1 |

捕获后任务即进入本地队列，菜单栏可看到最近 3 条任务的实时状态。

---

## 任务生命周期

每个捕获任务在本地队列中按下列状态流转。**失败不会静默丢弃**，错误信息人类可读，可一键重试：

```mermaid
flowchart LR
    classDef st fill:#1e40af,stroke:#bfdbfe,stroke-width:2px,color:#fff
    classDef work fill:#5b21b6,stroke:#ddd6fe,stroke-width:2px,color:#fff
    classDef ok fill:#047857,stroke:#a7f3d0,stroke-width:2px,color:#fff
    classDef bad fill:#b91c1c,stroke:#fecaca,stroke-width:2px,color:#fff

    Start((Capture)):::st
    Start --> Queued(["Queued"]):::st
    Queued --> Reading(["Reading"]):::work
    Reading --> Analyzing(["Analyzing"]):::work
    Analyzing --> Syncing(["Syncing"]):::work
    Syncing --> Synced(["Synced OK"]):::ok

    Reading -. 失败 .-> Failed(["Failed<br/>错误可见"]):::bad
    Analyzing -. 失败 .-> Failed
    Syncing -. 失败 .-> Failed
    Failed -. Retry .-> Queued
```

> 注意区分两套状态：上图是 **ReachNote 应用内的任务处理状态**；写入 Notion 后，研究卡本身还有一套**内容生命周期状态**（`Inbox / Reviewing / Follow-up / Archived`），由你在 Notion 里手动推进。

---

## 快速开始

> [!IMPORTANT]
> ReachNote 尚未发布二进制包。以下为**目标使用流程**，安装方式将在首个 Release 后更新。你可以先 Star / Watch 仓库以获取发布通知。

### 环境要求

- macOS（首版仅支持 macOS）
- 一个 Notion 账号（用于授权写入）
- 至少一种 AI Provider：
  - 本地 Claude CLI **或** 本地 Codex CLI（已登录），**或**
  - 一个 OpenAI 兼容 API 的 `base_url` + `api_key`

### 安装（计划中）

```bash
# 方式一：Homebrew（计划中）
brew install --cask reachnote

# 方式二：从源码构建（技术栈确定后补充）
git clone git@github.com:AliceDel66/ReachNote.git
cd ReachNote
# 构建命令待补充
```

### 首次配置（Onboarding）

目标：**2 分钟内完成第一次可用闭环。**

```mermaid
flowchart LR
    classDef step fill:#5b21b6,stroke:#ddd6fe,stroke-width:2px,color:#fff
    classDef done fill:#047857,stroke:#a7f3d0,stroke-width:2px,color:#fff

    A(["1 · 连接 Notion<br/>OAuth 授权"]):::step
    A --> B(["2 · 选择 / 创建<br/>Database"]):::step
    B --> C(["3 · 配置 AI Provider<br/>Claude / Codex / API"]):::step
    C --> D(["4 · Agent-Reach<br/>Doctor 体检"]):::step
    D --> E(["5 · 测试一次 Capture"]):::step
    E --> F((闭环完成)):::done
```

1. **连接 Notion** —— OAuth 授权 ReachNote 访问目标 workspace。
2. **选择或创建 Database** —— 选已有库，或一键创建默认的 `ReachNote Research Inbox`。
3. **配置 AI Provider** —— 三选一，详见下方。
4. **运行 Agent-Reach Doctor** —— 体检读取渠道是否就绪。
5. **测试一次 Capture** —— 收一条 GitHub repo，确认能成功写入 Notion。

---

## 配置说明

> 以下为**设计中的配置形态**（拟定路径 `~/.reachnote/config.toml`），最终以实现为准。多数用户可全程在 Settings 界面完成，无需手写配置。

### AI Provider

```toml
[ai]
# 三选一：claude-cli | codex-cli | openai-api
provider = "claude-cli"

# 方式一：本地 Claude CLI（复用已登录的 Claude Code）
[ai.claude-cli]
command = "claude"        # 可执行文件路径
extra_args = ["-p"]       # 非交互 / print 模式

# 方式二：本地 Codex CLI
[ai.codex-cli]
command = "codex"
extra_args = ["exec"]

# 方式三：任意 OpenAI 兼容 API（官方 / 代理 / 本地推理）
[ai.openai-api]
base_url = "https://api.openai.com/v1"
api_key  = "sk-..."        # 也可用环境变量注入
model    = "gpt-4o-mini"
```

> 指向本地推理（离线）示例：把 `base_url` 改为 `http://localhost:11434/v1`（Ollama）或 `http://localhost:1234/v1`（LM Studio）即可。

### Notion

```toml
[notion]
# 由 OAuth 流程自动写入，无需手填
database_id = "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
default_template = "github-project"
```

### Agent-Reach

```toml
[reach]
# 各内容源的读取策略；首版默认开启稳定源
sources = ["github", "web", "youtube", "rss"]
```

---

## Notion 数据库结构

默认 database：**`ReachNote Research Inbox`**

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `Title` | Title | 内容标题 |
| `URL` | URL | 原始链接 |
| `Source Type` | Select | `GitHub` / `Article` / `Video` / `RSS` / `Social` |
| `Summary` | Text | AI 摘要 |
| `Key Points` | Text | 关键观点 |
| `Tags` | Multi-select | 自动打标 |
| `Status` | Select | `Inbox` / `Reviewing` / `Follow-up` / `Archived` |
| `Score` | Number | 价值评分 |
| `Captured At` | Date | 捕获时间 |
| `Synced At` | Date | 写入时间 |
| `AI Model` | Text | 实际使用的模型 / Provider |
| `Template` | Select | 使用的分析模板 |
| `Raw Content` | Text | 清洗后的原文（便于回溯） |
| `Next Action` | Text | 下一步建议 |

> `Status` 首版刻意只保留 4 个值，避免被当成任务管理器。

---

## 内置 AI 模板

模板决定 AI 输出的结构。首版内置 4 个，按来源自动选择，也可手动指定（P1）：

<table>
<tr><th>模板</th><th>适用来源</th><th>输出字段</th></tr>
<tr>
<td><b>GitHub 项目分析</b></td><td>GitHub repo</td>
<td>项目定位 · 核心功能 · 技术栈 · 适用场景 · 亮点 · 风险 · 是否值得跟进</td>
</tr>
<tr>
<td><b>文章精读</b></td><td>博客 / 网页文章</td>
<td>一句话摘要 · 关键观点 · 证据 · 可复用结论 · 相关标签</td>
</tr>
<tr>
<td><b>视频笔记</b></td><td>YouTube 字幕</td>
<td>主题 · 章节摘要 · 关键观点 · 行动项 · 适合谁看</td>
</tr>
<tr>
<td><b>RSS Brief</b></td><td>RSS / 订阅文章</td>
<td>更新摘要 · 为什么值得看 · 归类标签 · 是否需要后续阅读</td>
</tr>
</table>

---

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
│  ├─ Processing / Synced / Failed
│  └─ Retry · Open URL · Open in Notion
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

---

## 路线图

### P0 — MVP 闭环

- [ ] Notion OAuth 绑定
- [ ] Database 选择 / 创建
- [ ] 手动 URL / 剪贴板 URL 捕获
- [ ] GitHub / 网页 / YouTube / RSS 读取
- [ ] AI 结构化输出（三种 Provider）
- [ ] Notion 写入
- [ ] 本地任务状态与队列
- [ ] 失败重试

### P1 — 体验增强

- [ ] 全局快捷键
- [ ] 当前浏览器 URL 捕获
- [ ] 模板选择
- [ ] Agent-Reach Doctor 可视化
- [ ] 简单批量处理

### P2 — 持续监控与自动化

- [ ] RSS 定时监控
- [ ] GitHub repo watch
- [ ] 周报 / 月报生成
- [ ] 自定义 Notion 字段映射
- [ ] 社交平台登录态渠道（小红书 / Twitter / Reddit 等）

> 首版**刻意不做**：社交平台大规模采集、团队协作知识库、自建云端内容库、完整 RAG 搜索、Notion 双向同步、复杂模板编辑器。原因来自当前 MVP 边界收敛：先打磨稳定内容源到 Notion 的最小闭环。

---

## 隐私与数据

ReachNote 是**本地优先**的开源工具，没有云端账号，也没有 ReachNote 自己的服务器。

- **内容只流经三方：** 你的机器 → 你选择的 AI Provider → 你的 Notion。中间没有第三方中转。
- **自带 Key（BYOK）：** API Key 与 Notion 凭证存在本地，不上传。
- **可完全离线分析：** 选择本地 CLI 或本地推理（Ollama / LM Studio）时，内容不出本机。
- **失败可见：** 任务与错误都落在本地，便于排查，绝不静默丢弃数据。

---

## 技术选型（待定）

> 技术栈尚未最终敲定。核心约束：**菜单栏常驻、能 spawn 本地 CLI 子进程、本地持久化任务队列、未来可扩展到其他平台**。候选方案：

| 方案 | 优点 | 权衡 |
| --- | --- | --- |
| **Tauri**（Rust + Web） | 体积小、子进程与文件系统能力强、跨平台 | 团队需 Rust 经验 |
| **Electron**（Node + Web） | 生态成熟、`child_process` 调 CLI 简单 | 体积偏大 |
| **原生 Swift / SwiftUI** | Mac 原生体验最佳、菜单栏一等公民 | 跨平台扩展成本高 |

欢迎在 [Discussions](https://github.com/AliceDel66/ReachNote/discussions) 一起讨论。

---

## 参与贡献

ReachNote 处于早期阶段，**正是参与塑造它的最佳时机**。

- 💡 有想法 / 用例 / 内容源需求 → 开一个 [Discussion](https://github.com/AliceDel66/ReachNote/discussions)
- 🐛 发现问题 / 设计漏洞 → 提 [Issue](https://github.com/AliceDel66/ReachNote/issues)
- 🔧 想写代码 → 关注路线图 P0，欢迎认领

```bash
git clone git@github.com:AliceDel66/ReachNote.git
```

设计与决策记录暂不随仓库上传，公开讨论请使用 GitHub Discussions。

---

## License

本项目计划以 **[MIT License](LICENSE)** 开源 —— 无商业化计划，自由使用、修改、分发。

---

<p align="center">
  <sub>Built for people who collect more than they read.</sub><br/>
  <sub>ReachNote · <a href="https://github.com/AliceDel66/ReachNote">github.com/AliceDel66/ReachNote</a></sub>
</p>
