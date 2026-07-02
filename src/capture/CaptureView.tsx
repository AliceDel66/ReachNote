import {
  CheckCircle2,
  Clipboard,
  FileText,
  Link2,
  ListChecks,
  Loader2,
  Sparkles,
  Star,
  Tag,
  Text
} from "lucide-react";
import { useMemo, type ReactNode } from "react";

import { AI_PROVIDERS } from "../constants";
import type { AiProviderId, PlatformAction, PlatformAvailability, SourcePlatformStatus, TemplateId, TemplateItem } from "../types";
import { sourcePlatformFallbackName } from "../utils";

interface CaptureViewProps {
  url: string;
  note: string;
  setUrl: (url: string) => void;
  setNote: (note: string) => void;
  isUrlValid: boolean;
  isSubmitting: boolean;
  error: string | null;
  selectedProviderId: AiProviderId;
  selectedTemplateId: TemplateId;
  suggestedTemplateId: TemplateId;
  templates: TemplateItem[];
  onProviderChange: (providerId: AiProviderId) => void;
  onTemplateChange: (templateId: TemplateId) => void;
  onSubmit: () => void;
  onPasteFromClipboard: () => void;
  onOpenSettings: () => void;
  sourcePlatformKey: string | null;
  sourcePlatforms: SourcePlatformStatus[];
  sourcePlatformsChecked: boolean;
}

export function CaptureView({
  url,
  note,
  setUrl,
  setNote,
  isUrlValid,
  isSubmitting,
  error,
  selectedProviderId,
  selectedTemplateId,
  suggestedTemplateId,
  templates,
  onProviderChange,
  onTemplateChange,
  onSubmit,
  onPasteFromClipboard,
  onOpenSettings,
  sourcePlatformKey,
  sourcePlatforms,
  sourcePlatformsChecked
}: CaptureViewProps) {
  const hasUrl = url.trim().length > 0;
  const sourceHint = useMemo(() => {
    if (!sourcePlatformKey) {
      return null;
    }

    const platform = sourcePlatforms.find((item) => item.key === sourcePlatformKey);
    return {
      key: sourcePlatformKey,
      name: platform?.name ?? sourcePlatformFallbackName(sourcePlatformKey),
      platform
    };
  }, [sourcePlatformKey, sourcePlatforms]);
  const suggestedTemplateName = templates.find((template) => template.id === suggestedTemplateId)?.name ?? "网页文章笔记";

  return (
    <div className="capture-screen">
      <section className="panel capture-panel">
        <h1>采集研究来源</h1>
        <label className="field-label" htmlFor="article-url">
          文章 URL
        </label>
        <div className="input-shell">
          <span className="input-icon">
            <Link2 size={22} strokeWidth={2} />
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
        {sourceHint && (
          <div className={`source-hint ${sourceHint.platform?.availability === "ready" ? "ready" : "warn"}`}>
            <strong>检测到：{sourceHint.name}</strong>
            <span>
              {sourceHint.platform
                ? `${availabilityHint(sourceHint.platform.availability)} · ${actionHint(sourceHint.platform.action)}`
                : sourcePlatformsChecked
                  ? "最近快照未包含该平台"
                  : "尚未刷新平台能力"}
            </span>
          </div>
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

        <label className="field-label" htmlFor="ai-provider">
          AI 提供方
        </label>
        <select
          id="ai-provider"
          className="select-shell native-select"
          value={selectedProviderId}
          onChange={(event) => onProviderChange(event.currentTarget.value as AiProviderId)}
        >
          {AI_PROVIDERS.map((provider) => (
            <option key={provider.id} value={provider.id}>
              {provider.label}
            </option>
          ))}
        </select>

        <label className="field-label" htmlFor="capture-template">
          研究模板
        </label>
        <select
          id="capture-template"
          className="select-shell native-select"
          value={selectedTemplateId}
          onChange={(event) => onTemplateChange(event.currentTarget.value as TemplateId)}
        >
          {templates.map((template) => (
            <option key={template.id} value={template.id}>
              {template.name}
            </option>
          ))}
        </select>
        <p className="template-suggestion">
          推荐模板：{suggestedTemplateName}
          {selectedTemplateId !== suggestedTemplateId ? "；当前使用手动选择" : ""}
        </p>

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
            <strong>Notion 同步使用本地配置</strong>
            <p>在「设置」页配置 Token 和 Database ID 后，研究卡会自动同步。</p>
          </div>
          <button className="notion-goto-settings" type="button" onClick={onOpenSettings}>
            去设置
          </button>
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

function availabilityHint(availability: PlatformAvailability): string {
  const labels: Record<PlatformAvailability, string> = {
    ready: "可用",
    needs_install: "需安装",
    needs_login: "需登录",
    needs_config: "需配置",
    blocked: "受阻",
    unknown: "未知"
  };

  return labels[availability];
}

function actionHint(action: PlatformAction): string {
  const labels: Record<PlatformAction, string> = {
    capture_url: "可收录",
    read_content: "可读取",
    search: "仅搜索",
    transcribe: "可转写",
    metadata_only: "仅元数据",
    not_supported_yet: "暂不支持"
  };

  return labels[action];
}

interface PreviewSectionProps {
  icon: ReactNode;
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
