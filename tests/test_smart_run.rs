use codeaware_mcp::tools::smart_run::{smart_run, SmartRunInput};

#[tokio::test]
async fn test_run_echo() {
    let input = SmartRunInput {
        command: "echo hello".into(),
        max_output_lines: 50,
        capture_relevant_code: false,
        scan_secrets: false,
    };
    let result = smart_run(&input).await.unwrap();
    assert_eq!(result.exit_code, 0);
    assert_eq!(result.command_type, "generic");
    assert!(result.summary.contains("hello") || result.compressed_output.contains("hello"));
}

#[tokio::test]
async fn test_run_nonexistent_command() {
    let input = SmartRunInput {
        command: "nonexistent_cmd_xyz_12345".into(),
        max_output_lines: 50,
        capture_relevant_code: false,
        scan_secrets: false,
    };
    let result = smart_run(&input).await.unwrap();
    assert_ne!(result.exit_code, 0);
}

#[tokio::test]
async fn test_generic_compression_head_tail() {
    let input = SmartRunInput {
        command: "seq 1 200".into(),
        max_output_lines: 50,
        capture_relevant_code: false,
        scan_secrets: false,
    };
    let result = smart_run(&input).await.unwrap();
    assert_eq!(result.exit_code, 0);
    assert!(result.raw_lines >= 200);
    assert!(result.compressed_lines < result.raw_lines);
    assert!(result.compressed_output.contains("1\n"));
    assert!(result.compressed_output.contains("200"));
}

#[tokio::test]
async fn test_secret_scanning_in_output() {
    let input = SmartRunInput {
        command: "echo API_KEY=sk-abc123def456ghi789jkl012mno345pqr678".into(),
        max_output_lines: 50,
        capture_relevant_code: false,
        scan_secrets: true,
    };
    let result = smart_run(&input).await.unwrap();
    assert!(result.secrets_detected);
}

#[tokio::test]
async fn test_command_type_detection() {
    use codeaware_mcp::compressor::classify_command;
    assert_eq!(classify_command("cargo test"), "test_runner");
    assert_eq!(classify_command("pytest -v"), "test_runner");
    assert_eq!(classify_command("cargo build"), "compiler");
    assert_eq!(classify_command("cargo clippy"), "linter");
    assert_eq!(classify_command("git status"), "git_info");
    assert_eq!(classify_command("npm install"), "package_mgr");
    assert_eq!(classify_command("rustfmt src/main.rs"), "formatter");
    assert_eq!(classify_command("my-custom-script"), "generic");
}
