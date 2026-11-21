//! LP2 conformance tests for chapter 05's run record and chapter 09's reproduction (`c600_session::persist::run`,
//! M6's remaining rows): CT-PS1-16, CT-DET-04, CT-DET-14 and CT-BND-23.
//!
//! Programs are parsed, checked and run on route RT1's interpreter by the test, which plays the executor's half that
//! LP3's runtime plays in a session: it captures the inputs, seals the key, runs, and builds the exact payload with
//! `run::exact`. Expected values come from the chapters or the test's own construction.
//!
//! Evidence kind: source and fixture. One run covers one platform. Nothing here is Windows/DirectX evidence.

use std::cell::Cell;
use std::collections::BTreeMap;

use c600_check::check;
use c600_digest::{sha256, sha256_hex};
use c600_kernel::IdentityBlock;
use c600_run::interp::{load, CancelFlag, Host, Loaded, Options, Request, RowTariff, Sink};
use c600_run::meter::{self, Amounts, Interruption};
use c600_run::provenance::{self, Author, Pin, Producer, Profile};
use c600_run::value::Value;
use c600_session::persist::codec::{self, canonical, Json};
use c600_session::persist::codes::{DIGEST, FORMAT, PROVENANCE, VERSION};
use c600_session::persist::record::{self, Read};
use c600_session::persist::run::{
    self, capture, codes, exact, policy_identity, reading, reproduce, Admission, Captured, Coverage, Ending, Grant,
    Inputs, Key, Parameters, Record, Reproduction, Seed,
};
use c600_syntax::{lower, parse};

const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };
const MODEL: &str = "fixture-model";
/// The puzzle digest every run here reads at `labels` level.
const PUZZLE: &str = "1111111111111111111111111111111111111111111111111111111111111111";

const ADD: &str = "edition \"6cs-draft-1\";\nmodule m requires [];\n\n\
                   export fn main(a: Nat) -> Nat effects [] {\n    return exact_add(a, 2);\n}\n";

/// Endless recursion: only a budget stops it.
const DOWN: &str = "edition \"6cs-draft-1\";\nmodule m requires [];\n\n\
                    export fn main(n: Nat) -> Nat effects [] {\n    return main(exact_add(n, 1));\n}\n";

/// Reads a granted file, a workspace record and the protection input, each served from captured bytes.
const READS: &str = "edition \"6cs-draft-1\";\nmodule m requires [file];\n\n\
                     export fn main() -> Nat effects [file, snapshot] {\n    \
                     let f = file_read<Nat>(\"fixtures/n\");\n    \
                     let w = read_snapshot<Nat>(\"best\");\n    \
                     let p = read_snapshot<Nat>(\"protection\");\n    \
                     return exact_add(f, exact_add(w, p));\n}\n";

/// Searches `0 <= n < top` for `n * n == square` and publishes each witness it finds.
const SEARCH: &str = "edition \"6cs-draft-1\";\nmodule m requires [];\n\n\
                      fn search(n: Nat, top: Nat, square: Nat) -> Nat effects [view] {\n    \
                      return match exact_less(n, top) {\n        true => visit(n, top, square);\n        _ => n;\n    };\n}\n\n\
                      fn visit(n: Nat, top: Nat, square: Nat) -> Nat effects [view] {\n    \
                      return match exact_eq(exact_mul(n, n), square) {\n        true => found(n, top, square);\n        \
                      _ => search(exact_add(n, 1), top, square);\n    };\n}\n\n\
                      fn found(n: Nat, top: Nat, square: Nat) -> Nat effects [view] {\n    \
                      let shown = publish_view(n);\n    return search(exact_add(n, 1), top, square);\n}\n\n\
                      export fn main(top: Nat, square: Nat) -> Nat effects [view] {\n    return search(0, top, square);\n}\n";

/// The search's checker: whether `w` squares to 49.
const CHECK: &str = "edition \"6cs-draft-1\";\nmodule m requires [];\n\n\
                     export fn check(w: Nat) -> Bool effects [] {\n    return exact_eq(exact_mul(w, w), 49);\n}\n";

/// A "checker" that takes nothing in.
const ALWAYS: &str = "edition \"6cs-draft-1\";\nmodule m requires [];\n\n\
                      export fn check() -> Bool effects [] {\n    return true;\n}\n";

/// One draw from a generator on the run's seed.
const DRAW: &str = "edition \"6cs-draft-1\";\nmodule m requires [random];\n\n\
                    export fn main() -> Nat effects [random] {\n    \
                    let g = rng_init(\"s\");\n    return rng_draw(g);\n}\n";

// ------------------------------------------------------------------------------------------------- the executor

/// Parses, lowers, checks and loads one source file; the checker must accept it.
fn build(source: &[u8]) -> Loaded {
    let file = parse(source);
    assert!(file.diagnostics.is_empty(), "{:#?}", file.diagnostics);
    let lowered = lower(&[file]).unwrap_or_else(|e| panic!("{e}"));
    let report = check(&lowered.program).unwrap_or_else(|e| panic!("{e}"));
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    load(&lowered.program).unwrap_or_else(|r| panic!("{r:?}"))
}

/// A host that serves captured bytes only, each a decimal Nat: a file read by its granted scope and a snapshot query
/// by its captured name. It keeps what the run publishes to the view.
#[derive(Default)]
struct Served {
    files: BTreeMap<String, Vec<u8>>,
    queries: BTreeMap<String, Vec<u8>>,
    published: Vec<Vec<u8>>,
}

