import type { AiProviderId, NavKey, TemplateId } from "./types";

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

export const TEMPLATE_PRESENTATION: Record<string, { icon: "github" | "article" | "video" | "rss"; chips: string[] }> = {
  github_project: { icon: "github", chips: ["摘要", "要点", "标签", "风险"] },
  web_article: { icon: "article", chips: ["摘要", "要点", "标签"] },
  video_note: { icon: "video", chips: ["摘要", "要点", "片段"] },
  rss_digest: { icon: "rss", chips: ["摘要", "要点", "来源"] },
  platform_discussion: { icon: "article", chips: ["共识", "分歧", "可信度"] }
};

export function providerLabel(providerId: AiProviderId): string {
  return AI_PROVIDERS.find((provider) => provider.id === providerId)?.label ?? "Claude CLI";
}

export function isAiProviderId(value: string | null | undefined): value is AiProviderId {
  return AI_PROVIDERS.some((provider) => provider.id === value);
}
