//! Shared setup for stage 18: the archive's paths and pins, its records' inputs read in place, the check entry, and the
//! synthetic records. A synthetic record's claims are computed here with direct engine calls (the engine's own menu
//! transport), apart from the sub-checkers, so a fixture is never produced by the code it tests.

#![allow(dead_code)]

use std::sync::{Arc, OnceLock};

use c600_checkers::archive::s18;
use c600_checkers::gate::Fixture;
use c600_checkers::{check, CertificateRef, CheckReport, CheckerKind, Input};
use c600_jumble::state::{State, Status};
use c600_jumble::twist::Menu;
use c600_kernel::json::Value;

use crate::common::*;

pub const S18: &str = "research/jumbling/restoration/18-tail-completion/results";
pub const T6: &str = "research/jumbling/restoration/18-tail-completion/results/d1-tail";
pub const RULE: &str = "research/jumbling/restoration/18-tail-completion/results/rule-stop";
pub const RUN: &str = "research/jumbling/restoration/18-tail-completion/results/lattice-d1/run";

/// The stop table both chains start from (stage 13, D1's run 4, table 00113; outside the Git-blob manifest, pinned
/// here), and the table before it, a wrong start.
pub const STOP_TABLE: &str =
    "research/jumbling/restoration/13-tutorial-runner/runs/dev/d1-acc-s7101/out4/tables/00113.json";
pub const STOP_TABLE_SHA256: &str = "c11c890580554a920a6466df0a736553a0e56bd0882b640aa401f6acd16536f6";
pub const STOP_DIGEST: &str = "966dc8a96f7bf40b4c0361d7a26d9a3efbf62e6c59c1a6677cf5de9f8aefe661";
pub const BEFORE_STOP_TABLE: &str =
    "research/jumbling/restoration/13-tutorial-runner/runs/dev/d1-acc-s7101/out4/tables/00112.json";
pub const BEFORE_STOP_TABLE_SHA256: &str = "b574ce8951b20170f9911747968d7e3056d35fc1a5e5b083e42928752d4862bf";

/// T6's end, stored as the lattice run's start table.
pub const T6_END_TABLE: &str =
    "research/jumbling/restoration/18-tail-completion/results/lattice-d1/run/start-table.json";
pub const T6_END_TABLE_SHA256: &str = "f6af3dc848493dc4efc0717baea666a325af35e53461ecf655d35f1fbf9d9b9a";
pub const T6_END_DIGEST: &str = "c75c8dde2ff850d247c07722cd1059cb50c33ada4657632fed90f16704bfffc0";

pub const T6_STEPS_SHA256: &str = "1b7f295fdc066c29f326cc162eeaac939fd6c07ed4f910cb53bbece716b1a3a8";
pub const RULE_STEPS_SHA256: &str = "15579b4dcda043b404ce52160310b09eb73380cdafebd36264e5ca976cfd21f7";
pub const RUN_SHA256: &str = "829c9c52fe202079eb27951ef9ddd78fd432c9eff4edd7f663b78a438db11416";

pub fn kind(sub: &'static c600_checkers::archive::SubChecker) -> CheckerKind {
    CheckerKind::Archive(sub.id())
}

pub fn expansion() -> CheckerKind {
    kind(&s18::expansion::SUBCHECKER)
}

pub fn composed() -> CheckerKind {
    kind(&s18::composed::SUBCHECKER)
}

pub fn tail_replay() -> CheckerKind {
    kind(&s18::tail_replay::SUBCHECKER)
}

pub fn lattice_run() -> CheckerKind {
    kind(&s18::lattice_run::SUBCHECKER)
}

pub fn lattice_replay() -> CheckerKind {
    kind(&s18::lattice_replay::SUBCHECKER)
}

/// A pinned repository file, read in place.
pub fn pinned(relative: &str, sha256: &str) -> Vec<u8> {
    let bytes = read(relative);
    assert_eq!(sha(&bytes), sha256, "{relative} is not the pinned file");
    bytes
}

/// Checks `certificate` with owned inputs, stated to have its own digest.
pub fn check_owned(kind: CheckerKind, certificate: &[u8], inputs: &[(String, Vec<u8>)]) -> CheckReport {
    let inputs: Vec<Input> = inputs.iter().map(|(role, bytes)| Input { role, bytes }).collect();
    let digest = sha(certificate);
    check(&engines(), &CertificateRef { digest: &digest, bytes: certificate, inputs: &inputs }, kind)
        .expect("a checker")
}

