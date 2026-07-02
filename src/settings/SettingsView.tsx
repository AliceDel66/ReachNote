import { invoke } from "@tauri-apps/api/core";
import { BadgeInfo, Loader2, RefreshCw } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";

import { AI_PROVIDERS } from "../constants";
import type {
  AiProviderId,
  AppSettings,
  EnvironmentStatus,
  NotionSettingsView,
  PlatformAction,
  PlatformAvailability,
  QueueLoadState
} from "../types";
import { formatTimestamp, readableError } from "../utils";

interface SettingsViewProps {
  appSettings: AppSettings | null;
  environmentStatus: EnvironmentStatus | null;
  selectedProviderId: AiProviderId;
  onProviderChange: (providerId: AiProviderId) => void;
  onRefreshEnvironment: () => void;
  onRefreshSourcePlatforms: () => void;
  onDestinationConfigured: () => void;
  isRefreshingEnvironment: boolean;
  isRefreshingSourcePlatforms: boolean;
  sourcePlatformError: string | null;
}

export function SettingsView({
  appSettings,
  environmentStatus,
  selectedProviderId,
  onProviderChange,
  onRefreshEnvironment,
  onRefreshSourcePlatforms,
  onDestinationConfigured,
  isRefreshingEnvironment,
  isRefreshingSourcePlatforms,
  sourcePlatformError
}: SettingsViewProps) {
  const providerStatusById = useMemo(() => {
    return new Map(environmentStatus?.ai_providers.map((provider) => [provider.id, provider]));
  }, [environmentStatus]);

  return (
    <div className="settings-screen">
      <div className="settings-heading">
        <h1>偏好设置</h1>
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
      <section className="settings-card">
        <h2>1. AI 提供方</h2>
        {AI_PROVIDERS.map((provider) => {
          const status = providerStatusById.get(provider.id);
          return (
            <RadioLine
              active={selectedProviderId === provider.id}
              hint={status ? status.detail : provider.hint}
              key={provider.id}
              label={provider.label}
              ready={status?.ready}
              recommended={status?.is_recommended}
              onClick={() => onProviderChange(provider.id)}
            />
          );
        })}
      </section>
      <section className="settings-card">
        <div className="card-title-row settings-card-title-row">
          <h2>2. Agent-Reach</h2>
          <button
            className="secondary-action compact-action"
            type="button"
            disabled={isRefreshingSourcePlatforms}
            onClick={onRefreshSourcePlatforms}
          >
            <RefreshCw size={15} className={isRefreshingSourcePlatforms ? "spin" : ""} />
            刷新平台
          </button>
        </div>
        <div className="settings-row">
          <span>安装状态</span>
          <strong>{environmentStatus?.agent_reach.installed ? "已检测到" : "未检测到"}</strong>
        </div>
        <div className="settings-row muted">
          <span>版本</span>
          <em>{environmentStatus?.agent_reach.version ?? "未返回版本"}</em>
        </div>
        <p className="settings-card-note">
          {environmentStatus?.agent_reach.detail ?? "首次启动时会检测本地 agent-reach CLI。"}
        </p>
        <PlatformCapabilityMatrix
          environmentStatus={environmentStatus}
          error={sourcePlatformError}
          isRefreshing={isRefreshingSourcePlatforms}
        />
      </section>
      <NotionSettingsCard onConfigured={onDestinationConfigured} />
      <section className="settings-card privacy-card">
        <h2>4. 快捷键与隐私</h2>
        <div className="settings-row">
          <span>采集快捷键</span>
          <strong>{appSettings?.global_shortcut ?? "CommandOrControl+Shift+R"}</strong>
        </div>
        <div className="settings-row muted">
          <span>状态</span>
          <em>{appSettings?.global_shortcut_enabled ? "已启用" : "待启用"}</em>
        </div>
        <p>本地优先、无中间服务器；Notion Token 当前仍保存在本地数据库，钥匙串托管后续接入。</p>
      </section>
      <div className="soft-banner settings-note">
        <BadgeInfo size={24} />
        <span>AI provider、默认模板和默认目标会写入本地 app_settings；采集页会沿用这里选择的 provider。</span>
      </div>
    </div>
  );
}

