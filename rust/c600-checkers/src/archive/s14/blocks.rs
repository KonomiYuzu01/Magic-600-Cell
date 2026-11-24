//! Stage 14's block and holder records at a stored table (part of LB5.15's second stage 14 family, "holder/co-held
//! facts"; the claim matrix's `14/relations` groups of the formatless `s7304-coheld.json`, `s7304-end-caps.json`,
//! `s7304-jam.json` and `s7304-moves-522.json`), each recomputed from the table and the model.
//!
//! Every record states its table's digest (`digest`), which the stated table must reproduce (KN6.5). The labels of every
//! piece off the lattice are recomputed at all 600 grips, the heights and branches from the exact tree facts, and the
//! terms are README section 1's: a grip is clean when no piece straddles its cut, and pure when it is clean and holds no
//! piece of height at least 2 whose branch is not its class; the holders of a set of pieces are the grips whose caps
//! hold every one; a unit is a set of pieces with one pose.
//!
//! - Co-held foreign pieces (Lemma C), one row per high unit (pose of height at least 4, in pose order): its holders
//!   and own holders, the number of distinct holder rows of its members, the pieces of another branch at height at
//!   least 2 whose holder set equals the unit's holder set (counted as co-held only when every member has that same
//!   set, as Lemma C requires per member) and those whose holder set contains it, with their heights.
//! - Per-grip rows: the straddlers by height and by own or other branch; for a clean grip what its cap holds (lattice
//!   pieces, and own- and other-branch pieces by height, a key for every height and branch the state has) and whether
//!   it is pure; for a blocked grip its covers by one clean cap (the first 20), whether every straddler has a clean
//!   holder, and the retained twists (c, k) at clean caps that free it (the first 20), over caps c in ascending order
//!   and k in A4_c in the engine's order. Each candidate twist is applied on the engine to a copy of the state, must be
//!   admissible and move exactly its cap's pieces, and frees d when no piece it moved straddles d at its new pose,
//!   evaluated exactly; only a cap that holds every straddler of d can free it, since a twist moves no other piece.
//!   Corollary L1's count is not used, so a wrong lemma would show as a mismatch.
//! - The anatomy of target grips (the own holders of the high units, in unit order, or the record's named target):
//!   its straddlers and its foreign pieces in units with their cells, holders by kind (clean pure own class, clean
//!   impure own class, clean other class, blocked) and the foreign load by height and branch. With `moves`, every
//!   non-identity I_a letter at every clean holder of those units, applied exactly to a copy of the state: whether it
//!   is admitted, the pieces its cap holds, Φ's change, the largest height, the off count, the straddlers and foreign
//!   load of the target after it, and the units (by pose) whose members it moves and all lands at height 0.

use std::collections::{BTreeMap, BTreeSet};

use c600_jumble::model::{NP, POLES};
use c600_jumble::observe::Tables;
use c600_jumble::state::{State, Status};
use c600_jumble::twist::Twist;
use c600_kernel::json::Value;
use c600_kernel::parallel;

use super::common::{
    self, claim_bool, claim_int, claim_ints, engine, heights, holds, labels, lattice_inside, start, Form, Labels,
};
use crate::archive::SubChecker;
use crate::claims::{array, at, document, dot, field, hex64, keys, object, reject, Run, Step};
use crate::kinds::{Checker, Needs};
use crate::replay::{tree_tables, Jumbling};
use crate::report::Sources;
use crate::Engines;

pub static SOURCES: Sources = Sources::new(
    "s14.blocks",
    &[
        ("c600-checkers/Cargo.toml", include_bytes!("../../../Cargo.toml")),
        ("c600-checkers/src/archive/s14/blocks.rs", include_bytes!("blocks.rs")),
        ("c600-checkers/src/archive/s14/common.rs", include_bytes!("common.rs")),
        ("c600-checkers/src/claims.rs", include_bytes!("../../claims.rs")),
        ("c600-checkers/src/kinds/mod.rs", include_bytes!("../../kinds/mod.rs")),
        ("c600-checkers/src/lib.rs", include_bytes!("../../lib.rs")),
        ("c600-checkers/src/replay.rs", include_bytes!("../../replay.rs")),
        ("c600-checkers/src/report.rs", include_bytes!("../../report.rs")),
    ],
);

