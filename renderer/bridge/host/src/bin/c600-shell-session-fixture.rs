//! Writes C9's session fixture (charter E3, runbook U6; a draft) for a product build:
//!
//!     c600-shell-session-fixture --assets DIR --menu FILE --out NEW_DIR [--runtime EXE]
//!
//! It makes the fixture with [`c600_shell_host::fixture::write`] and the draft presses
//! [`c600_shell_host::fixture::KEYS`], then reads the record back as the host will. With `--runtime` it also starts the
//! language runtime's process once, as the host's `m6h_load_runtime` does, and ends it, so that a product build shows
//! that its runtime starts. Exit 0 on success, 1 on a failure, 2 for a usage error. Nothing here launches a window or
//! uses a GPU.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use c600_kernel::Kernel;
use c600_shell_host::fixture;

const USAGE: &str = "usage: c600-shell-session-fixture --assets DIR --menu FILE --out NEW_DIR [--runtime EXE]";

struct Args {
    assets: PathBuf,
    menu: PathBuf,
    out: PathBuf,
    runtime: Option<PathBuf>,
}

fn parse() -> Option<Args> {
    let mut assets = None;
    let mut menu = None;
    let mut out = None;
    let mut runtime = None;
    let mut args = std::env::args_os().skip(1);
    while let Some(flag) = args.next() {
        let slot = match flag.to_str()? {
            "--assets" => &mut assets,
            "--menu" => &mut menu,
            "--out" => &mut out,
            "--runtime" => &mut runtime,
            _ => return None,
        };
        if slot.is_some() {
            return None;
        }
        *slot = Some(PathBuf::from(args.next()?));
    }
    Some(Args { assets: assets?, menu: menu?, out: out?, runtime })
}

fn run(args: &Args) -> Result<(), String> {
    let kernel = Arc::new(Kernel::load(&args.assets).map_err(|error| format!("the model: {}", error.0))?);
    fixture::write(kernel, &args.assets, &args.menu, &args.out, &fixture::KEYS)?;
    let read = fixture::read(&args.out.join(fixture::RECORD))?;
    println!("fixture: {} presses, revision {}, state {}", read.committed.len(), read.revision, read.digest);
    if let Some(program) = &args.runtime {
        let mut runtime = c600_shell_host::start_runtime(program)?;
        println!("runtime: started (process {})", runtime.child_id().map_or("unknown".to_owned(), |id| id.to_string()));
        runtime.stop();
    }
    Ok(())
}

fn main() -> ExitCode {
    let Some(args) = parse() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("c600-shell-session-fixture: {error}");
            ExitCode::FAILURE
        }
    }
}
