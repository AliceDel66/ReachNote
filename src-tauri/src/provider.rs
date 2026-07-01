use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use reachnote_core::analysis::{
    build_analysis_prompt, parse_analysis_result, AnalysisRequest, AnalysisResult, ProviderId,
};
use reachnote_core::task::ErrorKind;
use serde_json::{json, Value};
use wait_timeout::ChildExt;

#[derive(Debug)]
pub struct ProviderError {
    pub kind: ErrorKind,
    pub message: String,
}

pub struct ProviderRunner {
    timeout: Duration,
}

impl ProviderRunner {
    pub fn from_env() -> Self {
        let seconds = env::var("REACHNOTE_AI_TIMEOUT_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(120);

        Self {
            timeout: Duration::from_secs(seconds),
        }
    }

    pub fn analyze(
        &self,
        provider_id: ProviderId,
        request: &AnalysisRequest,
    ) -> Result<AnalysisResult, ProviderError> {
        match provider_id {
            ProviderId::ClaudeCli => self.analyze_claude_cli(request),
            ProviderId::CodexCli => self.analyze_codex_cli(request),
            ProviderId::OpenAiCompatible => self.analyze_openai_compatible(request),
        }
    }

    fn analyze_claude_cli(
        &self,
        request: &AnalysisRequest,
    ) -> Result<AnalysisResult, ProviderError> {
        let command = provider_command("REACHNOTE_CLAUDE_CMD", "claude");
        ensure_executable("Claude CLI", "REACHNOTE_CLAUDE_CMD", &command)?;

        let prompt = build_analysis_prompt(request);
        let mut process = Command::new(&command);
        // prompt 一律走 stdin，不作为 argv 传：`--tools` 是变长参数（`<tools...>`），
        // 如果 prompt 紧跟在它后面会被一并吞进 tools 列表，导致 --print 模式收不到
        // prompt 转而阻塞等 stdin；未显式配置 stdin 时又会挂到超时（已用真实 CLI 复现）。
        process.args([
            "-p",
            "--output-format",
            "text",
            "--disable-slash-commands",
            "--safe-mode",
            "--no-session-persistence",
            "--tools",
            "",
        ]);

        let output = run_process("Claude CLI", &mut process, self.timeout, &prompt)?;
        parse_provider_output(&output)
    }

    fn analyze_codex_cli(
        &self,
        request: &AnalysisRequest,
    ) -> Result<AnalysisResult, ProviderError> {
        let command = provider_command("REACHNOTE_CODEX_CMD", "codex");
        ensure_executable("Codex CLI", "REACHNOTE_CODEX_CMD", &command)?;

        let prompt = build_analysis_prompt(request);
        // codex exec 会把 header、回显 prompt、"tokens used" 等噪音一起打到 stdout，而
        // prompt 里本身含 JSON schema 的花括号，naive 的"首个 { 到末个 }"提取会把回显
        // 和答案一并吞掉导致解析失败。用 --output-last-message 把"模型最终消息"单独落盘，
        // 只解析这份干净输出（真实 codex 0.132 实测为单行合法 JSON）。
        let output_path = codex_last_message_path();
        let mut process = Command::new(&command);
        // 关键修复：codex exec 没有 --ask-for-approval（那是交互式 TUI 的参数），
        // 传了会 `error: unexpected argument` 直接非零退出——这正是"切 Codex 必失败"的根因。
        // exec 本身就是非交互，read-only sandbox 下不会弹审批。
        // --ignore-user-config 跳过 ~/.codex/config.toml，-c mcp_servers={} 再清掉 MCP，
        // 否则会尝试连 Notion/Figma 等个人 MCP，单次分析多花几万 token 并逼近超时。
        process.args([
            "exec",
            "--skip-git-repo-check",
            "--ignore-rules",
            "--ignore-user-config",
            "-c",
            "mcp_servers={}",
            "--sandbox",
            "read-only",
            "--color",
            "never",
            "--ephemeral",
            "--output-last-message",
        ]);
        process.arg(&output_path);

        let stdout = match run_process("Codex CLI", &mut process, self.timeout, &prompt) {
            Ok(stdout) => stdout,
            Err(error) => {
                let _ = std::fs::remove_file(&output_path);
                return Err(error);
            }
        };

        // 优先用 --output-last-message 落盘的干净最终消息；文件缺失/为空时（如测试里
        // 忽略该参数、只往 stdout 打印 JSON 的 fake CLI）回退到 stdout，兼顾健壮与可测。
        let last_message = std::fs::read_to_string(&output_path)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let _ = std::fs::remove_file(&output_path);

        parse_provider_output(&last_message.unwrap_or(stdout))
    }

    fn analyze_openai_compatible(
        &self,
        request: &AnalysisRequest,
    ) -> Result<AnalysisResult, ProviderError> {
        let base_url = required_env(
            "REACHNOTE_OPENAI_BASE_URL",
            "OpenAI-compatible API base URL",
        )?;
        let model = required_env("REACHNOTE_OPENAI_MODEL", "OpenAI-compatible API model")?;
        let endpoint = chat_completions_endpoint(&base_url);
        let prompt = build_analysis_prompt(request);

        let payload = json!({
            "model": model,
            "messages": [
                {
                    "role": "system",
                    "content": "你是 ReachNote 的结构化研究卡生成器。只返回合法 JSON object。"
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.2,
            "response_format": { "type": "json_object" }
        });

        let client = reqwest::blocking::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|error| ProviderError {
                kind: ErrorKind::NetworkFailed,
                message: format!("无法初始化 OpenAI-compatible API 客户端: {error}"),
            })?;

        let mut request_builder = client.post(endpoint).json(&payload);
        if let Ok(api_key) = env::var("REACHNOTE_OPENAI_API_KEY") {
            if !api_key.trim().is_empty() {
                request_builder = request_builder.bearer_auth(api_key.trim());
            }
        }

        let response = request_builder.send().map_err(|error| ProviderError {
            kind: ErrorKind::NetworkFailed,
            message: format!("OpenAI-compatible API 请求失败: {error}"),
        })?;

        let status = response.status();
        let body = response.text().map_err(|error| ProviderError {
            kind: ErrorKind::NetworkFailed,
            message: format!("OpenAI-compatible API 响应读取失败: {error}"),
        })?;

        if !status.is_success() {
            return Err(ProviderError {
                kind: ErrorKind::ProviderUnavailable,
                message: format!(
                    "OpenAI-compatible API 返回非成功状态 {}: {}",
                    status.as_u16(),
                    truncate_for_message(&body)
                ),
            });
        }

        let content = openai_message_content(&body)?;
        parse_provider_output(&content)
    }
}

fn codex_last_message_path() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};

