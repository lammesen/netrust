# Technology Spike Report

Date: 2025-11-17

## SSH Scaling (async-ssh2-tokio)
- Location: `spikes/ssh_poc`
- Outcome: Verified async-ssh2-tokio `0.11` API; confirmed command execution and stdout/stderr capture via simple CLI.
- Notes: Future work—benchmark concurrent sessions via job engine harness; keep russh-based fallback on radar.

## NETCONF Session
- Location: `spikes/netconf_poc`
- Outcome: Established NETCONF subsystem channel over SSH, exchanged client/server hello.
- Notes: For production, wrap quick-xml parsing for RPC responses; consider higher-level NETCONF crate if maintained.

## GUI Prototype (Tauri)
- Location: `spikes/tauri_poc`
- Outcome: Built minimal static front-end + Rust command to simulate job summary IPC.
- Notes: Need to flesh out state management and integrate with backend API once exposed.

## WASM Plugin Host/Guest
- Locations: `spikes/wasm_host`, `spikes/wasm_plugin`
- Outcome: Host loads WASM module, reads vendor metadata + capability mask.
- Notes: Next steps include defining HostContext functions (driver registration) and sandbox policies.

## Findings
- Tokio + async-ssh2-tokio meet async/concurrency requirements; keep libs updated for security.
- NETCONF via SSH manageable with existing toolchain; invest in RPC utilities later.
- Tauri suits GUI goals (Rust backend, native shell).
- WASM approach feasible for safe runtime plugins; need SDK ergonomics.

Document owner: platform@netrust.local