pub static SUBCHECKER: SubChecker = SubChecker {
    checker: Checker { sources: &SOURCES, needs: Needs::JUMBLE, run },
    stage: 14,
    family: "S14 holder, co-held and block records (Lemma C, Corollary L1)",
};

const COHELD: [&str; 12] = [
    "branch",
    "coheld_foreign",
    "coheld_foreign_heights",
    "common_holder_set_foreign",
    "dominating_foreign",
    "dominating_foreign_heights",
    "height",
    "holders",
    "member_holder_rows",
    "own",
    "pose",
    "size",
];

fn run(engines: &Engines, run: &mut Run) -> Step<()> {
    let ctx = engine(engines)?;
    let doc = document(run.cert.bytes)?;
    let form = common::own(&doc, SOURCES.id)?;
    let fields: &[&str] = match form {
        Form::Coheld => &["digest", "groups"],
        Form::Caps => &["caps", "digest"],
        Form::Anatomy => &["anatomy", "digest", "phi", "targets"],
        other => return Err(reject("certificate", format!("{other:?} is not a block form"))),
    };
    keys(&doc, "certificate", fields, &[])?;
    let stated = hex64(field(&doc, "digest", "digest")?, "digest")?.to_owned();
    let j = start(ctx, run, "digest", &stated)?;
    let tables = tree_tables(ctx, "digest")?;
    let facts = Facts::build(&j, tables)?;
    match form {
        Form::Coheld => coheld(run, &j.state, &facts, &doc),
        Form::Caps => caps(run, &j, &facts, &doc),
        _ => anatomy(run, &j, tables, &facts, &doc),
    }
}

/// The facts of the stated table every block record reads.
struct Facts {
    labels: Labels,
    /// Each off-lattice piece's holders, ascending.
    holders: BTreeMap<u32, Vec<usize>>,
    class: Vec<i64>,
    clean: Vec<bool>,
    pure: Vec<bool>,
    /// Height and branch of every piece.
    h: Vec<i64>,
    b: Vec<i64>,
}

impl Facts {
    fn build(j: &Jumbling, tables: &Tables) -> Step<Facts> {
        let ctx = j.ctx;
        let state = &j.state;
        let all: Vec<usize> = (0..POLES).collect();
        let labels = labels(ctx, state, &all, false, "digest")?;
        let heights = heights(ctx, tables, state, "digest")?;
        let class = (0..POLES)
            .map(|c| tables.cls(ctx, c).map_err(|e| reject("digest", format!("the class of grip {c}: {e}"))))
            .collect::<Step<Vec<i64>>>()?;
        let mut holders: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
        for c in 0..POLES {
            for &p in labels.inside(c) {
                holders.entry(p).or_default().push(c);
            }
        }
        let (mut h, mut b) = (vec![0i64; NP], vec![0i64; NP]);
        for p in 0..NP {
            (h[p], b[p]) = heights.of(state, p);
        }
        let clean: Vec<bool> = (0..POLES).map(|c| labels.straddle(c).is_empty()).collect();
        let pure: Vec<bool> = (0..POLES)
            .map(|c| clean[c] && labels.inside(c).iter().all(|&p| h[p as usize] < 2 || b[p as usize] == class[c]))
            .collect();
        Ok(Facts { labels, holders, class, clean, pure, h, b })
    }

    fn row(&self, p: u32) -> &[usize] {
        self.holders.get(&p).map_or(&[], |r| &r[..])
    }

    /// The high units: pose ID and ascending members.
    fn high_units(&self, state: &State) -> BTreeMap<u32, Vec<u32>> {
        let mut units: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for p in 0..NP {
            if self.h[p] >= 4 {
                units.entry(state.pose_ids()[p]).or_default();
            }
        }
        for p in 0..NP as u32 {
            if let Some(members) = units.get_mut(&state.pose_ids()[p as usize]) {
                members.push(p);
            }
        }
        units
    }

