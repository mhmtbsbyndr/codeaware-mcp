# codeaware-mcp v2.1.1

Released: 2026-04-06

## What's Changed

### Robustness
- Replace 39 bare `.unwrap()` calls with descriptive panic messages
- Fix Linux platform compatibility in xray/server.rs (sockaddr_in)

### Architecture
- Extract tool dispatch from server.rs into tools/dispatch.rs (~100 LOC reduction)
- Extract post-tool metrics/hooks into dedicated helper method

### Performance
- Move regex compilation out of hot loop in error_signature

### Testing
- 30+ new unit tests: DenyList, SecretScanner, command classifier
- 52 lib tests (up from ~20), 0 clippy warnings

### Docs
- Updated architecture diagram with dispatch.rs, hooks module, all 17 tools, 10 languages
- Updated test coverage section
