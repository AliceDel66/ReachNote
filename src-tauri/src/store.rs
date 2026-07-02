use std::path::Path;
use std::sync::Mutex;

use reachnote_core::analysis::ProviderId;
use reachnote_core::notion::NotionSettings;
use reachnote_core::task::{ErrorKind, Task, TaskStatus};
use reachnote_core::template::{template_by_id, DEFAULT_TEMPLATE_ID};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

pub struct TaskStore {
    connection: Mutex<Connection>,
}

#[derive(Debug)]
pub struct StoreError {
    pub kind: ErrorKind,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AppSettings {
    pub onboarding_completed: bool,
    pub default_provider_id: Option<String>,
    pub default_template_id: Option<String>,
    pub default_destination_id: Option<String>,
    pub global_shortcut: Option<String>,
    pub global_shortcut_enabled: bool,
    pub last_environment_check_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CapabilitySnapshot {
    pub id: i64,
    pub agent_reach_version: Option<String>,
    pub doctor_json: String,
    pub normalized_json: String,
    pub created_at: String,
}

impl StoreError {
    fn database(error: rusqlite::Error) -> Self {
        Self {
            kind: ErrorKind::ReadFailed,
            message: format!("本地队列数据库错误: {error}"),
        }
    }

    fn invalid_record(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::ParseFailed,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} ({})", self.message, self.kind.as_str())
    }
}

impl std::error::Error for StoreError {}

impl From<rusqlite::Error> for StoreError {
    fn from(error: rusqlite::Error) -> Self {
        Self::database(error)
    }
}

impl TaskStore {
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        let connection = Connection::open(path).map_err(StoreError::database)?;
        let store = Self {
            connection: Mutex::new(connection),
        };
        store.migrate()?;
        Ok(store)
    }

