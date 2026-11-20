//! CT-LB1-01, the F2, F5 and Q(√5) sign parts (LB1.8's row "Exact linear algebra"; chapter 03's test table): F2
//! and F5 equation ranks and solutions and exact Q(√5) signs agree with their references, and a scalar solve asserts
//! no puzzle reachability.
//! The ranks and solutions are language code, the library's module `linalg` on the IR draft's F2 and F5 (LP2's pull
//! request 222) and loops (LP2's pull request 234), run on `c600-run` with no host.
//!
//! The references:
//!
//! - the 36-macro correction library of the jumbling lattice phase, `research/jumbling/restoration/09-general-method/
//!   data/library-merged.json.gz` (decompressed SHA-256 8a13eb6c…), has "rank 34 over F2 and 2 over F5" (that
//!   directory's `README.md`; its `SENIOR-REVIEW.md` re-read it: binary rank 34 and rank 2 over F5). Its ν rows ship,
//!   bound to that SHA-256, in `c600-jumble/data/corrections-I_a.json`, which the test reads;
//! - the cycle criterion of the same `SENIOR-REVIEW.md` (proved, model review): for a cycle of k edge cells with
//!   ratios r_i and R = r_0 ··· r_(k-1), with column i holding 1 at v_i and r_i at v_(i+1), the target τ·e_X with
//!   τ ≠ 0 has exactly one solution over F5 when R ≠ (-1)^k and none otherwise, and the solution has
//!   t_i = (-1)^i (r_0 ··· r_(i-1)) t_0 with [1 - (-1)^k R] t_0 = τ.
//!
//! A test-local Gauss-Jordan elimination in Rust also checks ranks and solutions of small fixed systems with negative
//! entries.
//!
//! The exact Q(√5) signs are language code too, the library's module `quadratic` on the IR draft's `Q5`, run on
//! `c600-run` with no host. Their reference is the kernel's exact sign of a + b√5 (`c600_kernel::exact::sign5`, KN3.1):
//! the research pins no sign fixture (LX-5's planned Q(√5) fixtures were not written), so the entries are checked
//! against the kernel's own decision on the convergents of √5, where a² - 5b² = ±1 and the sign is hardest to decide,
//! up to 128-bit coefficients.
//!
//! CT-LB1-01's quaternion factor reconstruction and tree outputs run in `ct_lb1_01_trees.rs` (pull request 347), so the
//! test runs whole across the two files. Not built: kernels (null spaces), and ranks and solutions over Z, Q and Q(√5).
//! A pass is source and fixture evidence.

mod common;

use std::collections::BTreeSet;

use c600_check::{check, separation, Policy, Separation};
use c600_digest::sha256_hex;
use c600_kernel::exact::sign5;
use c600_kernel::json::{self, Value as Json};
use c600_library::{
    exact_linear_algebra, exact_q5_signs, linalg_sha256, lower_source, lower_sources, quadratic_sha256, LINALG_EXPORTS,
    LINALG_MODULE, LINALG_SOURCE, QUADRATIC_EXPORTS, QUADRATIC_MODULE, QUADRATIC_SOURCE,
};
use c600_run::interp::{load, Loaded, NoHost, Options};
use c600_run::meter::Outcome;
use c600_run::value::{equal, Value};
use c600_syntax::{format, parse};
use common::{repository, WIDE};

/// The research library's decompressed SHA-256, to which the shipped ν rows are bound.
const LIBRARY_SHA256: &str = "8a13eb6c4f4359ce4dd58a0018cf86eed3089111e11f20b7c00476f6393a661a";

fn linalg() -> Loaded {
    let program = lower_source(LINALG_SOURCE).expect("the source lowers");
    let report = check(&program).expect("well-formed IR");
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    load(&program).expect("the program loads")
}

fn matrix(rows: &[Vec<i64>]) -> Value {
    Value::list(rows.iter().map(|row| Value::list(row.iter().map(|&n| Value::int(n)).collect())).collect())
}

fn ints(values: &[i64]) -> Value {
    Value::list(values.iter().map(|&n| Value::int(n)).collect())
}

fn run(loaded: &Loaded, function: &str, args: Vec<Value>) -> Value {
    let run = loaded.run(LINALG_MODULE, function, args, &mut NoHost, &Options::new(WIDE)).expect("the run starts");
    assert_eq!(run.summary.outcome, Outcome::Completed, "{function}: {:?}", run.summary.code);
    run.value.expect("a value")
}