impl Served {
    fn of(inputs: &Inputs) -> Served {
        let mut host = Served::default();
        for (&(kind, name), bytes) in &inputs.captured {
            let served = if kind == "file" { &mut host.files } else { &mut host.queries };
            served.insert(name.to_owned(), bytes.to_vec());
        }
        host
    }

    fn served(&self, request: &Request) -> Option<&Vec<u8>> {
        match request {
            Request::FileRead { path, .. } => self.files.get(*path),
            Request::Snapshot { query, .. } => self.queries.get(*query),
            _ => None,
        }
    }
}

impl Host for Served {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        self.served(request).ok_or(meter::code::NOT_GRANTED)?;
        Ok(RowTariff { kernel_cost: 1, result_bound: 16 })
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        let bytes = self.served(request).ok_or(meter::code::NOT_GRANTED)?;
        let n: u64 = std::str::from_utf8(bytes).ok().and_then(|t| t.parse().ok()).ok_or(meter::code::NOT_GRANTED)?;
        Ok(Value::int(n))
    }

    fn sink(&mut self, sink: &Sink, _: &Value, bytes: &[u8]) -> Result<(), &'static str> {
        match sink {
            Sink::View => {
                self.published.push(bytes.to_vec());
                Ok(())
            }
            _ => Err(meter::code::NOT_GRANTED),
        }
    }
}

/// The canonical bytes of a Nat, as a run outputs and publishes it.
fn nat_bytes(n: u64) -> Vec<u8> {
    let mut bytes = Vec::new();
    c600_run::encode::encode(&Value::int(n), &mut bytes).unwrap();
    bytes
}

/// The executor's half: runs the key's entry on the checked bytes and builds the exact payload. `cancel` interrupts
/// the run before its first operation, as a solver would.
fn execute(key: &Key, inputs: &Inputs, cancel: bool) -> Json {
    let loaded = build(inputs.modules[key.parameters.module.as_str()]);
    let mut host = Served::of(inputs);
    let args = key.parameters.args.iter().map(|arg| {
        let n: u64 = arg.get("int").and_then(Json::as_text).and_then(|t| t.parse().ok()).expect("a Nat argument");
        Value::int(n)
    });
    let path: Vec<&str> = key.seeds.first().map(|s| s.path.iter().map(String::as_str).collect()).unwrap_or_default();
    let flag = CancelFlag::default();
    if cancel {
        flag.cancel(Interruption::ByUser);
    }
    let options = Options {
        seed: inputs.seeds.first().map(|seed| (*seed, path.as_slice())),
        cancel: cancel.then_some(&flag as _),
        ..Options::new(key.limits)
    };
    let p = &key.parameters;
    let run =
        loaded.run(&p.module, &p.function, args.collect(), &mut host, &options).unwrap_or_else(|r| panic!("{r:?}"));
    exact(&Ending::Run { summary: &run.summary, output: run.output.as_deref() }, &host.published)
}

fn rerun(key: &Key, inputs: &Inputs) -> Result<Json, Reproduction> {
    Ok(execute(key, inputs, false))
}

// ------------------------------------------------------------------------------------------------- jobs

fn file_grant(scope: &str, source: &[u8]) -> Grant {
    Grant { kind: "file".into(), rights: vec!["read".into()], scope: scope.into(), grantee: sha256_hex(source) }
}

/// One run: module `m`'s source, the entry and its arguments, the inputs to capture, the grants, the profile and
/// policy, the limits, an optional seed and whether the solver cancels it.
#[derive(Clone)]
struct Job {
    source: Vec<u8>,
    function: &'static str,
    args: Vec<u64>,
    /// `(kind, name, bytes)`: a file is served by its scope, any other kind as a snapshot query by its name.
    inputs: Vec<(&'static str, &'static str, Vec<u8>)>,
    grants: Vec<Grant>,
    profile: Profile,
    policy: Pin,
    limits: Amounts,
    /// The 32-byte seed of the run's generator, streamed on the path `["test"]`.
    seed: Option<Vec<u8>>,
    cancel: bool,
}

impl Job {
    fn new(source: &str, function: &'static str, args: &[u64]) -> Job {
        Job {
            source: source.as_bytes().to_vec(),
            function,
            args: args.to_vec(),
            inputs: Vec::new(),
            grants: Vec::new(),
            profile: Profile::Research,
            policy: policy_identity(&[]),
            limits: WIDE,
            seed: None,
            cancel: false,
        }
    }

    /// Replaces or adds a captured input.
    fn input(mut self, kind: &'static str, name: &'static str, bytes: &[u8]) -> Job {
        self.inputs.retain(|&(k, n, _)| (k, n) != (kind, name));
        self.inputs.push((kind, name, bytes.to_vec()));
        self
    }

