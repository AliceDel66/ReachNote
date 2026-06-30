import {
  BadgeInfo,
  Beaker,
  CheckCircle2,
  ChevronDown,
  Clipboard,
  FileText,
  Github,
  ListChecks,
  Loader2,
  Play,
  Rss,
  Search,
  Settings,
  ShieldCheck,
  Sparkles,
  Star,
  Tag,
  Text,
  CircleAlert
} from "lucide-react";
import { Button } from "@heroui/react";
import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useMemo, useState } from "react";

import brandMark from "../assets/reachnote_brand_assets/png/icon/reachnote-symbol-transparent-64.png";

type NavKey = "capture" | "queue" | "templates" | "settings";
type TaskStatus = "queued" | "reading" | "analyzing" | "syncing" | "synced" | "failed";
type QueueFilter = "all" | "processing" | "done" | "failed";
type QueueLoadState = "loading" | "ready" | "error";

interface Task {
  id: string;
  url: string;
  source_type: string;
  template_id: string;
  status: TaskStatus;
  title: string | null;
  source_domain: string | null;
  score: number | null;
  model: string | null;
  notion_page_id: string | null;
  error_kind: string | null;
  error_message: string | null;
  created_at: string;
  updated_at: string;
  synced_at: string | null;
}

interface QueueRow {
  id: string;
  title: string;
  source: string;
  status: TaskStatus;
  time: string;
  score: number | null;
  model: string;
}

interface TemplateItem {
  title: string;
  description: string;
  icon: "github" | "article" | "video" | "rss";
  chips: string[];
  state?: "planned" | "preview";
}

const NAV_ITEMS: Array<{ key: NavKey; label: string }> = [
  { key: "capture", label: "采集" },
  { key: "queue", label: "队列" },
  { key: "templates", label: "模板" },
  { key: "settings", label: "设置" }
];

const TEMPLATES: TemplateItem[] = [
  {
    title: "GitHub 项目分析",
    description: "分析代码库、架构与生态",
    icon: "github",
    chips: ["摘要", "要点", "标签", "风险"],
    state: "planned"
  },
  {
    title: "文章阅读笔记",
    description: "长文阅读与观点提炼",
    icon: "article",
    chips: ["摘要", "要点", "标签"],
    state: "preview"
  },
  {
    title: "视频笔记",
    description: "提取要点与关键片段",
    icon: "video",
    chips: ["摘要", "要点", "片段"],
    state: "planned"
  },
  {
    title: "RSS 简报",
    description: "聚合与总结多源资讯",
    icon: "rss",
    chips: ["摘要", "要点", "来源"],
    state: "planned"
  }
];

function taskToQueueRow(task: Task): QueueRow {
  return {
    id: task.id,
    title: task.title ?? task.url,
    source: task.source_domain ?? task.source_type,
    status: task.status,
    time: formatTimestamp(task.created_at),
    score: task.score,
    model: task.model ?? "-"
  };
}

function taskMatchesFilter(status: TaskStatus, filter: QueueFilter): boolean {
  if (filter === "all") {
    return true;
  }

  if (filter === "processing") {
    return status === "queued" || status === "reading" || status === "analyzing" || status === "syncing";
  }

  if (filter === "done") {
    return status === "synced";
  }

  return status === "failed";
}

function statusLabel(status: TaskStatus): string {
  const labels: Record<TaskStatus, string> = {
    queued: "排队中",
    reading: "读取中",
    analyzing: "分析中",
    syncing: "同步中",
    synced: "已完成",
    failed: "失败"
  };

  return labels[status];
}

