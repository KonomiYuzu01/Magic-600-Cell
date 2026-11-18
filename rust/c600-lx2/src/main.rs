//! Runs LX-2's metered and forced-kill trials against a runtime child binary (see the library):
//!
//! ```text
//! c600-lx2-faults --runtime <c600-runtime binary> [--out <record.json>]
//! ```
//!
//! It prints one line per trial and writes the record to `--out`, which must not exist yet. Exit 0 when every trial
//! agrees with its level, 1 when one does not, 2 for a usage or file error.

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use c600_digest::sha256_hex;
use c600_lx2::{record_json, run_trial, Trial};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (runtime, out) = match args.as_slice() {
        [flag, runtime] if flag == "--runtime" => (PathBuf::from(runtime), None),
        [flag, runtime, out_flag, out] if flag == "--runtime" && out_flag == "--out" => {
            (PathBuf::from(runtime), Some(PathBuf::from(out)))
        }
        _ => {
            eprintln!("usage: c600-lx2-faults --runtime <c600-runtime binary> [--out <record.json>]");
            return ExitCode::from(2);
        }
    };
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
    let name = runtime.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let record = record_json(&name, &sha256_hex(&bytes), &records);
    if let Some(out) = out {
        if let Err(error) = fs::write(&out, record) {
            eprintln!("the record cannot be written: {error}");
            return ExitCode::from(2);
        }
    }
    if records.iter().all(|r| r.agrees()) {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
