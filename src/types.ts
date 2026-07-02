export type NavKey = "capture" | "queue" | "templates" | "settings";
export type TaskStatus = "queued" | "reading" | "analyzing" | "analyzed" | "syncing" | "synced" | "failed";
export type QueueFilter = "all" | "processing" | "done" | "failed";
export type QueueLoadState = "loading" | "ready" | "error";
export type AiProviderId = "claude_cli" | "codex_cli" | "openai_compatible";
export type TemplateId = "web_article" | "github_project" | "video_note" | "rss_digest" | "platform_discussion";
export type PlatformAvailability =
  | "ready"
  | "needs_install"
  | "needs_login"
  | "needs_config"
  | "blocked"
  | "unknown";
export type PlatformAction =
  | "capture_url"
  | "read_content"
  | "search"
  | "transcribe"
  | "metadata_only"
  | "not_supported_yet";

export interface Task {
  id: string;
  url: string;
  source_type: string;
  template_id: string;
  status: TaskStatus;
  title: string | null;
  source_domain: string | null;
  score: number | null;
  model: string | null;
  provider_id: AiProviderId;
  note: string | null;
  analysis_json: string | null;
  notion_page_id: string | null;
  error_kind: string | null;
  error_message: string | null;
  created_at: string;
  updated_at: string;
  synced_at: string | null;
}

export interface NotionSettingsView {
  configured: boolean;
  database_id: string;
  token_preview: string | null;
}

export interface QueueRow {
  id: string;
  title: string;
  source: string;
  templateLabel: string;
  status: TaskStatus;
  time: string;
  score: number | null;
  model: string;
  errorKind: string | null;
  errorMessage: string | null;
  notionPageId: string | null;
}

export interface TemplateItem {
  id: TemplateId;
  title: string;
  description: string;
  icon: "github" | "article" | "video" | "rss";
  chips: string[];
  compatibleSourceTypes: string[];
  promptProfile: string;
  state?: "planned" | "preview";
}

export interface AppSettings {
  onboarding_completed: boolean;
  default_provider_id: AiProviderId | null;
  default_template_id: string | null;
  default_destination_id: string | null;
  global_shortcut: string | null;
  global_shortcut_enabled: boolean;
  last_environment_check_json: string | null;
  created_at: string;
  updated_at: string;
}

export interface AiProviderStatus {
  id: AiProviderId;
  label: string;
  ready: boolean;
  detail: string;
  is_recommended: boolean;
}

export interface AgentReachStatus {
  installed: boolean;
  version: string | null;
  detail: string;
}

export interface SourcePlatformStatus {
  key: string;
  name: string;
  availability: PlatformAvailability;
  active_backend: string | null;
  action: PlatformAction;
  message: string;
  summary: string;
  raw_status: string;
}

export interface EnvironmentStatus {
  ai_providers: AiProviderStatus[];
  agent_reach: AgentReachStatus;
  recommended_provider_id: AiProviderId | null;
  source_platforms: SourcePlatformStatus[];
  source_platforms_checked: boolean;
  source_platforms_updated_at: string | null;
  source_platforms_error: string | null;
}