function formatTimestamp(value: string): string {
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

function isValidArticleUrl(value: string): boolean {
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

function readableError(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }

  if (error instanceof Error) {
    return error.message;
  }

  return "未知错误";
}

function App() {
  const [activeNav, setActiveNav] = useState<NavKey>("queue");
  const [queueFilter, setQueueFilter] = useState<QueueFilter>("all");
  const [tasks, setTasks] = useState<Task[]>([]);
  const [queueLoadState, setQueueLoadState] = useState<QueueLoadState>("loading");
  const [queueError, setQueueError] = useState<string | null>(null);
  const [searchOpen, setSearchOpen] = useState(false);
  const [searchTerm, setSearchTerm] = useState("");
  const [url, setUrl] = useState("");
  const [note, setNote] = useState("");
  const [captureError, setCaptureError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const isUrlValid = isValidArticleUrl(url);

  const loadTasks = useCallback(async () => {
    setQueueLoadState("loading");
    try {
      const nextTasks = await invoke<Task[]>("list_capture_tasks");
      setTasks(nextTasks);
      setQueueError(null);
      setQueueLoadState("ready");
    } catch (error) {
      setQueueError(readableError(error));
      setQueueLoadState("error");
    }
  }, []);

  useEffect(() => {
    void loadTasks();
  }, [loadTasks]);

  const queueRows = useMemo(() => tasks.map(taskToQueueRow), [tasks]);

  const visibleRows = useMemo(() => {
    const filteredRows = queueFilter === "all"
      ? queueRows
      : queueRows.filter((item) => taskMatchesFilter(item.status, queueFilter));

    const normalizedSearch = searchTerm.trim().toLowerCase();
    if (!normalizedSearch) {
      return filteredRows;
    }

    return filteredRows.filter((item) => {
      return [item.title, item.source, item.model, item.status].some((value) =>
        value.toLowerCase().includes(normalizedSearch)
      );
    });
  }, [queueFilter, queueRows, searchTerm]);

  const handleSearchClick = () => {
    const shouldOpen = activeNav !== "queue" || !searchOpen;
    setActiveNav("queue");
    setSearchOpen(shouldOpen);
    if (!shouldOpen) {
      setSearchTerm("");
    }
  };

  const handleCaptureSubmit = async () => {
    if (!isUrlValid || isSubmitting) {
      setCaptureError("请输入合法的 http(s) 文章 URL");
      return;
    }

    setIsSubmitting(true);
    setCaptureError(null);
    try {
      await invoke<Task>("create_capture_task", {
        url: url.trim(),
        note: note.trim() ? note : null
      });
      setUrl("");
      setNote("");
      await loadTasks();
      setActiveNav("queue");
    } catch (error) {
      setCaptureError(readableError(error));
    } finally {
      setIsSubmitting(false);
    }
  };

  const handlePasteFromClipboard = async () => {
    try {
      const clipboardText = await navigator.clipboard.readText();
      if (!isValidArticleUrl(clipboardText)) {
        setCaptureError("剪贴板内容不是合法的 http(s) URL");
        return;
      }
      setUrl(clipboardText.trim());
      setCaptureError(null);
    } catch {
      setCaptureError("无法读取剪贴板，请手动粘贴 URL");
    }
  };

  return (
    <main className="app-shell">
      <AppHeader
        activeNav={activeNav}
        onNavChange={setActiveNav}
        onSearchClick={handleSearchClick}
        searchActive={activeNav === "queue" && searchOpen}
      />
      <section className="app-content">
        {activeNav === "queue" && (
          <QueueView
            rows={visibleRows}
            filter={queueFilter}
            onFilterChange={setQueueFilter}
            loadState={queueLoadState}
            error={queueError}
            onRetryLoad={loadTasks}
            searchOpen={searchOpen}
            searchTerm={searchTerm}
            onSearchTermChange={setSearchTerm}
          />
        )}
        {activeNav === "capture" && (
          <CaptureView
            note={note}
            setNote={setNote}
            url={url}
            setUrl={setUrl}
            isUrlValid={isUrlValid}
            isSubmitting={isSubmitting}
            error={captureError}
            onSubmit={handleCaptureSubmit}
            onPasteFromClipboard={handlePasteFromClipboard}
          />
        )}
        {activeNav === "templates" && <TemplatesView />}
        {activeNav === "settings" && <SettingsView />}
      </section>
      {activeNav === "queue" && <StatusBar />}
    </main>
  );
}

interface AppHeaderProps {
  activeNav: NavKey;
  onNavChange: (key: NavKey) => void;
  onSearchClick: () => void;
  searchActive: boolean;
}

function AppHeader({ activeNav, onNavChange, onSearchClick, searchActive }: AppHeaderProps) {
  return (
    <header className="app-header">
      <div className="brand-block">
        <img
          className="brand-mark"
          src={brandMark}
          alt="ReachNote"
        />
        <span className="brand-name">ReachNote</span>
      </div>
      <nav className="top-nav" aria-label="主导航">
        {NAV_ITEMS.map((item) => (
          <button
            key={item.key}
            type="button"
            className={`nav-item ${activeNav === item.key ? "active" : ""}`}
            onClick={() => onNavChange(item.key)}
          >
            {item.label}
          </button>
        ))}
      </nav>
      <div className="header-actions">
        <IconButton label="搜索" active={searchActive} onClick={onSearchClick}>
          <Search size={30} strokeWidth={2.4} />
        </IconButton>
        <IconButton
          label="设置"
          active={activeNav === "settings"}
          onClick={() => onNavChange("settings")}
        >
          <Settings size={28} strokeWidth={2.4} />
        </IconButton>
      </div>
    </header>
  );
}

interface IconButtonProps {
  label: string;
  active?: boolean;
  onClick?: () => void;
  children: React.ReactNode;
}

function IconButton({ label, active, onClick, children }: IconButtonProps) {
  return (
    <button
      type="button"
      className={`icon-button ${active ? "active" : ""}`}
      aria-label={label}
      title={label}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

interface QueueViewProps {
  rows: QueueRow[];
  filter: QueueFilter;
  onFilterChange: (filter: QueueFilter) => void;
  loadState: QueueLoadState;
  error: string | null;
  onRetryLoad: () => void;
  searchOpen: boolean;
  searchTerm: string;
  onSearchTermChange: (term: string) => void;
}

function QueueView({
  rows,
  filter,
  onFilterChange,
  loadState,
  error,
  onRetryLoad,
  searchOpen,
  searchTerm,
  onSearchTermChange
}: QueueViewProps) {
  const filters: Array<{ key: QueueFilter; label: string }> = [
    { key: "all", label: "全部" },
    { key: "processing", label: "处理中" },
    { key: "done", label: "已完成" },
    { key: "failed", label: "失败" }
  ];

  return (
    <div className="queue-screen">
      <div className="filter-row" role="tablist" aria-label="队列状态筛选">
        {filters.map((item) => (
          <button
            key={item.key}
            type="button"
            className={`filter-chip ${filter === item.key ? "active" : ""}`}
            onClick={() => onFilterChange(item.key)}
          >
            {item.label}
          </button>
        ))}
      </div>

      {searchOpen && (
        <label className="queue-search">
          <Search size={17} strokeWidth={2.2} />
          <input
            aria-label="搜索队列"
            autoFocus
            value={searchTerm}
            onChange={(event) => onSearchTermChange(event.currentTarget.value)}
            placeholder="搜索标题、来源或模型"
          />
        </label>
      )}

      <div className="queue-table" role="table" aria-label="研究队列">
        <div className="queue-row table-head" role="row">
          <span>标题</span>
          <span>来源</span>
          <span>状态</span>
          <span>时间</span>
          <span>评分</span>
          <span>模型</span>
        </div>
        {loadState === "ready" && rows.map((item) => (
          <div className="queue-row" role="row" key={item.id}>
            <span className="queue-title">{item.title}</span>
            <span className="source-cell">{item.source}</span>
            <span>
              <StatusPill status={item.status} />
            </span>
            <span className="muted-cell">{item.time}</span>
            <span>
              <Score value={item.score} />
            </span>
            <span className="model-cell">{item.model}</span>
          </div>
        ))}
        {loadState === "loading" && (
          <div className="queue-empty" role="row">
            正在读取本地队列...
          </div>
        )}
        {loadState === "error" && (
          <div className="queue-empty error-row" role="row">
            <span>无法读取本地队列：{error}</span>
            <button type="button" onClick={onRetryLoad}>
              重试
            </button>
          </div>
        )}
        {loadState === "ready" && rows.length === 0 && (
          <div className="queue-empty" role="row">
            {searchTerm.trim() ? "没有匹配的队列记录" : "暂无任务，前往采集页添加第一条文章 URL。"}
          </div>
        )}
      </div>

      <div className="soft-banner">
        <BadgeInfo size={24} />
        <span>本地 SQLite 队列已启用；本阶段仅创建排队任务，AI 与 Notion 后续接入。</span>
      </div>
    </div>
  );
}

function StatusPill({ status }: { status: TaskStatus }) {
  if (taskMatchesFilter(status, "processing")) {
    return (
      <span className="status-pill processing">
        <Loader2 size={18} className="spin" />
        {statusLabel(status)}
      </span>
    );
  }

  if (status === "synced") {
    return (
      <span className="status-pill done">
        <CheckCircle2 size={18} />
        已完成
      </span>
    );
  }

  return (
    <span className="status-pill failed">
      <CircleAlert size={18} />
      失败
    </span>
  );
}

function Score({ value }: { value?: number | null }) {
  if (!value) {
    return <span className="score-empty">-</span>;
  }

  return (
    <span className="stars" aria-label={`${value} 星`}>
      {Array.from({ length: 5 }).map((_, index) => (
        <Star
          key={index}
          size={20}
          fill={index < value ? "currentColor" : "none"}
          strokeWidth={1.8}
          className={index < value ? "filled" : ""}
        />
      ))}
    </span>
  );
}

interface CaptureViewProps {
  url: string;
  note: string;
  setUrl: (url: string) => void;
  setNote: (note: string) => void;
  isUrlValid: boolean;
  isSubmitting: boolean;
  error: string | null;
  onSubmit: () => void;
  onPasteFromClipboard: () => void;
}

function CaptureView({
  url,
  note,
  setUrl,
  setNote,
  isUrlValid,
  isSubmitting,
  error,
  onSubmit,
  onPasteFromClipboard
}: CaptureViewProps) {
  const hasUrl = url.trim().length > 0;

  return (
    <div className="capture-screen">
      <section className="panel capture-panel">
        <h1>采集研究来源</h1>
        <label className="field-label" htmlFor="article-url">
          文章 URL
        </label>
        <div className="input-shell">
          <span className="input-icon">
            <LinkIcon />
          </span>
          <input
            id="article-url"
            aria-label="文章 URL"
            className="input-text"
            value={url}
            onChange={(event) => setUrl(event.currentTarget.value)}
            placeholder="https://example.com/article"
          />
        </div>
        <button
          className="secondary-action"
          type="button"
          onClick={onPasteFromClipboard}
        >
          <Clipboard size={16} />
          从剪贴板粘贴
        </button>
        {hasUrl && !isUrlValid && (
          <p className="field-error">请输入合法的 http(s) 文章 URL。</p>
        )}

        <label className="field-label" htmlFor="capture-note">
          补充说明（可选）
        </label>
        <div className="textarea-wrap">
          <textarea
            id="capture-note"
            aria-label="补充说明"
            className="textarea-shell textarea-text"
            value={note}
            onChange={(event) => setNote(event.currentTarget.value.slice(0, 500))}
            placeholder="添加背景、关注点或研究问题..."
          />
          <span className="counter">{note.length} / 500</span>
        </div>

        <label className="field-label">AI 提供方</label>
        <button className="select-shell" type="button">
          <span>Claude CLI</span>
          <ChevronDown size={20} />
        </button>

        <button
          className="primary-action"
          type="button"
          disabled={!isUrlValid || isSubmitting}
          onClick={onSubmit}
        >
          {isSubmitting ? <Loader2 size={19} className="spin" /> : <Sparkles size={19} />}
          {isSubmitting ? "正在加入队列" : "分析并生成研究卡"}
        </button>
        {error && <p className="field-error submit-error">{error}</p>}

        <div className="notion-card">
          <div className="notion-logo">N</div>
          <div>
            <strong>Notion 同步未连接</strong>
            <p>连接后可将生成的研究卡同步到 Notion 数据库。</p>
          </div>
          <Button className="disabled-connect" isDisabled>
            连接 Notion
          </Button>
        </div>
      </section>

      <section className="panel preview-panel">
        <h2>研究卡预览</h2>
        <PreviewSection icon={<FileText />} label="标题" widths={["58%"]} />
        <PreviewSection icon={<Text />} label="摘要" widths={["76%", "88%", "64%"]} />
        <PreviewSection icon={<ListChecks />} label="关键要点" widths={["87%", "85%", "84%"]} bullets />
        <PreviewSection icon={<Tag />} label="标签" chips />
        <PreviewSection icon={<CheckCircle2 />} label="下一步行动" widths={["86%", "85%"]} bullets />
        <div className="preview-score">
          <Star size={32} />
          <span>评分</span>
          <span className="large-stars">★★★★<span>★</span></span>
        </div>
      </section>
    </div>
  );
}

function LinkIcon() {
  return (
    <svg width="22" height="22" viewBox="0 0 24 24" aria-hidden="true">
      <path
        d="M10.5 13.5a3 3 0 0 0 4.24 0l3.55-3.55a3 3 0 1 0-4.24-4.24l-.72.72"
        fill="none"
        stroke="currentColor"
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth="2"
      />
      <path
        d="M13.5 10.5a3 3 0 0 0-4.24 0l-3.55 3.55a3 3 0 1 0 4.24 4.24l.72-.72"
        fill="none"
        stroke="currentColor"
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth="2"
      />
    </svg>
  );
}

interface PreviewSectionProps {
  icon: React.ReactNode;
  label: string;
  widths?: string[];
  bullets?: boolean;
  chips?: boolean;
}

function PreviewSection({ icon, label, widths = [], bullets, chips }: PreviewSectionProps) {
  return (
    <div className="preview-section">
      <div className="preview-label">
        <span className="preview-icon">{icon}</span>
        <span>{label}</span>
      </div>
      <div className="preview-lines">
        {chips ? (
          <div className="skeleton-chips" aria-hidden="true">
            <span />
            <span />
            <span />
            <span />
          </div>
        ) : (
          widths.map((width, index) => (
            <div className="line-wrap" key={`${label}-${width}-${index}`}>
              {bullets && <i />}
              <span className="skeleton-line" style={{ width }} />
            </div>
          ))
        )}
      </div>
    </div>
  );
}

function TemplatesView() {
  return (
    <div className="templates-screen">
      <div className="page-heading">
        <h1>研究模板</h1>
        <p>当前仅展示模板方向，暂不支持编辑与保存。</p>
      </div>
      <div className="template-grid">
        {TEMPLATES.map((item) => (
          <TemplateCard item={item} key={item.title} />
        ))}
      </div>
      <div className="soft-banner templates-note">
        <BadgeInfo size={24} />
        <span>当前仅展示模板方向，暂不支持编辑与保存。</span>
      </div>
    </div>
  );
}

function TemplateCard({ item }: { item: TemplateItem }) {
  return (
    <article className="template-card">
      <div className={`template-icon ${item.icon}`}>
        {item.icon === "github" && <Github size={54} fill="currentColor" />}
        {item.icon === "article" && <FileText size={56} />}
        {item.icon === "video" && <Play size={46} fill="currentColor" />}
        {item.icon === "rss" && <Rss size={52} />}
      </div>
      <div className="template-copy">
        <h2>{item.title}</h2>
        <p>{item.description}</p>
        <div className="template-tags">
          {item.chips.map((chip) => (
            <span key={chip}>{chip}</span>
          ))}
        </div>
      </div>
      <span className={`template-state ${item.state === "preview" ? "preview" : ""}`}>
        {item.state === "preview" ? "预览" : "计划中"}
      </span>
    </article>
  );
}

function SettingsView() {
  return (
    <div className="settings-screen">
      <h1>偏好设置</h1>
      <section className="settings-card">
        <h2>1. AI 提供方</h2>
        <RadioLine active label="Claude CLI（默认）" />
        <RadioLine label="Codex CLI（计划中）" />
        <RadioLine label="OpenAI-compatible API（计划中）" />
      </section>
      <section className="settings-card">
        <h2>2. Agent-Reach</h2>
        <div className="settings-row">
          <span>命令</span>
          <strong>agent-reach</strong>
        </div>
        <div className="settings-row muted">
          <span>Doctor（计划中）</span>
          <em>计划中</em>
        </div>
      </section>
      <section className="settings-card notion-settings">
        <div>
          <h2>3. Notion 连接</h2>
          <strong>未接入</strong>
          <p>OAuth 授权与数据库映射功能（计划中）</p>
        </div>
        <Button className="disabled-connect" isDisabled>
          连接 Notion
        </Button>
      </section>
      <section className="settings-card privacy-card">
        <h2>4. 隐私与存储</h2>
        <p>本地优先、无中间服务器，凭证未来接入系统钥匙串。</p>
      </section>
      <div className="soft-banner settings-note">
        <BadgeInfo size={24} />
        <span>当前设置项仅作为界面承载，后续将接入真实持久化配置。</span>
      </div>
    </div>
  );
}

function RadioLine({ active, label }: { active?: boolean; label: string }) {
  return (
    <div className="radio-line">
      <span className={`radio-dot ${active ? "active" : ""}`} />
      <span>{label}</span>
    </div>
  );
}

function StatusBar() {
  return (
    <footer className="status-bar">
      <span>
        <ShieldCheck size={26} />
        本地优先
      </span>
      <span>
        <Beaker size={28} />
        Pre-alpha
      </span>
      <span>
        <span className="ai-badge">AI</span>
        Claude CLI
      </span>
    </footer>
  );
}

export default App;
