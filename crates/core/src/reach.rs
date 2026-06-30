//! 内容读取层：封装对 Agent-Reach 的子进程调用。
//!
//! Agent-Reach（<https://github.com/Panniantong/Agent-Reach>）是一个 Python CLI，
//! 负责“看见互联网”——读取 GitHub / 网页 / YouTube / RSS 等。ReachNote 不自己实现
//! 抓取，而是 spawn `agent-reach` 复用其能力，并用 `agent-reach doctor` 做渠道体检。

use crate::ai::SourceType;
use tokio::process::Command;

#[derive(Debug, thiserror::Error)]
pub enum ReachError {
    #[error("Agent-Reach 未安装或不在 PATH：{0}")]
    NotFound(String),
    #[error("读取失败：{0}")]
    ReadFailed(String),
    #[error("IO 错误：{0}")]
    Io(String),
}

/// Agent-Reach 读取结果，作为 AI 分析的输入。
#[derive(Debug, Clone)]
pub struct ReachContent {
    pub title: Option<String>,
    pub content: String,
    pub source_type: SourceType,
}

/// 根据 URL 粗判来源类型，决定默认模板。
pub fn detect_source(url: &str) -> SourceType {
    let u = url.to_ascii_lowercase();
    if u.contains("github.com") {
        SourceType::Github
    } else if u.contains("youtube.com") || u.contains("youtu.be") || u.contains("bilibili.com") {
        SourceType::Video
    } else if u.contains("/rss") || u.ends_with(".xml") || u.contains("/feed") {
        SourceType::Rss
    } else if u.contains("twitter.com")
        || u.contains("x.com")
        || u.contains("reddit.com")
        || u.contains("xiaohongshu")
    {
        SourceType::Social
    } else {
        SourceType::Article
    }
}

/// 对 `agent-reach` CLI 的封装。
pub struct AgentReach {
    command: String,
}

impl AgentReach {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
        }
    }

    /// 渠道体检，对应 `agent-reach doctor`，用于 Onboarding 与 Settings。
    pub async fn doctor(&self) -> Result<String, ReachError> {
        let out = Command::new(&self.command)
            .arg("doctor")
            .output()
            .await
            .map_err(|e| ReachError::NotFound(format!("{}: {e}", self.command)))?;
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }

    /// 读取 URL 内容。
    ///
    /// 注意：`agent-reach` 的具体读取子命令以其 CLI 为准，此处为占位接口，
    /// 待对齐真实命令（如 `agent-reach read <url> --json`）。
    pub async fn read(&self, url: &str) -> Result<ReachContent, ReachError> {
        let out = Command::new(&self.command)
            .arg("read")
            .arg(url)
            .output()
            .await
            .map_err(|e| ReachError::NotFound(format!("{}: {e}", self.command)))?;
        if !out.status.success() {
            return Err(ReachError::ReadFailed(
                String::from_utf8_lossy(&out.stderr).into_owned(),
            ));
        }
        Ok(ReachContent {
            title: None,
            content: String::from_utf8_lossy(&out.stdout).into_owned(),
            source_type: detect_source(url),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_sources() {
        assert_eq!(detect_source("https://github.com/a/b"), SourceType::Github);
        assert_eq!(detect_source("https://youtu.be/x"), SourceType::Video);
        assert_eq!(
            detect_source("https://example.com/feed"),
            SourceType::Rss
        );
        assert_eq!(
            detect_source("https://x.com/u/status/1"),
            SourceType::Social
        );
        assert_eq!(
            detect_source("https://blog.example.com/post"),
            SourceType::Article
        );
    }
}