/// The inputs of a `steps.json` record read in place: the menu, the stop table, every step's files and, with `end`,
/// T6's stored end table.
pub fn chain_inputs(dir: &str, end: bool) -> Vec<(String, Vec<u8>)> {
    let record = parse(&read(&format!("{dir}/steps.json")));
    let mut inputs =
        vec![("menu".to_owned(), menu_record("I_a")), ("start".to_owned(), pinned(STOP_TABLE, STOP_TABLE_SHA256))];
    for step in record.get("steps").and_then(Value::as_array).unwrap() {
        let name = step.get("dir").and_then(Value::as_str).unwrap();
        let mut files = vec!["w.json", "v.json.gz", "winv.json", "unit.json"];
        if step.get("X").and_then(Value::as_i64).unwrap() > 0 {
            files.insert(0, "x.json.gz");
        }
        for file in files {
            inputs.push((format!("{name}/{file}"), read(&format!("{dir}/{name}/{file}"))));
        }
    }
    if end {
        inputs.push(("end".to_owned(), pinned(T6_END_TABLE, T6_END_TABLE_SHA256)));
    }
    inputs
}

/// The inputs of the lattice run record read in place: the menu, every member it names.
pub fn run_inputs() -> Vec<(String, Vec<u8>)> {
    let record = parse(&read(&format!("{RUN}/run.json")));
    let mut roles =
        vec!["start-table.json".to_owned(), "tables/bulk-end.json".to_owned(), "lattice/report.json".to_owned()];
    for step in record.get("steps").and_then(Value::as_array).unwrap() {
        roles.push(step.get("word").and_then(Value::as_str).unwrap().to_owned());
        roles.push(step.get("tree").and_then(|t| t.get("file")).and_then(Value::as_str).unwrap().to_owned());
        if let Some(table) = step.get("table").and_then(Value::as_str) {
            roles.push(table.to_owned());
        }
    }
    let mut inputs = vec![("menu".to_owned(), menu_record("I_a"))];
    inputs.extend(roles.into_iter().map(|role| {
        let bytes = read(&format!("{RUN}/{role}"));
        (role, bytes)
    }));
    inputs
}

/// Replaces one input's bytes.
pub fn replace(inputs: &[(String, Vec<u8>)], role: &str, bytes: Vec<u8>) -> Vec<(String, Vec<u8>)> {
    let mut found = false;
    let out = inputs
        .iter()
        .map(|(r, b)| {
            if r == role {
                found = true;
                (r.clone(), bytes.clone())
            } else {
                (r.clone(), b.clone())
            }
        })
        .collect();
    assert!(found, "no input {role}");
    out
}

/// A mutation-gate fixture with owned inputs.
pub fn gate_fixture(
    name: &str,
    kind: CheckerKind,
    certificate: Vec<u8>,
    inputs: Vec<(String, Vec<u8>)>,
    claims: &[&str],
    skip: &[&str],
    wrong_start: (&str, Vec<u8>),
) -> Fixture {
    Fixture {
        name: name.to_owned(),
        kind,
        certificate,
        inputs,
        claims: claims.iter().map(|c| (*c).to_owned()).collect(),
        skip: skip.iter().map(|c| (*c).to_owned()).collect(),
        wrong_start: Some((wrong_start.0.to_owned(), wrong_start.1)),
        optional: Vec::new(),
    }
}

/// A gzip member of `data` in stored DEFLATE blocks (RFC 1951 section 3.2.4, RFC 1952).
pub fn gzip_stored(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0xff];
    let chunks: Vec<&[u8]> = if data.is_empty() { vec![&[][..]] } else { data.chunks(65_535).collect() };
    for (n, chunk) in chunks.iter().enumerate() {
        out.push(u8::from(n + 1 == chunks.len()));
        let len = chunk.len() as u16;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(chunk);
    }
    out.extend_from_slice(&c600_kernel::archive::crc32(data).to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out
}

/// A list of letters as `json.dumps` writes it.
pub fn dumps(letters: &[(usize, usize)]) -> String {
    let inner: Vec<String> = letters.iter().map(|(c, i)| format!("[{c}, {i}]")).collect();
    format!("[{}]", inner.join(", "))
}

/// The inverse item of every I_a item, from the exact rotations.
pub fn inverse_items(menu: &Menu) -> Vec<usize> {
    (0..menu.items.len())
        .map(|i| {
            let t = c600_jumble::q5::transpose(&menu.items[i].1);
            menu.items.iter().position(|(_, m)| *m == t).unwrap()
        })
        .collect()
}