    /// The grips holding every one of `members`, ascending, with the number of members each grip holds.
    fn holders_of(&self, members: &[u32]) -> (Vec<usize>, Vec<i64>) {
        let mut counts = vec![0i64; POLES];
        for &p in members {
            for &c in self.row(p) {
                counts[c] += 1;
            }
        }
        let whole = (0..POLES).filter(|&c| counts[c] == members.len() as i64).collect();
        (whole, counts)
    }
}

fn grips_value(grips: &[usize]) -> Vec<i64> {
    grips.iter().map(|&c| c as i64).collect()
}

fn list_claim(run: &mut Run, obj: &Value, path: &str, key: &str, computed: &[i64]) -> Step<()> {
    claim_ints(run, field(obj, key, path)?, &dot(path, key), computed)
}

// ------------------------------------------------------------------------------------------------ Lemma C

fn coheld(run: &mut Run, state: &State, facts: &Facts, doc: &Value) -> Step<()> {
    let candidates: Vec<u32> = (0..NP as u32).filter(|&p| facts.h[p as usize] >= 2).collect();
    let groups = array(field(doc, "groups", "groups")?, "groups")?;
    let units = facts.high_units(state);
    run.claim("groups.length", &groups.len(), &units.len())?;
    for (k, (value, (pose, members))) in groups.iter().zip(&units).enumerate() {
        let path = at("groups", k);
        keys(value, &path, &COHELD, &[])?;
        let (h, b) = (facts.h[members[0] as usize], facts.b[members[0] as usize]);
        claim_int(run, value, &path, "pose", *pose as i64)?;
        claim_int(run, value, &path, "size", members.len() as i64)?;
        claim_int(run, value, &path, "height", h)?;
        claim_int(run, value, &path, "branch", b)?;
        let (whole, _) = facts.holders_of(members);
        list_claim(run, value, &path, "holders", &grips_value(&whole))?;
        let own: Vec<usize> = whole.iter().copied().filter(|&c| facts.class[c] == b).collect();
        list_claim(run, value, &path, "own", &grips_value(&own))?;
        let rows: BTreeSet<&[usize]> = members.iter().map(|&p| facts.row(p)).collect();
        claim_int(run, value, &path, "member_holder_rows", rows.len() as i64)?;
        let uniform = rows.len() == 1;
        let (mut common, mut dominating) = (Vec::new(), Vec::new());
        for &p in &candidates {
            if facts.b[p as usize] == b {
                continue;
            }
            let row = facts.row(p);
            if row == &whole[..] {
                common.push(p);
            }
            if whole.iter().all(|c| row.binary_search(c).is_ok()) {
                dominating.push(p);
            }
        }
        let heights_of = |pieces: &[u32]| -> Vec<i64> {
            pieces.iter().map(|&p| facts.h[p as usize]).collect::<BTreeSet<i64>>().into_iter().collect()
        };
        let coheld: &[u32] = if uniform { &common } else { &[] };
        claim_int(run, value, &path, "coheld_foreign", coheld.len() as i64)?;
        list_claim(run, value, &path, "coheld_foreign_heights", &heights_of(coheld))?;
        claim_int(run, value, &path, "common_holder_set_foreign", common.len() as i64)?;
        claim_int(run, value, &path, "dominating_foreign", dominating.len() as i64)?;
        list_claim(run, value, &path, "dominating_foreign_heights", &heights_of(&dominating))?;
    }
    Ok(())
}

// ------------------------------------------------------------------------------------------- per-grip rows

