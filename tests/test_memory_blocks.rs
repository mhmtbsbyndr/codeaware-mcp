use codeaware_mcp::session::memory_blocks::{MemoryBlockStore, BlockName};

#[test]
fn test_write_and_read_block() {
    let mut store = MemoryBlockStore::new();
    store.write(BlockName::ProjectContext, "Initial project context");
    let delivery = store.read(BlockName::ProjectContext);
    assert!(delivery.full);
    assert_eq!(delivery.content, "Initial project context");
    assert_eq!(delivery.version, 1);
}

#[test]
fn test_second_read_returns_no_change() {
    let mut store = MemoryBlockStore::new();
    store.write(BlockName::ProjectContext, "Context");
    let _ = store.read(BlockName::ProjectContext); // first read
    let delivery = store.read(BlockName::ProjectContext); // second read
    assert!(!delivery.full);
    assert_eq!(delivery.content, ""); // no changes
}

#[test]
fn test_write_then_read_returns_diff() {
    let mut store = MemoryBlockStore::new();
    store.write(BlockName::ProjectContext, "Version 1");
    let _ = store.read(BlockName::ProjectContext);
    store.write(BlockName::ProjectContext, "Version 2");
    let delivery = store.read(BlockName::ProjectContext);
    assert!(!delivery.full);
    assert!(delivery.content.contains("Version 2"));
    assert_eq!(delivery.version, 2);
}

#[test]
fn test_unread_block_returns_empty() {
    let mut store = MemoryBlockStore::new();
    let delivery = store.read(BlockName::SessionPatterns);
    assert!(delivery.full);
    assert_eq!(delivery.content, "");
}

#[test]
fn test_all_block_names() {
    let mut store = MemoryBlockStore::new();
    store.write(BlockName::ProjectContext, "a");
    store.write(BlockName::SessionPatterns, "b");
    store.write(BlockName::PendingItems, "c");
    store.write(BlockName::UserPreferences, "d");
    assert_eq!(store.read(BlockName::ProjectContext).content, "a");
    assert_eq!(store.read(BlockName::SessionPatterns).content, "b");
    assert_eq!(store.read(BlockName::PendingItems).content, "c");
    assert_eq!(store.read(BlockName::UserPreferences).content, "d");
}

#[test]
fn test_content_max_length() {
    let mut store = MemoryBlockStore::new();
    let long = "x".repeat(10_001);
    let result = store.try_write(BlockName::ProjectContext, &long);
    assert!(result.is_err());
}
