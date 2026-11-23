//! CT-EV1-08 (chapter 04; LP4 under PHS.26 and PHS.62; EV1.11): activation atomically changes A to B for later
//! invocations while a suspended run stays on A; a stale expected digest and an unchecked candidate refuse without
//! changing the active version. The run here follows the frozen vector `old-run-survives-activation` (FR-B06 of the
//! closure packet's p5 manifest): R runs checked A with its locked closure; checked B is activated with expected A
//! while R is suspended; activation returns previous A and new B; R resumes with A, its old source and dependencies; a
//! later invocation binds B; another activation expecting A is `EV1:stale.activation_base`; no source range is
//! rewritten.
//!
//! CT-EV1-08 (source, part: the activation service, a run suspended at a host call on the in-process executor, and
//! invocations racing activation on threads; the same run suspended in the runtime's own process and bound jobs racing
//! activation are in `c600-runtime/tests/ct_ev1_08_process.rs`, which with this file is the whole by union).

mod ev1;
mod support;

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};

use c600_check::Grant;
use c600_digest::sha256;
use c600_ir::Capability;
use c600_module::code::{CONTRACT_MISMATCH, MODULE_NOT_CHECKED, NOT_GRANTED, STALE_ACTIVATION_BASE};
use c600_module::{Activation, Binding, CheckReference, Modules, CHECK_REFERENCE_FORMAT};
use c600_run::provenance::Json;

use ev1::*;

fn assert_send_sync<T: Send + Sync>() {}

/// The set and a binding cross threads: a host shares one set between its invocations and its activations.
#[test]
fn ct_ev1_08_the_module_set_and_bindings_are_shared_across_threads() {
    assert_send_sync::<Modules>();
    assert_send_sync::<Binding>();
}

/// EV1.11: checking creates a candidate without changing the active module, the first activation installs it over
/// none, a later activation returns the previous and new digests, and later invocations bind the new one. Checking
/// again is no switch.
#[test]
fn ct_ev1_08_activation_changes_a_to_b_for_later_invocations() {
    let (a, b) = (version(10), version(20));
    let modules = modules();
    assert_eq!(code(modules.bind("m", "go")), MODULE_NOT_CHECKED, "nothing is active before an activation");

    let ca = check(&modules, &a);
    assert_eq!((ca.module.as_str(), ca.digest, ca.source), ("m", a.closure(), a.source()));
    assert_eq!(ca.reference, CheckReference(sha256(&ca.record)));
    assert_eq!(modules.active("m"), None, "a check creates a candidate and switches nothing");
    let first = modules.activate_module("m", None, ca.digest, ca.reference).unwrap();
    assert_eq!(first, Activation { module: "m".into(), previous: None, new: a.closure() });
    let ran = run(&modules.bind("m", "go").unwrap(), &mut engine());
    assert_eq!((ran.closure, ran.source), (a.closure(), a.source()));
    assert_eq!(ran.outcome.run.output, Some(a.output()));

    let cb = check(&modules, &b);
    assert_ne!(cb.digest, ca.digest);
    assert_eq!(modules.active("m"), Some(a.closure()), "checking B leaves A active");
    assert_eq!(run(&modules.bind("m", "go").unwrap(), &mut engine()).outcome.run.output, Some(a.output()));

    let switched = modules.activate_module("m", Some(a.closure()), cb.digest, cb.reference).unwrap();
    assert_eq!(switched, Activation { module: "m".into(), previous: Some(a.closure()), new: b.closure() });
    let ran = run(&modules.bind("m", "go").unwrap(), &mut engine());
    assert_eq!((ran.closure, ran.source, ran.outcome.run.output), (b.closure(), b.source(), Some(b.output())));
    // Activating the active candidate again over itself changes nothing.
    let again = modules.activate_module("m", Some(b.closure()), cb.digest, cb.reference).unwrap();
    assert_eq!((again.previous, again.new), (Some(b.closure()), b.closure()));
}

