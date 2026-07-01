use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::{env, sync::Arc};

use reachnote_core::analysis::{parse_analysis_result, AnalysisRequest, ProviderId};
use reachnote_core::notion::{
    build_notion_properties, NotionSettings, NotionSettingsView, NOTION_API_VERSION,
};
use reachnote_core::task::{validate_article_url, ErrorKind, Task, TaskStatus};
use tauri::{Manager, State};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

mod notion;
mod provider;
mod reader;
mod store;

use notion::NotionClient;
use provider::ProviderRunner;
use reader::AgentReachWebReader;
use store::TaskStore;

#[tauri::command]
fn shell_status() -> reachnote_core::ShellStatus {
    reachnote_core::shell_status()
}

#[tauri::command]
fn create_capture_task(
    store: State<'_, Arc<TaskStore>>,
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
fn list_capture_tasks(store: State<'_, Arc<TaskStore>>) -> Result<Vec<Task>, String> {
    store
        .list_tasks()
        .map_err(|error| command_error(error.kind, &error.message))
}

#[tauri::command]
fn recover_interrupted_tasks(
    store: State<'_, Arc<TaskStore>>,
    stale_after_seconds: Option<u64>,
) -> Result<Vec<Task>, String> {
    let stale_after_seconds = stale_after_seconds
        .filter(|value| *value > 0)
        .unwrap_or_else(default_stale_task_seconds);
    let now = current_unix_timestamp();
    store
        .recover_stale_processing_tasks(&now, stale_after_seconds)
        .map_err(|error| command_error(error.kind, &error.message))
}

#[tauri::command]
fn get_notion_settings(store: State<'_, Arc<TaskStore>>) -> Result<NotionSettingsView, String> {
    let settings = store
        .get_notion_settings()
        .map_err(|error| command_error(error.kind, &error.message))?;

    Ok(settings
        .map(|settings| settings.to_view())
        .unwrap_or_else(NotionSettingsView::unconfigured))
}

#[tauri::command]
fn save_notion_settings(
    store: State<'_, Arc<TaskStore>>,
    token: Option<String>,
    database_id: String,
) -> Result<NotionSettingsView, String> {
    let database_id = database_id.trim().to_string();
    if database_id.is_empty() {
        return Err(command_error(
            ErrorKind::NotionUnauthorized,
            "请填写 Notion Database ID",
        ));
    }

    let existing_settings = store
        .get_notion_settings()
        .map_err(|error| command_error(error.kind, &error.message))?;
    let token = token
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            existing_settings
                .as_ref()
                .map(|settings| settings.token.clone())
        })
        .ok_or_else(|| {
            command_error(
                ErrorKind::NotionUnauthorized,
                "首次保存 Notion 设置需要填写 Integration Token",
            )
        })?;
    let version = existing_settings
        .as_ref()
        .map(|settings| settings.version.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| NOTION_API_VERSION.to_string());
    let settings = NotionSettings {
        token,
        database_id,
        version,
    };

    store
        .save_notion_settings(&settings, &current_unix_timestamp())
        .map_err(|error| command_error(error.kind, &error.message))?;

    Ok(settings.to_view())
}

#[tauri::command]
async fn test_notion_connection(store: State<'_, Arc<TaskStore>>) -> Result<String, String> {
    // 阻塞式 Notion HTTP 请求放进 spawn_blocking：命令本身是 async，永不占用 UI 主线程；
    // 闭包跑在 tokio 阻塞线程池上，reqwest::blocking 在那里可安全执行。
    let store = Arc::clone(store.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let settings = require_notion_settings(&store)?;
        NotionClient::from_settings(settings)
            .and_then(|client| client.test_connection())
            .map_err(|error| command_error(error.kind, &error.message))
    })
    .await
    .map_err(|error| format!("测试 Notion 连接任务执行失败: {error}"))?
}

