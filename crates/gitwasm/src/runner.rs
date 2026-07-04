use crate::manifest::Limits;
use anyhow::{Context, Result};
use std::path::Path;
use wasmtime::{Config, Engine, Linker, Module, Store, StoreLimits, StoreLimitsBuilder};
use wasmtime_wasi::p1::{add_to_linker_sync, WasiP1Ctx};
use wasmtime_wasi::{DirPerms, FilePerms, I32Exit, WasiCtxBuilder};

/// The sandbox a module runs in. This is the whole security story:
/// the module sees exactly one directory (mounted at "."), its argv,
/// and inherited stdout/stderr. No network, no env, no other files —
/// and fuel/memory limits bound how much CPU and RAM it may consume.
pub struct Sandbox<'a> {
    pub dir: &'a Path,
    pub writable: bool,
    pub argv: Vec<String>,
    pub limits: Limits,
}

struct Ctx {
    wasi: WasiP1Ctx,
    limits: StoreLimits,
}

/// Run a WASI command module to completion; returns its exit code.
pub fn run_module(wasm_path: &Path, sandbox: Sandbox) -> Result<i32> {
    let module_bytes = std::fs::read(wasm_path)
        .with_context(|| format!("reading wasm module {}", wasm_path.display()))?;
    run_module_bytes(&module_bytes, sandbox)
        .with_context(|| format!("running {}", wasm_path.display()))
}

pub fn run_module_bytes(wasm: &[u8], sandbox: Sandbox) -> Result<i32> {
    let mut config = Config::new();
    config.consume_fuel(true);
    let engine = Engine::new(&config)?;
    let module = Module::new(&engine, wasm).context("compiling wasm module")?;

    let mut linker: Linker<Ctx> = Linker::new(&engine);
    add_to_linker_sync(&mut linker, |ctx| &mut ctx.wasi)?;

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

    let ctx = Ctx {
        wasi: builder.build_p1(),
        limits: StoreLimitsBuilder::new()
            .memory_size(sandbox.limits.memory_bytes as usize)
            .build(),
    };
    let mut store = Store::new(&engine, ctx);
    store.limiter(|ctx| &mut ctx.limits);
    store.set_fuel(sandbox.limits.fuel)?;

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
            None => Err(err.context("module trapped (limit exceeded or crash)")),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A hostile module that loops forever must be stopped by the fuel limit,
    /// not hang the host.
    #[test]
    fn fuel_limit_stops_infinite_loop() {
        let wasm = wat::parse_str(r#"(module (func (export "_start") (loop br 0)))"#).unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let result = run_module_bytes(
            &wasm,
            Sandbox {
                dir: tmp.path(),
                writable: false,
                argv: vec!["loop".into()],
                limits: Limits {
                    fuel: 1_000_000,
                    memory_bytes: 64 * 1024 * 1024,
                },
            },
        );
        let err = result.expect_err("infinite loop must trap on fuel exhaustion");
        assert!(
            format!("{err:#}").contains("fuel"),
            "unexpected error: {err:#}"
        );
    }
}
