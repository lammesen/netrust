# WASM Plugin Prototype

Build the plugin for WASI:

```bash
cargo build -p wasm_plugin --target wasm32-wasi
```

Then run the host with the compiled module:

```bash
cargo run -p wasm_host -- target/wasm32-wasi/debug/wasm_plugin.wasm
```

The plugin uses `nauto_plugin_sdk::export_plugin!` to declare vendor metadata + capability mask. The host loads the WASM module with Wasmtime, resolves the exported helpers, and prints the decoded capabilities.

