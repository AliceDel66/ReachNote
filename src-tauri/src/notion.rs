use std::env;
use std::error::Error;
use std::time::Duration;

use reachnote_core::notion::NotionSettings;
use reachnote_core::task::ErrorKind;
use serde_json::{json, Value};

#[derive(Debug)]
pub struct NotionError {
    pub kind: ErrorKind,
    pub message: String,
}

pub struct NotionClient {
    token: String,
    database_id: String,
    version: String,
    timeout: Duration,
    base_url: String,
}

impl NotionClient {
    pub fn from_settings(settings: NotionSettings) -> Result<Self, NotionError> {
        let token = required_setting(settings.token, "Notion Integration Token")?;
        let database_id = required_setting(settings.database_id, "Notion Database ID")?;
        let version = {
            let value = settings.version.trim().to_string();
            if value.is_empty() {
                "2022-06-28".to_string()
            } else {
                value
            }
        };
        let timeout = env::var("REACHNOTE_NOTION_TIMEOUT_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(30);
        let base_url = env::var("REACHNOTE_NOTION_BASE_URL")
            .ok()
            .map(|value| value.trim().trim_end_matches('/').to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "https://api.notion.com".to_string());

        Ok(Self {
            token,
            database_id,
            version,
            timeout: Duration::from_secs(timeout),
            base_url,
        })
    }

    pub fn test_connection(&self) -> Result<String, NotionError> {
        let client = self.build_client()?;
        let response = client
            .get(format!(
                "{}/v1/databases/{}",
                self.base_url, self.database_id
            ))
            .bearer_auth(&self.token)
            .header("Notion-Version", &self.version)
            .send()
            .map_err(|error| NotionError {
                kind: ErrorKind::NetworkFailed,
                message: format!("Notion 连接测试失败: {}", error_chain(&error)),
            })?;

        let body = read_success_body(response)?;
        let value: Value = serde_json::from_str(&body).map_err(|error| NotionError {
            kind: ErrorKind::ParseFailed,
            message: format!("Notion database 响应不是合法 JSON: {error}"),
        })?;
        let object = value.get("object").and_then(Value::as_str).unwrap_or("");
        if object != "database" {
            return Err(NotionError {
                kind: ErrorKind::ParseFailed,
                message: "Notion 连接测试成功但响应不是 database 对象".to_string(),
            });
        }

        Ok("Notion 连接可用，database 可读取。".to_string())
    }

    pub fn create_page(&self, properties: Value) -> Result<String, NotionError> {
        let payload = json!({
            "parent": { "database_id": self.database_id },
            "properties": properties
        });

        let client = self.build_client()?;

        let response = client
            .post(format!("{}/v1/pages", self.base_url))
            .bearer_auth(&self.token)
            .header("Notion-Version", &self.version)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .map_err(|error| NotionError {
                kind: ErrorKind::NetworkFailed,
                message: format!("Notion 请求失败: {}", error_chain(&error)),
            })?;

        let body = read_success_body(response)?;
        notion_page_id(&body)
    }

    fn build_client(&self) -> Result<reqwest::blocking::Client, NotionError> {
        reqwest::blocking::Client::builder()
            .timeout(self.timeout)
            .http1_only()
            .build()
            .map_err(|error| NotionError {
                kind: ErrorKind::NetworkFailed,
                message: format!("无法初始化 Notion 客户端: {}", error_chain(&error)),
            })
    }
}

fn required_setting(value: String, label: &str) -> Result<String, NotionError> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        return Err(NotionError {
            kind: ErrorKind::NotionUnauthorized,
            message: format!("未配置 {label}，请在设置页保存 Notion 连接。"),
        });
    }

    Ok(trimmed)
}

fn read_success_body(response: reqwest::blocking::Response) -> Result<String, NotionError> {
    let status = response.status();
    let body = response.text().map_err(|error| NotionError {
        kind: ErrorKind::NetworkFailed,
        message: format!("Notion 响应读取失败: {error}"),
    })?;

    if !status.is_success() {
        return Err(classify_notion_response(status.as_u16(), &body));
    }

    Ok(body)
}

fn classify_notion_response(status: u16, body: &str) -> NotionError {
    let message = notion_error_message(body);
    match status {
        401 | 403 => NotionError {
            kind: ErrorKind::NotionUnauthorized,
            message: format!("Notion 授权失败，请检查本地保存的 Notion token 和 database share: {message}"),
        },
        404 => NotionError {
            kind: ErrorKind::NotionUnauthorized,
            message: format!(
                "Notion database 不可访问，请检查本地保存的 Database ID 并确认已 share 给 integration: {message}"
            ),
        },
        400 => NotionError {
            kind: ErrorKind::SchemaMismatch,
            message: format!("Notion 字段映射不匹配: {message}"),
        },
        _ => NotionError {
            kind: ErrorKind::NetworkFailed,
            message: format!("Notion 返回非成功状态 {status}: {message}"),
        },
    }
}

