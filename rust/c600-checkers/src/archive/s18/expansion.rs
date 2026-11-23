//! `s18.expansion`: stage 18's gate 1 on a `steps.json` record of either form (matrix family 18/expansion, the 35
//! groups of both records and their step files; LB5.12; CT-LB5-05, CT-LB5-09, CT-LB1-04's expansion half).
//!
//! Gate 1 is the exact length before expansion, at most 10⁷, and the expanded word's length and hash. For every step:
//! the record's own claims ([`super::record::read_chain`]: the exact length X + 2 + copies·V equals `letters` and is
//! admitted before any word file is opened; the counts and the chain), then each stored part streamed once through its
//! length and hash ([`super::record::read_step`]): X, W, V′ and W⁻¹ with their recorded SHA-256 and lengths, W the
//! record's letter, W⁻¹ its inverse in the exact menu, and the unit of the recorded size. The four-segment form's V′⁻¹ is
//! the stored V′ read backwards with inverse letters; it is counted, never materialized. The supplied start table must
//! reproduce its own digest and the record's start, and carry the first step's off count; an optional stored end table
//! must carry the last step's end digest and off count.
//!
//! Nothing here replays a letter, so a pass says nothing about a state reached, the composed contract (gate 2) or a
//! twist-by-twist replay (gate 3); both recorded forms keep their own end (the matrix family's limits).

use super::record::{end_table, expansion_note, open, read_step, start_state};
use crate::archive::SubChecker;
use crate::claims::{dot, Run, Step};
use crate::kinds::{Checker, Needs};
use crate::report::Sources;
use crate::Engines;

pub static SOURCES: Sources = Sources::new(
    "s18.expansion",
    &[
        ("c600-checkers/Cargo.toml", include_bytes!("../../../Cargo.toml")),
        ("c600-checkers/src/archive/s18/expansion.rs", include_bytes!("expansion.rs")),
        ("c600-checkers/src/archive/s18/record.rs", include_bytes!("record.rs")),
        ("c600-checkers/src/archive/s18/words.rs", include_bytes!("words.rs")),
        ("c600-checkers/src/claims.rs", include_bytes!("../../claims.rs")),
        ("c600-checkers/src/kinds/mod.rs", include_bytes!("../../kinds/mod.rs")),
        ("c600-checkers/src/lib.rs", include_bytes!("../../lib.rs")),
        ("c600-checkers/src/replay.rs", include_bytes!("../../replay.rs")),
        ("c600-checkers/src/report.rs", include_bytes!("../../report.rs")),
    ],
);

pub static SUBCHECKER: SubChecker = SubChecker {
    checker: Checker { sources: &SOURCES, needs: Needs::JUMBLE, run },
    stage: 18,
    family: "18/expansion: S18 exact expanded length and hash, gate 1 (LB5.12, LB5.15)",
};

const SCOPE: &str = "gate 1 of a steps.json record (exact length before expansion and the stored words' hashes)";

fn run(engines: &Engines, run: &mut Run) -> Step<()> {
    let opened = open(engines, run, SCOPE)?;
    let state = start_state(&opened, run)?;
    let first = &opened.chain.steps[0];
    run.claim(&dot(&first.path, "off_before"), &first.off_before, &(state.off_lattice_count() as u64))?;
    drop(state);
    for step in &opened.chain.steps {
        read_step(run, step, &opened.inverse, false)?;
        run.held(dot(&step.path, "expansion"), expansion_note(step, opened.chain.form));
    }
    end_table(&opened, run, None)
}
