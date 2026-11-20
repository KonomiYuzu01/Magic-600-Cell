//! Render feature J10's exposed-face geometry on M5's real model (JI.41, JI.45; CT-RM-07 row 10 and CT-RPI-08's J10
//! part at the producer). The geometry is derived from `assets/`, certified in exact Q(√5) and admitted by its size
//! profile; these tests recompute its invariants from the returned table without the builder, compare its counts with
//! EN2's recorded reference (EH-05), check transported faces against M5's regions for sampled pieces and every cell of
//! the full table, and refuse corrupted tables and a missing or different profile with
//! `KN2:input.exposed_geometry_unavailable`. The size profile is recorded in
//! `docs/progress/1.0/evidence/j10-exposed-face-size-profile.json`; `C600_WRITE_EVIDENCE=1` rewrites it. Headless
//! source evidence only: no renderer, Windows/DirectX or performance claim.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_jumble::faces::{self, BaseFace, Geometry, Profile, Side, CORE, PROFILE};
use c600_jumble::model::{NP, POLES};
use c600_jumble::observe::{self, Batch, Rows, Scope};
use c600_jumble::q5::{det4, dot, Mat4, Vec4, Q5};
use c600_jumble::state::{State, Status};
use c600_jumble::twist::{Menu, Twist};
use c600_jumble::{codes, Context};
use c600_kernel::json::{self, Value};

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

const FIXTURE: &str = "research/jumbling/fixtures/wj-I_a.json";
const EVIDENCE: &str = "docs/progress/1.0/evidence/j10-exposed-face-size-profile.json";
const EN2: &str = "research/jumbling/differential/leads/results/meshes-2/meshes-summary.json";

struct Setup {
    ctx: Context,
    menu: Arc<Menu>,
    records: Vec<Value>,
    fixture_digest: String,
}

fn setup() -> &'static Setup {
    static SETUP: OnceLock<Setup> = OnceLock::new();
    SETUP.get_or_init(|| {
        let ctx = Context::load(&repository().join("assets")).expect("the jumbling model loads");
        let bytes = std::fs::read(repository().join(FIXTURE)).expect("the fixture");
        let fixture = json::parse(&bytes).expect("the I_a fixture is JSON");
        let menu = Menu::from_record(&ctx, fixture.get("menu").expect("a menu")).expect("the pinned I_a menu");
        let records = fixture.get("journal").and_then(|j| j.get("records")).and_then(Value::as_array);
        Setup {
            ctx,
            menu: Arc::new(menu),
            records: records.expect("the journal's records").to_vec(),
            fixture_digest: c600_digest::sha256_hex(&bytes),
        }
    })
}

impl Setup {
    fn geometry(&self) -> &Geometry {
        faces::geometry(&self.ctx).expect("the exposed-face geometry certifies")
    }

    /// The state after the walk's first `n` records, every one admissible.
    fn after(&self, n: usize) -> State {
        let mut state = State::new(Some(self.menu.clone()));
        for (i, record) in self.records[..n].iter().enumerate() {
            let twist = Twist::from_record(&self.ctx, record).expect("a menu record");
            let outcome = state.apply(&self.ctx, &twist, None, &[]).expect("no engine error");
            assert_eq!(outcome.status, Status::Admissible, "record {i}");
        }
        state
    }
}

