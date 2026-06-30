//! reachnote-core
//!
//! ReachNote 的纯逻辑核心，不依赖 Tauri，可独立测试：
//! - [`ai`]：AI Provider 抽象（本地 Claude CLI / Codex CLI / OpenAI 兼容 API）与结构化分析。
//! - [`reach`]：内容读取层，封装对 Agent-Reach CLI 的调用。

pub mod ai;
pub mod reach;

pub use ai::{
    build_prompt, build_provider, AiError, AiProvider, AnalysisRequest, AnalysisResult,
    ProviderConfig, SourceType,
};
pub use reach::{detect_source, AgentReach, ReachContent, ReachError};
