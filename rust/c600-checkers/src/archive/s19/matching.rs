//! `s19.gamma-matching`: Γ(Y)'s finite matching certificate, `profile-family-validation-matching-certificate.json`.
//!
//! The certificate is the archived record itself. Its inputs are the full reference table Y (`reference`) and the
//! synthetic input table Z (`input`), each a stored pose table read and validated in full. The family Γ(Y) holds Z
//! when a type-preserving bijection σ and right gauges t_p in K⁺ give Z_p = Y_σ(p)·t_p with t_p carrying the home
//! region of p onto the home region of σ(p) (stage 19 README). The checker recomputes:
//!
//! - `reference_digest`, `input_digest`: the state digests of the supplied tables (a wrong table is the wrong start);
//! - `admission`: the rows whose exact poses differ; every edge test of those rows (the K⁺ element Y_rᵀ·Z_p, its
//!   type and its exact home-region map); the number of bijections of those rows and, by brute force over all of
//!   them, the number whose every edge holds (CT-LB1-05's matching half: the witness meets every edge constraint and
//!   brute force agrees on the infeasible ones); the stated σ as a bijection of valid edges; each gauge's K⁺ index and
//!   exact matrix; and that σ is a permutation of all 177,120 pieces that keeps each piece's physical type;
//! - `construction_home_maps`: per changed row, σ(p), t_p and its matrix, the heights of p in Y and in Z, both sets of
//!   exact home vertices, the vertex-set map under t_p and the equality of the posed world regions Z_p(home p) and
//!   Y_σ(p)(home σ(p));
//! - `all177120_initial_region_bijection`: Z = Y·γ on every piece, each product exact.
//!
//! `reachability` must say UNKNOWN: the checker establishes no reachability, and a certificate claiming it is outside
//! the kind. `admission.method` is prose. The trace records what the check does not give: no sticker-goal transport,
//! one supplied fibre (the theorem's 73 indexed references are not 73 distinct fibres), no general matcher.

use c600_jumble::model::NP;
use c600_jumble::q5::matrix_from_json;
use c600_jumble::state::State;
use c600_jumble::Context;
use c600_kernel::json::Value;

use super::common::{
    claim_bool, claim_int, claim_ints, edge, form, height, home, image, join, outside, read_table, Gamma,
};
use crate::archive::SubChecker;
use crate::claims::{
    array, at, document, dot, field, hex64, int, keys, not_applicable, object, reject, string, Run, Step,
};
use crate::kinds::{Checker, Needs};
use crate::replay::point;
use crate::report::{codes, Sources};
use crate::Engines;

pub static SOURCES: Sources = Sources::new(
    "s19.gamma-matching",
    &[
        ("c600-checkers/Cargo.toml", include_bytes!("../../../Cargo.toml")),
        ("c600-checkers/src/archive/s19/common.rs", include_bytes!("common.rs")),
        ("c600-checkers/src/archive/s19/matching.rs", include_bytes!("matching.rs")),
        ("c600-checkers/src/claims.rs", include_bytes!("../../claims.rs")),
        ("c600-checkers/src/kinds/mod.rs", include_bytes!("../../kinds/mod.rs")),
        ("c600-checkers/src/lib.rs", include_bytes!("../../lib.rs")),
        ("c600-checkers/src/replay.rs", include_bytes!("../../replay.rs")),
        ("c600-checkers/src/report.rs", include_bytes!("../../report.rs")),
    ],
);

pub static SUBCHECKER: SubChecker = SubChecker {
    checker: Checker { sources: &SOURCES, needs: Needs::JUMBLE, run },
    stage: 19,
    family: "S19 regional matching Γ(Y): the finite bijection and gauge certificate",
};

const KEYS: [&str; 6] = [
    "reference_digest",
    "input_digest",
    "admission",
    "construction_home_maps",
    "all177120_initial_region_bijection",
    "reachability",
];

/// The most changed rows the brute force covers: 8! = 40,320 bijections.
const BRUTE_FORCE_ROWS: usize = 8;

/// The matching the certificate states, once checked.
pub struct Matched {
    pub rows: Vec<u32>,
    pub gamma: Gamma,
}

