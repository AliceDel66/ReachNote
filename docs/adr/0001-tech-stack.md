# ADR 0001 · 技术栈定板

- 状态：Accepted
- 日期：2026-06-30
- 关联：[`mvp-prd-information-architecture.md`](../discussions/mvp-prd-information-architecture.md)

## 背景

ReachNote 是一个**桌面常驻**的 AI 信息采集工具，需要满足以下硬约束：

1. **跨平台**：首版同时支持 **macOS 与 Windows**（不再只做 Mac）。
2. **常驻后台**：系统托盘 / 菜单栏长期驻留，对**空闲时的内存与电量占用敏感**。
3. **编排多个外部子进程**：内容读取依赖 [Agent-Reach](https://github.com/Panniantong/Agent-Reach)（Python CLI，`agent-reach`），AI 分析可选择 spawn 本地 `claude` / `codex` CLI。
4. **HTTP 客户端**：调用 OpenAI 兼容 API 与 Notion API。
5. **本地持久化**：任务队列、历史记录、失败重试需落盘。
6. **安全存储凭证**：Notion token、API Key 不能明文裸存。
7. **现代化 UI**：指定使用 [HeroUI](https://heroui.com/)（React + Tailwind CSS 组件库）。

约束 7 直接锁定了**前端必须是 React**，约束 1/2/3 则决定了应用外壳的选型。

## 决策

```
应用外壳 ： Tauri 2
前端     ： React 18 + TypeScript + Vite
UI 组件  ： HeroUI + Tailwind CSS
后端核心 ： Rust（reachnote-core crate）
持久化   ： SQLite（rusqlite / sqlx）
凭证存储 ： 操作系统钥匙串（keyring crate）
子进程   ： std::process / tokio::process 调用 agent-reach、claude、codex
打包分发 ： Tauri bundler → .dmg (macOS) / .msi / NSIS (Windows)
```

## 选型对比

| 维度 | **Tauri 2（采用）** | Electron | 原生 Swift/SwiftUI |
| --- | --- | --- | --- |
| 跨平台 Mac+Win | ✅ 一套代码 | ✅ 一套代码 | ❌ 仅 macOS |
| 兼容 HeroUI(React) | ✅ WebView 渲染 | ✅ Chromium | ❌ 无法使用 Web 组件 |
| 常驻空闲占用 | ✅ 系统 WebView，进程轻 | ❌ 每 app 自带 Chromium，内存大 | ✅ 最低 |
| spawn 外部 CLI | ✅ Rust 进程编排强 | ✅ Node child_process | ⚠️ 可行但跨平台无意义 |
| 凭证安全 | ✅ Rust + OS keychain | ⚠️ 需自行加固 | ✅ Keychain |
| 安装包体积 | ✅ 数 MB ~ 十几 MB | ❌ 上百 MB | ✅ 小 |

**为什么不是 Electron**：ReachNote 是常驻托盘应用，约束 2（空闲占用）权重最高。Electron 每个应用打包整套 Chromium，常驻内存通常上百 MB；Tauri 复用系统 WebView（macOS 的 WKWebView、Windows 的 WebView2），常驻进程显著更轻。对"装上就一直挂着"的工具，这是决定性差异。

**为什么不是原生 Swift**：约束 1（跨平台）与约束 7（HeroUI 是 Web 组件）直接排除了纯原生路线——SwiftUI 无法渲染 HeroUI，且不能覆盖 Windows。

**为什么前端锁定 React**：HeroUI 是 React 专属组件库（React Aria + Tailwind + Framer Motion），由产品需求直接给定，因此前端框架不再是开放问题。

## 关键设计：AI Provider 抽象

后端用一个 Rust trait 统一三种 Provider，路由由用户配置决定。核心逻辑放在**不依赖 Tauri 的 `reachnote-core` crate**，以便独立单元测试、快速验证，不被 Tauri 的首次编译拖慢：

```rust
#[async_trait]
pub trait AiProvider {
    async fn analyze(&self, req: &AnalysisRequest) -> Result<AnalysisResult, AiError>;
    fn id(&self) -> &'static str;
}
```

- `ClaudeCli` / `CodexCli`：`tokio::process` spawn 本地 CLI，stdin 喂 prompt，stdout 收 JSON。
- `OpenAiApi`：`reqwest` 调用 `{base_url}/chat/completions`，支持官方 / 代理 / 本地推理（Ollama、LM Studio）。
- 三者都向模型请求**同一套结构化输出**，按 JSON Schema 校验后映射到 Notion 字段。

## 关键设计：内容读取

内容读取**不自己实现**，而是复用 Agent-Reach：Rust 后端 spawn `agent-reach` 子进程读取 URL，`agent-reach doctor` 直接作为 Onboarding 的"渠道体检"。ReachNote 专注于其上层：捕获、AI 分析、结构化、Notion 同步、桌面编排。

## 工程结构

```
rearchnote/
├─ Cargo.toml              # Rust workspace
├─ crates/core/            # reachnote-core：AI Provider / reach / 类型（无 Tauri 依赖，可独立测试）
├─ src-tauri/              # Tauri 应用：command + 托盘 + 持久化
├─ src/                    # React + HeroUI 前端
└─ package.json / vite / tailwind / tsconfig
```

## 后果与风险

- **WebView 渲染差异**：macOS(WKWebView) 与 Windows(WebView2) 存在细微 CSS/字体差异，需双平台回归。对工具型 UI 可接受。
- **Rust 门槛**：核心后端需要 Rust 能力，但换来的是进程编排、性能与安全收益。
- **外部依赖**：`agent-reach`、`claude`、`codex` 都是外部 CLI，必须有 doctor 体检与友好的缺失提示（首启动引导安装）。
- **Windows 环境**：`agent-reach` 是 Python CLI，Windows 上需确保 Python 运行时与 PATH，doctor 要覆盖此场景。