#[tauri::command]
async fn run_capture_task(store: State<'_, Arc<TaskStore>>, id: String) -> Result<Task, String> {
    // 读网页 + 调 AI CLI 最长可达 120s。放进 spawn_blocking：async 命令不占用 UI 主线程，
    // 前端轮询 list_capture_tasks 期间可实时读到 reading/analyzing 状态（store 在长操作间隙
    // 释放锁）。分析完成后的 Notion 同步也在同一个后端链路里兜底，避免前端 reload
    // 导致任务停在 Analyzed。
    let store = Arc::clone(store.inner());
    tauri::async_runtime::spawn_blocking(move || run_and_sync_capture_task_blocking(&store, id))
        .await
        .map_err(|error| format!("采集任务执行失败: {error}"))?
}

#[tauri::command]
async fn retry_capture_task(store: State<'_, Arc<TaskStore>>, id: String) -> Result<Task, String> {
    let store = Arc::clone(store.inner());
    tauri::async_runtime::spawn_blocking(move || retry_capture_task_blocking(&store, id))
        .await
        .map_err(|error| format!("重试任务执行失败: {error}"))?
}

#[tauri::command]
async fn sync_pending_analyzed_tasks(
    store: State<'_, Arc<TaskStore>>,
) -> Result<Vec<Task>, String> {
    let store = Arc::clone(store.inner());
    tauri::async_runtime::spawn_blocking(move || sync_pending_analyzed_tasks_blocking(&store))
        .await
        .map_err(|error| format!("补同步任务执行失败: {error}"))?
}

