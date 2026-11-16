//! LP2 conformance tests for chapter 09's determinism rows (CT-DET): the parts that run on the interpreter of route
//! RT1, its `lp1-exec-draft-1` meter, the generator `sha256-counter-draft-1` and the checker's admission.
//!
//! Every row here is `part`. What waits: `ReproKey`, the run record, jobs and the session store (M6); the runtime
//! process, its hosts and file grants (LP3); the other routes RT2 and RT3, which LQ9 did not select, and the headless
//! Linux leg, which CI does not have (no owner yet). Each test names its own gaps. The frozen contract-vector traces
//! replay in `tests/traces.rs`, and the canonical-codec rows are in `tests/canonical_ps1.rs`.
//!
//! The generator vectors were computed independently of this crate, with Python's `hashlib` from DET.61's definition.
//! Evidence kind: source and fixture. One run covers one platform; nothing here is Windows/DirectX evidence.

use std::thread;

use c600_check::{admit, check, Grant, Policy, Profile, EFFECT_UNDECLARED};
use c600_ir::{Capability, Lit};
use c600_run::approx::Approx;
use c600_run::interp::{code, load, CancelFlag, Host, Loaded, NoHost, Options, Request, RowTariff, Run, Sink};
use c600_run::meter::{self, Amounts, Dim, IntOp, Interruption, Meter, Outcome, Row, Summary, Tariffs};
use c600_run::rng::{self, Cursor};
use c600_run::value::{self, Value};
use c600_syntax::{lower, parse, sha256_hex, Lowered};
use num_bigint::{BigInt, Sign};

const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };
const NOT_GRANTED: &str = meter::code::NOT_GRANTED;
const PROFILES: [Profile; 3] = [Profile::Solving, Profile::ToolAuthoring, Profile::Research];

/// A source file of module `m` that requires the given capabilities.
fn source(requires: &str, declarations: &str) -> String {
    format!("edition \"6cs-draft-1\";\nmodule m requires [{requires}];\n\n{declarations}")
}

fn lowered(sources: &[&str]) -> Lowered {
    let files: Vec<_> = sources
        .iter()
        .map(|s| {
            let file = parse(s.as_bytes());
            assert!(file.diagnostics.is_empty(), "{s}\n{:#?}", file.diagnostics);
            file
        })
        .collect();
    lower(&files).unwrap_or_else(|e| panic!("{e}"))
}

/// Parses, lowers, checks and loads source files; the checker must accept them.
fn build(sources: &[&str]) -> Loaded {
    let lowered = lowered(sources);
    let report = check(&lowered.program).unwrap_or_else(|e| panic!("{e}"));
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    load(&lowered.program).unwrap_or_else(|r| panic!("{r:?}"))
}

fn run(loaded: &Loaded, function: &str, args: Vec<Value>, options: &Options) -> Run {
    loaded.run("m", function, args, &mut NoHost, options).unwrap_or_else(|r| panic!("{r:?}"))
}

fn output(run: &Run) -> &str {
    std::str::from_utf8(run.output.as_deref().expect("a completed run")).expect("UTF-8")
}

fn rat(num: &str, den: &str) -> Value {
    value::literal(&Lit::Rat { num: num.into(), den: den.into() }).expect("a reduced ratio")
}

const ADD: &str = "fn add(x: Nat, y: Nat) -> Nat effects [] {\n    return exact_add(x, y);\n}\n\n\
                   export fn main(a: Nat) -> Nat effects [] {\n    return add(a, 2);\n}\n";

/// Two draws, `a` first.
const TWO: &str = "export fn two() -> record { a: Nat, b: Nat } effects [random] {\n    \
                   return record { a = random_draw(), b = random_draw() };\n}\n";

/// Endless recursion.
const DOWN: &str = "export fn down(n: Nat) -> Nat effects [] {\n    return down(exact_add(n, 1));\n}\n";

/// Recursion to a given depth.
const UP: &str = "fn up(n: Nat, top: Nat) -> Nat effects [] {\n    \
                  return match exact_less(n, top) {\n        true => up(exact_add(n, 1), top);\n        _ => n;\n    };\n}\n\n\
                  export fn main(top: Nat) -> Nat effects [] {\n    return up(0, top);\n}\n";

fn with_seed<'a>(seed: &'a [u8], path: &'a [&'a str], limits: Amounts) -> Options<'a> {
    Options { seed: Some((seed, path)), limits, ..Options::new(WIDE) }
}

/// One of DET.61's reference vectors: the stream key of a seed and a path, draws 0 and 1 as decimal integers, draw
/// 2^64 - 1, and the SHA-256 of `TWO`'s output for the stream.
struct GeneratorVector {
    seed: [u8; 32],
    path: &'static [&'static str],
    key: &'static str,
    draws: [&'static str; 2],
    last: &'static str,
    output_sha256: &'static str,
}

fn generator_vectors() -> [GeneratorVector; 4] {
    let counting = std::array::from_fn(|i| i as u8);
    [
        GeneratorVector {
            seed: [7; 32],
            path: &["test", "two"],
            key: "748d10ea9bf8592f2e1ceb6ee6d1be77d3ee190c94a5527217db3adc21b6ad50",
            draws: [
                "68136969901604871819982940424625422614862904027949873336240424488025528815056",
                "99328748394080387806080858785052179345219148182313128270649871327866910024048",
            ],
            last: "19965386077830177276817960487262202641198613943962126504174536451329644963228",
            output_sha256: "cbb4ee0e1b1b78f663fb865dd92b6061ede648c75546350a3c21d6f45eff965b",
        },
        GeneratorVector {
            seed: counting,
            path: &[],
            key: "33bde433b71dddb6f9655c03644833b42744824ae6501bed5c3f20be10cb27ba",
            draws: [
                "92344695402732220446952285503741189908144195709847757160052883699618525368059",
                "23610452917124596181052025546150603307180061354727293677192621437519003340502",
            ],
            last: "79182245645738814282042368199890048697900094628609708307699437862282711674538",
            output_sha256: "dc2161774c13d4fcd0160cac1fbf0fc3e85fc7b638e6d3a7c949e50af50cf65d",
        },
        GeneratorVector {
            seed: counting,
            path: &["é", ""],
            key: "bb2d5a69e5fcd32ec3c1a1d29d765e850e13be19de407f828d59bd80cb347eb6",
            draws: [
                "74587400856858864569793372597004103745363939044965248726845875442166412951622",
                "74405280977401297789047984120038688903201768849846037650772645350463811132099",
            ],
            last: "2284055453495358579834840135039678460663169030453480424326749785288743806015",
            output_sha256: "c7783cca390e518b043bd5af1e6a763e765aa773945e9fefc6c831a215216531",
        },
        GeneratorVector {
            seed: counting,
            path: &["0"],
            key: "1a8eecfcee0b4dedad8ebd7d106b41ddd0e7d2354102746e2cb63ce2853a2c59",
            draws: [
                "44069354714584647068211889321388289011164935586702569155508538279297058431993",
                "64281502732133143608795184767636825169407951955885493627379087308334341789714",
            ],
            last: "28832310321085382556461263967505765726640393028967217975029046326241767692359",
            output_sha256: "506e054efce2dfd24064c0f24434ab74e1e18afc07b277de3e43d575f57eca11",
        },
    ]
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn decimal(block: [u8; 32]) -> String {
    BigInt::from_bytes_be(Sign::Plus, &block).to_string()
}

/// Every request the runtime makes of its host. Exhaustive on purpose: an environment, process, network or
/// session-file request would have to be named here (DET.35, DET.42).
fn request_name(request: &Request) -> &'static str {
    match request {
        Request::Snapshot { .. } => "snapshot",
        Request::Declassified { .. } => "declassified",
        Request::FileRead { .. } => "file_read",
        Request::Clock => "clock",
        Request::Propose { .. } => "propose",
    }
}

/// Every place a value goes. Exhaustive on purpose: a commit, a commit token or a session file would have to be named
/// here (DET.35, DET.42).
fn sink_name(sink: &Sink) -> &'static str {
    match sink {
        Sink::View => "view",
        Sink::Workspace { .. } => "workspace",
        Sink::File { .. } => "file",
        Sink::Exact { .. } => "exact",
    }
}

/// A host that grants nothing and records every call it receives.
#[derive(Default)]
struct Spy {
    calls: Vec<String>,
}

