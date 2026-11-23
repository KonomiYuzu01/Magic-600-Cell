//! `s16.mobility`: stage 16's unit mobility records (`results/mobility/730N.json`; README section 2.3), the relations
//! of the high units at each jam, recomputed from the exact state.
//!
//! A record is formatless: `{"seed", "digest", "summary", "units"}`. Inputs: `jams`, `table` (the seed's D2 end
//! table) and `menu` (I_a). The jam is replayed from its exact table ([`super::release::jam_state`]), `digest` is its
//! digest, and the full label table is classified exactly ([`crate::archive::s15::labels`]). Then:
//!
//! - the high units are the pose groups of left height at least 4; each is named by its least piece (`piece`), with
//!   its `size`, `height` and branch (`cls`); the record's units are exactly these, in any order (`pose`, the
//!   producer's own pose-table index, is not a claim: the engine's canonical table numbers poses its own way);
//! - `whole_holders`: the grips whose caps hold every member; `clean_whole`: those that are clean; `clean_partial`:
//!   the clean grips whose caps hold some members but not all; `whole_levels`: the distinct levels of the whole
//!   holders, ascending, with stage 14's greatest-fixed-point levels over the whole table;
//! - `own`: the whole holders of the unit's own class, each with `clean`, `level`, `S` (straddlers), `F` (foreign
//!   load) and `maxD` (the largest lowering count over the jumbles at the grip);
//! - `summary`: the number of units and of high pieces, the frozen units (no clean grip holds any member) and the
//!   units carried whole (some clean grip holds every member), each as their least pieces and their piece count.
//!
//! The observed relations hold for this state only; they are a finite record, not a theorem about other states.

use std::collections::{BTreeMap, BTreeSet};

use c600_kernel::json::Value;

use super::release::jam_state;
use crate::archive::s15::grips::{class, foreign, max_lowering};
use crate::archive::s15::labels::{levels, LabelTable, INF};
use crate::archive::SubChecker;
use crate::claims::{array, at, document, dot, field, hex64, int, keys, reject, string, Run, Step};
use crate::kinds::{Checker, Needs};
use crate::replay::{left_facts, tree_tables};
use crate::report::Sources;
use crate::Engines;

