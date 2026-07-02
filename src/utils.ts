import { templateLabel } from "./constants";
import type { QueueFilter, QueueRow, Task, TaskStatus } from "./types";

export function taskToQueueRow(task: Task): QueueRow {
  return {
    id: task.id,
    title: task.title ?? task.url,
    source: task.source_domain ?? task.source_type,
    templateLabel: templateLabel(task.template_id),
    status: task.status,
    time: formatTimestamp(task.created_at),
    score: task.score,
    model: task.model ?? "-",
    errorKind: task.error_kind,
    errorMessage: task.error_message,
    notionPageId: task.notion_page_id
  };
}

export function upsertTask(tasks: Task[], nextTask: Task): Task[] {
  let replaced = false;
  const nextTasks = tasks.map((task) => {
    if (task.id === nextTask.id) {
      replaced = true;
      return nextTask;
    }

    return task;
  });

  return replaced ? nextTasks : [nextTask, ...nextTasks];
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
    queued: "排队中",
    reading: "读取中",
    analyzing: "分析中",
    analyzed: "已分析",
    syncing: "同步中",
    synced: "已完成",
    failed: "失败"
  };

  return labels[status];
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

export function sourcePlatformKeyForUrl(value: string): string | null {
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
  if (host === "github.com") {
    return "github";
  }
  if (host === "youtu.be" || host.endsWith("youtube.com")) {
    return "youtube";
  }
  if (host === "twitter.com" || host === "x.com") {
    return "twitter";
  }
  if (host.endsWith("reddit.com")) {
    return "reddit";
  }
  if (host.endsWith("facebook.com") || host === "fb.com") {
    return "facebook";
  }
  if (host.endsWith("instagram.com")) {
    return "instagram";
  }
  if (host.endsWith("bilibili.com") || host === "b23.tv") {
    return "bilibili";
  }
  if (host.endsWith("xiaohongshu.com") || host === "xhslink.com") {
    return "xiaohongshu";
  }
  if (host.endsWith("linkedin.com")) {
    return "linkedin";
  }
  if (host.endsWith("xiaoyuzhoufm.com")) {
    return "xiaoyuzhou";
  }
  if (host === "v2ex.com") {
    return "v2ex";
  }
  if (host.endsWith("xueqiu.com")) {
    return "xueqiu";
  }
  if (parsed.pathname.toLowerCase().includes("rss") || parsed.pathname.toLowerCase().includes("feed")) {
    return "rss";
  }

  return "web";
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
