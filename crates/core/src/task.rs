use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Queued,
    Reading,
    Analyzing,
    Analyzed,
    Syncing,
    Synced,
    Failed,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Reading => "reading",
            Self::Analyzing => "analyzing",
            Self::Analyzed => "analyzed",
            Self::Syncing => "syncing",
            Self::Synced => "synced",
            Self::Failed => "failed",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "queued" => Some(Self::Queued),
            "reading" => Some(Self::Reading),
            "analyzing" => Some(Self::Analyzing),
            "analyzed" => Some(Self::Analyzed),
            "syncing" => Some(Self::Syncing),
            "synced" => Some(Self::Synced),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    InvalidUrl,
    ReadFailed,
    ProviderUnavailable,
    ParseFailed,
    NotionUnauthorized,
    SchemaMismatch,
    NetworkFailed,
}

impl ErrorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidUrl => "invalid_url",
            Self::ReadFailed => "read_failed",
            Self::ProviderUnavailable => "provider_unavailable",
            Self::ParseFailed => "parse_failed",
            Self::NotionUnauthorized => "notion_unauthorized",
            Self::SchemaMismatch => "schema_mismatch",
            Self::NetworkFailed => "network_failed",
        }
    }

    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "invalid_url" => Some(Self::InvalidUrl),
            "read_failed" => Some(Self::ReadFailed),
            "provider_unavailable" => Some(Self::ProviderUnavailable),
            "parse_failed" => Some(Self::ParseFailed),
            "notion_unauthorized" => Some(Self::NotionUnauthorized),
            "schema_mismatch" => Some(Self::SchemaMismatch),
            "network_failed" => Some(Self::NetworkFailed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Task {
    pub id: String,
    pub url: String,
    pub source_type: String,
    pub template_id: String,
    pub status: TaskStatus,
    pub title: Option<String>,
    pub source_domain: Option<String>,
    pub score: Option<u8>,
    pub model: Option<String>,
    pub provider_id: String,
    pub note: Option<String>,
    pub analysis_json: Option<String>,
    pub notion_page_id: Option<String>,
    pub error_kind: Option<ErrorKind>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub synced_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedUrl {
    pub url: String,
    pub source_domain: String,
}

pub fn validate_article_url(input: &str) -> Result<ValidatedUrl, ErrorKind> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ErrorKind::InvalidUrl);
    }

    let parsed = ParsedUrl::parse(trimmed)?;
    if parsed.scheme != "http" && parsed.scheme != "https" {
        return Err(ErrorKind::InvalidUrl);
    }

    Ok(ValidatedUrl {
        url: parsed.normalized_url,
        source_domain: parsed.host,
    })
}

pub fn source_domain(input: &str) -> Result<String, ErrorKind> {
    let parsed = ParsedUrl::parse(input.trim())?;
    Ok(parsed.host)
}

struct ParsedUrl {
    scheme: String,
    host: String,
    normalized_url: String,
}

impl ParsedUrl {
    fn parse(input: &str) -> Result<Self, ErrorKind> {
        let (scheme, rest) = input.split_once("://").ok_or(ErrorKind::InvalidUrl)?;
        if scheme.is_empty() || rest.is_empty() {
            return Err(ErrorKind::InvalidUrl);
        }

        let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
        let authority = &rest[..authority_end];
        let suffix = &rest[authority_end..];
        if authority.is_empty() || authority.contains(' ') {
            return Err(ErrorKind::InvalidUrl);
        }

        let host_start = authority.rfind('@').map(|index| index + 1).unwrap_or(0);
        let host_port = &authority[host_start..];
        let host_end = host_port.find(':').unwrap_or(host_port.len());
        let host = host_port[..host_end]
            .rsplit('@')
            .next()
            .ok_or(ErrorKind::InvalidUrl)?
            .trim()
            .to_ascii_lowercase();

        if host.is_empty() || !host.contains('.') || host.contains(' ') {
            return Err(ErrorKind::InvalidUrl);
        }

        let lower_scheme = scheme.to_ascii_lowercase();
        let normalized_authority = format!(
            "{}{}{}",
            &authority[..host_start],
            host,
            &host_port[host_end..]
        );

        Ok(Self {
            scheme: lower_scheme.clone(),
            host,
            normalized_url: format!("{lower_scheme}://{normalized_authority}{suffix}"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_status_serializes_to_snake_case() {
        let serialized = serde_json::to_string(&TaskStatus::Queued).unwrap();

        assert_eq!(serialized, "\"queued\"");
    }

    #[test]
    fn validate_article_url_accepts_http_url_and_extracts_domain() {
        let validated = validate_article_url(" https://OpenAI.com/index/gpt-4o ").unwrap();

        assert_eq!(validated.url, "https://openai.com/index/gpt-4o");
        assert_eq!(validated.source_domain, "openai.com");
    }

    #[test]
    fn validate_article_url_rejects_empty_input() {
        let error = validate_article_url("   ").unwrap_err();

        assert_eq!(error, ErrorKind::InvalidUrl);
    }

    #[test]
    fn validate_article_url_rejects_non_url() {
        let error = validate_article_url("abc").unwrap_err();

        assert_eq!(error, ErrorKind::InvalidUrl);
    }

    #[test]
    fn validate_article_url_rejects_non_http_scheme() {
        let error = validate_article_url("file://openai.com/index").unwrap_err();

        assert_eq!(error, ErrorKind::InvalidUrl);
    }

    #[test]
    fn source_domain_extracts_host() {
        let domain = source_domain("https://docs.rs/crate/rusqlite").unwrap();

        assert_eq!(domain, "docs.rs");
    }
}
