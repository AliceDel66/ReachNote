import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useMemo, useState } from "react";

import { CaptureView } from "./capture/CaptureView";
import { AppHeader } from "./components/AppHeader";
import { StatusBar } from "./components/StatusBar";
import { isAiProviderId, providerLabel } from "./constants";
import { OnboardingView } from "./onboarding/OnboardingView";
import { QueueView } from "./queue/QueueView";
import { SettingsView } from "./settings/SettingsView";
import { TemplatesView } from "./templates/TemplatesView";
import type {
  AiProviderId,
  AppSettings,
  EnvironmentStatus,
  NavKey,
  QueueFilter,
  QueueLoadState,
  SourcePlatformStatus,
  Task,
  TemplateId,
  TemplateRegistry
} from "./types";
import {
  isValidArticleUrl,
  mergeTaskList,
  normalizeTemplateId,
  readableError,
  sourcePlatformKeyForUrl,
  taskMatchesFilter,
  templateForSourcePlatformKey,
  templateItemsFromRegistry,
  taskToQueueRow,
  upsertTask
} from "./utils";

type SetupLoadState = "loading" | "ready" | "error";
const EMPTY_TEMPLATE_REGISTRY: TemplateRegistry = {
  templates: [],
  template_aliases: [],
  platform_rules: [],
  platform_template_mappings: []
};

function preferredProvider(settings: AppSettings, environment: EnvironmentStatus): AiProviderId {
  if (isAiProviderId(settings.default_provider_id)) {
    return settings.default_provider_id;
  }

  if (isAiProviderId(environment.recommended_provider_id)) {
    return environment.recommended_provider_id;
  }

  return "claude_cli";
}

function preferredTemplate(settings: AppSettings, templateRegistry: TemplateRegistry): TemplateId {
  return normalizeTemplateId(settings.default_template_id, templateRegistry);
}

