use crate::backend::types::{ChatMessage, FunctionCall, Role, ToolCall};

use super::{Block, jsonl::strip_ansi};

pub fn to_api_messages(blocks: &[Block]) -> Vec<ChatMessage> {
    blocks.iter().map(to_api_message).collect()
}

fn to_api_message(block: &Block) -> ChatMessage {
    match block {
        Block::System { content, .. } => ChatMessage {
            role: Role::System,
            content: Some(content.clone()),
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: None,
            name: None,
        },
        Block::User { content, .. } => ChatMessage {
            role: Role::User,
            content: Some(content.clone()),
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: None,
            name: None,
        },
        Block::Assistant {
            reasoning,
            cmd,
            tool_call_id,
            content,
            ..
        } => {
            let tool_calls = tool_call_id.as_ref().map(|id| {
                vec![ToolCall {
                    id: id.clone(),
                    r#type: "function".to_string(),
                    function: FunctionCall {
                        name: "sh".to_string(),
                        // 用 serde_json 安全序列化,避免 cmd 含引号/换行破坏 JSON
                        arguments: serde_json::json!({ "command": cmd }).to_string(),
                    },
                }]
            });

            ChatMessage {
                role: Role::Assistant,
                // 保留 LLM 原始 content(无工具调用时即为发给用户的消息),不可丢失
                content: content.clone(),
                reasoning_content: Some(reasoning.clone()),
                tool_calls,
                tool_call_id: None,
                name: None,
            }
        }
        Block::Tool {
            output,
            tool_call_id,
            ..
        } => ChatMessage {
            role: Role::Tool,
            content: Some(strip_ansi(output)),
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: Some(tool_call_id.clone()),
            name: None,
        },
    }
}
