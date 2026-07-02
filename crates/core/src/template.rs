use serde::Serialize;

pub const DEFAULT_TEMPLATE_ID: &str = "web_article";
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

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct TemplateAlias {
    pub alias: &'static str,
    pub template_id: &'static str,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct PlatformRule {
    pub platform_key: &'static str,
    pub exact_hosts: &'static [&'static str],
    pub host_suffixes: &'static [&'static str],
    pub path_keywords: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct PlatformTemplateMapping {
    pub platform_key: &'static str,
    pub template_id: &'static str,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct TemplateRegistry {
    pub templates: &'static [ResearchTemplate],
    pub template_aliases: &'static [TemplateAlias],
    pub platform_rules: &'static [PlatformRule],
    pub platform_template_mappings: &'static [PlatformTemplateMapping],
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

pub const TEMPLATE_ALIASES: &[TemplateAlias] = &[TemplateAlias {
    alias: "article",
    template_id: DEFAULT_TEMPLATE_ID,
}];

pub const PLATFORM_RULES: &[PlatformRule] = &[
    PlatformRule {
        platform_key: "github",
        exact_hosts: &["github.com"],
        host_suffixes: &[],
        path_keywords: &[],
    },
    PlatformRule {
        platform_key: "youtube",
        exact_hosts: &["youtu.be"],
        host_suffixes: &["youtube.com"],
        path_keywords: &[],
    },
    PlatformRule {
        platform_key: "bilibili",
        exact_hosts: &["b23.tv"],
        host_suffixes: &["bilibili.com"],
        path_keywords: &[],
    },
    PlatformRule {
        platform_key: "xiaoyuzhou",
        exact_hosts: &[],
        host_suffixes: &["xiaoyuzhoufm.com"],
        path_keywords: &[],
    },
    PlatformRule {
        platform_key: "twitter",
        exact_hosts: &["twitter.com", "x.com"],
        host_suffixes: &[],
        path_keywords: &[],
    },
    PlatformRule {
        platform_key: "reddit",
        exact_hosts: &[],
        host_suffixes: &["reddit.com"],
        path_keywords: &[],
    },
    PlatformRule {
        platform_key: "facebook",
        exact_hosts: &["fb.com"],
        host_suffixes: &["facebook.com"],
        path_keywords: &[],
    },
    PlatformRule {
        platform_key: "instagram",
        exact_hosts: &[],
        host_suffixes: &["instagram.com"],
        path_keywords: &[],
    },
    PlatformRule {
        platform_key: "xiaohongshu",
        exact_hosts: &["xhslink.com"],
        host_suffixes: &["xiaohongshu.com"],
        path_keywords: &[],
    },
    PlatformRule {
        platform_key: "linkedin",
        exact_hosts: &[],
        host_suffixes: &["linkedin.com"],
        path_keywords: &[],
    },
    PlatformRule {
        platform_key: "v2ex",
        exact_hosts: &["v2ex.com"],
        host_suffixes: &[],
        path_keywords: &[],
    },
    PlatformRule {
        platform_key: "xueqiu",
        exact_hosts: &[],
        host_suffixes: &["xueqiu.com"],
        path_keywords: &[],
    },
    PlatformRule {
        platform_key: "rss",
        exact_hosts: &[],
        host_suffixes: &[],
        path_keywords: &["rss", "feed"],
    },
];

pub const PLATFORM_TEMPLATE_MAPPINGS: &[PlatformTemplateMapping] = &[
    PlatformTemplateMapping {
        platform_key: "web",
        template_id: DEFAULT_TEMPLATE_ID,
    },
    PlatformTemplateMapping {
        platform_key: "github",
        template_id: "github_project",
    },
    PlatformTemplateMapping {
        platform_key: "youtube",
        template_id: "video_note",
    },
    PlatformTemplateMapping {
        platform_key: "bilibili",
        template_id: "video_note",
    },
    PlatformTemplateMapping {
        platform_key: "xiaoyuzhou",
        template_id: "video_note",
    },
    PlatformTemplateMapping {
        platform_key: "rss",
        template_id: "rss_digest",
    },
    PlatformTemplateMapping {
        platform_key: "twitter",
        template_id: "platform_discussion",
    },
    PlatformTemplateMapping {
        platform_key: "reddit",
        template_id: "platform_discussion",
    },
    PlatformTemplateMapping {
        platform_key: "facebook",
        template_id: "platform_discussion",
    },
    PlatformTemplateMapping {
        platform_key: "instagram",
        template_id: "platform_discussion",
    },
    PlatformTemplateMapping {
        platform_key: "xiaohongshu",
        template_id: "platform_discussion",
    },
    PlatformTemplateMapping {
        platform_key: "linkedin",
        template_id: "platform_discussion",
    },
    PlatformTemplateMapping {
        platform_key: "v2ex",
        template_id: "platform_discussion",
    },
];

pub fn built_in_templates() -> &'static [ResearchTemplate] {
    BUILT_IN_TEMPLATES
}

