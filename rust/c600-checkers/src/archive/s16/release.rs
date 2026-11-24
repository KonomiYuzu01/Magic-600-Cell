//! `s16.release`: stage 16's literal traces (`results/certificates/*-trace.json`: SJ-E1, T2-cx, I-7304, I-7306),
//! checked letter by letter, not as macro certificates (LB5.11). A release such as SJ-E1 ends in a retained letter and
//! is no descending macro ending in a jumble; this check recomputes what the trace states after every letter and
//! claims nothing about a descent.
//!
//! A trace record is formatless: `{"seed", "word", "trace"}`. Inputs: `jams` (stage 16's catalogue), `table` (the
//! seed's D2 end table) and `menu` (I_a). The table must be the seed's (its state digest is the catalogue's
//! `table_digest`); the jam word (the chain and the macros) is replayed letter by letter from it and must end at the
//! catalogue's `end_digest` and `end_phi`. Then the trace is checked event by event:
//!
//! - event 0: the jam itself: `phi`, `digest`, and at every named grip `S`, `F` and `maxD`;
//! - event e ≥ 1: `letter` is the word's e-th letter, admissible when it acts; `support` is the number of pieces it
//!   moves; `phi`, `delta` (the change of Φ) and `digest` after it; and `S`, `F` and `maxD` at every named grip after
//!   it ([`crate::archive::s15::grips`]).
//!
//! Every event names the same grips, at least one. A record of another shape fails at the field that departs from it.

use c600_jumble::nu::{I_A_MENU_IDENTITY, I_A_ORDER_DIGEST};
use c600_jumble::state::Status;
use c600_kernel::json::Value;

use crate::archive::s15::grips::sfd;
use crate::archive::s15::jam::{jam_of_seed, letters, read_jams, Letter};
use crate::archive::SubChecker;
use crate::claims::{
    array, at, document, dot, field, hex64, int, keys, object, reject, start_mismatch, string, Run, Step,
};
use crate::kinds::{Checker, Needs};
use crate::replay::{phi, read_menu, read_state, tree_tables, Jumbling};
use crate::report::{codes, Sources};
use crate::Engines;

pub static SOURCES: Sources = Sources::new(
    "s16.release",
    &[
        ("c600-checkers/Cargo.toml", include_bytes!("../../../Cargo.toml")),
        ("c600-checkers/src/archive/s15/grips.rs", include_bytes!("../s15/grips.rs")),
        ("c600-checkers/src/archive/s15/jam.rs", include_bytes!("../s15/jam.rs")),
        ("c600-checkers/src/archive/s16/release.rs", include_bytes!("release.rs")),
        ("c600-checkers/src/claims.rs", include_bytes!("../../claims.rs")),
        ("c600-checkers/src/kinds/mod.rs", include_bytes!("../../kinds/mod.rs")),
        ("c600-checkers/src/lib.rs", include_bytes!("../../lib.rs")),
        ("c600-checkers/src/replay.rs", include_bytes!("../../replay.rs")),
        ("c600-checkers/src/report.rs", include_bytes!("../../report.rs")),
    ],
);

pub static SUB: SubChecker = SubChecker {
    checker: Checker { sources: &SOURCES, needs: Needs::JUMBLE, run },
    stage: 16,
    family:
        "S16 literal release and jam traces (SJ-E1, T2-cx, I-7304, I-7306): letter-by-letter checks, not MC (LB5.11)",
};

/// Replays the jam of `seed` from the supplied table: the table must be the seed's, every letter admissible, and the
/// end the catalogue's. Returns the state at the jam.
pub fn jam_state<'e>(engines: &Engines<'e>, run: &mut Run, seed: &str, claim: &str) -> Step<Jumbling<'e>> {
    let ctx = engines.jumble.ok_or_else(|| reject("certificate", "the jumbling engine is absent"))?;
    let menu = read_menu(ctx, run, "menu")?;
    run.claim("menu.identity", &I_A_MENU_IDENTITY.to_owned(), &menu.identity)?;
    run.claim("menu.order", &I_A_ORDER_DIGEST.to_owned(), &menu.order)?;
    let jams = read_jams(run)?;
    let jam = jam_of_seed(&jams, seed, claim)?.clone();
    if run.optional_input("table").is_none() {
        return Err(start_mismatch("table", "the jam's table is absent"));
    }
    let table = read_state(ctx, run, &menu, "table")?;
    let mut j = Jumbling::new(ctx, menu, table);
    run.compare("table", &jam.table_digest, &j.digest(), codes::START_TABLE_MISMATCH)?;
    for (k, &(cap, item)) in jam.word().iter().enumerate() {
        let claim = at("jam.word", k);
        let twist = j.twist(cap, item, &claim)?;
        let outcome = j.apply(&twist, &claim)?;
        if outcome.status != Status::Admissible {
            return Err(run.failed(reject(&claim, format!("the letter is {}", outcome.status.as_str())), "refused"));
        }
    }
    run.held("jam.word", format!("{} letters admissible", jam.word().len()));
    let tables = tree_tables(ctx, "jam.end_phi")?;
    let end = j.digest();
    run.claim("jam.end_digest", &jam.end_digest, &end)?;
    let phi_now = phi(ctx, tables, &j.state, "jam.end_phi")?;
    run.claim("jam.end_phi", &jam.end_phi, &phi_now)?;
    Ok(j)
}

