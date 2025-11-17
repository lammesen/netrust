# Spike Prototypes

Quick experiments covering the riskiest technology choices from `plan.md` Section 3.

## SSH POC
- Location: `spikes/ssh_poc`
- Demonstrates opening hundreds of async SSH sessions via `async-ssh2-tokio`.
- Usage: `cargo run -p ssh_poc -- --host 10.0.0.5 --username admin --password **** --command "show version"`

## NETCONF POC
- Location: `spikes/netconf_poc`
- Shows RFC 6242 negotiation using the same SSH transport, emitting device hello.
- Usage: `NETCONF_HOST=10.0.0.6 NETCONF_USER=netconf NETCONF_PASSWORD=**** cargo run -p netconf_poc`

## GUI (Tauri) POC
- Location: `spikes/tauri_poc`
- Minimal static front-end + Tauri backend command to prove IPC wiring.
- Usage: `cd spikes/tauri_poc && cargo tauri dev`

## WASM Plugin Host/Guest
- Host crate: `spikes/wasm_host`
- Guest crate: `spikes/wasm_plugin`
- Demonstrates calling sandboxed logic that emits device driver metadata.
- Usage: `cargo run -p wasm_host -- target/wasm32-wasi/debug/wasm_plugin.wasm`