/// A research pose table (`poses`, `pose_id`) as an engine state, built here from the engine's exact matrices.
pub fn table_state(bytes: &[u8], menu: &Arc<Menu>) -> State {
    use c600_jumble::state::Pose;
    let doc = parse(bytes);
    let ctx = jumble();
    let table: Vec<Pose> = doc
        .get("poses")
        .and_then(Value::as_array)
        .unwrap()
        .iter()
        .map(|m| {
            let m = c600_jumble::q5::matrix_from_json(m).unwrap();
            match ctx.kplus.index_of_matrix(&m, &ctx.model) {
                Some(k) => Pose::K(k),
                None => Pose::M(Arc::new(m)),
            }
        })
        .collect();
    let per_piece: Vec<Pose> = doc
        .get("pose_id")
        .and_then(Value::as_array)
        .unwrap()
        .iter()
        .map(|id| table[id.as_i64().unwrap() as usize].clone())
        .collect();
    c600_jumble::levels::state_of(ctx, Some(menu.clone()), &per_piece).unwrap()
}

/// Applies a letter through the engine's own menu transport; it must be admissible. Returns the inside pieces.
pub fn apply(state: &mut State, menu: &Menu, (cap, item): (usize, usize)) -> Vec<u32> {
    let outcome = state.apply(jumble(), &twist(menu, cap, item), None, &[]).expect("no engine error");
    assert_eq!(outcome.status, Status::Admissible, "[{cap}, {item}]");
    outcome.inside
}

// ------------------------------------------------------------------------------------------- the synthetic records

/// The letters of the synthetic records: two jumbles at piece-disjoint caps c and c′ (their contents are the two
/// units), a retained letter W at a third cap and two retained letters for V′ at a fourth and a fifth, all five caps
/// pairwise piece-disjoint, so W commutes with V′ and V′ keeps every off piece.
pub struct Letters {
    pub jumble_a: (usize, usize),
    pub jumble_b: (usize, usize),
    pub w: (usize, usize),
    pub v: [(usize, usize); 2],
    pub inverse: Vec<usize>,
}

pub fn letters() -> &'static Letters {
    static LETTERS: OnceLock<Letters> = OnceLock::new();
    LETTERS.get_or_init(|| {
        let menu = menu("I_a");
        let solved = State::new(Some(menu.clone()));
        let mut taken: Vec<Vec<u32>> = Vec::new();
        let mut pick = |retained: bool| -> (usize, usize) {
            for cap in 0..600 {
                let twists = menu.for_cap(jumble(), cap).unwrap();
                let Some(item) = (0..twists.len()).find(|&i| match twists[i].kidx {
                    Some(k) => retained && k != 0,
                    None => !retained,
                }) else {
                    continue;
                };
                let mut state = solved.clone();
                let outcome = state.apply(jumble(), &twists[item], None, &[]).unwrap();
                if outcome.status != Status::Admissible {
                    continue;
                }
                let inside = outcome.inside;
                if taken.iter().all(|other| other.iter().all(|p| inside.binary_search(p).is_err())) {
                    taken.push(inside);
                    return (cap, item);
                }
            }
            panic!("no cap left for the synthetic record");
        };
        let jumble_a = pick(false);
        let jumble_b = pick(false);
        let w = pick(true);
        let v = [pick(true), pick(true)];
        Letters { jumble_a, jumble_b, w, v, inverse: inverse_items(&menu) }
    })
}

/// A synthetic `steps.json` record and its inputs.
pub struct Synthetic {
    pub record: Vec<u8>,
    pub inputs: Vec<(String, Vec<u8>)>,
    /// A wrong start: only the first unit's cap jumbled.
    pub wrong_start: Vec<u8>,
    /// The state each step ends at.
    pub ends: Vec<String>,
    /// A table of the chain's end, for the optional `end` input (not among `inputs`).
    pub end_table: Vec<u8>,
}

/// One step's words.
struct StepSpec {
    dir: &'static str,
    x: Vec<(usize, usize)>,
    unit: Vec<u32>,
}