    // 进程 id + 进程内单调序号，避免并发/连续分析在同一 temp 目录里撞文件名。
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    env::temp_dir().join(format!(
        "reachnote-codex-last-{}-{}.json",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ))
}

fn provider_command(env_key: &str, default_command: &str) -> String {
    env::var(env_key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default_command.to_string())
}

fn required_env(env_key: &str, label: &str) -> Result<String, ProviderError> {
    env::var(env_key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ProviderError {
            kind: ErrorKind::ProviderUnavailable,
            message: format!("未配置 {label}，请设置环境变量 {env_key}。"),
        })
}

fn ensure_executable(label: &str, env_key: &str, command: &str) -> Result<PathBuf, ProviderError> {
    if is_path_like(command) {
        return executable_path(&PathBuf::from(command))
            .ok_or_else(|| missing_cli(label, env_key, command));
    }

    env::var_os("PATH")
        .and_then(|path| {
            env::split_paths(&path).find_map(|dir| executable_path(&dir.join(command)))
        })
        .ok_or_else(|| missing_cli(label, env_key, command))
}

fn missing_cli(label: &str, env_key: &str, command: &str) -> ProviderError {
    ProviderError {
        kind: ErrorKind::ProviderUnavailable,
        message: format!(
            "未找到 {label}：当前 PATH 中没有可执行的 `{command}`。请安装 {label}，或设置 {env_key} 指向可执行文件。"
        ),
    }
}

