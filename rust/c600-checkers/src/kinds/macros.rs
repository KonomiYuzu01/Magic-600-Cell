//! Macro certificates: the archive's `macro-certificate/1` (stage 15, `scripts/macro.py`), checked claim by claim by
//! literal replay on the jumbling engine.
//!
//! The kind covers what Theorem MC covers: descending macros of the I_a menu ending in a jumble (LB5.11). A macro whose
//! last event is a retained letter, such as the release SJ-E1, is `not_applicable` here and is checked letter by letter
//! by another kind. Every claim is recomputed from the exact state after each event, never read from a label table
//! or a cache:
//!
//! - `table_digest`: the digest of the supplied start table, which must also reproduce its own stated digest;
//! - `start`: the digest and Φ after the prefix, every prefix letter admissible;
//! - per event, in order: admissibility (no straddler at the acting cap), the support (the moved pieces, as their count
//!   and the SHA-256 of the sorted IDs as int64), δ (the change of Φ), Φ and the state digest;
//! - at the last event, a jumble: F, the inside pieces of height 2 or more whose branch is not the cap's class, and D,
//!   the inside pieces of height 4 or more of the cap's class that the jumble lowers by 2, both read from the heights
//!   before and after the event; Lemma Φ, δ = 2(F − D);
//! - per macro: the excursion (the largest partial sum of δ, at most the declared allowance), and a strict descent;
//! - with `unit_piece`, the unit (the pieces of that piece's pose at the macro's start) is high at the start, wholly
//!   inside the lowering cap and still high before the last event, and every member is lowered by 2.
//!
//! Φ = Σ max(0, h − 2) over the left heights h of every piece. Heights and branches come from the engine's exact tree
//! facts; the right height of every pose must be 0, as stage 14 requires. The menu must be I_a by identity and order.

use c600_jumble::model::NP;
use c600_jumble::nu::{I_A_MENU_IDENTITY, I_A_ORDER_DIGEST};
use c600_jumble::state::{Pose, Status};
use c600_kernel::json::Value;

use super::{Checker, Needs};
use crate::claims::{
    array, at, count, document, dot, field, hex64, int, keys, not_applicable, object, reject, require_format, Run, Step,
};
use crate::replay::{left_facts, letter, phi, read_menu, read_state, support, tree_tables, Jumbling};
use crate::report::{codes, Sources};
use crate::Engines;

/// The archive format this kind reads.
pub const FORMAT: &str = "macro-certificate/1";

pub static SOURCES: Sources = Sources::new(
    "kn6.macro-certificate/1",
    &[
        ("c600-checkers/Cargo.toml", include_bytes!("../../Cargo.toml")),
        ("c600-checkers/src/claims.rs", include_bytes!("../claims.rs")),
        ("c600-checkers/src/kinds/macros.rs", include_bytes!("macros.rs")),
        ("c600-checkers/src/kinds/mod.rs", include_bytes!("mod.rs")),
        ("c600-checkers/src/lib.rs", include_bytes!("../lib.rs")),
        ("c600-checkers/src/replay.rs", include_bytes!("../replay.rs")),
        ("c600-checkers/src/report.rs", include_bytes!("../report.rs")),
    ],
);

pub static CHECKER: Checker = Checker { sources: &SOURCES, needs: Needs::JUMBLE, run };

const CLAIM_NAMES: [&str; 7] = ["supports", "delta", "phi", "digests", "excursion", "F", "D"];

/// One macro as the certificate states it.
struct Macro {
    events: Vec<(usize, usize)>,
    unit: Option<usize>,
    supports: Vec<(u64, String)>,
    delta: Vec<i64>,
    phi: Vec<i64>,
    digests: Vec<String>,
    excursion: i64,
    f: u64,
    d: u64,
}

