//! `s18.lattice_replay`: a fresh letter-by-letter replay of stage 18's lattice phase from T6's stored end table
//! (matrix family 18/lattice_run; LB5.13; CT-LB5-05, CT-LB5-09).
//!
//! After the record's own claims, read in its exact form ([`super::lattice::read_run`]; the inputs are the menu, the
//! start table, the words and the step tables it names), the start table must reproduce its own digest and the
//! record's start (`KN6:evidence.start_table_mismatch` otherwise). Each step's word is admitted, decoded within its
//! declared size and hashed ([`super::words::ia16_member`]), then played from the state the previous step reached, one
//! twist per letter, every twist admissible; the step must end at its recorded after digest with nothing off. A table
//! the record stores at a step's end, when supplied, must reproduce its own digest and the digest reached.
//!
//! This is a new check record, not a reproduction of an archived one: `check_run.py`'s output
//! `results/lattice-d1/check.jsonl`, which the stage README and LB5.15 cite, is not in the repository, so there is
//! nothing to match this replay against, and the trace says so. The labels the runner's report compares are not
//! computed here.

use super::lattice::{only, open_run, read_run, word_roles};
use super::play::{play, Letter};
use super::words::ia16_member;
use crate::archive::SubChecker;
use crate::claims::{dot, reject, start_mismatch, Run, Step};
use crate::kinds::{Checker, Needs};
use crate::replay::{read_menu, read_state, Jumbling};
use crate::report::{codes, Sources, Span};
use crate::Engines;
use c600_jumble::nu::{I_A_MENU_IDENTITY, I_A_ORDER_DIGEST};

pub static SOURCES: Sources = Sources::new(
    "s18.lattice_replay",
    &[
        ("c600-checkers/Cargo.toml", include_bytes!("../../../Cargo.toml")),
        ("c600-checkers/src/archive/s18/lattice.rs", include_bytes!("lattice.rs")),
        ("c600-checkers/src/archive/s18/lattice_replay.rs", include_bytes!("lattice_replay.rs")),
        ("c600-checkers/src/archive/s18/play.rs", include_bytes!("play.rs")),
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
    family:
        "18/lattice_run: a fresh literal replay of the lattice-d1 run's words from its stored start (LB5.13, LB5.15)",
};

const SCOPE: &str = "a fresh literal replay of a runner-run/3 lattice phase";

fn run(engines: &Engines, run: &mut Run) -> Step<()> {
    let ctx = engines.jumble.ok_or_else(|| reject("certificate", "the jumbling engine is absent"))?;
    let doc = open_run(run, SCOPE)?;
    let record = read_run(run, &doc)?;
    let mut named: Vec<&str> = vec!["menu", record.start_file.as_str()];
    named.extend(word_roles(&record));
    named.extend(record.steps.iter().filter_map(|s| s.table.as_deref()));
    only(run, &named)?;
    let menu = read_menu(ctx, run, "menu")?;
    run.claim("menu.identity", &I_A_MENU_IDENTITY.to_owned(), &menu.identity)?;
    run.claim("menu.order", &I_A_ORDER_DIGEST.to_owned(), &menu.order)?;

    if run.optional_input(&record.start_file).is_none() {
        return Err(start_mismatch("start", "the run's start table is absent"));
    }
    let state = read_state(ctx, run, &menu, &record.start_file)?;
    let digest = state.digest(ctx);
    run.compare("start.digest", &record.start.digest, &digest, codes::START_TABLE_MISMATCH)?;
    let mut j = Jumbling::new(ctx, menu.clone(), state);
    let mut played = 0u64;
    for step in &record.steps {
        let path = &step.path;
        let bytes = run.input(&step.word)?;
        let (decoded, sha) = ia16_member(&step.word, bytes, step.letters, path)?;
        run.bind(&step.word, bytes, vec![("decoded_sha256", sha.clone())]);
        run.claim(&dot(path, "word_sha256"), &step.sha, &sha)?;
        for (position, pair) in decoded.chunks_exact(2).enumerate() {
            let code = u16::from_le_bytes([pair[0], pair[1]]);
            play(&mut j, code, Letter::Twist, &format!("{path}.letters[{position}]"))
                .map_err(|stop| run.failed(stop, "refused"))?;
        }
        drop(decoded);
        played += step.letters;
        let reached = j.digest();
        if reached != step.after.digest {
            let stop = reject(dot(path, "after.digest"), format!("stated {}, computed {reached}", step.after.digest));
            return Err(run.failed(stop, &reached));
        }
        run.held_span(
            dot(path, "after.digest"),
            format!("{reached}, every twist admissible"),
            Span { word: sha, from: 0, to: step.letters },
        );
        run.claim(&dot(path, "after.off"), &step.after.off, &(j.state.off_lattice_count() as u64))?;
        if let Some(table) = &step.table {
            if run.optional_input(table).is_some() {
                let stored = read_state(ctx, run, &menu, table)?;
                run.claim(&dot(path, "table"), &reached, &stored.digest(ctx))?;
            }
        }
    }
    run.held(
        "fresh_check",
        format!(
            "{played} letters replayed from the stored start table; results/lattice-d1/check.jsonl is not in the \
             repository, so there is no archived check to match this new check record against"
        ),
    );
    Ok(())
}
