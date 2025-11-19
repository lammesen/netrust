# Job Engine Overview

## Pipeline
1. **Target Resolution** – `DeviceInventory::resolve_targets` maps selectors to concrete devices (IDs, tags).
2. **Execution** – `JobEngine::execute` spawns per-device tasks bounded by a semaphore (`max_parallel`).
3. **Pre/Dry Run** – Dry-run flag short-circuits devices lacking native dry-run support (log entry recorded).
4. **Result Aggregation** – Device results captured in `TaskSummary` (logs, diff, status, timestamps).
5. **Error Handling** – Failures logged per device; rest of fleet continues unless job-level policy stops it.

## Key Types
- `Job`: user-submitted definition (kind, targets, parameters, concurrency).
- `TaskSummary`: per-device outcome, used by CLI summaries/audit log.
- `DriverExecutionResult`: data returned by drivers (logs, snapshots, diff).

## Testing
- Unit test `runs_job_across_devices` (in `nauto_engine/src/lib.rs`) covers multi-device success path.
- Future work: add integration tests for rollback scenarios once driver mocks expose snapshots.

## Next Steps
- Emit progress events over channel for TUI/GUI streaming.
- Add configurable retry policies and failure thresholds.