    fn key(&self, captured: Vec<Captured>) -> Key {
        let source = sha256_hex(&self.source);
        let args = self.args.iter().map(|&n| codec::read_canonical(&nat_bytes(n)).unwrap()).collect();
        Key {
            source: vec![source.clone()],
            modules: vec![("m".into(), source)],
            edition: "6cs-draft-1".into(),
            ir_version: c600_ir::IR_VERSION.into(),
            identity: IdentityBlock::full(MODEL),
            level: "labels".into(),
            input: PUZZLE.into(),
            parameters: Parameters {
                module: "m".into(),
                function: self.function.into(),
                args,
                captured,
                grants: self.grants.clone(),
                profile: self.profile,
                policy: self.policy.clone(),
            },
            seeds: self
                .seed
                .iter()
                .map(|seed| Seed {
                    sha256: sha256_hex(seed),
                    path: vec!["test".into()],
                    algorithm: "sha256-counter-draft-1".into(),
                })
                .collect(),
            limits: self.limits,
            metering: Pin::of(c600_ir::EXEC_CONTRACT, b"c600_run::meter::Tariffs::TEST"),
            kernel_cost_table: Pin::of("fixture-kernel-costs-1", b"fixture kernel cost table"),
            kernel_api: Pin::of("fixture-kernel-api-1", b"fixture kernel implementation"),
            checkers: Vec::new(),
        }
    }

    /// Captures the inputs, seals the key, runs and records: the record, its bytes and every byte string the key names.
    fn record(&self) -> (Record, Vec<u8>, BTreeMap<String, Vec<u8>>) {
        let mut available = BTreeMap::from([(sha256_hex(&self.source), self.source.clone())]);
        let mut captured = Vec::new();
        for (kind, name, bytes) in &self.inputs {
            let (c, bytes) = capture(kind, name, || bytes.clone()).unwrap();
            available.insert(c.sha256.clone(), bytes);
            captured.push(c);
        }
        if let Some(seed) = &self.seed {
            available.insert(sha256_hex(seed), seed.clone());
        }
        let key = self.key(captured);
        let mut inputs = Inputs::default();
        inputs.modules.insert("m", &self.source);
        inputs.seeds.extend(self.seed.as_deref());
        for (kind, name, bytes) in &self.inputs {
            inputs.captured.insert((kind, name), bytes);
        }
        let result = execute(&key, &inputs, self.cancel);
        let record = recorded(key, result, &self.source);
        let bytes = record.write().unwrap_or_else(|r| panic!("{r}"));
        (record, bytes, available)
    }
}

/// A record of `key`'s run with its result, and the program block of its producer.
fn recorded(key: Key, result: Json, source: &[u8]) -> Record {
    let producer = Producer {
        contract: Pin::of("fixture-contract-1", b"fixture contract"),
        checker: None,
        author: Author::Own,
        module: sha256(source),
        entry: key.parameters.function.clone(),
        profile: key.parameters.profile,
    };
    let provenance = provenance::program(&producer, false, &[]);
    Record { key, result, safety_stop: None, continues: None, checks: None, provenance }
}

fn field<'a>(value: &'a Json, path: &[&str]) -> &'a Json {
    path.iter().fold(value, |v, name| v.get(name).unwrap_or_else(|| panic!("no {name}")))
}

/// `value` with the field at `path` replaced, or removed for `None`.
fn with(value: &Json, path: &[&str], new: Option<Json>) -> Json {
    let Json::Object(map) = value else { panic!("an object") };
    let mut map = map.clone();
    match path {
        [name] => match new {
            Some(new) => drop(map.insert((*name).into(), new)),
            None => drop(map.remove(*name)),
        },
        [name, rest @ ..] => drop(map.insert((*name).into(), with(&map[*name], rest, new))),
        [] => panic!("an empty path"),
    }
    Json::Object(map)
}

fn refused(bytes: &[u8]) -> &'static str {
    Record::read(bytes).expect_err("a refused record").code
}

// ------------------------------------------------------------------------------------------------- CT-PS1-16

