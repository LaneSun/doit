use std::fs;
use std::path::{Path, PathBuf};

use crate::block::Block;
use crate::error::Result;

const SESSION_ROOT: &str = ".doit/sessions";

pub struct Session {
    pub id: String,
    pub dir: PathBuf,
    pub blocks: Vec<Block>,
    pub cwd: PathBuf,
    pub model: String,
    pub created_at: i64,
}

impl Session {
    pub fn create(cwd: &Path, model: &str) -> Result<Self> {
        let root = cwd.join(SESSION_ROOT);
        fs::create_dir_all(&root)
            .map_err(|e| crate::error::DoitError::io(e, "cannot create sessions dir"))?;

        let id = generate_id(&root);
        let dir = root.join(&id);
        fs::create_dir_all(&dir)
            .map_err(|e| crate::error::DoitError::io(e, "cannot create session dir"))?;

        // Create logs dir
        fs::create_dir_all(dir.join("logs")).ok();

        // Create scratchpad with template
        let scratchpad_path = dir.join("scratchpad.md");
        fs::write(&scratchpad_path, "# TODO\n\n- [ ] \n")
            .map_err(|e| crate::error::DoitError::io(e, "cannot write scratchpad"))?;

        Ok(Self {
            id,
            dir,
            blocks: Vec::new(),
            cwd: cwd.to_path_buf(),
            model: model.to_string(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
        })
    }

    pub fn load(dir: &Path) -> Result<Self> {
        let conv_path = dir.join("conversation.jsonl");
        let blocks = crate::block::jsonl::load_safe(&conv_path)?;

        let id = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        Ok(Self {
            id,
            dir: dir.to_path_buf(),
            blocks,
            cwd: PathBuf::from("."),
            model: String::new(),
            created_at: 0,
        })
    }

    pub fn append(&mut self, block: Block) -> Result<()> {
        let conv_path = self.dir.join("conversation.jsonl");
        crate::block::jsonl::append(&conv_path, &block)?;
        self.blocks.push(block);
        Ok(())
    }

    pub fn last_block(&self) -> Option<&Block> {
        self.blocks.last()
    }

    pub fn next_seq(&self) -> u64 {
        self.blocks
            .last()
            .map(|b| b.seq() + 1)
            .unwrap_or(1)
    }

    pub fn build_messages(&self) -> Vec<crate::backend::types::ChatMessage> {
        crate::block::render::to_api_messages(&self.blocks)
    }
}

fn generate_id(root: &Path) -> String {
    loop {
        let id = &uuid::Uuid::new_v4().to_string()[..8];
        let id = id.to_string();
        if !root.join(&id).exists() {
            return id;
        }
    }
}
