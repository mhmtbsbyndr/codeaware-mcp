pub use crate::session::memory_blocks::{
    BlockName,
    MemoryBlockStore,
    MemoryBlockError,
    MemoryDelivery,
};

/// Returns the current memory delivery for all known block names, collecting
/// only blocks that have content to deliver (non-empty after diff check).
pub fn gather_context(store: &mut MemoryBlockStore) -> Vec<MemoryDelivery> {
    let all_blocks = [
        BlockName::ProjectContext,
        BlockName::SessionPatterns,
        BlockName::PendingItems,
        BlockName::UserPreferences,
    ];

    all_blocks
        .into_iter()
        .map(|name| store.read(name))
        .filter(|d| !d.content.is_empty() || d.full)
        .collect()
}