fn run(engines: &Engines, run: &mut Run) -> Step<()> {
    let ctx = engines.jumble.ok_or_else(|| reject("certificate", "the jumbling engine is absent"))?;
    let doc = document(run.cert.bytes)?;
    if form(&doc, &[&KEYS]).is_none() {
        return Err(outside(SOURCES.id, "Γ(Y)'s matching certificate", &doc));
    }
    keys(&doc, "certificate", &KEYS, &[])?;
    let y = read_table(ctx, None, run, "reference")?;
    let stated = hex64(field(&doc, "reference_digest", "reference_digest")?, "reference_digest")?.to_owned();
    run.compare("reference_digest", &stated, &y.digest, codes::START_TABLE_MISMATCH)?;
    let z = read_table(ctx, None, run, "input")?;
    let stated = hex64(field(&doc, "input_digest", "input_digest")?, "input_digest")?.to_owned();
    run.compare("input_digest", &stated, &z.digest, codes::START_TABLE_MISMATCH)?;
    reachability(run, field(&doc, "reachability", "reachability")?, "reachability")?;

    let matched = admission(ctx, run, field(&doc, "admission", "admission")?, "admission", &y.state, &z.state)?;
    construction(
        ctx,
        run,
        field(&doc, "construction_home_maps", "construction_home_maps")?,
        "construction_home_maps",
        &y.state,
        &z.state,
        &matched,
    )?;
    let joined = join(ctx, &y.state, &z.state, &matched.gamma);
    claim_bool(
        run,
        "all177120_initial_region_bijection",
        field(&doc, "all177120_initial_region_bijection", "")?,
        joined,
    )?;
    scope(run, &matched);
    Ok(())
}

/// The reachability texts that say unknown and nothing more, as the stage records them.
const UNKNOWN: [&str; 2] = ["UNKNOWN", "UNKNOWN; no reaching word or component-membership claim"];

/// A reachability field must be one of the [`UNKNOWN`] texts exactly: any other text may claim reachability, which is
/// outside every stage 19 sub-checker.
pub fn reachability(run: &mut Run, value: &Value, claim: &str) -> Step<()> {
    let text = string(value, claim)?;
    if !UNKNOWN.contains(&text) {
        return Err(not_applicable(
            "a reachability claim is outside this kind: the synthetic state's reachability from solved stays unknown \
             (no reaching word, no component-membership claim; CX.15, LB5.4)",
        ));
    }
    run.held(claim, "unknown: nothing in this check establishes reachability from solved");
    Ok(())
}

/// The scope the trace states for every Γ(Y) check.
pub fn scope(run: &mut Run, matched: &Matched) {
    run.held(
        "scope.sticker_goal",
        format!(
            "not transported: σ moves {} pieces, so Γ(Y) carries occupied regions, legal words and the shape metrics, \
             not sticker labels (LB5.6)",
            matched.gamma.moved()
        ),
    );
    run.held(
        "scope.indexed_references",
        "one supplied fibre: the regional-profile theorem's 73 indexed references are reference indices, not 73 \
         distinct fibres or 73 fresh replays (LB5.6, LB5.16)",
    );
    run.held(
        "scope.matcher",
        format!(
            "a finite supplied certificate, checked by brute force over its {} changed rows; no general matcher is \
             implied (LB5.16)",
            matched.rows.len()
        ),
    );
}

