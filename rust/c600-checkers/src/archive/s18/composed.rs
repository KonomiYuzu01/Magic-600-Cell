//! `s18.composed`: stage 18's gate 2, the composed strict′ contract, on a `steps.json` record of either form, step by
//! step from the actual chained starts (matrix family 18/composed; LB5.12; CT-LB5-05).
//!
//! For each step, after gate 1's checks of its record and files ([`super::record::read_step`]):
//!
//! 1. the start is the stored start table for the first step and, for every later one, the end the replay reached,
//!    rebuilt by exact replay of the stored words under this checker's identity and marked `derived` (LP5-3); its
//!    digest must be the step's recorded start, its off count the recorded one, and every unit piece off;
//! 2. X, W and W⁻¹ are played twist by twist, every twist admissible; X ends at its file's output digest and W at its
//!    own;
//! 3. V′ and, in the four-segment form, V′⁻¹ are the composed gate's SAFE-ALL spans: every letter a nonidentity
//!    retained letter that moves only pieces on the lattice (stage 18 README section 3.3);
//! 4. the step ends at its recorded end digest with the recorded off count, and is strict′: the unit on the lattice,
//!    no collateral, every other off piece at its exact pose ([`super::play`]).
//!
//! An optional stored table of the chain's end must then carry the last step's end digest, which the replay reached,
//! and its off count ([`super::record::end_table`]).
//!
//! The SAFE-ALL spans are applied letter by letter here (KN6.2), not composed into one action as the archive's gate
//! did; the claims checked are gate 2's. This check is not gate 3: it claims no twist-by-twist replay of a step and
//! never completes the archive's third gate, which stays `historical_check_incomplete` for T6's step 4 and the rule's
//! three stages (`s18.tail_replay`). A step whose start table is not stored is checked only from the start this chain
//! reached: a matching profile is not a matching start, and dropping V′⁻¹ changes the end (the family's limits).

use c600_jumble::state::State;

use super::play::{check_start, check_strict, play, start_note, strict_prime, Letter};
use super::record::{end_table, open, read_step, start_state, word_digest, Form};
use crate::archive::SubChecker;
use crate::claims::{dot, Run, Step};
use crate::kinds::{Checker, Needs};
use crate::replay::Jumbling;
use crate::report::{codes, Sources, Span};
use crate::Engines;

pub static SOURCES: Sources = Sources::new(
    "s18.composed",
    &[
        ("c600-checkers/Cargo.toml", include_bytes!("../../../Cargo.toml")),
        ("c600-checkers/src/archive/s18/composed.rs", include_bytes!("composed.rs")),
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
    family: "18/composed: S18 composed strict-prime contract at the actual chained starts, gate 2 (LB5.12, LB5.15)",
};

const SCOPE: &str = "gate 2 of a steps.json record (the composed strict-prime contract from the actual chained starts)";

fn run(engines: &Engines, run: &mut Run) -> Step<()> {
    let opened = open(engines, run, SCOPE)?;
    let state = start_state(&opened, run)?;
    let mut j = Jumbling::new(opened.ctx, opened.menu.clone(), state);
    for (k, step) in opened.chain.steps.iter().enumerate() {
        let path = &step.path;
        let digest = j.digest();
        let code = if k == 0 { codes::START_TABLE_MISMATCH } else { codes::CERTIFICATE_REJECTED };
        run.compare(&dot(path, "start_digest"), &step.start, &digest, code)?;
        run.held(dot(path, "start"), start_note(k, &digest));
        let words = read_step(run, step, &opened.inverse, true)?;
        check_start(run, path, &j.state, &words.unit, step.off_before)?;
        let before: State = j.state.clone();
        let word = words.expanded(opened.chain.form, &opened.inverse);
        let id = word_digest(&word);
        let span = |from: usize, to: usize| Span { word: id.clone(), from: from as u64, to: to as u64 };

        let mut at = 0usize;
        let mut segment =
            |run: &mut Run, j: &mut Jumbling, name: &str, len: usize, kind: Letter| -> Step<(usize, usize)> {
                let from = at;
                for position in from..from + len {
                    play(j, word[position], kind, &format!("{path}.letters[{position}]"))
                        .map_err(|stop| run.failed(stop, "refused"))?;
                }
                at += len;
                let note = match kind {
                    Letter::Twist => format!("{len} letters, every twist admissible"),
                    Letter::Safe => format!("{len} letters, every letter SAFE-ALL at its state"),
                };
                run.held_span(dot(path, name), note, span(from, at));
                Ok((from, at))
            };

        if let Some((_, output)) = &words.x_digests {
            segment(run, &mut j, "x.play", words.x.len(), Letter::Twist)?;
            run.claim(&dot(path, "x.output_digest"), output, &j.digest())?;
        }
        segment(run, &mut j, "w.play", 1, Letter::Twist)?;
        run.claim(&dot(path, "w.output_digest"), &words.w_digests.1, &j.digest())?;
        segment(run, &mut j, "v.safe_all", words.v.len(), Letter::Safe)?;
        segment(run, &mut j, "winv.play", 1, Letter::Twist)?;
        if opened.chain.form == Form::FourSegment {
            segment(run, &mut j, "v_inverse.safe_all", words.v.len(), Letter::Safe)?;
        }
        let end = j.digest();
        if end != step.end {
            let stop = crate::claims::reject(dot(path, "end_digest"), format!("stated {}, computed {end}", step.end));
            return Err(run.failed(stop, &end));
        }
        run.held_span(dot(path, "end_digest"), &end, span(0, word.len()));
        let sp = strict_prime(&before, &j.state, &words.unit);
        check_strict(run, path, &sp, step.off_after)?;
    }
    end_table(&opened, run)?;
    run.held("scope", "gate 2 for every step; gate 3 is not claimed by this check (LB5.12 keeps the gates apart)");
    Ok(())
}
