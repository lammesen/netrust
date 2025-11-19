# Compliance Engine (Milestone 1)

## Engine
- Crate: `crates/nauto_compliance`
- Supports substring-based expressions:
  - `contains:foo` – device config must contain `foo`
  - `not:bar` – config must NOT contain `bar`
  - bare string fallback uses `contains`
- Outputs per-device / per-rule outcomes and summary stats.

## CLI Usage
```bash
nauto_cli compliance \
  --rules examples/compliance_rules.yaml \
  --inputs examples/compliance_inputs.yaml \
  --format json \
  --output compliance_report.json
```

CSV export:
```bash
nauto_cli compliance --rules ... --inputs ... --format csv > report.csv
```

## File Formats
- Rules (`examples/compliance_rules.yaml`): list of `ComplianceRule` entries.
- Inputs (`examples/compliance_inputs.yaml`): map of device IDs to config snippets.

## Next Steps
- Support richer rule language (regex, numeric comparisons).
- Integrate with job engine outputs to fetch configs automatically.
- Surface results in GUI dashboard.