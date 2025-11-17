# Plugin Architecture (Prototype)

## Goals
- Allow third parties to add drivers/job executors without rebuilding core binary.
- Provide sandboxing and deterministic interfaces (WASM-first approach).

## Beta SDK
- Crate: `crates/nauto_plugin_sdk`
  - Provides `CapabilityMask` bitflags and `PluginMetadata`.
  - Macro `export_plugin!` generates the required WASM exports.
- Host: `spikes/wasm_host`
  - Loads `.wasm` via Wasmtime, reads metadata using SDK-defined export names.
- Guest example: `spikes/wasm_plugin`
  - Uses `export_plugin!` to declare vendor + capabilities.

## Next Steps
1. Define HostContext functions (register driver, log events) and expose via WIT bindings.
2. Expand SDK with HostContext callbacks (logging, driver registration).
3. Add signature validation + manifest metadata (CLI verify command prints recorded signature for now).
4. Build CLI commands for installing/listing/removing plugins. ✅

## Interim Strategy
- If WASM timeline slips, consider native dynamic loading with ABI-stable wrapper, but default to WASM for security.

Document owner: extensions@netrust.local

