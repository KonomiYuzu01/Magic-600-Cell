//! Admissibility and straddlers on the jumbling puzzle (`kn6-admissibility/1`).
//!
//! A certificate names a menu and a start state by the SHA-256 of its inputs, then a word whose every letter is claimed
//! admissible (applied, with its moved set) or blocked (not applied, with a straddler: a piece and two exact points of
//! its posed region on either side of the cut), the end digest, and optionally a survey of grips at the end, each
//! admissible or blocked with its straddler.
//!
//! An admissible claim is decided by the engine's exact classification of every piece for the cut, with no budget; a
//! blocked claim is decided by its straddler alone: both points must lie in the piece's posed region, checked against
//! all its constraints, one strictly below and one strictly above the cut (`State::check_certificate`). The check
//! never trusts a stated status: a claimed admissible letter that the engine finds blocked fails, and so does a
//! straddler whose points do not straddle.
//!
//! ```text
//! { "format": "kn6-admissibility/1",
//!   "inputs": {"menu": sha256, "start": sha256},          (start optional: absent means the solved state)
//!   "menu": {"identity": ..., "order": ...},
//!   "start": {"digest": ...},
//!   "steps": [{"letter": [cap, item], "status": "admissible", "moved": {"size": n, "sha256": ...}},
//!             {"letter": [cap, item], "status": "blocked", "straddler": {"piece": p, "below": point, "above": point}}],
//!   "end": {"digest": ...},
//!   "survey": [{"grip": g, "status": "admissible"}, {"grip": g, "status": "blocked", "straddler": {...}}] }
//! ```
//!
//! A point is `[[a, b, d] × 4]`, each coordinate (a + b√5)/d in lowest terms; `moved` is the count and the SHA-256 of
//! the sorted piece IDs as int64.

use c600_jumble::model::{NP, POLES};
use c600_jumble::state::Status;
use c600_kernel::json::Value;

use super::{Checker, Needs};
use crate::claims::{
    array, at, count, document, dot, field, hex64, int, keys, reject, require_format, string, Run, Step,
};
use crate::replay::{letter, point, read_menu, read_state, support, Jumbling};
use crate::report::{codes, Sources};
use crate::Engines;

pub const FORMAT: &str = "kn6-admissibility/1";

pub static SOURCES: Sources = Sources::new(
    "kn6.admissibility/1",
    &[
        ("c600-checkers/Cargo.toml", include_bytes!("../../Cargo.toml")),
        ("c600-checkers/src/claims.rs", include_bytes!("../claims.rs")),
        ("c600-checkers/src/kinds/admissibility.rs", include_bytes!("admissibility.rs")),
        ("c600-checkers/src/kinds/mod.rs", include_bytes!("mod.rs")),
        ("c600-checkers/src/lib.rs", include_bytes!("../lib.rs")),
        ("c600-checkers/src/replay.rs", include_bytes!("../replay.rs")),
        ("c600-checkers/src/report.rs", include_bytes!("../report.rs")),
    ],
);

pub static CHECKER: Checker = Checker { sources: &SOURCES, needs: Needs::JUMBLE, run };

