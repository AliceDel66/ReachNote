import type { AiProviderId, NavKey, TemplateId, TemplateItem } from "./types";

export const NAV_ITEMS: Array<{ key: NavKey; label: string }> = [
  { key: "capture", label: "采集" },
  { key: "queue", label: "队列" },
  { key: "templates", label: "模板" },
  { key: "settings", label: "设置" }
];

export const AI_PROVIDERS: Array<{ id: AiProviderId; label: string; hint: string }> = [
  { id: "claude_cli", label: "Claude CLI", hint: "默认，本地 claude 命令" },
  { id: "codex_cli", label: "Codex CLI", hint: "本地 codex exec 非交互分析" },
  { id: "openai_compatible", label: "OpenAI-compatible API", hint: "使用 REACHNOTE_OPENAI_* 环境变量" }
];

export const STALE_TASK_SECONDS = 300;
export const DEFAULT_TEMPLATE_ID: TemplateId = "web_article";

export const TEMPLATES: TemplateItem[] = [
  {
    id: "github_project",
    title: "GitHub 项目分析",
    description: "分析代码库、架构与生态",
    icon: "github",
    chips: ["摘要", "要点", "标签", "风险"],
    compatibleSourceTypes: ["github_repo", "github"],
    promptProfile: "项目定位、核心能力、技术栈、适用场景、风险、下一步验证",
    state: "preview"
  },
  {
    id: "web_article",
    title: "网页文章笔记",
    description: "长文阅读与观点提炼",
    icon: "article",
    chips: ["摘要", "要点", "标签"],
    compatibleSourceTypes: ["article", "web"],
    promptProfile: "摘要、关键论点、可复用观点、标签、下一步行动",
    state: "preview"
  },
  {
    id: "video_note",
    title: "视频笔记",
    description: "提取主题、章节与关键片段",
    icon: "video",
    chips: ["摘要", "要点", "片段"],
    compatibleSourceTypes: ["youtube_video", "bilibili_video", "podcast"],
    promptProfile: "主题、章节/片段、关键观点、可执行动作",
    state: "preview"
  },
  {
    id: "rss_digest",
    title: "RSS 简报",
    description: "聚合与总结多源资讯",
    icon: "rss",
    chips: ["摘要", "要点", "来源"],
    compatibleSourceTypes: ["rss_feed", "rss_item"],
    promptProfile: "多条来源聚合、趋势、重点链接、待读优先级",
    state: "preview"
  },
  {
    id: "platform_discussion",
    title: "平台讨论摘要",
    description: "总结社交平台讨论共识与分歧",
    icon: "article",
    chips: ["共识", "分歧", "可信度"],
    compatibleSourceTypes: ["twitter", "reddit", "v2ex", "xiaohongshu", "facebook", "instagram", "linkedin"],
    promptProfile: "讨论共识、分歧、代表性观点、可信度提醒",
    state: "preview"
  }
];

export function providerLabel(providerId: AiProviderId): string {
  return AI_PROVIDERS.find((provider) => provider.id === providerId)?.label ?? "Claude CLI";
}

export function isAiProviderId(value: string | null | undefined): value is AiProviderId {
  return AI_PROVIDERS.some((provider) => provider.id === value);
}

export function isTemplateId(value: string | null | undefined): value is TemplateId {
  return TEMPLATES.some((template) => template.id === value);
}

export function normalizeTemplateId(value: string | null | undefined): TemplateId {
  if (value === "article") {
    return DEFAULT_TEMPLATE_ID;
  }

  return isTemplateId(value) ? value : DEFAULT_TEMPLATE_ID;
}

export function templateLabel(templateId: string | null | undefined): string {
  const normalizedTemplateId = normalizeTemplateId(templateId);
  return TEMPLATES.find((template) => template.id === normalizedTemplateId)?.title ?? "网页文章笔记";
}

export function templateForSourcePlatformKey(key: string | null | undefined): TemplateId {
  if (key === "github") {
    return "github_project";
  }

  if (key === "youtube" || key === "bilibili" || key === "xiaoyuzhou") {
    return "video_note";
  }

  if (key === "rss") {
    return "rss_digest";
  }

  if (
    key === "twitter" ||
    key === "reddit" ||
    key === "v2ex" ||
    key === "xiaohongshu" ||
    key === "facebook" ||
    key === "instagram" ||
    key === "linkedin"
  ) {
    return "platform_discussion";
  }

  return DEFAULT_TEMPLATE_ID;
}