/// Two whole-unit steps: from Z = solved·(c′, j′)·(c, j), step 1 lands the content of c with X = (c, j⁻¹) and step 2
/// the content of c′ with X = (c′, j′⁻¹), each followed by W·V′·W⁻¹ (and V′⁻¹ in the four-segment form).
pub fn synthetic(four: bool) -> Synthetic {
    let l = letters();
    let menu = menu("I_a");
    let inv = |(c, i): (usize, usize)| (c, l.inverse[i]);
    let mut state = State::new(Some(menu.clone()));
    let b = apply(&mut state, &menu, l.jumble_b);
    let mut wrong = State::new(Some(menu.clone()));
    apply(&mut wrong, &menu, l.jumble_a);
    let a = apply(&mut state, &menu, l.jumble_a);
    let start = state.clone();
    let specs = [
        StepSpec { dir: "01-a", x: vec![inv(l.jumble_a)], unit: a },
        StepSpec { dir: "02-b", x: vec![inv(l.jumble_b)], unit: b },
    ];
    let copies = if four { 2 } else { 1 };
    let mut inputs = vec![("menu".to_owned(), menu_record("I_a")), ("start".to_owned(), state_record(&start))];
    let mut steps = Vec::new();
    let mut ends = Vec::new();
    for spec in specs {
        let begin = state.digest(jumble());
        let off_before = state.off_lattice_count() as i64;
        for &letter in &spec.x {
            apply(&mut state, &menu, letter);
        }
        let after_x = state.digest(jumble());
        apply(&mut state, &menu, l.w);
        let after_w = state.digest(jumble());
        for &letter in &l.v {
            apply(&mut state, &menu, letter);
        }
        apply(&mut state, &menu, inv(l.w));
        if four {
            for &letter in l.v.iter().rev() {
                apply(&mut state, &menu, inv(letter));
            }
        }
        let end = state.digest(jumble());
        let file = |text: String| text.into_bytes();
        inputs.push((
            format!("{}/x.json.gz", spec.dir),
            gzip_stored(&file(format!(
                "{{\"word\": {}, \"input_digest\": \"{begin}\", \"output_digest\": \"{after_x}\"}}\n",
                dumps(&spec.x)
            ))),
        ));
        inputs.push((
            format!("{}/w.json", spec.dir),
            file(format!(
                "{{\"word\": {}, \"input_digest\": \"{after_x}\", \"output_digest\": \"{after_w}\"}}\n",
                dumps(&[l.w])
            )),
        ));
        inputs.push((
            format!("{}/v.json.gz", spec.dir),
            gzip_stored(&file(format!("{{\"word\": {}, \"input_digest\": \"{after_w}\"}}\n", dumps(&l.v)))),
        ));
        let core = format!("{{\"P\": [], \"Q\": {}}}", dumps(&[inv(l.w)]));
        inputs.push((format!("{}/winv.json", spec.dir), file(core.clone())));
        let mut unit = spec.unit.clone();
        unit.sort_unstable();
        let pieces: Vec<String> = unit.iter().map(u32::to_string).collect();
        inputs.push((format!("{}/unit.json", spec.dir), file(format!("{{\"pieces\": [{}]}}", pieces.join(", ")))));
        let off_after = state.off_lattice_count() as i64;
        steps.push(obj(vec![
            ("dir", text(spec.dir)),
            ("grip", int(l.w.0 as i64)),
            ("item", int(l.w.1 as i64)),
            ("start_digest", text(&begin)),
            ("end_digest", text(&end)),
            ("off_before", int(off_before)),
            ("off_after", int(off_after)),
            ("X", int(spec.x.len() as i64)),
            ("V", int(l.v.len() as i64)),
            ("letters", int(spec.x.len() as i64 + 2 + copies * l.v.len() as i64)),
            ("unit_size", int(unit.len() as i64)),
            ("x_sha256", text(&sha(dumps(&spec.x).as_bytes()))),
            ("w_sha256", text(&sha(dumps(&[l.w]).as_bytes()))),
            ("v_sha256", text(&sha(dumps(&l.v).as_bytes()))),
            ("winv_sha256", text(&sha(core.as_bytes()))),
        ]));
        ends.push(end);
    }
    let mut fields = vec![("start_digest", text(&start.digest(jumble()))), ("steps", Value::Array(steps))];
    if !four {
        fields.push(("form", text("three-segment")));
    }
    Synthetic {
        record: obj(fields).canonical().into_bytes(),
        inputs,
        wrong_start: state_record(&wrong),
        ends,
        end_table: state_record(&state),
    }
}

/// The I_a menu, shared.
pub fn ia() -> Arc<Menu> {
    menu("I_a")
}

// ------------------------------------------------------------------------------------- structural mutations of a record

/// One step of a claim path (`a.b[2].c`).
pub enum Seg {
    Key(String),
    Index(usize),
}

