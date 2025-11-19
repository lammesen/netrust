# Notifications & Approvals

## CLI Support

### Notifications
```bash
nauto_cli notify --channel slack --message "Job Pending Approval" \
  --webhook https://hooks.slack.com/services/XXX --dry-run
```
- Without `--webhook`, messages print to stdout (useful for local testing).
- With `--webhook`, payload is POSTed using `reqwest`.

### Approvals
```bash
# create request
nauto_cli approvals request --job jobs/ntp.yaml --requested-by alice --note "Batch change"

# list
nauto_cli approvals list

# approve
nauto_cli approvals approve --id <uuid> --approver bob
```
- Records stored in `approvals/approvals.json`.
- Job path is validated before request is accepted.

## GUI Roadmap
- Add approval modal in Tauri app listing pending requests.
- Link scheduler and job wizard to notifications (auto ping Slack on submit).

## Future Enhancements
- Enforce approval requirement flag on jobs before execution.
- Integrate with external ticketing (ServiceNow) via approvals command.