fn run_process(
    label: &str,
    process: &mut Command,
    timeout: Duration,
    stdin_input: &str,
) -> Result<String, ProviderError> {
    let mut child = process
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| ProviderError {
            kind: ErrorKind::ProviderUnavailable,
            message: format!("{label} 启动失败: {error}"),
        })?;

    // stdin 写入、stdout/stderr 读取各用独立线程：CLI 可能边读 prompt 边产生输出，
    // 若在本线程顺序 写 stdin -> 等 wait_timeout，一旦子进程先写满 stdout/stderr 的
    // OS pipe 缓冲区（macOS 约 64KB）就会相互阻塞，直到超时才失败。
    let mut stdin_pipe = child.stdin.take().expect("stdin 已配置为 piped");
    let stdin_payload = stdin_input.to_string();
    let stdin_writer = std::thread::spawn(move || {
        let _ = std::io::Write::write_all(&mut stdin_pipe, stdin_payload.as_bytes());
        // drop stdin_pipe：关闭写端，子进程读到 EOF。
    });
    let mut stdout_pipe = child.stdout.take().expect("stdout 已配置为 piped");
    let mut stderr_pipe = child.stderr.take().expect("stderr 已配置为 piped");
    let stdout_reader = std::thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = std::io::Read::read_to_end(&mut stdout_pipe, &mut buffer);
        buffer
    });
    let stderr_reader = std::thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = std::io::Read::read_to_end(&mut stderr_pipe, &mut buffer);
        buffer
    });

    let exit_status = match child.wait_timeout(timeout).map_err(|error| ProviderError {
        kind: ErrorKind::ProviderUnavailable,
        message: format!("{label} 等待失败: {error}"),
    })? {
        Some(status) => Some(status),
        None => {
            let _ = child.kill();
            let _ = child.wait();
            None
        }
    };

    // 进程退出或被 kill 后管道关闭，读/写线程自然结束（读线程读到 EOF，写线程写完或收到 EPIPE）。
    let _ = stdin_writer.join();
    let stdout_buffer = stdout_reader.join().unwrap_or_default();
    let stderr_buffer = stderr_reader.join().unwrap_or_default();

    let status = match exit_status {
        Some(status) => status,
        None => {
            return Err(ProviderError {
                kind: ErrorKind::NetworkFailed,
                message: format!(
                    "{label} 在 {} 秒内没有返回结果，已停止本次分析。",
                    timeout.as_secs()
                ),
            });
        }
    };

    if !status.success() {
        return Err(ProviderError {
            kind: ErrorKind::ProviderUnavailable,
            message: format!(
                "{label} 执行失败: {}",
                truncate_for_message(&String::from_utf8_lossy(&stderr_buffer))
            ),
        });
    }

    String::from_utf8(stdout_buffer).map_err(|error| ProviderError {
        kind: ErrorKind::ParseFailed,
        message: format!("{label} 输出不是 UTF-8 文本: {error}"),
    })
}

fn parse_provider_output(output: &str) -> Result<AnalysisResult, ProviderError> {
    parse_analysis_result(output).map_err(|error| ProviderError {
        kind: error.kind,
        message: error.message,
    })
}

fn openai_message_content(body: &str) -> Result<String, ProviderError> {
    let value: Value = serde_json::from_str(body).map_err(|error| ProviderError {
        kind: ErrorKind::ParseFailed,
        message: format!("OpenAI-compatible API 响应不是合法 JSON: {error}"),
    })?;

    value
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| ProviderError {
            kind: ErrorKind::SchemaMismatch,
            message: "OpenAI-compatible API 响应缺少 choices[0].message.content".to_string(),
        })
}

fn chat_completions_endpoint(base_url: &str) -> String {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.ends_with("/chat/completions") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/chat/completions")
    }
}

fn truncate_for_message(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= 240 {
        return trimmed.to_string();
    }

    let prefix = trimmed.chars().take(240).collect::<String>();
    format!("{prefix}...")
}

fn is_path_like(command: &str) -> bool {
    command.contains('/') || command.contains('\\') || Path::new(command).is_absolute()
}

fn executable_path(path: &Path) -> Option<PathBuf> {
    let metadata = path.metadata().ok()?;
    if metadata.is_file() && is_executable(&metadata) {
        Some(path.to_path_buf())
    } else {
        None
    }
}

