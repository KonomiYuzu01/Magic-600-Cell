//! `s18.lattice_run`: stage 18's lattice run record and its self-contained members, without a replay (matrix family
//! 18/lattice_run: `run.json`, `lattice/report.json`, `start-table.json`, the 53 step trees and the two stored
//! tables; the 53 `ia16` words; LB5.13, LB5.15; CT-LB5-05, CT-LB5-09).
//!
//! The record (`runner-run/3`), in its exact form and before any input is read: the claims of
//! [`super::lattice::read_run`]. The inputs are exactly the members the record names.
//!
//! The members: the start table and the bulk phase's end table reproduce their own digests and the record's
//! (`KN6:evidence.start_table_mismatch` for the start); each word is admitted before decoding, decoded within its
//! declared size and hashed; each step tree has its recorded SHA-256 (of the decoded JSON) and node count, spells the
//! step's rule, and is a tree of the word: its leaves' spans tile the word in order, every node's length is its
//! parts' sum, an orbit tree has the step's factor count, a correction tree has `times` copies of the named macro, each
//! of the macro's length and with equal letters, and every part a tree names as the inverse of another holds that
//! part's letters reversed and inverted in the exact menu; a table the record stores at a step's end reproduces the
//! digest stated there. The report (`literal-lattice/1`) has its recorded SHA-256 and agrees with the record: its
//! input table and digest, the letters it scheduled and played (retained letters of the orbit steps and corrections),
//! its final digest, its step counts and the orbits' order.
//!
//! Not checked here: a state after any letter (that is `s18.lattice_replay`'s fresh replay), the labels the report
//! compares (`labels_sha256`, `labels_identity`, `junction`), the restorer's own outputs and the timings. The R22B-10
//! certificates behind stage 18's words are not in the archive; the record names their builders only by script digest.

use std::collections::BTreeMap;

use c600_jumble::nu::{I_A_MENU_IDENTITY, I_A_ORDER_DIGEST};
use c600_kernel::json::{self, Value};

use super::lattice::{null, only, open_run, read_run, word_is, Kind, LatticeStep, RunRecord};
use super::record::{inverses, invert};
use super::words::{decode, ia16_member};
use crate::archive::SubChecker;
use crate::claims::{array, at, count, dot, field, keys, object, reject, start_mismatch, string, Run, Step};
use crate::kinds::{Checker, Needs};
use crate::replay::{read_menu, read_state};
use crate::report::{codes, Sources, Span};
use crate::Engines;

