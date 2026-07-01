use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::analysis::AnalysisResult;
use crate::task::Task;

const NOTION_TEXT_LIMIT: usize = 2_000;

/// MVP 固定走 2022-06-28 + database_id 的 create-page 路径（对应单 data source 场景）；
/// 2025-09-03 + data_source_id 是尚未拍板的开放决策，先不做成可配置项。
pub const NOTION_API_VERSION: &str = "2022-06-28";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NotionSettings {
    pub token: String,
    pub database_id: String,
    pub version: String,
}

/// 返回给前端的视图：绝不回传明文 token，只回传是否已配置和末位预览。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NotionSettingsView {
    pub configured: bool,
    pub database_id: String,
    pub token_preview: Option<String>,
}

impl NotionSettings {
    pub fn to_view(&self) -> NotionSettingsView {
        NotionSettingsView {
            configured: true,
            database_id: self.database_id.clone(),
            token_preview: Some(mask_token(&self.token)),
        }
    }
}

impl NotionSettingsView {
    pub fn unconfigured() -> Self {
        Self {
            configured: false,
            database_id: String::new(),
            token_preview: None,
        }
    }
}

fn mask_token(token: &str) -> String {
    let chars = token.chars().collect::<Vec<_>>();
    if chars.len() <= 8 {
        return "*".repeat(chars.len());
    }

    let prefix = chars[..4].iter().collect::<String>();
    let suffix = chars[chars.len() - 4..].iter().collect::<String>();
    format!("{prefix}...{suffix}")
}

pub fn build_notion_properties(
    task: &Task,
    analysis: &AnalysisResult,
    captured_at_iso: &str,
    synced_at_iso: &str,
) -> Value {
    json!({
        "Title": {
            "title": [
                { "text": { "content": truncate_notion_text(&analysis.title) } }
            ]
        },
        "URL": {
            "url": task.url
        },
        "Source Type": {
            "select": { "name": source_type_to_select(&task.source_type) }
        },
        "Summary": rich_text(&analysis.summary),
        "Key Points": rich_text(&analysis.key_points.join("\n")),
        "Tags": {
            "multi_select": analysis.tags.iter().map(|tag| {
                json!({ "name": truncate_notion_text(tag) })
            }).collect::<Vec<_>>()
        },
        "Status": {
            "select": { "name": "Inbox" }
        },
        "Score": {
            "number": i64::from(analysis.score) * 20
        },
        "Captured At": {
            "date": { "start": captured_at_iso }
        },
        "Synced At": {
            "date": { "start": synced_at_iso }
        },
        "AI Model": rich_text(&analysis.model),
        "Template": {
            "select": { "name": template_to_select(&task.template_id) }
        },
        "Next Action": rich_text(&analysis.next_action)
    })
}

pub fn source_type_to_select(source_type: &str) -> &'static str {
    match source_type {
        "github" => "GitHub",
        "video" => "Video",
        "rss" => "RSS",
        "social" => "Social",
        "article" => "Article",
        _ => "Article",
    }
}

pub fn template_to_select(template_id: &str) -> &'static str {
    match template_id {
        "github" => "GitHub Project Analysis",
        "video" => "Video Note",
        "rss" => "RSS Digest",
        "article" => "Article Reading Note",
        _ => "Article Reading Note",
    }
}

fn rich_text(content: &str) -> Value {
    json!({
        "rich_text": [
            { "text": { "content": truncate_notion_text(content) } }
        ]
    })
}

