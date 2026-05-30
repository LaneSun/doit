use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: Option<String>,
    pub reasoning_content: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub reasoning: Option<String>,
    pub cmd: Option<String>,
    pub tool_call_id: Option<String>,
    pub content: Option<String>,
    pub is_prompt: bool,
}

/// 流式增量事件:后端边收边回调,前端据此实时渲染。
pub enum StreamEvent<'a> {
    /// 思维链增量(灰色显示)
    Reasoning(&'a str),
    /// 对话内容增量(橘色块显示)
    Content(&'a str),
    /// 命令增量(从工具调用参数中实时解码出的 command 字段增量)
    Command(&'a str),
}
