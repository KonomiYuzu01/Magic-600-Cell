//! `s19.pose-table`: a stored pose table of stage 19, checked as stored and as decoded.
//!
//! The certificate (`kn6-s19-pose-table/1`) states one archive member as `IMPORT-MANIFEST.json` lists it, with the
//! state digest the table states for itself:
//!
//! ```text
//! {"format": "kn6-s19-pose-table/1", "path": "records/...json.gz",
//!  "sha256": stored SHA-256, "stored_bytes": n, "uncompressed_sha256": decoded SHA-256, "uncompressed_bytes": n,
//!  "digest": the state digest}
//! ```
//!
//! The input `table` is the member's stored bytes. The checker compares the stored length and SHA-256, decodes a gzip
//! member within the stated decoded length, compares the decoded length and SHA-256, and validates the table in full
//! (exact Q(√5) entries in lowest terms, exact rotations, identity first, no pose repeated, 177,120 pose
//! IDs, an empty journal) before comparing the engine's state digest with the table's own and the stated one. Every
//! mismatch of the input is the wrong start table. `path` names the member and is no claim. The original-source
//! digest of the manifest is PS3's correspondence, not this check (LB5.14).

use c600_kernel::json::Value;

use super::common::{decode, form, outside, parse_table, DECODED_LIMIT};
use crate::archive::SubChecker;
use crate::claims::{
    count, document, dot, field, hex64, keys, reject, require_format, start_mismatch, string, Run, Step,
};
use crate::kinds::{Checker, Needs};
use crate::report::{codes, Sources};
use crate::Engines;

pub const FORMAT: &str = "kn6-s19-pose-table/1";

const KEYS: [&str; 7] =
    ["format", "path", "sha256", "stored_bytes", "uncompressed_sha256", "uncompressed_bytes", "digest"];

pub static SOURCES: Sources = Sources::new(
    "s19.pose-table",
    &[
        ("c600-checkers/Cargo.toml", include_bytes!("../../../Cargo.toml")),
        ("c600-checkers/src/archive/s19/common.rs", include_bytes!("common.rs")),
        ("c600-checkers/src/archive/s19/pose_table.rs", include_bytes!("pose_table.rs")),
        ("c600-checkers/src/claims.rs", include_bytes!("../../claims.rs")),
        ("c600-checkers/src/kinds/mod.rs", include_bytes!("../../kinds/mod.rs")),
        ("c600-checkers/src/lib.rs", include_bytes!("../../lib.rs")),
        ("c600-checkers/src/replay.rs", include_bytes!("../../replay.rs")),
        ("c600-checkers/src/report.rs", include_bytes!("../../report.rs")),
    ],
);

pub static SUBCHECKER: SubChecker = SubChecker {
    checker: Checker { sources: &SOURCES, needs: Needs::JUMBLE, run },
    stage: 19,
    family: "S19 complete pose-table validity (stored and decoded digests, exact entries, state digest)",
};

fn run(engines: &Engines, run: &mut Run) -> Step<()> {
    let ctx = engines.jumble.ok_or_else(|| reject("certificate", "the jumbling engine is absent"))?;
    let doc = document(run.cert.bytes)?;
    // A record of another form is outside the kind; one of this form must be well formed, its format included.
    if form(&doc, &[&KEYS]).is_none() {
        return Err(outside(SOURCES.id, "a stored pose table's member certificate", &doc));
    }
    keys(&doc, "certificate", &KEYS, &[])?;
    require_format(&doc, FORMAT)?;
    let path = string(field(&doc, "path", "path")?, "path")?.to_owned();
    let stated = |key: &str| -> Step<&Value> { field(&doc, key, key) };
    let bytes = run.input("table")?;

    let stored_bytes = count(stated("stored_bytes")?, "stored_bytes")?;
    run.compare("stored_bytes", &stored_bytes, &(bytes.len() as u64), codes::START_TABLE_MISMATCH)?;
    let sha = hex64(stated("sha256")?, "sha256")?.to_owned();
    run.compare("sha256", &sha, &c600_digest::sha256_hex(bytes), codes::START_TABLE_MISMATCH)?;

    // The stated length bounds the decoding, within the checker's own bound.
    let length = count(stated("uncompressed_bytes")?, "uncompressed_bytes")?;
    let bound = usize::try_from(length).unwrap_or(usize::MAX).min(DECODED_LIMIT);
    let decoded = decode(bytes, bound).map_err(|error| start_mismatch("uncompressed_bytes", error))?;
    run.compare("uncompressed_bytes", &length, &(decoded.len() as u64), codes::START_TABLE_MISMATCH)?;
    let decoded_sha = hex64(stated("uncompressed_sha256")?, "uncompressed_sha256")?.to_owned();
    run.compare("uncompressed_sha256", &decoded_sha, &c600_digest::sha256_hex(&decoded), codes::START_TABLE_MISMATCH)?;
    drop(decoded);

    let table = parse_table(ctx, None, bytes, "table")?;
    run.held(dot("table", "validity"), format!("{} exact poses, every one a rotation, used and distinct", table.poses));
    let digest = hex64(stated("digest")?, "digest")?.to_owned();
    run.claim("digest", &digest, &table.digest)?;
    run.bind(
        "table",
        bytes,
        vec![
            ("path", path),
            ("decoded_sha256", decoded_sha),
            ("digest", table.digest.clone()),
            ("off_lattice", table.state.off_lattice_count().to_string()),
        ],
    );
    Ok(())
}