/// EV1.11: the check reference covers the candidate's source, manifest, lock closure, editions, execution contract
/// and checker version, and a check under other entries is another reference.
#[test]
fn ct_ev1_08_the_check_reference_covers_exactly_the_candidate() {
    let a = version(10);
    let modules = modules();
    let c = check(&modules, &a);
    let record = c600_run::provenance::parse(&c.record).expect("canonical JSON");
    assert_eq!(record.get("format"), Some(&Json::text(CHECK_REFERENCE_FORMAT)));
    let payload = record.get("payload").unwrap();
    for field in ["approvals", "checker", "closure", "contract", "ir", "manifest", "module", "modules"] {
        assert!(payload.get(field).is_some(), "the record names {field}");
    }
    let text = String::from_utf8(c.record.clone()).unwrap();
    for digest in a.modules().into_iter().chain([a.closure(), sha256(&a.package.manifest)]) {
        assert!(text.contains(&hex_of(digest)), "the record names {}", hex_of(digest));
    }
    assert!(text.contains("6cs-draft-1"), "the record names the editions");

    let only_go = check_with(&modules, &a, &[own("go")]).unwrap();
    assert_eq!(only_go.digest, c.digest, "the same closure");
    assert_ne!(only_go.reference, c.reference, "under other entries the check is another one");
    let other_pins = Modules::new(
        c600_module::Identity::full(),
        support::contract(),
        c600_run::provenance::Pin::of("m1-draft", b"another checker\n"),
        Grant::none(),
    );
    assert_ne!(check(&other_pins, &a).reference, c.reference, "another checker version is another check");

    // An entry outside the root module, a check of no entry and an entry twice are refused, and store nothing.
    let mut stray = own("value");
    stray.module = "helper".into();
    assert_eq!(code(check_with(&modules, &a, &[stray])), CONTRACT_MISMATCH);
    assert_eq!(code(check_with(&modules, &a, &[])), CONTRACT_MISMATCH);
    assert_eq!(code(check_with(&modules, &a, &[own("go"), own("go")])), CONTRACT_MISMATCH);
    assert_eq!(modules.candidate("m").unwrap().reference, only_go.reference);
}

