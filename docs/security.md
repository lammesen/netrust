# Security & Compliance Summary

## Credentials
- Stored via `nauto_security::KeyringStore`, which wraps OS keychain providers.
- CLI command `nauto_cli creds` writes credentials to vault; devices store only references.

## Safeguards
- Audit log writer (`logs/audit.log`) captures job summary per execution.
- Dry-run flag and driver capability checks prevent unintended changes on unsupported devices.
- Read-only mode (CLI) achieved via `--dry-run` or command scopes; future GUI toggle planned.

## TLS / CA Handling
- Workspace `.cargo/config.toml` points to bundled Mozilla CA store (`certs/cacert.pem`) for reproducible builds.
- Runtime HTTP clients (reqwest) rely on system trust store by default; can be overridden for private CAs.

## Rollback Strategy
- Junos driver leverages commit-confirm semantics.
- Generic/Cisco drivers capture pre/post snapshots + diffs; rollback hook stubbed for future configure replace.

## Compliance
- docs/stakeholder_signoff.md records MVP scope + assumptions.
- Upcoming compliance engine (roadmap) will evaluate rules and emit reports.

Last updated: 2025-11-17

