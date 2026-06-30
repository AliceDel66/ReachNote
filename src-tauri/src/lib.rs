use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reachnote_core::analysis::{AnalysisRequest, ProviderId};
use reachnote_core::task::{validate_article_url, ErrorKind, Task, TaskStatus};
use tauri::{Manager, State};

mod provider;
mod reader;
mod store;

use provider::ProviderRunner;
use reader::AgentReachWebReader;
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
    provider_id: Option<String>,
) -> Result<Task, String> {
    let validated = validate_article_url(&url)
        .map_err(|kind| command_error(kind, "请输入合法的 http(s) 文章 URL"))?;
    let provider = match provider_id.as_deref() {
        Some(value) => ProviderId::from_str(value).ok_or_else(|| {
            command_error(
                ErrorKind::SchemaMismatch,
                &format!("未知 AI provider: {value}"),
            )
        })?,
        None => ProviderId::ClaudeCli,
    };
    let normalized_note = note
        .map(|value| value.chars().take(500).collect::<String>())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
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
        model: Some(provider.label().to_string()),
        provider_id: provider.as_str().to_string(),
        note: normalized_note,
        analysis_json: None,
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
        .ok_or_else(|| {
            command_error(ErrorKind::ReadFailed, &format!("找不到本地队列任务: {id}"))
        })?;

    task.status = TaskStatus::Reading;
    task.error_kind = None;
    task.error_message = None;
    task.updated_at = current_unix_timestamp();
    store
        .update_task(&task)
        .map_err(|error| command_error(error.kind, &error.message))?;

    let content = match AgentReachWebReader::from_env().read_article(&task.url) {
        Ok(content) => content,
        Err(error) => {
            task.status = TaskStatus::Failed;
            task.error_kind = Some(error.kind);
            task.error_message = Some(error.message);
            task.updated_at = current_unix_timestamp();
            store
                .update_task(&task)
                .map_err(|error| command_error(error.kind, &error.message))?;
            return Ok(task);
        }
    };

    task.status = TaskStatus::Analyzing;
    task.updated_at = current_unix_timestamp();
    store
        .update_task(&task)
        .map_err(|error| command_error(error.kind, &error.message))?;

    let provider_id = ProviderId::from_str(&task.provider_id).ok_or_else(|| {
        command_error(
            ErrorKind::SchemaMismatch,
            &format!("未知 AI provider: {}", task.provider_id),
        )
    })?;
    let request = AnalysisRequest {
        url: task.url.clone(),
        source_type: task.source_type.clone(),
        source_domain: task.source_domain.clone(),
        template_id: task.template_id.clone(),
        note: task.note.clone(),
        content_text: Some(content.text),
        content_reader: Some(content.reader),
    };

    match ProviderRunner::from_env().analyze(provider_id, &request) {
        Ok(analysis) => {
            task.status = TaskStatus::Analyzed;
            task.title = Some(analysis.title.clone());
            task.score = Some(analysis.score);
            task.model = Some(analysis.model.clone());
            task.analysis_json = Some(serde_json::to_string(&analysis).map_err(|error| {
                command_error(
                    ErrorKind::ParseFailed,
                    &format!("结构化研究卡序列化失败: {error}"),
                )
            })?);
            task.error_kind = None;
            task.error_message = None;
        }
        Err(error) => {
            task.status = TaskStatus::Failed;
            task.error_kind = Some(error.kind);
            task.error_message = Some(error.message);
        }
    }

    task.updated_at = current_unix_timestamp();
    store
        .update_task(&task)
        .map_err(|error| command_error(error.kind, &error.message))?;

    Ok(task)
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir().map_err(|error| {
                std::io::Error::other(format!("无法读取 app data 目录: {error}"))
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
    use std::sync::atomic::{AtomicU64, Ordering};

    // 进程内单调序号，消除同一纳秒 + 同进程下的 id 碰撞。
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let now = current_duration();
    format!(
        "task-{}-{}-{}-{}",
        now.as_secs(),
        now.subsec_nanos(),
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
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
