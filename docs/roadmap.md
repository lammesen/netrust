# Roadmap Beyond MVP

## Milestone 1 (v1.0)
1. **Vendor Coverage**
   - Build drivers for Arista EOS (CLI), Cisco NX-OS API, Meraki REST APIs.
2. **GUI GA**
   - Promote Tauri app to production: job wizard, scheduling UI, compliance dashboards, live log streaming.
3. **Compliance Engine**
   - Define rule syntax, executor, CLI/GUI integrations, export formats (JSON/CSV).
4. **Scheduling & GitOps**
   - Recurring jobs, cron-like syntax, Git repo integration for desired state + config backups.
5. **Plugin SDK Beta**
   - Release WASM SDK crate, docs, sample driver plugin; add CLI commands for plugin management.

## Milestone 2
1. Telemetry dashboard (SNMP/gNMI collectors feeding GUI widgets).
2. Performance/scale tuning for 10k+ devices (connection pooling, worker sharding, HA runner).
3. Notification + approval workflows (Slack/email, canary/device batching).
4. Plugin marketplace with signing, version negotiation, sandbox policies.

## Milestone 3
1. Integrations: NetBox inventory sync, ServiceNow change hooks, GitOps pipelines.
2. Advanced change management (multi-device transactions, staged rollouts, pre/post validations).
3. Distributed execution fabric (coordinated workers, persistent queues).
4. Observability suite (Prometheus metrics, tracing exporters, audit/reporting APIs).

Document owner: product@netrust.local

