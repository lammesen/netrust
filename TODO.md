# Netrust TODO

This document lists the outstanding high-priority (P0) tasks for the `netrust` project, based on the original `CODEBASE_REVIEW.md`. The most critical issue—the simulated SSH drivers—has been addressed, but several other important tasks remain.

## P0: Critical Correctness / Security / Scalability Issues

### Concurrency & Scalability
- [ ] Add per-device timeout to `JobEngine::execute` (wrap `run_device` in `tokio::time::timeout`).
- [ ] Implement graceful task cancellation (store `JoinHandle`s, expose `cancel()` method that calls `abort()`).
- [ ] Stream results to `JobStore` instead of accumulating in memory (refactor `device_results` vector).
- [ ] Make `execute_compliance_job` async and parallel (use `FuturesUnordered` like other job types).
- [ ] Replace `expect` on semaphore with `ok()?` to prevent panic on cancellation.

### Security
- [ ] Encrypt fallback credential file using `age` or `sodiumoxide` (derive key from keyring).
- [ ] Implement WASM plugin signature verification using `ed25519-dalek`.
- [ ] Restrict WASM plugin capabilities using WASI (deny filesystem/network access by default).
- [ ] Add audit logging for credential access events (who/when/which credential).

### Architecture
- [ ] Define and implement `JobStore` trait for persisting job state and results (sqlite MVP, Postgres production).
- [ ] Add `JobQueue` trait with Redis/SQS/Postgres implementations (remove JSONL file dependency).

### Testing
- [ ] Add integration tests: `test_run_job_e2e`, `test_cli_commands`.
- [ ] Add driver behavior tests with mock SSH/NETCONF/HTTP servers.
- [ ] Add failure path tests (timeout, connection refused, rollback on error).
- [ ] Add security tests (keyring round-trip, signature verification).

### CI
- [ ] Enforce clippy warnings as errors (`-- -D warnings` in CI).
- [ ] Add e2e test job to CI (run CLI against test inventory, validate output).
