//! OpenAI 兼容 API Provider：POST `{base_url}/chat/completions`。
//!
//! 覆盖官方 API、第三方代理，以及本地推理的 OpenAI 兼容端点
//! （Ollama `http://localhost:11434/v1`、LM Studio `http://localhost:1234/v1`）。

use async_trait::async_trait;
use serde_json::json;

use super::types::{AnalysisRequest, AnalysisResult};
use super::{build_prompt, AiError, AiProvider};

pub struct OpenAiApi {
    base_url: String,
    api_key: String,
    model: String,
    http: reqwest::Client,
}

impl OpenAiApi {
    pub fn new(base_url: String, api_key: String, model: String) -> Self {
        Self {
            base_url,
            api_key,
            model,
            http: reqwest::Client::new(),
        }
    }
}

/// 构造 chat/completions 请求体（纯函数，便于测试）。
pub fn build_request_body(model: &str, prompt: &str) -> serde_json::Value {
    json!({
        "model": model,
        "messages": [
            {"role": "system", "content": "You are a precise content analyst. Reply with a single JSON object and nothing else."},
            {"role": "user", "content": prompt}
        ],
        "temperature": 0.2,
        "response_format": {"type": "json_object"}
    })
}

/// 从响应中提取 `choices[0].message.content`（纯函数，便于测试）。
pub fn parse_chat_content(resp: &serde_json::Value) -> Result<String, AiError> {
    resp.get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AiError::Parse(format!("响应缺少 choices[0].message.content：{resp}")))
}

#[async_trait]
impl AiProvider for OpenAiApi {
    async fn analyze(&self, req: &AnalysisRequest) -> Result<AnalysisResult, AiError> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let body = build_request_body(&self.model, &build_prompt(req));
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| AiError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AiError::Http(format!("{status}: {text}")));
        }

        let value: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AiError::Http(e.to_string()))?;
        let content = parse_chat_content(&value)?;
        AnalysisResult::from_model_output(&content, &format!("openai:{}", self.model))
    }

    fn id(&self) -> &'static str {
        "openai-api"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_request_body() {
        let b = build_request_body("gpt-4o-mini", "hi");
        assert_eq!(b["model"], "gpt-4o-mini");
        assert_eq!(b["messages"][1]["content"], "hi");
        assert_eq!(b["response_format"]["type"], "json_object");
    }

    #[test]
    fn parses_chat_content() {
        let resp = json!({"choices":[{"message":{"content":"{\"x\":1}"}}]});
        assert_eq!(parse_chat_content(&resp).unwrap(), "{\"x\":1}");
    }

    #[test]
    fn parse_error_when_missing() {
        let resp = json!({"choices":[]});
        assert!(parse_chat_content(&resp).is_err());
    }
}