function App() {
  const [activeNav, setActiveNav] = useState<NavKey>("queue");
  const [queueFilter, setQueueFilter] = useState<QueueFilter>("all");
  const [tasks, setTasks] = useState<Task[]>([]);
  const [queueLoadState, setQueueLoadState] = useState<QueueLoadState>("loading");
  const [queueError, setQueueError] = useState<string | null>(null);
  const [workerError, setWorkerError] = useState<string | null>(null);
  const [searchOpen, setSearchOpen] = useState(false);
  const [searchTerm, setSearchTerm] = useState("");
  const [url, setUrl] = useState("");
  const [note, setNote] = useState("");
  const [captureError, setCaptureError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [retryingTaskId, setRetryingTaskId] = useState<string | null>(null);
  const [selectedProviderId, setSelectedProviderId] = useState<AiProviderId>("claude_cli");
  const [selectedTemplateId, setSelectedTemplateId] = useState<TemplateId>("web_article");
  const [templateSelectionDirty, setTemplateSelectionDirty] = useState(false);
  const [isTogglingCompact, setIsTogglingCompact] = useState(false);
  const [appSettings, setAppSettings] = useState<AppSettings | null>(null);
  const [environmentStatus, setEnvironmentStatus] = useState<EnvironmentStatus | null>(null);
  const [templateRegistry, setTemplateRegistry] = useState<TemplateRegistry | null>(null);
  const [setupLoadState, setSetupLoadState] = useState<SetupLoadState>("loading");
  const [setupError, setSetupError] = useState<string | null>(null);
  const [isRefreshingEnvironment, setIsRefreshingEnvironment] = useState(false);
  const [isRefreshingSourcePlatforms, setIsRefreshingSourcePlatforms] = useState(false);
  const [sourcePlatformError, setSourcePlatformError] = useState<string | null>(null);
  const [didRunOnboardingDoctor, setDidRunOnboardingDoctor] = useState(false);
  const [isCompletingOnboarding, setIsCompletingOnboarding] = useState(false);
  const isUrlValid = isValidArticleUrl(url);

  const loadSetup = useCallback(async () => {
    setSetupLoadState("loading");
    try {
      const [settings, environment, registry] = await Promise.all([
        invoke<AppSettings>("get_app_settings"),
        invoke<EnvironmentStatus>("get_environment_status"),
        invoke<TemplateRegistry>("list_templates")
      ]);
      setAppSettings(settings);
      setEnvironmentStatus(environment);
      setTemplateRegistry(registry);
      setSelectedProviderId(preferredProvider(settings, environment));
      setSelectedTemplateId(preferredTemplate(settings, registry));
      setSetupError(null);
      setSetupLoadState("ready");
    } catch (error) {
      setSetupError(readableError(error));
      setSetupLoadState("error");
    }
  }, []);

  useEffect(() => {
    void loadSetup();
  }, [loadSetup]);

  const refreshEnvironment = useCallback(async () => {
    setIsRefreshingEnvironment(true);
    try {
      const environment = await invoke<EnvironmentStatus>("get_environment_status");
      setEnvironmentStatus(environment);
      if (!isAiProviderId(appSettings?.default_provider_id) && isAiProviderId(environment.recommended_provider_id)) {
        setSelectedProviderId(environment.recommended_provider_id);
      }
      setSetupError(null);
    } catch (error) {
      setSetupError(readableError(error));
    } finally {
      setIsRefreshingEnvironment(false);
    }
  }, [appSettings?.default_provider_id]);

  const refreshSourcePlatforms = useCallback(async () => {
    setIsRefreshingSourcePlatforms(true);
    try {
      await invoke<SourcePlatformStatus[]>("run_agent_reach_doctor");
      const environment = await invoke<EnvironmentStatus>("get_environment_status");
      setEnvironmentStatus(environment);
      setSourcePlatformError(null);
      setSetupError(null);
    } catch (error) {
      setSourcePlatformError(readableError(error));
    } finally {
      setIsRefreshingSourcePlatforms(false);
    }
  }, []);

  useEffect(() => {
    if (
      setupLoadState !== "ready" ||
      !appSettings ||
      appSettings.onboarding_completed ||
      didRunOnboardingDoctor
    ) {
      return;
    }

    setDidRunOnboardingDoctor(true);
    void refreshSourcePlatforms();
  }, [appSettings, didRunOnboardingDoctor, refreshSourcePlatforms, setupLoadState]);

  const saveProviderSelection = useCallback(async (providerId: AiProviderId) => {
    setSelectedProviderId(providerId);
    try {
      const settings = await invoke<AppSettings>("save_app_settings", {
        defaultProviderId: providerId
      });
      setAppSettings(settings);
      setSetupError(null);
    } catch (error) {
      setSetupError(readableError(error));
    }
  }, []);

  const saveTemplateSelection = useCallback(async (templateId: TemplateId) => {
    setSelectedTemplateId(templateId);
    setTemplateSelectionDirty(true);
    try {
      const settings = await invoke<AppSettings>("save_app_settings", {
        defaultTemplateId: templateId
      });
      setAppSettings(settings);
      setSetupError(null);
    } catch (error) {
      setSetupError(readableError(error));
    }
  }, []);

  const completeOnboarding = useCallback(async (nextNav: NavKey) => {
    setIsCompletingOnboarding(true);
    try {
      const settings = await invoke<AppSettings>("save_app_settings", {
        onboardingCompleted: true,
        defaultProviderId: selectedProviderId,
        defaultTemplateId: selectedTemplateId
      });
      setAppSettings(settings);
      setActiveNav(nextNav);
      setSetupError(null);
    } catch (error) {
      setSetupError(readableError(error));
    } finally {
      setIsCompletingOnboarding(false);
    }
  }, [selectedProviderId, selectedTemplateId]);

  const markNotionAsDefaultDestination = useCallback(async () => {
    try {
      const settings = await invoke<AppSettings>("save_app_settings", {
        defaultDestinationId: "notion"
      });
      setAppSettings(settings);
      setSetupError(null);
    } catch (error) {
      setSetupError(readableError(error));
    }
  }, []);

  const loadTasks = useCallback(async () => {
    setQueueLoadState("loading");
    try {
      const nextTasks = await invoke<Task[]>("list_capture_tasks");
      setTasks((currentTasks) => mergeTaskList(currentTasks, nextTasks));
      setQueueError(null);
      setQueueLoadState("ready");
    } catch (error) {
      setQueueError(readableError(error));
      setQueueLoadState("error");
    }
  }, []);

  const refreshTasks = useCallback(async () => {
    try {
      const nextTasks = await invoke<Task[]>("list_capture_tasks");
      setTasks((currentTasks) => mergeTaskList(currentTasks, nextTasks));
      setQueueError(null);
      setQueueLoadState("ready");
    } catch {
      // 后台刷新失败时保留上一次的成功状态，不打断用户。
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    const taskUnlisten = listen<Task>("task:updated", (event) => {
      setTasks((currentTasks) => upsertTask(currentTasks, event.payload));
      setWorkerError(null);
      setQueueError(null);
      setQueueLoadState("ready");
    });
    const workerErrorUnlisten = listen<string>("worker:error", (event) => {
      setWorkerError(readableError(event.payload));
    });

    void Promise.all([taskUnlisten, workerErrorUnlisten])
      .then(() => {
        if (!cancelled) {
          void loadTasks();
        }
      })
      .catch((error) => {
        if (!cancelled) {
          setQueueError(readableError(error));
          setQueueLoadState("error");
        }
      });

    return () => {
      cancelled = true;
      void taskUnlisten.then((unlisten) => unlisten()).catch(() => undefined);
      void workerErrorUnlisten.then((unlisten) => unlisten()).catch(() => undefined);
    };
  }, [loadTasks]);

  const activeTemplateRegistry = templateRegistry ?? EMPTY_TEMPLATE_REGISTRY;
  const templateItems = useMemo(
    () => templateItemsFromRegistry(activeTemplateRegistry),
    [activeTemplateRegistry]
  );
  const queueRows = useMemo(
    () => tasks.map((task) => taskToQueueRow(task, activeTemplateRegistry)),
    [activeTemplateRegistry, tasks]
  );
  const defaultTemplateId = useMemo(
    () => normalizeTemplateId(appSettings?.default_template_id, activeTemplateRegistry),
    [activeTemplateRegistry, appSettings?.default_template_id]
  );
  const sourcePlatformKey = useMemo(
    () => sourcePlatformKeyForUrl(url, activeTemplateRegistry.platform_rules),
    [activeTemplateRegistry.platform_rules, url]
  );
  const suggestedTemplateId = useMemo(
    () => templateForSourcePlatformKey(sourcePlatformKey, activeTemplateRegistry),
    [activeTemplateRegistry, sourcePlatformKey]
  );

  useEffect(() => {
    const timer = window.setInterval(() => {
      void refreshTasks();
    }, 30_000);
    const handleFocus = () => {
      void refreshTasks();
    };
    window.addEventListener("focus", handleFocus);

    return () => {
      window.clearInterval(timer);
      window.removeEventListener("focus", handleFocus);
    };
  }, [refreshTasks]);

  const hideToMenuBar = useCallback(async () => {
    setIsTogglingCompact(true);
    try {
      await invoke("set_compact_mode", { compact: true });
      setQueueError(null);
    } catch (error) {
      setQueueError(readableError(error));
    } finally {
      setIsTogglingCompact(false);
    }
  }, []);

  const visibleRows = useMemo(() => {
    const filteredRows = queueFilter === "all"
      ? queueRows
      : queueRows.filter((item) => taskMatchesFilter(item.status, queueFilter));

    const normalizedSearch = searchTerm.trim().toLowerCase();
    if (!normalizedSearch) {
      return filteredRows;
    }

    return filteredRows.filter((item) => {
      return [
        item.title,
        item.source,
        item.templateLabel,
        item.model,
        item.status,
        item.errorKind,
        item.errorMessage
      ].some((value) => value && value.toLowerCase().includes(normalizedSearch));
    });
  }, [queueFilter, queueRows, searchTerm]);

  const handleUrlChange = useCallback((nextUrl: string) => {
    setUrl(nextUrl);
    setCaptureError(null);
    if (!templateSelectionDirty) {
      const nextPlatformKey = sourcePlatformKeyForUrl(nextUrl, activeTemplateRegistry.platform_rules);
      setSelectedTemplateId(templateForSourcePlatformKey(nextPlatformKey, activeTemplateRegistry));
    }
  }, [activeTemplateRegistry, templateSelectionDirty]);

  const handleRunTask = useCallback(async (id: string) => {
    setRetryingTaskId(id);
    setTasks((currentTasks) =>
      currentTasks.map((item) =>
        item.id === id && item.status === "queued"
          ? {
              ...item,
              status: "reading",
              error_kind: null,
              error_message: null
            }
          : item
      )
    );
    try {
      const updatedTask = await invoke<Task>("run_capture_task", { id });
      setTasks((currentTasks) => upsertTask(currentTasks, updatedTask));
      setQueueError(null);
    } catch (error) {
      setQueueError(readableError(error));
    } finally {
      setRetryingTaskId(null);
    }
  }, []);

  const handleRetryTask = useCallback(async (id: string) => {
    setRetryingTaskId(id);
    setTasks((currentTasks) =>
      currentTasks.map((item) =>
        item.id === id
          ? {
              ...item,
              status: item.analysis_json ? "analyzed" : "queued",
              error_kind: null,
              error_message: null
            }
          : item
      )
    );

    try {
      const updatedTask = await invoke<Task>("retry_capture_task", { id });
      setTasks((currentTasks) => upsertTask(currentTasks, updatedTask));
      setQueueError(null);
    } catch (error) {
      setQueueError(readableError(error));
    } finally {
      setRetryingTaskId(null);
    }
  }, []);

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
      const createdTask = await invoke<Task>("create_capture_task", {
        url: url.trim(),
        note: note.trim() ? note : null,
        providerId: selectedProviderId,
        templateId: selectedTemplateId
      });
      setUrl("");
      setNote("");
      setTasks((currentTasks) => upsertTask(currentTasks, createdTask));
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
      const nextUrl = clipboardText.trim();
      setUrl(nextUrl);
      if (!templateSelectionDirty) {
        const nextPlatformKey = sourcePlatformKeyForUrl(nextUrl, activeTemplateRegistry.platform_rules);
        setSelectedTemplateId(templateForSourcePlatformKey(nextPlatformKey, activeTemplateRegistry));
      }
      setCaptureError(null);
    } catch {
      setCaptureError("无法读取剪贴板，请手动粘贴 URL");
    }
  };

  if (setupLoadState !== "ready" || !appSettings || !environmentStatus || !templateRegistry) {
    return (
      <LoadingScreen
        state={setupLoadState}
        error={setupError}
        onRetry={() => void loadSetup()}
      />
    );
  }

  if (!appSettings.onboarding_completed) {
    return (
      <OnboardingView
        environmentStatus={environmentStatus}
        selectedProviderId={selectedProviderId}
        onProviderChange={(providerId) => void saveProviderSelection(providerId)}
        onRefreshEnvironment={() => void refreshEnvironment()}
        onRefreshSourcePlatforms={() => void refreshSourcePlatforms()}
        isRefreshingEnvironment={isRefreshingEnvironment}
        isRefreshingSourcePlatforms={isRefreshingSourcePlatforms}
        onComplete={() => void completeOnboarding("queue")}
        onOpenSettings={() => void completeOnboarding("settings")}
        isCompleting={isCompletingOnboarding}
        error={setupError ?? sourcePlatformError}
      />
    );
  }

  return (
    <main className="app-shell">
      <AppHeader
        activeNav={activeNav}
        onNavChange={setActiveNav}
        onSearchClick={handleSearchClick}
        searchActive={activeNav === "queue" && searchOpen}
        onShrink={() => void hideToMenuBar()}
        shrinkDisabled={isTogglingCompact}
      />
      <section className="app-content">
        {(setupError || workerError) && (
          <div className="app-error-banner">
            {setupError ?? workerError}
          </div>
        )}
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
            onRunTask={handleRunTask}
            onRetryTask={handleRetryTask}
            retryingTaskId={retryingTaskId}
          />
        )}
        {activeNav === "capture" && (
          <CaptureView
            note={note}
            setNote={setNote}
            url={url}
            setUrl={handleUrlChange}
            isUrlValid={isUrlValid}
            isSubmitting={isSubmitting}
            error={captureError}
            selectedProviderId={selectedProviderId}
            selectedTemplateId={selectedTemplateId}
            suggestedTemplateId={suggestedTemplateId}
            templates={templateItems}
            onProviderChange={(providerId) => void saveProviderSelection(providerId)}
            onTemplateChange={(templateId) => void saveTemplateSelection(templateId)}
            onSubmit={handleCaptureSubmit}
            onPasteFromClipboard={handlePasteFromClipboard}
            onOpenSettings={() => setActiveNav("settings")}
            sourcePlatformKey={sourcePlatformKey}
            sourcePlatforms={environmentStatus.source_platforms}
            sourcePlatformsChecked={environmentStatus.source_platforms_checked}
          />
        )}
        {activeNav === "templates" && (
          <TemplatesView
            defaultTemplateId={defaultTemplateId}
            selectedTemplateId={selectedTemplateId}
            templates={templateItems}
            onTemplateChange={(templateId) => void saveTemplateSelection(templateId)}
          />
        )}
        {activeNav === "settings" && (
          <SettingsView
            appSettings={appSettings}
            environmentStatus={environmentStatus}
            selectedProviderId={selectedProviderId}
            onProviderChange={(providerId) => void saveProviderSelection(providerId)}
            onRefreshEnvironment={() => void refreshEnvironment()}
            onRefreshSourcePlatforms={() => void refreshSourcePlatforms()}
            onDestinationConfigured={() => void markNotionAsDefaultDestination()}
            isRefreshingEnvironment={isRefreshingEnvironment}
            isRefreshingSourcePlatforms={isRefreshingSourcePlatforms}
            sourcePlatformError={sourcePlatformError}
          />
        )}
      </section>
      {activeNav === "queue" && <StatusBar providerLabel={providerLabel(selectedProviderId)} />}
    </main>
  );
}

function LoadingScreen({
  state,
  error,
  onRetry
}: {
  state: SetupLoadState;
  error: string | null;
  onRetry: () => void;
}) {
  return (
    <main className="loading-screen">
      <div className="loading-card">
        <strong>ReachNote</strong>
        {state === "error" ? (
          <>
            <p>启动检查失败：{error}</p>
            <button className="secondary-action" type="button" onClick={onRetry}>
              重试
            </button>
          </>
        ) : (
          <p>正在读取本地设置和环境检测结果...</p>
        )}
      </div>
    </main>
  );
}

export default App;