    pub fn insert_task(&self, task: &Task) -> Result<(), StoreError> {
        validate_task_for_storage(task)?;
        let connection = self.lock_connection()?;
        connection.execute(
            "INSERT INTO tasks (
                id, url, source_type, template_id, status, title, source_domain,
                score, model, provider_id, note, analysis_json, notion_page_id,
                error_kind, error_message, created_at, updated_at, synced_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                &task.id,
                &task.url,
                &task.source_type,
                &task.template_id,
                task.status.as_str(),
                &task.title,
                &task.source_domain,
                task.score.map(i64::from),
                &task.model,
                &task.provider_id,
                &task.note,
                &task.analysis_json,
                &task.notion_page_id,
                task.error_kind.map(ErrorKind::as_str),
                &task.error_message,
                &task.created_at,
                &task.updated_at,
                &task.synced_at,
            ],
        )?;
        Ok(())
    }

    pub fn update_task(&self, task: &Task) -> Result<(), StoreError> {
        validate_task_for_storage(task)?;
        let connection = self.lock_connection()?;
        let changed_rows = connection.execute(
            "UPDATE tasks SET
                url = ?2,
                source_type = ?3,
                template_id = ?4,
                status = ?5,
                title = ?6,
                source_domain = ?7,
                score = ?8,
                model = ?9,
                provider_id = ?10,
                note = ?11,
                analysis_json = ?12,
                notion_page_id = ?13,
                error_kind = ?14,
                error_message = ?15,
                created_at = ?16,
                updated_at = ?17,
                synced_at = ?18
            WHERE id = ?1",
            params![
                &task.id,
                &task.url,
                &task.source_type,
                &task.template_id,
                task.status.as_str(),
                &task.title,
                &task.source_domain,
                task.score.map(i64::from),
                &task.model,
                &task.provider_id,
                &task.note,
                &task.analysis_json,
                &task.notion_page_id,
                task.error_kind.map(ErrorKind::as_str),
                &task.error_message,
                &task.created_at,
                &task.updated_at,
                &task.synced_at,
            ],
        )?;

        if changed_rows == 0 {
            return Err(StoreError {
                kind: ErrorKind::ReadFailed,
                message: format!("找不到本地队列任务: {}", task.id),
            });
        }

        Ok(())
    }

    pub fn list_tasks(&self) -> Result<Vec<Task>, StoreError> {
        let connection = self.lock_connection()?;
        let mut statement = connection.prepare(
            "SELECT
                id, url, source_type, template_id, status, title, source_domain,
                score, model, provider_id, note, analysis_json, notion_page_id,
                error_kind, error_message, created_at, updated_at, synced_at
            FROM tasks
            ORDER BY CAST(created_at AS INTEGER) DESC, id DESC",
        )?;

        let tasks = statement
            .query_map([], map_task_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(tasks)
    }

    pub fn list_pending_sync_tasks(&self) -> Result<Vec<Task>, StoreError> {
        Ok(self
            .list_tasks()?
            .into_iter()
            .filter(task_needs_auto_sync)
            .collect())
    }

    pub fn get_task(&self, id: &str) -> Result<Option<Task>, StoreError> {
        let connection = self.lock_connection()?;
        connection
            .query_row(
                "SELECT
                    id, url, source_type, template_id, status, title, source_domain,
                    score, model, provider_id, note, analysis_json, notion_page_id,
                    error_kind, error_message, created_at, updated_at, synced_at
                FROM tasks
                WHERE id = ?1",
                [id],
                map_task_row,
            )
            .optional()
            .map_err(StoreError::database)
    }

    pub fn get_notion_settings(&self) -> Result<Option<NotionSettings>, StoreError> {
        let connection = self.lock_connection()?;
        connection
            .query_row(
                "SELECT token, database_id, version FROM notion_settings WHERE id = 1",
                [],
                |row| {
                    Ok(NotionSettings {
                        token: row.get(0)?,
                        database_id: row.get(1)?,
                        version: row.get(2)?,
                    })
                },
            )
            .optional()
            .map_err(StoreError::database)
    }

    pub fn save_notion_settings(
        &self,
        settings: &NotionSettings,
        updated_at: &str,
    ) -> Result<(), StoreError> {
        validate_notion_settings_for_storage(settings)?;
        let connection = self.lock_connection()?;
        connection.execute(
            "INSERT INTO notion_settings (id, token, database_id, version, updated_at)
            VALUES (1, ?1, ?2, ?3, ?4)
            ON CONFLICT(id) DO UPDATE SET
                token = excluded.token,
                database_id = excluded.database_id,
                version = excluded.version,
                updated_at = excluded.updated_at",
            params![
                &settings.token,
                &settings.database_id,
                &settings.version,
                updated_at
            ],
        )?;
        Ok(())
    }

    pub fn get_app_settings(&self) -> Result<AppSettings, StoreError> {
        let connection = self.lock_connection()?;
        connection
            .query_row(
                "SELECT
                    onboarding_completed, default_provider_id, default_template_id,
                    default_destination_id, global_shortcut, global_shortcut_enabled,
                    last_environment_check_json, created_at, updated_at
                FROM app_settings
                WHERE id = 'singleton'",
                [],
                map_app_settings_row,
            )
            .optional()
            .map_err(StoreError::database)?
            .ok_or_else(|| StoreError::invalid_record("缺少 app_settings singleton"))
    }

    pub fn save_app_settings(&self, settings: &AppSettings) -> Result<(), StoreError> {
        validate_app_settings_for_storage(settings)?;
        let connection = self.lock_connection()?;
        connection.execute(
            "INSERT INTO app_settings (
                id, onboarding_completed, default_provider_id, default_template_id,
                default_destination_id, global_shortcut, global_shortcut_enabled,
                last_environment_check_json, created_at, updated_at
            ) VALUES ('singleton', ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(id) DO UPDATE SET
                onboarding_completed = excluded.onboarding_completed,
                default_provider_id = excluded.default_provider_id,
                default_template_id = excluded.default_template_id,
                default_destination_id = excluded.default_destination_id,
                global_shortcut = excluded.global_shortcut,
                global_shortcut_enabled = excluded.global_shortcut_enabled,
                last_environment_check_json = excluded.last_environment_check_json,
                updated_at = excluded.updated_at",
            params![
                bool_to_int(settings.onboarding_completed),
                &settings.default_provider_id,
                &settings.default_template_id,
                &settings.default_destination_id,
                &settings.global_shortcut,
                bool_to_int(settings.global_shortcut_enabled),
                &settings.last_environment_check_json,
                &settings.created_at,
                &settings.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn save_environment_snapshot(
        &self,
        environment_json: &str,
        updated_at: &str,
    ) -> Result<(), StoreError> {
        let mut settings = self.get_app_settings()?;
        settings.last_environment_check_json = Some(environment_json.to_string());
        settings.updated_at = updated_at.to_string();
        self.save_app_settings(&settings)
    }

    pub fn save_capability_snapshot(
        &self,
        agent_reach_version: Option<&str>,
        doctor_json: &str,
        normalized_json: &str,
        created_at: &str,
    ) -> Result<(), StoreError> {
        if doctor_json.trim().is_empty() || normalized_json.trim().is_empty() {
            return Err(StoreError::invalid_record(
                "平台能力快照 doctor_json / normalized_json 不能为空",
            ));
        }

        let connection = self.lock_connection()?;
        connection.execute(
            "INSERT INTO source_capability_snapshots (
                agent_reach_version, doctor_json, normalized_json, created_at
            ) VALUES (?1, ?2, ?3, ?4)",
            params![
                agent_reach_version,
                doctor_json,
                normalized_json,
                created_at
            ],
        )?;
        Ok(())
    }

    pub fn get_latest_capability_snapshot(&self) -> Result<Option<CapabilitySnapshot>, StoreError> {
        let connection = self.lock_connection()?;
        connection
            .query_row(
                "SELECT id, agent_reach_version, doctor_json, normalized_json, created_at
                FROM source_capability_snapshots
                ORDER BY id DESC
                LIMIT 1",
                [],
                |row| {
                    Ok(CapabilitySnapshot {
                        id: row.get(0)?,
                        agent_reach_version: row.get(1)?,
                        doctor_json: row.get(2)?,
                        normalized_json: row.get(3)?,
                        created_at: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(StoreError::database)
    }

    pub fn recover_stale_processing_tasks(
        &self,
        now: &str,
        stale_after_seconds: u64,
    ) -> Result<Vec<Task>, StoreError> {
        let now_seconds = now.parse::<u64>().unwrap_or(0);
        let tasks = self.list_tasks()?;
        let mut recovered_tasks = Vec::new();

        for mut task in tasks {
            let previous_status = task.status;
            if !task_status_can_recover(previous_status)
                || !task_is_stale(&task, now_seconds, stale_after_seconds)
            {
                continue;
            }

            task.status = TaskStatus::Failed;
            task.error_kind = Some(ErrorKind::ReadFailed);
            task.error_message = Some(interrupted_task_message(previous_status).to_string());
            task.updated_at = now.to_string();
            self.update_task(&task)?;
            recovered_tasks.push(task);
        }

        Ok(recovered_tasks)
    }

    fn migrate(&self) -> Result<(), StoreError> {
        let connection = self.lock_connection()?;
        let existing_schema = connection
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'tasks'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?;

        match existing_schema {
            None => {
                connection.execute_batch(TASKS_SCHEMA)?;
            }
            Some(schema) if schema_needs_rebuild(&schema) => {
                connection.execute_batch(TASKS_REBUILD_SCHEMA)?;
            }
            Some(_) => {}
        }

        connection.execute_batch(NOTION_SETTINGS_SCHEMA)?;
        connection.execute_batch(APP_SETTINGS_SCHEMA)?;
        connection.execute_batch(SOURCE_CAPABILITY_SNAPSHOTS_SCHEMA)?;
        ensure_app_settings_row(&connection)?;

        Ok(())
    }

    fn lock_connection(&self) -> Result<std::sync::MutexGuard<'_, Connection>, StoreError> {
        self.connection.lock().map_err(|_| StoreError {
            kind: ErrorKind::ReadFailed,
            message: "本地队列数据库连接已不可用".to_string(),
        })
    }
}

fn map_task_row(row: &rusqlite::Row<'_>) -> Result<Task, rusqlite::Error> {
    let status_text: String = row.get(4)?;
    let status = TaskStatus::from_str(&status_text).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            4,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("未知任务状态: {status_text}"),
            )),
        )
    })?;

    let error_kind_text: Option<String> = row.get(13)?;
    let error_kind = match error_kind_text {
        Some(value) => Some(ErrorKind::from_str(&value).ok_or_else(|| {
            rusqlite::Error::FromSqlConversionFailure(
                13,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("未知错误分类: {value}"),
                )),
            )
        })?),
        None => None,
    };

    let score: Option<i64> = row.get(7)?;
    let score = score.and_then(|value| u8::try_from(value).ok());

    Ok(Task {
        id: row.get(0)?,
        url: row.get(1)?,
        source_type: row.get(2)?,
        template_id: row.get(3)?,
        status,
        title: row.get(5)?,
        source_domain: row.get(6)?,
        score,
        model: row.get(8)?,
        provider_id: row.get(9)?,
        note: row.get(10)?,
        analysis_json: row.get(11)?,
        notion_page_id: row.get(12)?,
        error_kind,
        error_message: row.get(14)?,
        created_at: row.get(15)?,
        updated_at: row.get(16)?,
        synced_at: row.get(17)?,
    })
}