fn text(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn sub(a: &Vec4, b: &Vec4) -> Vec4 {
    std::array::from_fn(|j| a[j].sub(&b[j]))
}

fn key(v: &Vec4) -> String {
    format!("{v:?}")
}

/// The outward normal of a region on a cut face of this side.
fn outward(ctx: &Context, pole: usize, side: Side) -> Vec4 {
    let n = &ctx.model.poles[pole];
    match side {
        Side::In => std::array::from_fn(|j| n[j].neg()),
        Side::Out => n.clone(),
    }
}

/// The invariants of one face with these exact vertices, recomputed without the builder: every vertex on the cut
/// hyperplane, every edge in exactly two 2-faces, V − E + F = 2, 2V − 4 triangles, and every fan triangle and every
/// other vertex of its 2-face strictly to the left seen from outside the region.
fn check_face(ctx: &Context, face: &BaseFace, pole: usize, side: Side, verts: &[Vec4]) {
    let at = format!("orbit {}, pole {pole}", face.orbit);
    let n = &ctx.model.poles[pole];
    for &v in &face.vertices {
        assert_eq!(dot(n, &verts[v as usize]), ctx.model.kappa, "{at}: a vertex off the cut hyperplane");
    }
    let mut edges: BTreeMap<(u16, u16), u32> = BTreeMap::new();
    for c in &face.polygons {
        assert!(c.len() >= 3, "{at}");
        for k in 0..c.len() {
            let (a, b) = (c[k], c[(k + 1) % c.len()]);
            assert!(face.vertices.binary_search(&a).is_ok(), "{at}: a cycle leaves the face");
            *edges.entry((a.min(b), a.max(b))).or_default() += 1;
        }
    }
    assert!(edges.values().all(|&n| n == 2), "{at}: an edge outside exactly two 2-faces");
    let (v, e, f) = (face.vertices.len() as i64, edges.len() as i64, face.polygons.len() as i64);
    assert_eq!(v - e + f, 2, "{at}: Euler");
    assert_eq!(face.fan().len() as i64, 2 * v - 4, "{at}: triangles");
    let m = Q5::int(v);
    let mut sum: Vec4 = std::array::from_fn(|_| Q5::zero());
    for &i in &face.vertices {
        sum = std::array::from_fn(|j| sum[j].add(&verts[i as usize][j]));
    }
    let normal = outward(ctx, pole, side);
    let sign = |a: &Vec4, b: &Vec4, w: &Vec4| {
        let x: Vec4 = std::array::from_fn(|j| m.mul(&a[j]).sub(&sum[j]));
        let rows: Mat4 = [sub(b, a), sub(w, a), x, normal.clone()];
        det4(&rows).sign()
    };
    for [a, b, c] in face.fan() {
        let (a, b, c) = (&verts[a as usize], &verts[b as usize], &verts[c as usize]);
        assert_eq!(sign(a, b, c), 1, "{at}: a fan triangle is not positively oriented");
    }
    for c in &face.polygons {
        for k in 0..c.len() {
            let (a, b) = (&verts[c[k] as usize], &verts[c[(k + 1) % c.len()] as usize]);
            for (w, &x) in c.iter().enumerate() {
                if w != k && w != (k + 1) % c.len() {
                    assert_eq!(sign(a, b, &verts[x as usize]), 1, "{at}: a 2-face is not strictly convex");
                }
            }
        }
    }
}

/// Check 1 to 5 recomputed from the returned base table: every face's invariants, every representative's faces on
/// the side its signature names, and each orbit's cut faces against its region's vertices.
#[test]
fn base_table_invariants_hold_without_the_builder() {
    let s = setup();
    let g = s.geometry();
    assert_eq!(g.table.faces.len(), 235);
    for face in &g.table.faces {
        let o = face.orbit as usize;
        let rep = s.ctx.regions.reps[o] as usize;
        let verts = &s.ctx.regions.rep_vertices[o];
        assert_eq!(face.side == Side::In, s.ctx.model.in_sig(rep, face.pole as usize));
        check_face(&s.ctx, face, face.pole as usize, face.side, verts);
        // The whole tight set: no other region vertex lies on the hyperplane, and the region is on its side.
        let n = &s.ctx.model.poles[face.pole as usize];
        for (i, v) in verts.iter().enumerate() {
            let d = dot(n, v).sub(&s.ctx.model.kappa).sign();
            assert_eq!(d == 0, face.vertices.binary_search(&(i as u16)).is_ok(), "orbit {o}, pole {}", face.pole);
            assert!(if face.side == Side::In { d >= 0 } else { d <= 0 });
        }
    }
    assert_eq!(g.digest, c600_digest::sha256_hex(&g.canonical));
    assert_eq!(faces::certify(&s.ctx, &g.table), Ok(g.profile));
}

/// EH-05: the measured profile is the admitted one, and its counts are EN2's recorded reference counts.
#[test]
fn profile_matches_en2_and_the_admitted_profile() {
    let s = setup();
    let g = s.geometry();
    assert_eq!(g.profile, PROFILE, "the measured profile is the admitted profile");
    let en2 = json::parse(&std::fs::read(repository().join(EN2)).expect("EN2's summary")).expect("JSON");
    let ours = g.profile.to_json();
    for scope in ["base", "full"] {
        for part in ["in", "out", "cut", "host"] {
            for count in ["faces3", "vertices", "faces2", "triangles"] {
                let want = en2.get(scope).and_then(|v| v.get(part)).and_then(|v| v.get(count)).and_then(Value::as_i64);
                let have = ours.get(scope).and_then(|v| v.get(part)).and_then(|v| v.get(count)).and_then(Value::as_i64);
                assert_eq!(have, want, "{scope}.{part}.{count}");
            }
        }
    }
    let bytes = |v: &Value, k: &str| v.get(k).and_then(Value::as_i64);
    let mesh = en2.get("full_mesh").expect("full_mesh");
    assert_eq!(Some(g.profile.soup_bytes() as i64), bytes(mesh, "cut_bytes"));
    let base_mesh = en2.get("base_mesh").expect("base_mesh");
    assert_eq!(Some(g.profile.base_soup_bytes() as i64), bytes(base_mesh, "cut_bytes"));
    let sizes: Vec<i64> =
        en2.get("orbit_sizes").and_then(Value::as_array).expect("sizes").iter().filter_map(Value::as_i64).collect();
    let mut ours_sizes = vec![0i64; s.ctx.regions.reps.len()];
    for p in 0..NP {
        ours_sizes[s.ctx.regions.orbit_of[p] as usize] += 1;
    }
    assert_eq!(ours_sizes, sizes, "the same orbits in the same order");
}

/// Transported faces of sampled pieces (one per orbit, a few fixed IDs): the face of piece p on pole r(e) has the
/// exact vertices of M5's region of p, satisfies every invariant, and its cell's other chamber has the opposite face on
/// the same pole with the same vertices.
#[test]
fn transported_faces_match_the_regions() {
    let s = setup();
    let (ctx, g) = (&s.ctx, s.geometry());
    let mut sample: Vec<usize> = vec![0, 1, 433, 50_000, 123_456, NP - 1];
    for o in 0..ctx.regions.reps.len() {
        sample.push((0..NP).rev().find(|&p| ctx.regions.orbit_of[p] as usize == o).expect("a piece"));
    }
    for p in sample {
        let verts = ctx.regions.vertices(p, &ctx.kplus, &ctx.model);
        for face in g.piece_faces(ctx, p).expect("the faces of a piece") {
            let pole = face.pole as usize;
            assert_eq!(face.side == Side::In, ctx.model.in_sig(p, pole), "piece {p}, pole {pole}");
            check_face(ctx, face.base, pole, face.side, &verts);
            let mut sig: Vec<u16> = ctx.model.signature(p).to_vec();
            let other = match face.side {
                Side::In => {
                    assert_eq!(face.cell[1], p as u32);
                    sig.retain(|&e| e as usize != pole);
                    face.cell[0]
                }
                Side::Out => {
                    assert_eq!(face.cell[0], p as u32);
                    sig.push(pole as u16);
                    face.cell[1]
                }
            };
            sig.sort_unstable();
            if other == CORE {
                assert!(sig.is_empty(), "piece {p}, pole {pole}: the core is the empty signature");
                continue;
            }
            let mut theirs = ctx.model.signature(other as usize).to_vec();
            theirs.sort_unstable();
            assert_eq!(theirs, sig, "piece {p}, pole {pole}: the cell's other chamber");
            let opposite = g.piece_faces(ctx, other as usize).expect("faces").into_iter().find(|f| f.pole == face.pole);
            let opposite = opposite.expect("the other chamber has a face on the cell");
            assert_ne!(opposite.side, face.side);
            assert_eq!(opposite.cell, face.cell);
            let their_verts = ctx.regions.vertices(other as usize, &ctx.kplus, &ctx.model);
            let mut a: Vec<String> = face.base.vertices.iter().map(|&v| key(&verts[v as usize])).collect();
            let mut b: Vec<String> = opposite.base.vertices.iter().map(|&v| key(&their_verts[v as usize])).collect();
            a.sort();
            b.sort();
            assert_eq!(a, b, "piece {p}, pole {pole}: the two sides of the cell");
        }
    }
}

/// The full table: 939,000 faces over the 177,120 pieces with the profile's counts; every cell off the core has one
/// `in` and one `out` face, the 600 core cells one `in` face, and every face's home pole is in its piece's signature
/// exactly when it is `in`.
#[test]
fn full_table_pairs_every_cell() {
    let s = setup();
    let (ctx, g) = (&s.ctx, s.geometry());
    let (mut faces, mut vertices, mut triangles) = (0u64, 0u64, 0u64);
    let mut cells: HashMap<(u32, u32, u16), u8> = HashMap::with_capacity(g.profile.cells as usize);
    for p in 0..NP {
        for face in g.piece_faces(ctx, p).expect("the faces of a piece") {
            assert!((face.pole as usize) < POLES);
            assert_eq!(face.side == Side::In, ctx.model.in_sig(p, face.pole as usize));
            faces += 1;
            vertices += face.base.vertices.len() as u64;
            triangles += face.base.triangles() as u64;
            let bit = if face.side == Side::In { 1 } else { 2 };
            let seen = cells.entry((face.cell[0], face.cell[1], face.pole)).or_default();
            assert_eq!(*seen & bit, 0, "a side of a cell twice");
            *seen |= bit;
        }
    }
    let cut = g.profile.full_cut();
    assert_eq!((faces, vertices, triangles), (cut.faces3, cut.vertices, cut.triangles));
    assert_eq!(cells.len() as u64, g.profile.cells);
    let core = cells.iter().filter(|(k, _)| k.0 == CORE).count() as u64;
    assert_eq!(core, g.profile.core_cells);
    assert!(cells.iter().all(|(k, &sides)| sides == if k.0 == CORE { 1 } else { 3 }));
}

fn refused(answer: &Value, missing: &[&str], state: &State, ctx: &Context) {
    assert_eq!(text(answer, "status").as_deref(), Some("refused"));
    assert_eq!(text(answer, "code").as_deref(), Some(codes::EXPOSED_GEOMETRY_UNAVAILABLE));
    assert_eq!(text(answer, "feature").as_deref(), Some(faces::FEATURE));
    assert_eq!(text(answer, "render_row").as_deref(), Some("J10"));
    let names: Vec<Value> = missing.iter().map(|m| Value::String((*m).to_owned())).collect();
    assert_eq!(answer.get("missing").and_then(Value::as_array), Some(names.as_slice()));
    assert!(text(answer, "failed").is_some_and(|f| !f.is_empty()), "the failed fact is named");
    assert_eq!(answer.get("draft"), Some(&Value::Null));
    for key in ["geometry", "profile", "mesh", "faces", "vertices", "size"] {
        assert!(answer.get(key).is_none(), "no fabricated `{key}`");
    }
    let source = answer.get("source").expect("the exact input reference");
    assert_eq!(text(source, "state_digest"), Some(state.digest(ctx)));
    assert_eq!(text(source, "model_id").as_deref(), Some(ctx.model.model_id.as_str()));
}

/// Certification refuses corrupted tables on the real model: a vertex off its plane, a dropped 2-face, a reversed
/// cycle, a dropped face (a cell with one side), a swapped side and an unsorted table. The answer for a table that
/// does not certify is the refusal naming `certified_geometry`, and nothing changes.
#[test]
fn corrupted_tables_are_refused() {
    let s = setup();
    let (ctx, g) = (&s.ctx, s.geometry());
    let corrupt = |f: &dyn Fn(&mut Vec<BaseFace>)| {
        let mut table = g.table.clone();
        f(&mut table.faces);
        faces::certify(ctx, &table)
    };
    let off_plane = corrupt(&|faces| {
        let face = &mut faces[0];
        let verts = &ctx.regions.rep_vertices[face.orbit as usize];
        let n = &ctx.model.poles[face.pole as usize];
        let off = (0..verts.len() as u16).find(|&v| dot(n, &verts[v as usize]) != ctx.model.kappa).expect("a vertex");
        face.vertices[0] = off;
        face.vertices.sort_unstable();
    });
    assert!(off_plane.is_err_and(|e| e.contains("off the cut hyperplane")), "a vertex off its plane");
    let dropped_polygon = corrupt(&|faces| {
        faces[3].polygons.remove(0);
    });
    assert!(dropped_polygon.is_err_and(|e| e.contains("2-faces")), "a dropped 2-face");
    let reversed = corrupt(&|faces| {
        let c = &mut faces[5].polygons[0];
        c[1..].reverse();
    });
    assert!(reversed.is_err_and(|e| e.contains("convex")), "a reversed cycle");
    let dropped_face = corrupt(&|faces| {
        faces.remove(7);
    });
    assert!(dropped_face.is_err_and(|e| e.contains("cut 3-faces")), "a cell missing one side");
    let swapped = corrupt(&|faces| {
        faces[9].side = if faces[9].side == Side::In { Side::Out } else { Side::In };
    });
    assert!(swapped.is_err_and(|e| e.contains("side")), "a swapped side");
    let unsorted = corrupt(&|faces| faces.swap(0, 1));
    assert!(unsorted.is_err_and(|e| e.contains("sorted")), "an unsorted table");

    let state = s.after(1);
    let before = state.record(ctx);
    let reason = corrupt(&|faces| faces.swap(0, 1)).expect_err("refused");
    let answer = faces::answer(ctx, &state, Err(&reason), Some(&PROFILE));
    refused(&answer, &[faces::MISSING_GEOMETRY], &state, ctx);
    assert_eq!(text(&answer, "failed"), Some(reason));
    let neither = faces::answer(ctx, &state, Err("no geometry"), None);
    refused(&neither, &faces::MISSING, &state, ctx);
    assert_eq!(state.record(ctx), before);
}

/// A certified geometry without an admitted profile, or with another profile, is refused naming `size_profile`.
#[test]
fn a_missing_or_different_profile_is_refused() {
    let s = setup();
    let (ctx, g) = (&s.ctx, s.geometry());
    let state = s.after(1);
    let before = state.record(ctx);
    refused(&faces::answer(ctx, &state, Ok(g), None), &[faces::MISSING_PROFILE], &state, ctx);
    for other in [
        Profile { cells: PROFILE.cells + 1, ..PROFILE },
        Profile { exact_bytes: PROFILE.exact_bytes + 1, ..PROFILE },
        Profile { full_out: Profile { ..PROFILE }.full_in, ..PROFILE },
    ] {
        let answer = faces::answer(ctx, &state, Ok(g), Some(&other));
        refused(&answer, &[faces::MISSING_PROFILE], &state, ctx);
        assert!(text(&answer, "failed").is_some_and(|f| f.contains(&other.id())));
    }
    assert_eq!(state.record(ctx), before);
}

/// The available answer binds the certified geometry to the state: its exact input reference, the format, the model,
/// the exact-source digest and the admitted profile with its ID. Another state is another input with the same
/// geometry, and no call changes the state.
#[test]
fn exposed_faces_are_available_and_change_nothing() {
    let s = setup();
    let (ctx, g) = (&s.ctx, s.geometry());
    let state = s.after(1);
    let before = state.record(ctx);
    let answer = faces::exposed_faces(ctx, &state);
    assert_eq!(text(&answer, "status").as_deref(), Some("available"));
    assert_eq!(text(&answer, "feature").as_deref(), Some(faces::FEATURE));
    assert_eq!(text(&answer, "render_row").as_deref(), Some("J10"));
    assert!(answer.get("code").is_none() && answer.get("missing").is_none());
    let source = answer.get("source").expect("the exact input reference");
    assert_eq!(text(source, "state_digest"), Some(state.digest(ctx)));
    assert_eq!(text(source, "menu_id").as_deref(), Some(s.menu.identity.as_str()));
    let geometry = answer.get("geometry").expect("the geometry reference");
    assert_eq!(text(geometry, "format").as_deref(), Some(faces::FORMAT));
    assert_eq!(text(geometry, "model_id").as_deref(), Some(ctx.model.model_id.as_str()));
    assert_eq!(text(geometry, "digest"), Some(c600_digest::sha256_hex(&g.canonical)));
    assert_eq!(geometry.get("base_faces").and_then(Value::as_i64), Some(235));
    assert_eq!(text(&answer, "profile_id"), Some(PROFILE.id()));
    assert_eq!(answer.get("profile"), Some(&PROFILE.to_json()));
    let solved = faces::exposed_faces(ctx, &State::new(Some(s.menu.clone())));
    assert_ne!(solved.get("source"), answer.get("source"));
    assert_eq!(solved.get("geometry"), answer.get("geometry"));
    assert_eq!(state.record(ctx), before);
}

/// The size profile's evidence file: the admitted profile, its ID and the geometry it measures, and how many
/// exposed faces (mismatched cut cells, one held side each) the fixture walk's states show. `C600_WRITE_EVIDENCE=1`
/// writes the file instead of comparing.
#[test]
fn size_profile_evidence_is_current() {
    let s = setup();
    let (ctx, g) = (&s.ctx, s.geometry());
    let whole = Batch { chunk: 1 << 20, budget: None, cancel_after: None };
    let mut states = Vec::new();
    for (label, n) in [("solved", 0usize), ("first record", 1), ("three records", 3), ("thirty records", 30)] {
        let mut state = s.after(n);
        let facts = observe::seam_facts(ctx, &mut state, Scope::All, &whole, Rows::None, None).expect("seam facts");
        assert_eq!(facts.get("complete"), Some(&Value::Bool(true)));
        let totals = facts.get("totals").expect("totals");
        let n_total = totals.get("total_N").and_then(Value::as_i64).expect("total N");
        states.push(json::object([
            ("label", Value::String(label.to_owned())),
            ("records", Value::Int(n as i64)),
            ("state_digest", Value::String(state.digest(ctx))),
            ("exposed_faces", Value::Int(n_total)),
            ("domains", facts.get("domains").cloned().unwrap_or(Value::Null)),
        ]));
    }
    let evidence: Vec<(&str, Value)> = vec![
        ("evidence_kind", Value::String("source/fixture measurement: exact geometry derived from assets/ by c600-jumble and certified in exact arithmetic, measured headless; no renderer, Windows/DirectX or performance evidence".to_owned())),
        ("ceilings", Value::String(format!("the payload's RM.11 input_bytes are {} (binary32_instanced); no numeric RM.12/RM.20 ceiling is fixed to check them against: RM.11 asks for a LimitProfile only before a feature is advertised as supported, J10 is not (RM.15), and RM.13 leaves production ceilings to be pinned before enablement; c600-display's jumble::admit_exposed_faces applies RM.12 to a supplied ceiling", PROFILE.instanced_bytes()))),
        ("feature", Value::String("J10".to_owned())),
        ("format", Value::String(faces::FORMAT.to_owned())),
        ("model_id", Value::String(ctx.model.model_id.clone())),
        ("geometry_digest", Value::String(g.digest.clone())),
        ("profile_id", Value::String(g.profile.id())),
        ("profile", g.profile.to_json()),
        ("reference", Value::String(format!("EH-05, EN2's {EN2}: equal counts, checked by rust/c600-jumble/tests/faces.rs"))),
        ("states_note", Value::String("exposed_faces is total N, the mismatched cut cells of every domain at the state; each has one held side, whose face a renderer draws".to_owned())),
        ("states", Value::Array(states)),
        ("fixture", Value::String(FIXTURE.to_owned())),
        ("fixture_sha256", Value::String(s.fixture_digest.clone())),
        ("producer", Value::String(format!("c600-jumble {} (faces::geometry)", env!("CARGO_PKG_VERSION")))),
        ("command", Value::String("cargo test --manifest-path rust/Cargo.toml --locked --offline -p c600-jumble --test faces size_profile_evidence_is_current (C600_WRITE_EVIDENCE=1 writes)".to_owned())),
    ];
    let mut out = String::from("{\n");
    for (i, (k, v)) in evidence.iter().enumerate() {
        out.push_str(&format!(
            "  {}: {}{}\n",
            Value::String((*k).to_owned()).canonical(),
            v.canonical(),
            if i + 1 < evidence.len() { "," } else { "" }
        ));
    }
    out.push_str("}\n");
    let path = repository().join(EVIDENCE);
    if std::env::var("C600_WRITE_EVIDENCE").as_deref() == Ok("1") {
        std::fs::create_dir_all(path.parent().expect("a folder")).expect("the evidence folder");
        std::fs::write(&path, out.as_bytes()).expect("the evidence file");
    }
    let recorded = std::fs::read_to_string(&path).expect("the evidence file exists");
    assert_eq!(
        recorded, out,
        "the recorded size profile is current; rerun with C600_WRITE_EVIDENCE=1 after a reviewed change"
    );
    let parsed = json::parse(recorded.as_bytes()).expect("the evidence is JSON");
    assert_eq!(parsed.get("profile"), Some(&PROFILE.to_json()));
}
