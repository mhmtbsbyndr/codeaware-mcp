#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompressionExperiment {
    pub experiment_id: String,
    pub pipeline: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExperimentResult {
    pub pipeline: String,
    pub average_tokens: f64,
    pub quality_good_ratio: f64,
    pub test_pass_ratio: f64,
}

pub fn select_pipeline(index: usize, pipelines: &[CompressionExperiment]) -> Option<String> {
    if pipelines.is_empty() {
        return None;
    }

    let enabled: Vec<&CompressionExperiment> = pipelines
        .iter()
        .filter(|pipeline| pipeline.enabled)
        .collect();

    if enabled.is_empty() {
        return None;
    }

    let selected = enabled[index % enabled.len()];
    Some(selected.pipeline.clone())
}

pub fn render_experiment_report(results: &[ExperimentResult]) -> String {
    let mut output = String::new();

    output.push_str("# Compression Experiment Report\n\n");

    for result in results {
        output.push_str(&format!(
            "Pipeline: {}\n  - Average tokens: {:.2}\n  - Quality GOOD: {:.2}%\n  - Tests pass: {:.2}%\n\n",
            result.pipeline,
            result.average_tokens,
            result.quality_good_ratio * 100.0,
            result.test_pass_ratio * 100.0,
        ));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_enabled_pipeline() {
        let pipelines = vec![
            CompressionExperiment {
                experiment_id: "1".to_string(),
                pipeline: "ast_diff_only".to_string(),
                enabled: true,
            },
            CompressionExperiment {
                experiment_id: "2".to_string(),
                pipeline: "git_only".to_string(),
                enabled: true,
            },
        ];

        let selected = select_pipeline(1, &pipelines).unwrap();

        assert_eq!(selected, "git_only");
    }

    #[test]
    fn renders_report() {
        let report = render_experiment_report(&[ExperimentResult {
            pipeline: "ast_diff_only".to_string(),
            average_tokens: 1200.0,
            quality_good_ratio: 0.85,
            test_pass_ratio: 0.92,
        }]);

        assert!(report.contains("Compression Experiment Report"));
        assert!(report.contains("ast_diff_only"));
    }
}