impl Host for Spy {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        self.calls.push(format!("tariff {}", request_name(request)));
        Err(NOT_GRANTED)
    }
    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        self.calls.push(format!("perform {}", request_name(request)));
        Err(NOT_GRANTED)
    }
    fn sink(&mut self, sink: &Sink, _: &Value, _: &[u8]) -> Result<(), &'static str> {
        self.calls.push(format!("sink {}", sink_name(sink)));
        Err(NOT_GRANTED)
    }
}

/// A host that grants reading the one file `data.txt`, a published view, and nothing else.
#[derive(Default)]
struct OneFile {
    views: Vec<Vec<u8>>,
    refused: Vec<String>,
}

impl Host for OneFile {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        match request {
            Request::FileRead { path: "data.txt", .. } => Ok(RowTariff { kernel_cost: 4, result_bound: 2 }),
            other => {
                self.refused.push(request_name(other).into());
                Err(NOT_GRANTED)
            }
        }
    }
    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        match request {
            Request::FileRead { path: "data.txt", .. } => Ok(Value::int(9)),
            _ => Err(NOT_GRANTED),
        }
    }
    fn sink(&mut self, sink: &Sink, _: &Value, bytes: &[u8]) -> Result<(), &'static str> {
        match sink {
            Sink::View => {
                self.views.push(bytes.to_vec());
                Ok(())
            }
            other => {
                self.refused.push(sink_name(other).into());
                Err(NOT_GRANTED)
            }
        }
    }
}

/// The first row a run with `limits` cannot admit and its failing dimension, predicted from the rows of the same run
/// completed (DET.56): the step first, then kernel cost, memory (live plus the request) and output.
fn first_stop(rows: &[Row], limits: Amounts) -> Option<(usize, Dim)> {
    let (mut used, mut live) = (Amounts::default(), 0u128);
    for (i, row) in rows.iter().enumerate() {
        let needs = [
            (Dim::Steps, used.steps + row.steps),
            (Dim::KernelCost, used.kernel_cost + row.kernel_cost),
            (Dim::Memory, live + row.memory),
            (Dim::Output, used.output + row.output),
        ];
        if let Some(&(dim, _)) = needs.iter().find(|&&(dim, need)| need > limits.get(dim)) {
            return Some((i, dim));
        }
        used.steps += row.steps;
        used.kernel_cost += row.kernel_cost;
        used.output += row.output;
        live = live + row.memory - row.released;
    }
    None
}

fn one_short(used: Amounts, dims: &[Dim]) -> Amounts {
    let mut limits = used;
    for dim in dims {
        match dim {
            Dim::Steps => limits.steps -= 1,
            Dim::KernelCost => limits.kernel_cost -= 1,
            Dim::Memory => limits.memory -= 1,
            Dim::Output => limits.output -= 1,
        }
    }
    limits
}

/// Runs `function` at limits equal to its own use, then with every combination of dimensions one short, twice each.
/// Returns the number of cut-offs checked.
fn check_boundaries(loaded: &Loaded, function: &str, args: &[Value], options: Options) -> usize {
    let reference = run(loaded, function, args.to_vec(), &options);
    assert_eq!(reference.summary.outcome, Outcome::Completed, "{function}");
    let (used, rows) = (reference.summary.used, &reference.summary.rows);
    let mut cut_offs = 0;
    for mask in 0..16usize {
        let dims: Vec<Dim> = (0..4).filter(|i| mask >> i & 1 == 1).map(|i| Dim::ALL[i]).collect();
        if dims.iter().any(|&dim| used.get(dim) == 0) {
            continue;
        }
        let limits = one_short(used, &dims);
        let first = run(loaded, function, args.to_vec(), &Options { limits, ..options });
        let again = run(loaded, function, args.to_vec(), &Options { limits, ..options });
        let context = format!("{function} with {dims:?} one short");
        assert_eq!(first.summary, again.summary, "{context}: a fixed run stops at one boundary");
        assert_eq!(first.output, again.output, "{context}");
        let s = &first.summary;
        match first_stop(rows, limits) {
            None => {
                assert!(dims.is_empty(), "{context}");
                assert_eq!(*s, reference.summary, "{context}: equality is admitted");
                assert_eq!(first.output, reference.output, "{context}");
            }
            Some((i, dim)) => {
                assert_eq!((s.outcome, s.code), (Outcome::CutOff, Some(dim.code())), "{context}");
                let stop = s.stop.as_ref().expect("a stop");
                assert_eq!(
                    (stop.at.as_str(), stop.phase, stop.scope.as_str()),
                    (rows[i].at.as_str(), dim.name(), "root"),
                    "{context}"
                );
                assert_eq!((&s.rows[..i], s.rows.len()), (&rows[..i], i + 1), "{context}");
                // A failed step debits nothing; a failed request keeps the step and debits none of the three.
                let last = &s.rows[i];
                let step = u128::from(dim != Dim::Steps);
                assert_eq!(
                    (last.steps, last.kernel_cost, last.memory, last.output, last.released),
                    (step, 0, 0, 0, 0),
                    "{context}"
                );
                assert!(Dim::ALL.iter().all(|&d| s.used.get(d) <= limits.get(d)), "{context}");
                assert!(first.output.is_none() && first.value.is_none(), "{context}: a cut-off claims no result");
                assert_eq!(s.live_after_cleanup, 0, "{context}");
                cut_offs += 1;
            }
        }
    }
    cut_offs
}

/// CT-DET-01 (part: the run's inputs, run by run; not run: `ReproKey` and key equality, including menu order, the
/// cost-table revision and the checker identity (M6), and a change of machine, user name, location or locale (no owner
/// yet)): changing one input of a run alone, whether the seed, the stream path, an argument, a tariff or a limit,
/// changes its result, while repeating it on another thread with another stack and allocation history does not.
#[test]
fn ct_det_01_each_run_input_changes_the_result_and_ambient_state_does_not() {
    let two = build(&[&source("random", TWO)]);
    let add = build(&[&source("", ADD)]);
    let seed = [7u8; 32];
    let path = ["test", "two"];
    let reference = run(&two, "two", vec![], &with_seed(&seed, &path, WIDE));
    assert_eq!(reference.summary.outcome, Outcome::Completed);

    let elsewhere = thread::scope(|s| {
        s.spawn(|| {
            let ballast: Vec<Vec<u8>> = (0..64).map(|n| vec![n as u8; 97 * n]).collect();
            let r = run(&two, "two", vec![], &with_seed(&seed, &path, WIDE));
            drop(ballast);
            r
        })
        .join()
        .expect("the thread")
    });
    assert_eq!((&elsewhere.summary, &elsewhere.output), (&reference.summary, &reference.output));

    let mut flipped = seed;
    flipped[31] ^= 1;
    let changed = [
        run(&two, "two", vec![], &with_seed(&flipped, &path, WIDE)),
        run(&two, "two", vec![], &with_seed(&seed, &["test", "twO"], WIDE)),
        run(&two, "two", vec![], &with_seed(&seed, &["test"], WIDE)),
        run(&two, "two", vec![], &with_seed(&seed, &["testtwo"], WIDE)),
    ];
    for r in &changed {
        assert_eq!(r.summary.outcome, Outcome::Completed);
        assert_ne!(r.output, reference.output, "a seed or a path is part of the result");
    }

    let tariffs = Tariffs { rng_block: Tariffs::TEST.rng_block + 1, ..Tariffs::TEST };
    let priced = run(&two, "two", vec![], &Options { tariffs, ..with_seed(&seed, &path, WIDE) });
    assert_eq!(priced.output, reference.output);
    assert_eq!(priced.summary.used.kernel_cost, reference.summary.used.kernel_cost + 2, "two draws, one unit each");

    let limits = Amounts { kernel_cost: reference.summary.used.kernel_cost - 1, ..WIDE };
    let limited = run(&two, "two", vec![], &with_seed(&seed, &path, limits));
    assert_eq!(limited.summary.code, Some(meter::code::KERNEL_COST));

    let five = run(&add, "main", vec![Value::int(5)], &Options::new(WIDE));
    let six = run(&add, "main", vec![Value::int(6)], &Options::new(WIDE));
    assert_eq!((output(&five), output(&six)), ("{\"int\":\"7\"}\n", "{\"int\":\"8\"}\n"));
}

