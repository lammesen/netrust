use anyhow::{Context, Result};
use nauto_plugin_sdk::CapabilityMask;
use std::path::PathBuf;
use wasmtime::{Engine, Linker, Module, Store, TypedFunc};

fn main() -> Result<()> {
    let module_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .context("usage: wasm_host <plugin.wasm>")?;

    let engine = Engine::default();
    let module = Module::from_file(&engine, &module_path)
        .with_context(|| format!("failed to load {}", module_path.display()))?;
    let linker = Linker::new(&engine);
    let mut store = Store::new(&engine, ());
    let instance = linker.instantiate(&mut store, &module)?;

    let vendor_ptr: TypedFunc<(), i32> = instance
        .get_typed_func(&mut store, "plugin_vendor_ptr")
        .context("missing plugin_vendor_ptr export")?;
    let vendor_len: TypedFunc<(), i32> = instance
        .get_typed_func(&mut store, "plugin_vendor_len")
        .context("missing plugin_vendor_len export")?;
    let mask: TypedFunc<(), u32> = instance
        .get_typed_func(&mut store, "plugin_capabilities")
        .context("missing plugin_capabilities export")?;

    let ptr = vendor_ptr.call(&mut store, ())? as u32 as usize;
    let len = vendor_len.call(&mut store, ())? as usize;
    let memory = instance
        .get_memory(&mut store, "memory")
        .context("plugin missing linear memory export")?;
    let vendor_bytes = memory
        .data(&store)
        .get(ptr..ptr + len)
        .context("string slice out of bounds")?
        .to_vec();
    let vendor = std::str::from_utf8(&vendor_bytes)?.to_owned();

    let mask_bits = mask.call(&mut store, ())?;
    let capabilities = CapabilityMask::from_bits_retain(mask_bits);
    println!("Loaded plugin: {vendor}");
    println!("Capabilities: {:?}", capabilities);

    Ok(())
}