/// `admission`: changed rows, edge tests, the brute-force count, σ and its gauges, and σ's bijectivity and types.
pub fn admission(ctx: &Context, run: &mut Run, value: &Value, claim: &str, y: &State, z: &State) -> Step<Matched> {
    let required = [
        "method",
        "changed_rows",
        "edge_tests",
        "tested_bijections",
        "valid_bijections",
        "recovered_pi_overrides",
        "recovered_gauges",
        "all177120_bijection_and_type_preservation",
    ];
    keys(value, claim, &required, &[])?;
    let get = |key: &str| field(value, key, claim);
    string(get("method")?, &dot(claim, "method"))?;

    let rows = super::common::changed(ctx, y, z);
    if rows.len() > BRUTE_FORCE_ROWS {
        return Err(not_applicable(format!(
            "{} rows differ; the brute-force matching check covers at most {BRUTE_FORCE_ROWS}",
            rows.len()
        )));
    }
    let rows_i: Vec<i64> = rows.iter().map(|&p| p as i64).collect();
    claim_ints(run, &dot(claim, "changed_rows"), get("changed_rows")?, &rows_i)?;

    // Every edge of the changed rows, p-major.
    let n = rows.len();
    let mut edges = vec![None; n * n];
    let tests = array(get("edge_tests")?, &dot(claim, "edge_tests"))?;
    if tests.len() != n * n {
        return Err(reject(dot(claim, "edge_tests"), format!("{} edge tests, not {}", tests.len(), n * n)));
    }
    for (i, &p) in rows.iter().enumerate() {
        for (j, &r) in rows.iter().enumerate() {
            let k = edge(ctx, y, z, p as usize, r as usize);
            edges[i * n + j] = k;
            let path = at(&dot(claim, "edge_tests"), i * n + j);
            let test = &tests[i * n + j];
            keys(test, &path, &["physical", "reference", "right_K_index", "valid_edge"], &[])?;
            claim_int(run, &dot(&path, "physical"), field(test, "physical", &path)?, p as i64)?;
            claim_int(run, &dot(&path, "reference"), field(test, "reference", &path)?, r as i64)?;
            let index = k.map_or(-1, i64::from);
            claim_int(run, &dot(&path, "right_K_index"), field(test, "right_K_index", &path)?, index)?;
            claim_bool(run, &dot(&path, "valid_edge"), field(test, "valid_edge", &path)?, k.is_some())?;
        }
    }

    // Brute force over every bijection of the changed rows.
    let tested: i64 = (1..=n as i64).product();
    let mut valid = 0i64;
    count_matchings(&edges, n, 0, &mut vec![false; n], &mut valid);
    claim_int(run, &dot(claim, "tested_bijections"), get("tested_bijections")?, tested)?;
    claim_int(run, &dot(claim, "valid_bijections"), get("valid_bijections")?, valid)?;

    // The stated σ: a bijection of the changed rows through valid edges.
    let path = dot(claim, "recovered_pi_overrides");
    let pairs = array(get("recovered_pi_overrides")?, &path)?;
    if pairs.len() != n {
        return Err(reject(&path, format!("{} overrides for {n} changed rows", pairs.len())));
    }
    let mut targets = vec![false; n];
    let mut sigma = Vec::with_capacity(n);
    for (i, pair) in pairs.iter().enumerate() {
        let item = at(&path, i);
        let pr = array(pair, &item)?;
        if pr.len() != 2 {
            return Err(reject(&item, "an override is [physical, reference]"));
        }
        claim_int(run, &at(&item, 0), &pr[0], rows[i] as i64)?;
        let r = int(&pr[1], &at(&item, 1))?;
        let j = rows.iter().position(|&x| x as i64 == r).ok_or_else(|| reject(at(&item, 1), "not a changed row"))?;
        if std::mem::replace(&mut targets[j], true) {
            return Err(reject(at(&item, 1), "σ takes this row twice"));
        }
        let k = edges[i * n + j].ok_or_else(|| reject(&item, format!("({}, {r}) is not a valid edge", rows[i])))?;
        sigma.push((rows[i], rows[j], k));
    }
    run.held(&path, format!("σ is a bijection of the {n} changed rows through valid edges"));

    let path = dot(claim, "recovered_gauges");
    let gauges = array(get("recovered_gauges")?, &path)?;
    if gauges.len() != n {
        return Err(reject(&path, format!("{} gauges for {n} changed rows", gauges.len())));
    }
    for (i, gauge) in gauges.iter().enumerate() {
        let item = at(&path, i);
        keys(gauge, &item, &["physical", "right_K_index", "matrix"], &[])?;
        let (p, _, k) = sigma[i];
        claim_int(run, &dot(&item, "physical"), field(gauge, "physical", &item)?, p as i64)?;
        claim_int(run, &dot(&item, "right_K_index"), field(gauge, "right_K_index", &item)?, k as i64)?;
        kplus_matrix(ctx, run, &dot(&item, "matrix"), field(gauge, "matrix", &item)?, k)?;
    }

    let gamma = Gamma::with(&sigma);
    let typed = gamma.bijective_and_typed(ctx);
    let key = "all177120_bijection_and_type_preservation";
    claim_bool(run, &dot(claim, key), get(key)?, typed)?;
    Ok(Matched { rows, gamma })
}

/// The number of bijections of the rows whose every edge is valid.
fn count_matchings(edges: &[Option<u16>], n: usize, i: usize, taken: &mut Vec<bool>, count: &mut i64) {
    if i == n {
        *count += 1;
        return;
    }
    for j in 0..n {
        if !taken[j] && edges[i * n + j].is_some() {
            taken[j] = true;
            count_matchings(edges, n, i + 1, taken, count);
            taken[j] = false;
        }
    }
}

