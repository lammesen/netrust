# MVP Scope & Target Outcomes

Derived from `plan.md` Section 1.

## Supported Use Cases
- Bulk configuration rollouts with dry-run, diff, and commit stages.
- Batch interface operations (disable/enable/update descriptions).
- Inventory collection and telemetry snapshots via CLI/API commands.
- Compliance checks comparing configs against baselines with reporting.

## Operating Assumptions
- Reliable management connectivity (SSH/API) to every target device.
- Users maintain an inventory (ID, address, vendor, credentials, tags).
- Credentials are supplied securely (OS keychain or SSH keys).
- Tool executes per-device jobs; no complex cross-device orchestration for MVP.

## Explicit Non-Goals
- Real-time NMS/telemetry streaming (gNMI, SNMP polling dashboards).
- Comprehensive vendor/protocol coverage beyond initial drivers.
- Multi-step transactional workflows spanning device dependencies.
- Acting as a controller replacement (e.g., Cisco DNA Center).

## Acceptance Checklist
- [ ] Stakeholders agree the above use cases cover MVP launch.
- [ ] Assumptions are documented in onboarding material.
- [ ] Non-goals communicated to prevent scope creep.

Document owner: Core architecture team.

