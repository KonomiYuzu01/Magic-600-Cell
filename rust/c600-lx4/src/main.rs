//! Writes LX-4's module set and edit script (see the library):
//!
//! ```text
//! c600-lx4-fixtures --out <folder>
//! ```
//!
//! The folder must be new or empty. The tool generates the set, checks it and the script through the front end, and
//! writes nothing unless both agree: S.6cs, M.6cs, L.6cs, W/W00.6cs to W/W49.6cs, MANIFEST.json and EDITS.json. It
//! prints one summary line. Exit 0 when written, 1 when the front end disagrees, 2 for a usage or file error.

#![forbid(unsafe_code)]

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use c600_lx4::{edits, edits_json, generate, manifest_json, validate};
use c600_syntax::sha256_hex;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out = match args.as_slice() {
        [flag, folder] if flag == "--out" => PathBuf::from(folder),
        _ => {
            eprintln!("usage: c600-lx4-fixtures --out <folder>");
            return ExitCode::from(2);
        }
    };
    if out.exists() && fs::read_dir(&out).map_or(true, |mut entries| entries.next().is_some()) {
        eprintln!("{}: the folder is not empty; fixtures are never written over", out.display());
        return ExitCode::from(2);
    }
    let checked = generate().and_then(|fixtures| {
        let script = edits(&fixtures)?;
        let validation = validate(&fixtures, &script)?;
        let manifest = manifest_json(&fixtures, &validation)?;
        Ok((fixtures, script, validation, manifest))
    });
    let (fixtures, script, validation, manifest) = match checked {
        Ok(checked) => checked,
        Err(reason) => {
            eprintln!("lx4 fixtures refused: {reason}");
            return ExitCode::from(1);
        }
    };
    let script_json = edits_json(&fixtures, &script);
    let write = || -> std::io::Result<()> {
        fs::create_dir_all(out.join("W"))?;
        for module in fixtures.all() {
            fs::write(out.join(module.path()), &module.source)?;
        }
        fs::write(out.join("EDITS.json"), &script_json)?;
        fs::write(out.join("MANIFEST.json"), &manifest)
    };
    if let Err(error) = write() {
        eprintln!("{}: {error}", out.display());
        return ExitCode::from(2);
    }
    let lines: usize = fixtures.all().map(|m| m.lines()).sum();
    println!(
        "lx4 fixtures: {} modules, {lines} lines, {} edits, {} direct dependents of W00, manifest {}",
        fixtures.all().count(),
        script.len(),
        validation.direct_dependents.len(),
        sha256_hex(manifest.as_bytes())
    );
    ExitCode::SUCCESS
}
