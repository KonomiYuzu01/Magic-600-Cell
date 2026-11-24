//! ν and label checkpoints along a word (`kn6-checkpoints/1`), on either puzzle.
//!
//! A certificate names a start state and a word, and at strictly increasing positions claims what the state after that
//! many letters is. The word is replayed letter by letter from the start and each checkpoint's claims are recomputed
//! from the state at its position:
//!
//! - 600-cell-Full: `labels`, `Digest<labels>` of the state (0.4's state hash);
//! - the jumbling puzzle (every letter admissible): `digest`, the state's canonical digest; optionally `lattice`, the
//!   SHA-256 of one lattice flag per piece; `labels`, `Digest<labels>` of π(X), or `null` where π is partial; and `nu`,
//!   ν's 45 coordinates where ν is total, or the word `off_lattice` or `pieces_collide` where it is undefined (A5).
//!
//! A checkpoint's trace step cites its span of the word by the word's digest and positions (KN6.4).
//!
//! ```text
//! { "format": "kn6-checkpoints/1",
//!   "puzzle": "600-cell-Full" | "jumbling",
//!   "inputs": {"start": sha256, "menu": sha256, "word": sha256},   (start optional; menu for the jumbling puzzle)
//!   "menu": {"identity": ..., "order": ...},                       (jumbling puzzle)
//!   "start": {"digest": ...},
//!   "word": [...] | {"input": "word", "encoding": ..., "letters": n, "sha256": ...},
//!   "checkpoints": [{"at": k, "labels": ...}, {"at": k, "digest": ..., "lattice": ..., "labels": ..., "nu": ...}] }
//! ```

use c600_jumble::nu;
use c600_jumble::state::Status;
use c600_jumble::{levels, levels::Partial};
use c600_kernel::json::Value;

use super::{Checker, Needs};
use crate::claims::{array, at, count, document, dot, field, hex64, keys, reject, require_format, string, Run, Step};
use crate::replay::{read_full_state, read_menu, read_state, read_word, Full, Jumbling, Word};
use crate::report::{codes, Sources, Span};
use crate::Engines;

pub const FORMAT: &str = "kn6-checkpoints/1";

pub static SOURCES: Sources = Sources::new(
    "kn6.checkpoints/1",
    &[
        ("c600-checkers/Cargo.toml", include_bytes!("../../Cargo.toml")),
        ("c600-checkers/src/claims.rs", include_bytes!("../claims.rs")),
        ("c600-checkers/src/kinds/checkpoints.rs", include_bytes!("checkpoints.rs")),
        ("c600-checkers/src/kinds/mod.rs", include_bytes!("mod.rs")),
        ("c600-checkers/src/lib.rs", include_bytes!("../lib.rs")),
        ("c600-checkers/src/replay.rs", include_bytes!("../replay.rs")),
        ("c600-checkers/src/report.rs", include_bytes!("../report.rs")),
    ],
);

pub static CHECKER: Checker = Checker { sources: &SOURCES, needs: Needs::BOTH, run };

/// The stated checkpoints: position and its claims object. The last checkpoint is at the word's end, so every letter of
/// the word is replayed and lies under a claim.
fn positions(doc: &Value, length: usize) -> Step<Vec<(u64, &Value)>> {
    let mut out: Vec<(u64, &Value)> = Vec::new();
    for (k, entry) in array(field(doc, "checkpoints", "checkpoints")?, "checkpoints")?.iter().enumerate() {
        let path = at("checkpoints", k);
        let position = count(field(entry, "at", &path)?, &dot(&path, "at"))?;
        if position > length as u64 || out.last().is_some_and(|(previous, _)| *previous >= position) {
            return Err(reject(dot(&path, "at"), "checkpoints are strictly increasing positions within the word"));
        }
        out.push((position, entry));
    }
    if out.last().map(|(position, _)| *position) != Some(length as u64) {
        return Err(reject("checkpoints", "the last checkpoint is at the word's end"));
    }
    Ok(out)
}

fn run(engines: &Engines, run: &mut Run) -> Step<()> {
    let doc = document(run.cert.bytes)?;
    require_format(&doc, FORMAT)?;
    let puzzle = string(field(&doc, "puzzle", "puzzle")?, "puzzle")?;
    match puzzle {
        "600-cell-Full" => {
            keys(&doc, "certificate", &["format", "puzzle", "inputs", "start", "word", "checkpoints"], &[])?;
            full(engines, run, &doc)
        }
        "jumbling" => {
            keys(&doc, "certificate", &["format", "puzzle", "inputs", "menu", "start", "word", "checkpoints"], &[])?;
            jumbling(engines, run, &doc)
        }
        _ => Err(reject("puzzle", "the puzzle is 600-cell-Full or jumbling")),
    }
}

fn start_digest(run: &mut Run, doc: &Value, computed: &str) -> Step<()> {
    let start = field(doc, "start", "start")?;
    keys(start, "start", &["digest"], &[])?;
    let stated = hex64(field(start, "digest", "start")?, "start.digest")?.to_owned();
    run.compare("start.digest", &stated, &computed.to_owned(), codes::START_TABLE_MISMATCH)
}

