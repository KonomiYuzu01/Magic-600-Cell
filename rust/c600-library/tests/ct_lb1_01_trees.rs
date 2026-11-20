//! CT-LB1-01, the quaternion factor and tree parts (LB1.8's row "Quaternion and icosian algebra"; chapter 03's test
//! table): quaternion factor reconstruction over Q(√5) and the tree outputs agree with the pinned research reference.
//!
//! The entries are language code, the library's module `trees` on the IR draft's `Q5` with LP2's `q5_parts` and
//! `q5_of`, run on `c600-run` with no host. Their reference is `research/jumbling/theory/trees.py` (with its
//! `research/jumbling/exact.py`), pinned by SHA-256: `tests/fixtures/trees_reference.py` wrote trees.py's exact answers
//! to `tests/fixtures/trees-reference.json`, which records the two pins, and this test checks that the pinned files are
//! still those bytes. Every answer must equal the reference exactly: factors coefficient for coefficient, not up to
//! scale. A pass is source and fixture evidence.

mod common;

use std::collections::BTreeSet;
use std::sync::Arc;

use c600_check::{check, separation, Policy, Separation};
use c600_digest::sha256_hex;
use c600_kernel::json::{self, Value as Json};
use c600_library::{
    lower_source, lower_sources, quaternion_and_icosian_algebra, trees_sha256, TREES_EXPORTS, TREES_MODULE,
    TREES_SOURCE,
};
use c600_run::interp::{load, Loaded, NoHost, Options};
use c600_run::meter::Outcome;
use c600_run::value::{Quad, Value};
use c600_syntax::{format, parse};
use common::{repository, WIDE};

/// The pinned reference files and their SHA-256, as the fixture records them.
const PINS: [(&str, &str); 2] = [
    ("research/jumbling/theory/trees.py", "8431eedfb778a68b04e5938f3bcfee92feac8e54e718471ea33d6ccedb55a642"),
    ("research/jumbling/exact.py", "763dc7803c82971d76520c53cfb2a492e230e901dcf9cfa6a8fbdf34308bc7a6"),
];

/// The library's quaternion and tree source parses with no diagnostic, is in the formatter's canonical layout with LF
/// line endings, and lowers to one module, `trees`, that requests no capability and exports the seven entries. The
/// checker reports no diagnostic, and every export is pure: no effect, capability, state read or row, so an answer
/// says something about the supplied matrix or quaternion only. The catalogue row is LB1.8's "Quaternion and icosian
/// algebra", its Effect field is the checker's effects and its digest is the source's.
#[test]
fn ct_lb1_01_the_trees_source_checks_and_is_pure() {
    let file = parse(TREES_SOURCE.as_bytes());
    assert!(file.diagnostics.is_empty() && file.holes.is_empty(), "{:#?}", file.diagnostics);
    assert_eq!(format(&file).as_deref(), Some(TREES_SOURCE), "the source is in the canonical layout");
    assert!(!TREES_SOURCE.contains('\r') && TREES_SOURCE.ends_with('\n'), "LF line endings");

    let program = lower_source(TREES_SOURCE).expect("the source lowers");
    let [module] = &program.modules[..] else { panic!("one module, not {}", program.modules.len()) };
    assert_eq!(module.name, TREES_MODULE);
    assert!(module.capabilities.is_empty(), "the module requests no capability");
    let exports: Vec<&str> = module.functions.iter().filter(|f| f.exported).map(|f| f.name.as_str()).collect();
    assert_eq!(exports, TREES_EXPORTS);

    let report = check(&program).expect("well-formed IR");
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    let row = quaternion_and_icosian_algebra();
    for function in TREES_EXPORTS {
        let summary = report.summary(TREES_MODULE, function).expect("a summary");
        assert!(summary.effects.is_empty(), "{function}: {:?}", summary.effects);
        assert!(summary.capabilities.is_empty() && summary.vars.is_empty() && summary.unresolved.is_empty());
        assert!(summary.state_reads.is_empty() && summary.rows.is_empty(), "{function} reads no state");
        assert_eq!(
            separation(summary, &Policy::interim()),
            Separation { reads_state: false, prepares_previews: false }
        );
        assert_eq!(row.effects.iter().copied().collect::<BTreeSet<_>>(), summary.effects, "Effect: pure");
    }
    assert_eq!(row.entry, "Quaternion and icosian algebra");
    assert!(row.level.starts_with("Exact pose matrices or algebraic values"));
    assert!(row.notes.contains("asserts no puzzle reachability"));
    assert_eq!((row.module, row.functions), (TREES_MODULE, &TREES_EXPORTS[..]));
    assert_eq!(row.source_sha256, sha256_hex(TREES_SOURCE.as_bytes()));
    assert_eq!(row.source_sha256, trees_sha256());
}

fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 {
        a.abs()
    } else {
        gcd(b, a % b)
    }
}

/// The source literal of a fixture triple [a, b, d]: `q5(a/d, b/d)` with each coefficient in lowest terms.
fn literal(json: &Json) -> String {
    let [a, b, d] = triple(json);
    let coefficient = |n: i64| {
        let g = gcd(n, d);
        format!("{}/{}", n / g, d / g)
    };
    format!("q5({}, {})", coefficient(a), coefficient(b))
}

/// The body of the private function `name` in the source, from its signature to its closing brace.
fn body(name: &str) -> &'static str {
    let start = TREES_SOURCE.find(&format!("\nfn {name}() ")).unwrap_or_else(|| panic!("fn {name}"));
    let end = start + TREES_SOURCE[start..].find("\n}\n").expect("a closing brace");
    &TREES_SOURCE[start..end]
}

/// The module's constants are trees.py's, as the fixture records them: the inverse of the local basis (`binv`), the
/// 60 projective unit icosians (`punits`, in trees.py's order, which branch indices name) and the five tetrahedral
/// subgroups of A5 as sorted index sets into them (`tets`, in trees.py's order, which branches are numbered by).
#[test]
fn ct_lb1_01_the_trees_constants_are_the_references() {
    let bytes =
        std::fs::read(repository().join("rust/c600-library/tests/fixtures/trees-reference.json")).expect("the fixture");
    let fixture = json::parse(&bytes).expect("JSON");
    let constants = fixture.get("constants").expect("constants");
    let quaternions = |key: &str| -> Vec<String> {
        entries(constants, key)
            .iter()
            .map(|q| {
                let coefficients: Vec<String> = q.as_array().expect("four").iter().map(literal).collect();
                format!("list<Q5>({})", coefficients.join(", "))
            })
            .collect()
    };
    let binv = quaternions("binv");
    assert_eq!(binv.len(), 4);
    assert!(body("local_basis_inverse").contains(&format!("list<List<Q5>>({})", binv.join(", "))), "binv");
    let punits = quaternions("punits");
    assert_eq!(punits.len(), 60);
    assert!(body("projective_units").contains(&format!("list<List<Q5>>({})", punits.join(", "))), "punits");
    let tets: Vec<String> = entries(constants, "tets")
        .iter()
        .map(|set| {
            let members: Vec<String> =
                set.as_array().expect("a set").iter().map(|n| n.as_i64().expect("an index").to_string()).collect();
            format!("list<Nat>({})", members.join(", "))
        })
        .collect();
    assert_eq!(tets.len(), 5);
    assert!(body("tetrahedral_subgroups").contains(&format!("list<List<Nat>>({})", tets.join(", "))), "tets");
}

/// A client of `trees` in the canonical layout: it imports the module by the SHA-256 of its source (the placeholder
/// `TREES`) calls each entry, and forms a quaternion's norm x0² + x1² + x2² + x3² for `trees.v2`.
const TREES_CLIENT: &str = r#"edition "6cs-draft-1";
module trees_client requires [];
import trees "TREES";

export fn factors_of(m: List<List<Q5>>) -> record { l: List<Q5>, r: List<Q5>, reconstructs: Bool } effects [] {
    return trees.factors(m);
}

export fn reconstructs_of(m: List<List<Q5>>, l: List<Q5>, r: List<Q5>) -> Bool effects [] {
    return trees.reconstructs(m, l, r);
}

export fn v2_of(x: Q5) -> Int effects [] {
    return trees.v2(x);
}

export fn kval_of(x: List<Q5>) -> Int effects [] {
    return trees.kval(x);
}

export fn dist_of(x: List<Q5>) -> Int effects [] {
    return trees.dist(x);
}

