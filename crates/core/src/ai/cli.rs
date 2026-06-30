//! 本地 CLI Provider：spawn `claude` / `codex`，从 stdin 喂入 prompt，stdout 收 JSON。
//!
//! 复用已登录的本地 CLI，用户无需另配 API Key，内容也不经第三方中转。

use std::process::Stdio;

use async_trait::async_trait;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use super::types::{AnalysisRequest, AnalysisResult};
use super::{build_prompt, AiError, AiProvider};

/// 通用：spawn 一个 CLI，把 prompt 从 stdin 写入，收集 stdout。
async fn run_cli(command: &str, args: &[String], prompt: &str) -> Result<String, AiError> {
    let mut child = Command::new(command)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| AiError::Spawn(format!("{command}: {e}")))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(prompt.as_bytes())
            .await
            .map_err(|e| AiError::Io(e.to_string()))?;
        stdin.shutdown().await.map_err(|e| AiError::Io(e.to_string()))?;
    }

    let output = child
        .wait_with_output()
        .await
        .map_err(|e| AiError::Io(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AiError::NonZeroExit(format!(
            "{command} ({}): {stderr}",
            output.status
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// 本地 Claude CLI（`claude -p`，非交互 print 模式）。
pub struct ClaudeCli {
    command: String,
    args: Vec<String>,
}

impl ClaudeCli {
    pub fn new(command: String) -> Self {
        Self {
            command,
            args: vec!["-p".into()],
        }
    }
}

#[async_trait]
impl AiProvider for ClaudeCli {
    async fn analyze(&self, req: &AnalysisRequest) -> Result<AnalysisResult, AiError> {
        let out = run_cli(&self.command, &self.args, &build_prompt(req)).await?;
        AnalysisResult::from_model_output(&out, self.id())
    }
    fn id(&self) -> &'static str {
        "claude-cli"
    }
}

/// 本地 Codex CLI（`codex exec`，非交互执行）。
pub struct CodexCli {
    command: String,
    args: Vec<String>,
}

impl CodexCli {
    pub fn new(command: String) -> Self {
        Self {
            command,
            args: vec!["exec".into()],
        }
    }
}

#[async_trait]
impl AiProvider for CodexCli {
    async fn analyze(&self, req: &AnalysisRequest) -> Result<AnalysisResult, AiError> {
        let out = run_cli(&self.command, &self.args, &build_prompt(req)).await?;
        AnalysisResult::from_model_output(&out, self.id())
    }
    fn id(&self) -> &'static str {
        "codex-cli"
    }
}