/// CT-DET-02 (part: completed exact runs repeated on the platform that runs the test, against bytes pinned from an
/// independent computation; not run: the headless Linux leg, since CI runs `cargo test` on its Windows shards only
/// (no owner yet), and evidence records and the host envelope (M6)): payload bytes, charge rows and checker reports of
/// completed exact runs repeat byte for byte and match their pinned values.
#[test]
fn ct_det_02_completed_exact_runs_repeat_their_pinned_bytes() {
    let two = build(&[&source("random", TWO)]);
    for v in generator_vectors() {
        let first = run(&two, "two", vec![], &with_seed(&v.seed, v.path, WIDE));
        let again = run(&two, "two", vec![], &with_seed(&v.seed, v.path, WIDE));
        assert_eq!((&first.summary, &first.output), (&again.summary, &again.output), "{:?}", v.path);
        let bytes = first.output.as_deref().expect("a completed run");
        assert_eq!(sha256_hex(bytes), v.output_sha256, "{:?}", v.path);
        assert_eq!(bytes.len(), 186, "{:?}", v.path);
    }

    let add = build(&[&source("", ADD)]);
    let r = run(&add, "main", vec![Value::int(5)], &Options::new(WIDE));
    assert_eq!(output(&r), "{\"int\":\"7\"}\n");
    assert_eq!(r.summary.used, Amounts { steps: 11, kernel_cost: 26, memory: 94, output: 12 });

    let sums = source(
        "",
        "export fn sums(x: Int) -> record { big: Int, phi: Q5, r: Rat, less: Bool } effects [] {\n    \
         return record { big = exact_add(x, x), phi = exact_add(q5(1/2, 1/2), q5(1/2, -1/2)), \
         r = exact_add(1/2, 1/3), less = exact_less(1/3, 1/2) };\n}\n",
    );
    let sums = build(&[&sums]);
    let x = Value::int("-170141183460469231731687303715884105728".parse::<BigInt>().unwrap());
    let first = run(&sums, "sums", vec![x.clone()], &Options::new(WIDE));
    assert_eq!(
        output(&first),
        "{\"big\":{\"int\":\"-340282366920938463463374607431768211456\"},\"less\":true,\
         \"phi\":{\"q5\":[\"1\",\"0\",\"1\"]},\"r\":{\"rat\":[\"5\",\"6\"]}}\n"
    );
    let again = run(&sums, "sums", vec![x], &Options::new(WIDE));
    assert_eq!((&first.summary, &first.output), (&again.summary, &again.output));

    // The checker's report, diagnostics and summaries included, repeats too.
    let refused = source("", "export fn f() -> Nat effects [] {\n    return read_snapshot<Nat>(\"q\");\n}\n");
    let refused = lowered(&[&refused]).program;
    let report = check(&refused).unwrap();
    assert_eq!(report.codes(), [EFFECT_UNDECLARED]);
    assert_eq!(check(&refused).unwrap(), report);
}

/// CT-DET-03 (part: refusals before execution in lowering, loading and the run entry; not run: reproduction from a
/// recorded key, which checks the key's identities and input digests (M6)): a missing or substituted dependency, an
/// unsupported IR identity, an unknown entry, a wrong argument count, an argument without a canonical form and an
/// invalid seed are refused or stop before any host request, with no fallback to another version.
#[test]
fn ct_det_03_missing_or_substituted_inputs_are_refused_before_any_effect() {
    let helpers = "edition \"6cs-draft-1\";\nmodule helpers requires [];\n\n\
                   export fn twice(x: Nat) -> Nat effects [] {\n    return exact_add(x, x);\n}\n";
    let client = format!(
        "edition \"6cs-draft-1\";\nmodule m requires [];\nimport h \"{}\";\n\n\
         export fn four() -> Nat effects [] {{\n    return h.twice(2);\n}}\n",
        sha256_hex(helpers.as_bytes())
    );
    let parsed = |sources: &[&str]| sources.iter().map(|s| parse(s.as_bytes())).collect::<Vec<_>>();
    let pinned = build(&[helpers, &client]);
    assert_eq!(output(&run(&pinned, "four", vec![], &Options::new(WIDE))), "{\"int\":\"4\"}\n");
    // Missing, and substituted by a newer version of the same module: neither lowers.
    let newer = helpers.replace("exact_add(x, x)", "exact_add(x, 1)");
    for sources in [vec![client.as_str()], vec![newer.as_str(), client.as_str()]] {
        let refused = lower(&parsed(&sources[..])).expect_err("lowered without the pinned dependency");
        assert!(refused.to_string().contains("no given source file"), "{refused}");
    }

    // An IR identity this interpreter does not run.
    let program = lowered(&[&source("", ADD)]).program;
    for (ir_version, exec_contract) in
        [("c600-ir-draft-0", c600_ir::EXEC_CONTRACT), (c600_ir::IR_VERSION, c600_ir::FORM2_EXEC_CONTRACT)]
    {
        let mut other = program.clone();
        (other.ir_version, other.exec_contract) = (ir_version.into(), exec_contract.into());
        assert_eq!(load(&other).unwrap_err().code, code::IR_VERSION_UNSUPPORTED);
        assert_eq!(check(&other).unwrap().codes(), [c600_check::IR_VERSION_UNSUPPORTED]);
    }

    // Entry refusals: none of them reaches the host.
    let loaded = load(&program).unwrap();
    let mut spy = Spy::default();
    let options = Options::new(WIDE);
    let refusals = [
        loaded.run("m", "missing", vec![], &mut spy, &options).unwrap_err().code,
        loaded.run("other", "main", vec![Value::int(5)], &mut spy, &options).unwrap_err().code,
        loaded.run("m", "main", vec![], &mut spy, &options).unwrap_err().code,
        loaded.run("m", "main", vec![Value::int(1), Value::int(2)], &mut spy, &options).unwrap_err().code,
        loaded.run("m", "main", vec![Value::Fn(0)], &mut spy, &options).unwrap_err().code,
    ];
    assert_eq!(
        refusals,
        [code::IR_REJECTED, code::IR_REJECTED, code::IR_REJECTED, code::IR_REJECTED, code::NONCANONICAL_VALUE]
    );
    // A seed that is not 32 bytes stops at `rng_init`, the first operation.
    let two = build(&[&source("random", TWO)]);
    for len in [0, 31, 33] {
        let seed = vec![7u8; len];
        let path = ["test"];
        let bad = two.run("m", "two", vec![], &mut spy, &with_seed(&seed, &path, WIDE)).unwrap();
        assert_eq!(bad.summary.code, Some(meter::code::GENERATOR_INPUT), "{len}");
        assert_eq!(bad.summary.rows.len(), 1, "{len}");
    }
    assert!(spy.calls.is_empty(), "{:?}", spy.calls);
}

/// CT-DET-05 (part: hash state, declaration and insertion order, allocation and threads within this process; not
/// run: another process or platform (the headless Linux leg, no owner yet)): canonical traversal, key order, draw order
/// and output do not depend on hash seeds, declaration order, record insertion order or allocation history.
#[test]
fn ct_det_05_hash_state_insertion_order_and_allocation_do_not_change_results() {
    let reordered = "export fn main(a: Nat) -> Nat effects [] {\n    return add(a, 2);\n}\n\n\
                     fn add(x: Nat, y: Nat) -> Nat effects [] {\n    return exact_add(x, y);\n}\n";
    let reference = run(&build(&[&source("", ADD)]), "main", vec![Value::int(5)], &Options::new(WIDE));
    for i in 0..8usize {
        // Each load builds its name tables with a fresh `RandomState`, so each one hashes under another seed.
        let ballast: Vec<Vec<u64>> = (0..i * 5).map(|n| vec![n as u64; 31 * n + i]).collect();
        let declared = if i % 2 == 0 { ADD } else { reordered };
        let loaded = build(&[&source("", declared)]);
        let r = run(&loaded, "main", vec![Value::int(5)], &Options::new(WIDE));
        assert_eq!((&r.summary, &r.output), (&reference.summary, &reference.output), "load {i}");
        drop(ballast);
    }

    // Records encode in key order, whatever order their fields were written or inserted in.
    let ab =
        "export fn pair() -> record { a: Nat, b: Text } effects [] {\n    return record { a = 1, b = \"x\" };\n}\n";
    let ba =
        "export fn pair() -> record { b: Text, a: Nat } effects [] {\n    return record { b = \"x\", a = 1 };\n}\n";
    let ab = run(&build(&[&source("", ab)]), "pair", vec![], &Options::new(WIDE));
    let ba = run(&build(&[&source("", ba)]), "pair", vec![], &Options::new(WIDE));
    assert_eq!((output(&ab), output(&ba)), ("{\"a\":{\"int\":\"1\"},\"b\":\"x\"}\n", output(&ab)));

    // Draws follow evaluation order, never key order: the first field written takes draw 0.
    let swapped = TWO
        .replace("a: Nat, b: Nat", "b: Nat, a: Nat")
        .replace("a = random_draw(), b = random_draw()", "b = random_draw(), a = random_draw()");
    let seed = [7u8; 32];
    let path = ["test", "two"];
    let swapped = run(&build(&[&source("random", &swapped)]), "two", vec![], &with_seed(&seed, &path, WIDE));
    let [d0, d1] = generator_vectors()[0].draws;
    assert_eq!(output(&swapped), format!("{{\"a\":{{\"int\":\"{d1}\"}},\"b\":{{\"int\":\"{d0}\"}}}}\n"));
}

