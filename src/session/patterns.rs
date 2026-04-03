use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternType {
    CoAccess,
    ErrorFix,
    EditSequence,
    TestFirst,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LearnedPattern {
    pattern_type: PatternType,
    evidence_count: u32,
    confidence: f32,
    data: PatternData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum PatternData {
    CoAccess { file_a: String, file_b: String },
    ErrorFix { signature: String, fix: String },
    EditSequence { files: Vec<String> },
    TestFirst { observed: bool },
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PatternStore {
    patterns: Vec<LearnedPattern>,
}

impl PatternStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_co_access(&mut self, file_a: &str, file_b: &str) {
        if let Some(p) = self.find_co_access_mut(file_a, file_b) {
            p.evidence_count += 1;
            p.confidence = (p.confidence + 0.1).min(1.0);
        } else {
            self.patterns.push(LearnedPattern {
                pattern_type: PatternType::CoAccess,
                evidence_count: 1,
                confidence: 0.3,
                data: PatternData::CoAccess {
                    file_a: file_a.into(),
                    file_b: file_b.into(),
                },
            });
        }
    }

    pub fn record_error_fix(&mut self, signature: &str, fix: &str) {
        if let Some(p) = self.find_error_fix_mut(signature) {
            p.evidence_count += 1;
            p.confidence = (p.confidence + 0.1).min(1.0);
            if let PatternData::ErrorFix { fix: ref mut f, .. } = p.data {
                *f = fix.to_string();
            }
        } else {
            self.patterns.push(LearnedPattern {
                pattern_type: PatternType::ErrorFix,
                evidence_count: 1,
                confidence: 0.5,
                data: PatternData::ErrorFix {
                    signature: signature.into(),
                    fix: fix.into(),
                },
            });
        }
    }

    pub fn get_co_access_patterns(&self, file: &str) -> Vec<(String, u32)> {
        self.patterns
            .iter()
            .filter_map(|p| match &p.data {
                PatternData::CoAccess { file_a, file_b }
                    if file_a == file || file_b == file =>
                {
                    let other = if file_a == file { file_b } else { file_a };
                    Some((other.clone(), p.evidence_count))
                }
                _ => None,
            })
            .collect()
    }

    pub fn get_known_fix(&self, signature: &str) -> Option<String> {
        self.patterns.iter().find_map(|p| match &p.data {
            PatternData::ErrorFix {
                signature: s,
                fix,
            } if s == signature => Some(fix.clone()),
            _ => None,
        })
    }

    pub fn get_confidence(&self, file_a: &str, file_b: &str) -> Option<f32> {
        self.find_co_access(file_a, file_b).map(|p| p.confidence)
    }

    pub fn apply_decay(&mut self, rate: f32) {
        for p in &mut self.patterns {
            p.confidence *= 1.0 - rate;
        }
    }

    pub fn prune(&mut self, threshold: f32) {
        self.patterns.retain(|p| p.confidence >= threshold);
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    fn find_co_access(&self, a: &str, b: &str) -> Option<&LearnedPattern> {
        self.patterns.iter().find(|p| match &p.data {
            PatternData::CoAccess { file_a, file_b } => {
                (file_a == a && file_b == b) || (file_a == b && file_b == a)
            }
            _ => false,
        })
    }

    fn find_co_access_mut(&mut self, a: &str, b: &str) -> Option<&mut LearnedPattern> {
        self.patterns.iter_mut().find(|p| match &p.data {
            PatternData::CoAccess { file_a, file_b } => {
                (file_a == a && file_b == b) || (file_a == b && file_b == a)
            }
            _ => false,
        })
    }

    fn find_error_fix_mut(&mut self, sig: &str) -> Option<&mut LearnedPattern> {
        self.patterns.iter_mut().find(|p| match &p.data {
            PatternData::ErrorFix { signature, .. } => signature == sig,
            _ => false,
        })
    }
}