/// CT-DET-02 (part: a completed exact run's payload bytes repeat on reproduction and read back from the stored record,
/// with the wall-clock observation outside the bytes; not run: evidence records and checker traces in the record, and
/// the pinned cross-platform comparison, which `c600-run`'s `tests/conformance_det.rs` runs).
/// CT-DET-03 (whole: reproduction from a recorded key answers `not run` for missing source, module, seed or captured
/// bytes and refuses other bytes under a digest and another edition, IR version, identity, metering revision, kernel
/// cost table, kernel implementation or checker set, all before the re-run starts and with no fallback; the refusals in
/// lowering, loading and the run entry are in `c600-run/tests/conformance_det.rs`).
/// CT-LG5-03 (part: a run record's key names the module digests, edition, IR version, metering revision, kernel cost
/// table, kernel and checker identities, and `reproduce` answers `not run` for missing bytes and refuses another
/// edition, IR version, identity, metering revision, kernel cost table, kernel or checker set before executing; not
/// run: the closure, manifest and lock digests, which the draft payload does not carry (PS1.31), and the record's tie
/// to `c600-module`'s approval).
/// CT-PS1-16 (whole), PS1.31, DET.1, DET.5 to DET.7, DET.17, KN7.10: a run record keeps chapter 09's complete key, its
/// version and digest, and the exact payload; it reads back field for field. Reproducing a completed run or a
/// deterministic cut-off gives the same exact bytes. Missing bytes give `not run`; other supplied bytes, another
/// implementation identity or another payload give a refusal. A cut-off run keeps its covered set and reads open; a
/// safety stop and a cancellation read open, never reproduce, and their wall-clock observation stays out of the key and
/// the exact payload. Neither ever supports a negative claim. A seeded run reproduces from its seed, fetched by digest.
/// PS1.31 also lists evidence references and checker traces, lockfile dependencies and the derivation or reachability
/// references of derived inputs; the row asks for the key's inputs and the exact result bytes, and those fields are not
/// in the draft payload yet.
#[test]
fn ct_ps1_16_a_run_record_reproduces_its_key_and_exact_bytes() {
    let job = Job::new(ADD, "main", &[40]);
    let (record, bytes, available) = job.record();
    assert_eq!(Record::read(&bytes).unwrap(), record);
    assert_eq!(record.write().unwrap(), bytes, "the bytes are the record's FP.5 bytes");
    let value = codec::read_canonical(&bytes).unwrap();
    assert_eq!(field(&value, &["format"]), &Json::text("c600-run-draft"));
    let key = field(&value, &["payload", "key"]);
    let names: Vec<&str> = match key {
        Json::Object(map) => map.keys().map(String::as_str).collect(),
        _ => panic!("a key object"),
    };
    let det_1 = [
        "budgets",
        "checkers",
        "edition",
        "identity",
        "input",
        "ir_version",
        "kernel_api",
        "kernel_cost_table",
        "modules",
        "parameters",
        "seeds",
        "source",
    ];
    assert_eq!(names, det_1, "every DET.1 field");
    assert_eq!(field(&value, &["payload", "key_version"]), &Json::text(run::KEY_VERSION));
    let key_digest = sha256_hex(&canonical(key));
    assert_eq!(field(&value, &["payload", "key_digest", "sha256"]), &Json::Text(key_digest.clone()));
    assert_eq!(record.key.digest(), key_digest);
    assert_eq!(
        field(&value, &["payload", "key", "input"]),
        &Json::object([("role", Json::text("labels")), ("sha256", Json::text(PUZZLE))])
    );
    assert_eq!(field(&value, &["identity"]), &record::identity_json(&IdentityBlock::full(MODEL)));
    assert_eq!((record.outcome(), record.code()), ("completed", None));
    assert_eq!(record.value(), Some(sha256_hex(&nat_bytes(42)).as_str()));
    let read = reading(&record, &[]);
    assert_eq!((read.coverage.clone(), read.may_deny()), (Coverage::Complete, true));
    let recorded = Read::Recorded { bytes: bytes.clone(), code: VERSION };
    assert_eq!(record::read_record(&bytes, run::FORMAT_ID), Ok(recorded), "FP.4: no generic active reader");

    // Reproduction gives the recorded exact bytes.
    let now = Admission::of(&record.key);
    let same = reproduce(&bytes, &available, &now, rerun);
    assert_eq!(same, Reproduction::Same(record.result.clone()));

    // Missing bytes: not run, and nothing runs. Other bytes under the key's digest: refused.
    let source = sha256_hex(ADD.as_bytes());
    let ran = Cell::new(false);
    let watched = |key: &Key, inputs: &Inputs| {
        ran.set(true);
        rerun(key, inputs)
    };
    let mut gone = available.clone();
    gone.remove(&source);
    let not_run = Reproduction::NotRun(codes::INPUT_MISSING.into());
    assert_eq!(reproduce(&bytes, &gone, &now, watched), not_run);
    let mut other = available.clone();
    other.insert(source.clone(), DOWN.as_bytes().to_vec());
    assert_eq!(reproduce(&bytes, &other, &now, watched), Reproduction::Refused(codes::KEY_MISMATCH.into()));
    assert!(!ran.get(), "admission precedes execution");

    // Another implementation identity: refused before execution, never run on the current one.
    let edits: [(&str, fn(&mut Admission)); 7] = [
        ("another edition", |a| a.edition = "6cs-draft-2".into()),
        ("another IR version", |a| a.ir_version = "c600-ir-draft-2".into()),
        ("another identity", |a| a.identity = IdentityBlock::full("another-model")),
        ("another metering revision", |a| a.metering = Pin::of(c600_ir::EXEC_CONTRACT, b"other tariffs")),
        ("another kernel cost table", |a| a.kernel_cost_table = Pin::of("fixture-kernel-costs-2", b"other costs")),
        ("another kernel implementation", |a| a.kernel_api = Pin::of("fixture-kernel-api-1", b"other kernel")),
        ("another checker set", |a| a.checkers = vec![Pin::of("fixture-checker-1", b"checker")]),
    ];
    for (what, edit) in edits {
        let mut other = now.clone();
        edit(&mut other);
        let answer = reproduce(&bytes, &available, &other, watched);
        assert_eq!(answer, Reproduction::Refused(codes::KEY_MISMATCH.into()), "{what}");
    }
    assert!(!ran.get());

    // A record whose exact payload the run does not give: refused as a reproducibility mismatch.
    let forged = Record {
        result: with(&record.result, &["value"], Some(codec::digest_ref("bytes", &nat_bytes(43)))),
        ..record.clone()
    };
    let forged = forged.write().unwrap();
    let answer = reproduce(&forged, &available, &now, rerun);
    assert_eq!(answer, Reproduction::Refused(codes::REPRODUCIBILITY.into()));

    // The key and its digest are checked when read.
    let wrong = with(&value, &["payload", "key_digest", "sha256"], Some(Json::text(PUZZLE)));
    assert_eq!(refused(&canonical(&wrong)), DIGEST);
    let edited = with(&value, &["payload", "key", "parameters", "profile"], Some(Json::text("solving")));
    assert_eq!(refused(&canonical(&edited)), DIGEST, "an edited key no longer has its digest");
    for path in
        [&["payload", "key", "seeds"][..], &["payload", "key", "parameters", "policy"], &["payload", "key_version"]]
    {
        assert_eq!(refused(&canonical(&with(&value, path, None))), FORMAT, "{path:?}");
    }
    let newer = with(&value, &["payload", "key_version"], Some(Json::text("c600-repro-key-draft-2")));
    assert_eq!(refused(&canonical(&newer)), VERSION);
    let mut twice = record.clone();
    twice.key.modules.push(("m".into(), sha256_hex(DOWN.as_bytes())));
    assert_eq!(twice.write().unwrap_err().code, FORMAT, "each module name once");

    // The outcome and its code agree: a completed run has none, any other stop has one, and only a cut-off is a safety
    // stop. A disagreeing payload is no record, so it can never read complete.
    let error = with(&record.result, &["outcome"], Some(Json::text("error")));
    let disagreeing = [
        ("a completed run with a code", with(&record.result, &["code"], Some(Json::text(meter::code::STEPS))), None),
        ("a cut-off without a code", with(&record.result, &["outcome"], Some(Json::text("cut_off"))), None),
        ("a safety stop that is an error", with(&error, &["code"], Some(Json::text(codes::SAFETY_STOP))), Some(1_000)),
    ];
    for (what, result, safety_stop) in disagreeing {
        let forged = Record { result, safety_stop, ..record.clone() };
        assert_eq!(forged.write().unwrap_err().code, FORMAT, "{what}");
    }

    // A seeded run: the seed is a key input, fetched by its digest like any other.
    let seed = [7u8; 32];
    let mut drawn = Job::new(DRAW, "main", &[]);
    drawn.grants =
        vec![Grant { kind: "randomness".into(), rights: vec!["draw".into()], ..file_grant("stream", DRAW.as_bytes()) }];
    drawn.seed = Some(seed.to_vec());
    let (seeded, seeded_bytes, seeded_available) = drawn.record();
    assert_eq!((seeded.outcome(), seeded.key.seeds[0].sha256.as_str()), ("completed", sha256_hex(&seed).as_str()));
    let now = Admission::of(&seeded.key);
    let answer = reproduce(&seeded_bytes, &seeded_available, &now, rerun);
    assert_eq!(answer, Reproduction::Same(seeded.result.clone()));
    let mut gone = seeded_available.clone();
    gone.remove(&sha256_hex(&seed));
    assert_eq!(reproduce(&seeded_bytes, &gone, &now, rerun), not_run);
    let mut other = seeded_available.clone();
    other.insert(sha256_hex(&seed), vec![8; 32]);
    let answer = reproduce(&seeded_bytes, &other, &now, rerun);
    assert_eq!(answer, Reproduction::Refused(codes::KEY_MISMATCH.into()));
    drawn.seed = Some(vec![8; 32]);
    let (reseeded, _, _) = drawn.record();
    assert_ne!(reseeded.key.digest(), seeded.key.digest(), "another seed is another key");
    assert_ne!(reseeded.value(), seeded.value(), "and another draw");

    // A deterministic cut-off: the covered set is kept, the rest is open, and the same key stops at the same place.
    let mut cut = Job::new(DOWN, "main", &[0]);
    cut.limits = Amounts { steps: 200, ..WIDE };
    let (cut_off, cut_bytes, cut_available) = cut.record();
    assert_eq!((cut_off.outcome(), cut_off.code()), ("cut_off", Some(meter::code::STEPS)));
    assert_eq!(cut_off.value(), None);
    assert_ne!(field(&cut_off.result, &["stop"]), &Json::Null, "the stop boundary");
    let read = reading(&cut_off, &[]);
    assert_eq!((read.coverage.clone(), read.may_deny()), (Coverage::WithinBudget, false));
    let now = Admission::of(&cut_off.key);
    assert_eq!(reproduce(&cut_bytes, &cut_available, &now, rerun), Reproduction::Same(cut_off.result.clone()));

    // A wall-clock safety stop: open and not reproducible; its observation is outside the key and the exact payload.
    let stopped = |ms: u128| {
        let key = job.key(Vec::new());
        Record { safety_stop: Some(ms), ..self::recorded(key, exact(&Ending::SafetyStop, &[]), ADD.as_bytes()) }
    };
    let (early, late) = (stopped(1_000), stopped(9_000));
    let (early_bytes, late_bytes) = (early.write().unwrap(), late.write().unwrap());
    assert_ne!(early_bytes, late_bytes);
    assert_eq!(early.key.digest(), late.key.digest());
    assert_eq!(canonical(&early.result), canonical(&late.result));
    assert_eq!(early.code(), Some(codes::SAFETY_STOP));
    let read = reading(&early, &[]);
    assert_eq!((read.coverage.clone(), read.may_deny()), (Coverage::Open(codes::SAFETY_STOP.into()), false));
    let now = Admission::of(&early.key);
    let answer = reproduce(&early_bytes, &available, &now, watched);
    assert_eq!(answer, Reproduction::NotRun(codes::SAFETY_STOP.into()));
    let unobserved = Record { safety_stop: None, ..early.clone() };
    assert_eq!(unobserved.write().unwrap_err().code, FORMAT, "a safety stop keeps its observation");
    let observed = Record { safety_stop: Some(5), ..record.clone() };
    assert_eq!(observed.write().unwrap_err().code, FORMAT, "only a safety stop has one");

    // A cancellation: open, with its actual stop reason, and not reproducible.
    let mut cancelled = Job::new(ADD, "main", &[40]);
    cancelled.cancel = true;
    let (cancelled, cancelled_bytes, available) = cancelled.record();
    assert_eq!((cancelled.outcome(), cancelled.code()), ("cancelled", Some(meter::code::CANCELLED_BY_USER)));
    let read = reading(&cancelled, &[]);
    assert_eq!(read.coverage, Coverage::Open(meter::code::CANCELLED_BY_USER.into()));
    assert!(!read.may_deny());
    let now = Admission::of(&cancelled.key);
    let answer = reproduce(&cancelled_bytes, &available, &now, watched);
    assert_eq!(answer, Reproduction::NotRun(meter::code::CANCELLED_BY_USER.into()));
    assert!(!ran.get());

    // The provenance block is the run's program: its profile, module and entry are the key's.
    let mut other = record.clone();
    other.provenance = with(&other.provenance, &["profile"], Some(Json::text("solving")));
    assert_eq!(other.write().unwrap_err().code, PROVENANCE);
    let mut other = record.clone();
    other.provenance = Json::Null;
    assert_eq!(other.write().unwrap_err().code, FORMAT);
}

