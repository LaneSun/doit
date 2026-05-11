pub mod jsonl;
pub mod render;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role")]
pub enum Block {
    #[serde(rename = "system")]
    System { seq: u64, content: String },
    #[serde(rename = "user")]
    User { seq: u64, content: String },
    #[serde(rename = "assistant")]
    Assistant {
        seq: u64,
        reasoning: String,
        cmd: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_call_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<String>,
    },
    #[serde(rename = "tool")]
    Tool {
        seq: u64,
        output: String,
        exit_code: i32,
        tool_call_id: String,
    },
}

impl Block {
    pub fn seq(&self) -> u64 {
        match self {
            Block::System { seq, .. }
            | Block::User { seq, .. }
            | Block::Assistant { seq, .. }
            | Block::Tool { seq, .. } => *seq,
        }
    }
}
