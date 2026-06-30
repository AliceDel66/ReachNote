use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reachnote_core::task::{validate_article_url, ErrorKind, Task, TaskStatus};
use tauri::{Manager, State};

mod provider;
mod store;

use provider::ClaudeCliAvailability;
use store::TaskStore;

#[tauri::command]
fn shell_status() -> reachnote_core::ShellStatus {
    reachnote_core::shell_status()
}

#[tauri::command]
fn create_capture_task(
    store: State<'_, TaskStore>,
    url: String,
    note: Option<String>,
) -> Result<Task, String> {
    let validated = validate_article_url(&url)
        .map_err(|kind| command_error(kind, "请输入合法的 http(s) 文章 URL"))?;
    let _normalized_note: Option<String> =
        note.map(|value| value.chars().take(500).collect::<String>());
    let timestamp = current_unix_timestamp();
    let task = Task {
        id: create_task_id(),
        url: validated.url,
        source_type: "article".to_string(),
        template_id: "article".to_string(),
        status: TaskStatus::Queued,
        title: None,
        source_domain: Some(validated.source_domain),
        score: None,
        model: Some("Claude CLI".to_string()),
        notion_page_id: None,
        error_kind: None,
        error_message: None,
        created_at: timestamp.clone(),
        updated_at: timestamp,
        synced_at: None,
    };

    store
        .insert_task(&task)
        .map_err(|error| command_error(error.kind, &error.message))?;

    Ok(task)
}

#[tauri::command]
fn list_capture_tasks(store: State<'_, TaskStore>) -> Result<Vec<Task>, String> {
    store
        .list_tasks()
        .map_err(|error| command_error(error.kind, &error.message))
}

#[tauri::command]
fn run_capture_task(store: State<'_, TaskStore>, id: String) -> Result<Task, String> {
    let mut task = store
        .get_task(&id)
        .map_err(|error| command_error(error.kind, &error.message))?
        .ok_or_else(|| command_error(ErrorKind::ReadFailed, &format!("找不到本地队列任务: {id}")))?;

    task.status = TaskStatus::Analyzing;
    task.error_kind = None;
    task.error_message = None;
    task.updated_at = current_unix_timestamp();
    store
        .update_task(&task)
        .map_err(|error| command_error(error.kind, &error.message))?;

    if let Err(message) = ClaudeCliAvailability::from_env().check() {
        task.status = TaskStatus::Failed;
        task.error_kind = Some(ErrorKind::ProviderUnavailable);
        task.error_message = Some(message);
        task.updated_at = current_unix_timestamp();
        store
            .update_task(&task)
            .map_err(|error| command_error(error.kind, &error.message))?;
    }

    Ok(task)
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir().map_err(|error| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("无法读取 app data 目录: {error}"),
                )
            })?;
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join("reachnote.db");
            let store = TaskStore::open(&db_path)?;
            app.manage(store);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            shell_status,
            create_capture_task,
            list_capture_tasks,
            run_capture_task
        ])
        .run(tauri::generate_context!())
        .expect("failed to run ReachNote");
}

fn command_error(kind: ErrorKind, message: &str) -> String {
    format!("{message} ({})", kind.as_str())
}

fn create_task_id() -> String {
    let now = current_duration();
    format!(
        "task-{}-{}-{}",
        now.as_secs(),
        now.subsec_nanos(),
        std::process::id()
    )
}

fn current_unix_timestamp() -> String {
    current_duration().as_secs().to_string()
}

fn current_duration() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
}
