# Repo Review & Planning (2025-11-17)

## 1. Architecture & Testing Snapshot

### Current Architecture
- **Workspace layout** – `README.md` plus `docs/architecture.md` describe a layered Rust workspace: shared crates (`nauto_model`, `nauto_drivers`, `nauto_engine`, `nauto_security`, `nauto_telemetry`) feed into the CLI/TUI app under `apps/nauto_cli`. Spikes for GUI (`spikes/tauri_poc`), NETCONF/SSH, and WASM plugins validate future directions (`docs/spikes_report.md`).
- **Execution flow** – Jobs originate from CLI/TUI/GUI, run through `JobEngine` (concurrency bounded via semaphore) and dispatch to vendor drivers via `DriverRegistry` (`docs/job_engine.md`, `docs/drivers.md`). Credentials resolve through `KeyringStore` (`crates/nauto_security`), inventories live in YAML (`examples/*`), and audit logging is handled in `apps/nauto_cli/src/audit.rs`.
- **Service mode roadmap** – `docs/service_architecture.md` outlines API Gateway → Scheduler → Queue → Worker topology. Current worker implementations (`apps/nauto_cli/src/worker.rs`, `apps/nauto_cli/src/bin/worker_daemon.rs`) still operate on JSONL queues, but share the same binaries as the CLI for eventual deployment parity.
- **UX surfaces** – `docs/ux.md` and `docs/quick_start.md` document the CLI command surface, ratatui-based TUI (`apps/nauto_cli/src/tui.rs`), and the Tauri control-center spike (commands in `spikes/tauri_poc/src-tauri/src/main.rs`). Notifications/approvals/gitops/compliance modules are already wired into the CLI subcommand tree (`apps/nauto_cli/src/*.rs`).

### Testing Status
- **Automated checks** – `docs/testing.md` (dated 2025-11-17) confirms `cargo check`, `cargo test`, and CI workflows (fmt/clippy/test/audit + Tauri smoke build) are green. Unit tests primarily cover `nauto_engine::JobEngine` concurrency and `nauto_model` serialization (`crates/nauto_engine/src/lib.rs`, `crates/nauto_model/tests/serde.rs`).
- **Manual smoke tests** – CLI paths for `bench`, `telemetry`, `transactions`, `worker`, and `integrations netbox-import` were exercised per the testing report, producing sample artifacts (transaction plans, inventories).
- **Gaps** – No integration tests wire the CLI into real driver transports; queue/worker code lacks automated coverage; `spikes/tauri_poc` is entirely manual; compliance/approvals/gitops modules rely on filesystem state without regression tests. Security-sensitive flows (credential storage, webhook notifications) currently have no automated validation.

## 2. Issues & Recommended Fixes

| # | Area | Reference | Description | Recommended Fix |
|---|------|-----------|-------------|-----------------|
| 1 | Toolchain portability | `.cargo/config.toml` + `docs/security.md` | Config hardcodes a macOS-specific absolute CA path (`/Users/mlt/.../certs/cacert.pem`) even though the docs promise a repo-local bundle. Linux hosts/CI can’t resolve the file. No `certs/cacert.pem` exists in the repo. | Check `certs/cacert.pem` into the repo (or fetch during build) and point `cainfo` to a workspace-relative path using `${CARGO_MANIFEST_DIR}`/env vars. Document fallback to system store if file missing. |
| 2 | Credential command security | `apps/nauto_cli/src/main.rs` (`Commands::Creds`) | Users must pass `--password` on the command line, which leaks secrets via shell history and process listings—contradicting the `docs/security.md` assurance about secure handling. | Add `--password-stdin`/`--prompt` support (read via `rpassword`), defaulting to interactive input. Keep `--password` only for automation with explicit warning, and update docs/quick start. |
| 3 | Observability UX | `apps/nauto_cli/src/observability.rs` | `ObservabilityCmd` exposes a `--format` flag but the implementation ignores it and always emits Prometheus text. Users requesting JSON receive the same output, causing confusion. | Either remove the argument or honor it: when `format == "json"`, emit structured JSON (e.g., encode gathered metrics to JSON) and keep text as default. Add tests covering both modes. |
| 4 | Transaction planner robustness | `apps/nauto_cli/src/transactions.rs` | Supplying `--batch-size 0` (or via YAML overrides) creates an infinite loop because `rest.drain(..0)` never removes devices. No validation prevents zero/negative batch sizes. | Validate `canary_size`/`batch_size` > 0 on input (via Clap `value_parser` or manual check) and return a helpful error instead of hanging. Add regression test. |
| 5 | Meraki driver correctness | `crates/nauto_drivers/src/drivers/meraki_cloud.rs` | `submit_meraki_request` only logs and drops the `reqwest::Client` handle; it never sends an HTTP request, never adds required API key headers, and never surfaces errors—so jobs falsely appear successful. | Implement actual `client.post(...).header("X-Cisco-Meraki-API-Key", ...)` calls, source API keys from credentials, propagate errors into `DriverExecutionResult`, and add retries/timeout config. |

