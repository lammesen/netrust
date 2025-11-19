# Production Readiness Checklist

## Platform & Architecture
- Replace JSONL queue with pluggable durable backend (Redis/SQS/Postgres) and add worker retry/backoff + DLQ.
- Add HTTP/gRPC control plane for job submission, approvals, telemetry, and progress streaming; secure with authn/z and rate limits.
- Implement plugin host callbacks (logging, driver registration) and enforce signature validation for WASM drivers.

## Reliability & Safety
- Wire transaction plans into rollback policies (snapshot/replace) and add configurable failure thresholds and retries per device/batch.
- Integrate approvals enforcement into service path (scheduler/worker) and block execution when missing.
- Add config/credential validation pre-flight (lint job/inventory) and connection tests before rollout.

## Security
- Harden credential storage: at-rest encryption for fallback file, secret rotation, and assume-role/token flows for cloud APIs.
- Ship SBOM + dependency audit gates; enable signing/verification of release artifacts and plugins.
- Enforce TLS verification and timeouts across HTTP/SSH/NETCONF; add policy for CA bundle updates.

## Observability
- Replace CLI-only metrics snapshot with an always-on exporter endpoint; ship OpenTelemetry tracing/metrics to centralized backends.
- Persist per-device logs/diffs centrally and surface dashboards for success/failure rates, latency, and capacity alerts.

## Drivers & Compliance
- Expand driver coverage tests with mocks/simulators; add rollback coverage and failure-path tests.
- Integrate compliance jobs with live config collection (per driver) and support richer rule language (regex/numeric comparisons).

## UX & Packaging
- Package signed binaries/installers (CLI + GUI) for macOS/Windows/Linux; provide upgrade path and health checks.
- Add GUI parity for approvals, transaction plans, and observability; stream live job progress.
- Document onboarding/runbooks (backup/restore queue, rotate keys, incident response) and publish REST/gRPC API docs.

## CI/CD & QA
- Add CI matrix for fmt/clippy/test + driver/unit mocks + Tauri build; include nightly scale tests (bench/telemetry) with thresholds.
- Create staging environment with synthetic devices; run canary pipelines before production releases.

## Critical Gaps (from Codebase Review)
- **Drivers are Simulations**: No actual network communication occurs.
- **No Per-Device Timeouts**: Hung devices can block execution indefinitely.
- **No Task Cancellation**: Cannot stop running jobs gracefully.
- **Unbounded Memory**: Results accumulate in memory, risking OOM at scale.
- **Missing Tests**: Extremely low test coverage (no integration/failure path tests).

See [TODO.md](../TODO.md) for the plan to address these gaps.