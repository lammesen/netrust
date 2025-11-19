# Netrust Roadmap

This document serves as the single source of truth for the project roadmap, consolidating backend tasks from the codebase review and the frontend implementation plan.

## Backend & Core

### P0: Critical Correctness / Security / Scalability Issues

**Drivers & Protocol**
- [x] Remove `tokio::sleep` simulations from all drivers (`cisco_ios.rs`, `juniper_junos.rs`, `arista_eos.rs`, `cisco_nxos_api.rs`, `meraki_cloud.rs`, `generic_ssh.rs`).
- [x] Implement real SSH execution in `nauto_drivers/src/ssh.rs::connect` using `async_ssh2_tokio::Client::execute`.
- [x] Implement NETCONF protocol in `juniper_junos.rs::NetconfSession` (hello exchange, RPC framing with `]]>]]>`).
- [x] Implement Arista eAPI HTTP client in `arista_eos.rs::run_command_batch_eapi` using `reqwest`.
- [x] Implement NX-OS NX-API HTTP client in `cisco_nxos_api.rs` using JSON-RPC over `/ins`.
- [x] Add timeout wrapper to every driver operation: `tokio::time::timeout(30.seconds(), client.execute(...))`.

**Concurrency & Scalability**
- [x] Add per-device timeout to `JobEngine::execute` (wrap `run_device` in `tokio::time::timeout`).
- [x] Implement graceful task cancellation (store `JoinHandle`s, expose `cancel()` method that calls `abort()`).
- [x] Stream results to `JobStore` instead of accumulating in memory (refactor `device_results` vector).
- [x] Make `execute_compliance_job` async and parallel (use `FuturesUnordered` like other job types).
- [x] Replace `expect` on semaphore with `ok()?` to prevent panic on cancellation.

**Security**
- [x] Encrypt fallback credential file using `age` or `sodiumoxide` (derive key from keyring).
- [x] Implement WASM plugin signature verification using `ed25519-dalek`.
- [x] Restrict WASM plugin capabilities using WASI (deny filesystem/network access by default).
- [x] Add audit logging for credential access events (who/when/which credential).

**Architecture**
- [x] Define and implement `JobStore` trait for persisting job state and results (sqlite MVP, Postgres production).
- [x] Add `JobQueue` trait with Redis/SQS/Postgres implementations (remove JSONL file dependency).

**Testing**
- [x] Add integration tests: `test_run_job_e2e`, `test_cli_commands`, `test_driver_simulation`.
- [x] Add driver behavior tests with mock SSH/NETCONF/HTTP servers.
- [x] Add failure path tests (timeout, connection refused, rollback on error).
- [x] Add security tests (keyring round-trip, signature verification).

**CI**
- [x] Enforce clippy warnings as errors (`-- -D warnings` in CI).
- [x] Add e2e test job to CI (run CLI against test inventory, validate output).

**Docs**
- [x] Add disclaimer in `README.md` that drivers are currently simulated (not production-ready).

### P1: Important But Not Blocking

**Drivers & Protocol**
- [ ] Add retry with exponential backoff for transient connection failures (use `tokio_retry`).
- [ ] Integrate rollback into `JobEngine` failure handling (call `driver.rollback` on task failure).
- [ ] Add configurable diff line limit (warn when truncation occurs).
- [ ] Support SSH key authentication (use `Credential::SshKey` in drivers).

**Concurrency & Scalability**
- [ ] Add retry logic with exponential backoff for transient failures in `run_device`.
- [ ] Add telemetry for active tasks, queued tasks, task latency distribution.

**Security**
- [ ] Support external credential providers (AWS Secrets Manager, Vault, Azure Key Vault).
- [ ] Add TLS certificate pinning for critical API endpoints (Meraki Dashboard).
- [ ] Send audit logs to remote syslog or SIEM (Splunk, Datadog).

**Architecture**
- [ ] Implement device locking mechanism (Redis distributed lock or DB advisory locks).
- [ ] Integrate transaction plan execution into `JobEngine` (canary batch, staged rollout, auto-rollback).
- [ ] Add `ApprovalStore` trait backed by database (replace file-based approvals).

