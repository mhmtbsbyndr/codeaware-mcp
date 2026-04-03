use serde::{Serialize, Deserialize};
use std::collections::HashMap;

const MAX_BLOCK_CHARS: usize = 10_000;

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum BlockName {
    ProjectContext,
    SessionPatterns,
    PendingItems,
    UserPreferences,
}

impl BlockName {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "project_context" => Some(Self::ProjectContext),
            "session_patterns" => Some(Self::SessionPatterns),
            "pending_items" => Some(Self::PendingItems),
            "user_preferences" => Some(Self::UserPreferences),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct MemoryDelivery {
    pub block: BlockName,
    pub version: u64,
    pub full: bool,
    pub content: String,
}

struct BlockState {
    content: String,
    version: u64,
    last_delivered_version: u64,
}

pub struct MemoryBlockStore {
    blocks: HashMap<BlockName, BlockState>,
}

impl MemoryBlockStore {
    pub fn new() -> Self {
        Self { blocks: HashMap::new() }
    }

    pub fn write(&mut self, name: BlockName, content: &str) {
        let entry = self.blocks.entry(name).or_insert(BlockState {
            content: String::new(),
            version: 0,
            last_delivered_version: 0,
        });
        entry.content = content.to_string();
        entry.version += 1;
    }

    pub fn try_write(&mut self, name: BlockName, content: &str) -> Result<(), MemoryBlockError> {
        if content.len() > MAX_BLOCK_CHARS {
            return Err(MemoryBlockError::ContentTooLarge(content.len(), MAX_BLOCK_CHARS));
        }
        self.write(name, content);
        Ok(())
    }

    /// Returns the current delivery for the block and marks it as delivered.
    /// - If block has never been written: returns full=true, content=""
    /// - If block has been written but never delivered (or changed since last delivery): returns content
    ///   - full=true if never delivered before, full=false if this is a re-delivery after change
    /// - If block has not changed since last delivery: returns full=false, content=""
    pub fn read(&mut self, name: BlockName) -> MemoryDelivery {
        match self.blocks.get_mut(&name) {
            None => MemoryDelivery {
                block: name,
                version: 0,
                full: true,
                content: String::new(),
            },
            Some(state) => {
                if state.last_delivered_version == state.version {
                    // No changes since last delivery
                    MemoryDelivery {
                        block: name,
                        version: state.version,
                        full: false,
                        content: String::new(),
                    }
                } else {
                    // Changed (or first time): deliver content
                    let is_first_delivery = state.last_delivered_version == 0;
                    let content = state.content.clone();
                    let version = state.version;
                    state.last_delivered_version = version;
                    MemoryDelivery {
                        block: name,
                        version,
                        full: is_first_delivery,
                        content,
                    }
                }
            }
        }
    }

    pub fn validate_content(&self, content: &str) -> Result<(), MemoryBlockError> {
        if content.len() > MAX_BLOCK_CHARS {
            return Err(MemoryBlockError::ContentTooLarge(content.len(), MAX_BLOCK_CHARS));
        }
        Ok(())
    }
}

impl Default for MemoryBlockStore {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MemoryBlockError {
    #[error("Content too large: {0} chars (max {1})")]
    ContentTooLarge(usize, usize),
}
