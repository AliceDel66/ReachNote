//! Tauri 应用层：暴露 `capture` command，把 reachnote-core 的能力接到前端。

use reachnote_core::{
    build_provider, detect_source, AgentReach, AnalysisRequest, AnalysisResult, ProviderConfig,
    SourceType,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CaptureArgs {
    url: String,
    /// 三选一的 AI Provider 配置。
    provider: ProviderConfig,
    /// 可选：直接提供正文（跳过 Agent-Reach），便于先跑通 AI 竖切。
    #[serde(default)]
    content: Option<String>,
    /// 可选：自定义 agent-reach 命令路径。
    #[serde(default)]
    reach_command: Option<String>,
}

fn template_for(source: SourceType) -> String {
    match source {
        SourceType::Github => "github-project",
        SourceType::Video => "video",
        SourceType::Rss => "rss",
        _ => "article",
    }
    .to_string()
}

/// P0 端到端竖切：URL（+可选正文）→ 读取 → AI 分析 → 结构化结果。
#[tauri::command]
async fn capture(args: CaptureArgs) -> Result<AnalysisResult, String> {
    let source_type = detect_source(&args.url);

    // 有正文则直接用（便于先验证 AI 链路，不依赖 agent-reach 安装）；否则走 Agent-Reach。
    let (title, content) = match args.content {
        Some(c) if !c.trim().is_empty() => (None, c),
        _ => {
            let reach =
                AgentReach::new(args.reach_command.unwrap_or_else(|| "agent-reach".to_string()));
            let read = reach.read(&args.url).await.map_err(|e| e.to_string())?;
            (read.title, read.content)
        }
    };

    let req = AnalysisRequest {
        url: args.url,
        source_type,
        template: template_for(source_type),
        title,
        content,
    };

    let provider = build_provider(args.provider);
    provider.analyze(&req).await.map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .invoke_handler(tauri::generate_handler![capture])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