fn rank(loaded: &Loaded, p: i64, rows: &[Vec<i64>]) -> u64 {
    let value = run(loaded, if p == 2 { "rank_f2" } else { "rank_f5" }, vec![matrix(rows)]);
    (0..=rows.len() as u64).find(|&r| equal(&value, &Value::int(r))).unwrap_or_else(|| panic!("rank {value:?}"))
}

/// What `solve_f2` or `solve_f5` answers: the rank, the solution (empty when the system is inconsistent) and whether
/// the system is consistent.
fn answer(rank: usize, solution: &[i64], solvable: bool) -> Value {
    Value::record(vec![
        ("rank", Value::int(rank as i64)),
        ("solution", ints(solution)),
        ("solvable", Value::Bool(solvable)),
    ])
}

fn solve(loaded: &Loaded, p: i64, rows: &[Vec<i64>], target: &[i64]) -> Value {
    run(loaded, if p == 2 { "solve_f2" } else { "solve_f5" }, vec![matrix(rows), ints(target)])
}

/// The test's own Gauss-Jordan elimination over F_p on the augmented matrix, pivoting on the first `unknowns`
/// columns: the rank and, when the system is consistent, the solution with every free unknown 0.
fn oracle(p: i64, rows: &[Vec<i64>], target: &[i64]) -> (usize, Option<Vec<i64>>) {
    let unknowns = rows.first().map_or(0, Vec::len);
    let mut m: Vec<Vec<i64>> =
        rows.iter().zip(target).map(|(row, &t)| row.iter().chain([&t]).map(|&x| x.rem_euclid(p)).collect()).collect();
    let inverse = |x: i64| (1..p).find(|&y| (x * y) % p == 1).expect("a unit");
    let (mut rank, mut pivots) = (0, Vec::new());
    for column in 0..unknowns {
        let Some(at) = (rank..m.len()).find(|&i| m[i][column] != 0) else { continue };
        m.swap(rank, at);
        let factor = inverse(m[rank][column]);
        m[rank].iter_mut().for_each(|x| *x = (*x * factor) % p);
        let pivot = m[rank].clone();
        for (i, row) in m.iter_mut().enumerate() {
            if i != rank && row[column] != 0 {
                let f = row[column];
                for (x, y) in row.iter_mut().zip(&pivot) {
                    *x = (*x - f * y).rem_euclid(p);
                }
            }
        }
        pivots.push(column);
        rank += 1;
    }
    if m[rank..].iter().any(|row| row[unknowns] != 0) {
        return (rank, None);
    }
    let mut solution = vec![0; unknowns];
    for (k, &column) in pivots.iter().enumerate() {
        solution[column] = m[k][unknowns];
    }
    (rank, Some(solution))
}

/// The library's linear algebra source parses with no diagnostic, is in the formatter's canonical layout with LF line
/// endings, and lowers to one module, `linalg`, that requests no capability and exports the two ranks and the two
/// solves. The checker reports no diagnostic, and every export's summary has no effect, capability, effect variable,
/// state read, row or unresolved call: a solve reads no state and prepares no preview, so it asserts nothing about a
/// puzzle state, reachability included. The catalogue row's Effect field is the checker's effects (the catalogue's
/// `pure`), and its digest is the source's.
#[test]
fn ct_lb1_01_the_linear_algebra_source_checks_and_is_pure() {
    let file = parse(LINALG_SOURCE.as_bytes());
    assert!(file.diagnostics.is_empty() && file.holes.is_empty(), "{:#?}", file.diagnostics);
    assert_eq!(format(&file).as_deref(), Some(LINALG_SOURCE), "the source is in the canonical layout");
    assert!(!LINALG_SOURCE.contains('\r') && LINALG_SOURCE.ends_with('\n'), "LF line endings");

    let program = lower_source(LINALG_SOURCE).expect("the source lowers");
    let [module] = &program.modules[..] else { panic!("one module, not {}", program.modules.len()) };
    assert_eq!(module.name, LINALG_MODULE);
    assert!(module.capabilities.is_empty(), "the module requests no capability");
    let exports: Vec<&str> = module.functions.iter().filter(|f| f.exported).map(|f| f.name.as_str()).collect();
    assert_eq!(exports, LINALG_EXPORTS);

    let report = check(&program).expect("well-formed IR");
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    let row = exact_linear_algebra();
    for function in LINALG_EXPORTS {
        let summary = report.summary(LINALG_MODULE, function).expect("a summary");
        assert!(summary.effects.is_empty(), "{function}: {:?}", summary.effects);
        assert!(summary.capabilities.is_empty() && summary.vars.is_empty() && summary.unresolved.is_empty());
        assert!(summary.state_reads.is_empty() && summary.rows.is_empty(), "{function} reads no state");
        assert_eq!(
            separation(summary, &Policy::interim()),
            Separation { reads_state: false, prepares_previews: false }
        );
        assert_eq!(row.effects.iter().copied().collect::<BTreeSet<_>>(), summary.effects, "Effect: pure");
    }
    assert_eq!(row.entry, "Exact linear algebra");
    assert!(row.level.starts_with("F2 or F5 values"));
    assert!(row.notes.contains("asserts no puzzle reachability") && row.notes.contains("research tier candidate"));
    assert_eq!((row.module, row.functions), (LINALG_MODULE, &LINALG_EXPORTS[..]));
    assert_eq!(row.source_sha256, sha256_hex(LINALG_SOURCE.as_bytes()));
    assert_eq!(row.source_sha256, linalg_sha256());
}