fn run_capture_task_blocking(store: &TaskStore, id: String) -> Result<Task, String> {
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

fn run_and_sync_capture_task_blocking(store: &TaskStore, id: String) -> Result<Task, String> {
    let updated_task = run_capture_task_blocking(store, id.clone())?;
    if updated_task.status == TaskStatus::Analyzed {
        sync_capture_task_blocking(store, id)
    } else {
        Ok(updated_task)
    }
}

#[tauri::command]
async fn sync_capture_task(store: State<'_, Arc<TaskStore>>, id: String) -> Result<Task, String> {
    // Notion 同步走阻塞 HTTP，同样放进 spawn_blocking，避免冻结 UI 主线程。
    let store = Arc::clone(store.inner());
    tauri::async_runtime::spawn_blocking(move || sync_capture_task_blocking(&store, id))
        .await
        .map_err(|error| format!("同步任务执行失败: {error}"))?
}

fn sync_capture_task_blocking(store: &TaskStore, id: String) -> Result<Task, String> {
    let mut task = store
        .get_task(&id)
        .map_err(|error| command_error(error.kind, &error.message))?
        .ok_or_else(|| {
            command_error(ErrorKind::ReadFailed, &format!("找不到本地队列任务: {id}"))
        })?;

    if !task_can_sync(&task) {
        return Err(command_error(
            ErrorKind::SchemaMismatch,
            "只能同步已分析任务；分析失败或尚未生成研究卡的任务需要先重试分析",
        ));
    }

    let analysis_json = match task.analysis_json.as_deref() {
        Some(value) => value,
        None => {
            return fail_sync_task(
                store,
                task,
                ErrorKind::ParseFailed,
                "任务缺少结构化研究卡，无法同步到 Notion".to_string(),
            );
        }
    };

    let analysis = match parse_analysis_result(analysis_json) {
        Ok(analysis) => analysis,
        Err(error) => return fail_sync_task(store, task, error.kind, error.message),
    };

    task.status = TaskStatus::Syncing;
    task.error_kind = None;
    task.error_message = None;
    task.updated_at = current_unix_timestamp();
    store
        .update_task(&task)
        .map_err(|error| command_error(error.kind, &error.message))?;

    let captured_at_iso = match unix_seconds_to_rfc3339(&task.created_at) {
        Ok(value) => value,
        Err(error) => return fail_sync_task(store, task, ErrorKind::ParseFailed, error),
    };
    let synced_duration = current_duration();
    let synced_at_unix = synced_duration.as_secs().to_string();
    let synced_at_iso = match duration_to_rfc3339(synced_duration) {
        Ok(value) => value,
        Err(error) => return fail_sync_task(store, task, ErrorKind::ParseFailed, error),
    };

    let properties = build_notion_properties(&task, &analysis, &captured_at_iso, &synced_at_iso);
    let settings = match store.get_notion_settings() {
        Ok(Some(settings)) => settings,
        Ok(None) => {
            return fail_sync_task(
                store,
                task,
                ErrorKind::NotionUnauthorized,
                "尚未配置 Notion 连接，请先在设置页保存 Integration Token 和 Database ID"
                    .to_string(),
            );
        }
        Err(error) => return Err(command_error(error.kind, &error.message)),
    };
    match NotionClient::from_settings(settings).and_then(|client| client.create_page(properties)) {
        Ok(page_id) => {
            task.status = TaskStatus::Synced;
            task.notion_page_id = Some(page_id);
            task.synced_at = Some(synced_at_unix);
            task.error_kind = None;
            task.error_message = None;
        }
        Err(error) => {
            return fail_sync_task(store, task, error.kind, error.message);
        }
    }

    task.updated_at = current_unix_timestamp();
    store
        .update_task(&task)
        .map_err(|error| command_error(error.kind, &error.message))?;

    Ok(task)
}

fn retry_capture_task_blocking(store: &TaskStore, id: String) -> Result<Task, String> {
    let task = store
        .get_task(&id)
        .map_err(|error| command_error(error.kind, &error.message))?
        .ok_or_else(|| {
            command_error(ErrorKind::ReadFailed, &format!("找不到本地队列任务: {id}"))
        })?;

    match task.status {
        TaskStatus::Queued => run_and_sync_capture_task_blocking(store, id),
        TaskStatus::Failed if task.analysis_json.is_none() => {
            run_and_sync_capture_task_blocking(store, id)
        }
        TaskStatus::Analyzed | TaskStatus::Failed => sync_capture_task_blocking(store, id),
        TaskStatus::Synced => Ok(task),
        TaskStatus::Reading | TaskStatus::Analyzing | TaskStatus::Syncing => Err(command_error(
            ErrorKind::ReadFailed,
            "任务仍在处理中；长时间无变化时会自动恢复为失败后可重试",
        )),
    }
}

fn sync_pending_analyzed_tasks_blocking(store: &TaskStore) -> Result<Vec<Task>, String> {
    let tasks = store
        .list_pending_sync_tasks()
        .map_err(|error| command_error(error.kind, &error.message))?;
    let mut updated_tasks = Vec::with_capacity(tasks.len());

    for task in tasks {
        updated_tasks.push(sync_capture_task_blocking(store, task.id)?);
    }

    Ok(updated_tasks)
}

#[tauri::command]
fn set_compact_mode(window: tauri::WebviewWindow, compact: bool) -> Result<(), String> {
    if compact {
        // “缩小到系统菜单栏”不是渲染一个伪窗口，而是隐藏主窗口并保持 Tauri
        // 进程存活；后续全局快捷键监听可以挂在这个常驻进程上。
        window
            .hide()
            .map_err(|error| format!("无法隐藏主窗口: {error}"))?;
    } else {
        // 保留给后续原生菜单/全局快捷键直接恢复窗口使用。
        restore_main_window(&window)?;
    }

    Ok(())
}

fn restore_main_window(window: &tauri::WebviewWindow) -> Result<(), String> {
    window
        .show()
        .map_err(|error| format!("无法显示主窗口: {error}"))?;
    let _ = window.unminimize();
    let _ = window.set_focus();

    Ok(())
}

fn restore_main_window_from_app(app: &tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "找不到主窗口 main".to_string())?;
    restore_main_window(&window)
}