#[cfg(unix)]
fn is_executable(metadata: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;

    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(metadata: &std::fs::Metadata) -> bool {
    !metadata.permissions().readonly()
}

#[cfg(test)]
mod tests {
    use super::*;
    use reachnote_core::analysis::AnalysisRequest;
    use std::fs;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Mutex, MutexGuard};
    use std::thread;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn lock_env() -> MutexGuard<'static, ()> {
        ENV_LOCK.lock().expect("provider env test lock poisoned")
    }

    fn sample_request() -> AnalysisRequest {
        AnalysisRequest {
            url: "https://example.com/article".to_string(),
            source_type: "article".to_string(),
            source_domain: Some("example.com".to_string()),
            template_id: "article".to_string(),
            note: Some("关注 provider adapter".to_string()),
            content_text: Some("Mock reader body for provider tests.".to_string()),
            content_reader: Some("Agent-Reach web / Jina Reader".to_string()),
        }
    }

    #[cfg(unix)]
    fn fake_cli(path: &Path, title: &str, model: &str) {
        use std::os::unix::fs::PermissionsExt;

        let body = format!(
            r#"#!/bin/sh
printf '%s\n' '{{"title":"{title}","summary":"基于 URL 的初步判断","key_points":["要点一","要点二","要点三"],"tags":["AI","测试"],"score":4,"next_action":"复核原文","model":"{model}"}}'
"#
        );
        fs::write(path, body).unwrap();
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).unwrap();
    }

    // 读 stdin 并把是否收到指定 marker 写进 summary，用来证明 prompt 真的通过 stdin
    // 到达子进程（回归 --tools 变长参数吞掉 argv prompt 导致的挂起 bug）。
    #[cfg(unix)]
    fn fake_cli_stdin_echo(path: &Path, marker: &str) {
        use std::os::unix::fs::PermissionsExt;

        let body = format!(
            r#"#!/bin/sh
INPUT="$(cat)"
case "$INPUT" in
  *"{marker}"*) FLAG="marker-found" ;;
  *) FLAG="marker-missing" ;;