/// The ν rows of the 36-macro correction library (`c600-jumble/data/corrections-I_a.json`, bound to the research
/// library by its SHA-256) have rank 34 over F2 on the 43 C2 coordinates (the 35 signs b0 to b34 and the 8 C2 frames)
/// and rank 2 over F5 on the C5 coordinates of orbits 17 and 28, as the research records. The research README is read
/// for the recorded ranks. Every row is 0 on b0 to b3, b5, b6 and the frames f0, f3 and f6, so the F2 row space is
/// the coordinate space of the 34 others.
#[test]
fn ct_lb1_01_correction_library_ranks_agree_with_the_research() {
    let root = repository();
    let readme = std::fs::read_to_string(root.join("research/jumbling/restoration/09-general-method/README.md"))
        .expect("the research README");
    assert!(readme.contains("The 36-macro correction library of the lattice phase (rank 34 over F2 and 2 over F5)"));

    let bytes = std::fs::read(root.join("rust/c600-jumble/data/corrections-I_a.json")).expect("the shipped rows");
    let data = json::parse(&bytes).expect("JSON");
    let source = data.get("source").expect("a source");
    assert_eq!(source.get("sha256").and_then(Json::as_str), Some(LIBRARY_SHA256));
    assert_eq!(
        source.get("library").and_then(Json::as_str),
        Some("research/jumbling/restoration/09-general-method/data/library-merged.json.gz")
    );
    let names: Vec<&str> =
        data.get("names").and_then(Json::as_array).expect("names").iter().filter_map(Json::as_str).collect();
    let c2_names: Vec<String> =
        (0..35).map(|o| format!("b{o}")).chain([0, 3, 6, 8, 11, 22, 25, 32].map(|o| format!("f{o}"))).collect();
    assert_eq!(names[..43], c2_names.iter().map(String::as_str).collect::<Vec<_>>()[..]);
    assert_eq!(names[43..], ["C5_17", "C5_28"]);

    let corrections = data.get("corrections").and_then(Json::as_array).expect("corrections");
    assert_eq!(corrections.len(), 36);
    let rows: Vec<Vec<i64>> = corrections
        .iter()
        .map(|c| {
            c.get("row").and_then(Json::as_array).expect("a row").iter().map(|x| x.as_i64().expect("int")).collect()
        })
        .collect();
    let c2: Vec<Vec<i64>> = rows.iter().map(|row| row[..43].to_vec()).collect();
    let c5: Vec<Vec<i64>> = rows.iter().map(|row| row[43..].to_vec()).collect();

    let loaded = linalg();
    assert_eq!(rank(&loaded, 2, &c2), 34, "rank 34 over F2");
    assert_eq!(rank(&loaded, 5, &c5), 2, "rank 2 over F5");
    assert_eq!((oracle(2, &c2, &[0; 36]).0, oracle(5, &c5, &[0; 36]).0), (34, 2), "the test's own elimination agrees");

    // Every row is 0 on the 9 coordinates no correction changes, so with rank 34 over the other 34 the row space is
    // exactly the coordinate space on those 34 (the senior review's "row space exactly V34").
    let unreached = ["b0", "b1", "b2", "b3", "b5", "b6", "f0", "f3", "f6"];
    let columns: Vec<usize> = (0..43).filter(|&j| unreached.contains(&c2_names[j].as_str())).collect();
    assert_eq!(columns.len(), 9);
    assert!(c2.iter().all(|row| columns.iter().all(|&j| row[j] == 0)), "no correction changes {unreached:?}");
}

