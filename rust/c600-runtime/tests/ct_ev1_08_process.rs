//! CT-EV1-08 (chapter 04; LP4 under PHS.26 and PHS.62; EV1.11) in the runtime's own process (route RT1, LQ9): a run
//! bound to A and suspended in the child at a host call stays on A while B activates; a later job binds B; a job built
//! while an invocation races activation carries one version wholly.
//!
//! CT-EV1-08 (source, part: the suspended run and racing bound jobs on the child; the activation service, its
//! refusals, the in-process executor and racing invocations are in `c600-module/tests/ct_ev1_08.rs`, which with this
//! file is the whole by union).

mod support;

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, Mutex};

use c600_check::{Grant, Policy};
use c600_digest::sha256;
use c600_module::code::{MODULE_NOT_CHECKED, STALE_ACTIVATION_BASE};
use c600_module::{resolve, Activation, Candidate, Dependency, Entry, Files, Identity, Manifest, Modules, Refusal};
use c600_run::interp::RowTariff;
use c600_run::provenance::{hex, Author, Pin, Profile};
use c600_run::value::Value;
use c600_runtime::{run_bound_in_process, BoundJob, Ended, JobOptions, Stage};

use support::{assert_same_end, output, ran, runtime, Session, WIDE};

const EDITION: &str = "6cs-draft-1";
const PROBE: i64 = 5;

fn module(name: &str, imports: &[(&str, &str)], body: &str) -> String {
    let mut text = format!("edition \"{EDITION}\";\nmodule {name} requires [];\n\n");
    for (alias, source) in imports {
        text.push_str(&format!("import {alias} \"{}\";\n", hex(&sha256(source.as_bytes()))));
    }
    if !imports.is_empty() {
        text.push('\n');
    }
    text + body
}

const BODY: &str = "export fn go() -> Int effects [snapshot] {\n    \
                    let n = read_snapshot<Int>(\"probe\");\n    \
                    return exact_add(n, helper.value());\n}\n\n\
                    export fn fail() -> Int effects [snapshot] {\n    \
                    let n = read_snapshot<Int>(\"probe\");\n    \
                    return exact_add(n, read_snapshot<Int>(\"refused\"));\n}\n";

/// Version `n` of `m`: the same root text over a dependency `helper` whose `value` returns `n`.
struct Version {
    n: i64,
    root: String,
    dep: String,
    manifest: Vec<u8>,
    lock: Vec<u8>,
    files: Files,
}

fn version(n: i64) -> Version {
    let dep = module("helper", &[], &format!("export fn value() -> Int effects [] {{\n    return {n};\n}}\n"));
    let root = module("m", &[("helper", &dep)], BODY);
    let manifest = Manifest {
        module: "m".into(),
        edition: EDITION.into(),
        source: sha256(root.as_bytes()),
        dependencies: vec![Dependency { alias: "helper".into(), source: sha256(dep.as_bytes()) }],
        capabilities: vec![],
        boundary: None,
        principle: None,
        supports: vec![Identity::full()],
    }
    .bytes();
    let files = Files::new().with("src/root.6cs", root.as_bytes()).with("src/dep.6cs", dep.as_bytes());
    let lock = resolve(&manifest, &files).unwrap_or_else(|r| panic!("{r:?}")).bytes();
    Version { n, root, dep, manifest, lock, files }
}

impl Version {
    fn source(&self) -> [u8; 32] {
        sha256(self.root.as_bytes())
    }

    fn modules(&self) -> BTreeSet<[u8; 32]> {
        [sha256(self.root.as_bytes()), sha256(self.dep.as_bytes())].into_iter().collect()
    }

    /// The output of `go` on this version: its canonical encoding.
    fn output(&self) -> String {
        format!("{{\"int\":\"{}\"}}\n", PROBE + self.n)
    }
}

fn entry(function: &str) -> Entry {
    Entry {
        module: "m".into(),
        function: function.into(),
        author: Author::Own,
        profile: Profile::ToolAuthoring,
        grant: Grant::none(),
        policy: Policy::interim(),
    }
}

fn modules() -> Modules {
    let contract = Pin::of("fixture-1", b"p3 fixture capability contract 1\n");
    Modules::new(Identity::full(), contract, Pin::of("m1-draft", b"c600-check fixture\n"), Grant::none())
}

fn check(modules: &Modules, v: &Version) -> Candidate {
    let entries = [entry("go"), entry("fail")];
    modules.check(&v.manifest, &v.lock, &v.files, &entries).unwrap_or_else(|r| panic!("{r:?}"))
}