fn run(engines: &Engines, run: &mut Run) -> Step<()> {
    let ctx = engines.jumble.ok_or_else(|| reject("certificate", "the jumbling engine is absent"))?;
    let doc = document(run.cert.bytes)?;
    require_format(&doc, FORMAT)?;
    keys(&doc, "certificate", &["format", "table_digest", "prefix", "start", "excursion", "macros"], &[])?;
    let allowance = count(field(&doc, "excursion", "excursion")?, "excursion")?;
    if allowance % 2 != 0 {
        return Err(reject("excursion", "the allowance is an even nonnegative integer"));
    }
    let allowance = allowance as i64;

    let menu = read_menu(ctx, run, "menu")?;
    run.claim("menu.identity", &I_A_MENU_IDENTITY.to_owned(), &menu.identity)?;
    run.claim("menu.order", &I_A_ORDER_DIGEST.to_owned(), &menu.order)?;
    let items = menu.items.len();

    let prefix = array(field(&doc, "prefix", "prefix")?, "prefix")?
        .iter()
        .enumerate()
        .map(|(k, value)| letter(value, &at("prefix", k), items))
        .collect::<Step<Vec<_>>>()?;
    let macros = array(field(&doc, "macros", "macros")?, "macros")?
        .iter()
        .enumerate()
        .map(|(m, spec)| read_macro(spec, &at("macros", m), items))
        .collect::<Step<Vec<_>>>()?;

    // LB5.11: decided from the certificate alone, before any input is read.
    let probe = Jumbling::new(ctx, menu.clone(), c600_jumble::state::State::new(Some(menu.clone())));
    for (m, spec) in macros.iter().enumerate() {
        let (cap, item) = *spec.events.last().expect("a macro has events");
        if probe.twist(cap, item, &at("macros", m))?.kidx.is_some() {
            return Err(not_applicable(format!(
                "macro {m} ends in a retained letter; {FORMAT} covers descending macros ending in a jumble (LB5.11)"
            )));
        }
    }
    drop(probe);

    if run.optional_input("table").is_none() {
        return Err(crate::claims::start_mismatch("table", "the certificate's start table is absent"));
    }
    let table = read_state(ctx, run, &menu, "table")?;
    let mut j = Jumbling::new(ctx, menu, table);
    let stated = hex64(field(&doc, "table_digest", "table_digest")?, "table_digest")?.to_owned();
    run.compare("table_digest", &stated, &j.digest(), codes::START_TABLE_MISMATCH)?;

    for (k, &(cap, item)) in prefix.iter().enumerate() {
        let claim = at("prefix", k);
        let twist = j.twist(cap, item, &claim)?;
        let outcome = j.apply(&twist, &claim)?;
        if outcome.status != Status::Admissible {
            return Err(run.failed(reject(&claim, format!("the letter is {}", outcome.status.as_str())), "refused"));
        }
        run.held(&claim, "admissible");
    }

    let tables = tree_tables(ctx, "start.phi")?;
    let start = field(&doc, "start", "start")?;
    keys(start, "start", &["digest", "phi"], &[])?;
    let stated = hex64(field(start, "digest", "start")?, "start.digest")?.to_owned();
    run.claim("start.digest", &stated, &j.digest())?;
    let mut phi_now = phi(ctx, tables, &j.state, "start.phi")?;
    run.claim("start.phi", &int(field(start, "phi", "start")?, "start.phi")?, &phi_now)?;

    for (m, spec) in macros.iter().enumerate() {
        check_macro(run, &mut j, tables, &at("macros", m), spec, allowance, &mut phi_now)?;
    }
    Ok(())
}

