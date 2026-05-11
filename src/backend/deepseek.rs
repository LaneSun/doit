use reqwest::Client;
use serde_json::{Value, json};

use super::types::{ChatMessage, ChatResponse, Role};
use crate::error::Result;

pub struct DeepSeekBackend {
    client: Client,
    api_base: String,
    api_key: String,
    model: String,
}

impl DeepSeekBackend {
    pub fn new(api_base: String, api_key: String, model: String) -> Self {
        Self {
            client: Client::new(),
            api_base,
            api_key,
            model,
        }
    }

    pub async fn chat(&self, messages: &[ChatMessage]) -> Result<ChatResponse> {
        let api_messages: Vec<Value> = messages.iter().map(|m| self.build_api_message(m)).collect();

        let body = json!({
            "model": self.model,
            "thinking": {"type": "enabled"},
            "messages": api_messages,
            "tools": [self.sh_tool_definition()],
        });

        tracing::debug!("API request: {} messages", api_messages.len());
        let response = self
            .client
            .post(format!("{}/chat/completions", self.api_base))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| crate::error::DoitError::backend(format!("API request failed: {e}")))?;
        tracing::debug!("API response status: {}", response.status());

        let raw: Value = response
            .json()
            .await
            .map_err(|e| crate::error::DoitError::backend(format!("API response parse: {e}")))?;

        self.parse_response(&raw)
    }

    fn build_api_message(&self, msg: &ChatMessage) -> Value {
        match msg.role {
            Role::System => json!({
                "role": "system",
                "content": msg.content,
            }),
            Role::Assistant => {
                let mut m = json!({
                    "role": "assistant",
                    "content": msg.content,
                });
                if let Some(ref reasoning) = msg.reasoning_content {
                    m["reasoning_content"] = json!(reasoning);
                }
                if let Some(ref tool_calls) = msg.tool_calls {
                    m["tool_calls"] = json!(tool_calls);
                }
                m
            }
            Role::Tool => json!({
                "role": "tool",
                "tool_call_id": msg.tool_call_id,
                "content": msg.content,
            }),
            Role::User => json!({
                "role": "user",
                "content": msg.content,
            }),
        }
    }

    fn sh_tool_definition(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": "sh",
                "description": "Execute a shell command. All doit built-in subcommands are available.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The shell command to execute"
                        }
                    },
                    "required": ["command"]
                }
            }
        })
    }

    fn parse_response(&self, raw: &Value) -> Result<ChatResponse> {
        if let Some(error) = raw.get("error") {
            return Err(crate::error::DoitError::backend(format!(
                "API error: {}",
                error["message"].as_str().unwrap_or("unknown")
            )));
        }

        let choice = &raw["choices"][0];
        let msg = &choice["message"];
        let reasoning = msg["reasoning_content"].as_str().map(|s| s.to_string());

        if let Some(tool_calls) = msg["tool_calls"].as_array() {
            if let Some(tc) = tool_calls.first() {
                let func = &tc["function"];
                return Ok(ChatResponse {
                    reasoning,
                    cmd: func["arguments"].as_str().map(|args| {
                        if let Ok(v) = serde_json::from_str::<Value>(args) {
                            v["command"].as_str().unwrap_or(args).to_string()
                        } else {
                            args.to_string()
                        }
                    }),
                    tool_call_id: tc["id"].as_str().map(|s| s.to_string()),
                    content: None,
                    is_prompt: false,
                });
            }
        }

        // No tool_calls — treat content as prompt
        Ok(ChatResponse {
            reasoning,
            cmd: None,
            tool_call_id: None,
            content: msg["content"].as_str().map(|s| s.to_string()),
            is_prompt: true,
        })
    }
}
