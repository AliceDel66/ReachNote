use std::path::Path;
use std::sync::Mutex;

use reachnote_core::analysis::ProviderId;
use reachnote_core::task::{ErrorKind, Task, TaskStatus};
use rusqlite::{params, Connection, OptionalExtension};

pub struct TaskStore {
    connection: Mutex<Connection>,
}

#[derive(Debug)]
pub struct StoreError {
    pub kind: ErrorKind,
    pub message: String,
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

    if task.template_id != "article" {
        return Err(StoreError::invalid_record(format!(
            "当前阶段只允许 article template_id: {}",
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
    }
}
