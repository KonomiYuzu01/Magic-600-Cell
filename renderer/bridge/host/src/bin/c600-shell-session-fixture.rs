//! Writes C9's session fixture (charter E3, runbook U6; a draft) for a product build:
//!
//!     c600-shell-session-fixture --assets DIR --menu FILE --out NEW_DIR [--puzzle jumble-off|jumble-on] [--runtime EXE]
//!
//! It makes the fixture of the puzzle `--puzzle` names (default jumble-off; [`Puzzle::NAMES`]) with
//! [`c600_shell_host::fixture::write_for`] and the draft presses [`c600_shell_host::fixture::KEYS`], then reads the
//! record back as the host will and checks that it names that puzzle. With `--runtime` it also starts the
//! language runtime's process once, as the host's `m6h_load_runtime` does, and ends it, so that a product build shows
//! that its runtime starts. Exit 0 on success, 1 on a failure, 2 for a usage error. Nothing here launches a window or
//! uses a GPU.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use c600_kernel::Kernel;
use c600_shell_host::fixture;
use c600_shell_host::turns::Puzzle;

const USAGE: &str =
    "usage: c600-shell-session-fixture --assets DIR --menu FILE --out NEW_DIR [--puzzle jumble-off|jumble-on] [--runtime EXE]";

struct Args {
    assets: PathBuf,
    menu: PathBuf,
    out: PathBuf,
    puzzle: Puzzle,
    runtime: Option<PathBuf>,
}

fn parse() -> Option<Args> {
    let mut assets = None;
    let mut menu = None;
    let mut out = None;
    let mut puzzle = None;
    let mut runtime = None;
    let mut args = std::env::args_os().skip(1);
    while let Some(flag) = args.next() {
        let slot = match flag.to_str()? {
            "--assets" => &mut assets,
            "--menu" => &mut menu,
            "--out" => &mut out,
            "--puzzle" => &mut puzzle,
            "--runtime" => &mut runtime,
            _ => return None,
        };
        if slot.is_some() {
            return None;
        }
        *slot = Some(PathBuf::from(args.next()?));
    }
    let puzzle = match puzzle {
        None => Puzzle::JumbleOff,
        Some(name) => Puzzle::from_name(name.to_str()?)?,
    };
    Some(Args { assets: assets?, menu: menu?, out: out?, puzzle, runtime })
}

fn run(args: &Args) -> Result<(), String> {
    let kernel = Arc::new(Kernel::load(&args.assets).map_err(|error| format!("the model: {}", error.0))?);
    fixture::write_for(args.puzzle, kernel, &args.assets, &args.menu, &args.out, &fixture::KEYS)?;
    let read = fixture::read(&args.out.join(fixture::RECORD))?;
    if read.puzzle != args.puzzle {
        return Err(format!("the record names {}, not {}", read.puzzle.name(), args.puzzle.name()));
    }
    println!(
        "fixture: {}, {} presses, revision {}, state {}",
        read.puzzle.name(),
        read.committed.len(),
        read.revision,
        read.digest
    );
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
