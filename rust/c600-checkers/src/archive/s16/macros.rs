//! `s16.macro`: the stage's `macro-certificate/1` certificates and `macro-check/1` records, checked by the macro kind
//! with the chain binding to the jam catalogue ([`crate::archive::s15::jam`]): stage 16 starts at the jam (the prefix is the jam word, the start is the jam end).
//!
//! Inputs: `table` (the D2 end table the certificate names), `menu` (I_a), `jams` (stage 16's `jams.json`), and for
//! a check record `certificate` (the certificate it checked). A release that ends in a retained letter is outside the
//! macro kind (LB5.11) and is `not_applicable` here.

use crate::archive::s15::jam::{run_family, Start};
use crate::archive::SubChecker;
use crate::claims::{Run, Step};
use crate::kinds::{Checker, Needs};
use crate::report::Sources;
use crate::Engines;

pub static SOURCES: Sources = Sources::new(
    "s16.macro",
    &[
        ("c600-checkers/Cargo.toml", include_bytes!("../../../Cargo.toml")),
        ("c600-checkers/src/archive/s15/jam.rs", include_bytes!("../s15/jam.rs")),
        ("c600-checkers/src/archive/s16/macros.rs", include_bytes!("macros.rs")),
        ("c600-checkers/src/claims.rs", include_bytes!("../../claims.rs")),
        ("c600-checkers/src/kinds/macros.rs", include_bytes!("../../kinds/macros.rs")),
        ("c600-checkers/src/kinds/mod.rs", include_bytes!("../../kinds/mod.rs")),
        ("c600-checkers/src/lib.rs", include_bytes!("../../lib.rs")),
        ("c600-checkers/src/replay.rs", include_bytes!("../../replay.rs")),
        ("c600-checkers/src/report.rs", include_bytes!("../../report.rs")),
    ],
);

pub static SUB: SubChecker = SubChecker {
    checker: Checker { sources: &SOURCES, needs: Needs::JUMBLE, run },
    stage: 16,
    family: "S16 macro-certificate/1 and macro-check/1: jam descents (I, O, SJ rows) and P** whole descents from the jams (LB5.15)",
};

fn run(engines: &Engines, run: &mut Run) -> Step<()> {
    run_family(engines, run, Start::JamEnd)
}