fn run(engines: &Engines, run: &mut Run) -> Step<()> {
    let doc = document(run.cert.bytes)?;
    keys(&doc, "record", &["seed", "trace", "word"], &[])?;
    let seed = string(field(&doc, "seed", "seed")?, "seed")?.to_owned();
    let word: Vec<Letter> = letters(field(&doc, "word", "word")?, "word")?;
    let events = array(field(&doc, "trace", "trace")?, "trace")?;
    if events.len() != word.len() + 1 {
        return Err(reject(
            "trace",
            format!("the trace has {} events for a word of {} letters", events.len(), word.len()),
        ));
    }
    let mut j = jam_state(engines, run, &seed, "seed")?;
    let ctx = j.ctx;
    let tables = tree_tables(ctx, "trace")?;
    let mut phi_now = phi(ctx, tables, &j.state, "trace")?;
    let mut named: Option<Vec<String>> = None;
    for (e, event) in events.iter().enumerate() {
        let path = at("trace", e);
        if e == 0 {
            keys(event, &path, &["event", "phi", "digest", "grips"], &[])?;
        } else {
            keys(event, &path, &["event", "letter", "phi", "delta", "digest", "support", "grips"], &[])?;
        }
        run.claim(&dot(&path, "event"), &int(field(event, "event", &path)?, &dot(&path, "event"))?, &(e as i64))?;
        if e > 0 {
            let (cap, item) = word[e - 1];
            let stated = letters(&Value::Array(vec![field(event, "letter", &path)?.clone()]), &dot(&path, "letter"))?;
            if stated != [(cap, item)] {
                return Err(reject(dot(&path, "letter"), format!("the letter is not the word's letter {cap}:{item}")));
            }
            let twist = j.twist(cap, item, &dot(&path, "letter"))?;
            let outcome = j.apply(&twist, &dot(&path, "letter"))?;
            if outcome.status != Status::Admissible {
                let stop = reject(dot(&path, "letter"), format!("the letter is {}", outcome.status.as_str()));
                return Err(run.failed(stop, "refused"));
            }
            run.held(dot(&path, "letter"), format!("{cap}:{item} admissible"));
            let support = int(field(event, "support", &path)?, &dot(&path, "support"))?;
            run.claim(&dot(&path, "support"), &support, &(outcome.inside.len() as i64))?;
            let after = phi(ctx, tables, &j.state, &dot(&path, "phi"))?;
            let delta = int(field(event, "delta", &path)?, &dot(&path, "delta"))?;
            run.claim(&dot(&path, "delta"), &delta, &(after - phi_now))?;
            phi_now = after;
        }
        run.claim(&dot(&path, "phi"), &int(field(event, "phi", &path)?, &dot(&path, "phi"))?, &phi_now)?;
        let digest = hex64(field(event, "digest", &path)?, &dot(&path, "digest"))?.to_owned();
        run.claim(&dot(&path, "digest"), &digest, &j.digest())?;
        let grips_path = dot(&path, "grips");
        let grips = object(field(event, "grips", &path)?, &grips_path)?;
        let names: Vec<String> = grips.keys().cloned().collect();
        match &named {
            None if names.is_empty() => return Err(reject(&grips_path, "the trace names no grip")),
            None => named = Some(names),
            Some(first) if *first != names => {
                return Err(reject(&grips_path, format!("the event names grips {names:?}, not the trace's {first:?}")))
            }
            Some(_) => {}
        }
        for (name, stated) in grips {
            let claim = dot(&grips_path, name);
            keys(stated, &claim, &["S", "F", "maxD"], &[])?;
            let x: usize = name.parse().map_err(|_| reject(&claim, "a grip is named by its number"))?;
            let (s, f, d) = sfd(&mut j, tables, x, &claim)?;
            run.claim(&dot(&claim, "S"), &int(field(stated, "S", &claim)?, &dot(&claim, "S"))?, &(s as i64))?;
            run.claim(&dot(&claim, "F"), &int(field(stated, "F", &claim)?, &dot(&claim, "F"))?, &(f as i64))?;
            run.claim(&dot(&claim, "maxD"), &int(field(stated, "maxD", &claim)?, &dot(&claim, "maxD"))?, &(d as i64))?;
        }
    }
    Ok(())
}
