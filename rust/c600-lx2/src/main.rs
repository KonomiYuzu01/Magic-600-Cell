//! Runs LX-2's metered and forced-kill trials against a runtime child binary, and its native trials against the
//! injection child (see the library):
//!
//! ```text
//! c600-lx2-faults --runtime <c600-runtime binary> [--injection <c600-lx2-child binary>] [--sessions <new folder>]
//!                 [--assets <folder>] [--out <record.json>]
//! ```
//!
//! `--injection` names the fault-injection child of option (a), built from the same source in the same profile as the
//! product's runtime; the native trials run only on it. With `--sessions` each trial also runs with a session open
//! (RTE.42 conditions 1 and 2), each in its own subfolder of that folder, which must be new or empty; `--assets` names
//! the model's folder (default `assets`, from the repository root). It prints one line per trial and writes the record
//! to `--out`, which must not exist yet. Exit 0 when every trial agrees with its level, 1 when one does not, 2 for a
//! usage or file error.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use c600_digest::sha256_hex;
use c600_kernel::Kernel;
use c600_lx2::session::{self, run_session_trial};
use c600_lx2::{record_json, run_trial, Device, Trial};

const USAGE: &str = "usage: c600-lx2-faults --runtime <c600-runtime binary> [--injection <c600-lx2-child binary>] \
                     [--sessions <new folder>] [--assets <folder>] [--out <record.json>]";
const FLAGS: [&str; 5] = ["--runtime", "--injection", "--sessions", "--assets", "--out"];

fn usage() -> ExitCode {
    eprintln!("{USAGE}");
    ExitCode::from(2)
}

/// A child binary's file name and SHA-256, or the error to print.
fn identify(binary: &Path, what: &str) -> Result<Device, String> {
    let bytes = fs::read(binary).map_err(|error| format!("the {what} binary cannot be read: {error}"))?;
    let file = binary.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    Ok(Device { file, sha256: sha256_hex(&bytes) })
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut flags = BTreeMap::new();
    for pair in args.chunks(2) {
        let [flag, value] = pair else { return usage() };
        if !FLAGS.contains(&flag.as_str()) || flags.insert(flag.clone(), PathBuf::from(value)).is_some() {
            return usage();
        }
    }
    let Some(runtime) = flags.get("--runtime").cloned() else { return usage() };
    let injection = flags.get("--injection").cloned();
    let out = flags.get("--out").cloned();
    if out.as_ref().is_some_and(|out| out.exists()) {
        eprintln!("the record exists already; a record is never written over");
        return ExitCode::from(2);
    }
    let product = match identify(&runtime, "runtime") {
        Ok(product) => product,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    let device = match injection.as_deref().map(|binary| identify(binary, "injection child")).transpose() {
        Ok(device) => device,
        Err(error) => {
            eprintln!("{error}");
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
    // Every trial with the child it runs on: the product's own, or the injection child for the native trials.
    let mut plan: Vec<(Trial, &Path)> = Trial::ALL.iter().map(|&trial| (trial, runtime.as_path())).collect();
    if let Some(binary) = &injection {
        plan.extend(Trial::NATIVE.iter().map(|&trial| (trial, binary.as_path())));
    }
    let records: Vec<_> = plan.iter().map(|&(trial, child)| run_trial(child, trial)).collect();
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
        for &(trial, child) in &plan {
            let place = folder.join(trial.name());
            if let Err(error) = fs::create_dir_all(&place) {
                eprintln!("the session folder cannot be made: {error}");
                return ExitCode::from(2);
            }
            sessions.push(run_session_trial(child, &kernel, &place, trial));
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
    let record = record_json(&product.file, &product.sha256, device.as_ref(), &records, &sessions);
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