pub static SOURCES: Sources = Sources::new(
    "s16.mobility",
    &[
        ("c600-checkers/Cargo.toml", include_bytes!("../../../Cargo.toml")),
        ("c600-checkers/src/archive/s15/grips.rs", include_bytes!("../s15/grips.rs")),
        ("c600-checkers/src/archive/s15/jam.rs", include_bytes!("../s15/jam.rs")),
        ("c600-checkers/src/archive/s15/labels.rs", include_bytes!("../s15/labels.rs")),
        ("c600-checkers/src/archive/s16/mobility.rs", include_bytes!("mobility.rs")),
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
    family: "S16 unit mobility at the jams (relations): high units, holders, clean holders and levels",
};

/// One high unit as computed.
struct Unit {
    size: usize,
    height: i64,
    cls: i64,
    whole: Vec<usize>,
    clean_whole: Vec<usize>,
    clean_partial: Vec<usize>,
}

fn ints<T: Copy + Into<i64>>(values: &[T]) -> String {
    Value::Array(values.iter().map(|&v| Value::Int(v.into())).collect()).canonical()
}

fn level_value(level: i64) -> Value {
    if level >= INF {
        Value::String("infinity".to_owned())
    } else {
        Value::Int(level)
    }
}

fn run(engines: &Engines, run: &mut Run) -> Step<()> {
    let doc = document(run.cert.bytes)?;
    keys(&doc, "record", &["digest", "seed", "summary", "units"], &[])?;
    let seed = string(field(&doc, "seed", "seed")?, "seed")?.to_owned();
    let j = jam_state(engines, run, &seed, "seed")?;
    let ctx = j.ctx;
    run.claim("digest", &hex64(field(&doc, "digest", "digest")?, "digest")?.to_owned(), &j.digest())?;
    let tables = tree_tables(ctx, "units")?;

    // The high units: pose groups of left height at least 4.
    let mut members: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    let mut facts: BTreeMap<u32, (i64, i64)> = BTreeMap::new();
    for (p, &pid) in j.state.pose_ids().iter().enumerate() {
        let h = match facts.get(&pid) {
            Some(&(h, _)) => h,
            None => {
                let f = left_facts(ctx, tables, &j.state.poses()[pid as usize], "units")?;
                facts.insert(pid, f);
                f.0
            }
        };
        if h >= 4 {
            members.entry(pid).or_default().push(p as u32);
        }
    }
    let table = LabelTable::build(ctx, &j.state, "units")?;
    let lev = levels(&table);
    let mut unit_of: BTreeMap<u32, u32> = BTreeMap::new();
    for (pid, list) in &members {
        for &p in list {
            unit_of.insert(p, *pid);
        }
    }
    let mut held: BTreeMap<u32, Vec<(usize, usize)>> = BTreeMap::new();
    for (x, list) in table.inside.iter().enumerate() {
        let mut count: BTreeMap<u32, usize> = BTreeMap::new();
        for p in list {
            if let Some(&pid) = unit_of.get(p) {
                *count.entry(pid).or_default() += 1;
            }
        }
        for (pid, n) in count {
            held.entry(pid).or_default().push((x, n));
        }
    }
    let mut units: BTreeMap<u32, Unit> = BTreeMap::new();
    for (pid, list) in &members {
        let (height, cls) = facts[pid];
        let holders = held.get(pid).map(Vec::as_slice).unwrap_or(&[]);
        let whole: Vec<usize> = holders.iter().filter(|(_, n)| *n == list.len()).map(|(x, _)| *x).collect();
        let clean_whole = whole.iter().copied().filter(|&x| table.clean(x)).collect();
        let clean_partial =
            holders.iter().filter(|(x, n)| *n < list.len() && table.clean(*x)).map(|(x, _)| *x).collect();
        units.insert(list[0], Unit { size: list.len(), height, cls, whole, clean_whole, clean_partial });
    }

    // The record's units, matched by their least piece.
    let stated = array(field(&doc, "units", "units")?, "units")?;
    run.claim("units", &stated.len(), &units.len())?;
    let mut seen = BTreeSet::new();
    for (u, row) in stated.iter().enumerate() {
        let path = at("units", u);
        keys(
            row,
            &path,
            &[
                "pose",
                "size",
                "height",
                "cls",
                "piece",
                "whole_holders",
                "clean_whole",
                "own",
                "clean_partial",
                "whole_levels",
            ],
            &[],
        )?;
        let piece = int(field(row, "piece", &path)?, &dot(&path, "piece"))?;
        let unit = u32::try_from(piece)
            .ok()
            .and_then(|p| units.get(&p))
            .ok_or_else(|| reject(dot(&path, "piece"), format!("{piece} is the least piece of no high unit")))?;
        if !seen.insert(piece) {
            return Err(reject(dot(&path, "piece"), "the unit is listed twice"));
        }
        run.held(dot(&path, "piece"), piece);
        run.claim(&dot(&path, "size"), &int(field(row, "size", &path)?, &dot(&path, "size"))?, &(unit.size as i64))?;
        run.claim(&dot(&path, "height"), &int(field(row, "height", &path)?, &dot(&path, "height"))?, &unit.height)?;
        run.claim(&dot(&path, "cls"), &int(field(row, "cls", &path)?, &dot(&path, "cls"))?, &unit.cls)?;
        let as_i64 = |v: &[usize]| v.iter().map(|&x| x as i64).collect::<Vec<i64>>();
        for (name, computed) in
            [("whole_holders", &unit.whole), ("clean_whole", &unit.clean_whole), ("clean_partial", &unit.clean_partial)]
        {
            run.claim(&dot(&path, name), &field(row, name, &path)?.canonical(), &ints(&as_i64(computed)))?;
        }
        let whole_levels: BTreeSet<i64> = unit.whole.iter().map(|&x| lev[x]).collect();
        let whole_levels = Value::Array(whole_levels.into_iter().map(level_value).collect()).canonical();
        run.claim(&dot(&path, "whole_levels"), &field(row, "whole_levels", &path)?.canonical(), &whole_levels)?;
        let own_path = dot(&path, "own");
        let mut own: Vec<usize> = Vec::new();
        for &x in &unit.whole {
            if class(ctx, tables, x, &own_path)? == unit.cls {
                own.push(x);
            }
        }
        let rows = array(field(row, "own", &path)?, &own_path)?;
        run.claim(&own_path, &rows.len(), &own.len())?;
        for (k, (holder, &x)) in rows.iter().zip(&own).enumerate() {
            let claim = at(&own_path, k);
            keys(holder, &claim, &["grip", "clean", "level", "S", "F", "maxD"], &[])?;
            run.claim(&dot(&claim, "grip"), &int(field(holder, "grip", &claim)?, &dot(&claim, "grip"))?, &(x as i64))?;
            let clean = field(holder, "clean", &claim)?.canonical();
            run.claim(&dot(&claim, "clean"), &clean, &Value::Bool(table.clean(x)).canonical())?;
            let level = field(holder, "level", &claim)?.canonical();
            run.claim(&dot(&claim, "level"), &level, &level_value(lev[x]).canonical())?;
            let s = table.straddle[x].len() as i64;
            run.claim(&dot(&claim, "S"), &int(field(holder, "S", &claim)?, &dot(&claim, "S"))?, &s)?;
            let f = foreign(ctx, tables, &j.state, x, &table.inside[x], &claim)? as i64;
            run.claim(&dot(&claim, "F"), &int(field(holder, "F", &claim)?, &dot(&claim, "F"))?, &f)?;
            let d = max_lowering(&j, tables, x, &table.inside[x], &claim)? as i64;
            run.claim(&dot(&claim, "maxD"), &int(field(holder, "maxD", &claim)?, &dot(&claim, "maxD"))?, &d)?;
        }
    }

    // The summary.
    let summary = field(&doc, "summary", "summary")?;
    keys(
        summary,
        "summary",
        &["units", "high_pieces", "frozen", "frozen_pieces", "carried_whole", "carried_whole_pieces"],
        &[],
    )?;
    let high: usize = units.values().map(|u| u.size).sum();
    run.claim("summary.units", &int(field(summary, "units", "summary")?, "summary.units")?, &(units.len() as i64))?;
    run.claim(
        "summary.high_pieces",
        &int(field(summary, "high_pieces", "summary")?, "summary.high_pieces")?,
        &(high as i64),
    )?;
    let frozen: Vec<(u32, usize)> = units
        .iter()
        .filter(|(_, u)| u.clean_whole.is_empty() && u.clean_partial.is_empty())
        .map(|(p, u)| (*p, u.size))
        .collect();
    let carried: Vec<(u32, usize)> =
        units.iter().filter(|(_, u)| !u.clean_whole.is_empty()).map(|(p, u)| (*p, u.size)).collect();
    for (name, pieces_name, list) in
        [("frozen", "frozen_pieces", &frozen), ("carried_whole", "carried_whole_pieces", &carried)]
    {
        let claim = dot("summary", name);
        let mut stated: Vec<i64> =
            array(field(summary, name, "summary")?, &claim)?.iter().map(|v| int(v, &claim)).collect::<Step<_>>()?;
        stated.sort_unstable();
        let computed: Vec<i64> = list.iter().map(|(p, _)| *p as i64).collect();
        run.claim(&claim, &ints(&stated), &ints(&computed))?;
        let count: usize = list.iter().map(|(_, n)| n).sum();
        let claim = dot("summary", pieces_name);
        run.claim(&claim, &int(field(summary, pieces_name, "summary")?, &claim)?, &(count as i64))?;
    }
    Ok(())
}