/// EV1.11: every refusal leaves A active and later invocations on A. An unchecked candidate, a reference of another
/// check, a candidate of another module and a candidate a later check replaced are `EV1:input.module_not_checked`; a
/// grant the host no longer holds is `LG3:capability.not_granted`; another active digest than the expected one is
/// `EV1:stale.activation_base`.
#[test]
fn ct_ev1_08_every_refusal_leaves_a_active() {
    let (a, b, c) = (version(10), version(20), version(30));
    let modules = modules();
    let ca = install(&modules, &a);
    let still_a = |why: &str| {
        assert_eq!(modules.active("m"), Some(a.closure()), "{why}");
        let ran = run(&modules.bind("m", "go").unwrap(), &mut engine());
        assert_eq!((ran.closure, ran.outcome.run.output), (a.closure(), Some(a.output())), "{why}");
    };

    // C is verified but never checked.
    let unchecked = c.closure();
    assert_eq!(code(modules.activate_module("m", Some(a.closure()), unchecked, ca.reference)), MODULE_NOT_CHECKED);
    still_a("an unchecked candidate");

    let cb = check(&modules, &b);
    let forged = CheckReference([0; 32]);
    assert_eq!(code(modules.activate_module("m", Some(a.closure()), cb.digest, forged)), MODULE_NOT_CHECKED);
    still_a("a forged reference");
    assert_eq!(code(modules.activate_module("m", Some(a.closure()), cb.digest, ca.reference)), MODULE_NOT_CHECKED);
    still_a("A's reference for B");
    assert_eq!(code(modules.activate_module("n", None, cb.digest, cb.reference)), MODULE_NOT_CHECKED);
    still_a("B as another module");
    assert_eq!(modules.active("n"), None);

    assert_eq!(code(modules.activate_module("m", None, cb.digest, cb.reference)), STALE_ACTIVATION_BASE);
    still_a("expected none while A is active");
    assert_eq!(code(modules.activate_module("m", Some(b.closure()), cb.digest, cb.reference)), STALE_ACTIVATION_BASE);
    still_a("expected B while A is active");

    // A later check replaces B as the candidate, so B's check is no longer held.
    let cc = check(&modules, &c);
    assert_eq!(code(modules.activate_module("m", Some(a.closure()), cb.digest, cb.reference)), MODULE_NOT_CHECKED);
    still_a("a replaced candidate");

    // Grants: a version checked with a clock grant activates only while the host grants the clock.
    modules.set_grant(Grant::of(&[Capability::Clock]));
    let clock = |function: &str| {
        let mut entry = own(function);
        entry.grant = Grant::of(&[Capability::Clock]);
        entry
    };
    let cc_clock = check_with(&modules, &c, &[clock("go"), own("fail")]).unwrap();
    assert_ne!(cc_clock.reference, cc.reference);
    modules.set_grant(Grant::none());
    let refused = modules.activate_module("m", Some(a.closure()), cc_clock.digest, cc_clock.reference);
    assert_eq!(code(refused), NOT_GRANTED);
    still_a("a revoked activation grant");

    // With the grant back, C activates; with it revoked again, C's clock entry no longer binds, its other entry does,
    // and a binding made before the revocation keeps its version.
    modules.set_grant(Grant::of(&[Capability::Clock]));
    let before = modules.bind("m", "fail").unwrap();
    modules.activate_module("m", Some(a.closure()), cc_clock.digest, cc_clock.reference).unwrap();
    let go = modules.bind("m", "go").unwrap();
    modules.set_grant(Grant::none());
    assert_eq!(code(modules.bind("m", "go")), NOT_GRANTED, "an invocation checks its own grant");
    assert_eq!(modules.bind("m", "fail").unwrap().digest(), c.closure());
    assert_eq!(run(&go, &mut engine()).outcome.run.output, Some(c.output()));
    assert_eq!(before.digest(), a.closure());
    assert_eq!(stop_digest(&run(&before, &mut engine())), hex_of(a.source()));
}

/// EV1.11 and the vector `old-run-survives-activation`: R, bound to A, is suspended at its probe read while B
/// activates with expected A. Activation returns previous A and new B; R resumes on A, its old source and dependency,
/// and its stop's source range still names A's bytes; a later invocation binds B; another activation expecting A is
/// `EV1:stale.activation_base`.
#[test]
fn ct_ev1_08_a_suspended_run_on_a_finishes_on_a_after_b_activates() {
    let (a, b) = (version(10), version(20));
    for function in ["go", "fail"] {
        let modules = modules();
        install(&modules, &a);
        let cb = check(&modules, &b);
        let r = modules.bind("m", function).unwrap();
        let mut activation = None;
        let mut host = Suspending::new(|| {
            assert_eq!(modules.active("m"), Some(a.closure()), "R is bound to A and suspended");
            activation = Some(modules.activate_module("m", Some(a.closure()), cb.digest, cb.reference));
        });
        let ran = run(&r, &mut host);
        assert!(host.ran(), "R was suspended at its probe read");
        drop(host);
        let activation = activation.unwrap().unwrap();
        assert_eq!((activation.previous, activation.new), (Some(a.closure()), b.closure()));
        assert_eq!((ran.closure, ran.source), (a.closure(), a.source()), "R names A");
        assert_eq!(r.approval().closure, a.closure(), "R's approval stays A's");
        assert_eq!(r.sources().iter().map(|(_, d, _)| *d).collect::<BTreeSet<_>>(), a.modules());
        let later = modules.bind("m", function).unwrap();
        assert_eq!(later.digest(), b.closure(), "a later invocation binds B");
        match function {
            "go" => {
                assert_eq!(ran.outcome.run.output, Some(a.output()), "R resumed with A's dependency");
                assert_eq!(run(&later, &mut engine()).outcome.run.output, Some(b.output()));
            }
            _ => {
                assert_eq!(stop_digest(&ran), hex_of(a.source()), "R's source range is not rewritten");
                assert_eq!(stop_digest(&run(&later, &mut engine())), hex_of(b.source()));
            }
        }
        let c = version(30);
        let cc = check(&modules, &c);
        assert_eq!(
            code(modules.activate_module("m", Some(a.closure()), cc.digest, cc.reference)),
            STALE_ACTIVATION_BASE,
            "another activation expecting A"
        );
        assert_eq!(modules.active("m"), Some(b.closure()));
    }
}

