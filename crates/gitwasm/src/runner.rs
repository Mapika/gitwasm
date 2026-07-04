use anyhow::{Context, Result};
use std::path::Path;
use wasmtime::{Engine, Linker, Module, Store};
use wasmtime_wasi::p1::{add_to_linker_sync, WasiP1Ctx};
use wasmtime_wasi::{DirPerms, FilePerms, I32Exit, WasiCtxBuilder};

/// The sandbox a module runs in. This is the whole security story:
/// the module sees exactly one directory (mounted at "."), its argv,
/// and inherited stdout/stderr. No network, no env, no other files.
pub struct Sandbox<'a> {
    pub dir: &'a Path,
    pub writable: bool,
    pub argv: Vec<String>,
}

/// Run a WASI command module to completion; returns its exit code.
pub fn run_module(wasm_path: &Path, sandbox: Sandbox) -> Result<i32> {
    let engine = Engine::default();
    let module = Module::from_file(&engine, wasm_path)
        .with_context(|| format!("loading wasm module {}", wasm_path.display()))?;

    let mut linker: Linker<WasiP1Ctx> = Linker::new(&engine);
    add_to_linker_sync(&mut linker, |ctx| ctx)?;

    let (dir_perms, file_perms) = if sandbox.writable {
        (DirPerms::all(), FilePerms::all())
    } else {
        (DirPerms::READ, FilePerms::READ)
    };

    let mut builder = WasiCtxBuilder::new();
    builder
        .inherit_stdout()
        .inherit_stderr()
        .args(&sandbox.argv)
        .preopened_dir(sandbox.dir, ".", dir_perms, file_perms)
        .with_context(|| format!("preopening {}", sandbox.dir.display()))?;
    let wasi = builder.build_p1();

    let mut store = Store::new(&engine, wasi);
    let instance = linker
        .instantiate(&mut store, &module)
        .context("instantiating module")?;
    let start = instance
        .get_typed_func::<(), ()>(&mut store, "_start")
        .context("module has no _start (not a WASI command module?)")?;

    match start.call(&mut store, ()) {
        Ok(()) => Ok(0),
        Err(err) => match err.downcast_ref::<I32Exit>() {
            Some(exit) => Ok(exit.0),
            None => Err(err.context("module trapped")),
        },
    }
}