// ------------------------------------------------------------------------------------------------- CT-DET-04

/// CT-DET-04 (whole), DET.4, DET.40, DET.41: a run that consults a granted file, a workspace record and the protection
/// input names each by its captured digest. Changing any of them, or the grant, changes the key while the puzzle digest
/// stays; reproducing the old record over the changed bytes is refused, and without them it does not run. A file that
/// changes while it is captured is refused before the key is sealed.
#[test]
fn ct_det_04_consulted_inputs_enter_the_key_while_the_puzzle_digest_stays() {
    let mut base = Job::new(READS, "main", &[])
        .input("file", "fixtures/n", b"5")
        .input("workspace", "best", b"7")
        .input("protection", "protection", b"11");
    base.grants = vec![file_grant("fixtures/n", READS.as_bytes())];
    let (record, bytes, available) = base.record();
    assert_eq!(record.value(), Some(sha256_hex(&nat_bytes(23)).as_str()));
    // The captured inputs are a set: another gathering order gives the same key and the same record bytes.
    let mut kinds: Vec<(&str, &str)> =
        record.key.parameters.captured.iter().map(|c| (c.kind.as_str(), c.name.as_str())).collect();
    kinds.sort_unstable();
    assert_eq!(kinds, [("file", "fixtures/n"), ("protection", "protection"), ("workspace", "best")]);
    let mut regathered = base.clone();
    regathered.inputs.reverse();
    let (regathered, regathered_bytes, _) = regathered.record();
    assert_ne!(regathered.key.parameters.captured, record.key.parameters.captured, "another gathering order");
    assert_eq!((regathered.key.digest(), regathered_bytes), (record.key.digest(), bytes.clone()));
    let read = Record::read(&bytes).unwrap();
    assert_eq!(read.key.digest(), record.key.digest());
    let mut twice = record.clone();
    twice
        .key
        .parameters
        .captured
        .push(Captured { sha256: sha256_hex(b"9"), ..record.key.parameters.captured[1].clone() });
    assert_eq!(twice.write().unwrap_err().code, FORMAT, "one capture per kind and name");

    let mut regranted = base.clone();
    regranted.grants =
        vec![Grant { rights: vec!["read".into(), "list".into()], ..file_grant("fixtures/n", READS.as_bytes()) }];
    let variants = [
        ("a workspace record", base.clone().input("workspace", "best", b"8"), 24),
        ("a protection input", base.clone().input("protection", "protection", b"12"), 24),
        ("a file", base.clone().input("file", "fixtures/n", b"6"), 24),
        ("a grant", regranted, 23),
    ];
    let mut keys = vec![record.key.digest()];
    for (what, job, value) in variants {
        let (changed, _, changed_available) = job.record();
        assert_eq!((&changed.key.level, &changed.key.input), (&record.key.level, &record.key.input), "{what}");
        assert_eq!(changed.value(), Some(sha256_hex(&nat_bytes(value)).as_str()), "{what}");
        assert!(!keys.contains(&changed.key.digest()), "{what} changes the key");
        keys.push(changed.key.digest());
        if value != 23 {
            // The old record over the new bytes under the old digests: refused; the new bytes are another key.
            let mut swapped = available.clone();
            for c in &record.key.parameters.captured {
                let current = changed.key.parameters.captured.iter().find(|n| (&n.kind, &n.name) == (&c.kind, &c.name));
                swapped.insert(c.sha256.clone(), changed_available[&current.unwrap().sha256].clone());
            }
            let now = Admission::of(&record.key);
            assert_eq!(
                reproduce(&bytes, &swapped, &now, rerun),
                Reproduction::Refused(codes::KEY_MISMATCH.into()),
                "{what}"
            );
        }
    }
    let now = Admission::of(&record.key);
    assert_eq!(reproduce(&bytes, &available, &now, rerun), Reproduction::Same(record.result.clone()));
    for c in &record.key.parameters.captured {
        let mut gone = available.clone();
        gone.remove(&c.sha256);
        assert_eq!(
            reproduce(&bytes, &gone, &now, rerun),
            Reproduction::NotRun(codes::INPUT_MISSING.into()),
            "{}",
            c.name
        );
    }

    // DET.41: bytes that change while they are captured refuse admission; stable bytes are captured once.
    let reads = Cell::new(0u8);
    let changing = capture("file", "fixtures/n", || {
        reads.set(reads.get() + 1);
        vec![b'0' + reads.get()]
    });
    assert_eq!(changing.unwrap_err().code, DIGEST);
    let (stable, captured) = capture("workspace", "best", || b"7".to_vec()).unwrap();
    let expected = Captured { kind: "workspace".into(), name: "best".into(), sha256: sha256_hex(b"7"), size: 1 };
    assert_eq!((stable, captured), (expected, b"7".to_vec()));
    assert_eq!(capture("network", "x", Vec::new).unwrap_err().code, FORMAT, "DET.4's kinds only");
}

