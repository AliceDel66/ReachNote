use std::path::Path;
use std::sync::Mutex;

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
                score, model, notion_page_id, error_kind, error_message,
                created_at, updated_at, synced_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
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
                notion_page_id = ?10,
                error_kind = ?11,
                error_message = ?12,
                created_at = ?13,
                updated_at = ?14,
                synced_at = ?15
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
                score, model, notion_page_id, error_kind, error_message,
                created_at, updated_at, synced_at
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
                    score, model, notion_page_id, error_kind, error_message,
                    created_at, updated_at, synced_at
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
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                url TEXT NOT NULL,
                source_type TEXT NOT NULL,
                template_id TEXT NOT NULL,
                status TEXT NOT NULL CHECK (status IN ('queued', 'reading', 'analyzing', 'syncing', 'synced', 'failed')),
                title TEXT,
                source_domain TEXT,
                score INTEGER,
                model TEXT,
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
            );",
        )?;
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

    let error_kind_text: Option<String> = row.get(10)?;
    let error_kind = match error_kind_text {
        Some(value) => Some(ErrorKind::from_str(&value).ok_or_else(|| {
            rusqlite::Error::FromSqlConversionFailure(
                10,
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
        notion_page_id: row.get(9)?,
        error_kind,
        error_message: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
        synced_at: row.get(14)?,
    })
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

    Ok(())
}
