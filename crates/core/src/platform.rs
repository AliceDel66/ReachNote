use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlatformAvailability {
    Ready,
    NeedsInstall,
    NeedsLogin,
    NeedsConfig,
    Blocked,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlatformAction {
    CaptureUrl,
    ReadContent,
    Search,
    Transcribe,
    MetadataOnly,
    NotSupportedYet,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourcePlatformStatus {
    pub key: String,
    pub name: String,
    pub availability: PlatformAvailability,
    pub active_backend: Option<String>,
    pub action: PlatformAction,
    pub message: String,
    pub summary: String,
    pub raw_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformParseError {
    pub message: String,
}

#[derive(Debug, Deserialize)]
struct DoctorEntry {
    status: Option<String>,
    name: Option<String>,
    message: Option<String>,
    active_backend: Option<String>,
    #[allow(dead_code)]
    tier: Option<i64>,
    #[allow(dead_code)]
    backends: Option<Vec<String>>,
}

pub fn normalize_doctor_output(
    doctor_json: &str,
) -> Result<Vec<SourcePlatformStatus>, PlatformParseError> {
    let entries: BTreeMap<String, DoctorEntry> =
        serde_json::from_str(doctor_json).map_err(|error| PlatformParseError {
            message: format!("Agent-Reach doctor JSON 解析失败: {error}"),
        })?;

    let mut platforms = entries
        .into_iter()
        .map(|(key, entry)| normalize_entry(key, entry))
        .collect::<Vec<_>>();
    platforms.sort_by_key(|platform| {
        (
            platform_order(&platform.key).unwrap_or(usize::MAX),
            platform.key.clone(),
        )
    });

    Ok(platforms)
}

fn normalize_entry(key: String, entry: DoctorEntry) -> SourcePlatformStatus {
    let raw_status = entry
        .status
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    let message = entry.message.unwrap_or_default();
    let availability = availability_from_status(&raw_status, &message);
    let action = action_for_platform(&key, availability);
    let summary = summarize_message(&message);
    let name = entry.name.unwrap_or_else(|| key.clone());

    SourcePlatformStatus {
        key,
        name,
        availability,
        active_backend: entry
            .active_backend
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        action,
        message,
        summary,
        raw_status,
    }
}

fn availability_from_status(status: &str, message: &str) -> PlatformAvailability {
    match status {
        "ok" => PlatformAvailability::Ready,
        "off" => PlatformAvailability::NeedsInstall,
        "warn" if contains_login_signal(message) => PlatformAvailability::NeedsLogin,
        "warn" => PlatformAvailability::NeedsInstall,
        _ => PlatformAvailability::Unknown,
    }
}

fn contains_login_signal(message: &str) -> bool {
    let normalized = message.to_ascii_lowercase();
    ["登录", "登录态", "cookie", "login"]
        .iter()
        .any(|keyword| normalized.contains(keyword))
}

fn action_for_platform(key: &str, availability: PlatformAvailability) -> PlatformAction {
    if availability != PlatformAvailability::Ready {
        return PlatformAction::NotSupportedYet;
    }

    match key {
        "github" | "web" | "rss" => PlatformAction::ReadContent,
        "exa_search" | "bilibili" => PlatformAction::Search,
        _ => PlatformAction::MetadataOnly,
    }
}

fn summarize_message(message: &str) -> String {
    let first_line = message.lines().next().unwrap_or("").trim();
    let mut summary = first_line.chars().take(120).collect::<String>();
    if first_line.chars().count() > 120 {
        summary.push('…');
    }
    summary
}

fn platform_order(key: &str) -> Option<usize> {
    PLATFORM_KEYS.iter().position(|candidate| *candidate == key)
}

const PLATFORM_KEYS: [&str; 15] = [
    "github",
    "twitter",
    "youtube",
    "reddit",
    "facebook",
    "instagram",
    "bilibili",
    "xiaohongshu",
    "linkedin",
    "xiaoyuzhou",
    "v2ex",
    "xueqiu",
    "rss",
    "exa_search",
    "web",
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    const SAMPLE: &str = include_str!("testdata/agent_reach_doctor.sample.json");

    #[test]
    fn normalizes_agent_reach_doctor_fixture() {
        let platforms = normalize_doctor_output(SAMPLE).expect("normalize fixture");
        assert_eq!(platforms.len(), 15);

        let keys = platforms
            .iter()
            .map(|platform| platform.key.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(keys, BTreeSet::from(PLATFORM_KEYS));

        let github = platform(&platforms, "github");
        assert_eq!(github.availability, PlatformAvailability::Ready);
        assert_eq!(github.action, PlatformAction::ReadContent);
        assert!(github.active_backend.is_some());

        let web = platform(&platforms, "web");
        assert_eq!(web.availability, PlatformAvailability::Ready);
        assert_eq!(web.action, PlatformAction::ReadContent);
        assert!(web.active_backend.is_some());

        let twitter = platform(&platforms, "twitter");
        assert_eq!(twitter.raw_status, "warn");
        assert_eq!(twitter.availability, PlatformAvailability::NeedsInstall);
        assert_eq!(twitter.action, PlatformAction::NotSupportedYet);

        let xueqiu = platform(&platforms, "xueqiu");
        assert_eq!(xueqiu.raw_status, "warn");
        assert_eq!(xueqiu.availability, PlatformAvailability::NeedsLogin);

        let youtube = platform(&platforms, "youtube");
        assert_eq!(youtube.raw_status, "off");
        assert_eq!(youtube.availability, PlatformAvailability::NeedsInstall);
        assert_eq!(youtube.action, PlatformAction::NotSupportedYet);

        assert!(platforms.iter().any(|platform| platform.raw_status == "ok"));
        assert!(platforms
            .iter()
            .any(|platform| platform.raw_status == "warn"));
        assert!(platforms
            .iter()
            .any(|platform| platform.raw_status == "off"));
    }

    #[test]
    fn malformed_json_returns_parse_error() {
        let error = normalize_doctor_output("not json").unwrap_err();
        assert!(error.message.contains("JSON"));
    }

    fn platform<'a>(platforms: &'a [SourcePlatformStatus], key: &str) -> &'a SourcePlatformStatus {
        platforms
            .iter()
            .find(|platform| platform.key == key)
            .unwrap_or_else(|| panic!("missing platform {key}"))
    }
}