// ------------------------------------------------------------------------------------------------- CT-DET-14

/// CT-DET-09 (part: a search cut off by its step limit is kept in a run record with its stop boundary, reads as within
/// its budget and never complete, and the same key reproduces the same cut-off (`ct_ps1_16_*` above); not run: the
/// record under a kernel-cost, memory or output cut-off, and a cut-off that two limits reach together).
/// CT-DET-13 (part: a record of a completed run reads complete and may deny; a cut-off within its budget, a wall-clock
/// safety stop and a cancellation read open and never deny, and `reproduce` answers `not run` for the open ones; a
/// checked witness counts once; not run: an empty completed enumeration read as complete, the record of a host error,
/// and a safety stop that the runtime process produced).
/// CT-DET-14 (whole), DET.29, DET.30, KN7.4, PS1.31: a search cut off by its budget keeps the witness it published; a
/// separate check record confirms it, and it then counts, once; a check that reads open itself, as under a clock grant,
/// confirms nothing. The unexamined domain stays open: the record never supports a negative claim. A continuation has
/// new limits and a new key, names the cut-off run, and leaves its bytes as they were.
#[test]
fn ct_det_14_a_checked_witness_survives_an_interrupted_search() {
    let whole = Job::new(SEARCH, "main", &[100, 49]);
    let (complete, _, _) = whole.record();
    assert_eq!(complete.outcome(), "completed");
    assert_eq!(complete.outputs(), [sha256_hex(&nat_bytes(7))]);
    let used = codec::nat(field(&complete.result, &["used", "steps"])).unwrap();

    // A fifth of the search's steps: past the witness at 7, short of the domain's end at 100.
    let mut cut = whole.clone();
    cut.limits = Amounts { steps: used / 5, ..WIDE };
    let (search, search_bytes, _) = cut.record();
    assert_eq!((search.outcome(), search.code()), ("cut_off", Some(meter::code::STEPS)));
    assert_eq!(search.outputs(), [sha256_hex(&nat_bytes(7))], "the witness was published before the stop");
    let read = reading(&search, &[]);
    assert_eq!(
        (read.coverage.clone(), read.witnesses.clone(), read.may_deny()),
        (Coverage::WithinBudget, vec![], false)
    );

    let target = record::reference(&search.json());
    let check_of = |source: &str, args: &[u64], run: &Json, index: u128| {
        let (check, _, _) = Job::new(source, "check", args).record();
        let check = Record { checks: Some((run.clone(), index)), ..check };
        Record::read(&check.write().unwrap()).unwrap()
    };
    let confirmed = check_of(CHECK, &[7], &target, 0);
    assert_eq!(confirmed.checks, Some((target.clone(), 0)));
    let read = reading(&search, &[confirmed.clone(), confirmed.clone()]);
    assert_eq!(read.witnesses, [0], "a checked witness counts, once");
    assert_eq!(read.coverage, Coverage::WithinBudget, "the unexamined domain stays open");
    assert!(!read.may_deny());

    // Checks that do not confirm: a false verdict, another run, an output that is not there, a check that never took
    // the witness in, and a check that reads open itself because it holds a clock grant.
    let other_run = record::reference(&complete.json());
    let clocked = {
        let mut job = Job::new(CHECK, "check", &[7]);
        job.grants =
            vec![Grant { kind: "clock".into(), rights: vec!["read".into()], ..file_grant("wall", CHECK.as_bytes()) }];
        let (check, _, _) = job.record();
        let check = Record { checks: Some((target.clone(), 0)), ..check };
        Record::read(&check.write().unwrap()).unwrap()
    };
    let open = Coverage::Open(codes::INPUT_MISSING.into());
    assert_eq!((clocked.outcome(), reading(&clocked, &[]).coverage), ("completed", open));
    let not_confirming = [
        ("a false verdict", check_of(CHECK, &[8], &target, 0)),
        ("another run", check_of(CHECK, &[7], &other_run, 0)),
        ("no such output", check_of(CHECK, &[7], &target, 1)),
        ("the witness not taken in", check_of(ALWAYS, &[], &target, 0)),
        ("a check under a clock grant", clocked),
    ];
    for (what, check) in not_confirming {
        assert_eq!(reading(&search, &[check]).witnesses, Vec::<u128>::new(), "{what}");
    }

    // A continuation: new limits, a new key, a Ref to the cut-off run, whose bytes do not change.
    let mut next = whole.clone();
    next.limits = Amounts { steps: used * 2, ..WIDE };
    let (continued, _, _) = next.record();
    let continued = Record { continues: Some(target.clone()), ..continued };
    let continued = Record::read(&continued.write().unwrap()).unwrap();
    assert_eq!(continued.continues, Some(target));
    assert_ne!(continued.key.digest(), search.key.digest());
    assert_eq!(continued.key.parameters, search.key.parameters, "the same question");
    assert_eq!(search.write().unwrap(), search_bytes, "the cut-off run is never rewritten");
    assert_eq!(reading(&search, &[]).coverage, Coverage::WithinBudget);
    assert_eq!(reading(&continued, &[]).coverage, Coverage::Complete);

    // A Ref names a run record.
    let not_a_run = Json::object([
        ("format", Json::text("c600-word")),
        ("version", Json::Int(1)),
        ("content", codec::digest_ref("content", b"x")),
    ]);
    let wrong = Record { continues: Some(not_a_run), ..continued.clone() };
    assert_eq!(wrong.write().unwrap_err().code, VERSION);
}