/// A claim path's steps; the root is the empty path.
pub fn segments(path: &str) -> Vec<Seg> {
    let mut out = Vec::new();
    let mut rest = path;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix('[') {
            let close = after.find(']').expect("a closed index");
            out.push(Seg::Index(after[..close].parse().expect("an index")));
            rest = &after[close + 1..];
        } else {
            let key = rest.strip_prefix('.').unwrap_or(rest);
            let end = key.find(['.', '[']).unwrap_or(key.len());
            out.push(Seg::Key(key[..end].to_owned()));
            rest = &key[end..];
        }
    }
    out
}

/// The node at a path.
pub fn node_mut<'v>(mut value: &'v mut Value, path: &[Seg]) -> &'v mut Value {
    for seg in path {
        value = match (seg, value) {
            (Seg::Key(key), Value::Object(map)) => map.get_mut(key).expect("a member"),
            (Seg::Index(i), Value::Array(items)) => &mut items[*i],
            _ => panic!("no such node"),
        };
    }
    value
}

/// Sets the node at a claim path.
pub fn set_at(doc: &mut Value, path: &str, value: Value) {
    *node_mut(doc, &segments(path)) = value;
}

/// Every node of a document (objects, lists and scalars) by claim path, the root first as the empty path.
fn nodes(value: &Value, path: String, out: &mut Vec<String>) {
    out.push(path.clone());
    match value {
        Value::Object(map) => {
            for (key, member) in map {
                nodes(member, if path.is_empty() { key.clone() } else { format!("{path}.{key}") }, out);
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                nodes(item, format!("{path}[{index}]"), out);
            }
        }
        _ => {}
    }
}

fn under(path: &str, prefix: &str) -> bool {
    path == prefix || (path.starts_with(prefix) && matches!(path.as_bytes().get(prefix.len()), Some(b'.') | Some(b'[')))
}

/// The structural mutations of the line's mutation gate, applied here to this stage's fixtures: at every node at or
/// under the claim paths (every node of the record when `claims` is empty) and not under `skip`, a member deleted, a
/// member renamed, a list element dropped, an unknown member added to an object and a non-empty list emptied.
pub fn structural_mutants(doc: &Value, claims: &[&str], skip: &[&str]) -> Vec<(String, Value)> {
    let mut paths = Vec::new();
    nodes(doc, String::new(), &mut paths);
    let mut out = Vec::new();
    for path in paths {
        let selected =
            if claims.is_empty() { true } else { !path.is_empty() && claims.iter().any(|c| under(&path, c)) };
        if !selected || skip.iter().any(|s| under(&path, s)) {
            continue;
        }
        let segs = segments(&path);
        let mut mutated = doc.clone();
        let label = match node_mut(&mut mutated, &segs) {
            Value::Object(map) => {
                map.insert("unknown~".to_owned(), Value::Int(0));
                Some(format!("unknown member added at {path:?}"))
            }
            Value::Array(items) if !items.is_empty() => {
                items.clear();
                Some(format!("{path} emptied"))
            }
            _ => None,
        };
        if let Some(label) = label {
            out.push((label, mutated));
        }
        if let Some((last, parent)) = segs.split_last() {
            match last {
                Seg::Key(key) => {
                    for rename in [false, true] {
                        let mut mutated = doc.clone();
                        if let Value::Object(map) = node_mut(&mut mutated, parent) {
                            let value = map.remove(key).expect("a member");
                            if rename {
                                map.insert(format!("{key}~"), value);
                            }
                        }
                        out.push((format!("{path} {}", if rename { "renamed" } else { "deleted" }), mutated));
                    }
                }
                Seg::Index(index) => {
                    let mut mutated = doc.clone();
                    if let Value::Array(items) = node_mut(&mut mutated, parent) {
                        items.remove(*index);
                    }
                    out.push((format!("{path} dropped"), mutated));
                }
            }
        }
    }
    out
}

/// Checks every structural mutant of a passing certificate: each must fail. Returns the number of mutants.
pub fn structural_gate(
    name: &str,
    kind: CheckerKind,
    certificate: &[u8],
    inputs: &[(String, Vec<u8>)],
    claims: &[&str],
    skip: &[&str],
) -> usize {
    let mutants = structural_mutants(&parse(certificate), claims, skip);
    let mut problems = Vec::new();
    for (mutation, doc) in &mutants {
        let report = check_owned(kind, doc.canonical().as_bytes(), inputs);
        if !report.verdict.is_fail() {
            problems.push(format!("{mutation}: {:?}", report.verdict));
        }
    }
    assert!(
        problems.is_empty(),
        "{name}: {} of {} structural mutations did not fail:\n{}",
        problems.len(),
        mutants.len(),
        problems.join("\n")
    );
    mutants.len()
}
