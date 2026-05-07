use codeaware_mcp::v4::{GetTaskContextRequest, TaskIntent, V4Tools};

#[test]
fn get_task_context_returns_contract_and_instructions() {
    let response = V4Tools::get_task_context(
        GetTaskContextRequest {
            task_id: None,
            goal: "Implement context package assembly".to_string(),
            intent: TaskIntent::ImplementFeature,
        },
        "/tmp/repo".to_string(),
    );

    assert!(!response.task_id.is_empty());
    assert!(!response.agent_instructions.is_empty());
    assert_eq!(response.context_package.repo_root, "/tmp/repo");
}

#[test]
fn task_context_contains_excluded_paths() {
    let response = V4Tools::get_task_context(
        GetTaskContextRequest {
            task_id: None,
            goal: "Analyze memory layer".to_string(),
            intent: TaskIntent::Analyze,
        },
        "/workspace/project".to_string(),
    );

    assert!(!response.context_package.excluded_context.is_empty());
}