/// CT-DET-06 and CT-DET-29 (part: the stream keys, draws and exhaustion of DET.61 and the interpreter's streams; not
/// run: child stream assignments in a program, since the IR has no child construct (they replay in `tests/traces.rs`),
/// other routes (no owner yet), the headless Linux leg (no owner yet), and the run record that seals the seed and
/// refuses host entropy or a clock-derived seed (M6)): four independent reference vectors give their keys, first draws,
/// last draw and program output; segment boundaries and the segment count are part of the key; a run has no
/// generator without a seed; and a seed of the wrong length is refused.
#[test]
fn ct_det_06_generator_reference_vectors() {
    let two = build(&[&source("random", TWO)]);
    for v in generator_vectors() {
        let (key, len) = rng::stream_key(&v.seed, v.path).expect("a valid stream");
        assert_eq!(hex(&key), v.key, "{:?}", v.path);
        assert_eq!(len as u128, rng::key_input_len(32, v.path));
        let mut cursor = Cursor::new(key);
        let draws = [decimal(cursor.draw().unwrap()), decimal(cursor.draw().unwrap())];
        assert_eq!(draws, v.draws, "{:?}", v.path);
        assert_eq!(cursor.counter, 2);
        let mut end = Cursor { key, counter: rng::END - 1 };
        assert_eq!(decimal(end.draw().expect("the last draw")), v.last, "{:?}", v.path);
        assert_eq!((end.draw(), end.counter), (None, rng::END), "no wraparound");
        let r = run(&two, "two", vec![], &with_seed(&v.seed, v.path, WIDE));
        let [d0, d1] = v.draws;
        assert_eq!(output(&r), format!("{{\"a\":{{\"int\":\"{d0}\"}},\"b\":{{\"int\":\"{d1}\"}}}}\n"));
    }

    // The path is length-prefixed: no two segmentations of the same bytes share a stream.
    let seed = [7u8; 32];
    let paths: [&[&str]; 6] = [&[], &[""], &["", ""], &["ab"], &["a", "b"], &["ab", ""]];
    let keys: Vec<_> = paths.iter().map(|p| rng::stream_key(&seed, p).unwrap().0).collect();
    for i in 0..keys.len() {
        for j in 0..i {
            assert_ne!(keys[i], keys[j], "{:?} and {:?}", paths[i], paths[j]);
        }
    }

    // No seed, no draw; a seed must be exactly 32 bytes.
    let none = run(&two, "two", vec![], &Options::new(WIDE));
    assert_eq!((none.summary.outcome, none.summary.code), (Outcome::Error, Some(NOT_GRANTED)));
    for len in [0, 16, 31, 33, 64] {
        assert!(rng::stream_key(&vec![0u8; len], &["root"]).is_none(), "{len}");
    }
}

/// CT-DET-07 (part: exact integers far beyond the host word, fixed-width letter and seed inputs, and Unicode
/// normalization and case pairs in this process's locale; not run: other host locales and platforms (no owner yet)):
/// integer results are exact at every size, invalid fixed-width inputs are refused, and text equality compares scalar
/// values without normalization or case folding, priced on both scalar lengths (DET.62).
#[test]
fn ct_det_07_big_integers_fixed_width_inputs_and_text_pairs() {
    let loaded = build(&[&source(
        "",
        "export fn add(x: Int, y: Int) -> Int effects [] {\n    return exact_add(x, y);\n}\n\n\
         export fn less(x: Int, y: Int) -> Bool effects [] {\n    return exact_less(x, y);\n}\n\n\
         export fn same(a: Text, b: Text) -> Bool effects [] {\n    return exact_eq(a, b);\n}\n",
    )]);
    let options = Options::new(WIDE);
    let word = BigInt::from(u64::MAX);
    let cases = [
        (word.clone(), BigInt::from(1)),
        (BigInt::from(i64::MIN), BigInt::from(-1)),
        (BigInt::from(1) << 200usize, (BigInt::from(1) << 200usize) - 1),
        (-(BigInt::from(1) << 4000usize), BigInt::from(1) << 3999usize),
    ];
    for (x, y) in cases {
        let r = run(&loaded, "add", vec![Value::int(x.clone()), Value::int(y.clone())], &options);
        assert_eq!(output(&r), format!("{{\"int\":\"{}\"}}\n", &x + &y));
        let l = run(&loaded, "less", vec![Value::int(x.clone()), Value::int(y.clone())], &options);
        assert_eq!(output(&l), format!("{}\n", x < y));
    }

    // Letters outside the 1,200 generators, the missing letter 0 and seeds of the wrong width are refused.
    for letters in [vec![0], vec![1201], vec![-1201], vec![1, 0, 2]] {
        assert!(value::literal(&Lit::Word(letters.clone())).is_none(), "{letters:?}");
    }
    assert!(value::literal(&Lit::Word(vec![1200, -1200, 1])).is_some());
    assert!(rng::stream_key(&[0; 31], &[]).is_none());

    // Text equality is scalar equality: no normalization, no case folding, no locale (DET.14, DET.62).
    let pairs = [
        ("é", "e\u{301}", false),
        ("Å", "A\u{30a}", false),
        ("A", "a", false),
        ("ß", "SS", false),
        ("K", "\u{212a}", false),
        ("ı", "i", false),
        ("😀é", "😀é", true),
    ];
    for (a, b, same) in pairs {
        let r = run(&loaded, "same", vec![Value::text(a), Value::text(b)], &options);
        assert_eq!(output(&r), format!("{same}\n"), "{a:?} {b:?}");
        let row = r.summary.rows.iter().find(|row| row.at.ends_with("#exact_eq")).expect("the comparison");
        let scalars = (a.chars().count() + b.chars().count()) as u128;
        assert_eq!(row.kernel_cost, Tariffs::TEST.text_per_scalar * scalars, "{a:?} {b:?}");
    }
}

/// CT-DET-08 (part: cold and warm loads, repeated runs and arguments shared with other runs; not run: kernel and
/// value-store caches and their eviction, which RT1 does not have yet (M4, M6)): result bytes and every charge row
/// match whether the program is loaded afresh or reused, and whether an argument's payload is shared with values other
/// runs keep.
#[test]
fn ct_det_08_cold_and_warm_loads_and_shared_values_keep_bytes_and_charges() {
    let text = source(
        "",
        "export fn main(t: Text, n: Nat) -> record { n: Nat, same: Bool, t: Text } effects [] {\n    \
         return record { n = exact_add(n, n), same = exact_eq(t, \"abc\"), t = t };\n}\n",
    );
    let shared = Value::text("abc");
    let args = || vec![shared.clone(), Value::int(21)];
    let cold: Vec<Run> = (0..3).map(|_| run(&build(&[&text]), "main", args(), &Options::new(WIDE))).collect();
    let loaded = build(&[&text]);
    let warm: Vec<Run> = (0..3).map(|_| run(&loaded, "main", args(), &Options::new(WIDE))).collect();
    let kept = shared.clone();
    let concurrent: Vec<Run> = thread::scope(|s| {
        let handles: Vec<_> = (0..4).map(|_| s.spawn(|| run(&loaded, "main", args(), &Options::new(WIDE)))).collect();
        handles.into_iter().map(|h| h.join().expect("the thread")).collect()
    });
    let reference = &cold[0];
    assert_eq!(output(reference), "{\"n\":{\"int\":\"42\"},\"same\":true,\"t\":\"abc\"}\n");
    for r in cold.iter().chain(&warm).chain(&concurrent) {
        assert_eq!((&r.summary, &r.output), (&reference.summary, &reference.output));
    }
    assert!(value::equal(&kept, &Value::text("abc")), "runs do not change a shared payload");
}

