# Risk Register

| ID | Risk | Impact | Likelihood | Mitigation / Owner |
|----|------|--------|------------|--------------------|
| R1 | SSH library regressions (async-ssh2-tokio) | Connection failures at scale | Medium | Monitor crate releases, keep russh fallback plan. Owner: Platform Eng |
| R2 | NETCONF crate gaps | Limited vendor support | Medium | Maintain custom NETCONF shim (quick-xml) + integration tests. Owner: Driver Team |
| R3 | GUI portability (Tauri dependencies) | GUI unusable on some OS versions | Medium | Document prerequisites, add CI runs on macOS/Windows/Linux. Owner: UX Team |
| R4 | Scale/perf (10k devices) | Job failures, resource exhaustion | High | Stress tests with mock devices, tune semaphore/batching, track metrics. Owner: Job Engine Team |
| R5 | Plugin security | Malicious extension compromises host | Medium | Default to WASM sandbox, require signatures, document best practices. Owner: Extensions Team |

## Stress Test Plan
- **SSH Load**: Simulate 5k+ connections using mock SSH server; measure memory/CPU.
- **GUI Builds**: Nightly CI building Tauri app for macOS/Windows/Linux.
- **Plugin Sandbox**: Run fuzzing on host interface + WASM boundaries.

Last updated: 2025-11-17