pub fn canonical_template_id(value: &str) -> Option<&'static str> {
    let trimmed = value.trim();
    if let Some(template) = built_in_templates()
        .iter()
        .find(|template| template.id == trimmed)
    {
        return Some(template.id);
    }

    TEMPLATE_ALIASES
        .iter()
        .find(|alias| alias.alias == trimmed)
        .map(|alias| alias.template_id)
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
    let Some(platform_key) = platform_key_for_url(url) else {
        return DEFAULT_TEMPLATE_ID;
    };

    template_id_for_platform_key(platform_key).unwrap_or(DEFAULT_TEMPLATE_ID)
}

pub fn template_registry() -> TemplateRegistry {
    TemplateRegistry {
        templates: BUILT_IN_TEMPLATES,
        template_aliases: TEMPLATE_ALIASES,
        platform_rules: PLATFORM_RULES,
        platform_template_mappings: PLATFORM_TEMPLATE_MAPPINGS,
    }
}

pub fn platform_key_for_url(url: &str) -> Option<&'static str> {
    let Some((host, path)) = split_url_host_path(url) else {
        return None;
    };
    let host = host.strip_prefix("www.").unwrap_or(&host);

    for rule in PLATFORM_RULES {
        if rule.exact_hosts.iter().any(|exact_host| *exact_host == host) {
            return Some(rule.platform_key);
        }
    }

    for rule in PLATFORM_RULES {
        if rule
            .host_suffixes
            .iter()
            .any(|suffix| host_matches_suffix(host, suffix))
        {
            return Some(rule.platform_key);
        }
    }

    for rule in PLATFORM_RULES {
        if rule
            .path_keywords
            .iter()
            .any(|keyword| path.contains(keyword))
        {
            return Some(rule.platform_key);
        }
    }

    Some("web")
}

pub fn template_id_for_platform_key(platform_key: &str) -> Option<&'static str> {
    PLATFORM_TEMPLATE_MAPPINGS
        .iter()
        .find(|mapping| mapping.platform_key == platform_key)
        .map(|mapping| mapping.template_id)
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

fn host_matches_suffix(host: &str, suffix: &str) -> bool {
    host == suffix || host.strip_suffix(suffix).is_some_and(|prefix| prefix.ends_with('.'))
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
    fn registry_exposes_alias_rules_and_mappings() {
        let registry = template_registry();
        assert_eq!(registry.templates.len(), 5);
        assert_eq!(registry.template_aliases[0].alias, "article");
        assert_eq!(
            registry.template_aliases[0].template_id,
            DEFAULT_TEMPLATE_ID
        );
        assert!(registry
            .platform_rules
            .iter()
            .any(|rule| rule.platform_key == "github"));
        assert_eq!(template_id_for_platform_key("github"), Some("github_project"));
    }

    #[test]
    fn platform_rules_match_by_priority() {
        assert_eq!(
            platform_key_for_url("https://github.com/AliceDel66/ReachNote/feed"),
            Some("github")
        );
        assert_eq!(
            platform_key_for_url("https://www.youtube.com/watch?v=1"),
            Some("youtube")
        );
        assert_eq!(platform_key_for_url("https://youtu.be/abc"), Some("youtube"));
        assert_eq!(
            platform_key_for_url("https://www.bilibili.com/video/BV1"),
            Some("bilibili")
        );
        assert_eq!(
            platform_key_for_url("https://example.com/feed.xml"),
            Some("rss")
        );
        assert_eq!(platform_key_for_url("https://x.com/openai"), Some("twitter"));
        assert_eq!(platform_key_for_url("https://example.com/post"), Some("web"));
        assert_eq!(platform_key_for_url("not a url"), None);
    }

    #[test]
    fn suggests_templates_from_platform_rules() {
        let cases = [
            ("https://github.com/AliceDel66/ReachNote", "github_project"),
            ("https://example.com/post", DEFAULT_TEMPLATE_ID),
            ("https://example.com/feed.xml", "rss_digest"),
            ("https://www.youtube.com/watch?v=1", "video_note"),
            ("https://www.bilibili.com/video/BV1", "video_note"),
            ("https://twitter.com/openai/status/1", "platform_discussion"),
            ("https://xueqiu.com/1234567890", DEFAULT_TEMPLATE_ID),
            ("not a url", DEFAULT_TEMPLATE_ID),
        ];

        for (url, expected) in cases {
            assert_eq!(suggest_template_id_for_url(url), expected);
        }
    }
}
