//! Stage 14's literal replay rows (LB5.15's third stage 14 family; the claim matrix's `14/literal` groups): a stored
//! word replayed from a stored stage 13 table with every chronological observable recomputed after every letter.
//!
//! The records carry no format key; one is recognised by any of its fields ([`common::own`]) and must then have
//! exactly these: `table` (the table's historical path, which binds nothing: LB5.14), `word` (the letters
//! `[cap, item]` of the I_a menu) and `rows`. Row 0 is the start
//! (`letter` and `applied` null); row k + 1 follows letter k. Each row states n₂, n₄ and n₆ (the pieces of left height
//! 2, 4 and 6), the largest height, Φ = Σ max(0, h − 2), the number of pieces off the lattice and the state digest.
//!
//! The check reads the start table, whose digest must be row 0's (else the wrong start table), then applies the
//! letters one by one on the exact engine, in order. Each letter's `applied` is whether the engine admits it (a refused
//! letter changes nothing), and each row's observables are recomputed from the heights of every pose of the state after
//! it, never carried from the row before. The rows' order is the word's: a row stated for another letter fails.

use c600_jumble::state::Status;
use c600_kernel::json::Value;

use super::common::{self, claim_bool, claim_int, claim_str, engine, observe, start, word};
use crate::archive::SubChecker;
use crate::claims::{array, at, document, field, hex64, keys, reject, string, Run, Step};
use crate::kinds::{Checker, Needs};
use crate::replay::{letter, tree_tables};
use crate::report::Sources;
use crate::Engines;

pub static SOURCES: Sources = Sources::new(
    "s14.literal",
    &[
        ("c600-checkers/Cargo.toml", include_bytes!("../../../Cargo.toml")),
        ("c600-checkers/src/archive/s14/common.rs", include_bytes!("common.rs")),
        ("c600-checkers/src/archive/s14/literal.rs", include_bytes!("literal.rs")),
        ("c600-checkers/src/claims.rs", include_bytes!("../../claims.rs")),
        ("c600-checkers/src/kinds/mod.rs", include_bytes!("../../kinds/mod.rs")),
        ("c600-checkers/src/lib.rs", include_bytes!("../../lib.rs")),
        ("c600-checkers/src/replay.rs", include_bytes!("../../replay.rs")),
        ("c600-checkers/src/report.rs", include_bytes!("../../report.rs")),
    ],
);

pub static SUBCHECKER: SubChecker = SubChecker {
    checker: Checker { sources: &SOURCES, needs: Needs::JUMBLE, run },
    stage: 14,
    family: "S14 literal chronological observables (replay rows)",
};

/// The fields of one row.
const ROW: [&str; 9] = ["letter", "applied", "n2", "n4", "n6", "max_h", "phi", "off", "digest"];

fn run(engines: &Engines, run: &mut Run) -> Step<()> {
    let ctx = engine(engines)?;
    let doc = document(run.cert.bytes)?;
    common::own(&doc, SOURCES.id)?;
    keys(&doc, "certificate", &["table", "word", "rows"], &[])?;
    string(field(&doc, "table", "table")?, "table")?;
    let rows = array(field(&doc, "rows", "rows")?, "rows")?;
    let Some(first) = rows.first() else {
        return Err(reject("rows", "a replay has its start row"));
    };
    keys(first, "rows[0]", &ROW, &[])?;
    let stated = hex64(field(first, "digest", "rows[0]")?, "rows[0].digest")?.to_owned();
    let mut j = start(ctx, run, "rows[0].digest", &stated)?;
    let letters = word(field(&doc, "word", "word")?, "word", j.menu.items.len())?;
    run.claim("rows.length", &rows.len(), &(letters.len() + 1))?;
    for key in ["letter", "applied"] {
        let claim = format!("rows[0].{key}");
        if !matches!(field(first, key, "rows[0]")?, Value::Null) {
            return Err(run.failed(reject(&claim, "the start row names no letter"), "null"));
        }
        run.held(&claim, "null");
    }
    let tables = tree_tables(ctx, "rows")?;
    check_row(run, &j, tables, first, "rows[0]")?;
    for (k, &(cap, item)) in letters.iter().enumerate() {
        let path = at("rows", k + 1);
        let row = &rows[k + 1];
        keys(row, &path, &ROW, &[])?;
        let claim = format!("{path}.letter");
        let stated = letter(field(row, "letter", &path)?, &claim, j.menu.items.len())?;
        if stated != (cap, item) {
            let stop = reject(&claim, format!("the row names [{}, {}], not the word's letter", stated.0, stated.1));
            return Err(run.failed(stop, format!("[{cap}, {item}]")));
        }
        run.held(&claim, format!("[{cap}, {item}]"));
        let twist = j.twist(cap, item, &claim)?;
        let outcome = j.apply(&twist, &claim)?;
        claim_bool(run, row, &path, "applied", outcome.status == Status::Admissible)?;
        check_row(run, &j, tables, row, &path)?;
    }
    Ok(())
}

/// One row's observables, recomputed from the state.
fn check_row(
    run: &mut Run,
    j: &crate::replay::Jumbling,
    tables: &c600_jumble::observe::Tables,
    row: &Value,
    path: &str,
) -> Step<()> {
    let seen = observe(j.ctx, tables, &j.state, path)?;
    claim_int(run, row, path, "n2", seen.n2)?;
    claim_int(run, row, path, "n4", seen.n4)?;
    claim_int(run, row, path, "n6", seen.n6)?;
    claim_int(run, row, path, "max_h", seen.max_h)?;
    claim_int(run, row, path, "phi", seen.phi)?;
    claim_int(run, row, path, "off", seen.off)?;
    claim_str(run, row, path, "digest", &seen.digest)
}