**UX**
- [ ] Add `nauto_cli job status <job-id>` command to query job state.
- [ ] Stream job progress to stdout as devices complete (don't wait for all).
- [ ] Add `--output json|yaml|table` flag to all CLI commands.
- [ ] Generate shell completion scripts using `clap_complete`.
- [ ] Add "Jobs" tab to TUI showing active and recent jobs.
- [ ] Add "Logs" panel to TUI showing selected device's recent job logs.
- [ ] Add "Metrics" dashboard to TUI.
- [ ] Implement HTTP/gRPC client in Tauri backend (call `nauto_service` endpoints).
- [ ] Add authentication to GUI (login page, OAuth2, JWT session).

**Plugins**
- [ ] Integrate WASM host into driver registry (load plugins at startup, register drivers).
- [ ] Define host API for plugins using `wasmtime::Linker` (logging, credential access, command execution).
- [ ] Create plugin marketplace registry (HTTP index, download, verify signature).
- [ ] Add plugin version compatibility checks (min/max nauto version).

**Testing**
- [ ] Add benchmark regression tests in CI (fail if slower than baseline).

**CI**
- [ ] Add Dependabot config for automated dependency updates.

**Docs**
- [ ] Sync `service_architecture.md` with actual implementation (mark unimplemented features).
- [ ] Add code examples to `docs/quick_start.md` that reference real files.

### P2: Nice-to-Have / Cleanup

**Drivers & Protocol**
- [ ] Add TLS cert validation for HTTPS drivers (allow custom CA bundle).

**Security**
- [ ] Add pre-commit hook and CI check for secret scanning (`trufflehog` or `gitleaks`).
- [ ] Implement RBAC for CLI commands (operator vs admin roles).

**Architecture**
- [ ] Create HTTP/gRPC control plane service (`apps/nauto_service`) exposing job submit/status/cancel endpoints.

**UX**
- [ ] Add `--summary-only` flag for large jobs (show counts, not per-device details).
- [ ] Add help text overlay to TUI (press `?` to show keybindings).
- [ ] Add real-time job progress streaming to GUI (WebSocket or SSE).
- [ ] Add approvals workflow UI to GUI (show pending, approve/reject buttons).

**Plugins**
- [ ] Add plugin hot reload (watch directory, unload old, load new).
- [ ] Create plugin development guide (sample plugin, build instructions, testing harness).

**Testing**
- [ ] Publish rustdoc to GitHub Pages on each release.

**Docs**
- [ ] Generate and publish API reference docs (rustdoc) for public APIs.

## Frontend (Web UI)

### Stack
- React + Vite + TypeScript + Tailwind.
- UI kit: shadcn/ui + Headless UI.
- State: TanStack Query, React Hook Form + Zod.

### Completed
- [x] Vite + React + TS scaffolded with Tailwind, shadcn init.
- [x] Tailwind theme configured (dark-first).
- [x] shadcn components generated.
- [x] Layout shell (header + panels).
- [x] QueryClient + Toaster wired.
- [x] Job Wizard & Scheduling forms (RHF + Zod).
- [x] Mock data layer + TanStack Query hooks.

### Phased Plan

**Inventory Panel**
- [ ] Table using TanStack Table + shadcn styling for sorting/filtering/search.
- [ ] Empty/loading/error states; row actions (view device/run job) placeholders.

**Job Wizard**
- [ ] Single form or accordion sections.
- [ ] Fields: job name, type (select), target filter (combobox/autocomplete), commands/snippet.
- [ ] Dry run toggle; submit button with progress; status log area.

**Scheduling**
- [ ] Cron input with helper presets + validation.
- [ ] List of schedules (table/cards) with enable/disable/delete actions.

**Compliance**
- [ ] Refresh snapshot action; polling status; display last updated time and summary/log.

**Feedback UX**
- [ ] Toasts for success/failure; inline errors; optimistic updates.
- [ ] Skeletons/spinners for load states.

**Accessibility**
- [ ] Focus rings, aria labels, keyboard navigation.

**Testing**
- [ ] Vitest + React Testing Library for components.
- [ ] Optional Playwright smoke.