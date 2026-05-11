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
                        arguments: format!(r#"{{"command":"{}"}}"#, cmd),
                    },
                }]
            });

            let api_content = content.as_ref().map(|_| "".to_string());

            ChatMessage {
                role: Role::Assistant,
                content: api_content,
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