/// EV1.11: the host keeps A's immutable bytes while a run bound to A can resume, whatever the set does after: here
/// the set is dropped and the binding still runs A.
#[test]
fn ct_ev1_08_a_binding_keeps_its_version_after_the_set_is_gone() {
    let (a, b) = (version(10), version(20));
    let modules = modules();
    install(&modules, &a);
    let r = modules.bind("m", "go").unwrap();
    install(&modules, &b);
    drop(modules);
    assert_eq!(run(&r, &mut engine()).outcome.run.output, Some(a.output()));
    let bytes: Vec<&[u8]> = r.sources().iter().map(|(_, _, bytes)| *bytes).collect();
    assert!(bytes.contains(&a.root.as_bytes()) && bytes.contains(&a.dep.as_bytes()));
}

/// EV1.11: an invocation racing activation binds wholly before or after the switch, never to a mixture. Eight threads
/// bind and run `go` while the main thread activates B over A. Every binding's closure, approval, sources and result
/// belong to one version, and each thread sees A until it sees B, never A after B.
#[test]
fn ct_ev1_08_an_invocation_racing_activation_binds_wholly_to_one_side() {
    const THREADS: usize = 8;
    const RUNS: usize = 120;
    let (a, b) = (Arc::new(version(10)), Arc::new(version(20)));
    let modules = Arc::new(modules());
    install(&modules, &a);
    let cb = check(&modules, &b);
    let started = Arc::new(Barrier::new(THREADS + 1));
    let count = Arc::new(AtomicUsize::new(0));
    let threads: Vec<_> = (0..THREADS)
        .map(|_| {
            let (modules, a, b, started, count) =
                (Arc::clone(&modules), Arc::clone(&a), Arc::clone(&b), Arc::clone(&started), Arc::clone(&count));
            std::thread::spawn(move || {
                started.wait();
                let mut seen = Vec::with_capacity(RUNS);
                for _ in 0..RUNS {
                    let binding = modules.bind("m", "go").unwrap();
                    let v = if binding.digest() == a.closure() { &a } else { &b };
                    assert_eq!(binding.digest(), v.closure(), "a binding names A or B");
                    assert_eq!(binding.source(), v.source());
                    assert_eq!(binding.approval().closure, v.closure());
                    assert_eq!(binding.sources().iter().map(|(_, d, _)| *d).collect::<BTreeSet<_>>(), v.modules());
                    let ran = run(&binding, &mut engine());
                    assert_eq!((ran.closure, ran.outcome.run.output), (v.closure(), Some(v.output())));
                    seen.push(v.n);
                    count.fetch_add(1, Ordering::SeqCst);
                }
                seen
            })
        })
        .collect();
    started.wait();
    while count.load(Ordering::SeqCst) < THREADS * RUNS / 4 {
        std::thread::yield_now();
    }
    let switched = modules.activate_module("m", Some(a.closure()), cb.digest, cb.reference).unwrap();
    assert_eq!(switched.previous, Some(a.closure()));
    let mut both = BTreeSet::new();
    for thread in threads {
        let seen = thread.join().expect("a thread's runs");
        let first_b = seen.iter().position(|&n| n == b.n).unwrap_or(seen.len());
        assert!(seen[..first_b].iter().all(|&n| n == a.n) && seen[first_b..].iter().all(|&n| n == b.n), "{seen:?}");
        both.extend(seen);
    }
    assert!(both.contains(&a.n) && both.contains(&b.n), "invocations bound on both sides of the switch");
    assert_eq!(run(&modules.bind("m", "go").unwrap(), &mut engine()).outcome.run.output, Some(b.output()));
}
