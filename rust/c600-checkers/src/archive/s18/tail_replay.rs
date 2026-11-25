//! `s18.tail_replay`: stage 18's gate 3, a complete twist-by-twist replay of each step from the actual previous end,
//! reported as a chain of chunks (matrix family 18/tail_replay; LB5.12; CT-LB5-05, CT-LB5-09).
//!
//! For each step in scope, after gate 1's checks of its record and files ([`super::record::read_step`]): the start is
//! the stored start table for the first step and, for every later one, the end the replay reached, rebuilt by exact
//! replay of the stored words under this checker's identity and marked `derived` (LP5-3); its digest and off count
//! must be the recorded ones and every unit piece off. The step's whole word X·W·V′·W⁻¹(·V′⁻¹), whose length must be
//! the recorded one, is then played one twist per letter, every twist admissible, and must end at the recorded end
//! digest; the step must then be strict′.
//!
//! Chunks. The word is played as one continuous replay from the step's start and reported in chunks of at most 20,000
//! letters, as the archive cut its chunks: each chunk is a trace step citing the word by its digest and positions,
//! with the digests at its two ends. The archive played its chunks separately, each from a boundary table computed by
//! exact composition, so it had to check that its chunks tile the word, concatenate to it and chain digest to digest
//! (LB5.12). Here the chunks are consecutive slices of the one expanded word, played in order on one state, so they
//! tile it, concatenate to it and chain by construction; none of this is checked again, since such a check could not
//! fail. What the replay decides is checked: every twist, the end digest and strict′ (and, before it, the expanded
//! word's length against the record, a check of the expansion itself).
//!
//! Scope. The archive records gate 3 complete for T6's steps 1 to 3 and stopped it at the freeze for T6's step 4 and
//! for the rule's three stages (stage 18 README section 3.3). Those four steps keep `historical_check_incomplete` for
//! gate 3: they are never replayed or passed here, a record whose first step is one of them is `not_applicable`, and
//! every later step of a record is out of scope too, since its previous end is reached only through such a step. A
//! pass covers exactly the steps the trace's `scope` names. Each step in scope gets its history from the record alone,
//! before any table is read or letter played: a step whose start and end digests are one of [`COMPLETED`] is a step
//! the archive records gate 3 complete for, rechecked here; any other (a fixture's) is a new check, recorded as such.
//! No newly completed replay fills a historical gate.

use super::play::{check_start, check_strict, play, start_note, strict_prime, Letter};
use super::record::{open, read_step, start_state, word_digest, StepRecord};
use crate::archive::SubChecker;
use crate::claims::{dot, not_applicable, reject, Run, Step};
use crate::kinds::{Checker, Needs};
use crate::replay::Jumbling;
use crate::report::{codes, Sources, Span};
use crate::Engines;