fn full(engines: &Engines, run: &mut Run, doc: &Value) -> Step<()> {
    let kernel = engines.kernel.ok_or_else(|| reject("puzzle", "the 600-cell-Full kernel is absent"))?;
    run.check_bindings(doc, &["start"])?;
    let state = read_full_state(kernel, run, "start")?;
    start_digest(run, doc, state.digest())?;
    let Word::Full(letters) = read_word(run, field(doc, "word", "word")?, "word", None)? else {
        unreachable!("a 600-cell-Full word")
    };
    let word = Word::Full(letters.clone()).digest();
    let mut replay = Full { kernel, state };
    let mut done = 0u64;
    for (k, (position, entry)) in positions(doc, letters.len())?.into_iter().enumerate() {
        let path = at("checkpoints", k);
        keys(entry, &path, &["at", "labels"], &[])?;
        replay.advance(&letters[done as usize..position as usize], &path)?;
        let claim = dot(&path, "labels");
        let stated = hex64(field(entry, "labels", &path)?, &claim)?.to_owned();
        let computed = replay.state.digest().to_owned();
        if stated != computed {
            return Err(run.failed(reject(&claim, format!("stated {stated}, computed {computed}")), computed));
        }
        run.held_span(claim, computed, Span { word: word.clone(), from: done, to: position });
        done = position;
    }
    Ok(())
}

fn jumbling(engines: &Engines, run: &mut Run, doc: &Value) -> Step<()> {
    let ctx = engines.jumble.ok_or_else(|| reject("puzzle", "the jumbling engine is absent"))?;
    run.check_bindings(doc, &["start"])?;
    let menu = read_menu(ctx, run, "menu")?;
    let stated_menu = field(doc, "menu", "menu")?;
    keys(stated_menu, "menu", &["identity", "order"], &[])?;
    run.claim(
        "menu.identity",
        &hex64(field(stated_menu, "identity", "menu")?, "menu.identity")?.to_owned(),
        &menu.identity,
    )?;
    run.claim("menu.order", &hex64(field(stated_menu, "order", "menu")?, "menu.order")?.to_owned(), &menu.order)?;
    let state = read_state(ctx, run, &menu, "start")?;
    let mut j = Jumbling::new(ctx, menu.clone(), state);
    start_digest(run, doc, &j.digest())?;
    let word = read_word(run, field(doc, "word", "word")?, "word", Some(menu.items.len()))?;
    let Word::Jumbling { codes, items } = &word else { unreachable!("a jumbling word") };
    let digest = word.digest();
    let mut done = 0u64;
    for (k, (position, entry)) in positions(doc, codes.len())?.into_iter().enumerate() {
        let path = at("checkpoints", k);
        keys(entry, &path, &["at", "digest"], &["lattice", "labels", "nu"])?;
        for p in done..position {
            let code = codes[p as usize] as usize;
            let claim = at("word", p as usize);
            let twist = j.twist(code / items, code % items, &claim)?;
            let outcome = j.apply(&twist, &claim)?;
            if outcome.status != Status::Admissible {
                let stop = reject(&claim, format!("the letter at position {p} is {}", outcome.status.as_str()));
                return Err(run.failed(stop, outcome.status.as_str()));
            }
        }
        let span = Span { word: digest.clone(), from: done, to: position };
        let claim = dot(&path, "digest");
        let stated = hex64(field(entry, "digest", &path)?, &claim)?.to_owned();
        let computed = j.digest();
        if stated != computed {
            return Err(run.failed(reject(&claim, format!("stated {stated}, computed {computed}")), computed));
        }
        run.held_span(claim, computed, span.clone());
        if let Some(value) = entry.get("lattice") {
            let claim = dot(&path, "lattice");
            run.claim(&claim, &hex64(value, &claim)?.to_owned(), &j.state.lattice_digest())?;
        }
        if let Some(value) = entry.get("labels") {
            let claim = dot(&path, "labels");
            let computed = match levels::labels(ctx, &j.state)
                .map_err(|error| reject(&claim, format!("the engine stopped: {}", error.0)))?
            {
                Partial::Total(labels) => Some(labels.digest()),
                _ => None,
            };
            let stated = if matches!(value, Value::Null) { None } else { Some(hex64(value, &claim)?.to_owned()) };
            let show = |v: &Option<String>| v.clone().unwrap_or_else(|| "null".into());
            run.claim(&claim, &show(&stated), &show(&computed))?;
        }
        if let Some(value) = entry.get("nu") {
            let claim = dot(&path, "nu");
            let computed = match nu::value(ctx, &j.state)
                .map_err(|error| reject(&claim, format!("the engine stopped: {}", error.0)))?
            {
                Partial::Total(vector) => vector.values().iter().map(i64::to_string).collect::<Vec<_>>().join(","),
                Partial::Undefined(reason) => reason,
                Partial::Partial { .. } => "partial".to_owned(),
            };
            let stated = match value {
                Value::String(reason) => reason.clone(),
                Value::Array(values) => {
                    let values = values
                        .iter()
                        .enumerate()
                        .map(|(i, v)| crate::claims::int(v, &at(&claim, i)).map(|n| n.to_string()))
                        .collect::<Step<Vec<_>>>()?;
                    values.join(",")
                }
                _ => return Err(reject(&claim, "ν is a list of 45 coordinates or the reason it is undefined")),
            };
            run.claim(&claim, &stated, &computed)?;
        }
        done = position;
    }
    Ok(())
}