fn caps(run: &mut Run, j: &Jumbling, facts: &Facts, doc: &Value) -> Step<()> {
    let ctx = j.ctx;
    let rows = array(field(doc, "caps", "caps")?, "caps")?;
    run.claim("caps.length", &rows.len(), &POLES)?;
    let clean_grips: Vec<usize> = (0..POLES).filter(|&c| facts.clean[c]).collect();
    let lattice = lattice_inside(ctx, &j.state, &clean_grips, "caps")?;
    // The (height, branch) groups of the state, as blockmap keys them (branch −1 on the lattice).
    let groups: BTreeSet<(i64, i64)> = (0..NP).map(|p| (facts.h[p], facts.b[p])).collect();
    // M[c][d] for clean c: the pieces inside c that straddle d.
    let straddle = |d: usize| facts.labels.straddle(d);
    let m: BTreeMap<usize, Vec<i64>> = clean_grips
        .iter()
        .map(|&c| (c, (0..POLES).map(|d| common::intersection(facts.labels.inside(c), straddle(d)) as i64).collect()))
        .collect();
    let n_s: Vec<i64> = (0..POLES).map(|d| straddle(d).len() as i64).collect();
    let has_clean_holder = |p: u32| facts.row(p).iter().any(|&c| facts.clean[c]);
    for (c, value) in rows.iter().enumerate() {
        let path = at("caps", c);
        let blocked = !facts.clean[c];
        let fields: &[&str] = if blocked {
            &[
                "by_height",
                "cap",
                "clean_covers",
                "cls",
                "every_straddler_has_clean_holder",
                "freed_by_one_retained_twist",
                "n_clean_covers",
                "other_branch",
                "own_branch",
                "straddlers",
            ]
        } else {
            &["by_height", "cap", "cls", "inside", "other_branch", "own_branch", "pure", "straddlers"]
        };
        keys(value, &path, fields, &[])?;
        claim_int(run, value, &path, "cap", c as i64)?;
        claim_int(run, value, &path, "cls", facts.class[c])?;
        let s = straddle(c);
        claim_int(run, value, &path, "straddlers", s.len() as i64)?;
        let own = s.iter().filter(|&&p| facts.b[p as usize] == facts.class[c]).count() as i64;
        claim_int(run, value, &path, "own_branch", own)?;
        claim_int(run, value, &path, "other_branch", s.len() as i64 - own)?;
        let mut by_height: BTreeMap<String, i64> = BTreeMap::new();
        for &p in s {
            *by_height.entry(facts.h[p as usize].to_string()).or_default() += 1;
        }
        counts_claim(run, value, &path, "by_height", &by_height)?;
        if blocked {
            let covers: Vec<usize> = clean_grips.iter().copied().filter(|g| m[g][c] == n_s[c]).collect();
            list_claim(run, value, &path, "clean_covers", &grips_value(&covers[..covers.len().min(20)]))?;
            claim_int(run, value, &path, "n_clean_covers", covers.len() as i64)?;
            claim_bool(run, value, &path, "every_straddler_has_clean_holder", s.iter().all(|&p| has_clean_holder(p)))?;
            array(field(value, "freed_by_one_retained_twist", &path)?, &dot(&path, "freed_by_one_retained_twist"))?;
        } else {
            let index = clean_grips.binary_search(&c).map_err(|_| reject(&path, "a clean grip"))?;
            let mut inside: BTreeMap<String, i64> = BTreeMap::new();
            for &(h, b) in &groups {
                let key = if h == 0 {
                    "lattice".to_owned()
                } else {
                    format!("h{h}-{}", if b == facts.class[c] { "own" } else { "other" })
                };
                inside.entry(key).or_default();
            }
            for &p in facts.labels.inside(c).iter().chain(&lattice[index]) {
                let (h, b) = (facts.h[p as usize], facts.b[p as usize]);
                let key = if h == 0 {
                    "lattice".to_owned()
                } else {
                    format!("h{h}-{}", if b == facts.class[c] { "own" } else { "other" })
                };
                *inside.entry(key).or_default() += 1;
            }
            counts_claim(run, value, &path, "inside", &inside)?;
            claim_bool(run, value, &path, "pure", facts.pure[c])?;
        }
    }
    // The retained twists that free each blocked grip, compared last: they are the costly part.
    let freed = freed_twists(run, j, facts, &clean_grips, &lattice, &m, &n_s)?;
    for (c, value) in rows.iter().enumerate() {
        if facts.clean[c] {
            continue;
        }
        let path = at("caps", c);
        let twists = freed.get(&c).map_or(&[][..], |v| &v[..]);
        let fp = dot(&path, "freed_by_one_retained_twist");
        let stated = array(field(value, "freed_by_one_retained_twist", &path)?, &fp)?;
        let shown = &twists[..twists.len().min(20)];
        run.claim(&format!("{fp}.length"), &stated.len(), &shown.len())?;
        for (i, (entry, &(g, k))) in stated.iter().zip(shown).enumerate() {
            claim_ints(run, entry, &at(&fp, i), &[g as i64, k as i64])?;
        }
    }
    Ok(())
}

