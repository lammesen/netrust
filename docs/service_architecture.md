# Scheduler / Worker Architecture (Service Mode)

> [!NOTE]
> This architecture is currently **PLANNED** and not yet fully implemented. The current implementation runs as a standalone CLI tool. See [TODO.md](../TODO.md) for implementation status.


## Components
- **API Gateway**: exposes REST/gRPC endpoints for job submission, approvals, telemetry queries.
- **Scheduler**:
  - Persists job definitions, schedules (cron), and approval state.
  - Resolves targets (inventory, NetBox sync) and enqueues work items.
  - Ensures canary -> batch sequencing via transaction plans.
- **Queue**:
  - Durable message broker (initially JSONL on disk, roadmap to Redis/SQS).
  - Each entry references job definition + inventory snapshot + metadata (ticket IDs, approvals).
- **Workers**:
  - Stateless processes (same binary as CLI) running `nauto_cli worker` or the new prototype `worker_daemon`.
  - Pull jobs from queue, execute via JobEngine, stream telemetry/tracing data.
  - Report success/failure back to scheduler (for retries, notifications).
- **Telemetry / Observability**:
  - Workers emit Prometheus metrics + tracing spans.
  - Scheduler aggregates job states and surfaces dashboards (GUI + CLI).

## Flow
1. User submits job (or schedule fires) → Scheduler validates approvals.
2. Scheduler generates transaction plan (canary/batches) and enqueues each step.
3. Workers fetch next queue item, run job engine with configured drivers.
4. Results, telemetry, and audits pushed back to scheduler + storage.
5. Notifier triggers Slack/Email on success/failure thresholds.

## Deployment Notes
- All services are Rust binaries; can run on Kubernetes or simple VMs with systemd.
- Queue abstraction must be pluggable (local file for dev, Redis for prod).
- Observability stack: Prometheus scraping workers + scheduler, Jaeger/Tempo for tracing.

## Next Steps
- Extract scheduler/worker code into shared crate to avoid CLI duplication.
- Define protobuf/JSON schema for queue items and worker responses.
- Implement idempotent retry strategy (backoff, dead-letter queue).