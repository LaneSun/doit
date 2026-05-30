use futures_util::StreamExt;
use reqwest::Client;
use serde_json::{Value, json};

use super::types::{ChatMessage, ChatResponse, Role, StreamEvent};
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

    /// 流式对话:逐 SSE chunk 解析增量,通过 `on_event` 实时回调思维/内容/命令,
    /// 结束后汇总为与 `chat` 一致的 `ChatResponse`(供会话记账与执行)。
    pub async fn chat_stream(
        &self,
        messages: &[ChatMessage],
        mut on_event: impl FnMut(StreamEvent),
    ) -> Result<ChatResponse> {
        let api_messages: Vec<Value> = messages.iter().map(|m| self.build_api_message(m)).collect();
        let body = json!({
            "model": self.model,
            "thinking": {"type": "enabled"},
            "messages": api_messages,
            "tools": [self.sh_tool_definition()],
            "stream": true,
        });

        let response = self
            .client
            .post(format!("{}/chat/completions", self.api_base))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| crate::error::DoitError::backend(format!("API request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(crate::error::DoitError::backend(format!(
                "API error {status}: {text}"
            )));
        }

        let mut stream = response.bytes_stream();
        let mut buf = String::new();
        let mut reasoning = String::new();
        let mut content = String::new();
        let mut args = String::new(); // 累积的工具调用参数(JSON 片段)
        let mut cmd_emitted = 0usize; // 已发出的 command 解码长度(字节)
        let mut tool_call_id: Option<String> = None;
        let mut saw_tool = false;

        while let Some(chunk) = stream.next().await {
            let chunk =
                chunk.map_err(|e| crate::error::DoitError::backend(format!("stream read: {e}")))?;
            buf.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(nl) = buf.find('\n') {
                let line: String = buf.drain(..=nl).collect();
                let line = line.trim();
                let Some(data) = line.strip_prefix("data:") else {
                    continue;
                };
                let data = data.trim();
                if data == "[DONE]" {
                    continue;
                }
                let Ok(v) = serde_json::from_str::<Value>(data) else {
                    continue;
                };
                if let Some(err) = v.get("error") {
                    return Err(crate::error::DoitError::backend(format!(
                        "API error: {}",
                        err["message"].as_str().unwrap_or("unknown")
                    )));
                }
                let delta = &v["choices"][0]["delta"];

                if let Some(r) = delta["reasoning_content"].as_str() {
                    if !r.is_empty() {
                        reasoning.push_str(r);
                        on_event(StreamEvent::Reasoning(r));
                    }
                }
                if let Some(c) = delta["content"].as_str() {
                    if !c.is_empty() {
                        content.push_str(c);
                        on_event(StreamEvent::Content(c));
                    }
                }
                if let Some(tcs) = delta["tool_calls"].as_array() {
                    saw_tool = true;
                    for tc in tcs {
                        if let Some(id) = tc["id"].as_str() {
                            if !id.is_empty() {
                                tool_call_id = Some(id.to_string());
                            }
                        }
                        if let Some(a) = tc["function"]["arguments"].as_str() {
                            args.push_str(a);
                            if let Some(decoded) = decode_partial_command(&args) {
                                if decoded.len() > cmd_emitted {
                                    on_event(StreamEvent::Command(&decoded[cmd_emitted..]));
                                    cmd_emitted = decoded.len();
                                }
                            }
                        }
                    }
                }
            }
        }

        let opt = |s: String| if s.is_empty() { None } else { Some(s) };
        if saw_tool {
            let cmd = decode_partial_command(&args).unwrap_or(args);
            Ok(ChatResponse {
                reasoning: opt(reasoning),
                cmd: Some(cmd),
                tool_call_id,
                content: None,
                is_prompt: false,
            })
        } else {
            Ok(ChatResponse {
                reasoning: opt(reasoning),
                cmd: None,
                tool_call_id: None,
                content: opt(content),
                is_prompt: true,
            })
        }
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

}

/// 从增量累积的工具调用参数 JSON 中,解码出 `command` 字段当前可安全确定的前缀。
///
/// 参数形如 `{"command": "ls -la`(可能尚未闭合)。我们定位到值的起始引号后,
/// 逐字符解码直到遇到未转义的闭合引号或字符串结束;遇到不完整的转义序列(尾随 `\`)
/// 则停止,保证返回值始终是最终命令的一个前缀(单调增长),可安全计算增量。
fn decode_partial_command(args: &str) -> Option<String> {
    let key = args.find("\"command\"")?;
    let after_key = &args[key + "\"command\"".len()..];
    let colon = after_key.find(':')?;
    let after_colon = &after_key[colon + 1..];
    let open = after_colon.find('"')?;
    let val = &after_colon[open + 1..];

    let mut out = String::new();
    let mut chars = val.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => break, // 未转义的闭合引号
            '\\' => match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some('/') => out.push('/'),
                Some('b') => out.push('\u{0008}'),
                Some('f') => out.push('\u{000C}'),
                Some('u') => {
                    // 读 4 位十六进制;不完整则停止(下次更多数据到达再解码)
                    let hex: String = (0..4).filter_map(|_| chars.next()).collect();
                    if hex.len() < 4 {
                        break;
                    }
                    if let Some(ch) = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                        out.push(ch);
                    }
                }
                Some(other) => out.push(other),
                None => break, // 尾随反斜杠,转义未完成
            },
            _ => out.push(c),
        }
    }
    Some(out)
}
