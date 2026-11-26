//! Writes C9's session fixture (charter E3, runbook U6; a draft) for a product build:
//!
//!     c600-shell-session-fixture --assets DIR --menu FILE --out NEW_DIR [--puzzle jumble-off|jumble-on] [--runtime EXE]
//!     c600-shell-session-fixture --assets DIR --menu FILE --out NEW_DIR --word FILE
//!
//! It makes the fixture of the puzzle `--puzzle` names (default jumble-off; [`Puzzle::NAMES`]) with
//! [`c600_shell_host::fixture::write_for`] and the draft presses [`c600_shell_host::fixture::KEYS`], then reads the
//! record back as the host will and checks that it names that puzzle. With `--runtime` it also starts the
//! language runtime's process once, as the host's `m6h_load_runtime` does, and ends it, so that a product build shows
//! that its runtime starts.
//!
//! With `--word` (ADR-D02 B5) it makes a jumble-on word fixture instead ([`c600_shell_host::fixture::write_word`]):
//! the file is JSON whose `letters` is a list of `[cap, item]` of the pinned menu. It then reopens a copy of the
//! fixture's session once, as the shell does at a launch, checks the head, prints `reopen_ms: N` and removes the copy.
//! `--word` takes no `--puzzle` and no `--runtime`. The timed reopen runs beside the control at the close
//! ([`c600_shell_host::fixture::write_word_with`], ADR-D02 B26), since each replays the whole journal: the copy is
//! taken once every letter is committed, with its `lock` file made empty in place of the live session's, which the
//! layer may still hold. `reopen_ms` is then timed while the control runs on another thread.
//!
//! Exit 0 on success, 1 on a failure, 2 for a usage error. Nothing here launches a window or uses a GPU.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;

use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;
use c600_session::disk::LOCK;
use c600_shell_host::fixture;
use c600_shell_host::turns::{Puzzle, Turns};

const USAGE: &str = "usage: c600-shell-session-fixture --assets DIR --menu FILE --out NEW_DIR \
     [--puzzle jumble-off|jumble-on] [--runtime EXE] | [--word FILE]";

struct Args {
    assets: PathBuf,
    menu: PathBuf,
    out: PathBuf,
    puzzle: Puzzle,
    runtime: Option<PathBuf>,
    word: Option<PathBuf>,
}

fn parse() -> Option<Args> {
    let mut assets = None;
    let mut menu = None;
    let mut out = None;
    let mut puzzle = None;
    let mut runtime = None;
    let mut word = None;
    let mut args = std::env::args_os().skip(1);
    while let Some(flag) = args.next() {
        let slot = match flag.to_str()? {
            "--assets" => &mut assets,
            "--menu" => &mut menu,
            "--out" => &mut out,
            "--puzzle" => &mut puzzle,
            "--runtime" => &mut runtime,
            "--word" => &mut word,
            _ => return None,
        };
        if slot.is_some() {
            return None;
        }
        *slot = Some(PathBuf::from(args.next()?));
    }
    if word.is_some() && (puzzle.is_some() || runtime.is_some()) {
        return None;
    }
    let puzzle = match (&word, puzzle) {
        (Some(_), _) => Puzzle::JumbleOn,
        (None, None) => Puzzle::JumbleOff,
        (None, Some(name)) => Puzzle::from_name(name.to_str()?)?,
    };
    Some(Args { assets: assets?, menu: menu?, out: out?, puzzle, runtime, word })
}

/// The letters of a word file: `{"letters": [[cap, item], ...]}`.
fn letters(path: &Path) -> Result<Vec<[u64; 2]>, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("the word file: {error}"))?;
    let file = json::parse(&bytes).map_err(|error| format!("the word file is not JSON: {error:?}"))?;
    let items = file.get("letters").and_then(Value::as_array).ok_or("the word file has no `letters` list")?;
    items
        .iter()
        .enumerate()
        .map(|(index, letter)| {
            let pair = letter.as_array().filter(|pair| pair.len() == 2);
            let number = |value: &Value| value.as_i64().and_then(|n| u64::try_from(n).ok());
            match pair.map(|pair| (number(&pair[0]), number(&pair[1]))) {
                Some((Some(cap), Some(item))) => Ok([cap, item]),
                _ => Err(format!("letter {index} is not [cap, item]")),
            }
        })
        .collect()
}

/// Copies a session folder. Its `lock` file is made empty in the copy, as the store creates it, and never read: the
/// live session's layer may still hold it.
fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else if entry.file_name() == LOCK {
            std::fs::File::create(target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// Reopens a copy of the fixture's session as the shell does at a launch, checks its head, and answers the time the
/// reopen took in milliseconds. The copy is removed.
fn timed_reopen(kernel: Arc<Kernel>, args: &Args, read: &fixture::Fixture) -> Result<u128, String> {
    let copy = args.out.with_extension("reopen-check");
    let _ = std::fs::remove_dir_all(&copy);
    copy_dir(&args.out.join(fixture::SESSION), &copy).map_err(|error| format!("the session's copy: {error}"))?;
    let started = Instant::now();
    let reopened = Turns::reopen(kernel, &args.assets, &copy, &args.menu, read).map(|mut turns| turns.head());
    let elapsed = started.elapsed().as_millis();
    let _ = std::fs::remove_dir_all(&copy);
    let digest = reopened??.digest();
    if digest != read.digest {
        return Err(format!("the copy reopened at {digest}, not the fixture's {}", read.digest));
    }
    Ok(elapsed)
}

fn run(args: &Args) -> Result<(), String> {
    let kernel = Arc::new(Kernel::load(&args.assets).map_err(|error| format!("the model: {}", error.0))?);
    let reopened = match &args.word {
        Some(word) => {
            let side = kernel.clone();
            let timed = |shown: &fixture::Fixture| {
                timed_reopen(side, args, shown).map(|elapsed| (elapsed, shown.digest.clone()))
            };
            let (_, reopened) =
                fixture::write_word_with(kernel.clone(), &args.assets, &args.menu, &args.out, &letters(word)?, timed)?;
            Some(reopened?)
        }
        None => {
            fixture::write_for(args.puzzle, kernel.clone(), &args.assets, &args.menu, &args.out, &fixture::KEYS)?;
            None
        }
    };
    let read = fixture::read(&args.out.join(fixture::RECORD))?;
    if read.puzzle != args.puzzle {
        return Err(format!("the record names {}, not {}", read.puzzle.name(), args.puzzle.name()));
    }
    if let Some((_, digest)) = reopened.as_ref().filter(|(_, digest)| *digest != read.digest) {
        return Err(format!("the copy reopened at {digest}, not the record's {}", read.digest));
    }
    println!(
        "fixture: {}, {} presses, revision {}, state {}",
        read.puzzle.name(),
        read.committed.len(),
        read.revision,
        read.digest
    );
    if let Some((elapsed, _)) = reopened {
        println!("reopen_ms: {elapsed}");
    }
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