/// Reads one macro and checks the form of its claims, as `macro.py`'s `execute` does before any event.
fn read_macro(spec: &Value, path: &str, items: usize) -> Step<Macro> {
    keys(spec, path, &["events", "claims"], &["unit_piece"])?;
    let events_path = dot(path, "events");
    let events = array(field(spec, "events", path)?, &events_path)?
        .iter()
        .enumerate()
        .map(|(e, value)| letter(value, &at(&events_path, e), items))
        .collect::<Step<Vec<_>>>()?;
    if events.is_empty() {
        return Err(reject(&events_path, "a macro has at least one event"));
    }
    let unit = match object(spec, path)?.get("unit_piece") {
        None => None,
        Some(value) => {
            let claim = dot(path, "unit_piece");
            Some(
                usize::try_from(int(value, &claim)?)
                    .ok()
                    .filter(|&p| p < NP)
                    .ok_or_else(|| reject(&claim, format!("a unit piece is a piece ID 0 to {}", NP - 1)))?,
            )
        }
    };
    let claims_path = dot(path, "claims");
    let claims = field(spec, "claims", path)?;
    keys(claims, &claims_path, &CLAIM_NAMES, &[])?;
    let list = |name: &str| -> Step<&[Value]> {
        let claim = dot(&claims_path, name);
        let values = array(field(claims, name, &claims_path)?, &claim)?;
        if values.len() == events.len() {
            Ok(values)
        } else {
            Err(reject(&claim, format!("{name} names {} events, not {}", values.len(), events.len())))
        }
    };
    let even = |name: &str| -> Step<Vec<i64>> {
        let claim = dot(&claims_path, name);
        list(name)?
            .iter()
            .enumerate()
            .map(|(e, value)| {
                let n = int(value, &at(&claim, e))?;
                if n % 2 == 0 {
                    Ok(n)
                } else {
                    Err(reject(at(&claim, e), "an even integer"))
                }
            })
            .collect()
    };
    let delta = even("delta")?;
    let phi = even("phi")?;
    let digests_path = dot(&claims_path, "digests");
    let digests = list("digests")?
        .iter()
        .enumerate()
        .map(|(e, value)| hex64(value, &at(&digests_path, e)).map(str::to_owned))
        .collect::<Step<Vec<_>>>()?;
    let supports_path = dot(&claims_path, "supports");
    let supports = list("supports")?
        .iter()
        .enumerate()
        .map(|(e, value)| {
            let claim = at(&supports_path, e);
            keys(value, &claim, &["size", "sha256"], &[])?;
            let size = count(field(value, "size", &claim)?, &dot(&claim, "size"))?;
            let sha = hex64(field(value, "sha256", &claim)?, &dot(&claim, "sha256"))?.to_owned();
            Ok((size, sha))
        })
        .collect::<Step<Vec<_>>>()?;
    let scalar = |name: &str| count(field(claims, name, &claims_path)?, &dot(&claims_path, name));
    Ok(Macro {
        events,
        unit,
        supports,
        delta,
        phi,
        digests,
        excursion: scalar("excursion")? as i64,
        f: scalar("F")?,
        d: scalar("D")?,
    })
}