// ------------------------------------------------------------------------------------------------- CT-BND-23

/// CT-BND-23 (whole), BND.17, DET.4, DET.40, DET.7: every run record names its policy identity in the key; reproducing
/// it under another policy, profile or grant set is refused with `LG3:capability.grant_mismatch` before execution.
#[test]
fn ct_bnd_23_every_run_record_names_its_policy_and_reproduces_under_it_only() {
    let mut job = Job::new(READS, "main", &[])
        .input("file", "fixtures/n", b"1")
        .input("workspace", "best", b"2")
        .input("protection", "protection", b"3");
    job.grants = vec![file_grant("fixtures/n", READS.as_bytes())];
    job.policy = policy_identity(&["coarse_count", "coarse_count", "best_length"]);
    assert_eq!(job.policy, policy_identity(&["best_length", "coarse_count"]), "a set of declassifiers");
    assert_ne!(job.policy, policy_identity(&[]));
    let (record, bytes, available) = job.record();
    let value = codec::read_canonical(&bytes).unwrap();
    assert_eq!(field(&value, &["payload", "key", "parameters", "policy"]), &job.policy.json());
    assert_eq!(field(&value, &["payload", "key", "parameters", "profile"]), &Json::text("research"));
    for job in
        [Job { policy: policy_identity(&[]), ..job.clone() }, Job { profile: Profile::ToolAuthoring, ..job.clone() }]
    {
        assert_ne!(job.record().0.key.digest(), record.key.digest(), "the policy and profile are inputs");
    }

    let now = Admission::of(&record.key);
    assert_eq!(reproduce(&bytes, &available, &now, rerun), Reproduction::Same(record.result.clone()));

    // The grants are a set: the same grants offered in another order admit.
    let mut two = job.clone();
    let draw =
        Grant { kind: "randomness".into(), rights: vec!["draw".into()], ..file_grant("stream", READS.as_bytes()) };
    two.grants.push(draw);
    let (two_record, two_bytes, two_available) = two.record();
    let mut reordered = Admission::of(&two_record.key);
    reordered.grants.reverse();
    let answer = reproduce(&two_bytes, &two_available, &reordered, rerun);
    assert_eq!(answer, Reproduction::Same(two_record.result.clone()));

    let ran = Cell::new(false);
    let watched = |key: &Key, inputs: &Inputs| {
        ran.set(true);
        rerun(key, inputs)
    };
    let others = [
        ("another policy", Admission { policy: policy_identity(&["best_length"]), ..now.clone() }),
        ("another profile", Admission { profile: Profile::Solving, ..now.clone() }),
        ("no grant", Admission { grants: Vec::new(), ..now.clone() }),
        ("another scope", Admission { grants: vec![file_grant("fixtures/m", READS.as_bytes())], ..now.clone() }),
        ("another grantee", Admission { grants: vec![file_grant("fixtures/n", ADD.as_bytes())], ..now.clone() }),
    ];
    for (what, other) in others {
        let answer = reproduce(&bytes, &available, &other, watched);
        assert_eq!(answer, Reproduction::Refused(codes::GRANT_MISMATCH.into()), "{what}");
    }
    assert!(!ran.get(), "a grant mismatch is refused before execution");

    // A record without a policy identity, or with a grant of a capability 1.0 never grants, is not a run record.
    let unnamed = with(&value, &["payload", "key", "parameters", "policy"], None);
    assert_eq!(refused(&canonical(&unnamed)), FORMAT);
    let mut network = record.clone();
    network.key.parameters.grants = vec![Grant { kind: "network".into(), ..file_grant("x", READS.as_bytes()) }];
    assert_eq!(network.write().unwrap_err().code, FORMAT);
    let mut seeded = record.clone();
    seeded.key.seeds = vec![Seed {
        sha256: sha256_hex(&[7; 32]),
        path: vec!["test".into()],
        algorithm: "sha256-counter-draft-1".into(),
    }];
    assert_eq!(Record::read(&seeded.write().unwrap()).unwrap().key.seeds, seeded.key.seeds);
}
