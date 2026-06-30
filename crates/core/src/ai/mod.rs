//! AI Provider 抽象层。
//!
//! 三种 Provider 共享同一个 [`AiProvider`] trait，由 [`ProviderConfig`] 路由：
//! - 本地 `claude` CLI（[`ClaudeCli`]）
//! - 本地 `codex` CLI（[`CodexCli`]）
//! - 任意 OpenAI 兼容 API（[`OpenAiApi`]）
//!
//! 三者都向模型请求同一套结构化输出（[`AnalysisResult`]），便于后续映射到 Notion 字段。

mod cli;
mod openai;
mod types;

pub use cli::{ClaudeCli, CodexCli};
pub use openai::OpenAiApi;
pub use types::{AnalysisRequest, AnalysisResult, SourceType};

use async_trait::async_trait;
use serde::Deserialize;

/// 统一的 AI Provider 接口。实现体负责把一次分析请求变成结构化结果，
/// 无论底层是本地子进程还是远程 HTTP。
#[async_trait]
pub trait AiProvider: Send + Sync {
    async fn analyze(&self, req: &AnalysisRequest) -> Result<AnalysisResult, AiError>;
    fn id(&self) -> &'static str;
}

/// 用户配置：三选一。反序列化自 `~/.reachnote/config` 中的 `[ai]` 段。
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "provider", rename_all = "kebab-case")]
pub enum ProviderConfig {
    ClaudeCli {
        #[serde(default = "default_claude_cmd")]
        command: String,
    },
    CodexCli {
        #[serde(default = "default_codex_cmd")]
        command: String,
    },
    OpenaiApi {
        base_url: String,
        api_key: String,
        model: String,
    },
}

fn default_claude_cmd() -> String {
    "claude".into()
}
fn default_codex_cmd() -> String {
    "codex".into()
}

/// 按配置构造对应的 Provider 实例。
pub fn build_provider(cfg: ProviderConfig) -> Box<dyn AiProvider> {
    match cfg {
        ProviderConfig::ClaudeCli { command } => Box::new(ClaudeCli::new(command)),
        ProviderConfig::CodexCli { command } => Box::new(CodexCli::new(command)),
        ProviderConfig::OpenaiApi {
            base_url,
            api_key,
            model,
        } => Box::new(OpenAiApi::new(base_url, api_key, model)),
    }
}

/// 组装发给模型的 prompt：模板指令 + 严格 JSON 输出约束 + 正文。
pub fn build_prompt(req: &AnalysisRequest) -> String {
    let template_hint = match req.template.as_str() {
        "github-project" => "你在分析一个 GitHub 项目。关注：项目定位、核心功能、技术栈、适用场景、亮点、风险、是否值得跟进。",
        "article" => "你在精读一篇文章。关注：一句话摘要、关键观点、证据、可复用结论。",
        "video" => "你在总结一个视频。关注：主题、章节摘要、关键观点、行动项、适合谁看。",
        "rss" => "你在归纳一条 RSS 更新。关注：更新摘要、为什么值得看、归类、是否需要后续阅读。",
        _ => "请概括这条内容的核心价值。",
    };
    format!(
        "{hint}\n\n请仅输出一个 JSON 对象，不要包含任何额外文字或 Markdown 代码块。字段：\n\
         {{\"title\": string, \"summary\": string, \"key_points\": string[], \"tags\": string[], \"score\": number(0-100), \"next_action\": string}}\n\n\
         标题：{title}\n来源：{url}\n\n正文：\n{content}",
        hint = template_hint,
        title = req.title.as_deref().unwrap_or("(未知)"),
        url = req.url,
        content = req.content,
    )
}

/// AI 分析过程中的错误。错误信息面向最终用户，需可读、可定位。
#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("启动子进程失败：{0}")]
    Spawn(String),
    #[error("子进程以非零状态退出：{0}")]
    NonZeroExit(String),
    #[error("HTTP 请求失败：{0}")]
    Http(String),
    #[error("无法解析模型输出为结构化结果：{0}")]
    Parse(String),
    #[error("IO 错误：{0}")]
    Io(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_config_deserializes_and_routes() {
        let cfg: ProviderConfig = serde_json::from_str(r#"{"provider":"claude-cli"}"#).unwrap();
        assert_eq!(build_provider(cfg).id(), "claude-cli");

        let cfg: ProviderConfig =
            serde_json::from_str(r#"{"provider":"codex-cli","command":"codex"}"#).unwrap();
        assert_eq!(build_provider(cfg).id(), "codex-cli");

        let cfg: ProviderConfig = serde_json::from_str(
            r#"{"provider":"openai-api","base_url":"http://x","api_key":"k","model":"m"}"#,
        )
        .unwrap();
        assert_eq!(build_provider(cfg).id(), "openai-api");
    }

    #[test]
    fn prompt_includes_url_and_content() {
        let req = AnalysisRequest {
            url: "https://github.com/a/b".into(),
            source_type: SourceType::Github,
            template: "github-project".into(),
            title: Some("B".into()),
            content: "hello world".into(),
        };
        let p = build_prompt(&req);
        assert!(p.contains("https://github.com/a/b"));
        assert!(p.contains("hello world"));
        assert!(p.contains("JSON"));
        assert!(p.contains("GitHub 项目"));
    }
}
