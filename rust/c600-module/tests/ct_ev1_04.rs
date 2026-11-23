//! CT-EV1-04 (chapter 04; EV1.7, EV1.8, LG5.6): breaking a draft leaves the old digest running; every route refuses an
//! unchecked new digest.
//!
//! CT-EV1-04 (source, part: the broken drafts, and the routes of the module set in this process: invocation by name,
//! a route that names a digest, activation, and loading new bytes under the old approval; the host's routes, the
//! runtime's own process, a program job invoked from the palette and from a key, and a view source, are in
//! `c600-tools/tests/ev1_host.rs`, which with this file is the whole by union, the debugger's route (LP4-5) and a
//! recording's route (LP4-9) included).

mod ev1;
mod support;

use c600_digest::sha256;
use c600_ir::{EXEC_CONTRACT, IR_VERSION};
use c600_module::code::{DIGEST_MISMATCH, MODULE_NOT_CHECKED, STALE_ACTIVATION_BASE};
use c600_module::{load, Dependency, Files, Identity, Location, Lock, LockedModule, Manifest, RESOLVER};

use ev1::*;
use support::{module, package, Package, EDITION};

/// A version of `m` whose root is `root` and whose dependency is `dep`, with its manifest and lock written by hand, so
/// that a root that does not parse still has a package.
fn by_hand(root: &str, dep: &str) -> Package {
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
    let files = Files::new().with("src/root.6cs", root.as_bytes()).with("src/dep0.6cs", dep.as_bytes());
    let locked = |name: &str, bytes: &str| LockedModule {
        module: name.into(),
        source: sha256(bytes.as_bytes()),
        edition: EDITION.into(),
        ir_version: IR_VERSION.into(),
        exec_contract: EXEC_CONTRACT.into(),
    };
    let lock = Lock {
        root: sha256(&manifest),
        modules: vec![locked("m", root), locked("helper", dep)],
        locations: vec![
            Location { path: "src/root.6cs".into(), source: sha256(root.as_bytes()) },
            Location { path: "src/dep0.6cs".into(), source: sha256(dep.as_bytes()) },
        ],
        resolver: RESOLVER.into(),
    }
    .bytes();
    Package { manifest, lock, files }
}

const FAIL: &str = "export fn fail() -> Int effects [snapshot] {\n    \
                    let n = read_snapshot<Int>(\"probe\");\n    \
                    return exact_add(n, read_snapshot<Int>(\"refused\"));\n}\n";

/// EV1.7, LG5.6: three broken drafts of `m` (a syntax gap, a type error, an undeclared effect) are each refused by the
/// check with their own code and store nothing. A keeps running, by name, and its results and source ranges name A.
#[test]
fn ct_ev1_04_a_broken_draft_leaves_the_old_digest_running() {
    let a = version(10);
    let modules = modules();
    let ca = install(&modules, &a);
    let dep = helper(20);
    let syntax = module("m", &[], &[("helper", &dep)], "export fn go() -> Int effects [snapshot] {\n    let n = ");
    let typed = module(
        "m",
        &[],
        &[("helper", &dep)],
        &format!("export fn go() -> Int effects [] {{\n    return true;\n}}\n\n{FAIL}"),
    );
    let effect = module(
        "m",
        &[],
        &[("helper", &dep)],
        &format!("export fn go() -> Int effects [] {{\n    return read_snapshot<Int>(\"probe\");\n}}\n\n{FAIL}"),
    );
    let drafts =
        [(by_hand(&syntax, &dep), "LG1:"), (package(&typed, &[&dep]), "LG2:"), (package(&effect, &[&dep]), "LG3:")];
    for (draft, prefix) in drafts {
        let refused = modules.check(&draft.manifest, &draft.lock, &draft.files, &entries());
        let code = code(refused);
        assert!(code.starts_with(prefix), "{code} for a draft refused with {prefix}");
        assert_eq!(modules.candidate("m"), Some(ca.clone()), "a refused check stores nothing");
        assert_eq!(modules.active("m"), Some(a.closure()), "A stays active");
        let ran = run(&modules.bind("m", "go").unwrap(), &mut engine());
        assert_eq!((ran.closure, ran.source, ran.outcome.run.output), (a.closure(), a.source(), Some(a.output())));
        assert_eq!(stop_digest(&run(&modules.bind("m", "fail").unwrap(), &mut engine())), hex_of(a.source()));
    }
}

/// EV1.7, EV1.8, EV1.11: B, a new digest of `m` that no check covers, runs on no route in this process. Activation
/// refuses it; an invocation by name binds A; a route that names B's closure or source digest is refused; B's bytes
/// never load under A's approval. Once B is checked and not yet activated, a route that names it is stale and the
/// invocation by name still binds A. Before any activation nothing runs.
#[test]
fn ct_ev1_04_every_route_refuses_an_unchecked_new_digest() {
    let (a, b) = (version(10), version(20));
    let modules = modules();
    assert_eq!(code(modules.bind("m", "go")), MODULE_NOT_CHECKED, "nothing checked, nothing runs");
    assert_eq!(code(modules.bind_pinned("m", "go", a.closure())), MODULE_NOT_CHECKED);
    let ca = install(&modules, &a);

    let unchecked = b.package.verify().unwrap();
    let refusals = [
        ("activation", code(modules.activate_module("m", Some(a.closure()), unchecked.digest, ca.reference))),
        ("a route naming B's closure", code(modules.bind_pinned("m", "go", b.closure()))),
        ("a route naming B's source", code(modules.bind_pinned("m", "go", b.source()))),
        ("an entry no check covers", code(modules.bind("m", "value"))),
    ];
    for (route, code) in refusals {
        assert_eq!(code, MODULE_NOT_CHECKED, "{route}");
    }
    let a_binding = modules.bind("m", "go").unwrap();
    let loaded = load(&unchecked, a_binding.approval()).err().map(|r| r.code);
    assert_eq!(loaded, Some(DIGEST_MISMATCH), "B's bytes under A's approval");
    for digest in [a.closure(), a.source()] {
        assert_eq!(modules.bind_pinned("m", "go", digest).unwrap().digest(), a.closure(), "A's digest names A");
    }
    assert_eq!(run(&a_binding, &mut engine()).outcome.run.output, Some(a.output()));

    check(&modules, &b);
    for digest in [b.closure(), b.source()] {
        assert_eq!(code(modules.bind_pinned("m", "go", digest)), STALE_ACTIVATION_BASE, "checked, not active");
    }
    let ran = run(&modules.bind("m", "go").unwrap(), &mut engine());
    assert_eq!((ran.closure, ran.outcome.run.output), (a.closure(), Some(a.output())));
    assert_eq!(modules.active("m"), Some(a.closure()));
}
