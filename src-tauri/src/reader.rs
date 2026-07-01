use std::env;
use std::time::Duration;

use reachnote_core::task::ErrorKind;
use reqwest::header::{ACCEPT, USER_AGENT};
use serde_json::Value;

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
        if github_repo_from_url(url).is_some() {
            return read_github_repository(url, self.timeout);
        }

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

#[derive(Debug, Clone, PartialEq, Eq)]
struct GitHubRepo {
    owner: String,
    repo: String,
}

fn github_repo_from_url(url: &str) -> Option<GitHubRepo> {
    let (_, rest) = url.trim().split_once("://")?;
    let path_start = rest.find('/')?;
    let host = rest[..path_start]
        .split('@')
        .next_back()?
        .to_ascii_lowercase();
    if host.split(':').next()? != "github.com" {
        return None;
    }

    let path = &rest[path_start + 1..];
    let mut parts = path
        .split(['/', '?', '#'])
        .filter(|part| !part.trim().is_empty());
    let owner = parts.next()?.trim();
    let repo = parts.next()?.trim().trim_end_matches(".git");
    if owner.is_empty() || repo.is_empty() {
        return None;
    }

    Some(GitHubRepo {
        owner: owner.to_string(),
        repo: repo.to_string(),
    })
}

fn read_github_repository(url: &str, timeout: Duration) -> Result<ReadContent, ReaderError> {
    let repo = github_repo_from_url(url).ok_or_else(|| ReaderError {
        kind: ErrorKind::InvalidUrl,
        message: "不是合法的 GitHub repo URL".to_string(),
    })?;
    let client = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|error| ReaderError {
            kind: ErrorKind::NetworkFailed,
            message: format!("GitHub reader 初始化失败: {error}"),
        })?;
    let api_base = github_api_base_url();
    let repo_endpoint = format!("{}/repos/{}/{}", api_base, repo.owner, repo.repo);
    let repo_body = github_get_text(&client, &repo_endpoint, "application/vnd.github+json")?;
    let metadata: Value = serde_json::from_str(&repo_body).map_err(|error| ReaderError {
        kind: ErrorKind::ParseFailed,
        message: format!("GitHub repo metadata 不是合法 JSON: {error}"),
    })?;
    let default_branch = metadata
        .get("default_branch")
        .and_then(Value::as_str)
        .unwrap_or("main");
    let description = metadata
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("");
    let language = metadata
        .get("language")
        .and_then(Value::as_str)
        .unwrap_or("");
    let stars = metadata
        .get("stargazers_count")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let topics = metadata
        .get("topics")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "-".to_string());

    let readme_endpoint = format!(
        "{}/repos/{}/{}/readme?ref={}",
        api_base, repo.owner, repo.repo, default_branch
    );
    let readme = github_get_text(&client, &readme_endpoint, "application/vnd.github.raw")
        .unwrap_or_else(|error| format!("README 读取失败: {}", error.message));
    let text = format!(
        "# GitHub Repository: {}/{}\n\nURL: {}\nDescription: {}\nDefault branch: {}\nLanguage: {}\nStars: {}\nTopics: {}\n\n## README\n\n{}",
        repo.owner,
        repo.repo,
        url.trim(),
        description,
        default_branch,
        language,
        stars,
        topics,
        readme.trim()
    );

    Ok(ReadContent {
        reader: "GitHub API / README".to_string(),
        text,
    })
}

fn github_api_base_url() -> String {
    env::var("REACHNOTE_GITHUB_API_BASE_URL")
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "https://api.github.com".to_string())
}

fn github_get_text(
    client: &reqwest::blocking::Client,
    endpoint: &str,
    accept: &str,
) -> Result<String, ReaderError> {
    let response = client
        .get(endpoint)
        .header(USER_AGENT, "ReachNote")
        .header(ACCEPT, accept)
        .send()
        .map_err(|error| ReaderError {
            kind: ErrorKind::NetworkFailed,
            message: format!("GitHub reader 请求失败: {error}"),
        })?;
    let status = response.status();
    let body = response.text().map_err(|error| ReaderError {
        kind: ErrorKind::NetworkFailed,
        message: format!("GitHub reader 响应读取失败: {error}"),
    })?;

    if !status.is_success() {
        return Err(ReaderError {
            kind: ErrorKind::ReadFailed,
            message: format!(
                "GitHub reader 返回非成功状态 {}: {}",
                status.as_u16(),
                truncate_for_message(&body)
            ),
        });
    }

    Ok(body)
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
    fn github_repo_url_extracts_owner_and_repo() {
        assert_eq!(
            github_repo_from_url("https://github.com/AliceDel66/fe-fidelity-kit").unwrap(),
            GitHubRepo {
                owner: "AliceDel66".to_string(),
                repo: "fe-fidelity-kit".to_string(),
            }
        );
        assert_eq!(
            github_repo_from_url("https://github.com/AliceDel66/fe-fidelity-kit/tree/main")
                .unwrap(),
            GitHubRepo {
                owner: "AliceDel66".to_string(),
                repo: "fe-fidelity-kit".to_string(),
            }
        );
        assert_eq!(github_repo_from_url("https://example.com/a/b"), None);
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