/// The retained twists (c, k) that free each blocked grip d, in the order of the caps c and of k in A4_c, each applied
/// on the engine to a copy of the state. A twist at cap c moves exactly the pieces inside c, so a straddler of d
/// outside c stays: only a clean cap that covers d (holds every straddler of d) can free it. At each such cap every
/// retained letter (k in A4_c, the identity excluded, in the engine's order) must be admissible and move exactly the
/// cap's pieces, and d is freed when none of the pieces it moved straddles d at its new pose, evaluated exactly. The
/// twists run on the machine's threads; the result does not depend on them.
#[allow(clippy::type_complexity)]
fn freed_twists(
    run: &mut Run,
    j: &Jumbling,
    facts: &Facts,
    clean_grips: &[usize],
    lattice: &[Vec<u32>],
    m: &BTreeMap<usize, Vec<i64>>,
    n_s: &[i64],
) -> Step<BTreeMap<usize, Vec<(usize, u16)>>> {
    let ctx = j.ctx;
    let menu_items = j.menu.items.len();
    // (cap, K⁺ element, item, twist, the grips the cap covers, the pieces it holds)
    let mut jobs: Vec<(usize, u16, usize, Twist, Vec<usize>, Vec<u32>)> = Vec::new();
    for (index, &c) in clean_grips.iter().enumerate() {
        let covered: Vec<usize> = (0..POLES).filter(|&d| n_s[d] > 0 && m[&c][d] == n_s[d]).collect();
        if covered.is_empty() {
            continue;
        }
        let claim = format!("caps.freed_by_one_retained_twist (cap {c})");
        let mut moved = facts.labels.inside(c).to_vec();
        moved.extend(&lattice[index]);
        moved.sort_unstable();
        let mut retained: BTreeMap<u16, usize> = BTreeMap::new();
        for item in 1..menu_items {
            if let Some(k) = j.twist(c, item, &claim)?.kidx {
                retained.insert(k, item);
            }
        }
        for k in ctx.kplus.a4(c) {
            if k == 0 {
                continue;
            }
            let Some(&item) = retained.get(&k) else {
                return Err(reject(&claim, format!("the menu has no retained letter of K⁺ element {k} at cap {c}")));
            };
            jobs.push((c, k, item, j.twist(c, item, &claim)?, covered.clone(), moved.clone()));
        }
    }
    let results = parallel::map(jobs.len(), |n| -> Step<(bool, Vec<Vec<u32>>)> {
        let (c, _, _, twist, covered, moved) = &jobs[n];
        let claim = format!("caps.freed_by_one_retained_twist (cap {c})");
        let mut after = Jumbling::new(ctx, j.menu.clone(), j.state.clone());
        let outcome = after.apply(twist, &claim)?;
        if outcome.status != Status::Admissible || &outcome.inside != moved {
            return Ok((false, Vec::new()));
        }
        Ok((true, common::straddlers_among(ctx, &after.state, moved, covered, &claim)?))
    });
    let mut freed: BTreeMap<usize, Vec<(usize, u16)>> = BTreeMap::new();
    for ((c, k, item, _, covered, _), result) in jobs.iter().zip(results) {
        let claim = format!("caps.freed_by_one_retained_twist (cap {c})");
        let (moved_as_held, after) = result.map_err(|_| reject(&claim, "an engine application did not finish"))??;
        if !moved_as_held {
            let stop = reject(
                &claim,
                format!("the retained letter [{c}, {item}] at a clean cap is refused or moves another set"),
            );
            return Err(run.failed(stop, "refused or another set"));
        }
        for (d, now) in covered.iter().zip(after) {
            if now.is_empty() {
                freed.entry(*d).or_default().push((*c, *k));
            }
        }
    }
    Ok(freed)
}