fn install(modules: &Modules, v: &Version) -> Candidate {
    let c = check(modules, v);
    let active = modules.active("m");
    modules.activate_module("m", active, c.digest, c.reference).unwrap_or_else(|r| panic!("{r:?}"));
    c
}

fn job(modules: &Modules, function: &str) -> BoundJob {
    let binding = modules.bind("m", function).unwrap_or_else(|r| panic!("{r:?}"));
    BoundJob::new(&binding, vec![], JobOptions::new(WIDE)).unwrap_or_else(|r| panic!("{r:?}"))
}

/// The fake session with the probe row.
fn session() -> Session {
    let mut session = Session::default();
    session.snapshots.push(("probe", RowTariff { kernel_cost: 1, result_bound: 64 }, Value::int(PROBE)));
    session
}

fn stop_digest(ended: &Ended) -> String {
    let stop = ran(ended).summary.stop.as_ref().expect("a stop");
    stop.source.as_ref().expect("a source range").digest.clone()
}

/// A bound job carries its version's bytes in the lock's order, its entry and the approval's admission inputs, and the
/// child runs it as the in-process reference does. A built-in entry has no class rows to send and is refused.
#[test]
fn ct_ev1_08_a_bound_job_carries_one_version() {
    let a = version(10);
    let modules = modules();
    install(&modules, &a);
    let binding = modules.bind("m", "go").unwrap();
    let job = BoundJob::new(&binding, vec![], JobOptions::new(WIDE)).unwrap();
    assert_eq!((job.closure(), job.source()), (binding.digest(), a.source()));
    let sources: Vec<Vec<u8>> = binding.sources().iter().map(|(_, _, bytes)| bytes.to_vec()).collect();
    assert_eq!(job.job().sources, sources);
    assert_eq!(job.job().sources.iter().map(|s| sha256(s)).collect::<BTreeSet<_>>(), a.modules());
    assert_eq!((job.job().module.as_str(), job.job().entry.as_str()), ("m", "go"));
    assert_eq!(job.job().options.profile, c600_check::Profile::ToolAuthoring);

    let mut runtime = runtime();
    let (mut here, mut there) = (session(), session());
    let reference = run_bound_in_process(&job, &mut here, None);
    let child = runtime.run_bound(&job, &mut there, None).unwrap_or_else(|f| panic!("{f}\n{}", f.stderr));
    assert_same_end(&reference, &child);
    assert_eq!(output(&child), a.output());

    let built_in = Modules::new(
        Identity::full(),
        Pin::of("fixture-1", b"p3 fixture capability contract 1\n"),
        Pin::of("m1-draft", b"c600-check fixture\n"),
        Grant::none(),
    );
    let entries = [Entry { author: Author::BuiltIn, profile: Profile::Solving, ..entry("go") }];
    let c = built_in.check(&a.manifest, &a.lock, &a.files, &entries).unwrap();
    built_in.activate_module("m", None, c.digest, c.reference).unwrap();
    let refused = BoundJob::new(&built_in.bind("m", "go").unwrap(), vec![], JobOptions::new(WIDE)).unwrap_err();
    assert_eq!(refused.stage, Stage::Admit);
}

/// EV1.11 and the vector `old-run-survives-activation` on the child: R, bound to A, is suspended in the child at its
/// probe read while the host activates B with expected A. R resumes on A's bytes, its old source and dependency: `go`
/// returns A's sum and `fail` stops with a source range on A's root. A later job binds B, and another activation
/// expecting A is `EV1:stale.activation_base`.
#[test]
fn ct_ev1_08_a_run_suspended_in_the_child_finishes_on_a_after_b_activates() {
    let (a, b) = (Arc::new(version(10)), Arc::new(version(20)));
    let mut runtime = runtime();
    for function in ["go", "fail"] {
        let modules = Arc::new(modules());
        install(&modules, &a);
        let cb = check(&modules, &b);
        let r = job(&modules, function);
        let activation: Arc<Mutex<Option<Result<Activation, Refusal>>>> = Arc::default();
        let mut host = session();
        let (m, out, expected) = (Arc::clone(&modules), Arc::clone(&activation), Some(a.package_digest()));
        host.on_perform = Some(Box::new(move || {
            let mut out = out.lock().unwrap();
            if out.is_none() {
                assert_eq!(m.active("m"), expected, "R is bound to A and suspended in the child");
                *out = Some(m.activate_module("m", expected, cb.digest, cb.reference));
            }
        }));
        let ended = runtime.run_bound(&r, &mut host, None).unwrap_or_else(|f| panic!("{f}\n{}", f.stderr));
        let activation = activation.lock().unwrap().take().expect("R was suspended at a host call").unwrap();
        assert_eq!((activation.previous, activation.new), (Some(a.package_digest()), b.package_digest()));
        assert_eq!(r.closure(), a.package_digest(), "R's job names A");
        assert_eq!(runtime.spawns(), 1, "one child served every run");
        let later = job(&modules, function);
        assert_eq!(later.closure(), b.package_digest(), "a later job binds B");
        let after = runtime.run_bound(&later, &mut session(), None).unwrap();
        match function {
            "go" => {
                assert_eq!(output(&ended), a.output(), "R resumed with A's dependency");
                assert_eq!(output(&after), b.output());
            }
            _ => {
                assert_eq!(stop_digest(&ended), hex(&a.source()), "R's source range names A's root");
                assert_eq!(stop_digest(&after), hex(&b.source()));
            }
        }
        let c = version(30);
        let cc = check(&modules, &c);
        let stale = modules.activate_module("m", Some(a.package_digest()), cc.digest, cc.reference);
        assert_eq!(stale.map_err(|r| r.code).unwrap_err(), STALE_ACTIVATION_BASE);
        assert_eq!(modules.active("m"), Some(b.package_digest()));
    }
}