interface PlatformCapabilityMatrixProps {
  environmentStatus: EnvironmentStatus | null;
  error: string | null;
  isRefreshing: boolean;
}

function PlatformCapabilityMatrix({
  environmentStatus,
  error,
  isRefreshing
}: PlatformCapabilityMatrixProps) {
  const platforms = environmentStatus?.source_platforms ?? [];
  const hasSnapshot = Boolean(environmentStatus?.source_platforms_checked);
  const snapshotError = environmentStatus?.source_platforms_error;
  const updatedAt = environmentStatus?.source_platforms_updated_at;

  return (
    <div className="platform-matrix">
      <div className="platform-matrix-header">
        <div>
          <strong>Agent-Reach 平台能力</strong>
          <span>
            {updatedAt
              ? `上次检测 ${formatTimestamp(updatedAt)}`
              : hasSnapshot
                ? "已检测但无可用平台快照"
                : "尚未检测"}
          </span>
        </div>
        <em>{platforms.length ? `${platforms.length} 平台` : "等待刷新"}</em>
      </div>

      {isRefreshing && platforms.length === 0 && (
        <p className="platform-state-row">正在运行 agent-reach doctor --json...</p>
      )}
      {error && <p className="platform-error-row">{error}</p>}
      {snapshotError && <p className="platform-error-row">{snapshotError}</p>}
      {!isRefreshing && !hasSnapshot && !error && (
        <p className="platform-state-row">尚未检测平台能力。点击「刷新平台」后会保存最近一次成功快照。</p>
      )}

      {platforms.length > 0 && (
        <div className="platform-table">
          <div className="platform-row platform-head">
            <span>平台</span>
            <span>可用性</span>
            <span>Backend</span>
            <span>动作</span>
            <span>建议</span>
          </div>
          {platforms.map((platform) => (
            <div className="platform-row" key={platform.key}>
              <span className="platform-name">
                <strong>{platform.name}</strong>
                <small>{platform.key}</small>
              </span>
              <span className={`platform-pill ${availabilityClass(platform.availability)}`}>
                {availabilityLabel(platform.availability)}
              </span>
              <span className="platform-backend">{platform.active_backend ?? "—"}</span>
              <span className="platform-action">{actionLabel(platform.action)}</span>
              <span className="platform-message" title={platform.message}>
                {platform.summary || platform.message || "—"}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function availabilityLabel(availability: PlatformAvailability): string {
  const labels: Record<PlatformAvailability, string> = {
    ready: "ready",
    needs_install: "需安装",
    needs_login: "需登录",
    needs_config: "需配置",
    blocked: "受阻",
    unknown: "未知"
  };

  return labels[availability];
}

function availabilityClass(availability: PlatformAvailability): string {
  if (availability === "ready") {
    return "ready";
  }

  if (availability === "blocked") {
    return "blocked";
  }

  if (availability === "unknown") {
    return "unknown";
  }

  return "warn";
}

function actionLabel(action: PlatformAction): string {
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

interface NotionSettingsCardProps {
  onConfigured: () => void;
}

function NotionSettingsCard({ onConfigured }: NotionSettingsCardProps) {
  const [loadState, setLoadState] = useState<QueueLoadState>("loading");
  const [configured, setConfigured] = useState(false);
  const [tokenPreview, setTokenPreview] = useState<string | null>(null);
  const [tokenInput, setTokenInput] = useState("");
  const [databaseId, setDatabaseId] = useState("");
  const [isSaving, setIsSaving] = useState(false);
  const [isTesting, setIsTesting] = useState(false);
  const [message, setMessage] = useState<{ kind: "success" | "error"; text: string } | null>(null);

  const applySettings = (settings: NotionSettingsView) => {
    setConfigured(settings.configured);
    setTokenPreview(settings.token_preview);
    setDatabaseId(settings.database_id);
  };

  const loadSettings = useCallback(async () => {
    setLoadState("loading");
    try {
      const settings = await invoke<NotionSettingsView>("get_notion_settings");
      applySettings(settings);
      setLoadState("ready");
    } catch (error) {
      setMessage({ kind: "error", text: readableError(error) });
      setLoadState("error");
    }
  }, []);

  useEffect(() => {
    void loadSettings();
  }, [loadSettings]);

  const handleSave = async () => {
    const trimmedDatabaseId = databaseId.trim();
    if (!trimmedDatabaseId) {
      setMessage({ kind: "error", text: "请填写 Notion Database ID" });
      return;
    }

    setIsSaving(true);
    setMessage(null);
    try {
      const settings = await invoke<NotionSettingsView>("save_notion_settings", {
        token: tokenInput.trim() ? tokenInput.trim() : null,
        databaseId: trimmedDatabaseId
      });
      applySettings(settings);
      setTokenInput("");
      setMessage({ kind: "success", text: "已保存到本地配置" });
      onConfigured();
    } catch (error) {
      setMessage({ kind: "error", text: readableError(error) });
    } finally {
      setIsSaving(false);
    }
  };

  const handleTestConnection = async () => {
    setIsTesting(true);
    setMessage(null);
    try {
      const result = await invoke<string>("test_notion_connection");
      setMessage({ kind: "success", text: result });
    } catch (error) {
      setMessage({ kind: "error", text: readableError(error) });
    } finally {
      setIsTesting(false);
    }
  };

  return (
    <section className="settings-card notion-settings-card">
      <h2>3. Notion 连接</h2>
      {loadState === "loading" && <p className="settings-row muted">正在读取本地配置...</p>}
      {loadState !== "loading" && (
        <>
          <label className="field-label" htmlFor="notion-token">
            Integration Token
          </label>
          <div className="input-shell">
            <input
              id="notion-token"
              aria-label="Notion Integration Token"
              className="input-text"
              type="password"
              autoComplete="new-password"
              data-1p-ignore="true"
              data-lpignore="true"
              name="reachnote-notion-token-manual"
              value={tokenInput}
              onChange={(event) => setTokenInput(event.currentTarget.value)}
              placeholder={configured ? `已保存 ${tokenPreview ?? ""}，留空则不修改` : "ntn_..."}
            />
          </div>

          <label className="field-label" htmlFor="notion-database-id">
            Database ID
          </label>
          <div className="input-shell">
            <input
              id="notion-database-id"
              aria-label="Notion Database ID"
              className="input-text"
              autoComplete="off"
              data-1p-ignore="true"
              data-lpignore="true"
              name="reachnote-notion-database-id-manual"
              value={databaseId}
              onChange={(event) => setDatabaseId(event.currentTarget.value)}
              placeholder="32 位十六进制 database id"
            />
          </div>

          <div className="notion-settings-actions">
            <button
              className="secondary-action"
              type="button"
              disabled={isTesting || !configured}
              onClick={handleTestConnection}
            >
              {isTesting && <Loader2 size={14} className="spin" />}
              {isTesting ? "测试中..." : "测试连接"}
            </button>
            <button
              className="notion-save-action"
              type="button"
              disabled={isSaving || !databaseId.trim()}
              onClick={handleSave}
            >
              {isSaving && <Loader2 size={14} className="spin" />}
              {isSaving ? "保存中..." : "保存"}
            </button>
          </div>

          {message && (
            <p className={message.kind === "success" ? "field-success" : "field-error"}>
              {message.text}
            </p>
          )}
          {!configured && !message && (
            <p className="settings-row muted">尚未配置，保存后才能同步到 Notion，也才能测试连接。</p>
          )}
        </>
      )}
    </section>
  );
}

function RadioLine({
  active,
  hint,
  label,
  ready,
  recommended,
  onClick
}: {
  active?: boolean;
  hint: string;
  label: string;
  ready?: boolean;
  recommended?: boolean;
  onClick: () => void;
}) {
  return (
    <button className="radio-line" type="button" onClick={onClick}>
      <span className={`radio-dot ${active ? "active" : ""}`} />
      <span className="radio-copy">
        <strong>{label}</strong>
        <small>{hint}</small>
      </span>
      <span className={`provider-status ${ready ? "ready" : "missing"}`}>
        {recommended ? "推荐" : ready ? "可用" : "缺失"}
      </span>
    </button>
  );
}
