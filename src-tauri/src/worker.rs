use std::env;
use std::panic::{self, AssertUnwindSafe};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;

use reachnote_core::analysis::{parse_analysis_result, AnalysisRequest, ProviderId};
use reachnote_core::notion::build_notion_properties;
use reachnote_core::task::{ErrorKind, Task, TaskStatus};
use tauri::Emitter;

use crate::notion::NotionClient;
use crate::provider::ProviderRunner;
use crate::reader::AgentReachWebReader;
use crate::store::TaskStore;
use crate::{
    command_error, current_duration, current_unix_timestamp, duration_to_rfc3339,
    unix_seconds_to_rfc3339,
};

const TASK_UPDATED_EVENT: &str = "task:updated";
const WORKER_ERROR_EVENT: &str = "worker:error";

#[derive(Clone)]
pub(crate) struct WorkerNotifier {
    sender: Sender<()>,
}

impl WorkerNotifier {
    pub(crate) fn new(sender: Sender<()>) -> Self {
        Self { sender }
    }

    pub(crate) fn notify(&self) {
        if let Err(error) = self.sender.send(()) {
            eprintln!("[worker] notify failed: {error}");
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TickOutcome {
    Processed,
    Idle,
}

pub(crate) fn channel() -> (WorkerNotifier, Receiver<()>) {
    let (sender, receiver) = mpsc::channel();
    (WorkerNotifier::new(sender), receiver)
}

pub(crate) fn start_worker(
    store: Arc<TaskStore>,
    receiver: Receiver<()>,
    app_handle: tauri::AppHandle,
) {
    std::thread::spawn(move || worker_loop(store, receiver, app_handle));
}

pub(crate) fn run_capture_task_inline(
    store: &TaskStore,
    id: String,
    emit: &dyn Fn(&Task) -> Result<(), String>,
) -> Result<Task, String> {
    let now = current_unix_timestamp();
    match store
        .claim_task(&id, TaskStatus::Queued, TaskStatus::Reading, &now)
        .map_err(|error| command_error(error.kind, &error.message))?
    {
        Some(task) => {
            eprintln!("[worker] claim queued task {}: queued -> reading", task.id);
            emit_task_update(emit, &task);
            run_claimed_capture_task(store, task, emit)
        }
        None => {
            eprintln!("[worker] claim queued task {id} failed");
            store
                .get_task(&id)
                .map_err(|error| command_error(error.kind, &error.message))?
                .ok_or_else(|| {
                    command_error(ErrorKind::ReadFailed, &format!("找不到本地队列任务: {id}"))
                })
        }
    }
}

pub(crate) fn retry_capture_task_blocking(store: &TaskStore, id: String) -> Result<Task, String> {
    let mut task = store
        .get_task(&id)
        .map_err(|error| command_error(error.kind, &error.message))?
        .ok_or_else(|| {
            command_error(ErrorKind::ReadFailed, &format!("找不到本地队列任务: {id}"))
        })?;

    match task.status {
        TaskStatus::Queued | TaskStatus::Analyzed | TaskStatus::Synced => Ok(task),
        TaskStatus::Failed if task.analysis_json.is_none() => {
            let previous_status = task.status;
            task.status = TaskStatus::Queued;
            task.title = None;
            task.score = None;
            task.analysis_json = None;
            task.notion_page_id = None;
            task.synced_at = None;
            task.error_kind = None;
            task.error_message = None;
            task.updated_at = current_unix_timestamp();
            match store
                .update_task_if_status(&task, previous_status)
                .map_err(|error| command_error(error.kind, &error.message))?
            {
                Some(updated) => {
                    eprintln!("[worker] retry reset {}: failed -> queued", updated.id);
                    Ok(updated)
                }
                None => {
                    eprintln!("[worker] retry reset {} failed CAS", task.id);
                    current_task(store, &task.id)
                }
            }
        }
        TaskStatus::Failed => {
            let previous_status = task.status;
            task.status = TaskStatus::Analyzed;
            task.error_kind = None;
            task.error_message = None;
            task.updated_at = current_unix_timestamp();
            match store
                .update_task_if_status(&task, previous_status)
                .map_err(|error| command_error(error.kind, &error.message))?
            {
                Some(updated) => {
                    eprintln!("[worker] retry reset {}: failed -> analyzed", updated.id);
                    Ok(updated)
                }
                None => {
                    eprintln!("[worker] retry reset {} failed CAS", task.id);
                    current_task(store, &task.id)
                }
            }
        }
        TaskStatus::Reading | TaskStatus::Analyzing | TaskStatus::Syncing => Err(command_error(
            ErrorKind::ReadFailed,
            "任务仍在处理中；长时间无变化时会自动恢复为失败后可重试",
        )),
    }
}

pub(crate) fn sync_capture_task_inline(
    store: &TaskStore,
    id: String,
    emit: &dyn Fn(&Task) -> Result<(), String>,
) -> Result<Task, String> {
    let task = current_task(store, &id)?;
    if task.notion_page_id.is_some()
        && matches!(
            task.status,
            TaskStatus::Analyzed | TaskStatus::Failed | TaskStatus::Syncing
        )
    {
        let claimed = if task.status == TaskStatus::Syncing {
            task
        } else {
            match store
                .claim_task(
                    &id,
                    task.status,
                    TaskStatus::Syncing,
                    &current_unix_timestamp(),
                )
                .map_err(|error| command_error(error.kind, &error.message))?
            {
                Some(claimed) => claimed,
                None => return current_task(store, &id),
            }
        };
        return finalize_claimed_sync_task(store, claimed, emit);
    }

    if task.status != TaskStatus::Analyzed {
        return Err(command_error(
            ErrorKind::SchemaMismatch,
            "只能同步已分析任务；分析失败或尚未生成研究卡的任务需要先重试分析",
        ));
    }

    let claimed = match store
        .claim_task(
            &id,
            TaskStatus::Analyzed,
            TaskStatus::Syncing,
            &current_unix_timestamp(),
        )
        .map_err(|error| command_error(error.kind, &error.message))?
    {
        Some(claimed) => claimed,
        None => return current_task(store, &id),
    };

    eprintln!("[worker] manual sync claim {}: analyzed -> syncing", id);
    emit_task_update(emit, &claimed);
    sync_claimed_task(store, claimed, emit)
}

pub(crate) fn sync_pending_analyzed_tasks_blocking(
    store: &TaskStore,
    emit: &dyn Fn(&Task) -> Result<(), String>,
) -> Result<Vec<Task>, String> {
    let mut updated_tasks = Vec::new();

    loop {
        let now = current_unix_timestamp();
        if let Some(task) = store
            .claim_next_finalization_task(&now)
            .map_err(|error| command_error(error.kind, &error.message))?
        {
            eprintln!("[worker] manual pending sync finalizes {}", task.id);
            emit_task_update(emit, &task);
            updated_tasks.push(finalize_claimed_sync_task(store, task, emit)?);
            continue;
        }

        if let Some(task) = store
            .fail_next_analyzed_without_result(&now)
            .map_err(|error| command_error(error.kind, &error.message))?
        {
            eprintln!(
                "[worker] manual pending sync failed invalid analyzed {}",
                task.id
            );
            emit_task_update(emit, &task);
            updated_tasks.push(task);
            continue;
        }

        let task = match store.claim_next_pending_sync_task(&now) {
            Ok(Some(task)) => task,
            Ok(None) => break,
            Err(error) => {
                eprintln!(
                    "[worker] manual pending sync store error: {}",
                    error.message
                );
                break;
            }
        };

        eprintln!("[worker] manual pending sync claim {}", task.id);
        emit_task_update(emit, &task);
        match sync_claimed_task(store, task, emit) {
            Ok(task) => updated_tasks.push(task),
            Err(error) => {
                eprintln!("[worker] manual pending sync task failed: {error}");
                break;
            }
        }
    }

    Ok(updated_tasks)
}

pub(crate) fn recover_interrupted_tasks_blocking(
    store: &TaskStore,
    emit: &dyn Fn(&Task) -> Result<(), String>,
) -> Result<Vec<Task>, String> {
    let recovered = store
        .recover_stale_processing_tasks(&current_unix_timestamp(), effective_stale_task_seconds())
        .map_err(|error| command_error(error.kind, &error.message))?;
    for task in &recovered {
        eprintln!("[worker] recovered stale task {}", task.id);
        emit_task_update(emit, task);
    }
    Ok(recovered)
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn worker_tick(
    store: &TaskStore,
    emit: &dyn Fn(&Task) -> Result<(), String>,
) -> TickOutcome {
    match worker_tick_result(store, emit) {
        Ok(outcome) => outcome,
        Err(error) => {
            eprintln!("[worker] tick failed: {error}");
            TickOutcome::Idle
        }
    }
}

pub(crate) fn effective_stale_task_seconds() -> u64 {
    let configured = positive_env_u64("REACHNOTE_STALE_TASK_SECS").unwrap_or(300);
    let max_timeout = [
        positive_env_u64("REACHNOTE_AI_TIMEOUT_SECS").unwrap_or(120),
        positive_env_u64("REACHNOTE_READER_TIMEOUT_SECS").unwrap_or(30),
        positive_env_u64("REACHNOTE_NOTION_TIMEOUT_SECS").unwrap_or(30),
    ]
    .into_iter()
    .max()
    .unwrap_or(120)
        + 60;

    configured.max(max_timeout)
}

fn worker_idle_seconds() -> u64 {
    positive_env_u64("REACHNOTE_WORKER_IDLE_SECS").unwrap_or(30)
}

fn worker_loop(store: Arc<TaskStore>, receiver: Receiver<()>, app_handle: tauri::AppHandle) {
    let mut consecutive_errors = 0_u32;
    let emit = |task: &Task| emit_task_event(&app_handle, task);

    match recover_interrupted_tasks_blocking(&store, &emit) {
        Ok(_) => consecutive_errors = 0,
        Err(error) => record_worker_error(&app_handle, &mut consecutive_errors, error),
    }

    loop {
        match receiver.recv_timeout(Duration::from_secs(worker_idle_seconds())) {
            Ok(()) | Err(mpsc::RecvTimeoutError::Timeout) => loop {
                match catch_worker_tick(|| worker_tick_result(&store, &emit)) {
                    Ok(TickOutcome::Processed) => {
                        consecutive_errors = 0;
                    }
                    Ok(TickOutcome::Idle) => {
                        consecutive_errors = 0;
                        break;
                    }
                    Err(error) => {
                        record_worker_error(&app_handle, &mut consecutive_errors, error);
                        break;
                    }
                }
            },
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                eprintln!("[worker] notifier disconnected; worker exits");
                break;
            }
        }
    }
}

fn catch_worker_tick(
    tick: impl FnOnce() -> Result<TickOutcome, String>,
) -> Result<TickOutcome, String> {
    match panic::catch_unwind(AssertUnwindSafe(tick)) {
        Ok(result) => result,
        Err(_) => Err("worker tick panic".to_string()),
    }
}

fn worker_tick_result(
    store: &TaskStore,
    emit: &dyn Fn(&Task) -> Result<(), String>,
) -> Result<TickOutcome, String> {
    let now = current_unix_timestamp();
    if let Some(task) = store
        .claim_next_queued_task(&now)
        .map_err(|error| command_error(error.kind, &error.message))?
    {
        eprintln!("[worker] claim queued task {}: queued -> reading", task.id);
        emit_task_update(emit, &task);
        run_claimed_capture_task(store, task, emit)?;
        return Ok(TickOutcome::Processed);
    }

    if let Some(task) = store
        .claim_next_finalization_task(&now)
        .map_err(|error| command_error(error.kind, &error.message))?
    {
        eprintln!("[worker] claim finalization task {}", task.id);
        emit_task_update(emit, &task);
        finalize_claimed_sync_task(store, task, emit)?;
        return Ok(TickOutcome::Processed);
    }

    if let Some(task) = store
        .fail_next_analyzed_without_result(&now)
        .map_err(|error| command_error(error.kind, &error.message))?
    {
        eprintln!("[worker] analyzed task {} has no analysis_json", task.id);
        emit_task_update(emit, &task);
        return Ok(TickOutcome::Processed);
    }

    if let Some(task) = store
        .claim_next_pending_sync_task(&now)
        .map_err(|error| command_error(error.kind, &error.message))?
    {
        eprintln!("[worker] claim sync task {}: analyzed -> syncing", task.id);
        emit_task_update(emit, &task);
        sync_claimed_task(store, task, emit)?;
        return Ok(TickOutcome::Processed);
    }

    Ok(TickOutcome::Idle)
}

fn run_claimed_capture_task(
    store: &TaskStore,
    mut task: Task,
    emit: &dyn Fn(&Task) -> Result<(), String>,
) -> Result<Task, String> {
    let content = match AgentReachWebReader::from_env().read_article(&task.url) {
        Ok(content) => content,
        Err(error) => {
            return fail_task_if_status(
                store,
                task,
                TaskStatus::Reading,
                error.kind,
                error.message,
                emit,
            );
        }
    };

    task.status = TaskStatus::Analyzing;
    task.updated_at = current_unix_timestamp();
    task = update_task_if_status(
        store,
        task,
        TaskStatus::Reading,
        emit,
        "reading -> analyzing",
    )?;

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
            task.updated_at = current_unix_timestamp();
            task = update_task_if_status(
                store,
                task,
                TaskStatus::Analyzing,
                emit,
                "analyzing -> analyzed",
            )?;
            sync_capture_task_inline(store, task.id.clone(), emit)
        }
        Err(error) => fail_task_if_status(
            store,
            task,
            TaskStatus::Analyzing,
            error.kind,
            error.message,
            emit,
        ),
    }
}

fn sync_claimed_task(
    store: &TaskStore,
    mut task: Task,
    emit: &dyn Fn(&Task) -> Result<(), String>,
) -> Result<Task, String> {
    let analysis_json = match task.analysis_json.as_deref() {
        Some(value) => value,
        None => {
            return fail_task_if_status(
                store,
                task,
                TaskStatus::Syncing,
                ErrorKind::ParseFailed,
                "任务缺少结构化研究卡，无法同步到 Notion".to_string(),
                emit,
            );
        }
    };

    let analysis = match parse_analysis_result(analysis_json) {
        Ok(analysis) => analysis,
        Err(error) => {
            return fail_task_if_status(
                store,
                task,
                TaskStatus::Syncing,
                error.kind,
                error.message,
                emit,
            );
        }
    };

    let captured_at_iso = match unix_seconds_to_rfc3339(&task.created_at) {
        Ok(value) => value,
        Err(error) => {
            return fail_task_if_status(
                store,
                task,
                TaskStatus::Syncing,
                ErrorKind::ParseFailed,
                error,
                emit,
            );
        }
    };
    let synced_duration = current_duration();
    let synced_at_unix = synced_duration.as_secs().to_string();
    let synced_at_iso = match duration_to_rfc3339(synced_duration) {
        Ok(value) => value,
        Err(error) => {
            return fail_task_if_status(
                store,
                task,
                TaskStatus::Syncing,
                ErrorKind::ParseFailed,
                error,
                emit,
            );
        }
    };

    let properties = build_notion_properties(&task, &analysis, &captured_at_iso, &synced_at_iso);
    let settings = match store.get_notion_settings() {
        Ok(Some(settings)) => settings,
        Ok(None) => {
            return fail_task_if_status(
                store,
                task,
                TaskStatus::Syncing,
                ErrorKind::NotionUnauthorized,
                "尚未配置 Notion 连接，请先在设置页保存 Integration Token 和 Database ID"
                    .to_string(),
                emit,
            );
        }
        Err(error) => return Err(command_error(error.kind, &error.message)),
    };

    let page_id = match NotionClient::from_settings(settings)
        .and_then(|client| client.create_page(properties))
    {
        Ok(page_id) => page_id,
        Err(error) => {
            return fail_task_if_status(
                store,
                task,
                TaskStatus::Syncing,
                error.kind,
                error.message,
                emit,
            );
        }
    };

    task.notion_page_id = Some(page_id);
    task.error_kind = None;
    task.error_message = None;
    task.updated_at = current_unix_timestamp();
    let task = update_task_if_status(
        store,
        task,
        TaskStatus::Syncing,
        emit,
        "syncing write notion_page_id",
    )?;
    finalize_syncing_task(store, task, synced_at_unix, emit)
}

fn finalize_claimed_sync_task(
    store: &TaskStore,
    task: Task,
    emit: &dyn Fn(&Task) -> Result<(), String>,
) -> Result<Task, String> {
    finalize_syncing_task(store, task, current_unix_timestamp(), emit)
}

fn finalize_syncing_task(
    store: &TaskStore,
    task: Task,
    synced_at: String,
    emit: &dyn Fn(&Task) -> Result<(), String>,
) -> Result<Task, String> {
    match store
        .finalize_synced_task(&task.id, &synced_at, &current_unix_timestamp())
        .map_err(|error| command_error(error.kind, &error.message))?
    {
        Some(updated) => {
            eprintln!("[worker] finalize task {}: syncing -> synced", updated.id);
            emit_task_update(emit, &updated);
            Ok(updated)
        }
        None => {
            eprintln!("[worker] finalize task {} failed CAS", task.id);
            current_task(store, &task.id)
        }
    }
}

fn fail_task_if_status(
    store: &TaskStore,
    mut task: Task,
    expected_status: TaskStatus,
    kind: ErrorKind,
    message: String,
    emit: &dyn Fn(&Task) -> Result<(), String>,
) -> Result<Task, String> {
    task.status = TaskStatus::Failed;
    task.error_kind = Some(kind);
    task.error_message = Some(message);
    task.updated_at = current_unix_timestamp();
    update_task_if_status(store, task, expected_status, emit, "task -> failed")
}

fn update_task_if_status(
    store: &TaskStore,
    task: Task,
    expected_status: TaskStatus,
    emit: &dyn Fn(&Task) -> Result<(), String>,
    label: &str,
) -> Result<Task, String> {
    match store
        .update_task_if_status(&task, expected_status)
        .map_err(|error| command_error(error.kind, &error.message))?
    {
        Some(updated) => {
            eprintln!(
                "[worker] transition {} {} from {}",
                updated.id,
                label,
                expected_status.as_str()
            );
            emit_task_update(emit, &updated);
            Ok(updated)
        }
        None => {
            eprintln!(
                "[worker] transition {} {} failed CAS from {}",
                task.id,
                label,
                expected_status.as_str()
            );
            current_task(store, &task.id)
        }
    }
}

fn current_task(store: &TaskStore, id: &str) -> Result<Task, String> {
    store
        .get_task(id)
        .map_err(|error| command_error(error.kind, &error.message))?
        .ok_or_else(|| command_error(ErrorKind::ReadFailed, &format!("找不到本地队列任务: {id}")))
}

fn emit_task_update(emit: &dyn Fn(&Task) -> Result<(), String>, task: &Task) {
    if let Err(error) = emit(task) {
        eprintln!("[worker] emit task:updated failed for {}: {error}", task.id);
    }
}

fn emit_task_event(app_handle: &tauri::AppHandle, task: &Task) -> Result<(), String> {
    app_handle
        .emit(TASK_UPDATED_EVENT, task.clone())
        .map_err(|error| error.to_string())
}

fn record_worker_error(app_handle: &tauri::AppHandle, consecutive_errors: &mut u32, error: String) {
    *consecutive_errors += 1;
    eprintln!("[worker] error {}/3: {}", *consecutive_errors, error);
    if *consecutive_errors >= 3 {
        if let Err(emit_error) = app_handle.emit(WORKER_ERROR_EVENT, error.clone()) {
            eprintln!("[worker] emit worker:error failed: {emit_error}");
        }
    }
}

fn positive_env_u64(key: &str) -> Option<u64> {
    env::var(key)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use reachnote_core::analysis::ProviderId;
    use reachnote_core::task::validate_article_url;
    use std::cell::RefCell;
    use std::fs;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct EnvOverride {
        key: &'static str,
        previous: Option<std::ffi::OsString>,
    }

    impl EnvOverride {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = env::var_os(key);
            env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvOverride {
        fn drop(&mut self) {
            if let Some(previous) = self.previous.take() {
                env::set_var(self.key, previous);
            } else {
                env::remove_var(self.key);
            }
        }
    }

    fn temp_store() -> (TaskStore, std::path::PathBuf) {
        let db_path = env::temp_dir().join(format!(
            "reachnote-worker-test-{}-{}.db",
            std::process::id(),
            current_duration().as_nanos()
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
            "title": "研究卡",
            "summary": "用于 worker 状态机测试的结构化结果。",
            "key_points": ["保持本地状态一致"],
            "tags": ["Worker"],
            "score": 4,
            "next_action": "继续验证",
            "model": "fake-worker-model"
        })
        .to_string()
    }

    #[test]
    fn worker_tick_claims_queued_orphan_and_fails_read_once() {
        let _guard = ENV_LOCK.lock().unwrap();
        let _base_url = EnvOverride::set(
            "REACHNOTE_AGENT_REACH_WEB_READER_BASE_URL",
            "http://127.0.0.1:1",
        );
        let _timeout = EnvOverride::set("REACHNOTE_READER_TIMEOUT_SECS", "1");
        let (store, db_path) = temp_store();
        let task = sample_task("queued", TaskStatus::Queued);
        store.insert_task(&task).unwrap();
        let emitted = RefCell::new(Vec::new());

        let outcome = worker_tick(&store, &|task| {
            emitted.borrow_mut().push((task.id.clone(), task.status));
            Ok(())
        });

        assert_eq!(outcome, TickOutcome::Processed);
        let loaded = store.get_task("queued").unwrap().unwrap();
        assert_eq!(loaded.status, TaskStatus::Failed);
        assert_eq!(loaded.error_kind, Some(ErrorKind::NetworkFailed));
        let emitted = emitted.borrow();
        assert!(emitted
            .iter()
            .any(|(_, status)| *status == TaskStatus::Reading));
        assert!(emitted
            .iter()
            .any(|(_, status)| *status == TaskStatus::Failed));
        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn worker_tick_finalizes_failed_task_with_page_id_without_create_page() {
        let (store, db_path) = temp_store();
        let mut task = sample_task("finalize-failed", TaskStatus::Failed);
        task.analysis_json = Some(valid_analysis_json());
        task.notion_page_id = Some("page-id".to_string());
        task.error_kind = Some(ErrorKind::NetworkFailed);
        task.error_message = Some("create_page 已成功但落盘后崩溃".to_string());
        store.insert_task(&task).unwrap();

        let outcome = worker_tick(&store, &|_task| Ok(()));

        assert_eq!(outcome, TickOutcome::Processed);
        let loaded = store.get_task("finalize-failed").unwrap().unwrap();
        assert_eq!(loaded.status, TaskStatus::Synced);
        assert_eq!(loaded.notion_page_id.as_deref(), Some("page-id"));
        assert!(loaded.synced_at.is_some());
        assert_eq!(loaded.error_kind, None);
        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn worker_tick_finalizes_syncing_task_with_page_id_after_crash() {
        let (store, db_path) = temp_store();
        let mut task = sample_task("finalize-syncing", TaskStatus::Syncing);
        task.analysis_json = Some(valid_analysis_json());
        task.notion_page_id = Some("page-id".to_string());
        store.insert_task(&task).unwrap();

        let outcome = worker_tick(&store, &|_task| Ok(()));

        assert_eq!(outcome, TickOutcome::Processed);
        let loaded = store.get_task("finalize-syncing").unwrap().unwrap();
        assert_eq!(loaded.status, TaskStatus::Synced);
        assert_eq!(loaded.notion_page_id.as_deref(), Some("page-id"));
        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn worker_tick_fails_notion_unconfigured_once() {
        let (store, db_path) = temp_store();
        let mut task = sample_task("pending-sync", TaskStatus::Analyzed);
        task.title = Some("待同步研究卡".to_string());
        task.score = Some(4);
        task.model = Some("fake-worker-model".to_string());
        task.analysis_json = Some(valid_analysis_json());
        store.insert_task(&task).unwrap();

        let first = worker_tick(&store, &|_task| Ok(()));
        let second = worker_tick(&store, &|_task| Ok(()));

        assert_eq!(first, TickOutcome::Processed);
        assert_eq!(second, TickOutcome::Idle);
        let loaded = store.get_task("pending-sync").unwrap().unwrap();
        assert_eq!(loaded.status, TaskStatus::Failed);
        assert_eq!(loaded.error_kind, Some(ErrorKind::NotionUnauthorized));
        assert!(loaded.analysis_json.is_some());
        assert!(loaded.notion_page_id.is_none());
        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn worker_tick_fails_analyzed_without_analysis_json() {
        let (store, db_path) = temp_store();
        let task = sample_task("invalid-analyzed", TaskStatus::Analyzed);
        store.insert_task(&task).unwrap();

        let outcome = worker_tick(&store, &|_task| Ok(()));

        assert_eq!(outcome, TickOutcome::Processed);
        let loaded = store.get_task("invalid-analyzed").unwrap().unwrap();
        assert_eq!(loaded.status, TaskStatus::Failed);
        assert_eq!(loaded.error_kind, Some(ErrorKind::ParseFailed));
        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn repeated_worker_ticks_drain_multiple_queued_tasks() {
        let _guard = ENV_LOCK.lock().unwrap();
        let _base_url = EnvOverride::set(
            "REACHNOTE_AGENT_REACH_WEB_READER_BASE_URL",
            "http://127.0.0.1:1",
        );
        let _timeout = EnvOverride::set("REACHNOTE_READER_TIMEOUT_SECS", "1");
        let (store, db_path) = temp_store();
        let mut first = sample_task("first", TaskStatus::Queued);
        first.created_at = "100".to_string();
        let mut second = sample_task("second", TaskStatus::Queued);
        second.created_at = "101".to_string();
        store.insert_task(&first).unwrap();
        store.insert_task(&second).unwrap();

        let mut processed = 0;
        while worker_tick(&store, &|_task| Ok(())) == TickOutcome::Processed {
            processed += 1;
        }

        assert_eq!(processed, 2);
        assert_eq!(
            store.get_task("first").unwrap().unwrap().status,
            TaskStatus::Failed
        );
        assert_eq!(
            store.get_task("second").unwrap().unwrap().status,
            TaskStatus::Failed
        );
        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn panic_in_worker_tick_is_caught_and_next_tick_can_continue() {
        let panic_result = catch_worker_tick(|| -> Result<TickOutcome, String> {
            panic!("synthetic worker panic");
        });
        assert_eq!(panic_result.unwrap_err(), "worker tick panic");

        let next_result = catch_worker_tick(|| Ok(TickOutcome::Idle)).unwrap();
        assert_eq!(next_result, TickOutcome::Idle);
    }
}