export fn norm_v2_of(x: List<Q5>) -> Int effects [] {
    let n = exact_add(exact_add(exact_mul(index(x, 0), index(x, 0)), exact_mul(index(x, 1), index(x, 1))), exact_add(exact_mul(index(x, 2), index(x, 2)), exact_mul(index(x, 3), index(x, 3))));
    return trees.v2(n);
}

export fn branch_of(x: List<Q5>) -> Int effects [] {
    return trees.branch(x);
}

export fn pose_tree_of(m: List<List<Q5>>) -> record { branch_l: Int, branch_r: Int, dist_l: Int, dist_r: Int } effects [] {
    return trees.pose_tree(m);
}
"#;

fn client() -> Loaded {
    let client = TREES_CLIENT.replace("TREES", &trees_sha256());
    let file = parse(client.as_bytes());
    assert!(file.diagnostics.is_empty(), "{:#?}", file.diagnostics);
    assert_eq!(format(&file).as_deref(), Some(client.as_str()), "the client is in the canonical layout");
    let program = lower_sources(&[TREES_SOURCE, &client]).expect("the module and the client lower together");
    let report = check(&program).expect("well-formed IR");
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    load(&program).expect("the program loads")
}

/// A run of one client function that must complete.
fn run(loaded: &Loaded, function: &str, args: Vec<Value>) -> Value {
    let run = loaded.run("trees_client", function, args, &mut NoHost, &Options::new(WIDE)).expect("the run starts");
    assert_eq!(run.summary.outcome, Outcome::Completed, "{function}: {:?}", run.summary.code);
    run.value.expect("a value")
}

/// An exact triple [a, b, d] of the fixture, (a + b√5)/d.
fn triple(json: &Json) -> [i64; 3] {
    let parts = json.as_array().expect("a triple");
    assert_eq!(parts.len(), 3);
    [0, 1, 2].map(|i| parts[i].as_i64().expect("an integer"))
}

fn q5(json: &Json) -> Value {
    let [a, b, d] = triple(json);
    Value::Q5(Arc::new(Quad::new(a.into(), b.into(), d.into())))
}

fn quaternion(json: &Json) -> Value {
    let coefficients = json.as_array().expect("a quaternion");
    assert_eq!(coefficients.len(), 4);
    Value::list(coefficients.iter().map(q5).collect())
}

fn matrix(json: &Json) -> Value {
    let rows = json.as_array().expect("a matrix");
    assert_eq!(rows.len(), 4);
    Value::list(rows.iter().map(quaternion).collect())
}

/// The exact triple of a `Q5` answer, in lowest terms with d > 0, as decimal text.
fn answer_triple(value: &Value) -> [String; 3] {
    match value {
        Value::Q5(quad) => [quad.a.to_string(), quad.b.to_string(), quad.d.to_string()],
        other => panic!("a Q5, not {other:?}"),
    }
}

fn answer_quaternion(value: &Value) -> Vec<[String; 3]> {
    match value {
        Value::List(list) => list.items().iter().map(answer_triple).collect(),
        other => panic!("a list, not {other:?}"),
    }
}

fn want_quaternion(json: &Json) -> Vec<[String; 3]> {
    json.as_array().expect("a quaternion").iter().map(|c| triple(c).map(|n| n.to_string())).collect()
}

fn answer_int(value: &Value) -> i64 {
    match value {
        Value::Int(n) => n.to_string().parse().expect("a small integer"),
        other => panic!("an integer, not {other:?}"),
    }
}

fn answer_bool(value: &Value) -> bool {
    match value {
        Value::Bool(b) => *b,
        other => panic!("a Bool, not {other:?}"),
    }
}

fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
    match value {
        Value::Record(record) => record.get(name).unwrap_or_else(|| panic!("no field {name}")),
        other => panic!("a record, not {other:?}"),
    }
}

fn int_of(json: &Json, key: &str) -> i64 {
    json.get(key).and_then(Json::as_i64).unwrap_or_else(|| panic!("an integer {key}"))
}

fn name_of(json: &Json) -> &str {
    json.get("name").and_then(Json::as_str).expect("a name")
}

fn entries<'a>(json: &'a Json, key: &str) -> &'a [Json] {
    json.get(key).and_then(Json::as_array).unwrap_or_else(|| panic!("the fixture's {key}"))
}