/// CT-DET-09 and CT-DET-27 (part: RT1's runs at their own use and with every combination of limits one short; not
/// run: coverage of a search and the run record that keeps it (M6), and child scopes in a program, since the IR has no
/// child construct (the meter's child rules are in `ct_det_11` and `tests/traces.rs`)): a run with limits equal to its
/// use completes unchanged, since equality is admitted. With any set of limits one short, it stops at the row the
/// completed run's rows predict, in the root scope, with the step checked first and then kernel cost, memory and
/// output; a failed step debits nothing and a denied request keeps its step and debits none of the three. Each stop
/// repeats exactly.
#[test]
fn ct_det_09_each_budget_stops_at_one_recorded_boundary() {
    let add = build(&[&source("", ADD)]);
    let two = build(&[&source("random", TWO)]);
    let up = build(&[&source("", UP)]);
    let seed = [7u8; 32];
    let path = ["test", "two"];
    let cut_offs = [
        check_boundaries(&add, "main", &[Value::int(5)], Options::new(WIDE)),
        check_boundaries(&two, "two", &[], with_seed(&seed, &path, WIDE)),
        check_boundaries(&up, "main", &[Value::int(6)], Options::new(WIDE)),
    ];
    assert_eq!(cut_offs, [15, 15, 15], "every nonempty set of the four limits");
}

/// CT-DET-10 (part: RT1's recorded meter against its quiet meter, which keeps no rows; not run: RT2 and RT3, which LQ9
/// did not select and nobody builds (no owner yet), and kernel chunking inside the kernel (M4); the meter's streaming
/// rows replay in `tests/traces.rs`): the same runs under both meters end with the same counters, stop, phase, scope and
/// cleanup, and the recorded rows add up to the counters.
#[test]
fn ct_det_10_quiet_and_recorded_meters_agree() {
    let add = build(&[&source("", ADD)]);
    let two = build(&[&source("random", TWO)]);
    let down = build(&[&source("", DOWN)]);
    let seed = [7u8; 32];
    let path = ["test", "two"];
    let cases: [(&Loaded, &str, Vec<Value>, Options); 6] = [
        (&add, "main", vec![Value::int(5)], Options::new(WIDE)),
        (&add, "main", vec![Value::int(5)], Options::new(Amounts { output: 11, ..WIDE })),
        (&two, "two", vec![], with_seed(&seed, &path, WIDE)),
        (&two, "two", vec![], with_seed(&seed, &path, Amounts { memory: 100, ..WIDE })),
        (&down, "down", vec![Value::int(0)], Options::new(Amounts { steps: 5_000, ..WIDE })),
        (&down, "down", vec![Value::int(0)], Options::new(Amounts { memory: 5_000, ..WIDE })),
    ];
    for (loaded, function, args, options) in cases {
        let recorded = run(loaded, function, args.clone(), &Options { record: true, ..options });
        let quiet = run(loaded, function, args, &Options { record: false, ..options });
        let context = format!("{function} under {:?}", options.limits);
        assert!(quiet.summary.rows.is_empty() && !recorded.summary.rows.is_empty(), "{context}");
        assert_eq!(quiet.summary, Summary { rows: Vec::new(), ..recorded.summary.clone() }, "{context}");
        assert_eq!(quiet.output, recorded.output, "{context}");
        let rows = &recorded.summary.rows;
        let total = |f: fn(&Row) -> u128| rows.iter().map(f).sum::<u128>();
        let used = recorded.summary.used;
        assert_eq!(
            (total(|r| r.steps), total(|r| r.kernel_cost), total(|r| r.output)),
            (used.steps, used.kernel_cost, used.output),
            "{context}"
        );
    }
}

/// CT-DET-11 (part: the meter's child scopes and RT1's recursion; not run: child jobs and nested checks in a program,
/// since the IR has no child construct and jobs are M6): a child asks for more than its ancestors have left and gets
/// only what every one of them has left; a grandchild that exhausts its own cap absorbs its stop; the ledgers
/// reconcile, each child's use lying within its effective caps and inside its parent's use; and live memory is counted
/// in every scope that holds it. Recursion counts every live frame in the root, so it cannot exceed a limit either.
#[test]
fn ct_det_11_children_and_recursion_cannot_mint_allowance() {
    let mut m = Meter::new(Amounts { steps: 20, kernel_cost: 20, memory: 100, output: 5 });
    m.charge("a", None, 4, 10, 0).unwrap();
    let a = m.own(10);
    let child = m.enter_child("c", None, Amounts { steps: 100, kernel_cost: 8, memory: 1000, output: 2 }).unwrap();
    // `c/child` is charged in the root first: steps and memory are cut to the root's 20 - 2 and 100 - 10.
    assert_eq!(m.innermost().limits, Amounts { steps: 18, kernel_cost: 8, memory: 90, output: 2 });
    m.charge("c/x", None, 2, 20, 1).unwrap();
    let x = m.own(20);
    let grandchild =
        m.enter_child("c/g", None, Amounts { steps: 3, kernel_cost: 4, memory: 1000, output: 100 }).unwrap();
    // Memory is cut by both ancestors (90 - 20 and 100 - 30), output by the child alone (2 - 1, where the root has 4).
    assert_eq!(m.innermost().limits, Amounts { steps: 3, kernel_cost: 4, memory: 70, output: 1 });
    m.charge("c/g/k", None, 3, 0, 0).unwrap();
    let stop = m.charge("c/g/k2", None, 2, 0, 0).unwrap_err();
    assert_eq!((stop.code, stop.position.scope.as_str()), (meter::code::KERNEL_COST, "c/g"));
    m.leave_child(grandchild, Err(stop)).expect("the grandchild absorbs its own stop");
    m.charge("c/y", None, 1, 0, 0).unwrap();
    m.leave_child(child, Ok(20)).unwrap();
    assert!(m.is_live(a) && m.is_live(x));
    assert_eq!(m.live(), 30, "the returned owner stays live in the root");
    let summary = m.finish(None);
    assert_eq!(summary.used, Amounts { steps: 7, kernel_cost: 10, memory: 30, output: 1 });
    assert_eq!((summary.live_before_cleanup, summary.live_after_cleanup), (30, 0));
    let [g, c] = &summary.children[..] else { panic!("two children: {:#?}", summary.children) };
    assert_eq!(
        (g.at.as_str(), g.outcome, g.used, g.returned_memory),
        ("c/g", Outcome::CutOff, Amounts { steps: 2, kernel_cost: 3, memory: 0, output: 0 }, 0)
    );
    assert_eq!(
        (c.at.as_str(), c.outcome, c.used, c.returned_memory, c.live_after_return),
        ("c", Outcome::Completed, Amounts { steps: 5, kernel_cost: 6, memory: 20, output: 1 }, 20, 0)
    );
    for record in &summary.children {
        for dim in Dim::ALL {
            assert!(record.used.get(dim) <= record.effective.get(dim), "{}: {}", record.at, dim.name());
            assert!(record.effective.get(dim) <= record.requested.get(dim), "{}: {}", record.at, dim.name());
            assert!(record.used.get(dim) <= summary.used.get(dim), "{}: {}", record.at, dim.name());
        }
    }
    // The root's own steps (`a`, `c/child`) and kernel cost (`a`) plus the child's use give the root's use.
    assert_eq!((2 + c.used.steps, 4 + c.used.kernel_cost), (summary.used.steps, summary.used.kernel_cost));

    // Every live frame of a recursion counts in the root: the peak grows by one level's charge per level.
    let up = build(&[&source("", UP)]);
    let peak = |depth: i64| {
        let r = run(&up, "main", vec![Value::int(depth)], &Options::new(WIDE));
        assert_eq!(output(&r), format!("{{\"int\":\"{depth}\"}}\n"));
        r.summary.used.memory
    };
    let (four, five, sixteen) = (peak(4), peak(5), peak(16));
    assert!(five > four);
    assert_eq!(sixteen - four, 12 * (five - four));
    for limit in [sixteen - 1, sixteen / 2, four] {
        let r = run(&up, "main", vec![Value::int(16)], &Options::new(Amounts { memory: limit, ..WIDE }));
        assert_eq!((r.summary.code, r.summary.used.memory <= limit), (Some(meter::code::MEMORY), true), "{limit}");
        assert_eq!(r.summary.stop.as_ref().unwrap().scope, "root");
    }
}

