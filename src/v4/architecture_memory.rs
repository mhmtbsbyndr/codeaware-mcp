use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureRule {
    pub module: String,
    pub rule: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub id: String,
    pub title: String,
    pub decision: String,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArchitectureMemory {
    pub rules: Vec<ArchitectureRule>,
    pub decisions: Vec<DecisionRecord>,
}

impl ArchitectureMemory {
    pub fn add_rule(&mut self, module: impl Into<String>, rule: impl Into<String>, reason: impl Into<String>) {
        self.rules.push(ArchitectureRule {
            module: module.into(),
            rule: rule.into(),
            reason: reason.into(),
        });
    }

    pub fn add_decision(&mut self, title: impl Into<String>, decision: impl Into<String>, rationale: impl Into<String>) {
        self.decisions.push(DecisionRecord {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.into(),
            decision: decision.into(),
            rationale: rationale.into(),
        });
    }

    pub fn rules_for_module(&self, module: &str) -> Vec<ArchitectureRule> {
        self.rules
            .iter()
            .filter(|rule| rule.module == module || rule.module == "*")
            .cloned()
            .collect()
    }
}
