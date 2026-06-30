use std::env;
use std::time::Duration;

use reachnote_core::task::ErrorKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadContent {
    pub reader: String,
    pub text: String,
}

#[derive(Debug)]
pub struct ReaderError {
    pub kind: ErrorKind,
    pub message: String,
}

pub struct AgentReachWebReader {
    base_url: String,
    timeout: Duration,
}

impl AgentReachWebReader {
    pub fn from_env() -> Self {
        let base_url = env::var("REACHNOTE_AGENT_REACH_WEB_READER_BASE_URL")
            .ok()
            .map(|value| value.trim().trim_end_matches('/').to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "https://r.jina.ai".to_string());
        let timeout = env::var("REACHNOTE_READER_TIMEOUT_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(30);

        Self {
            base_url,
            timeout: Duration::from_secs(timeout),
        }
    }

    pub fn read_article(&self, url: &str) -> Result<ReadContent, ReaderError> {
        let endpoint = reader_endpoint(&self.base_url, url);
        let client = reqwest::blocking::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|error| ReaderError {
                kind: ErrorKind::NetworkFailed,
                message: format!("Agent-Reach web reader 初始化失败: {error}"),
            })?;

        let response = client.get(&endpoint).send().map_err(|error| ReaderError {
            kind: ErrorKind::NetworkFailed,
            message: format!("Agent-Reach web reader 读取失败: {error}"),
        })?;
        let status = response.status();
        let body = response.text().map_err(|error| ReaderError {
            kind: ErrorKind::NetworkFailed,
            message: format!("Agent-Reach web reader 响应读取失败: {error}"),
        })?;

        if !status.is_success() {
            return Err(ReaderError {
                kind: ErrorKind::ReadFailed,
                message: format!(
                    "Agent-Reach web reader 返回非成功状态 {}: {}",
                    status.as_u16(),
                    truncate_for_message(&body)
                ),
            });
        }

        let text = normalize_reader_body(&body)?;
        Ok(ReadContent {
            reader: "Agent-Reach web / Jina Reader".to_string(),
            text,
        })
    }
}

fn reader_endpoint(base_url: &str, url: &str) -> String {
    format!("{}/{}", base_url.trim_end_matches('/'), url.trim())
}

fn normalize_reader_body(body: &str) -> Result<String, ReaderError> {
    let normalized = body
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();

    if normalized.is_empty() {
        return Err(ReaderError {
            kind: ErrorKind::ReadFailed,
            message: "Agent-Reach web reader 没有返回可分析正文".to_string(),
        });
    }

    Ok(normalized)
}

fn truncate_for_message(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= 240 {
        return trimmed.to_string();
    }

    let prefix = trimmed.chars().take(240).collect::<String>();
    format!("{prefix}...")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn reader_endpoint_appends_original_url() {
        assert_eq!(
            reader_endpoint("https://r.jina.ai/", "https://example.com/a"),
            "https://r.jina.ai/https://example.com/a"
        );
    }

    #[test]
    fn reads_article_from_local_mock_server() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 2048];
            let _ = stream.read(&mut buffer).unwrap();
            let body = "# Example Article\n\nThis is the captured body.";
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        });

        let reader = AgentReachWebReader {
            base_url: format!("http://{address}"),
            timeout: Duration::from_secs(5),
        };
        let content = reader
            .read_article("https://example.com/article")
            .expect("mock reader should return content");

        handle.join().unwrap();
        assert_eq!(content.reader, "Agent-Reach web / Jina Reader");
        assert!(content.text.contains("Example Article"));
    }
}