/// EV1.11: jobs built by threads that bind while the main thread activates B over A each carry one version wholly:
/// their closure, sources and entry are A's or B's, and each thread sees A until it sees B. A job built before the
/// switch still runs A after it, in the child and in the reference; an unchecked version builds no job.
#[test]
fn ct_ev1_08_bound_jobs_racing_activation_carry_one_version() {
    const THREADS: usize = 6;
    const JOBS: usize = 150;
    let (a, b) = (Arc::new(version(10)), Arc::new(version(20)));
    let modules = Arc::new(modules());
    assert_eq!(modules.bind("m", "go").map(|_| ()).unwrap_err().code, MODULE_NOT_CHECKED);
    install(&modules, &a);
    let early = job(&modules, "go");
    let cb = check(&modules, &b);
    let started = Arc::new(Barrier::new(THREADS + 1));
    let count = Arc::new(AtomicUsize::new(0));
    let threads: Vec<_> = (0..THREADS)
        .map(|_| {
            let (modules, a, b, started, count) =
                (Arc::clone(&modules), Arc::clone(&a), Arc::clone(&b), Arc::clone(&started), Arc::clone(&count));
            std::thread::spawn(move || {
                started.wait();
                let mut seen = Vec::with_capacity(JOBS);
                for _ in 0..JOBS {
                    let j = job(&modules, "go");
                    let v = if j.closure() == a.package_digest() { &a } else { &b };
                    assert_eq!(j.closure(), v.package_digest());
                    assert_eq!(j.source(), v.source());
                    assert_eq!(j.job().sources.iter().map(|s| sha256(s)).collect::<BTreeSet<_>>(), v.modules());
                    assert_eq!((j.job().module.as_str(), j.job().entry.as_str()), ("m", "go"));
                    seen.push(v.n);
                    count.fetch_add(1, Ordering::SeqCst);
                }
                seen
            })
        })
        .collect();
    started.wait();
    while count.load(Ordering::SeqCst) < THREADS * JOBS / 4 {
        std::thread::yield_now();
    }
    modules.activate_module("m", Some(a.package_digest()), cb.digest, cb.reference).unwrap();
    let mut both = BTreeSet::new();
    for thread in threads {
        let seen = thread.join().expect("a thread's jobs");
        let first_b = seen.iter().position(|&n| n == b.n).unwrap_or(seen.len());
        assert!(seen[..first_b].iter().all(|&n| n == a.n) && seen[first_b..].iter().all(|&n| n == b.n), "{seen:?}");
        both.extend(seen);
    }
    assert!(both.contains(&a.n) && both.contains(&b.n), "jobs were built on both sides of the switch");

    let mut runtime = runtime();
    let child = runtime.run_bound(&early, &mut session(), None).unwrap();
    assert_same_end(&run_bound_in_process(&early, &mut session(), None), &child);
    assert_eq!(output(&child), a.output(), "a job built before the switch runs A after it");
    assert_eq!(output(&runtime.run_bound(&job(&modules, "go"), &mut session(), None).unwrap()), b.output());
}

impl Version {
    /// The closure digest: the candidate's digest.
    fn package_digest(&self) -> [u8; 32] {
        c600_module::verify(&self.manifest, &self.lock, &self.files, &Identity::full()).unwrap().digest
    }
}