/// The cycle criterion over F5 (`SENIOR-REVIEW.md`, proved): for cycles of 2 and 3 edge cells and every choice of
/// nonzero ratios, and for τ = 1 and τ = 3, `solve_f5` finds exactly one solution when R ≠ (-1)^k (rank k) and none
/// otherwise (rank k - 1), and its solution satisfies t_i = (-1)^i (r_0 ··· r_(i-1)) t_0 and [1 - (-1)^k R] t_0 = τ.
#[test]
fn ct_lb1_01_f5_cycle_criterion_agrees_with_the_research() {
    let loaded = linalg();
    for k in [2usize, 3] {
        for code in 0..4usize.pow(k as u32) {
            let ratios: Vec<i64> = (0..k).map(|i| (code / 4usize.pow(i as u32) % 4) as i64 + 1).collect();
            let mut rows = vec![vec![0i64; k]; k];
            for (i, &r) in ratios.iter().enumerate() {
                rows[i][i] = 1;
                rows[(i + 1) % k][i] = r;
            }
            let big_r = ratios.iter().product::<i64>() % 5;
            let sign = if k % 2 == 0 { 1 } else { 4 };
            for tau in [1i64, 3] {
                let mut target = vec![0; k];
                target[0] = tau;
                let value = solve(&loaded, 5, &rows, &target);
                if big_r == sign {
                    assert!(equal(&value, &answer(k - 1, &[], false)), "{ratios:?} τ={tau}: {value:?}");
                    continue;
                }
                let factor = (1 - if k % 2 == 0 { 1 } else { -1 } * big_r).rem_euclid(5);
                let t0 = (1..5).find(|&t| (factor * t) % 5 == tau).expect("t_0");
                let mut want = vec![t0];
                for i in 1..k {
                    want.push((-want[i - 1] * ratios[i - 1]).rem_euclid(5));
                }
                assert!(equal(&value, &answer(k, &want, true)), "{ratios:?} τ={tau}: {value:?} want {want:?}");
            }
        }
    }
}

/// Fixed systems over F2 and F5, with negative and out-of-field entries: `rank_f2`, `rank_f5`, `solve_f2` and
/// `solve_f5` agree with the test's own elimination, an inconsistent system answers `solvable: false` with no
/// solution, free unknowns are 0, and the empty system has rank 0 and the empty solution.
#[test]
fn ct_lb1_01_ranks_and_solutions_agree_with_elimination() {
    let loaded = linalg();
    let systems: [(Vec<Vec<i64>>, Vec<i64>); 6] = [
        (vec![], vec![]),
        (vec![vec![0, 0, 0], vec![0, 0, 0]], vec![0, 1]),
        (vec![vec![1, 1, 0], vec![0, 1, 1], vec![1, 0, 1]], vec![1, 0, 1]),
        (vec![vec![2, -1, 7, 0], vec![-3, 4, 0, 1], vec![5, 3, 7, 1]], vec![1, -2, 9]),
        (vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9], vec![-1, 0, 1]], vec![6, 15, 24, 0]),
        (
            vec![vec![3, 0, 4, 1, -6], vec![1, 1, 1, 1, 1], vec![0, -2, 5, 8, 10], vec![2, 4, 6, 8, 11]],
            vec![4, -1, 0, 3],
        ),
    ];
    for p in [2i64, 5] {
        for (rows, target) in &systems {
            let (want_rank, want) = oracle(p, rows, target);
            assert_eq!(rank(&loaded, p, rows) as usize, want_rank, "F{p} rank of {rows:?}");
            let value = solve(&loaded, p, rows, target);
            let expected = match &want {
                Some(solution) => answer(want_rank, solution, true),
                None => answer(want_rank, &[], false),
            };
            assert!(equal(&value, &expected), "F{p} {rows:?} = {target:?}: {value:?}, want {want:?}");
            if let Some(solution) = want {
                for (row, &t) in rows.iter().zip(target) {
                    let sum: i64 = row.iter().zip(&solution).map(|(a, x)| a * x).sum();
                    assert_eq!(sum.rem_euclid(p), t.rem_euclid(p), "the solution satisfies {row:?}");
                }
            }
        }
    }
}

