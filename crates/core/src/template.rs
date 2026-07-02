use serde::Serialize;

pub const DEFAULT_TEMPLATE_ID: &str = "web_article";
const LEGACY_ARTICLE_TEMPLATE_ID: &str = "article";
const OUTPUT_SCHEMA_RESEARCH_CARD_V1: &str = "research_card_v1";
const TEMPLATE_DATE: &str = "2026-07-01";

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct TemplateDestinationMapping {
    pub destination_id: &'static str,
    pub field_mapping: &'static str,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct ResearchTemplate {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub compatible_source_types: &'static [&'static str],
    pub prompt_profile: &'static str,
    pub output_schema: &'static str,
    pub destination_mappings: &'static [TemplateDestinationMapping],
    pub enabled: bool,
    pub system: bool,
    pub created_at: &'static str,
    pub updated_at: &'static str,
}

const NOTION_RESEARCH_CARD_MAPPING: &[TemplateDestinationMapping] = &[TemplateDestinationMapping {
    destination_id: "notion",
    field_mapping: "research_card_v1",
}];

pub const BUILT_IN_TEMPLATES: &[ResearchTemplate] = &[
    ResearchTemplate {
        id: "github_project",
        name: "GitHub 项目分析",
        description: "分析代码库、架构与生态",
        compatible_source_types: &["github_repo", "github"],
        prompt_profile: "重点提炼项目定位、核心能力、技术栈、适用场景、潜在风险和下一步验证动作。",
        output_schema: OUTPUT_SCHEMA_RESEARCH_CARD_V1,
        destination_mappings: NOTION_RESEARCH_CARD_MAPPING,
        enabled: true,
        system: true,
        created_at: TEMPLATE_DATE,
        updated_at: TEMPLATE_DATE,
    },
    ResearchTemplate {
        id: DEFAULT_TEMPLATE_ID,
        name: "网页文章笔记",
        description: "长文阅读与观点提炼",
        compatible_source_types: &["article", "web"],
        prompt_profile: "重点提炼摘要、关键论点、可复用观点、标签和下一步行动。",
        output_schema: OUTPUT_SCHEMA_RESEARCH_CARD_V1,
        destination_mappings: NOTION_RESEARCH_CARD_MAPPING,
        enabled: true,
        system: true,
        created_at: TEMPLATE_DATE,
        updated_at: TEMPLATE_DATE,
    },
    ResearchTemplate {
        id: "video_note",
        name: "视频笔记",
        description: "提取主题、章节与关键片段",
        compatible_source_types: &["youtube_video", "bilibili_video", "podcast"],
        prompt_profile:
            "重点提炼主题、章节或片段、关键观点和可执行动作；没有转写正文时必须提醒用户复核。",
        output_schema: OUTPUT_SCHEMA_RESEARCH_CARD_V1,
        destination_mappings: NOTION_RESEARCH_CARD_MAPPING,
        enabled: true,
        system: true,
        created_at: TEMPLATE_DATE,
        updated_at: TEMPLATE_DATE,
    },
    ResearchTemplate {
        id: "rss_digest",
        name: "RSS 简报",
        description: "聚合与总结多源资讯",
        compatible_source_types: &["rss_feed", "rss_item"],
        prompt_profile:
            "重点提炼多条来源聚合、趋势、重点链接和待读优先级；单条来源时说明样本有限。",
        output_schema: OUTPUT_SCHEMA_RESEARCH_CARD_V1,
        destination_mappings: NOTION_RESEARCH_CARD_MAPPING,
        enabled: true,
        system: true,
        created_at: TEMPLATE_DATE,
        updated_at: TEMPLATE_DATE,
    },
    ResearchTemplate {
        id: "platform_discussion",
        name: "平台讨论摘要",
        description: "总结社交平台讨论共识与分歧",
        compatible_source_types: &[
            "twitter",
            "reddit",
            "v2ex",
            "xiaohongshu",
            "facebook",
            "instagram",
            "linkedin",
        ],
        prompt_profile:
            "重点提炼讨论共识、分歧、代表性观点和可信度提醒；涉及登录态或样本不足时必须标注局限。",
        output_schema: OUTPUT_SCHEMA_RESEARCH_CARD_V1,
        destination_mappings: NOTION_RESEARCH_CARD_MAPPING,
        enabled: true,
        system: true,
        created_at: TEMPLATE_DATE,
        updated_at: TEMPLATE_DATE,
    },
];

pub fn built_in_templates() -> &'static [ResearchTemplate] {
    BUILT_IN_TEMPLATES
}

pub fn canonical_template_id(value: &str) -> Option<&'static str> {
    match value.trim() {
        LEGACY_ARTICLE_TEMPLATE_ID | DEFAULT_TEMPLATE_ID => Some(DEFAULT_TEMPLATE_ID),
        "github_project" => Some("github_project"),
        "video_note" => Some("video_note"),
        "rss_digest" => Some("rss_digest"),
        "platform_discussion" => Some("platform_discussion"),
        _ => None,
    }
}

pub fn template_by_id(value: &str) -> Option<&'static ResearchTemplate> {
    let canonical_id = canonical_template_id(value)?;
    built_in_templates()
        .iter()
        .find(|template| template.id == canonical_id)
}

pub fn template_label(value: &str) -> &'static str {
    template_by_id(value)
        .map(|template| template.name)
        .unwrap_or("未知模板")
}

pub fn suggest_template_id_for_url(url: &str) -> &'static str {
    let Some((host, path)) = split_url_host_path(url) else {
        return DEFAULT_TEMPLATE_ID;
    };
    let host = host.strip_prefix("www.").unwrap_or(&host);

    if host == "github.com" {
        return "github_project";
    }
    if host == "youtu.be"
        || host.ends_with("youtube.com")
        || host.ends_with("bilibili.com")
        || host == "b23.tv"
        || host.ends_with("xiaoyuzhoufm.com")
    {
        return "video_note";
    }
    if path.contains("rss") || path.contains("feed") {
        return "rss_digest";
    }
    if host == "twitter.com"
        || host == "x.com"
        || host.ends_with("reddit.com")
        || host.ends_with("facebook.com")
        || host == "fb.com"
        || host.ends_with("instagram.com")
        || host.ends_with("xiaohongshu.com")
        || host == "xhslink.com"
        || host.ends_with("linkedin.com")
        || host == "v2ex.com"
    {
        return "platform_discussion";
    }

    DEFAULT_TEMPLATE_ID
}

fn split_url_host_path(url: &str) -> Option<(String, String)> {
    let trimmed = url.trim().to_ascii_lowercase();
    let rest = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))?;
    let host_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let host = rest[..host_end]
        .rsplit('@')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("")
        .trim();
    if host.is_empty() {
        return None;
    }
    let path = rest[host_end..].to_string();
    Some((host.to_string(), path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalizes_legacy_article_template() {
        assert_eq!(canonical_template_id("article"), Some(DEFAULT_TEMPLATE_ID));
        assert_eq!(
            template_by_id("article").map(|template| template.name),
            Some("网页文章笔记")
        );
    }

    #[test]
    fn suggests_templates_from_url_shape() {
        assert_eq!(
            suggest_template_id_for_url("https://github.com/AliceDel66/ReachNote"),
            "github_project"
        );
        assert_eq!(
            suggest_template_id_for_url("https://example.com/post"),
            DEFAULT_TEMPLATE_ID
        );
        assert_eq!(
            suggest_template_id_for_url("https://example.com/feed.xml"),
            "rss_digest"
        );
        assert_eq!(
            suggest_template_id_for_url("https://www.youtube.com/watch?v=1"),
            "video_note"
        );
    }
}