/// CT-DET-12 (part: concurrent RT1 runs in one process sharing one loaded program; not run: jobs, the scheduler,
/// child completion order and atomic one-job admission (M6), and the runtime process (LP3)): runs with fixed inputs
/// keep their exact payloads, charges and cut-offs whatever other runs are scheduled beside them, cancelled ones
/// included.
#[test]
fn ct_det_12_concurrent_runs_keep_payloads_and_budget_coverage() {
    let two = build(&[&source("random", TWO)]);
    let add = build(&[&source("", ADD)]);
    let seed = [7u8; 32];
    let path = ["test", "two"];
    let cut = Amounts { kernel_cost: 70, ..WIDE };
    let completed = run(&two, "two", vec![], &with_seed(&seed, &path, WIDE));
    let cut_off = run(&two, "two", vec![], &with_seed(&seed, &path, cut));
    assert_eq!((completed.summary.outcome, cut_off.summary.outcome), (Outcome::Completed, Outcome::CutOff));
    for round in 0..4usize {
        let results: Vec<(usize, Run)> = thread::scope(|s| {
            let handles: Vec<_> = (0..8usize)
                .map(|i| {
                    let (two, add, seed, path) = (&two, &add, &seed, &path);
                    s.spawn(move || {
                        if (i + round) % 3 == 0 {
                            let flag = CancelFlag::default();
                            flag.cancel(Interruption::ByUser);
                            let cancelled = Options { cancel: Some(&flag), ..Options::new(WIDE) };
                            assert_eq!(
                                run(add, "main", vec![Value::int(5)], &cancelled).summary.outcome,
                                Outcome::Cancelled
                            );
                        }
                        let limits = if (i + round) % 2 == 0 { WIDE } else { cut };
                        (i, run(two, "two", vec![], &with_seed(seed, path, limits)))
                    })
                })
                .collect();
            handles.into_iter().rev().map(|h| h.join().expect("the thread")).collect()
        });
        for (i, r) in results {
            let expected = if (i + round) % 2 == 0 { &completed } else { &cut_off };
            assert_eq!((&r.summary, &r.output), (&expected.summary, &expected.output), "round {round}, run {i}");
        }
    }
}

/// CT-DET-13 (part: RT1's outcomes; not run: enumeration jobs, their witnesses and the claims a run record makes
/// (M6), and the wall-clock safety stop of the runtime process (LP3)): a completed run with an empty result, a
/// deterministic cut-off, a host error, a cancellation and a refusal before the run (not run) stay distinct, and only
/// the completed run has a result.
#[test]
fn ct_det_13_outcomes_stay_distinct_and_only_completion_has_a_result() {
    let empty = build(&[&source("", "export fn nothing() -> Unit effects [] {\n    return ();\n}\n")]);
    let add = build(&[&source("", ADD)]);
    let clock =
        build(&[&source("clock", "export fn now() -> Display<Nat> effects [clock] {\n    return read_clock();\n}\n")]);
    let completed = run(&empty, "nothing", vec![], &Options::new(WIDE));
    assert_eq!((completed.summary.outcome, output(&completed)), (Outcome::Completed, "null\n"));
    assert!(matches!(completed.value, Some(Value::Unit)));
    let flag = CancelFlag::default();
    flag.cancel(Interruption::ByUser);
    let ended = [
        run(&add, "main", vec![Value::int(5)], &Options::new(Amounts { steps: 3, ..WIDE })),
        run(&clock, "now", vec![], &Options::new(WIDE)),
        run(&add, "main", vec![Value::int(5)], &Options { cancel: Some(&flag), ..Options::new(WIDE) }),
    ];
    let kinds: Vec<_> = ended.iter().map(|r| (r.summary.outcome, r.summary.code)).collect();
    assert_eq!(
        kinds,
        [
            (Outcome::CutOff, Some(meter::code::STEPS)),
            (Outcome::Error, Some(NOT_GRANTED)),
            (Outcome::Cancelled, Some(meter::code::CANCELLED_BY_USER)),
        ]
    );
    for r in &ended {
        assert!(r.value.is_none() && r.output.is_none(), "no result, so no negative claim: {:?}", r.summary.code);
        assert!(r.summary.stop.is_some());
    }
    let not_run = add.run("m", "main", vec![], &mut NoHost, &Options::new(WIDE)).unwrap_err();
    assert_eq!(not_run.code, code::IR_REJECTED);
}

/// CT-DET-15 (part: exact and approximate display fields on the platform that runs the test, with binary64 as the
/// reference at the declared precision; not run: the headless Linux leg (no owner yet), and display plans of the maths
/// library (LP3)): exact fields compare byte for byte; an approximate field equals the binary64 result at its declared
/// precision; and perturbing an approximate input changes no exact decision.
#[test]
fn ct_det_15_exact_fields_ignore_approximate_perturbation() {
    let loaded = build(&[&source(
        "",
        "export fn show(x: Rat, a: Approx<P53>) -> record { exact: Bool, shown: Approx<P53> } effects [] {\n    \
         return record { exact = exact_less(x, 1/2), shown = approx_add(a, approx53(x)) };\n}\n",
    )]);
    let third = rat("1", "3");
    let tenth = f64::from_bits(0x3fb999999999999a);
    for bits in [0x3fb999999999999au64, 0x3fb999999999999b, 0x3fb9999999999999, 0x4415af1d78b58c40] {
        let a = f64::from_bits(bits);
        let approx = Approx::parse(53, &format!("{a:e}")).expect("a decimal");
        assert_eq!(approx.f64_bits(), Some(bits));
        let r = run(&loaded, "show", vec![third.clone(), Value::Approx(approx)], &Options::new(WIDE));
        let shown = (a + 1.0 / 3.0).to_bits();
        assert_eq!(output(&r), format!("{{\"exact\":true,\"shown\":{{\"f64\":\"{shown:016x}\"}}}}\n"), "{a:e}");
    }
    // The exact decision moves only with the exact input.
    let tenth = Value::Approx(Approx::parse(53, &format!("{tenth:e}")).unwrap());
    let half = run(&loaded, "show", vec![rat("1", "2"), tenth], &Options::new(WIDE));
    assert!(output(&half).starts_with("{\"exact\":false,"));
}

/// CT-DET-17 (part: the checker's admission in every profile and RT1 with a host that grants nothing; not run: the
/// runtime process and the shell's placement (LP3), and other routes (no owner yet)): without a grant, clock, file and
/// generator access is refused statically in each profile and at the operation at run time; network, process and
/// session-file access is refused whatever the grant; and the runtime's host interface has no environment, process,
/// network, session-file or commit request at all.
#[test]
fn ct_det_17_ambient_access_without_a_grant_is_refused() {
    let programs = [
        ("clock", "export fn go() -> Display<Nat> effects [clock] {\n    return read_clock();\n}\n", "read"),
        ("file", "export fn go() -> Nat effects [file] {\n    return file_read<Nat>(\"data.txt\");\n}\n", "read"),
        ("random", "export fn go() -> Nat effects [random] {\n    return random_draw();\n}\n", "rng_draw"),
    ];
    let every = Grant::of(&[
        Capability::File,
        Capability::Clock,
        Capability::Randomness,
        Capability::Network,
        Capability::Process,
        Capability::SessionFile,
    ]);
    for (cap, declarations, op) in programs {
        let program = lowered(&[&source(cap, declarations)]).program;
        assert!(check(&program).unwrap().is_admitted(), "{cap}");
        for profile in PROFILES {
            let refused = admit(&program, "m", "go", profile, &Grant::none(), &Policy::interim()).unwrap();
            assert_eq!(refused.codes(), [NOT_GRANTED], "{cap} in {profile:?}");
            assert_eq!(refused.diagnostics[0].path, "entry");
            assert!(admit(&program, "m", "go", profile, &every, &Policy::interim()).unwrap().is_admitted());
        }
        let mut spy = Spy::default();
        let r = load(&program).unwrap().run("m", "go", vec![], &mut spy, &Options::new(WIDE)).unwrap();
        assert_eq!((r.summary.outcome, r.summary.code), (Outcome::Error, Some(NOT_GRANTED)), "{cap}");
        assert!(r.summary.stop.as_ref().unwrap().at.ends_with(&format!("#{op}")), "{cap}");
        assert!(r.output.is_none());
        assert!(spy.calls.iter().all(|c| c == "tariff clock" || c == "tariff file_read"), "{cap}: {:?}", spy.calls);
    }
    for cap in ["network", "process", "session_file"] {
        let program = lowered(&[&source(cap, "export fn go() -> Nat effects [] {\n    return 1;\n}\n")]).program;
        for profile in PROFILES {
            let refused = admit(&program, "m", "go", profile, &every, &Policy::interim()).unwrap();
            assert_eq!(refused.codes(), [NOT_GRANTED], "{cap} in {profile:?}");
            assert_eq!(refused.diagnostics[0].path, "manifest");
        }
    }
    let ty = c600_ir::Ty::Nat;
    let requests = [Request::Clock, Request::FileRead { path: "p", ty: &ty }];
    assert_eq!(requests.map(|r| request_name(&r)), ["clock", "file_read"]);
    assert_eq!(sink_name(&Sink::File { path: "p" }), "file");
}