/// A stated matrix that must be K⁺ element `k`, exactly.
fn kplus_matrix(ctx: &Context, run: &mut Run, claim: &str, value: &Value, k: u16) -> Step<()> {
    let m = matrix_from_json(value).map_err(|error| reject(claim, error))?;
    if m != *ctx.kplus.matrix(k, &ctx.model) {
        return Err(reject(claim, format!("the matrix is not K⁺ element {k}")));
    }
    run.held(claim, format!("K⁺ element {k}, exactly"));
    Ok(())
}

/// A stated list of exact points, as a sorted set.
fn vertices(value: &Value, claim: &str) -> Step<Vec<c600_jumble::q5::Vec4>> {
    let mut points =
        array(value, claim)?.iter().enumerate().map(|(k, v)| point(v, &at(claim, k))).collect::<Step<Vec<_>>>()?;
    points.sort();
    Ok(points)
}

/// `construction_home_maps`: one entry per changed row (in any order).
pub fn construction(
    ctx: &Context,
    run: &mut Run,
    value: &Value,
    claim: &str,
    y: &State,
    z: &State,
    matched: &Matched,
) -> Step<()> {
    let entries = array(value, claim)?;
    if entries.len() != matched.rows.len() {
        return Err(reject(claim, format!("{} maps for {} changed rows", entries.len(), matched.rows.len())));
    }
    let mut seen = Vec::new();
    for (i, entry) in entries.iter().enumerate() {
        let path = at(claim, i);
        let required = [
            "physical",
            "reference",
            "initial_physical_height",
            "new_height",
            "right_K_index",
            "right_K_matrix",
            "home_vertices_source",
            "home_vertices_target",
            "complete_home_vertex_set_map",
            "actual_world_region_equality",
        ];
        keys(entry, &path, &required, &[])?;
        object(entry, &path)?;
        let get = |key: &str| field(entry, key, &path);
        let p = int(get("physical")?, &dot(&path, "physical"))?;
        let p = usize::try_from(p)
            .ok()
            .filter(|&p| p < NP && matched.rows.contains(&(p as u32)) && !seen.contains(&p))
            .ok_or_else(|| reject(dot(&path, "physical"), "not a changed row, or one mapped twice"))?;
        seen.push(p);
        let (r, k) = (matched.gamma.sigma[p] as usize, matched.gamma.t[p]);
        claim_int(run, &dot(&path, "reference"), get("reference")?, r as i64)?;
        claim_int(
            run,
            &dot(&path, "initial_physical_height"),
            get("initial_physical_height")?,
            height(ctx, y, p, &path)?,
        )?;
        claim_int(run, &dot(&path, "new_height"), get("new_height")?, height(ctx, z, p, &path)?)?;
        claim_int(run, &dot(&path, "right_K_index"), get("right_K_index")?, k as i64)?;
        kplus_matrix(ctx, run, &dot(&path, "right_K_matrix"), get("right_K_matrix")?, k)?;
        let (source, target) = (home(ctx, p), home(ctx, r));
        let stated = vertices(get("home_vertices_source")?, &dot(&path, "home_vertices_source"))?;
        if stated != source {
            return Err(reject(dot(&path, "home_vertices_source"), format!("not the home vertices of piece {p}")));
        }
        run.held(dot(&path, "home_vertices_source"), format!("the {} home vertices of piece {p}", source.len()));
        let stated = vertices(get("home_vertices_target")?, &dot(&path, "home_vertices_target"))?;
        if stated != target {
            return Err(reject(dot(&path, "home_vertices_target"), format!("not the home vertices of piece {r}")));
        }
        run.held(dot(&path, "home_vertices_target"), format!("the {} home vertices of piece {r}", target.len()));
        let mapped = image(ctx.kplus.matrix(k, &ctx.model), &source) == target;
        claim_bool(run, &dot(&path, "complete_home_vertex_set_map"), get("complete_home_vertex_set_map")?, mapped)?;
        let world =
            image(z.pose_matrix(ctx, z.pose_ids()[p]), &source) == image(y.pose_matrix(ctx, y.pose_ids()[r]), &target);
        claim_bool(run, &dot(&path, "actual_world_region_equality"), get("actual_world_region_equality")?, world)?;
    }
    Ok(())
}
