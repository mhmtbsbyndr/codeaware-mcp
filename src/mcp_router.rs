#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpServerRoute {
    pub server_name: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpRoutingDecision {
    pub task: String,
    pub selected_server: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpRouter {
    pub routes: Vec<McpServerRoute>,
}

impl McpRouter {
    pub fn new(routes: Vec<McpServerRoute>) -> Self {
        Self { routes }
    }

    pub fn route_task(&self, task: &str) -> Option<McpRoutingDecision> {
        let lowered = task.to_lowercase();

        for route in &self.routes {
            for capability in &route.capabilities {
                if lowered.contains(&capability.to_lowercase()) {
                    return Some(McpRoutingDecision {
                        task: task.to_string(),
                        selected_server: route.server_name.clone(),
                        reason: format!(
                            "Matched capability '{}' on server '{}'",
                            capability, route.server_name
                        ),
                    });
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_task_by_capability() {
        let router = McpRouter::new(vec![McpServerRoute {
            server_name: "codeaware".to_string(),
            capabilities: vec!["code".to_string(), "symbol".to_string()],
        }]);

        let decision = router.route_task("analyze code symbols").unwrap();

        assert_eq!(decision.selected_server, "codeaware");
    }
}