fn map_app_settings_row(row: &rusqlite::Row<'_>) -> Result<AppSettings, rusqlite::Error> {
    Ok(AppSettings {
        onboarding_completed: int_to_bool(row.get(0)?),
        default_provider_id: row.get(1)?,
        default_template_id: row.get(2)?,
        default_destination_id: row.get(3)?,
        global_shortcut: row.get(4)?,
        global_shortcut_enabled: int_to_bool(row.get(5)?),
        last_environment_check_json: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

const TASKS_SCHEMA: &str = "
CREATE TABLE tasks (
    id TEXT PRIMARY KEY,
    url TEXT NOT NULL,
    source_type TEXT NOT NULL,
    template_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('queued', 'reading', 'analyzing', 'analyzed', 'syncing', 'synced', 'failed')),
    title TEXT,
    source_domain TEXT,
    score INTEGER,
    model TEXT,
    provider_id TEXT NOT NULL DEFAULT 'claude_cli' CHECK (provider_id IN ('claude_cli', 'codex_cli', 'openai_compatible')),
    note TEXT,
    analysis_json TEXT,
    notion_page_id TEXT,
    error_kind TEXT CHECK (
        error_kind IS NULL OR error_kind IN (
            'invalid_url',
            'read_failed',
            'provider_unavailable',
            'parse_failed',
            'notion_unauthorized',
            'schema_mismatch',
            'network_failed'
        )
    ),
    error_message TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    synced_at TEXT
);
";

const TASKS_REBUILD_SCHEMA: &str = "
BEGIN;
CREATE TABLE tasks_next (
    id TEXT PRIMARY KEY,
    url TEXT NOT NULL,
    source_type TEXT NOT NULL,
    template_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('queued', 'reading', 'analyzing', 'analyzed', 'syncing', 'synced', 'failed')),
    title TEXT,
    source_domain TEXT,
    score INTEGER,
    model TEXT,
    provider_id TEXT NOT NULL DEFAULT 'claude_cli' CHECK (provider_id IN ('claude_cli', 'codex_cli', 'openai_compatible')),
    note TEXT,
    analysis_json TEXT,
    notion_page_id TEXT,
    error_kind TEXT CHECK (
        error_kind IS NULL OR error_kind IN (
            'invalid_url',
            'read_failed',
            'provider_unavailable',
            'parse_failed',
            'notion_unauthorized',
            'schema_mismatch',
            'network_failed'
        )
    ),
    error_message TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    synced_at TEXT
);
INSERT INTO tasks_next (
    id, url, source_type, template_id, status, title, source_domain,
    score, model, provider_id, note, analysis_json, notion_page_id,
    error_kind, error_message, created_at, updated_at, synced_at
)
SELECT
    id, url, source_type, template_id, status, title, source_domain,
    score, model, 'claude_cli', NULL, NULL, notion_page_id,
    error_kind, error_message, created_at, updated_at, synced_at
FROM tasks;
DROP TABLE tasks;
ALTER TABLE tasks_next RENAME TO tasks;
COMMIT;
";

const NOTION_SETTINGS_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS notion_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    token TEXT NOT NULL,
    database_id TEXT NOT NULL,
    version TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
";

const APP_SETTINGS_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS app_settings (
    id TEXT PRIMARY KEY CHECK (id = 'singleton'),
    onboarding_completed INTEGER NOT NULL CHECK (onboarding_completed IN (0, 1)),
    default_provider_id TEXT CHECK (
        default_provider_id IS NULL OR default_provider_id IN ('claude_cli', 'codex_cli', 'openai_compatible')
    ),
    default_template_id TEXT,
    default_destination_id TEXT,
    global_shortcut TEXT,
    global_shortcut_enabled INTEGER NOT NULL CHECK (global_shortcut_enabled IN (0, 1)),
    last_environment_check_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
";

const SOURCE_CAPABILITY_SNAPSHOTS_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS source_capability_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_reach_version TEXT,
    doctor_json TEXT NOT NULL,
    normalized_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);
";

fn schema_needs_rebuild(schema: &str) -> bool {
    !schema.contains("'analyzed'")
        || !schema.contains("provider_id")
        || !schema.contains("analysis_json")
        || !schema.contains("note TEXT")
}

fn validate_task_for_storage(task: &Task) -> Result<(), StoreError> {
    if task.source_type != "article" {
        return Err(StoreError::invalid_record(format!(
            "当前阶段只允许 article source_type: {}",
            task.source_type
        )));
    }

    if template_by_id(&task.template_id).is_none() {
        return Err(StoreError::invalid_record(format!(
            "未知 template_id: {}",
            task.template_id
        )));
    }

    if ProviderId::from_str(&task.provider_id).is_none() {
        return Err(StoreError::invalid_record(format!(
            "未知 AI provider: {}",
            task.provider_id
        )));
    }

    Ok(())
}

fn validate_app_settings_for_storage(settings: &AppSettings) -> Result<(), StoreError> {
    if let Some(provider_id) = settings.default_provider_id.as_deref() {
        if ProviderId::from_str(provider_id).is_none() {
            return Err(StoreError::invalid_record(format!(
                "未知默认 AI provider: {provider_id}"
            )));
        }
    }

    if let Some(template_id) = settings.default_template_id.as_deref() {
        if template_id.trim().is_empty() {
            return Err(StoreError::invalid_record("默认模板不能为空字符串"));
        }
        if template_by_id(template_id).is_none() {
            return Err(StoreError::invalid_record(format!(
                "未知默认模板: {template_id}"
            )));
        }
    }

    if settings
        .global_shortcut
        .as_deref()
        .map(str::trim)
        .is_some_and(str::is_empty)
    {
        return Err(StoreError::invalid_record("全局快捷键不能为空字符串"));
    }

    if settings.created_at.trim().is_empty() || settings.updated_at.trim().is_empty() {
        return Err(StoreError::invalid_record(
            "app_settings created_at / updated_at 不能为空",
        ));
    }

    Ok(())
}

fn ensure_app_settings_row(connection: &Connection) -> Result<(), StoreError> {
    let has_settings: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM app_settings WHERE id = 'singleton')",
        [],
        |row| row.get::<_, i64>(0).map(|value| value != 0),
    )?;
    if has_settings {
        return Ok(());
    }

    let task_count =
        connection.query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get::<_, i64>(0))?;
    let notion_count = connection.query_row("SELECT COUNT(*) FROM notion_settings", [], |row| {
        row.get::<_, i64>(0)
    })?;
    let existing_install = task_count > 0 || notion_count > 0;
    let timestamp = "0";
    let default_destination_id = if notion_count > 0 {
        Some("notion")
    } else {
        None
    };
    connection.execute(
        "INSERT INTO app_settings (
            id, onboarding_completed, default_provider_id, default_template_id,
            default_destination_id, global_shortcut, global_shortcut_enabled,
            last_environment_check_json, created_at, updated_at
        ) VALUES ('singleton', ?1, 'claude_cli', ?2, ?3, 'CommandOrControl+Shift+R', 0, NULL, ?4, ?5)",
        params![
            bool_to_int(existing_install),
            DEFAULT_TEMPLATE_ID,
            default_destination_id,
            timestamp,
            timestamp,
        ],
    )?;

    Ok(())
}