fn notion_error_message(body: &str) -> String {
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| {
            value
                .get("message")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .map(|value| truncate_for_message(&value))
        .unwrap_or_else(|| truncate_for_message(body))
}

fn notion_page_id(body: &str) -> Result<String, NotionError> {
    let value: Value = serde_json::from_str(body).map_err(|error| NotionError {
        kind: ErrorKind::ParseFailed,
        message: format!("Notion 响应不是合法 JSON: {error}"),
    })?;

    value
        .get("id")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| NotionError {
            kind: ErrorKind::ParseFailed,
            message: "Notion 创建 page 成功但响应缺少 id".to_string(),
        })
}

fn error_chain(error: &dyn Error) -> String {
    let mut parts = vec![error.to_string()];
    let mut source = error.source();
    while let Some(error) = source {
        parts.push(error.to_string());
        source = error.source();
    }

    truncate_for_message(&parts.join(": "))
}

fn truncate_for_message(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= 240 {
        return trimmed.to_string();
    }

    let prefix = trimmed.chars().take(240).collect::<String>();
    format!("{prefix}...")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn parses_page_id_from_success_response() {
        let page_id = notion_page_id(r#"{"id":"page-123"}"#).unwrap();
        assert_eq!(page_id, "page-123");
    }

    #[test]
    fn classifies_unauthorized_without_token_leak() {
        let error = classify_notion_response(
            401,
            r#"{"code":"unauthorized","message":"API token is invalid."}"#,
        );

        assert_eq!(error.kind, ErrorKind::NotionUnauthorized);
        assert!(!error.message.contains("ntn_"));
        assert!(error.message.contains("授权失败"));
    }

    #[test]
    fn classifies_schema_mismatch() {
        let error = classify_notion_response(
            400,
            r#"{"code":"validation_error","message":"Score is expected to be number."}"#,
        );

        assert_eq!(error.kind, ErrorKind::SchemaMismatch);
        assert!(error.message.contains("字段映射"));
    }

    #[test]
    fn create_page_posts_database_parent_and_properties() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 8192];
            let size = stream.read(&mut buffer).unwrap();
            let request = String::from_utf8_lossy(&buffer[..size]);
            let lower_request = request.to_ascii_lowercase();
            assert!(request.starts_with("POST /v1/pages "));
            assert!(lower_request.contains("authorization: bearer fake-token"));
            assert!(lower_request.contains("notion-version: 2022-06-28"));
            assert!(request.contains(r#""database_id":"database-123""#));
            assert!(request.contains(r#""Title""#));

            let body = r#"{"id":"page-123"}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        });

        let client = test_client(address.to_string());
        let page_id = client
            .create_page(json!({
                "Title": { "title": [{ "text": { "content": "测试" } }] }
            }))
            .unwrap();

        handle.join().unwrap();
        assert_eq!(page_id, "page-123");
    }

    #[test]
    fn rejects_empty_settings_without_token_leak() {
        let error = match NotionClient::from_settings(NotionSettings {
            token: String::new(),
            database_id: "database-123".to_string(),
            version: "2022-06-28".to_string(),
        }) {
            Ok(_) => panic!("empty token should be rejected"),
            Err(error) => error,
        };

        assert_eq!(error.kind, ErrorKind::NotionUnauthorized);
        assert!(!error.message.contains("database-123"));
        assert!(error.message.contains("未配置"));
    }

    #[test]
    fn test_connection_reads_database() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 8192];
            let size = stream.read(&mut buffer).unwrap();
            let request = String::from_utf8_lossy(&buffer[..size]);
            let lower_request = request.to_ascii_lowercase();
            assert!(request.starts_with("GET /v1/databases/database-123 "));
            assert!(lower_request.contains("authorization: bearer fake-token"));
            assert!(lower_request.contains("notion-version: 2022-06-28"));

            let body = r#"{"object":"database","id":"database-123"}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        });

        let message = test_client(address.to_string()).test_connection().unwrap();

        handle.join().unwrap();
        assert!(message.contains("连接可用"));
    }

    fn test_client(address: String) -> NotionClient {
        NotionClient {
            token: "fake-token".to_string(),
            database_id: "database-123".to_string(),
            version: "2022-06-28".to_string(),
            timeout: Duration::from_secs(5),
            base_url: format!("http://{address}"),
        }
    }
}
