# WASM Plugin Prototype

Build the plugin for WASI:

```bash
cargo build -p wasm_plugin --target wasm32-wasi
```

Then run the host with the compiled module:

```bash
cargo run -p wasm_host -- target/wasm32-wasi/debug/wasm_plugin.wasm
```

The host prints the vendor name and capability mask exported by this module, proving the sandboxed, cross-platform extension flow proposed in the plan.

