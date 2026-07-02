import { DEFAULT_TEMPLATE_ID, TEMPLATE_PRESENTATION } from "./constants";
import type { PlatformRule, QueueFilter, QueueRow, Task, TaskStatus, TemplateId, TemplateItem, TemplateRegistry } from "./types";

export function taskToQueueRow(task: Task, templateRegistry: TemplateRegistry): QueueRow {
  return {
    id: task.id,
    title: task.title ?? task.url,
    source: task.source_domain ?? task.source_type,
    templateLabel: templateLabel(task.template_id, templateRegistry),
    status: task.status,
    time: formatTimestamp(task.created_at),
    score: task.score,
    model: task.model ?? "-",
    errorKind: task.error_kind,
    errorMessage: task.error_message,
    notionPageId: task.notion_page_id
  };
}

export function templateItemsFromRegistry(templateRegistry: TemplateRegistry): TemplateItem[] {
  return templateRegistry.templates.map((template) => {
    const presentation = TEMPLATE_PRESENTATION[template.id] ?? TEMPLATE_PRESENTATION[DEFAULT_TEMPLATE_ID];
    return {
      id: template.id,
      name: template.name,
      description: template.description,
      compatible_source_types: template.compatible_source_types,
      prompt_profile: template.prompt_profile,
      enabled: template.enabled,
      icon: presentation.icon,
      chips: presentation.chips
    };
  });
}

export function isTemplateId(value: string | null | undefined, templateRegistry: TemplateRegistry): value is TemplateId {
  return Boolean(value && templateRegistry.templates.some((template) => template.id === value));
}

export function normalizeTemplateId(
  value: string | null | undefined,
  templateRegistry: TemplateRegistry
): TemplateId {
  if (isTemplateId(value, templateRegistry)) {
    return value;
  }

  const alias = templateRegistry.template_aliases.find((item) => item.alias === value?.trim());
  if (isTemplateId(alias?.template_id, templateRegistry)) {
    return alias.template_id;
  }

  return DEFAULT_TEMPLATE_ID;
}

export function templateLabel(templateId: string | null | undefined, templateRegistry: TemplateRegistry): string {
  const normalizedTemplateId = normalizeTemplateId(templateId, templateRegistry);
  return templateRegistry.templates.find((template) => template.id === normalizedTemplateId)?.name ?? "网页文章笔记";
}

export function templateForSourcePlatformKey(
  key: string | null | undefined,
  templateRegistry: TemplateRegistry
): TemplateId {
  const templateId = templateRegistry.platform_template_mappings.find((mapping) => mapping.platform_key === key)?.template_id;
  return normalizeTemplateId(templateId, templateRegistry);
}

export function upsertTask(tasks: Task[], nextTask: Task): Task[] {
  let replaced = false;
  const nextTasks = tasks.map((task) => {
    if (task.id === nextTask.id) {
      replaced = true;
      return taskIsNewerOrEqual(nextTask, task) ? nextTask : task;
    }

    return task;
  });

  return replaced ? nextTasks : [nextTask, ...nextTasks];
}

export function mergeTaskList(tasks: Task[], nextTasks: Task[]): Task[] {
  const byId = new Map<string, Task>();
  tasks.forEach((task) => byId.set(task.id, task));
  nextTasks.forEach((task) => {
    const current = byId.get(task.id);
    if (!current || taskIsNewerOrEqual(task, current)) {
      byId.set(task.id, task);
    }
  });

  return Array.from(byId.values()).sort((left, right) => {
    const leftCreated = Number(left.created_at);
    const rightCreated = Number(right.created_at);
    if (Number.isFinite(leftCreated) && Number.isFinite(rightCreated) && leftCreated !== rightCreated) {
      return rightCreated - leftCreated;
    }

    return right.id.localeCompare(left.id);
  });
}

export function taskMatchesFilter(status: TaskStatus, filter: QueueFilter): boolean {
  if (filter === "all") {
    return true;
  }

  if (filter === "processing") {
    return status === "queued" || status === "reading" || status === "analyzing" || status === "syncing";
  }

  if (filter === "done") {
    return status === "analyzed" || status === "synced";
  }

  return status === "failed";
}

