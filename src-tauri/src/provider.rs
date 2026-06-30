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
        process.arg(prompt);

        let output = run_process("Claude CLI", &mut process, self.timeout)?;
        parse_provider_output(&output)
    }

    fn analyze_codex_cli(
        &self,
        request: &AnalysisRequest,
    ) -> Result<AnalysisResult, ProviderError> {
        let command = provider_command("REACHNOTE_CODEX_CMD", "codex");
        ensure_executable("Codex CLI", "REACHNOTE_CODEX_CMD", &command)?;

        let prompt = build_analysis_prompt(request);
        let mut process = Command::new(&command);
        process.args([
            "exec",
            "--skip-git-repo-check",
            "--ignore-rules",
            "--sandbox",
            "read-only",
            "--color",
            "never",
            "--ephemeral",
        ]);
        process.arg(prompt);

        let output = run_process("Codex CLI", &mut process, self.timeout)?;
        parse_provider_output(&output)
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
) -> Result<String, ProviderError> {
    let mut child = process
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| ProviderError {
            kind: ErrorKind::ProviderUnavailable,
            message: format!("{label} 启动失败: {error}"),
        })?;

    // 在等待进程结束的同时持续 drain stdout/stderr：否则子进程一旦写满 OS pipe
    // 缓冲区（macOS 约 64KB）就会阻塞在 write 上，而本线程在 wait_timeout 处阻塞等它退出，
    // 形成死锁直到超时。用独立读线程消费管道，避免该死锁。
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

    // 进程退出或被 kill 后管道关闭，读线程读到 EOF 自然结束。
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
    use std::thread;

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
    fn codex_cli_provider_reads_json_from_fake_command() {
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
    fn openai_compatible_provider_reads_local_mock_response() {
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
