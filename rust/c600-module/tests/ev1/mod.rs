//! Shared by the CT-EV1-04 and CT-EV1-08 tests: versions of one module `m` that imports a dependency `helper`, the
//! module set they are checked and activated in, and an engine that serves the probe row and can act while a run is
//! suspended at it.
//!
//! Every version of `m` has the same text and imports another `helper`, whose `value` returns the version's number,
//! so two versions differ only in their dependency's bytes, and a run's result says which dependency ran: `go` returns
//! the probe row's value plus `helper.value()`. `fail` reads the probe row and then a row the engine refuses, so its
//! stop carries the source range of the bytes that ran.

#![allow(dead_code)]

use std::collections::BTreeSet;

use c600_check::Grant;
use c600_digest::sha256;
use c600_module::{Binding, BoundRun, Candidate, Entry, Identity, Modules, Refusal, RunRequest};
use c600_run::boundary::Store;
use c600_run::interp::{Host, Options, Request, RowTariff, Sink};
use c600_run::provenance::{hex, Author, Profile};
use c600_run::value::Value;

use crate::support::{checker, contract, encoded, entry, module, package, Engine, Package, WIDE};

/// The probe row's value.
pub const PROBE: i64 = 5;

/// The dependency of version `n`: `helper.value()` returns `n`.
pub fn helper(n: i64) -> String {
    module("helper", &[], &[], &format!("export fn value() -> Int effects [] {{\n    return {n};\n}}\n"))
}

/// The entries of `m`: `go` and `fail`, each reading the probe row first.
pub const BODY: &str = "export fn go() -> Int effects [snapshot] {\n    \
                        let n = read_snapshot<Int>(\"probe\");\n    \
                        return exact_add(n, helper.value());\n}\n\n\
                        export fn fail() -> Int effects [snapshot] {\n    \
                        let n = read_snapshot<Int>(\"probe\");\n    \
                        return exact_add(n, read_snapshot<Int>(\"refused\"));\n}\n";

/// The root `m` importing `dep` as `helper`.
pub fn root(dep: &str) -> String {
    module("m", &[], &[("helper", dep)], BODY)
}

/// One version of `m`: its root and dependency sources and its package.
#[derive(Clone, Debug)]
pub struct Version {
    pub n: i64,
    pub root: String,
    pub dep: String,
    pub package: Package,
}

/// Version `n` of `m`.
pub fn version(n: i64) -> Version {
    let dep = helper(n);
    let root = root(&dep);
    let package = package(&root, &[&dep]);
    Version { n, root, dep, package }
}

impl Version {
    /// The closure digest, which is the candidate's digest.
    pub fn closure(&self) -> [u8; 32] {
        self.package.verify().unwrap().digest
    }

    /// The root source digest.
    pub fn source(&self) -> [u8; 32] {
        sha256(self.root.as_bytes())
    }

    /// The source digests of the version's modules.
    pub fn modules(&self) -> BTreeSet<[u8; 32]> {
        [sha256(self.root.as_bytes()), sha256(self.dep.as_bytes())].into_iter().collect()
    }

    /// The output of `go` on this version.
    pub fn output(&self) -> Vec<u8> {
        encoded(&Value::int(PROBE + self.n))
    }
}

/// A solver's own entry of `m` in tool authoring, with no grant and the interim policy.
pub fn own(function: &str) -> Entry {
    entry(function, Author::Own, Profile::ToolAuthoring)
}

/// The entries every check of `m` approves.
pub fn entries() -> Vec<Entry> {
    vec![own("go"), own("fail")]
}

/// An empty module set for Full's identity under the fixture pins, whose host grants nothing.
pub fn modules() -> Modules {
    Modules::new(Identity::full(), contract(), checker(), Grant::none())
}

/// Checks a version with [`entries`].
pub fn check(modules: &Modules, v: &Version) -> Candidate {
    check_with(modules, v, &entries()).unwrap_or_else(|r| panic!("{r:?}"))
}

pub fn check_with(modules: &Modules, v: &Version, entries: &[Entry]) -> Result<Candidate, Refusal> {
    modules.check(&v.package.manifest, &v.package.lock, &v.package.files, entries)
}

/// Checks a version and activates it over whatever is active.
pub fn install(modules: &Modules, v: &Version) -> Candidate {
    let candidate = check(modules, v);
    let active = modules.active("m");
    modules.activate_module("m", active, candidate.digest, candidate.reference).unwrap_or_else(|r| panic!("{r:?}"));
    candidate
}

/// The fixture engine with the probe row.
pub fn engine() -> Engine {
    let mut engine = Engine::fixture();
    engine.rows.insert("probe".into(), Value::int(PROBE));
    engine
}

/// A host over the fixture engine that runs `hook` once, inside the first read of the probe row: while the run is
/// suspended at that host call, before the engine answers it.
pub struct Suspending<'h> {
    pub engine: Engine,
    hook: Option<Box<dyn FnOnce() + 'h>>,
}

impl<'h> Suspending<'h> {
    pub fn new(hook: impl FnOnce() + 'h) -> Suspending<'h> {
        Suspending { engine: engine(), hook: Some(Box::new(hook)) }
    }

    /// Whether the hook ran.
    pub fn ran(&self) -> bool {
        self.hook.is_none()
    }
}

impl Host for Suspending<'_> {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        self.engine.tariff(request)
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        if matches!(request, Request::Snapshot { query: "probe", .. }) {
            if let Some(hook) = self.hook.take() {
                hook();
            }
        }
        self.engine.perform(request)
    }

    fn sink(&mut self, sink: &Sink, value: &Value, bytes: &[u8]) -> Result<(), &'static str> {
        self.engine.sink(sink, value, bytes)
    }
}

/// Runs a binding in this process as run `r` on base `rev-7`.
pub fn run(binding: &Binding, host: &mut dyn Host) -> BoundRun {
    let request = RunRequest {
        run: "r",
        base: "rev-7",
        args: vec![],
        inputs: vec![],
        pins: binding.pins(),
        options: Options::new(WIDE),
    };
    binding.run(request, host, &mut Store::default()).unwrap_or_else(|r| panic!("{r:?}"))
}

/// The source digest a run's stop names, in hex.
pub fn stop_digest(ran: &BoundRun) -> String {
    let stop = ran.outcome.run.summary.stop.as_ref().expect("a stop");
    stop.source.as_ref().expect("a source range").digest.clone()
}

pub fn hex_of(digest: [u8; 32]) -> String {
    hex(&digest)
}

pub fn code<T: std::fmt::Debug>(result: Result<T, Refusal>) -> &'static str {
    match result {
        Ok(v) => panic!("expected a refusal, got {v:?}"),
        Err(r) => r.code,
    }
}