/// The quaternion factors and tree outputs, run on `c600-run` with no host, give exactly trees.py's answers recorded in
/// `tests/fixtures/trees-reference.json`, whose pins are the current bytes of trees.py and exact.py: v2 of every
/// recorded value, and v2(0) refused with `LG4:input.pointwise_failed`; kval, dist, branch and v2 of the norm of every
/// recorded quaternion; for every recorded pose matrix, the factors l and r coefficient for coefficient, that they
/// reconstruct the matrix, and the pose's tree distances and branches; and a wrong right factor that does not
/// reconstruct. A run completes with no host, so the entries read no state.
#[test]
fn ct_lb1_01_quaternion_factors_and_trees_agree_with_trees_py() {
    let root = repository();
    let bytes = std::fs::read(root.join("rust/c600-library/tests/fixtures/trees-reference.json")).expect("the fixture");
    let fixture = json::parse(&bytes).expect("JSON");
    let reference = fixture.get("reference").expect("the reference pins");
    for (path, pin) in PINS {
        assert_eq!(reference.get(path).and_then(Json::as_str), Some(pin), "the fixture's pin of {path}");
        let current = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../").to_owned() + path)
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        assert_eq!(sha256_hex(&current), pin, "{path} is the pinned reference");
    }

    let loaded = client();

    let v2 = entries(&fixture, "v2");
    assert!(v2.len() >= 10);
    for case in v2 {
        let x = case.get("x").expect("x");
        let value = run(&loaded, "v2_of", vec![q5(x)]);
        assert_eq!(answer_int(&value), int_of(case, "v2"), "v2 of {:?}", triple(x));
    }
    for case in entries(&fixture, "v2_refused") {
        let x = case.get("x").expect("x");
        let code = case.get("code").and_then(Json::as_str).expect("a code");
        let run = loaded.run("trees_client", "v2_of", vec![q5(x)], &mut NoHost, &Options::new(WIDE)).expect("starts");
        assert_ne!(run.summary.outcome, Outcome::Completed, "v2 of {:?} is refused", triple(x));
        assert_eq!(run.summary.code, Some(code), "v2 of {:?}", triple(x));
    }

    let quaternions = entries(&fixture, "quaternions");
    assert!(quaternions.len() >= 10);
    for case in quaternions {
        let x = case.get("x").expect("x");
        let label = format!("{:?}", want_quaternion(x));
        assert_eq!(answer_int(&run(&loaded, "kval_of", vec![quaternion(x)])), int_of(case, "kval"), "kval {label}");
        assert_eq!(answer_int(&run(&loaded, "dist_of", vec![quaternion(x)])), int_of(case, "dist"), "dist {label}");
        assert_eq!(answer_int(&run(&loaded, "branch_of", vec![quaternion(x)])), int_of(case, "branch"), "{label}");
        assert_eq!(answer_int(&run(&loaded, "norm_v2_of", vec![quaternion(x)])), int_of(case, "norm_v2"), "{label}");
    }

    let poses = entries(&fixture, "poses");
    assert!(poses.len() >= 10);
    for case in poses {
        let name = name_of(case);
        let m = matrix(case.get("m").expect("m"));
        let factors = run(&loaded, "factors_of", vec![m.clone()]);
        assert_eq!(answer_quaternion(field(&factors, "l")), want_quaternion(case.get("l").expect("l")), "{name}: l");
        assert_eq!(answer_quaternion(field(&factors, "r")), want_quaternion(case.get("r").expect("r")), "{name}: r");
        assert!(answer_bool(field(&factors, "reconstructs")), "{name}: the factors reconstruct");
        let (l, r) = (quaternion(case.get("l").expect("l")), quaternion(case.get("r").expect("r")));
        assert!(answer_bool(&run(&loaded, "reconstructs_of", vec![m.clone(), l, r])), "{name}: reconstructs");
        let tree = run(&loaded, "pose_tree_of", vec![m]);
        for key in ["branch_l", "branch_r", "dist_l", "dist_r"] {
            assert_eq!(answer_int(field(&tree, key)), int_of(case, key), "{name}: {key}");
        }
    }

    for case in entries(&fixture, "not_reconstructed") {
        let name = name_of(case);
        let args = vec![
            matrix(case.get("m").expect("m")),
            quaternion(case.get("l").expect("l")),
            quaternion(case.get("r").expect("r")),
        ];
        assert!(!answer_bool(&run(&loaded, "reconstructs_of", args)), "{name}: does not reconstruct");
    }
}
