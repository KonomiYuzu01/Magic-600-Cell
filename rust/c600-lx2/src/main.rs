//! Runs LX-2's metered and forced-kill trials against a runtime child binary (see the library):
//!
//! ```text
//! c600-lx2-faults --runtime <c600-runtime binary> [--sessions <new folder>] [--assets <folder>] [--out <record.json>]
//! ```
//!
//! With `--sessions` it also runs each trial with a session open (RTE.42 conditions 1 and 2), each in its own subfolder
//! of that folder, which must be new or empty; `--assets` names the model's folder (default `assets`, from the
//! repository root). It prints one line per trial and writes the record to `--out`, which must not exist yet. Exit 0
//! when every trial agrees with its level, 1 when one does not, 2 for a usage or file error.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use c600_digest::sha256_hex;
use c600_kernel::Kernel;
use c600_lx2::session::{self, run_session_trial};
use c600_lx2::{record_json, run_trial, Trial};

const USAGE: &str =
    "usage: c600-lx2-faults --runtime <c600-runtime binary> [--sessions <new folder>] [--assets <folder>] [--out <record.json>]";

fn usage() -> ExitCode {
    eprintln!("{USAGE}");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut flags = BTreeMap::new();
    for pair in args.chunks(2) {
        let [flag, value] = pair else { return usage() };
        if !["--runtime", "--sessions", "--assets", "--out"].contains(&flag.as_str())
            || flags.insert(flag.clone(), PathBuf::from(value)).is_some()
        {
            return usage();
        }
    }
    let Some(runtime) = flags.get("--runtime").cloned() else { return usage() };
    let out = flags.get("--out").cloned();
    if out.as_ref().is_some_and(|out| out.exists()) {
        eprintln!("the record exists already; a record is never written over");
        return ExitCode::from(2);
    }
    let bytes = match fs::read(&runtime) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("the runtime binary cannot be read: {error}");
            return ExitCode::from(2);
        }
    };
    let sessions_folder = flags.get("--sessions").cloned();
    if let Some(folder) = &sessions_folder {
        if fs::read_dir(folder).is_ok_and(|mut entries| entries.next().is_some()) {
            eprintln!("the sessions folder is not empty; a session is never written over");
            return ExitCode::from(2);
        }
    }
    let records: Vec<_> = Trial::ALL.iter().map(|&trial| run_trial(&runtime, trial)).collect();
    for r in &records {
        println!(
            "{}: {} (expected {}), preview kept {}, recovered {}, spawns {}: {}",
            r.trial.name(),
            r.observed.describe(),
            r.trial.expected().describe(),
            r.preview_kept,
            r.recovered,
            r.spawns,
            if r.agrees() { "agrees" } else { "DISAGREES" }
        );
    }
    let mut sessions = Vec::new();
    if let Some(folder) = &sessions_folder {
        let assets = flags.get("--assets").cloned().unwrap_or_else(|| PathBuf::from("assets"));
        let kernel = match Kernel::load(&assets) {
            Ok(kernel) => Arc::new(kernel),
            Err(error) => {
                eprintln!("the model does not load: {error}");
                return ExitCode::from(2);
            }
        };
        for &trial in &Trial::ALL {
            let place = folder.join(trial.name());
            if let Err(error) = fs::create_dir_all(&place) {
                eprintln!("the session folder cannot be made: {error}");
                return ExitCode::from(2);
            }
            sessions.push(run_session_trial(&runtime, &kernel, &place, trial));
        }
        let control = sessions[0].clone();
        for r in &sessions {
            println!(
                "session {}: {}, open {}, {} journal rows, replayed {}: {}",
                r.trial.name(),
                r.observed.describe(),
                r.open,
                r.journal.len(),
                r.replayed.as_deref() == Some(r.state.as_str()),
                if r.agrees(&control) { "agrees with the control" } else { "DISAGREES" }
            );
        }
    }
    let name = runtime.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let record = record_json(&name, &sha256_hex(&bytes), &records, &sessions);
    if let Some(out) = out {
        if let Err(error) = fs::write(&out, record) {
            eprintln!("the record cannot be written: {error}");
            return ExitCode::from(2);
        }
    }
    if records.iter().all(|r| r.agrees()) && session::all_agree(&sessions) {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