pub fn run() {
    let app = tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir().map_err(|error| {
                std::io::Error::other(format!("无法读取 app data 目录: {error}"))
            })?;
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join("reachnote.db");
            let store = TaskStore::open(&db_path)?;
            // Arc 注入：阻塞命令要把 store 克隆进 spawn_blocking 闭包（'static + Send），
            // 同步命令仍可经 State -> Arc -> TaskStore 自动解引用调用。
            app.manage(Arc::new(store));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            shell_status,
            create_capture_task,
            list_capture_tasks,
            recover_interrupted_tasks,
            run_capture_task,
            retry_capture_task,
            sync_pending_analyzed_tasks,
            sync_capture_task,
            get_notion_settings,
            save_notion_settings,
            test_notion_connection,
            set_compact_mode
        ])
        .build(tauri::generate_context!())
        .expect("failed to build ReachNote");

    app.run(|app_handle, event| {
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen {
            has_visible_windows: false,
            ..
        } = event
        {
            let _ = restore_main_window_from_app(app_handle);
        }

        #[cfg(not(target_os = "macos"))]
        let _ = (app_handle, event);
    });
}

fn command_error(kind: ErrorKind, message: &str) -> String {
    format!("{message} ({})", kind.as_str())
}

fn require_notion_settings(store: &TaskStore) -> Result<NotionSettings, String> {
    store
        .get_notion_settings()
        .map_err(|error| command_error(error.kind, &error.message))?
        .ok_or_else(|| {
            command_error(
                ErrorKind::NotionUnauthorized,
                "尚未配置 Notion 连接，请先在设置页保存 Integration Token 和 Database ID",
            )
        })
}

fn task_can_sync(task: &Task) -> bool {
    task.status == TaskStatus::Analyzed
        || task.status == TaskStatus::Syncing
        || (task.status == TaskStatus::Failed && task.analysis_json.is_some())
}