pub static SOURCES: Sources = Sources::new(
    "s18.tail_replay",
    &[
        ("c600-checkers/Cargo.toml", include_bytes!("../../../Cargo.toml")),
        ("c600-checkers/src/archive/s18/play.rs", include_bytes!("play.rs")),
        ("c600-checkers/src/archive/s18/record.rs", include_bytes!("record.rs")),
        ("c600-checkers/src/archive/s18/tail_replay.rs", include_bytes!("tail_replay.rs")),
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
    family: "18/tail_replay: S18 complete literal replay and chunk chaining, gate 3 (LB5.12, LB5.15)",
};

const SCOPE: &str = "gate 3 of a steps.json record (a complete twist-by-twist replay in chunks whose digests chain)";

/// The archive's chunk length (stage 18 README section 3.3).
pub const CHUNK: usize = 20_000;

/// LB5.14's reason for a gate the archive did not finish.
pub const HISTORICAL_CHECK_INCOMPLETE: &str = "historical_check_incomplete";

/// The steps whose gate 3 the archive stopped at the owner's freeze of 27 October 2026 (stage 18 README section 3.3;
/// LB5.12), by name, start digest and end digest.
pub const INCOMPLETE: [(&str, &str, &str); 4] = [
    (
        "T6 step 4 (04-91-14)",
        "b28448f3c4e1f1f7b6fd45fcf5c747f80536225b6f23ac534cc269ab0de95a42",
        "c75c8dde2ff850d247c07722cd1059cb50c33ada4657632fed90f16704bfffc0",
    ),
    (
        "rule stage 1 (01-91-14)",
        "966dc8a96f7bf40b4c0361d7a26d9a3efbf62e6c59c1a6677cf5de9f8aefe661",
        "d9edc19cf63f2811be0b5e3542700a9e92872a3c0a8efa3cd1d9c3adb2caec29",
    ),
    (
        "rule stage 2 (02-217-15)",
        "d9edc19cf63f2811be0b5e3542700a9e92872a3c0a8efa3cd1d9c3adb2caec29",
        "ec6785c301b43fcd4863d0f14e6cc4a07e49967f475b0fc1dd8089a3746468a1",
    ),
    (
        "rule stage 3 (03-548-12)",
        "ec6785c301b43fcd4863d0f14e6cc4a07e49967f475b0fc1dd8089a3746468a1",
        "eea9b972f8b68708e56fb01e85badebfd64aad9e67e86ffb5c3c617bcb6c7075",
    ),
];

/// The steps whose gate 3 the archive records complete, one run of `stepchk.py --require-strict` each (stage 18
/// README section 3.3).
pub const COMPLETED: [(&str, &str, &str); 3] = [
    (
        "T6 step 1 (01-353-22)",
        "966dc8a96f7bf40b4c0361d7a26d9a3efbf62e6c59c1a6677cf5de9f8aefe661",
        "1007a60158dcdbaed7e8a4105a167f7c56de37e162d66796cb8b0415238897fe",
    ),
    (
        "T6 step 2 (02-217-15)",
        "1007a60158dcdbaed7e8a4105a167f7c56de37e162d66796cb8b0415238897fe",
        "5e9f262ca419fdf18ddaba81a870c03e50379c95e35baab280f4a9153d536d62",
    ),
    (
        "T6 step 3 (03-560-13)",
        "5e9f262ca419fdf18ddaba81a870c03e50379c95e35baab280f4a9153d536d62",
        "b28448f3c4e1f1f7b6fd45fcf5c747f80536225b6f23ac534cc269ab0de95a42",
    ),
];

fn find<'t>(table: &'t [(&'t str, &'t str, &'t str)], step: &StepRecord) -> Option<&'t str> {
    table.iter().find(|(_, start, end)| *start == step.start && *end == step.end).map(|(name, _, _)| *name)
}

/// A step's gate-3 history in the archive, from its recorded start and end digests.
fn history(step: &StepRecord) -> String {
    match find(&COMPLETED, step) {
        Some(name) => format!("{name}: the archive records gate 3 complete; rechecked here"),
        None => "the archive holds no gate-3 record for this step; a new check".to_owned(),
    }
}

fn run(engines: &Engines, run: &mut Run) -> Step<()> {
    let opened = open(engines, run, SCOPE)?;
    let steps = &opened.chain.steps;
    let first = steps.iter().position(|s| find(&INCOMPLETE, s).is_some()).unwrap_or(steps.len());
    let excluded: Vec<String> = steps[first..]
        .iter()
        .map(|s| match find(&INCOMPLETE, s) {
            Some(name) => format!("{} = {name}: {HISTORICAL_CHECK_INCOMPLETE}", s.path),
            None => format!("{}: after a step whose gate 3 is {HISTORICAL_CHECK_INCOMPLETE}", s.path),
        })
        .collect();
    if first == 0 {
        return Err(not_applicable(format!(
            "gate 3 is not checked for {}; the archive's twist-by-twist replays were stopped at the freeze (stage 18 \
             README section 3.3, LB5.12) and no check here completes or passes them",
            excluded.join("; ")
        )));
    }
    let scope = match excluded.is_empty() {
        true => format!("gate 3 for steps[0] to steps[{}]", first - 1),
        false => format!("gate 3 for steps[0] to steps[{}]; not applicable: {}", first - 1, excluded.join("; ")),
    };
    run.held("scope", scope);
    for step in &steps[..first] {
        run.held(dot(&step.path, "history"), history(step));
    }

    let state = start_state(&opened, run)?;
    let mut j = Jumbling::new(opened.ctx, opened.menu.clone(), state);
    for (k, step) in steps[..first].iter().enumerate() {
        let path = &step.path;
        let digest = j.digest();
        let code = if k == 0 { codes::START_TABLE_MISMATCH } else { codes::CERTIFICATE_REJECTED };
        run.compare(&dot(path, "start_digest"), &step.start, &digest, code)?;
        run.held(dot(path, "start"), start_note(k, &digest));
        let words = read_step(run, step, &opened.inverse, true)?;
        check_start(run, path, &j.state, &words.unit, step.off_before)?;
        let before = j.state.clone();
        let word = words.expanded(opened.chain.form, &opened.inverse);
        run.claim(&dot(path, "letters"), &step.letters, &(word.len() as u64))?;
        let id = word_digest(&word);

        // One continuous replay, reported in chunks: each chunk starts at the digest the one before ended at.
        let mut start = digest;
        let chunks = word.len().div_ceil(CHUNK);
        for (n, chunk) in word.chunks(CHUNK).enumerate() {
            let from = n * CHUNK;
            for (i, &letter) in chunk.iter().enumerate() {
                let claim = format!("{path}.letters[{}]", from + i);
                play(&mut j, letter, Letter::Twist, &claim).map_err(|stop| run.failed(stop, "refused"))?;
            }
            let end = j.digest();
            run.held_span(
                format!("{path}.chunks[{n}]"),
                format!("{start} -> {end}, every twist admissible"),
                Span { word: id.clone(), from: from as u64, to: (from + chunk.len()) as u64 },
            );
            start = end;
        }
        run.held(
            dot(path, "chunks"),
            format!(
                "{chunks} chunks of at most {CHUNK} letters, consecutive slices of the word played as one replay, so \
                 they tile it and chain by construction"
            ),
        );
        let end = start;
        if end != step.end {
            let stop = reject(dot(path, "end_digest"), format!("stated {}, computed {end}", step.end));
            return Err(run.failed(stop, &end));
        }
        run.held_span(dot(path, "end_digest"), &end, Span { word: id.clone(), from: 0, to: word.len() as u64 });
        let sp = strict_prime(&before, &j.state, &words.unit);
        check_strict(run, path, &sp, step.off_after)?;
    }
    Ok(())
}
