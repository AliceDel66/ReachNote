import { BadgeInfo, CheckCircle2, CircleAlert, Loader2, Search, Star } from "lucide-react";

import type { QueueFilter, QueueLoadState, QueueRow, TaskStatus } from "../types";
import { notionPageUrl, statusLabel, taskMatchesFilter } from "../utils";

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
  onRetryTask: (id: string) => void;
  retryingTaskId: string | null;
}

export function QueueView({
  rows,
  filter,
  onFilterChange,
  loadState,
  error,
  onRetryLoad,
  searchOpen,
  searchTerm,
  onSearchTermChange,
  onRetryTask,
  retryingTaskId
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
          <span>模板</span>
          <span>状态</span>
          <span>时间</span>
          <span>评分</span>
          <span>模型</span>
        </div>
        {loadState === "ready" && rows.map((item) => (
          <div className="queue-row" role="row" key={item.id}>
            <span className="queue-title-cell">
              <span className="queue-title">{item.title}</span>
              {item.errorMessage && (
                <span className="queue-error-message">{item.errorMessage}</span>
              )}
            </span>
            <span className="source-cell">{item.source}</span>
            <span className="template-cell">{item.templateLabel}</span>
            <span>
              <StatusPill status={item.status} />
            </span>
            <span className="muted-cell">{item.time}</span>
            <span>
              <Score value={item.score} />
            </span>
            <span className="model-cell">
              <span>{item.model}</span>
              {item.status === "synced" && item.notionPageId && (
                <a
                  className="row-action"
                  href={notionPageUrl(item.notionPageId)}
                  rel="noreferrer"
                  target="_blank"
                >
                  Notion
                </a>
              )}
              {item.status === "failed" && (
                <button
                  className="row-action"
                  type="button"
                  disabled={retryingTaskId === item.id}
                  onClick={() => onRetryTask(item.id)}
                >
                  {retryingTaskId === item.id ? "重试中" : "重试"}
                </button>
              )}
            </span>
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
        <span>本地 SQLite 队列、Agent-Reach web 读取和 Notion 最小同步已启用；处理中任务超时会恢复为失败并保留可重试入口。</span>
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

  if (status === "analyzed" || status === "synced") {
    return (
      <span className="status-pill done">
        <CheckCircle2 size={18} />
        {statusLabel(status)}
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
