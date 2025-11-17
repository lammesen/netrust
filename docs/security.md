# Security & Compliance Summary

## Credentials
- Stored via `nauto_security::KeyringStore`, which wraps OS keychain providers.
- CLI command `nauto_cli creds` writes credentials to vault; devices store only references.
- Interactive usage now prompts for the password by default (`--password-prompt`), while automation can use `--password-stdin`; passing `--password` directly is allowed but prints a warning about argv exposure.

## Safeguards
- Audit log writer (`logs/audit.log`) captures job summary per execution.
- Dry-run flag and driver capability checks prevent unintended changes on unsupported devices.
- Read-only mode (CLI) achieved via `--dry-run` or command scopes; future GUI toggle planned.

## TLS / CA Handling
- Workspace `.cargo/config.toml` points to the repo-local Mozilla CA bundle at `certs/cacert.pem` for reproducible builds.
- Run `scripts/update_cacert.sh` to refresh the bundle (wrapper around the curl.se Mozilla export). The script creates the directory if needed.
- When `certs/cacert.pem` is missing, Cargo/reqwest fall back to the system trust store so developers can still build; the docs call out the discrepancy in CI.
- Runtime HTTP clients (reqwest) rely on system trust store by default today, but will adopt the shared bundle once the TLS helper described in `docs/repo_review.md` is implemented.

## Rollback Strategy
- Junos driver leverages commit-confirm semantics.
- Generic/Cisco drivers capture pre/post snapshots + diffs; rollback hook stubbed for future configure replace.

## Compliance
- docs/stakeholder_signoff.md records MVP scope + assumptions.
- Upcoming compliance engine (roadmap) will evaluate rules and emit reports.

Last updated: 2025-11-17