pub static SOURCES: Sources = Sources::new(
    "s18.lattice_run",
    &[
        ("c600-checkers/Cargo.toml", include_bytes!("../../../Cargo.toml")),
        ("c600-checkers/src/archive/s18/lattice.rs", include_bytes!("lattice.rs")),
        ("c600-checkers/src/archive/s18/lattice_run.rs", include_bytes!("lattice_run.rs")),
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
    family: "18/lattice_run: the lattice-d1 run record, report, tables, step trees and flat words (LB5.13, LB5.15)",
};

const SCOPE: &str = "a runner-run/3 lattice run record and its self-contained members";

fn run(engines: &Engines, run: &mut Run) -> Step<()> {
    let ctx = engines.jumble.ok_or_else(|| reject("certificate", "the jumbling engine is absent"))?;
    let doc = open_run(run, SCOPE)?;
    let record = read_run(run, &doc)?;

    // The inputs: exactly the members the record names.
    let mut named: Vec<&str> = vec!["menu", &record.start_file, &record.bulk_table, &record.report_file];
    for step in &record.steps {
        named.push(&step.word);
        named.push(&step.tree.file);
        named.extend(step.table.as_deref());
    }
    only(run, &named)?;

    let menu = read_menu(ctx, run, "menu")?;
    run.claim("menu.identity", &I_A_MENU_IDENTITY.to_owned(), &menu.identity)?;
    run.claim("menu.order", &I_A_ORDER_DIGEST.to_owned(), &menu.order)?;
    let inverse = inverses(&menu)?;
    if run.optional_input(&record.start_file).is_none() {
        return Err(start_mismatch("start", "the run's start table is absent"));
    }
    let state = read_state(ctx, run, &menu, &record.start_file)?;
    run.compare("start.digest", &record.start.digest, &state.digest(ctx), codes::START_TABLE_MISMATCH)?;
    run.claim("start.off", &record.start.off, &(state.off_lattice_count() as u64))?;
    drop(state);
    let state = read_state(ctx, run, &menu, &record.bulk_table)?;
    run.claim("phases.bulk.end.table", &record.start.digest, &state.digest(ctx))?;
    drop(state);

    check_report(run, &record, &record.report_file, &record.report_sha)?;

    for step in &record.steps {
        let path = &step.path;
        let bytes = run.input(&step.word)?;
        let (word, digest) = ia16_member(&step.word, bytes, step.letters, path)?;
        run.bind(&step.word, bytes, vec![("decoded_sha256", digest.clone())]);
        run.claim(&dot(path, "word_sha256"), &step.sha, &digest)?;
        run.held_span(
            dot(path, "word"),
            format!("{} letters streamed and hashed", step.letters),
            Span { word: digest.clone(), from: 0, to: step.letters },
        );
        check_tree(run, step, &word, &inverse)?;
        if let Some(table) = &step.table {
            let state = read_state(ctx, run, &menu, table)?;
            run.claim(&dot(path, "table"), &step.after.digest, &state.digest(ctx))?;
        }
    }
    run.held(
        "scope",
        "record, report, tables, trees and words; no state after a letter (s18.lattice_replay), no labels \
         (labels_sha256, labels_identity, junction), no restorer outputs, configuration or timings",
    );
    Ok(())
}

/// The lattice phase's report against the record.
fn check_report(run: &mut Run, record: &RunRecord, file: &str, stated: &str) -> Step<()> {
    let bytes = run.input(file)?;
    let digest = c600_digest::sha256_hex(bytes);
    run.bind(file, bytes, vec![("sha256", digest.clone())]);
    run.claim("phases.lattice.report.sha256", &stated.to_owned(), &digest)?;
    let report = json::parse(bytes).map_err(|error| reject("report", format!("the report is not JSON: {error}")))?;
    let player = field(&report, "player", "report")?;
    word_is(run, player, "format", "report.player", "literal-lattice/1")?;
    word_is(run, player, "result", "report.player", "solved")?;
    null(player, "failure", "report.player")?;
    let input = field(player, "input", "report.player")?;
    word_is(run, input, "table", "report.player.input", &record.start_file)?;
    word_is(run, input, "digest", "report.player.input", &record.start.digest)?;
    let (mut retained, mut corrections, mut orbits) = (0u64, 0u64, Vec::new());
    for step in &record.steps {
        match &step.kind {
            Kind::Orbit { orbit, .. } => {
                retained += step.letters;
                orbits.push(*orbit);
            }
            Kind::Correction { .. } => corrections += step.letters,
        }
    }
    for part in ["scheduled", "played"] {
        let value = field(player, part, "report.player")?;
        let path = dot("report.player", part);
        run.claim(&dot(&path, "retained"), &count(field(value, "retained", &path)?, &path)?, &retained)?;
        run.claim(&dot(&path, "corrections"), &count(field(value, "corrections", &path)?, &path)?, &corrections)?;
    }
    let played = field(player, "played", "report.player")?;
    run.claim(
        "report.player.played.total",
        &count(field(played, "total", "report.player.played")?, "total")?,
        &record.twists,
    )?;
    let last = &record.steps.last().expect("a run has steps").after.digest;
    word_is(run, field(player, "final", "report.player")?, "digest", "report.player.final", last)?;
    let counts = field(&report, "step_counts", "report")?;
    let orbit_steps = orbits.len() as u64;
    run.claim(
        "report.step_counts.orbit",
        &count(field(counts, "orbit", "report.step_counts")?, "orbit")?,
        &orbit_steps,
    )?;
    let correction_steps = record.steps.len() as u64 - orbit_steps;
    let stated = count(field(counts, "correction", "report.step_counts")?, "correction")?;
    run.claim("report.step_counts.correction", &stated, &correction_steps)?;
    let order: Vec<u64> = array(field(&report, "cascade_order", "report")?, "report.cascade_order")?
        .iter()
        .map(|v| count(v, "report.cascade_order"))
        .collect::<Step<_>>()?;
    if order != orbits {
        return Err(reject("report.cascade_order", "the report's orbit order is not the run's"));
    }
    if !array(field(&report, "skipped_orbits", "report")?, "report.skipped_orbits")?.is_empty() {
        return Err(reject("report.skipped_orbits", "the run skipped no orbit"));
    }
    for key in ["tree_folded", "tree_folded_depth"] {
        let path = dot("report", key);
        run.claim(&path, &count(field(&report, key, "report")?, &path)?, &0u64)?;
    }
    run.held("report", "input, scheduled and played letters, final digest, step counts and orbit order agree");
    Ok(())
}

/// A node's extent in the word and its letters.
type Extents = BTreeMap<Vec<usize>, (u64, u64)>;

/// One step's tree against its record and word.
fn check_tree(run: &mut Run, step: &LatticeStep, word: &[u8], inverse: &[usize]) -> Step<()> {
    let path = dot(&step.path, "tree");
    let (file, size) = (step.tree.file.as_str(), step.tree.size);
    let bytes = run.input(file)?;
    let decoded = decode(file, bytes, 1 << 26, &path)?;
    let digest = c600_digest::sha256_hex(&decoded);
    run.bind(file, bytes, vec![("decoded_sha256", digest.clone())]);
    run.claim(&dot(&path, "sha256"), &step.tree.sha, &digest)?;
    let tree = json::parse(&decoded).map_err(|error| reject(&path, format!("the tree is not JSON: {error}")))?;
    drop(decoded);
    let mut extents = Extents::new();
    let mut links = Links::default();
    let end = walk(&tree, &path, Vec::new(), 0, &mut extents, &mut links)?;
    run.claim(&dot(&path, "size"), &size, &(extents.len() as u64))?;
    run.claim(&dot(&step.path, "letters"), &step.letters, &end)?;
    let root = object(&tree, &path)?;
    let name = root.get("name").and_then(Value::as_str).unwrap_or_default().to_owned();
    let parts = root.get("parts").and_then(Value::as_array).unwrap_or_default();
    match &step.kind {
        Kind::Orbit { orbit, factors } => {
            run.claim(&dot(&path, "name"), &format!("orbit {orbit}"), &name)?;
            run.claim(&dot(&step.path, "factor_count"), factors, &(parts.len() as u64))?;
        }
        Kind::Correction { correction, name: macro_name, times, macro_letters } => {
            run.claim(&dot(&path, "name"), &format!("correction {correction}"), &name)?;
            run.claim(&dot(&step.path, "times"), times, &(parts.len() as u64))?;
            for (n, part) in parts.iter().enumerate() {
                let claim = at(&dot(&path, "parts"), n);
                let found = part.get("name").and_then(Value::as_str).unwrap_or_default().to_owned();
                run.claim(&dot(&claim, "name"), macro_name, &found)?;
                let (from, to) = extents[&vec![n]];
                run.claim(&dot(&claim, "letters"), macro_letters, &(to - from))?;
                let (first, _) = extents[&vec![0]];
                if word[2 * from as usize..2 * to as usize]
                    != word[2 * first as usize..2 * (first + to - from) as usize]
                {
                    return Err(reject(&claim, "a copy of the macro differs from the first"));
                }
            }
        }
    }
    let code = |position: u64| u16::from_le_bytes([word[2 * position as usize], word[2 * position as usize + 1]]);
    for (node, target) in &links.inverses_of {
        let claim = format!("{path}.inverse_of{node:?}");
        let (from, to) = extents[node];
        let (t_from, t_to) =
            *extents.get(target).ok_or_else(|| reject(&claim, "the inverse names no part of the tree"))?;
        if to - from != t_to - t_from {
            return Err(reject(&claim, "a part and its inverse differ in length"));
        }
        for i in 0..(to - from) {
            if code(from + i) != invert(code(t_to - 1 - i), inverse) {
                return Err(reject(&claim, format!("letter {} is not the inverse of its partner", from + i)));
            }
        }
    }
    for (node, target) in &links.same_as {
        let claim = format!("{path}.same_as{node:?}");
        let (from, to) = extents[node];
        let (t_from, t_to) =
            *extents.get(target).ok_or_else(|| reject(&claim, "the repeat names no part of the tree"))?;
        if to - from != t_to - t_from {
            return Err(reject(&claim, "a part and the part it repeats differ in length"));
        }
        for i in 0..(to - from) {
            if code(from + i) != code(t_from + i) {
                return Err(reject(&claim, format!("letter {} differs from the part it repeats", from + i)));
            }
        }
    }
    run.held_span(
        &path,
        format!(
            "{} nodes; spans tile the word; {} inverse parts hold their partners' inverses; {} repeated parts equal the parts they name",
            extents.len(),
            links.inverses_of.len(),
            links.same_as.len()
        ),
        Span { word: step.sha.clone(), from: 0, to: step.letters },
    );
    Ok(())
}

/// The tree's links between nodes, each (node path, named path) from the root.
#[derive(Default)]
struct Links {
    /// Parts whose letters are the inverse of the named part's.
    inverses_of: Vec<(Vec<usize>, Vec<usize>)>,
    /// Parts whose letters repeat the named part's.
    same_as: Vec<(Vec<usize>, Vec<usize>)>,
}

/// A link's node path.
fn link(value: &Value, claim: &str) -> Step<Vec<usize>> {
    array(value, claim)?.iter().map(|v| count(v, claim).map(|n| n as usize)).collect()
}

/// Walks a node from position `from`: a leaf `{name, letters, span}` or a node `{name, letters, parts}`, with an
/// optional `inverse_of` or `same_as` path. Returns the position after it.
fn walk(node: &Value, claim: &str, path: Vec<usize>, from: u64, extents: &mut Extents, links: &mut Links) -> Step<u64> {
    let map = object(node, claim)?;
    string(field(node, "name", claim)?, &dot(claim, "name"))?;
    let letters = count(field(node, "letters", claim)?, &dot(claim, "letters"))?;
    let to = match (map.get("span"), map.get("parts")) {
        (Some(span), None) => {
            keys(node, claim, &["name", "letters", "span"], &["inverse_of", "same_as"])?;
            let span = array(span, &dot(claim, "span"))?;
            let bounds: Vec<u64> = span.iter().map(|v| count(v, &dot(claim, "span"))).collect::<Step<_>>()?;
            if bounds.len() != 2 || bounds[0] != from || bounds[1] < bounds[0] || bounds[1] - bounds[0] != letters {
                return Err(reject(dot(claim, "span"), "the leaves do not tile the word in order"));
            }
            bounds[1]
        }
        (None, Some(parts)) => {
            keys(node, claim, &["name", "letters", "parts"], &["inverse_of", "same_as"])?;
            let mut at_ = from;
            for (n, part) in array(parts, &dot(claim, "parts"))?.iter().enumerate() {
                let mut child = path.clone();
                child.push(n);
                at_ = walk(part, &at(&dot(claim, "parts"), n), child, at_, extents, links)?;
            }
            if at_ - from != letters {
                return Err(reject(dot(claim, "letters"), "a node's length is not its parts' sum"));
            }
            at_
        }
        _ => return Err(reject(claim, "a node has a span or parts")),
    };
    if map.contains_key("inverse_of") && map.contains_key("same_as") {
        return Err(reject(claim, "a part is an inverse or a repeat, not both"));
    }
    if let Some(target) = map.get("inverse_of") {
        links.inverses_of.push((path.clone(), link(target, &dot(claim, "inverse_of"))?));
    }
    if let Some(target) = map.get("same_as") {
        links.same_as.push((path.clone(), link(target, &dot(claim, "same_as"))?));
    }
    extents.insert(path, (from, to));
    Ok(to)
}