esac
printf '{{"title":"stdin 回显","summary":"%s","key_points":["a","b","c"],"tags":["t"],"score":3,"next_action":"复核","model":"stdin-echo"}}\n' "$FLAG"
"#
        );
        fs::write(path, body).unwrap();
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).unwrap();
    }

    // 把最终 JSON 写到"最后一个 argv"（即 --output-last-message 的路径），并往 stdout
    // 打印带花括号的噪音，用来证明 codex provider 优先解析落盘文件而不是脏 stdout。
    #[cfg(unix)]
    fn fake_codex_writes_last_message(path: &Path, model: &str) {
        use std::os::unix::fs::PermissionsExt;

        let body = format!(
            r#"#!/bin/sh
for last_arg in "$@"; do :; done
printf '%s' '{{"title":"来自文件","summary":"落盘最终消息","key_points":["a","b","c"],"tags":["t"],"score":4,"next_action":"复核","model":"{model}"}}' > "$last_arg"
printf 'OpenAI Codex noise {{"title":"stdout 脏数据"}} tokens used 999\n'
"#
        );
        fs::write(path, body).unwrap();
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn codex_cli_provider_prefers_output_last_message_file() {
        let _env_guard = lock_env();
        let path = env::temp_dir().join(format!(
            "reachnote-fake-codex-lastmsg-{}",
            std::process::id()
        ));
        fake_codex_writes_last_message(&path, "file-codex");
        env::set_var("REACHNOTE_CODEX_CMD", &path);

        let result = ProviderRunner {
            timeout: Duration::from_secs(5),
        }
        .analyze(ProviderId::CodexCli, &sample_request())
        .unwrap();

        env::remove_var("REACHNOTE_CODEX_CMD");
        let _ = fs::remove_file(path);
        // 优先落盘文件：model 来自文件而非 stdout 噪音。
        assert_eq!(result.model, "file-codex");
        assert_eq!(result.title, "来自文件");
    }

    #[test]
    fn chat_completions_endpoint_accepts_v1_base_url() {
        assert_eq!(
            chat_completions_endpoint("http://localhost:1234/v1"),
            "http://localhost:1234/v1/chat/completions"
        );
    }

    #[test]
    fn chat_completions_endpoint_accepts_full_endpoint() {
        assert_eq!(
            chat_completions_endpoint("http://localhost:1234/v1/chat/completions"),
            "http://localhost:1234/v1/chat/completions"
        );
    }

    #[test]
    fn openai_message_content_reads_standard_response() {
        let content = openai_message_content(
            r#"{
              "choices": [
                {
                  "message": {
                    "content": "{\"title\":\"标题\",\"summary\":\"摘要\",\"key_points\":[\"一\"],\"tags\":[\"AI\"],\"score\":3,\"next_action\":\"复核\",\"model\":\"mock\"}"
                  }
                }
              ]
            }"#,
        )
        .unwrap();

        assert!(content.contains("\"title\""));
    }

    #[test]
    #[cfg(unix)]
    fn claude_cli_provider_reads_json_from_fake_command() {
        let _env_guard = lock_env();
        let path = env::temp_dir().join(format!("reachnote-fake-claude-{}", std::process::id()));
        fake_cli(&path, "Claude 研究卡", "fake-claude");
        env::set_var("REACHNOTE_CLAUDE_CMD", &path);

        let result = ProviderRunner {
            timeout: Duration::from_secs(5),
        }
        .analyze(ProviderId::ClaudeCli, &sample_request())
        .unwrap();

        env::remove_var("REACHNOTE_CLAUDE_CMD");
        let _ = fs::remove_file(path);
        assert_eq!(result.title, "Claude 研究卡");
        assert_eq!(result.model, "fake-claude");
    }

    #[test]
    #[cfg(unix)]
    fn claude_cli_provider_sends_prompt_via_stdin() {
        let _env_guard = lock_env();
        let path = env::temp_dir().join(format!(
            "reachnote-fake-claude-stdin-{}",
            std::process::id()
        ));
        fake_cli_stdin_echo(&path, "provider adapter");
        env::set_var("REACHNOTE_CLAUDE_CMD", &path);

        let result = ProviderRunner {
            timeout: Duration::from_secs(5),
        }
        .analyze(ProviderId::ClaudeCli, &sample_request())
        .unwrap();

        env::remove_var("REACHNOTE_CLAUDE_CMD");
        let _ = fs::remove_file(path);
        assert_eq!(result.summary, "marker-found");
    }

    #[test]
    #[cfg(unix)]
    fn codex_cli_provider_reads_json_from_fake_command() {
        let _env_guard = lock_env();
        let path = env::temp_dir().join(format!("reachnote-fake-codex-{}", std::process::id()));
        fake_cli(&path, "Codex 研究卡", "fake-codex");
        env::set_var("REACHNOTE_CODEX_CMD", &path);

        let result = ProviderRunner {
            timeout: Duration::from_secs(5),
        }
        .analyze(ProviderId::CodexCli, &sample_request())
        .unwrap();

        env::remove_var("REACHNOTE_CODEX_CMD");
        let _ = fs::remove_file(path);
        assert_eq!(result.title, "Codex 研究卡");
        assert_eq!(result.model, "fake-codex");
    }

    #[test]
    #[cfg(unix)]
    fn codex_cli_provider_sends_prompt_via_stdin() {
        let _env_guard = lock_env();
        let path =
            env::temp_dir().join(format!("reachnote-fake-codex-stdin-{}", std::process::id()));
        fake_cli_stdin_echo(&path, "provider adapter");
        env::set_var("REACHNOTE_CODEX_CMD", &path);

        let result = ProviderRunner {
            timeout: Duration::from_secs(5),
        }
        .analyze(ProviderId::CodexCli, &sample_request())
        .unwrap();

        env::remove_var("REACHNOTE_CODEX_CMD");
        let _ = fs::remove_file(path);
        assert_eq!(result.summary, "marker-found");
    }

    #[test]
    fn openai_compatible_provider_reads_local_mock_response() {
        let _env_guard = lock_env();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 4096];
            let _ = stream.read(&mut buffer).unwrap();
            let content = r#"{"title":"OpenAI 研究卡","summary":"基于 URL 的初步判断","key_points":["要点一","要点二","要点三"],"tags":["AI","测试"],"score":5,"next_action":"复核原文","model":"mock-openai"}"#;
            let body = format!(r#"{{"choices":[{{"message":{{"content":{content:?}}}}}]}}"#);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        });

        env::set_var("REACHNOTE_OPENAI_BASE_URL", format!("http://{address}/v1"));
        env::set_var("REACHNOTE_OPENAI_MODEL", "mock-model");
        env::remove_var("REACHNOTE_OPENAI_API_KEY");

        let result = ProviderRunner {
            timeout: Duration::from_secs(5),
        }
        .analyze(ProviderId::OpenAiCompatible, &sample_request())
        .unwrap();

        env::remove_var("REACHNOTE_OPENAI_BASE_URL");
        env::remove_var("REACHNOTE_OPENAI_MODEL");
        handle.join().unwrap();

        assert_eq!(result.title, "OpenAI 研究卡");
        assert_eq!(result.model, "mock-openai");
    }
}