/// The library's Q(√5) source parses with no diagnostic, is in the formatter's canonical layout with LF line endings,
/// and lowers to one module, `quadratic`, that requests no capability and exports `sign_q5` and `compare_q5`. The
/// checker reports no diagnostic, and both exports are pure: no effect, capability, state read or row, so a sign says
/// something about the supplied number only. The catalogue row is the Q(√5) part of "Exact linear algebra", its
/// Effect field is the checker's effects and its digest is the source's.
#[test]
fn ct_lb1_01_the_q5_sign_source_checks_and_is_pure() {
    let file = parse(QUADRATIC_SOURCE.as_bytes());
    assert!(file.diagnostics.is_empty() && file.holes.is_empty(), "{:#?}", file.diagnostics);
    assert_eq!(format(&file).as_deref(), Some(QUADRATIC_SOURCE), "the source is in the canonical layout");
    assert!(!QUADRATIC_SOURCE.contains('\r') && QUADRATIC_SOURCE.ends_with('\n'), "LF line endings");

    let program = lower_source(QUADRATIC_SOURCE).expect("the source lowers");
    let [module] = &program.modules[..] else { panic!("one module, not {}", program.modules.len()) };
    assert_eq!(module.name, QUADRATIC_MODULE);
    assert!(module.capabilities.is_empty(), "the module requests no capability");
    let exports: Vec<&str> = module.functions.iter().filter(|f| f.exported).map(|f| f.name.as_str()).collect();
    assert_eq!(exports, QUADRATIC_EXPORTS);

    let report = check(&program).expect("well-formed IR");
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    let row = exact_q5_signs();
    for function in QUADRATIC_EXPORTS {
        let summary = report.summary(QUADRATIC_MODULE, function).expect("a summary");
        assert!(summary.effects.is_empty(), "{function}: {:?}", summary.effects);
        assert!(summary.capabilities.is_empty() && summary.vars.is_empty() && summary.unresolved.is_empty());
        assert!(summary.state_reads.is_empty() && summary.rows.is_empty(), "{function} reads no state");
        assert_eq!(
            separation(summary, &Policy::interim()),
            Separation { reads_state: false, prepares_previews: false }
        );
        assert_eq!(row.effects.iter().copied().collect::<BTreeSet<_>>(), summary.effects, "Effect: pure");
    }
    assert_eq!(row.entry, "Exact linear algebra");
    assert!(row.level.starts_with("Q(√5) values"));
    assert!(row.notes.contains("asserts no puzzle reachability") && row.notes.contains("no approximation"));
    assert_eq!((row.module, row.functions), (QUADRATIC_MODULE, &QUADRATIC_EXPORTS[..]));
    assert_eq!(row.source_sha256, sha256_hex(QUADRATIC_SOURCE.as_bytes()));
    assert_eq!(row.source_sha256, quadratic_sha256());
}

/// A client of `quadratic` in the canonical layout: it imports the module by the SHA-256 of its source (the
/// placeholder `QUADRATIC`) and widens whole numbers to `Q5` with the IR draft's `exact_add` and `exact_mul`.
const Q5_CLIENT: &str = r#"edition "6cs-draft-1";
module q5_client requires [];
import quadratic "QUADRATIC";

fn quad(a: Int, b: Int) -> Q5 effects [] {
    return exact_add(exact_mul(a, q5(1/1, 0/1)), exact_mul(b, q5(0/1, 1/1)));
}

export fn sign_of(a: Int, b: Int) -> Int effects [] {
    return quadratic.sign_q5(quad(a, b));
}

export fn sign_of_thirds_and_sevenths(a: Int, b: Int) -> Int effects [] {
    return quadratic.sign_q5(exact_add(exact_mul(a, q5(1/3, 0/1)), exact_mul(b, q5(0/1, 2/7))));
}

export fn compare_of(a: Int, b: Int, c: Int, d: Int) -> Int effects [] {
    return quadratic.compare_q5(quad(a, b), quad(c, d));
}
"#;

fn q5_client() -> Loaded {
    let client = Q5_CLIENT.replace("QUADRATIC", &quadratic_sha256());
    let file = parse(client.as_bytes());
    assert!(file.diagnostics.is_empty(), "{:#?}", file.diagnostics);
    assert_eq!(format(&file).as_deref(), Some(client.as_str()), "the client is in the canonical layout");
    let program = lower_sources(&[QUADRATIC_SOURCE, &client]).expect("the module and the client lower together");
    let report = check(&program).expect("well-formed IR");
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    load(&program).expect("the program loads")
}