fn validate_notion_settings_for_storage(settings: &NotionSettings) -> Result<(), StoreError> {
    if settings.token.trim().is_empty() {
        return Err(StoreError::invalid_record(
            "Notion Integration Token 不能为空",
        ));
    }

    if settings.database_id.trim().is_empty() {
        return Err(StoreError::invalid_record("Notion Database ID 不能为空"));
    }

    if settings.version.trim().is_empty() {
        return Err(StoreError::invalid_record("Notion API version 不能为空"));
    }

    Ok(())
}

fn task_status_can_recover(status: TaskStatus) -> bool {
    matches!(
        status,
        TaskStatus::Reading | TaskStatus::Analyzing | TaskStatus::Syncing
    )
}

fn task_is_stale(task: &Task, now_seconds: u64, stale_after_seconds: u64) -> bool {
    let Ok(updated_at) = task.updated_at.parse::<u64>() else {
        return true;
    };

    now_seconds.saturating_sub(updated_at) >= stale_after_seconds
}

fn interrupted_task_message(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Reading => "任务在读取阶段中断，已停止；请点击重试重新读取和分析。",
        TaskStatus::Analyzing => "任务在分析阶段中断，已停止；请点击重试重新分析。",
        TaskStatus::Syncing => {
            "任务在同步阶段中断，已停止；本地研究卡已保留，请点击重试同步到 Notion。"
        }
        _ => "任务在上次运行中中断，已停止；请点击重试。",
    }
}