fn check_macro(
    run: &mut Run,
    j: &mut Jumbling,
    tables: &c600_jumble::observe::Tables,
    path: &str,
    spec: &Macro,
    allowance: i64,
    phi_now: &mut i64,
) -> Step<()> {
    let ctx = j.ctx;
    let claims = dot(path, "claims");
    let unit_claim = dot(path, "unit_piece");
    let members: Option<Vec<usize>> = spec.unit.map(|piece| {
        let id = j.state.pose_ids()[piece];
        j.state.pose_ids().iter().enumerate().filter(|(_, &q)| q == id).map(|(p, _)| p).collect()
    });
    if let (Some(members), Some(piece)) = (&members, spec.unit) {
        let (h, _) = left_facts(ctx, tables, j.state.pose_of(piece), &unit_claim)?;
        if h < 4 {
            return Err(run.failed(reject(&unit_claim, "the unit is not high at the macro's start"), h));
        }
        run.held(&unit_claim, format!("{} pieces of height {h}", members.len()));
    }
    let (mut cumulative, mut peak) = (0i64, 0i64);
    let last = spec.events.len() - 1;
    for (e, &(cap, item)) in spec.events.iter().enumerate() {
        let event = at(&dot(path, "events"), e);
        let twist = j.twist(cap, item, &event)?;
        let before = (e == last).then(|| (j.state.pose_ids().to_vec(), j.state.poses().to_vec()));
        let outcome = j.apply(&twist, &event)?;
        if outcome.status != Status::Admissible {
            return Err(run.failed(reject(&event, format!("the event is {}", outcome.status.as_str())), "refused"));
        }
        run.held(&event, "admissible");
        let support_claim = at(&dot(&claims, "supports"), e);
        let (size, sha) = support(&outcome.inside);
        run.claim(&dot(&support_claim, "size"), &spec.supports[e].0, &size)?;
        run.claim(&dot(&support_claim, "sha256"), &spec.supports[e].1, &sha)?;
        let phi_after = phi(ctx, tables, &j.state, &at(&dot(&claims, "phi"), e))?;
        let delta = phi_after - *phi_now;

        let mut lowering = None;
        if let Some((ids, poses)) = before {
            let cls = tables.cls(ctx, cap).map_err(|error| reject(&event, format!("the cap's class: {error}")))?;
            let (mut f, mut d) = (0u64, 0u64);
            let mut unit_heights = Vec::new();
            for &p in &outcome.inside {
                let old: &Pose = &poses[ids[p as usize] as usize];
                let (h, b) = left_facts(ctx, tables, old, &dot(&claims, "F"))?;
                if h >= 2 && b != cls {
                    f += 1;
                }
                if h >= 4 && b == cls {
                    let (lowered, _) = left_facts(ctx, tables, j.state.pose_of(p as usize), &dot(&claims, "D"))?;
                    if lowered == h - 2 {
                        d += 1;
                    }
                }
            }
            run.claim(&dot(&claims, "F"), &spec.f, &f)?;
            run.claim(&dot(&claims, "D"), &spec.d, &d)?;
            if let Some(members) = &members {
                for &p in members {
                    if outcome.inside.binary_search(&(p as u32)).is_err() {
                        let stop = reject(&unit_claim, "the unit is not wholly inside the lowering cap");
                        return Err(run.failed(stop, p));
                    }
                    let (h, _) = left_facts(ctx, tables, &poses[ids[p] as usize], &unit_claim)?;
                    let (after, _) = left_facts(ctx, tables, j.state.pose_of(p), &unit_claim)?;
                    unit_heights.push((p, h, after));
                }
                for &(p, h, after) in &unit_heights {
                    if h < 4 {
                        return Err(run.failed(reject(&unit_claim, "the unit is no longer high at the lowering"), p));
                    }
                    if after != h - 2 {
                        return Err(run.failed(reject(&unit_claim, "a member of the unit is not lowered by 2"), p));
                    }
                }
                run.held(&unit_claim, "lowered by 2");
            }
            lowering = Some((f as i64, d as i64));
        }

        cumulative += delta;
        peak = peak.max(cumulative);
        if peak > allowance {
            let stop =
                reject(dot(&claims, "excursion"), format!("the excursion {peak} exceeds the allowance {allowance}"));
            return Err(run.failed(stop, peak));
        }
        run.claim(&at(&dot(&claims, "delta"), e), &spec.delta[e], &delta)?;
        run.claim(&at(&dot(&claims, "phi"), e), &spec.phi[e], &phi_after)?;
        run.claim(&at(&dot(&claims, "digests"), e), &spec.digests[e], &j.digest())?;
        if let Some((f, d)) = lowering {
            if delta != 2 * (f - d) {
                let stop = reject(&claims, format!("Lemma Φ does not hold: δ = {delta}, F = {f}, D = {d}"));
                return Err(run.failed(stop, delta));
            }
            run.held(format!("{claims}.lemma_phi"), format!("δ = 2(F − D) = {delta}"));
        }
        *phi_now = phi_after;
    }
    run.claim(&dot(&claims, "excursion"), &spec.excursion, &peak)?;
    if cumulative >= 0 {
        return Err(run.failed(reject(path, "the macro does not lower Φ"), cumulative));
    }
    run.held(format!("{path}.descent"), cumulative);
    Ok(())
}
