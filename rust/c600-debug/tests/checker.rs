//! Checker stepping (EV2.6; LP4-6): the parts of CT-EV2-07 this build can run. No checker in the workspace writes
//! KN6.4's per-claim trace yet, so these steps run over a trace this test builds, as a stored checker output would
//! hold it. Each step binds the certificate and checker digests; a missing trace is unavailable, never a passed check;
//! the debugger imports no producer code (`tests/dependencies.rs`).

mod support;

use std::collections::BTreeMap;

use c600_debug::checker::{ClaimStep, Stepping, Trace, Traces};
use c600_debug::code::REPLAY_DEPENDENCY_MISSING;
use c600_debug::profile::Alphabet;
use c600_debug::words::{Carrier, Letter};
use support::identity;

#[derive(Default)]
struct Store(BTreeMap<(String, String), Trace>);

impl Traces for Store {
    fn trace(&self, certificate: &str, checker: &str) -> Option<Trace> {
        self.0.get(&(certificate.to_owned(), checker.to_owned())).cloned()
    }
}

const CERTIFICATE: &str = "c1c1c1";
const CHECKER: &str = "c2c2c2";

fn trace(word: &Carrier) -> Trace {
    let step = |claim: &str, outcome: &str, words| ClaimStep {
        certificate: CERTIFICATE.into(),
        checker: CHECKER.into(),
        claim: claim.into(),
        inputs: vec![format!("word:{}", word.digest())],
        outcome: outcome.into(),
        words,
    };
    Trace {
        certificate: CERTIFICATE.into(),
        checker: CHECKER.into(),
        steps: vec![
            step("legal_letters", "holds", vec![word.reference(0, 4)]),
            step("returns_home", "unestablished", vec![word.reference(2, 4)]),
        ],
    }
}

#[test]
fn ct_ev2_07_part_trace_steps_bind_their_digests() {
    let letters: Vec<Letter> = [1, -2, 3, 4].iter().map(|&g| Letter::Generator(g)).collect();
    let word = Carrier::literal(&letters, Alphabet::RetainedGenerators, identity(), None).unwrap();
    let mut store = Store::default();
    store.0.insert((CERTIFICATE.into(), CHECKER.into()), trace(&word));

    let mut stepping = Stepping::open(&store, CERTIFICATE, CHECKER).unwrap();
    let first = stepping.step().unwrap().clone();
    assert_eq!((first.certificate.as_str(), first.checker.as_str()), (CERTIFICATE, CHECKER));
    assert_eq!((first.claim.as_str(), first.outcome.as_str()), ("legal_letters", "holds"));
    assert_eq!(word.resolve(&first.words[0]).unwrap().letters, letters);
    let second = stepping.step().unwrap().clone();
    assert_eq!(second.outcome, "unestablished", "kept as the checker wrote it, not read as a failure");
    assert_eq!(word.resolve(&second.words[0]).unwrap().letters, letters[2..]);
    assert!(stepping.step().is_none());

    // A missing trace is unavailable, not a passed check; another checker's trace never stands in.
    let missing = Stepping::open(&store, CERTIFICATE, "other-checker").unwrap_err();
    assert_eq!(missing.code, REPLAY_DEPENDENCY_MISSING);
    assert!(missing.detail.contains("not a passed check"));

    // A trace that binds other digests than it is filed under, or a step that does, is refused whole.
    let mut misfiled = Store::default();
    misfiled.0.insert((CERTIFICATE.into(), "other-checker".into()), trace(&word));
    assert_eq!(Stepping::open(&misfiled, CERTIFICATE, "other-checker").unwrap_err().code, REPLAY_DEPENDENCY_MISSING);
    let mut tampered = trace(&word);
    tampered.steps[1].certificate = "c9c9c9".into();
    let mut store = Store::default();
    store.0.insert((CERTIFICATE.into(), CHECKER.into()), tampered);
    let refused = Stepping::open(&store, CERTIFICATE, CHECKER).unwrap_err();
    assert_eq!(refused.code, REPLAY_DEPENDENCY_MISSING);
    assert!(refused.detail.contains("step 1"));
}