/// CT-DET-18 (part: the checker's grants through calls and imports, and RT1 asking its host per resource and right;
/// not run: the file-grant host and its rights (LP3), grants of child jobs (M6; the IR has no child construct), and
/// saved descriptors and replay authority (M6)): a capability needed through a call or an import is the caller's
/// requirement, an import supplies none, and a grant of other capabilities does not cover it; at run time each file
/// request names its path, and a host that grants one file for reading refuses another file and a write.
#[test]
fn ct_det_18_grants_do_not_widen_through_calls_or_imports() {
    let lib = |path: &str| {
        format!(
            "edition \"6cs-draft-1\";\nmodule lib requires [file];\n\n\
             export fn load() -> Nat effects [file] {{\n    return file_read<Nat>(\"{path}\");\n}}\n"
        )
    };
    let client = |lib: &str, requires: &str| {
        format!(
            "edition \"6cs-draft-1\";\nmodule m requires [{requires}];\nimport l \"{}\";\n\n\
             fn inner() -> Nat effects [file] {{\n    return l.load();\n}}\n\n\
             export fn go() -> Nat effects [file] {{\n    return exact_add(inner(), 1);\n}}\n\n\
             export fn save() -> Unit effects [file] {{\n    return file_write(\"data.txt\", 1);\n}}\n",
            sha256_hex(lib.as_bytes())
        )
    };
    let data = lib("data.txt");
    let program = lowered(&[&data, &client(&data, "file")]).program;
    let report = check(&program).unwrap();
    assert!(report.is_admitted(), "{:#?}", report.diagnostics);
    assert!(report.summary("m", "go").unwrap().capabilities.contains(&Capability::File));
    let others = Grant::of(&[Capability::Clock, Capability::Randomness]);
    for profile in PROFILES {
        for grant in [Grant::none(), others.clone()] {
            let refused = admit(&program, "m", "go", profile, &grant, &Policy::interim()).unwrap();
            assert_eq!(refused.codes(), [NOT_GRANTED], "{profile:?} with {grant:?}");
        }
        let granted = admit(&program, "m", "go", profile, &Grant::of(&[Capability::File]), &Policy::interim());
        assert!(granted.unwrap().is_admitted(), "{profile:?}");
    }
    // The import supplies no capability: the client must declare it.
    let undeclared = lowered(&[&data, &client(&data, "")]).program;
    assert!(check(&undeclared).unwrap().codes().contains(&EFFECT_UNDECLARED));

    let loaded = load(&program).unwrap();
    let mut host = OneFile::default();
    let read = loaded.run("m", "go", vec![], &mut host, &Options::new(WIDE)).unwrap();
    assert_eq!(read.output.as_deref(), Some(&b"{\"int\":\"10\"}\n"[..]));
    let write = loaded.run("m", "save", vec![], &mut host, &Options::new(WIDE)).unwrap();
    assert_eq!((write.summary.outcome, write.summary.code), (Outcome::Error, Some(NOT_GRANTED)));
    let other = lib("other.txt");
    let escaping = load(&lowered(&[&other, &client(&other, "file")]).program).unwrap();
    let escaped = escaping.run("m", "go", vec![], &mut host, &Options::new(WIDE)).unwrap();
    assert_eq!((escaped.summary.outcome, escaped.summary.code), (Outcome::Error, Some(NOT_GRANTED)));
    assert_eq!(host.refused, ["file", "file_read"]);
}

/// CT-DET-20 (part: endless recursion and the logical exhaustion of each budget on RT1; not run: the journal and the
/// state hash, which the session store keeps (M6), and killed-runtime fixtures (LP3)): endless work and exhaustion of
/// steps, kernel cost, memory or output each end with a typed cut-off, make no host request, release every owner and
/// repeat exactly.
#[test]
fn ct_det_20_endless_work_and_exhaustion_end_typed_and_repeat() {
    let down = build(&[&source("", DOWN)]);
    let grow =
        build(&[&source("", "export fn grow(n: Int) -> Int effects [] {\n    return grow(exact_add(n, n));\n}\n")]);
    let add = build(&[&source("", ADD)]);
    let cases: [(&Loaded, &str, Vec<Value>, Amounts, &str); 4] = [
        (&down, "down", vec![Value::int(0)], Amounts { steps: 10_000, ..WIDE }, meter::code::STEPS),
        (&grow, "grow", vec![Value::int(1)], Amounts { kernel_cost: 10_000, ..WIDE }, meter::code::KERNEL_COST),
        (&down, "down", vec![Value::int(0)], Amounts { memory: 10_000, ..WIDE }, meter::code::MEMORY),
        (&add, "main", vec![Value::int(5)], Amounts { output: 11, ..WIDE }, meter::code::OUTPUT),
    ];
    for (loaded, function, args, limits, code) in cases {
        let options = Options { record: false, ..Options::new(limits) };
        let mut spy = Spy::default();
        let first = loaded.run("m", function, args.clone(), &mut spy, &options).unwrap();
        let again = loaded.run("m", function, args, &mut spy, &options).unwrap();
        assert_eq!((first.summary.outcome, first.summary.code), (Outcome::CutOff, Some(code)), "{function}");
        assert_eq!(first.summary, again.summary, "{function}");
        assert_eq!(first.summary.live_after_cleanup, 0, "{function}");
        assert!(first.output.is_none() && spy.calls.is_empty(), "{function}");
    }
}

/// A host that grants every literal-word preview at one tariff, records the request bytes it prices and counts the
/// requests it performs.
struct Previewer {
    tariff: RowTariff,
    requests: Vec<Vec<u8>>,
    dispatched: usize,
}

impl Host for Previewer {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        match request {
            Request::Propose { request, .. } => {
                self.requests.push(request.to_vec());
                Ok(self.tariff)
            }
            _ => Err(NOT_GRANTED),
        }
    }
    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        match request {
            Request::Propose { .. } => {
                self.dispatched += 1;
                Ok(Value::receipt(vec![("id", Value::text("r"))]))
            }
            _ => Err(NOT_GRANTED),
        }
    }
    fn sink(&mut self, _: &Sink, _: &Value, _: &[u8]) -> Result<(), &'static str> {
        Err(NOT_GRANTED)
    }
}