/// The answer of one client function, -1, 0 or 1.
fn q5_sign(loaded: &Loaded, function: &str, args: &[i128]) -> i8 {
    let args = args.iter().map(|&n| Value::int(n)).collect();
    let run = loaded.run("q5_client", function, args, &mut NoHost, &Options::new(WIDE)).expect("the run starts");
    assert_eq!(run.summary.outcome, Outcome::Completed, "{function}: {:?}", run.summary.code);
    let value = run.value.expect("a value");
    [-1i8, 0, 1].into_iter().find(|&s| equal(&value, &Value::int(s))).unwrap_or_else(|| panic!("a sign, not {value:?}"))
}

/// The convergents p/q of √5 (continued fraction [2; 4, 4, 4, ...]) while p fits in 128 bits: p² - 5q² = ±1, so
/// p - q√5 is the nonzero number of least size for its coefficients, and its sign alternates.
fn convergents() -> Vec<(i128, i128)> {
    let (mut previous, mut current) = ((1i128, 0i128), (2i128, 1i128));
    let mut all = vec![current];
    while let (Some(p), Some(q)) = (
        current.0.checked_mul(4).and_then(|p| p.checked_add(previous.0)),
        current.1.checked_mul(4).and_then(|q| q.checked_add(previous.1)),
    ) {
        (previous, current) = (current, (p, q));
        all.push(current);
    }
    all
}

/// Exact Q(√5) signs: `sign_q5` and `compare_q5`, run on `c600-run` with no host, agree with the kernel's exact sign of
/// a + b√5 (`c600_kernel::exact::sign5`) on every case below. The cases are zero and the units of each coefficient,
/// the 128-bit extremes, and for every convergent p/q of √5 up to 128 bits, p ± q√5 with both signs, its neighbours
/// (p ± 1) - q√5, the order of p and q√5, and the number p/3 + (2q/7)√5 with rational coefficients while 7p and 6q fit.
/// The convergents alternate in sign (p² - 5q² = ±1), so a decision that rounds would fail on them. A run completes with
/// no host, so the entries read no state.
#[test]
fn ct_lb1_01_q5_signs_agree_with_the_kernel() {
    let loaded = q5_client();
    let mut cases: Vec<(i128, i128)> = vec![
        (0, 0),
        (1, 0),
        (-1, 0),
        (0, 1),
        (0, -1),
        (i128::MAX, 0),
        (0, i128::MIN),
        (i128::MAX, i128::MIN),
        (i128::MIN, i128::MAX),
        (i128::MAX, -(i128::MAX / 3)),
    ];
    let convergents = convergents();
    assert!(convergents.len() >= 55, "{} convergents up to 128 bits", convergents.len());
    let signs: Vec<i8> = convergents.iter().map(|&(p, q)| sign5(p, -q)).collect();
    assert!(signs.windows(2).all(|pair| pair[0] == -pair[1]), "p - q√5 alternates in sign");
    for &(p, q) in &convergents {
        cases.extend([(p, -q), (-p, q), (p, q), (-p, -q)]);
        cases.extend([p.checked_add(1).map(|p| (p, -q)), p.checked_sub(1).map(|p| (p, -q))].into_iter().flatten());
    }
    for &(a, b) in &cases {
        assert_eq!(q5_sign(&loaded, "sign_of", &[a, b]), sign5(a, b), "sign of {a} + {b}√5");
    }
    for &(p, q) in &convergents {
        assert_eq!(q5_sign(&loaded, "compare_of", &[p, 0, 0, q]), sign5(p, -q), "compare {p} with {q}√5");
        assert_eq!(q5_sign(&loaded, "compare_of", &[0, q, p, 0]), sign5(-p, q), "compare {q}√5 with {p}");
        assert_eq!(q5_sign(&loaded, "compare_of", &[p, q, p, q]), 0, "{p} + {q}√5 equals itself");
        if let (Some(a), Some(b)) = (p.checked_mul(7), q.checked_mul(6)) {
            // p/3 - (2q/7)√5 = (7p - 6q√5) / 21.
            let rational = q5_sign(&loaded, "sign_of_thirds_and_sevenths", &[p, -q]);
            assert_eq!(rational, sign5(a, -b), "sign of {p}/3 - (2·{q}/7)√5");
        }
    }
}