fn task_needs_auto_sync(task: &Task) -> bool {
    task.status == TaskStatus::Analyzed
        && task.analysis_json.is_some()
        && task.notion_page_id.is_none()
}

fn bool_to_int(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

fn int_to_bool(value: i64) -> bool {
    value != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory_store() -> TaskStore {
        let store = TaskStore {
            connection: Mutex::new(Connection::open_in_memory().unwrap()),
        };
        store.migrate().unwrap();
        store
    }

    fn sample_task(id: &str) -> Task {
        Task {
            id: id.to_string(),
            url: "https://example.com/article".to_string(),
            source_type: "article".to_string(),
            template_id: "article".to_string(),
            status: TaskStatus::Queued,
            title: None,
            source_domain: Some("example.com".to_string()),
            score: None,
            model: Some("Claude CLI".to_string()),
            provider_id: "claude_cli".to_string(),
            note: Some("研究备注".to_string()),
            analysis_json: None,
            notion_page_id: None,
            error_kind: None,
            error_message: None,
            created_at: "100".to_string(),
            updated_at: "100".to_string(),
            synced_at: None,
        }
    }

    #[test]
    fn insert_and_get_round_trips_all_fields() {
        let store = memory_store();
        let task = sample_task("task-1");
        store.insert_task(&task).unwrap();

        let loaded = store.get_task("task-1").unwrap().unwrap();
        assert_eq!(loaded, task);
        assert_eq!(loaded.note.as_deref(), Some("研究备注"));
        assert_eq!(loaded.provider_id, "claude_cli");
    }

    #[test]
    fn update_persists_analysis_fields() {
        let store = memory_store();
        let mut task = sample_task("task-1");
        store.insert_task(&task).unwrap();

        task.status = TaskStatus::Analyzed;
        task.title = Some("研究卡标题".to_string());
        task.score = Some(4);
        task.analysis_json = Some(r#"{"title":"研究卡标题"}"#.to_string());
        store.update_task(&task).unwrap();

        let loaded = store.get_task("task-1").unwrap().unwrap();
        assert_eq!(loaded.status, TaskStatus::Analyzed);
        assert_eq!(loaded.score, Some(4));
        assert_eq!(loaded.title.as_deref(), Some("研究卡标题"));
        assert_eq!(
            loaded.analysis_json.as_deref(),
            Some(r#"{"title":"研究卡标题"}"#)
        );
    }

    #[test]
    fn update_missing_task_reports_error() {
        let store = memory_store();
        let task = sample_task("missing");
        let error = store.update_task(&task).unwrap_err();
        assert_eq!(error.kind, ErrorKind::ReadFailed);
    }

    #[test]
    fn rejects_non_article_source_type() {
        let store = memory_store();
        let mut task = sample_task("task-1");
        task.source_type = "video".to_string();
        let error = store.insert_task(&task).unwrap_err();
        assert_eq!(error.kind, ErrorKind::ParseFailed);
    }

    #[test]
    fn accepts_registered_template_ids() {
        let store = memory_store();
        let mut task = sample_task("task-1");
        task.template_id = "github_project".to_string();
        store.insert_task(&task).unwrap();

        let loaded = store.get_task("task-1").unwrap().unwrap();
        assert_eq!(loaded.template_id, "github_project");
    }

    #[test]
    fn rejects_unknown_template_id() {
        let store = memory_store();
        let mut task = sample_task("task-1");
        task.template_id = "unknown_template".to_string();

        let error = store.insert_task(&task).unwrap_err();

        assert_eq!(error.kind, ErrorKind::ParseFailed);
    }

    #[test]
    fn rejects_unknown_provider_id() {
        let store = memory_store();
        let mut task = sample_task("task-1");
        task.provider_id = "unknown".to_string();
        let error = store.insert_task(&task).unwrap_err();
        assert_eq!(error.kind, ErrorKind::ParseFailed);
    }

    #[test]
    fn list_orders_by_created_at_desc() {
        let store = memory_store();
        let mut older = sample_task("task-old");
        older.created_at = "100".to_string();
        let mut newer = sample_task("task-new");
        newer.created_at = "200".to_string();
        store.insert_task(&older).unwrap();
        store.insert_task(&newer).unwrap();

        let tasks = store.list_tasks().unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].id, "task-new");
        assert_eq!(tasks[1].id, "task-old");
    }

    #[test]
    fn recovers_only_stale_processing_tasks() {
        let store = memory_store();
        let mut reading = sample_task("reading");
        reading.status = TaskStatus::Reading;
        reading.updated_at = "10".to_string();
        let mut analyzing = sample_task("analyzing");
        analyzing.status = TaskStatus::Analyzing;
        analyzing.updated_at = "20".to_string();
        let mut syncing = sample_task("syncing");
        syncing.status = TaskStatus::Syncing;
        syncing.updated_at = "30".to_string();
        syncing.analysis_json = Some(r#"{"title":"研究卡"}"#.to_string());
        let mut recent = sample_task("recent");
        recent.status = TaskStatus::Analyzing;
        recent.updated_at = "95".to_string();
        let mut queued = sample_task("queued");
        queued.status = TaskStatus::Queued;
        queued.updated_at = "1".to_string();

        for task in [&reading, &analyzing, &syncing, &recent, &queued] {
            store.insert_task(task).unwrap();
        }

        let recovered = store.recover_stale_processing_tasks("100", 60).unwrap();
        let mut recovered_ids = recovered
            .iter()
            .map(|task| task.id.as_str())
            .collect::<Vec<_>>();
        recovered_ids.sort_unstable();
        assert_eq!(recovered_ids, vec!["analyzing", "reading", "syncing"]);

        let loaded_reading = store.get_task("reading").unwrap().unwrap();
        assert_eq!(loaded_reading.status, TaskStatus::Failed);
        assert_eq!(loaded_reading.error_kind, Some(ErrorKind::ReadFailed));
        assert!(loaded_reading
            .error_message
            .as_deref()
            .unwrap()
            .contains("读取阶段中断"));
        assert_eq!(loaded_reading.updated_at, "100");

        let loaded_syncing = store.get_task("syncing").unwrap().unwrap();
        assert_eq!(loaded_syncing.status, TaskStatus::Failed);
        assert!(loaded_syncing.analysis_json.is_some());
        assert!(loaded_syncing
            .error_message
            .as_deref()
            .unwrap()
            .contains("同步阶段中断"));

        assert_eq!(
            store.get_task("recent").unwrap().unwrap().status,
            TaskStatus::Analyzing
        );
        assert_eq!(
            store.get_task("queued").unwrap().unwrap().status,
            TaskStatus::Queued
        );
    }

    #[test]
    fn lists_only_analyzed_tasks_waiting_for_sync() {
        let store = memory_store();
        let mut pending = sample_task("pending-sync");
        pending.status = TaskStatus::Analyzed;
        pending.analysis_json = Some(r#"{"title":"待同步"}"#.to_string());
        let mut synced = sample_task("synced");
        synced.status = TaskStatus::Synced;
        synced.analysis_json = Some(r#"{"title":"已同步"}"#.to_string());
        synced.notion_page_id = Some("page-id".to_string());
        let mut failed_with_analysis = sample_task("failed");
        failed_with_analysis.status = TaskStatus::Failed;
        failed_with_analysis.analysis_json = Some(r#"{"title":"同步失败"}"#.to_string());
        let mut analyzed_without_json = sample_task("no-json");
        analyzed_without_json.status = TaskStatus::Analyzed;

        store.insert_task(&pending).unwrap();
        store.insert_task(&synced).unwrap();
        store.insert_task(&failed_with_analysis).unwrap();
        store.insert_task(&analyzed_without_json).unwrap();

        let tasks = store.list_pending_sync_tasks().unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, pending.id);
    }

    #[test]
    fn migrate_rebuilds_legacy_schema_and_preserves_rows() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE tasks (
                    id TEXT PRIMARY KEY,
                    url TEXT NOT NULL,
                    source_type TEXT NOT NULL,
                    template_id TEXT NOT NULL,
                    status TEXT NOT NULL,
                    title TEXT,
                    source_domain TEXT,
                    score INTEGER,
                    model TEXT,
                    notion_page_id TEXT,
                    error_kind TEXT,
                    error_message TEXT,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    synced_at TEXT
                );
                INSERT INTO tasks (id, url, source_type, template_id, status, created_at, updated_at)
                VALUES ('legacy-1', 'https://example.com/a', 'article', 'article', 'queued', '100', '100');",
            )
            .unwrap();

        let store = TaskStore {
            connection: Mutex::new(connection),
        };
        store.migrate().unwrap();

        let loaded = store.get_task("legacy-1").unwrap().unwrap();
        assert_eq!(loaded.url, "https://example.com/a");
        assert_eq!(loaded.provider_id, "claude_cli");
        assert_eq!(loaded.note, None);
        assert_eq!(loaded.analysis_json, None);
        assert!(store.get_app_settings().unwrap().onboarding_completed);

        let settings = NotionSettings {
            token: "ntn_fake_token".to_string(),
            database_id: "database-123".to_string(),
            version: "2022-06-28".to_string(),
        };
        store.save_notion_settings(&settings, "200").unwrap();
        assert_eq!(store.get_notion_settings().unwrap(), Some(settings));
    }

    #[test]
    fn notion_settings_round_trip_and_update() {
        let store = memory_store();
        assert_eq!(store.get_notion_settings().unwrap(), None);

        let mut settings = NotionSettings {
            token: "ntn_fake_token".to_string(),
            database_id: "database-123".to_string(),
            version: "2022-06-28".to_string(),
        };
        store.save_notion_settings(&settings, "100").unwrap();
        assert_eq!(store.get_notion_settings().unwrap(), Some(settings.clone()));

        settings.database_id = "database-456".to_string();
        store.save_notion_settings(&settings, "200").unwrap();
        assert_eq!(store.get_notion_settings().unwrap(), Some(settings));
    }

    #[test]
    fn rejects_empty_notion_settings() {
        let store = memory_store();
        let settings = NotionSettings {
            token: String::new(),
            database_id: "database-123".to_string(),
            version: "2022-06-28".to_string(),
        };

        let error = store.save_notion_settings(&settings, "100").unwrap_err();
        assert_eq!(error.kind, ErrorKind::ParseFailed);
    }

    #[test]
    fn new_store_starts_with_onboarding_required() {
        let store = memory_store();

        let settings = store.get_app_settings().unwrap();

        assert!(!settings.onboarding_completed);
        assert_eq!(settings.default_provider_id.as_deref(), Some("claude_cli"));
        assert_eq!(
            settings.default_template_id.as_deref(),
            Some(DEFAULT_TEMPLATE_ID)
        );
        assert_eq!(settings.default_destination_id, None);
        assert_eq!(
            settings.global_shortcut.as_deref(),
            Some("CommandOrControl+Shift+R")
        );
        assert!(!settings.global_shortcut_enabled);
    }

    #[test]
    fn app_settings_round_trip_and_environment_snapshot() {
        let store = memory_store();
        let mut settings = store.get_app_settings().unwrap();
        settings.onboarding_completed = true;
        settings.default_provider_id = Some("codex_cli".to_string());
        settings.default_template_id = Some("github_project".to_string());
        settings.default_destination_id = Some("notion".to_string());
        settings.updated_at = "200".to_string();
        store.save_app_settings(&settings).unwrap();

        store
            .save_environment_snapshot("{\"ok\":true}", "300")
            .unwrap();
        let loaded = store.get_app_settings().unwrap();

        assert!(loaded.onboarding_completed);
        assert_eq!(loaded.default_provider_id.as_deref(), Some("codex_cli"));
        assert_eq!(
            loaded.default_template_id.as_deref(),
            Some("github_project")
        );
        assert_eq!(loaded.default_destination_id.as_deref(), Some("notion"));
        assert_eq!(
            loaded.last_environment_check_json.as_deref(),
            Some("{\"ok\":true}")
        );
        assert_eq!(loaded.updated_at, "300");
    }

    #[test]
    fn capability_snapshot_round_trip_reads_latest() {
        let store = memory_store();
        assert_eq!(store.get_latest_capability_snapshot().unwrap(), None);

        store
            .save_capability_snapshot(
                Some("agent-reach v1.5.0"),
                r#"{"github":{"status":"ok"}}"#,
                r#"[{"key":"github"}]"#,
                "100",
            )
            .unwrap();
        store
            .save_capability_snapshot(
                None,
                r#"{"web":{"status":"ok"}}"#,
                r#"[{"key":"web"}]"#,
                "200",
            )
            .unwrap();

        let snapshot = store.get_latest_capability_snapshot().unwrap().unwrap();
        assert_eq!(snapshot.agent_reach_version, None);
        assert_eq!(snapshot.doctor_json, r#"{"web":{"status":"ok"}}"#);
        assert_eq!(snapshot.normalized_json, r#"[{"key":"web"}]"#);
        assert_eq!(snapshot.created_at, "200");
    }

    #[test]
    fn rejects_unknown_default_provider() {
        let store = memory_store();
        let mut settings = store.get_app_settings().unwrap();
        settings.default_provider_id = Some("unknown".to_string());

        let error = store.save_app_settings(&settings).unwrap_err();

        assert_eq!(error.kind, ErrorKind::ParseFailed);
    }

    #[test]
    fn rejects_unknown_default_template() {
        let store = memory_store();
        let mut settings = store.get_app_settings().unwrap();
        settings.default_template_id = Some("unknown_template".to_string());

        let error = store.save_app_settings(&settings).unwrap_err();

        assert_eq!(error.kind, ErrorKind::ParseFailed);
    }
}
