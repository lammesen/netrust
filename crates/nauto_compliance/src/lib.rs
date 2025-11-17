use nauto_model::ComplianceRule;
use serde::Serialize;
use std::collections::HashMap;
use thiserror::Error;

pub type DeviceConfigs = HashMap<String, String>;

#[derive(Debug, Clone, Serialize)]
pub struct RuleOutcome {
    pub device_id: String,
    pub rule: String,
    pub passed: bool,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComplianceSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
}

#[derive(Debug, Error)]
pub enum ComplianceError {
    #[error("csv error: {0}")]
    Csv(#[from] csv::Error),
}

pub struct ComplianceEngine;

impl ComplianceEngine {
    pub fn evaluate(rules: &[ComplianceRule], dataset: &DeviceConfigs) -> Vec<RuleOutcome> {
        let mut outcomes = Vec::new();
        for (device_id, config) in dataset {
            for rule in rules {
                let (passed, detail) = evaluate_expression(&rule.expression, config);
                outcomes.push(RuleOutcome {
                    device_id: device_id.clone(),
                    rule: rule.name.clone(),
                    passed,
                    details: detail,
                });
            }
        }
        outcomes
    }

    pub fn summarize(outcomes: &[RuleOutcome]) -> ComplianceSummary {
        let total = outcomes.len();
        let passed = outcomes.iter().filter(|o| o.passed).count();
        ComplianceSummary {
            total,
            passed,
            failed: total.saturating_sub(passed),
        }
    }

    pub fn export_json(outcomes: &[RuleOutcome]) -> serde_json::Value {
        serde_json::json!({
            "summary": Self::summarize(outcomes),
            "results": outcomes,
        })
    }

    pub fn export_csv<W: std::io::Write>(
        outcomes: &[RuleOutcome],
        mut writer: csv::Writer<W>,
    ) -> Result<(), ComplianceError> {
        writer.write_record(["device_id", "rule", "passed", "details"])?;
        for outcome in outcomes {
            writer.write_record([
                outcome.device_id.as_str(),
                outcome.rule.as_str(),
                if outcome.passed { "true" } else { "false" },
                outcome.details.as_deref().unwrap_or(""),
            ])?;
        }
        writer.flush().map_err(csv::Error::from)?;
        Ok(())
    }
}

fn evaluate_expression(expression: &str, config: &str) -> (bool, Option<String>) {
    if let Some(rest) = expression.strip_prefix("not:") {
        let found = config.contains(rest);
        (
            !found,
            if found {
                Some(format!("found forbidden pattern {}", rest))
            } else {
                None
            },
        )
    } else if let Some(rest) = expression.strip_prefix("contains:") {
        let found = config.contains(rest);
        (
            found,
            if found {
                None
            } else {
                Some(format!("missing required pattern {}", rest))
            },
        )
    } else {
        let found = config.contains(expression);
        (
            found,
            if found {
                None
            } else {
                Some(format!("missing required pattern {}", expression))
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluates_contains_and_not() {
        let rules = vec![
            ComplianceRule {
                name: "Require NTP".into(),
                description: "".into(),
                expression: "ntp server".into(),
            },
            ComplianceRule {
                name: "No Telnet".into(),
                description: "".into(),
                expression: "not:line vty 0 4\n transport input telnet".into(),
            },
        ];
        let mut dataset = DeviceConfigs::new();
        dataset.insert(
            "r1".into(),
            "ntp server 1.1.1.1\nline vty 0 4\n transport input ssh".into(),
        );
        dataset.insert("r2".into(), "interface Gi1/0\n description test".into());

        let outcomes = ComplianceEngine::evaluate(&rules, &dataset);
        let summary = ComplianceEngine::summarize(&outcomes);
        assert_eq!(summary.total, 4);
        assert_eq!(summary.failed, 1);
    }
}
