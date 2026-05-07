use codeaware_mcp::v4::{BudgetState, TaskIntent, V4Tools};

#[test]
fn budget_check_passes_within_limits() {
    let budget = BudgetState {
        task_id: "task-1".to_string(),
        files_read: 2,
        files_changed: 1,
        tool_calls: 3,
        estimated_context_tokens: 1000,
        max_files_read: 8,
        max_files_changed: 4,
        max_tool_calls: 12,
        max_context_tokens: 30000,
    };

    let result = budget.check();
    assert!(result.ok);
}

#[test]
fn default_contract_has_safe_limits() {
    let contract = V4Tools::default_contract(
        "Implement context cache".to_string(),
        TaskIntent::ImplementFeature,
    );

    assert_eq!(contract.scope.max_files_read, 8);
    assert_eq!(contract.scope.max_files_changed, 4);
    assert_eq!(contract.scope.max_tool_calls, 12);
}
