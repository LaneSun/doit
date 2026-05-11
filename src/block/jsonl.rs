use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use crate::error::Result;

use super::Block;

pub fn append(path: &Path, block: &Block) -> Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| crate::error::DoitError::io(e, "cannot open JSONL for append"))?;

    let line = serde_json::to_string(block)
        .map_err(|e| crate::error::DoitError::session(format!("serialize: {e}")))?;
    writeln!(file, "{}", line).map_err(|e| crate::error::DoitError::io(e, "cannot write JSONL"))?;

    Ok(())
}

pub fn load(path: &Path) -> Result<Vec<Block>> {
    let file = File::open(path).map_err(|e| crate::error::DoitError::io(e, "cannot open JSONL"))?;
    let reader = BufReader::new(file);

    let mut blocks = Vec::new();
    for line in reader.lines() {
        let line = line.map_err(|e| crate::error::DoitError::io(e, "cannot read JSONL line"))?;
        let block: Block = serde_json::from_str(&line)
            .map_err(|e| crate::error::DoitError::session(format!("deserialize: {e}")))?;
        blocks.push(block);
    }
    Ok(blocks)
}

pub fn load_safe(path: &Path) -> Result<Vec<Block>> {
    let content = fs::read_to_string(path)
        .map_err(|e| crate::error::DoitError::io(e, "cannot read JSONL"))?;

    let mut blocks = Vec::new();
    for (i, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Block>(line) {
            Ok(block) => blocks.push(block),
            Err(e) => {
                if i == content.lines().count() - 1 {
                    // Last line incomplete — discard silently
                    break;
                }
                return Err(crate::error::DoitError::session(format!(
                    "deserialize line {i}: {e}"
                )));
            }
        }
    }
    Ok(blocks)
}

/// Strip ANSI escape sequences for LLM content
pub fn strip_ansi(s: &str) -> String {
    let re = regex::Regex::new("\x1b\\[[0-9;]*[a-zA-Z]").unwrap();
    re.replace_all(s, "").into_owned()
}