export function statusLabel(status: TaskStatus): string {
  const labels: Record<TaskStatus, string> = {
    queued: "等待处理",
    reading: "读取中",
    analyzing: "分析中",
    analyzed: "已分析",
    syncing: "同步中",
    synced: "已完成",
    failed: "失败"
  };

  return labels[status];
}

function taskIsNewerOrEqual(nextTask: Task, currentTask: Task): boolean {
  const nextUpdatedAt = Number(nextTask.updated_at);
  const currentUpdatedAt = Number(currentTask.updated_at);
  if (Number.isFinite(nextUpdatedAt) && Number.isFinite(currentUpdatedAt)) {
    return nextUpdatedAt >= currentUpdatedAt;
  }

  return nextTask.updated_at >= currentTask.updated_at;
}

export function formatTimestamp(value: string): string {
  const seconds = Number(value);
  if (!Number.isFinite(seconds)) {
    return value;
  }

  return new Intl.DateTimeFormat("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit"
  }).format(new Date(seconds * 1000));
}

export function isValidArticleUrl(value: string): boolean {
  const trimmed = value.trim();
  if (!trimmed) {
    return false;
  }

  const parts = trimmed.split("://");
  if (parts.length !== 2) {
    return false;
  }

  const [scheme, rest] = parts;
  const lowerScheme = scheme.toLowerCase();
  if (lowerScheme !== "http" && lowerScheme !== "https") {
    return false;
  }

  const authority = rest.split(/[/?#]/)[0];
  const authorityParts = authority.split("@");
  const hostPort = authorityParts[authorityParts.length - 1] ?? "";
  const host = hostPort.split(":")[0].trim().toLowerCase();

  return Boolean(host) && host.includes(".") && !host.includes(" ");
}

export function sourcePlatformKeyForUrl(value: string, platformRules: PlatformRule[]): string | null {
  if (!isValidArticleUrl(value)) {
    return null;
  }

  let parsed: URL;
  try {
    parsed = new URL(value.trim());
  } catch {
    return null;
  }

  const host = parsed.hostname.toLowerCase().replace(/^www\./, "");
  const path = `${parsed.pathname}${parsed.search}${parsed.hash}`.toLowerCase();
  const exactMatch = platformRules.find((rule) => rule.exact_hosts.some((exactHost) => exactHost === host));
  if (exactMatch) {
    return exactMatch.platform_key;
  }

  const suffixMatch = platformRules.find((rule) =>
    rule.host_suffixes.some((suffix) => hostMatchesSuffix(host, suffix))
  );
  if (suffixMatch) {
    return suffixMatch.platform_key;
  }

  const pathMatch = platformRules.find((rule) =>
    rule.path_keywords.some((keyword) => path.includes(keyword.toLowerCase()))
  );
  if (pathMatch) {
    return pathMatch.platform_key;
  }

  return "web";
}

function hostMatchesSuffix(host: string, suffix: string): boolean {
  return host === suffix || host.endsWith(`.${suffix}`);
}

export function sourcePlatformFallbackName(key: string): string {
  const labels: Record<string, string> = {
    github: "GitHub",
    twitter: "Twitter/X",
    youtube: "YouTube",
    reddit: "Reddit",
    facebook: "Facebook",
    instagram: "Instagram",
    bilibili: "B站",
    xiaohongshu: "小红书",
    linkedin: "LinkedIn",
    xiaoyuzhou: "小宇宙",
    v2ex: "V2EX",
    xueqiu: "雪球",
    rss: "RSS",
    exa_search: "全网搜索",
    web: "网页"
  };

  return labels[key] ?? key;
}

export function readableError(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }

  if (error instanceof Error) {
    return error.message;
  }

  return "未知错误";
}

export function notionPageUrl(pageId: string): string {
  return `https://www.notion.so/${pageId.split("-").join("")}`;
}
