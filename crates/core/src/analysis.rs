use serde::{Deserialize, Serialize};

use crate::task::ErrorKind;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderId {
    ClaudeCli,
    CodexCli,
    OpenAiCompatible,
}

impl ProviderId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ClaudeCli => "claude_cli",
            Self::CodexCli => "codex_cli",
            Self::OpenAiCompatible => "openai_compatible",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::ClaudeCli => "Claude CLI",
            Self::CodexCli => "Codex CLI",
            Self::OpenAiCompatible => "OpenAI-compatible API",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "claude_cli" => Some(Self::ClaudeCli),
            "codex_cli" => Some(Self::CodexCli),
            "openai_compatible" => Some(Self::OpenAiCompatible),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnalysisRequest {
    pub url: String,
    pub source_type: String,
    pub source_domain: Option<String>,
    pub template_id: String,
    pub note: Option<String>,
    pub content_text: Option<String>,
    pub content_reader: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnalysisResult {
    pub title: String,
    pub summary: String,
    pub key_points: Vec<String>,
    pub tags: Vec<String>,
    pub score: u8,
    pub next_action: String,
    pub model: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisValidationError {
    pub kind: ErrorKind,
    pub message: String,
}

pub fn parse_analysis_result(output: &str) -> Result<AnalysisResult, AnalysisValidationError> {
    let mut result: AnalysisResult =
        serde_json::from_str(output.trim()).map_err(|error| AnalysisValidationError {
            kind: ErrorKind::ParseFailed,
            message: format!("AI provider 未返回合法 JSON: {error}"),
        })?;

    result.title = required_text(result.title, "title")?;
    result.summary = required_text(result.summary, "summary")?;
    result.next_action = required_text(result.next_action, "next_action")?;
    result.model = required_text(result.model, "model")?;
    result.key_points = required_items(result.key_points, "key_points")?;
    result.tags = required_items(result.tags, "tags")?;

    if !(1..=5).contains(&result.score) {
        return Err(AnalysisValidationError {
            kind: ErrorKind::SchemaMismatch,
            message: "AI provider 返回的 score 必须在 1 到 5 之间".to_string(),
        });
    }

    Ok(result)
}

pub fn build_analysis_prompt(request: &AnalysisRequest) -> String {
    let note = request
        .note
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("无");
    let source_domain = request.source_domain.as_deref().unwrap_or("未知");
    let content_reader = request
        .content_reader
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("未读取");
    let content_text = normalized_content(request.content_text.as_deref());
    let reading_instruction = if content_text == "未读取到正文" {
        "当前未读取到网页正文；必须明确说明这是基于 URL 和补充说明的待复核初步判断。"
    } else {
        "已读取网页正文；请优先基于正文提炼研究卡，并在 summary 中说明这是基于读取内容生成。"
    };

    format!(
        r#"你是 ReachNote 的本地研究卡分析器。{reading_instruction}

请只返回一个 JSON object，不要返回 Markdown、解释或代码块。字段必须完全符合：
{{
  "title": "string",
  "summary": "string",
  "key_points": ["string", "string", "string"],
  "tags": ["string", "string"],
  "score": 1,
  "next_action": "string",
  "model": "string"
}}

约束：
- title 用中文，尽量短。
- summary 明确说明这是基于 URL 和补充说明的初步判断。
- key_points 给 3 条。
- tags 给 2 到 5 个短标签。
- score 为 1 到 5 的整数，代表当前研究价值。
- next_action 给用户下一步该做什么。
- model 填你实际使用的模型或 provider 名称。
- 不要编造正文没有支持的事实。

输入：
- url: {url}
- source_type: {source_type}
- source_domain: {source_domain}
- template_id: {template_id}
- note: {note}
- content_reader: {content_reader}
- content_text:
{content_text}
"#,
        reading_instruction = reading_instruction,
        url = request.url,
        source_type = request.source_type,
        source_domain = source_domain,
        template_id = request.template_id,
        note = note,
        content_reader = content_reader,
        content_text = content_text,
    )
}

fn normalized_content(value: Option<&str>) -> String {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return "未读取到正文".to_string();
    };

    let max_chars = 12_000;
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    let prefix = value.chars().take(max_chars).collect::<String>();
    format!("{prefix}\n\n[正文过长，已截断到前 {max_chars} 字符]")
}

fn required_text(value: String, field: &str) -> Result<String, AnalysisValidationError> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        return Err(AnalysisValidationError {
            kind: ErrorKind::SchemaMismatch,
            message: format!("AI provider 返回缺少必填字段: {field}"),
        });
    }

    Ok(trimmed)
}

