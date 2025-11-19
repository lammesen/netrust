# Security & Compliance Summary

## Credentials
- Stored via `nauto_security::KeyringStore`, which wraps OS keychain providers.
- CLI command `nauto_cli creds` writes credentials to vault; devices store only references.
- Interactive usage now prompts for the password by default (`--password-prompt`), while automation can use `--password-stdin`; passing `--password` directly is allowed but prints a warning about argv exposure.
- For headless servers (where platform keyrings are unavailable), set `NAUTO_KEYRING_FILE=/secure/path/credentials.json`. The keyring helper now mirrors secrets into that JSON file and transparently falls back to it when OS APIs fail (file contents are keyed by credential name and should reside on encrypted storage).

## Safeguards
- Audit log writer (`logs/audit.log`) captures job summary per execution.
- Dry-run flag and driver capability checks prevent unintended changes on unsupported devices.
- Read-only mode (CLI) achieved via `--dry-run` or command scopes; future GUI toggle planned.

## TLS / CA Handling
- Workspace `.cargo/config.toml` points to the repo-local Mozilla CA bundle at `certs/cacert.pem` (now committed to git) so cargo/rustls builds are repeatable in hermetic CI.
- Run `scripts/update_cacert.sh` to refresh the bundle (wrapper around the curl.se Mozilla export). The script creates the directory if needed.
- Runtime HTTP clients (Meraki/NX-API/eAPI) continue to rely on the OS trust store, but now honor the same configurable timeout/retry settings (`NAUTO_HTTP_TIMEOUT_SECS`, `NAUTO_HTTP_RETRIES`).

## Rollback Strategy
- Junos driver leverages commit-confirm semantics.
- Generic/Cisco drivers capture pre/post snapshots + diffs; rollback hook stubbed for future configure replace.

## Compliance
- docs/stakeholder_signoff.md records MVP scope + assumptions.
- Upcoming compliance engine (roadmap) will evaluate rules and emit reports.

Last updated: 2025-11-17