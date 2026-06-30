# Design Source

Last updated: 2026-06-30

## Current Snapshot

状态：Registered / New UI is source of truth

用户在 2026-06-30 更新了 ReachNote UI。新版 UI 覆盖旧版菜单栏 popover 方向，PRD 和后续实现必须以新版四张图为核心。

## Registered Design Sources

| Date | Source | Covers | Notes |
| --- | --- | --- | --- |
| 2026-06-30 | `assets/ui/ChatGPT Image 2026年6月30日 19_12_36 (2).png` | 队列 / 默认工作台 / 状态筛选 / 任务表 | 队列优先，列为 `标题 / 来源 / 状态 / 时间 / 评分 / 模型`，底部状态栏显示本地优先、Pre-alpha、Claude CLI。 |
| 2026-06-30 | `assets/ui/ChatGPT Image 2026年6月30日 19_12_34 (1).png` | 采集 / 研究卡骨架预览 | 左侧 URL、剪贴板、补充说明、AI provider、CTA、Notion 连接；右侧为研究卡字段骨架。 |
| 2026-06-30 | `assets/ui/ChatGPT Image 2026年6月30日 19_12_36 (3).png` | 模板中心 | GitHub 项目分析、文章阅读笔记、视频笔记、RSS 简报；当前只展示方向，暂不支持编辑保存。 |
| 2026-06-30 | `assets/ui/ChatGPT Image 2026年6月30日 19_12_36 (4).png` | 设置 | AI 提供方、Agent-Reach、Notion 连接、隐私与存储；多项为计划中。 |

## Product UI Decisions

- 默认入口：`队列`，因为第一版最重要的是让用户看到采集任务是否处理成功、失败原因和可重试状态。
- 主导航：`队列 / 采集 / 模板 / 设置`。若单张图中的排序不一致，后续实现以队列优先的信息架构为准。
- 视觉形态：桌面工具窗口，不做 marketing landing page，不回到旧版 menu-bar popover 作为主界面。
- 状态栏：长期显示本地优先、Pre-alpha、当前 AI provider。
- 模板：第一版只提供系统模板展示和默认选择，不做复杂模板编辑器。
- 设置：第一版优先配置 Claude CLI、Agent-Reach doctor、Notion 连接状态；Codex CLI 和 OpenAI-compatible API 可以展示为计划中，除非 PRD 明确提升优先级。

## Open Questions

- 新版 UI 未提供窄屏或 Windows WebView 截图；后续实现需要单独定义响应式和跨平台验收。
- 搜索按钮在新版图中存在，但首版应仅支持队列本地搜索，不做全文检索或跨 Notion 搜索。
- 采集页右侧预览是 skeleton 还是示例内容，需要在实现切片中根据任务状态决定。

## Progress Log

### 2026-06-30

- 新版四张 UI 设计图已登记，并覆盖旧版 17:28 图作为后续实现核心。
- PRD 中已采用队列优先决策：默认入口为 `队列`，主导航为 `队列 / 采集 / 模板 / 设置`。
- P0 上下文校准：UI 源已从旧 `Downloads` 绝对路径改为仓库内 `assets/ui/` 路径；四张图均已确认存在，尺寸均为 `1448x1086`。