fn required_items(
    values: Vec<String>,
    field: &str,
) -> Result<Vec<String>, AnalysisValidationError> {
    let normalized = values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();

    if normalized.is_empty() {
        return Err(AnalysisValidationError {
            kind: ErrorKind::SchemaMismatch,
            message: format!("AI provider 返回缺少有效数组字段: {field}"),
        });
    }

    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_id_round_trips_snake_case() {
        assert_eq!(
            ProviderId::from_str("claude_cli"),
            Some(ProviderId::ClaudeCli)
        );
        assert_eq!(ProviderId::CodexCli.as_str(), "codex_cli");
        assert_eq!(
            ProviderId::OpenAiCompatible.label(),
            "OpenAI-compatible API"
        );
    }

    #[test]
    fn parses_valid_analysis_json() {
        let parsed = parse_analysis_result(
            r#"{
              "title": "OpenAI 发布研究笔记",
              "summary": "基于 URL 的初步判断。",
              "key_points": ["关注发布时间", "复核正文", "归档到 Notion"],
              "tags": ["AI", "研究"],
              "score": 4,
              "next_action": "打开原文复核重点。",
              "model": "fake-model"
            }"#,
        )
        .unwrap();

        assert_eq!(parsed.score, 4);
        assert_eq!(parsed.key_points.len(), 3);
    }

    #[test]
    fn prompt_includes_read_content_when_available() {
        let prompt = build_analysis_prompt(&AnalysisRequest {
            url: "https://example.com/article".to_string(),
            source_type: "article".to_string(),
            source_domain: Some("example.com".to_string()),
            template_id: "article".to_string(),
            note: Some("关注结论".to_string()),
            content_text: Some("这是一段已读取的网页正文。".to_string()),
            content_reader: Some("Agent-Reach web / Jina Reader".to_string()),
        });

        assert!(prompt.contains("已读取网页正文"));
        assert!(prompt.contains("这是一段已读取的网页正文"));
        assert!(prompt.contains("Agent-Reach web / Jina Reader"));
    }

    #[test]
    fn prompt_marks_missing_content_as_unread() {
        let prompt = build_analysis_prompt(&AnalysisRequest {
            url: "https://example.com/article".to_string(),
            source_type: "article".to_string(),
            source_domain: Some("example.com".to_string()),
            template_id: "article".to_string(),
            note: None,
            content_text: None,
            content_reader: None,
        });

        assert!(prompt.contains("未读取到网页正文"));
        assert!(prompt.contains("待复核初步判断"));
    }

    #[test]
    fn rejects_non_json_analysis_output() {
        let error = parse_analysis_result("not json").unwrap_err();

        assert_eq!(error.kind, ErrorKind::ParseFailed);
    }

    #[test]
    fn rejects_score_outside_contract() {
        let error = parse_analysis_result(
            r#"{
              "title": "标题",
              "summary": "摘要",
              "key_points": ["要点"],
              "tags": ["标签"],
              "score": 9,
              "next_action": "下一步",
              "model": "fake-model"
            }"#,
        )
        .unwrap_err();

        assert_eq!(error.kind, ErrorKind::SchemaMismatch);
    }
}