fn default_stale_task_seconds() -> u64 {
    env::var("REACHNOTE_STALE_TASK_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(300)
}

fn fail_sync_task(
    store: &TaskStore,
    mut task: Task,
    kind: ErrorKind,
    message: String,
) -> Result<Task, String> {
    task.status = TaskStatus::Failed;
    task.error_kind = Some(kind);
    task.error_message = Some(message);
    task.updated_at = current_unix_timestamp();
    store
        .update_task(&task)
        .map_err(|error| command_error(error.kind, &error.message))?;
    Ok(task)
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

fn unix_seconds_to_rfc3339(value: &str) -> Result<String, String> {
    let seconds = value
        .parse::<i64>()
        .map_err(|error| format!("任务时间戳不是合法 unix 秒，无法写入 Notion date: {error}"))?;
    OffsetDateTime::from_unix_timestamp(seconds)
        .map_err(|error| format!("任务时间戳超出 Notion date 可写范围: {error}"))?
        .format(&Rfc3339)
        .map_err(|error| format!("无法格式化 Notion date: {error}"))
}

fn duration_to_rfc3339(duration: Duration) -> Result<String, String> {
    OffsetDateTime::from_unix_timestamp(duration.as_secs() as i64)
        .map_err(|error| format!("当前时间超出 Notion date 可写范围: {error}"))?
        .format(&Rfc3339)
        .map_err(|error| format!("无法格式化 Notion date: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_store() -> (TaskStore, std::path::PathBuf) {
        let db_path = env::temp_dir().join(format!(
            "reachnote-retry-test-{}-{}.db",
            std::process::id(),
            create_task_id()
        ));
        let store = TaskStore::open(&db_path).expect("open temp store");
        (store, db_path)
    }

    fn sample_task(id: &str, status: TaskStatus) -> Task {
        let validated = validate_article_url("https://example.com/article").unwrap();
        Task {
            id: id.to_string(),
            url: validated.url,
            source_type: "article".to_string(),
            template_id: "article".to_string(),
            status,
            title: None,
            source_domain: Some(validated.source_domain),
            score: None,
            model: Some(ProviderId::ClaudeCli.label().to_string()),
            provider_id: ProviderId::ClaudeCli.as_str().to_string(),
            note: None,
            analysis_json: None,
            notion_page_id: None,
            error_kind: None,
            error_message: None,
            created_at: "100".to_string(),
            updated_at: "100".to_string(),
            synced_at: None,
        }
    }

    fn valid_analysis_json() -> String {
        serde_json::json!({
            "title": "可同步研究卡",
            "summary": "用于验证重试调度会走同步路径。",
            "key_points": ["保留本地研究卡", "不重新读取网页", "同步失败仍可重试"],
            "tags": ["Retry"],
            "score": 4,
            "next_action": "继续验证",
            "model": "fake-retry-model"
        })
        .to_string()
    }

    #[test]
    fn retry_rejects_active_processing_task() {
        let (store, db_path) = temp_store();
        let task = sample_task("processing", TaskStatus::Analyzing);
        store.insert_task(&task).unwrap();

        let error = retry_capture_task_blocking(&store, task.id.clone()).unwrap_err();
        assert!(error.contains("任务仍在处理中"));
        let loaded = store.get_task(&task.id).unwrap().unwrap();
        assert_eq!(loaded.status, TaskStatus::Analyzing);

        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn retry_failed_task_with_analysis_uses_sync_path() {
        let (store, db_path) = temp_store();
        let mut task = sample_task("sync-retry", TaskStatus::Failed);
        task.title = Some("可同步研究卡".to_string());
        task.score = Some(4);
        task.model = Some("fake-retry-model".to_string());
        task.analysis_json = Some(valid_analysis_json());
        task.error_kind = Some(ErrorKind::NetworkFailed);
        task.error_message = Some("上一轮同步失败".to_string());
        store.insert_task(&task).unwrap();

        let updated = retry_capture_task_blocking(&store, task.id.clone()).unwrap();
        assert_eq!(updated.status, TaskStatus::Failed);
        assert_eq!(updated.error_kind, Some(ErrorKind::NotionUnauthorized));
        assert!(updated.analysis_json.is_some());
        assert!(updated
            .error_message
            .as_deref()
            .unwrap()
            .contains("尚未配置 Notion 连接"));

        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn sync_pending_analyzed_tasks_attempts_orphaned_analysis() {
        let (store, db_path) = temp_store();
        let mut pending = sample_task("pending-sync", TaskStatus::Analyzed);
        pending.title = Some("待补同步研究卡".to_string());
        pending.score = Some(4);
        pending.model = Some("fake-analysis-model".to_string());
        pending.analysis_json = Some(valid_analysis_json());
        let mut already_synced = sample_task("already-synced", TaskStatus::Synced);
        already_synced.title = Some("已同步研究卡".to_string());
        already_synced.score = Some(4);
        already_synced.model = Some("fake-analysis-model".to_string());
        already_synced.analysis_json = Some(valid_analysis_json());
        already_synced.notion_page_id = Some("notion-page-id".to_string());
        let queued = sample_task("queued", TaskStatus::Queued);

        store.insert_task(&pending).unwrap();
        store.insert_task(&already_synced).unwrap();
        store.insert_task(&queued).unwrap();

        let updated = sync_pending_analyzed_tasks_blocking(&store).unwrap();
        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0].id, pending.id);
        assert_eq!(updated[0].status, TaskStatus::Failed);
        assert_eq!(updated[0].error_kind, Some(ErrorKind::NotionUnauthorized));
        assert!(updated[0].analysis_json.is_some());

        assert_eq!(
            store.get_task("already-synced").unwrap().unwrap().status,
            TaskStatus::Synced
        );
        assert_eq!(
            store.get_task("queued").unwrap().unwrap().status,
            TaskStatus::Queued
        );

        let _ = fs::remove_file(db_path);
    }
}

#[cfg(test)]
mod real_e2e_tests {
    use super::*;
    use std::env;
    use std::fs;

    #[test]
    #[ignore = "writes a real Notion page and calls the real Claude CLI"]
    fn real_e2e_fe_fidelity_kit_claude_to_notion() {
        let token = env::var("NOTION_TOKEN").expect("NOTION_TOKEN is required");
        let database_id = env::var("NOTION_DATABASE_ID").expect("NOTION_DATABASE_ID is required");
        let version = env::var("NOTION_VERSION").unwrap_or_else(|_| NOTION_API_VERSION.to_string());
        let db_path = env::temp_dir().join(format!(
            "reachnote-real-e2e-{}-{}.db",
            std::process::id(),
            current_unix_timestamp()
        ));
        let store = TaskStore::open(&db_path).expect("open temp store");
        store
            .save_notion_settings(
                &NotionSettings {
                    token,
                    database_id,
                    version,
                },
                &current_unix_timestamp(),
            )
            .expect("save notion settings");

        let validated =
            validate_article_url("https://github.com/AliceDel66/fe-fidelity-kit").unwrap();
        let timestamp = current_unix_timestamp();
        let mut task = Task {
            id: format!("real-e2e-{}", timestamp),
            url: validated.url,
            source_type: "article".to_string(),
            template_id: "article".to_string(),
            status: TaskStatus::Queued,
            title: None,
            source_domain: Some(validated.source_domain),
            score: None,
            model: Some(ProviderId::ClaudeCli.label().to_string()),
            provider_id: ProviderId::ClaudeCli.as_str().to_string(),
            note: Some(
                "真实 E2E：基于 AliceDel66/fe-fidelity-kit 仓库页面生成研究卡。".to_string(),
            ),
            analysis_json: None,
            notion_page_id: None,
            error_kind: None,
            error_message: None,
            created_at: timestamp.clone(),
            updated_at: timestamp,
            synced_at: None,
        };
        store.insert_task(&task).expect("insert task");

        task.status = TaskStatus::Reading;
        task.updated_at = current_unix_timestamp();
        store.update_task(&task).expect("mark reading");
        let content = AgentReachWebReader::from_env()
            .read_article(&task.url)
            .expect("read real GitHub repo page");
        assert!(
            content.text.contains("fe-fidelity-kit") || content.text.contains("Fidelity"),
            "reader content should contain the target repo identity"
        );

        task.status = TaskStatus::Analyzing;
        task.updated_at = current_unix_timestamp();
        store.update_task(&task).expect("mark analyzing");
        let request = AnalysisRequest {
            url: task.url.clone(),
            source_type: task.source_type.clone(),
            source_domain: task.source_domain.clone(),
            template_id: task.template_id.clone(),
            note: task.note.clone(),
            content_text: Some(content.text),
            content_reader: Some(content.reader),
        };
        let analysis = ProviderRunner::from_env()
            .analyze(ProviderId::ClaudeCli, &request)
            .expect("analyze with real Claude CLI");
        assert!(
            !analysis.model.to_ascii_lowercase().contains("fake"),
            "real E2E must not use fake model data"
        );

        task.status = TaskStatus::Analyzed;
        task.title = Some(analysis.title.clone());
        task.score = Some(analysis.score);
        task.model = Some(analysis.model.clone());
        task.analysis_json = Some(serde_json::to_string(&analysis).unwrap());
        task.updated_at = current_unix_timestamp();
        store.update_task(&task).expect("store analysis");

        task.status = TaskStatus::Syncing;
        task.error_kind = None;
        task.error_message = None;
        task.updated_at = current_unix_timestamp();
        store.update_task(&task).expect("mark syncing");
        let captured_at_iso = unix_seconds_to_rfc3339(&task.created_at).unwrap();
        let synced_duration = current_duration();
        let synced_at_unix = synced_duration.as_secs().to_string();
        let synced_at_iso = duration_to_rfc3339(synced_duration).unwrap();
        let settings = store.get_notion_settings().unwrap().unwrap();
        let properties =
            build_notion_properties(&task, &analysis, &captured_at_iso, &synced_at_iso);
        let page_id = NotionClient::from_settings(settings)
            .and_then(|client| client.create_page(properties))
            .expect("create real Notion page");
        assert!(!page_id.trim().is_empty());

        task.status = TaskStatus::Synced;
        task.notion_page_id = Some(page_id.clone());
        task.synced_at = Some(synced_at_unix);
        task.updated_at = current_unix_timestamp();
        store.update_task(&task).expect("store synced task");

        let loaded = store.get_task(&task.id).unwrap().unwrap();
        assert_eq!(loaded.status, TaskStatus::Synced);
        assert_eq!(loaded.notion_page_id.as_deref(), Some(page_id.as_str()));
        println!("REAL_E2E_PAGE_ID={page_id}");
        let _ = fs::remove_file(db_path);
    }
}
