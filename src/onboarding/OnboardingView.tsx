import { CheckCircle2, CircleAlert, Loader2, RefreshCw, Settings2, Sparkles } from "lucide-react";

import brandMark from "../../assets/reachnote_brand_assets/png/icon/reachnote-symbol-transparent-64.png";
import { AI_PROVIDERS } from "../constants";
import type { AiProviderId, EnvironmentStatus } from "../types";

interface OnboardingViewProps {
  environmentStatus: EnvironmentStatus | null;
  selectedProviderId: AiProviderId;
  onProviderChange: (providerId: AiProviderId) => void;
  onRefreshEnvironment: () => void;
  onRefreshSourcePlatforms: () => void;
  isRefreshingEnvironment: boolean;
  isRefreshingSourcePlatforms: boolean;
  onComplete: () => void;
  onOpenSettings: () => void;
  isCompleting: boolean;
  error: string | null;
}

export function OnboardingView({
  environmentStatus,
  selectedProviderId,
  onProviderChange,
  onRefreshEnvironment,
  onRefreshSourcePlatforms,
  isRefreshingEnvironment,
  isRefreshingSourcePlatforms,
  onComplete,
  onOpenSettings,
  isCompleting,
  error
}: OnboardingViewProps) {
  return (
    <main className="onboarding-screen">
      <section className="onboarding-shell">
        <div className="onboarding-hero">
          <div className="brand-block onboarding-brand">
            <img className="brand-mark" src={brandMark} alt="ReachNote" />
            <span className="brand-name">ReachNote</span>
          </div>
          <h1>首次启动检查</h1>
          <p>先确认本机 AI provider 和 Agent-Reach 能力，再选择默认沉淀目标。</p>
        </div>

        <div className="onboarding-grid">
          <section className="onboarding-card wide">
            <div className="card-title-row">
              <h2>1. 本机 AI provider</h2>
              <button
                className="secondary-action compact-action"
                type="button"
                disabled={isRefreshingEnvironment}
                onClick={onRefreshEnvironment}
              >
                <RefreshCw size={15} className={isRefreshingEnvironment ? "spin" : ""} />
                重新检测
              </button>
            </div>
            <div className="provider-list">
              {AI_PROVIDERS.map((provider) => {
                const status = environmentStatus?.ai_providers.find((item) => item.id === provider.id);
                return (
                  <button
                    className={`provider-choice ${selectedProviderId === provider.id ? "active" : ""}`}
                    key={provider.id}
                    type="button"
                    onClick={() => onProviderChange(provider.id)}
                  >
                    <span className={`status-dot ${status?.ready ? "ready" : "missing"}`}>
                      {status?.ready ? <CheckCircle2 size={15} /> : <CircleAlert size={15} />}
                    </span>
                    <span className="provider-lines">
                      <strong>{provider.label}</strong>
                      <small>{status?.detail ?? provider.hint}</small>
                    </span>
                    <span className="provider-choice-state">
                      {status?.is_recommended ? "推荐" : selectedProviderId === provider.id ? "已选" : ""}
                    </span>
                  </button>
                );
              })}
            </div>
          </section>

          <section className="onboarding-card">
            <h2>2. Agent-Reach</h2>
            <div className="check-line">
              <span className={`status-dot ${environmentStatus?.agent_reach.installed ? "ready" : "missing"}`}>
                {environmentStatus?.agent_reach.installed ? <CheckCircle2 size={15} /> : <CircleAlert size={15} />}
              </span>
              <div>
                <strong>{environmentStatus?.agent_reach.installed ? "已检测到 CLI" : "未检测到 CLI"}</strong>
                <p>{environmentStatus?.agent_reach.detail ?? "正在等待环境检测结果。"}</p>
              </div>
            </div>
            <button
              className="secondary-action compact-action onboarding-platform-action"
              type="button"
              disabled={isRefreshingSourcePlatforms}
              onClick={onRefreshSourcePlatforms}
            >
              <RefreshCw size={15} className={isRefreshingSourcePlatforms ? "spin" : ""} />
              {isRefreshingSourcePlatforms ? "检测中" : "刷新平台"}
            </button>
            <p className="settings-card-note">
              {environmentStatus?.source_platforms_checked
                ? `已记录 ${environmentStatus.source_platforms.length} 个平台能力。`
                : "首次启动会自动尝试记录平台能力快照，失败不影响进入系统。"}
            </p>
          </section>

          <section className="onboarding-card">
            <h2>3. 默认目标</h2>
            <div className="destination-option">
              <div className="notion-logo">N</div>
              <div>
                <strong>Notion</strong>
                <p>保持当前最小同步路径；飞书、企业微信、钉钉会在后续 slice 通过 destination adapter 接入。</p>
              </div>
            </div>
          </section>
        </div>

        {error && <p className="field-error onboarding-error">{error}</p>}
        <div className="onboarding-actions">
          <button
            className="primary-action onboarding-primary"
            type="button"
            disabled={isCompleting}
            onClick={onComplete}
          >
            {isCompleting ? <Loader2 size={18} className="spin" /> : <Sparkles size={18} />}
            {isCompleting ? "正在进入" : "进入队列"}
          </button>
          <button
            className="secondary-action onboarding-secondary"
            type="button"
            disabled={isCompleting}
            onClick={onOpenSettings}
          >
            <Settings2 size={16} />
            先配置 Notion
          </button>
        </div>
      </section>
    </main>
  );
}