## 3. Net-New Feature Suggestions
- **Inventory source-of-truth sync service** (impacts `nauto_engine`, `nauto_cli::integrations`, future scheduler): Build a daemon that continuously polls NetBox/ServiceNow APIs, normalizes device metadata, and publishes delta snapshots to the queue/GitOps layers. Adds drift detection alerts and keeps inventories consistent without manual exports.
- **Policy-driven job guardrails** (impacts `nauto_compliance`, `apps/nauto_cli::approvals`, scheduler): Introduce a rule engine that evaluates pending jobs against compliance/policy templates (e.g., “touching core devices requires on-call approval”) before execution. Integrate with approvals + notifications for auto-escalation.
- **Telemetry baseline analytics** (impacts `nauto_telemetry`, GUI/TUI): Persist telemetry snapshots, compute rolling baselines, and surface anomalies in the Tauri GUI/TUI dashboards. Enables proactive issue detection and ties into the observability CLI command.

## 4. Production Frontend (Tauri) Plan
- **App structure** – Split the Tauri project into `src-tauri` (Rust commands) and a modern UI (e.g., Svelte/React) housed under `spikes/tauri_poc/src`. Define routes/widgets for Inventory, Job Wizard, Schedules, Compliance, Telemetry, and Approvals. Adopt a state management library (TanStack Query redux) for caching backend responses.
- **Data layer** – Replace the in-memory `AppState` with IPC calls that talk to real backend APIs: start with invoking `nauto_cli` subcommands (via `Command` plugin) and evolve toward REST/gRPC endpoints once the scheduler/API gateway ship. Introduce a typed client module that centralizes request handling, auth headers, and streaming subscriptions.
- **Streaming/live updates** – Use Tauri’s event system (or WebSockets once scheduler exports them) to stream job progress, telemetry metrics, and compliance status into the UI. On the Rust side, expose channels fed by `tracing` events or queue notifications; on the frontend, render log streams with virtualization.
- **Offline/sandbox mode** – Keep a deterministic “demo data” provider (what the spike currently does) selectable via settings so QA/marketing can showcase the UI without backend connectivity.
- **Testing & QA** – Add Vitest/Playwright suites for the frontend, plus Rust integration tests that exercise the Tauri commands. Wire these into CI alongside the existing smoke build.
- **Packaging** – Configure `tauri.conf.json` for per-platform identifiers, icon pipeline, auto-updater strategy, and codesigning placeholders. Produce CI artifacts via GitHub Actions matrix (macOS, Windows, Linux), publishing installers to prerelease channels.

## 5. Certificate/TLS Implementation Plan
- **Provisioning** – Check in (or fetch during build) a Mozilla CA bundle under `certs/cacert.pem` to align with `docs/security.md`. Update `.cargo/config.toml` and CI scripts to point to this relative path so `cargo install`/`reqwest` builds succeed deterministically.
- **Runtime trust stores** – Build a `tls` helper inside `nauto_security` that loads (a) system trust roots, (b) optional workspace bundle, and (c) operator-provided private CAs. Wrap `reqwest::ClientBuilder` usage in `nauto_drivers` and CLI integrations (`notifications.rs`, NetBox/ServiceNow clients) so every HTTP client consistently applies the same root set, ALPN, and timeout policies.
- **Credential binding** – Extend credential records (`nauto_model::Credential`) to store API tokens/certs for HTTP drivers, ensuring secrets flow through `KeyringStore` rather than environment variables. Provide CLI commands to import certificates/keys with rotation metadata.
- **Rotation** – Document a rotation workflow in `docs/security.md`: generate new CA/intermediate, upload to `certs/`, trigger rolling deploys where workers reload trust stores without restart (e.g., watch file + rebuild clients). Integrate with approvals to require sign-off before distributing new trust bundles.
- **Verification & monitoring** – Add health-check commands/tests that call each external endpoint (`reqwest`) with the configured CA bundle, failing fast if trust mismatches. Emit metrics/logs when TLS errors occur so operators can detect expired certs quickly.

---

Prepared by: repo review automation (GPT-5.1 Codex) on 2025-11-17.