/// CT-DET-27 and CT-DET-31 (part: RT1 with a test host; not run: the engine's preview row and its declared tariff
/// (LP3), and RT2 and RT3 (no owner yet)): one `invoke` row requests kernel cost, memory and output in a single
/// admission, since DET.55's commands row charges argument construction, kernel work and the request's canonical
/// output unit before dispatch. DET.56 step 2 checks them in that order: with all three one short the stop is kernel
/// cost, with memory and output short it is memory, with output alone it is output; each denied row keeps only its
/// step, and nothing is dispatched. The output charge is exactly the request bytes the host prices. Every encoding
/// prices its work as kernel cost, apart from the byte charge (DET.59, CV.4): the request's `encode` pass on the
/// value's retained bound, and the top-level result's fused output unit on its exact bytes. So the encoding tariff
/// moves only those two rows' kernel cost.
#[test]
fn ct_det_27_one_invoke_admission_checks_kernel_then_memory_then_output() {
    let loaded = build(&[&source(
        "",
        "export fn go(w: Word) -> Text effects [preview] {\n    return receipt_id(propose(w, target(\"t\")));\n}\n",
    )]);
    let go = |limits: Amounts, tariffs: Tariffs| {
        let mut host =
            Previewer { tariff: RowTariff { kernel_cost: 5, result_bound: 200 }, requests: Vec::new(), dispatched: 0 };
        let options = Options { tariffs, ..Options::new(limits) };
        let run = loaded.run("m", "go", vec![Value::word(&[1, 2])], &mut host, &options).unwrap();
        (run, host)
    };
    let (all, host) = go(WIDE, Tariffs::TEST);
    assert_eq!(output(&all), "\"r\"\n");
    let rows = &all.summary.rows;
    let i = rows.iter().position(|r| r.at == "m.go:0#invoke").expect("the invoke row");
    let (encode, invoke) = (&rows[i - 1], &rows[i]);
    assert_eq!(host.requests.len(), 1);
    let request = host.requests[0].len() as u128;
    assert_eq!((invoke.kernel_cost, invoke.memory, invoke.output), (5, 200, request));
    assert_eq!((encode.at.as_str(), encode.memory - encode.released), ("m.go:0#encode", request));
    assert_eq!(host.dispatched, 1);

    let encodes: Vec<&Row> = rows.iter().filter(|r| r.at.ends_with("#encode")).collect();
    assert_eq!(encodes.len(), 1, "the request's pass; the top-level result is one fused `emit` row");
    assert!(encodes.iter().all(|r| r.kernel_cost == r.memory && r.output == 0));
    let unit = rows.last().unwrap();
    assert_eq!((unit.at.as_str(), unit.kernel_cost, unit.memory, unit.output), ("entry#emit", 4, 4, 4));
    let (double, _) = go(WIDE, Tariffs { encode_per_byte: 2, ..Tariffs::TEST });
    assert_eq!(double.summary.rows.len(), rows.len());
    for (a, b) in rows.iter().zip(&double.summary.rows) {
        let encoding = a.at.ends_with("#encode") || a.at == "entry#emit";
        let kernel = if encoding { 2 * a.kernel_cost } else { a.kernel_cost };
        assert_eq!(
            (b.at.as_str(), b.kernel_cost, b.memory, b.output, b.released),
            (a.at.as_str(), kernel, a.memory, a.output, a.released)
        );
    }

    let (mut before, mut live, mut peak) = (Amounts::default(), 0u128, 0u128);
    for r in &rows[..i] {
        before.steps += r.steps;
        before.kernel_cost += r.kernel_cost;
        before.output += r.output;
        peak = peak.max(live + r.memory);
        live = live + r.memory - r.released;
    }
    assert!(live + invoke.memory > peak, "the invoke row's reservation is the largest live request so far");
    for (short, code) in
        [([1, 1, 1], meter::code::KERNEL_COST), ([0, 1, 1], meter::code::MEMORY), ([0, 0, 1], meter::code::OUTPUT)]
    {
        let limits = Amounts {
            steps: before.steps + 1,
            kernel_cost: before.kernel_cost + invoke.kernel_cost - short[0],
            memory: live + invoke.memory - short[1],
            output: before.output + invoke.output - short[2],
        };
        let (cut, host) = go(limits, Tariffs::TEST);
        assert_eq!((cut.summary.outcome, cut.summary.code), (Outcome::CutOff, Some(code)), "{short:?}");
        assert_eq!(cut.summary.stop.as_ref().unwrap().at, "m.go:0#invoke", "{short:?}");
        let last = cut.summary.rows.last().unwrap();
        assert_eq!((last.steps, last.kernel_cost, last.memory, last.output, last.released), (1, 0, 0, 0, 0));
        assert_eq!(cut.summary.used, Amounts { steps: before.steps + 1, memory: peak, ..before }, "{short:?}");
        assert_eq!((host.requests.len(), host.dispatched), (1, 0), "priced, not dispatched: {short:?}");
    }
}

/// CT-DET-28 (synthetic, part: exact integer addition and text equality on RT1; not run: multiplication, which the
/// surface has no operation for, and loop tests and assertions, which the IR has no construct for (their abstract rows
/// replay in `tests/traces.rs`)): an addition's kernel cost and result reservation follow its operands' bit lengths,
/// whatever their size, and the reservation shrinks to the result; text equality is priced on both scalar lengths.
#[test]
fn ct_det_28_arithmetic_and_text_work_use_operand_sizes() {
    let loaded = build(&[&source(
        "",
        "export fn add(x: Int, y: Int) -> Int effects [] {\n    return exact_add(x, y);\n}\n\n\
         export fn same(a: Text, b: Text) -> Bool effects [] {\n    return exact_eq(a, b);\n}\n",
    )]);
    for (a_bits, b_bits) in [(1u128, 1u128), (8, 9), (64, 1), (65, 64), (1000, 8), (4096, 4096), (20_000, 3)] {
        let a = BigInt::from(1) << (a_bits as usize - 1);
        let b: BigInt = (BigInt::from(1) << (b_bits as usize)) - 1u8;
        assert_eq!((a.bits() as u128, b.bits() as u128), (a_bits, b_bits));
        let r = run(&loaded, "add", vec![Value::int(a.clone()), Value::int(b.clone())], &Options::new(WIDE));
        assert_eq!(output(&r), format!("{{\"int\":\"{}\"}}\n", &a + &b));
        let row = r.summary.rows.iter().find(|row| row.at.ends_with("#exact_add")).expect("the addition");
        let (cost, reservation) = Tariffs::TEST.integer(IntOp::Add, a_bits, b_bits);
        let size = Value::int(&a + &b).size();
        assert_eq!((row.kernel_cost, row.memory, row.released), (cost, reservation, reservation - size), "{a_bits}");
    }
    let text = run(&loaded, "same", vec![Value::text("😀é"), Value::text("abc")], &Options::new(WIDE));
    let row = text.summary.rows.iter().find(|row| row.at.ends_with("#exact_eq")).expect("the comparison");
    assert_eq!(row.kernel_cost, 5, "two scalars and three");
}

/// CT-DET-31 (part: RT1's published views and its top-level output; not run: the run record's retained output (M6)
/// and the shell's display (LP3)): output equal to the limit is admitted; a view published before a cut-off stays
/// delivered while the top-level result is withheld; and the top-level output is charged like every other output,
/// so no terminal reserve bypasses the byte count (DET.59).
#[test]
fn ct_det_31_output_limits_admit_equality_and_charge_the_final_bytes() {
    let loaded = build(&[&source(
        "",
        "export fn show(x: Nat) -> Unit effects [view] {\n    \
         return publish_view(record { b = x, a = \"é\\n\" });\n}\n",
    )]);
    let published = "{\"a\":\"é\\n\",\"b\":{\"int\":\"5\"}}\n";
    assert_eq!(published.len(), 29);
    let at = |limit: u128| {
        let mut host = OneFile::default();
        let r =
            loaded.run("m", "show", vec![Value::int(5)], &mut host, &Options::new(Amounts { output: limit, ..WIDE }));
        (r.unwrap(), host.views)
    };
    let (whole, views) = at(29 + 5);
    assert_eq!((whole.summary.outcome, output(&whole), whole.summary.used.output), (Outcome::Completed, "null\n", 34));
    assert_eq!(views, [published.as_bytes()]);
    let (short, views) = at(29 + 4);
    assert_eq!(
        (short.summary.code, short.summary.stop.as_ref().unwrap().at.as_str()),
        (Some(meter::code::OUTPUT), "entry#emit")
    );
    assert_eq!((short.output, short.summary.used.output), (None, 29));
    assert_eq!(views, [published.as_bytes()], "the view was delivered before the cut-off");
    let (none, views) = at(28);
    assert_eq!(none.summary.stop.as_ref().unwrap().at, "m.show:root#publish_view");
    assert_eq!((none.summary.used.output, views.len()), (0, 0));
}
