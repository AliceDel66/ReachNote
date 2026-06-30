//! AI 分析的请求与结果类型，以及模型输出的容错解析。

use super::AiError;
use serde::{Deserialize, Serialize};

/// 内容来源类型。由 URL 粗判，决定默认模板。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceType {
    Github,
    Article,
    Video,
    Rss,
    Social,
    Unknown,
}

/// 一次分析请求：已由 Agent-Reach 读取并清洗后的正文 + 模板。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisRequest {
    pub url: String,
    pub source_type: SourceType,
    /// 模板 id：github-project / article / video / rss。
    pub template: String,
    #[serde(default)]
    pub title: Option<String>,
    pub content: String,
}

/// 结构化分析结果，字段与 Notion database 对齐。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub title: String,
    pub summary: String,
    #[serde(default)]
    pub key_points: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// 价值评分 0-100。
    #[serde(default)]
    pub score: u8,
    #[serde(default)]
    pub next_action: String,
    /// 实际使用的模型 / Provider，由 Provider 回填。
    #[serde(default)]
    pub model: String,
}

impl AnalysisResult {
    /// 从模型原始输出解析为结构化结果。
    ///
    /// 容错：模型常会在 JSON 前后附带解释文字或 ```json 围栏，
    /// 这里提取第一个平衡的 JSON 对象再反序列化。
    pub fn from_model_output(raw: &str, model_id: &str) -> Result<Self, AiError> {
        let json = extract_json_object(raw).ok_or_else(|| {
            AiError::Parse(format!("输出中未找到 JSON 对象：{}", truncate(raw, 200)))
        })?;
        let mut result: AnalysisResult = serde_json::from_str(json)
            .map_err(|e| AiError::Parse(format!("{e}；原文：{}", truncate(json, 200))))?;
        if result.model.is_empty() {
            result.model = model_id.to_string();
        }
        Ok(result)
    }
}

/// 提取首个平衡的 JSON 对象子串，容忍字符串内的花括号与前后噪声。
pub fn extract_json_object(raw: &str) -> Option<&str> {
    let bytes = raw.as_bytes();
    let start = raw.find('{')?;
    let mut depth = 0usize;
    let mut in_str = false;
    let mut escaped = false;
    for i in start..bytes.len() {
        let c = bytes[i] as char;
        if in_str {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_str = false;
            }
            continue;
        }
        match c {
            '"' => in_str = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&raw[start..=i]);
                }
            }
            _ => {}
        }
    }
    None
}

fn truncate(s: &str, n: usize) -> String {
    let cut = s.char_indices().nth(n).map(|(i, _)| i).unwrap_or(s.len());
    if cut >= s.len() {
        s.to_string()
    } else {
        format!("{}…", &s[..cut])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_plain_json() {
        let r = r#"{"title":"T","summary":"S","score":80,"next_action":"go"}"#;
        let v = AnalysisResult::from_model_output(r, "m").unwrap();
        assert_eq!(v.title, "T");
        assert_eq!(v.score, 80);
        assert_eq!(v.model, "m"); // 回填
    }

    #[test]
    fn extracts_json_with_fence_and_prose() {
        let r = "好的，结果如下：\n```json\n{\"title\":\"T\",\"summary\":\"S\",\"key_points\":[\"a\",\"b\"]}\n```\n完成。";
        let v = AnalysisResult::from_model_output(r, "m").unwrap();
        assert_eq!(v.key_points, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(v.score, 0); // 缺省
    }

    #[test]
    fn handles_braces_inside_strings() {
        let r = r#"prefix {"title":"T","summary":"has {nested} brace","next_action":""} suffix"#;
        let v = AnalysisResult::from_model_output(r, "m").unwrap();
        assert!(v.summary.contains("nested"));
    }

    #[test]
    fn errors_on_no_json() {
        assert!(AnalysisResult::from_model_output("no json here", "m").is_err());
    }
}