fn truncate_notion_text(content: &str) -> String {
    let trimmed = content.trim();
    if trimmed.chars().count() <= NOTION_TEXT_LIMIT {
        return trimmed.to_string();
    }

    trimmed.chars().take(NOTION_TEXT_LIMIT).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::TaskStatus;

    fn sample_task(score: Option<u8>) -> Task {
        Task {
            id: "task-1".to_string(),
            url: "https://example.com/article".to_string(),
            source_type: "article".to_string(),
            template_id: "article".to_string(),
            status: TaskStatus::Analyzed,
            title: Some("测试标题".to_string()),
            source_domain: Some("example.com".to_string()),
            score,
            model: Some("fake-model".to_string()),
            provider_id: "claude_cli".to_string(),
            note: None,
            analysis_json: None,
            notion_page_id: None,
            error_kind: None,
            error_message: None,
            created_at: "1782840000".to_string(),
            updated_at: "1782840000".to_string(),
            synced_at: None,
        }
    }

    fn sample_analysis(score: u8) -> AnalysisResult {
        AnalysisResult {
            title: "结构化研究卡".to_string(),
            summary: "基于正文生成的摘要".to_string(),
            key_points: vec!["要点一".to_string(), "要点二".to_string()],
            tags: vec!["AI".to_string(), "Notion".to_string()],
            score,
            next_action: "复核原文后归档".to_string(),
            model: "fake-claude".to_string(),
        }
    }

    #[test]
    fn maps_score_to_notion_percent_scale() {
        let low = build_notion_properties(
            &sample_task(Some(1)),
            &sample_analysis(1),
            "2026-07-01T00:00:00Z",
            "2026-07-01T00:01:00Z",
        );
        let high = build_notion_properties(
            &sample_task(Some(5)),
            &sample_analysis(5),
            "2026-07-01T00:00:00Z",
            "2026-07-01T00:01:00Z",
        );

        assert_eq!(low["Score"]["number"], 20);
        assert_eq!(high["Score"]["number"], 100);
    }

    #[test]
    fn maps_source_and_template_to_select_options() {
        assert_eq!(source_type_to_select("article"), "Article");
        assert_eq!(source_type_to_select("github"), "GitHub");
        assert_eq!(source_type_to_select("unknown"), "Article");
        assert_eq!(template_to_select("article"), "Article Reading Note");
        assert_eq!(template_to_select("rss"), "RSS Digest");
        assert_eq!(template_to_select("unknown"), "Article Reading Note");
    }

    #[test]
    fn builds_expected_property_shapes() {
        let properties = build_notion_properties(
            &sample_task(Some(4)),
            &sample_analysis(4),
            "2026-07-01T00:00:00Z",
            "2026-07-01T00:01:00Z",
        );

        assert_eq!(
            properties["Title"]["title"][0]["text"]["content"],
            "结构化研究卡"
        );
        assert_eq!(properties["URL"]["url"], "https://example.com/article");
        assert_eq!(properties["Tags"]["multi_select"][0]["name"], "AI");
        assert_eq!(
            properties["Summary"]["rich_text"][0]["text"]["content"],
            "基于正文生成的摘要"
        );
        assert_eq!(properties["Source Type"]["select"]["name"], "Article");
    }

    #[test]
    fn masks_token_in_settings_view() {
        let settings = NotionSettings {
            token: "ntn_abcdefgh12345678".to_string(),
            database_id: "database-123".to_string(),
            version: NOTION_API_VERSION.to_string(),
        };

        let view = settings.to_view();

        assert!(view.configured);
        assert_eq!(view.database_id, "database-123");
        let preview = view.token_preview.unwrap();
        assert!(preview.starts_with("ntn_"));
        assert!(preview.ends_with("5678"));
        assert!(!preview.contains("abcdefgh"));
    }

    #[test]
    fn truncates_rich_text_to_notion_limit() {
        let long_summary = "a".repeat(NOTION_TEXT_LIMIT + 10);
        let mut analysis = sample_analysis(3);
        analysis.summary = long_summary;

        let properties = build_notion_properties(
            &sample_task(Some(3)),
            &analysis,
            "2026-07-01T00:00:00Z",
            "2026-07-01T00:01:00Z",
        );
        let summary = properties["Summary"]["rich_text"][0]["text"]["content"]
            .as_str()
            .unwrap();

        assert_eq!(summary.chars().count(), NOTION_TEXT_LIMIT);
    }
}