/// A stated object of counts compared key by key with the computed one.
fn counts_claim(run: &mut Run, obj: &Value, path: &str, key: &str, computed: &BTreeMap<String, i64>) -> Step<()> {
    let claim = dot(path, key);
    let stated = object(field(obj, key, path)?, &claim)?;
    let named: Vec<&String> = stated.keys().collect();
    let found: Vec<&String> = computed.keys().collect();
    if named != found {
        let stop = reject(&claim, format!("the keys are {named:?}, not {found:?}"));
        return Err(run.failed(stop, format!("{found:?}")));
    }
    for (k, &n) in computed {
        claim_int(run, field(obj, key, path)?, &claim, k, n)?;
    }
    Ok(())
}

// ----------------------------------------------------------------------------------------------- the anatomy

/// A unit row of an anatomy: pose, the members in the set, and its holders.
struct Unit {
    pose: u32,
    members: Vec<u32>,
}

fn units_of(state: &State, pieces: &[u32]) -> Vec<Unit> {
    let mut by_pose: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for &p in pieces {
        by_pose.entry(state.pose_ids()[p as usize]).or_default().push(p);
    }
    by_pose.into_iter().map(|(pose, members)| Unit { pose, members }).collect()
}

fn anatomy(run: &mut Run, j: &Jumbling, tables: &Tables, facts: &Facts, doc: &Value) -> Step<()> {
    let state = &j.state;
    let phi: i64 = (0..NP).map(|p| (facts.h[p] - 2).max(0)).sum();
    claim_int(run, doc, "", "phi", phi)?;
    let rows = array(field(doc, "anatomy", "anatomy")?, "anatomy")?;
    let with_moves = rows.iter().any(|r| r.get("moves").is_some());
    // The targets: the named one of a moves record, or the own holders of the high units in unit order.
    let targets: Vec<usize> = if with_moves {
        let named = array(field(doc, "targets", "targets")?, "targets")?;
        named.iter().enumerate().map(|(k, v)| common::grip(v, &at("targets", k))).collect::<Step<_>>()?
    } else {
        let mut targets = Vec::new();
        for members in facts.high_units(state).values() {
            let b = facts.b[members[0] as usize];
            let (whole, _) = facts.holders_of(members);
            for c in whole {
                if facts.class[c] == b && !targets.contains(&c) {
                    targets.push(c);
                }
            }
        }
        list_claim(run, doc, "", "targets", &grips_value(&targets))?;
        targets
    };
    run.claim("anatomy.length", &rows.len(), &targets.len())?;
    for (k, (value, &x)) in rows.iter().zip(&targets).enumerate() {
        let path = at("anatomy", k);
        let mut fields = vec![
            "cap",
            "clean",
            "cls",
            "foreign_load",
            "foreign_units",
            "pure",
            "straddlers",
            "units",
            "units_with_clean_holder",
            "units_with_clean_pure_own_holder",
        ];
        if with_moves {
            fields.push("moves");
        }
        keys(value, &path, &fields, &[])?;
        claim_int(run, value, &path, "cap", x as i64)?;
        claim_int(run, value, &path, "cls", facts.class[x])?;
        let s: Vec<u32> = facts.labels.straddle(x).to_vec();
        let foreign: Vec<u32> = facts
            .labels
            .inside(x)
            .iter()
            .copied()
            .filter(|&p| facts.h[p as usize] >= 2 && facts.b[p as usize] != facts.class[x])
            .collect();
        claim_int(run, value, &path, "straddlers", s.len() as i64)?;
        let units = units_of(state, &s);
        let funits = units_of(state, &foreign);
        let clean_holders = |u: &Unit| -> (Vec<usize>, Vec<usize>, Vec<usize>, usize, Vec<usize>, Vec<usize>) {
            let b = facts.b[u.members[0] as usize];
            let (whole, _) = facts.holders_of(&u.members);
            let cell: Vec<usize> = whole.iter().copied().filter(|&c| facts.class[c] == b).collect();
            let pure_own = whole.iter().copied().filter(|&c| facts.clean[c] && facts.pure[c] && facts.class[c] == b);
            let impure_own = whole.iter().copied().filter(|&c| facts.clean[c] && !facts.pure[c] && facts.class[c] == b);
            let other = whole.iter().copied().filter(|&c| facts.clean[c] && facts.class[c] != b);
            let blocked = whole.iter().filter(|&&c| !facts.clean[c]).count();
            (pure_own.collect(), impure_own.collect(), other.collect(), blocked, cell, whole)
        };
        // Returns the number of units with a clean pure own holder and the number with any clean holder.
        let mut unit_rows = |name: &str, key: &str, units: &[Unit]| -> Step<(i64, i64)> {
            let up = dot(&path, name);
            let stated = array(field(value, name, &path)?, &up)?;
            run.claim(&format!("{up}.length"), &stated.len(), &units.len())?;
            let (mut pure_held, mut clean_held) = (0, 0);
            for (i, (entry, unit)) in stated.iter().zip(units).enumerate() {
                let ep = at(&up, i);
                keys(
                    entry,
                    &ep,
                    &[
                        "blocked_holders",
                        "branch",
                        "cell",
                        "clean_impure_own",
                        "clean_other",
                        "clean_pure_own",
                        "height",
                        "holders",
                        "pose",
                        "size",
                        key,
                    ],
                    &[],
                )?;
                let first = unit.members[0] as usize;
                claim_int(run, entry, &ep, "pose", unit.pose as i64)?;
                claim_int(run, entry, &ep, "height", facts.h[first])?;
                claim_int(run, entry, &ep, "branch", facts.b[first])?;
                claim_int(run, entry, &ep, key, unit.members.len() as i64)?;
                let size = state.pose_ids().iter().filter(|&&id| id == unit.pose).count() as i64;
                claim_int(run, entry, &ep, "size", size)?;
                let (pure_own, impure_own, other, blocked, cell, whole) = clean_holders(unit);
                list_claim(run, entry, &ep, "cell", &grips_value(&cell))?;
                claim_int(run, entry, &ep, "holders", whole.len() as i64)?;
                list_claim(run, entry, &ep, "clean_pure_own", &grips_value(&pure_own))?;
                list_claim(run, entry, &ep, "clean_impure_own", &grips_value(&impure_own))?;
                list_claim(run, entry, &ep, "clean_other", &grips_value(&other))?;
                claim_int(run, entry, &ep, "blocked_holders", blocked as i64)?;
                if !pure_own.is_empty() {
                    pure_held += 1;
                }
                if !(pure_own.is_empty() && impure_own.is_empty() && other.is_empty()) {
                    clean_held += 1;
                }
            }
            Ok((pure_held, clean_held))
        };
        let (pure_held, clean_held) = unit_rows("units", "on_cut", &units)?;
        unit_rows("foreign_units", "in_cap", &funits)?;
        claim_int(run, value, &path, "units_with_clean_pure_own_holder", pure_held)?;
        claim_int(run, value, &path, "units_with_clean_holder", clean_held)?;
        let mut load: BTreeMap<String, i64> = BTreeMap::new();
        for &p in &foreign {
            let (h, b) = (facts.h[p as usize], facts.b[p as usize]);
            *load.entry(format!("h{h}-b{b}")).or_default() += 1;
        }
        counts_claim(run, value, &path, "foreign_load", &load)?;
        claim_bool(run, value, &path, "clean", facts.clean[x])?;
        claim_bool(run, value, &path, "pure", facts.pure[x])?;
        if with_moves {
            let mut caps: BTreeSet<usize> = BTreeSet::new();
            let mut members: Vec<(u32, Vec<u32>)> = Vec::new();
            for unit in units.iter().chain(&funits) {
                let (pure_own, impure_own, other, ..) = clean_holders(unit);
                caps.extend(pure_own.into_iter().chain(impure_own).chain(other));
                if !members.iter().any(|(pose, _)| *pose == unit.pose) {
                    let all: Vec<u32> = s
                        .iter()
                        .chain(&foreign)
                        .copied()
                        .filter(|&p| state.pose_ids()[p as usize] == unit.pose)
                        .collect::<BTreeSet<u32>>()
                        .into_iter()
                        .collect();
                    members.push((unit.pose, all));
                }
            }
            moves(run, j, tables, facts, value, &path, x, &caps, &members)?;
        }
    }
    Ok(())
}

