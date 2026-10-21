use async_trait::async_trait;
use max_core::errors::{MaxError, Result};
use max_core::models::{FunctionCall, Message, ToolCall};
use max_core::traits::{ModelProvider, ModelResponse};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Value};

pub struct GenericOpenAIProvider {
    name: String,
    base_url: String,
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl GenericOpenAIProvider {
    pub fn new(
        name: impl Into<String>,
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            base_url: base_url.into(),
            api_key: api_key.into(),
            model: model.into(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl ModelProvider for GenericOpenAIProvider {
    fn name(&self) -> &str {
        &self.name
    }

    async fn complete(&self, messages: &[Message], tools: &[Value]) -> Result<ModelResponse> {
        let endpoint = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.api_key))
                .map_err(|e| MaxError::Provider(e.to_string()))?,
        );

        let mut payload = json!({
            "model": self.model,
            "messages": messages,
        });

        if !tools.is_empty() {
            payload["tools"] = json!(tools);
        }

        let resp = self
            .client
            .post(&endpoint)
            .headers(headers)
            .json(&payload)
            .send()
            .await
            .map_err(|e| MaxError::Provider(e.to_string()))?;

        if !resp.status().is_success() {
            let error_text = resp.text().await.unwrap_or_default();
            return Err(MaxError::Provider(format!("API Error: {}", error_text)));
        }

        let body: Value = resp
            .json()
            .await
            .map_err(|e| MaxError::Provider(e.to_string()))?;

        let choice = body
            .get("choices")
            .and_then(|c| c.get(0))
            .ok_or_else(|| MaxError::Provider("Empty choices in response".to_string()))?;

        let message_val = choice
            .get("message")
            .ok_or_else(|| MaxError::Provider("No message in choice".to_string()))?;

        let text = message_val
            .get("content")
            .and_then(|c| c.as_str())
            .map(|s| s.to_string());

        let reasoning = message_val
            .get("reasoning")
            .or_else(|| message_val.get("reasoning_content"))
            .and_then(|r| r.as_str())
            .map(|s| s.to_string());

        let mut tool_calls = Vec::new();
        if let Some(tcs) = message_val.get("tool_calls").and_then(|t| t.as_array()) {
            for tc in tcs {
                if let (Some(id), Some(fn_obj)) = (
                    tc.get("id").and_then(|i| i.as_str()),
                    tc.get("function"),
                ) {
                    if let (Some(name), Some(args)) = (
                        fn_obj.get("name").and_then(|n| n.as_str()),
                        fn_obj.get("arguments").and_then(|a| a.as_str()),
                    ) {
                        tool_calls.push(ToolCall {
                            id: id.to_string(),
                            call_type: "function".to_string(),
                            function: FunctionCall {
                                name: name.to_string(),
                                arguments: args.to_string(),
                            },
                        });
                    }
                }
            }
        }

        let prompt_tokens = body
            .get("usage")
            .and_then(|u| u.get("prompt_tokens"))
            .and_then(|t| t.as_u64());

        let completion_tokens = body
            .get("usage")
            .and_then(|u| u.get("completion_tokens"))
            .and_then(|t| t.as_u64());

        Ok(ModelResponse {
            text,
            tool_calls,
            reasoning,
            prompt_tokens,
            completion_tokens,
        })
    }
}