fn run(engines: &Engines, run: &mut Run) -> Step<()> {
    let ctx = engines.jumble.ok_or_else(|| reject("certificate", "the jumbling engine is absent"))?;
    let doc = document(run.cert.bytes)?;
    require_format(&doc, FORMAT)?;
    keys(&doc, "certificate", &["format", "inputs", "menu", "start", "steps", "end"], &["survey"])?;
    run.check_bindings(&doc, &["start"])?;

    let menu = read_menu(ctx, run, "menu")?;
    let stated_menu = field(&doc, "menu", "menu")?;
    keys(stated_menu, "menu", &["identity", "order"], &[])?;
    run.claim(
        "menu.identity",
        &hex64(field(stated_menu, "identity", "menu")?, "menu.identity")?.to_owned(),
        &menu.identity,
    )?;
    run.claim("menu.order", &hex64(field(stated_menu, "order", "menu")?, "menu.order")?.to_owned(), &menu.order)?;
    let items = menu.items.len();

    let state = read_state(ctx, run, &menu, "start")?;
    let mut j = Jumbling::new(ctx, menu, state);
    let start = field(&doc, "start", "start")?;
    keys(start, "start", &["digest"], &[])?;
    let stated = hex64(field(start, "digest", "start")?, "start.digest")?.to_owned();
    run.compare("start.digest", &stated, &j.digest(), codes::START_TABLE_MISMATCH)?;

    for (k, step) in array(field(&doc, "steps", "steps")?, "steps")?.iter().enumerate() {
        let path = at("steps", k);
        keys(step, &path, &["letter", "status"], &["moved", "straddler"])?;
        let (cap, item) = letter(field(step, "letter", &path)?, &dot(&path, "letter"), items)?;
        let status_claim = dot(&path, "status");
        match string(field(step, "status", &path)?, &status_claim)? {
            "admissible" => {
                keys(step, &path, &["letter", "status", "moved"], &[])?;
                let twist = j.twist(cap, item, &path)?;
                let outcome = j.apply(&twist, &path)?;
                if outcome.status != Status::Admissible {
                    let stop = reject(&status_claim, format!("the letter is {}", outcome.status.as_str()));
                    return Err(run.failed(stop, outcome.status.as_str()));
                }
                run.held(&status_claim, "admissible");
                let moved_claim = dot(&path, "moved");
                let moved = field(step, "moved", &path)?;
                keys(moved, &moved_claim, &["size", "sha256"], &[])?;
                let (size, sha) = support(&outcome.inside);
                run.claim(
                    &dot(&moved_claim, "size"),
                    &count(field(moved, "size", &moved_claim)?, &moved_claim)?,
                    &size,
                )?;
                let stated = hex64(field(moved, "sha256", &moved_claim)?, &dot(&moved_claim, "sha256"))?.to_owned();
                run.claim(&dot(&moved_claim, "sha256"), &stated, &sha)?;
            }
            "blocked" => {
                keys(step, &path, &["letter", "status", "straddler"], &[])?;
                straddler(run, &j, cap, field(step, "straddler", &path)?, &dot(&path, "straddler"))?;
                run.held(&status_claim, "blocked");
            }
            _ => return Err(reject(&status_claim, "a status is admissible or blocked")),
        }
    }

    let end = field(&doc, "end", "end")?;
    keys(end, "end", &["digest"], &[])?;
    let stated = hex64(field(end, "digest", "end")?, "end.digest")?.to_owned();
    run.claim("end.digest", &stated, &j.digest())?;

    if let Some(survey) = doc.get("survey") {
        let mut seen = vec![false; POLES];
        for (k, entry) in array(survey, "survey")?.iter().enumerate() {
            let path = at("survey", k);
            keys(entry, &path, &["grip", "status"], &["straddler"])?;
            let grip = usize::try_from(int(field(entry, "grip", &path)?, &dot(&path, "grip"))?)
                .ok()
                .filter(|&g| g < POLES && !seen[g])
                .ok_or_else(|| reject(dot(&path, "grip"), "a grip is 0 to 599 and named once"))?;
            seen[grip] = true;
            let status_claim = dot(&path, "status");
            match string(field(entry, "status", &path)?, &status_claim)? {
                "admissible" => {
                    keys(entry, &path, &["grip", "status"], &[])?;
                    let classified = j
                        .state
                        .classify(ctx, grip, false, true, None)
                        .map_err(|error| reject(&status_claim, format!("the engine stopped: {}", error.0)))?;
                    if classified.status() != Status::Admissible {
                        let stop = reject(&status_claim, format!("the grip is {}", classified.status().as_str()));
                        return Err(run.failed(stop, classified.status().as_str()));
                    }
                    run.held(&status_claim, "admissible");
                }
                "blocked" => {
                    keys(entry, &path, &["grip", "status", "straddler"], &[])?;
                    straddler(run, &j, grip, field(entry, "straddler", &path)?, &dot(&path, "straddler"))?;
                    run.held(&status_claim, "blocked");
                }
                _ => return Err(reject(&status_claim, "a status is admissible or blocked")),
            }
        }
    }
    Ok(())
}

/// A straddler of the cut of `cap`: both points in the piece's posed region, one strictly on each side of the cut.
fn straddler(run: &mut Run, j: &Jumbling, cap: usize, value: &Value, claim: &str) -> Step<()> {
    keys(value, claim, &["piece", "below", "above"], &[])?;
    let piece = usize::try_from(int(field(value, "piece", claim)?, &dot(claim, "piece"))?)
        .ok()
        .filter(|&p| p < NP)
        .ok_or_else(|| reject(dot(claim, "piece"), format!("a piece is 0 to {}", NP - 1)))?;
    let below = point(field(value, "below", claim)?, &dot(claim, "below"))?;
    let above = point(field(value, "above", claim)?, &dot(claim, "above"))?;
    if !j.state.check_certificate(j.ctx, cap, piece, &below, &above) {
        let stop = reject(claim, format!("the points do not certify piece {piece} straddling the cut of cap {cap}"));
        return Err(run.failed(stop, "no straddle"));
    }
    run.held(claim, format!("piece {piece} straddles cap {cap}"));
    Ok(())
}