/// Every non-identity I_a letter at each of `caps`, applied exactly to a copy of the state.
#[allow(clippy::too_many_arguments)]
fn moves(
    run: &mut Run,
    j: &Jumbling,
    tables: &Tables,
    facts: &Facts,
    value: &Value,
    path: &str,
    x: usize,
    caps: &BTreeSet<usize>,
    units: &[(u32, Vec<u32>)],
) -> Step<()> {
    let ctx = j.ctx;
    let mp = dot(path, "moves");
    let stated = array(field(value, "moves", path)?, &mp)?;
    let items = j.menu.items.len();
    run.claim(&format!("{mp}.length"), &stated.len(), &(caps.len() * (items - 1)))?;
    let phi_before: i64 = (0..NP).map(|p| (facts.h[p] - 2).max(0)).sum();
    let mut k = 0;
    for &c in caps {
        let mut held = facts.labels.inside(c).to_vec();
        held.extend(lattice_inside(ctx, &j.state, &[c], &mp)?.remove(0));
        held.sort_unstable();
        for item in 1..items {
            let ep = at(&mp, k);
            k += 1;
            let entry = &stated[k - 1];
            let twist = j.twist(c, item, &ep)?;
            let mut after = Jumbling::new(ctx, j.menu.clone(), j.state.clone());
            let outcome = after.apply(&twist, &ep)?;
            claim_int(run, entry, &ep, "cap", c as i64)?;
            claim_int(run, entry, &ep, "item", item as i64)?;
            let applied = outcome.status == Status::Admissible;
            claim_bool(run, entry, &ep, "applied", applied)?;
            if !applied {
                keys(entry, &ep, &["applied", "cap", "item"], &[])?;
                continue;
            }
            keys(
                entry,
                &ep,
                &[
                    "applied",
                    "cap",
                    "dphi",
                    "foreign_x",
                    "item",
                    "landed_units",
                    "max_h",
                    "moved",
                    "off",
                    "straddlers_x",
                ],
                &[],
            )?;
            if outcome.inside != held {
                let stop = reject(dot(&ep, "moved"), "the engine moved another set than the cap's pieces");
                return Err(run.failed(stop, outcome.inside.len()));
            }
            claim_int(run, entry, &ep, "moved", held.len() as i64)?;
            let hs = heights(ctx, tables, &after.state, &ep)?;
            let seen = common::observe_from(ctx, &after.state, &hs);
            claim_int(run, entry, &ep, "dphi", seen.phi - phi_before)?;
            claim_int(run, entry, &ep, "max_h", seen.max_h)?;
            claim_int(run, entry, &ep, "off", seen.off)?;
            let at_x = labels(ctx, &after.state, &[x], false, &ep)?;
            claim_int(run, entry, &ep, "straddlers_x", at_x.straddle(x).len() as i64)?;
            let foreign = at_x
                .inside(x)
                .iter()
                .filter(|&&p| {
                    let (h, b) = hs.of(&after.state, p as usize);
                    h >= 2 && b != facts.class[x]
                })
                .count() as i64;
            claim_int(run, entry, &ep, "foreign_x", foreign)?;
            let landed: Vec<i64> = units
                .iter()
                .filter(|(_, members)| {
                    members.iter().all(|&p| holds(&held, p) && hs.of(&after.state, p as usize).0 == 0)
                })
                .map(|(pose, _)| *pose as i64)
                .collect();
            list_claim(run, entry, &ep, "landed_units", &landed)?;
        }
    }
    Ok(())
}
