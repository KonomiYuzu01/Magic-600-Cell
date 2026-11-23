//! CT-LG5-02 (part: a certificate's declared manifest, edition and requirements at a check job). The LP2 half
//! (`c600-module/tests/conformance_lg5.rs`) ran a module's manifest, edition and transitive requirements as data for
//! the module system; this half shows the same at a check job: what a certificate declares is a claim the checker
//! judges and grants nothing. The job's edition, grants, profile and policy are the host's, a certificate that names an
//! input the host did not grant fails as an absent input, and a stored record re-runs only under the edition and grants
//! it was sealed with.

use c600_checkers::{CheckerKind, Verdict};
use c600_research::record::{admission_now, EDITION};
use c600_research::Check;
use c600_session::persist::codec::Json;
use c600_session::persist::run::{
    codes, policy_identity, reproduce as reproduce_stored, Admission, Grant, Reproduction,
};

use crate::common::*;

/// CT-LG5-02 (part: declared fields are data): a certificate declaring an edition, requirements and grants fails as a
/// claim of the checker, and its job's admission is the plain job's.
#[test]
fn ct_lg5_02_declared_edition_requirements_and_grants_grant_nothing() {
    let declared = expansion_with(
        &[1, 2, 3],
        None,
        vec![
            ("edition", Json::text("6cs-draft-9")),
            ("requires", Json::List(vec![Json::text("kn6.expansion/2"), Json::text("research.search")])),
            ("grants", Json::List(vec![Json::text("file:corpus"), Json::text("clock")])),
        ],
    );
    let run = run_job(stated_job(vec![Check::new(CheckerKind::Expansion, declared, Vec::new())], WIDE));
    let report = run.children()[0].report.clone().expect("a report");
    match &report.verdict {
        Verdict::Fail { claim, reason, .. } => {
            assert_eq!(claim, "certificate");
            assert!(reason.contains("is unknown"), "{reason}");
        }
        other => panic!("not a failed claim: {other:?}"),
    }
    let plain = run_job(stated_job(vec![short_check(&[1, 2, 3])], WIDE));
    assert_eq!(Admission::of(run.key()), Admission::of(plain.key()), "the declaration changes no admission");
    let key = run.key();
    assert_eq!(key.edition, EDITION);
    assert!(key.parameters.grants.is_empty());
    assert_eq!(key.parameters.profile.name(), "research");
    assert_eq!(key.parameters.policy, policy_identity(&[]));
}

/// CT-LG5-02 (part: a requirement the host did not grant): a certificate that names a start input the job was not
/// granted fails as that input's absence; nothing is fetched for it.
#[test]
fn ct_lg5_02_an_ungranted_input_is_absent() {
    let start = b"{\"labels\":[]}".to_vec();
    let certificate = expansion(&[4, 5], Some((&start, &hex_of("s"), &hex_of("e"))));
    let run = run_job(stated_job(vec![Check::new(CheckerKind::Expansion, certificate, Vec::new())], WIDE));
    match &run.children()[0].report.as_ref().expect("a report").verdict {
        Verdict::Fail { claim, reason, .. } => {
            assert_eq!(claim, "inputs.start");
            assert!(reason.contains("absent"), "{reason}");
        }
        other => panic!("not a failed claim: {other:?}"),
    }
    assert!(run.key().parameters.captured.iter().all(|c| !c.name.contains("/input/")), "no input was captured");
}

/// CT-LG5-02 (part: the sealed edition and grants bind reproduction): the stored record re-runs under another
/// edition with `LG4:input.repro_key_mismatch`, and under an added grant or another policy with
/// `LG3:capability.grant_mismatch`, before anything runs.
#[test]
fn ct_lg5_02_reproduction_keeps_the_sealed_edition_and_grants() {
    let run = run_job(stated_job(vec![short_check(&[6, 7])], WIDE));
    let now = admission_now(run.key(), &engines()).expect("the host's admission");
    let attempt = |admission: &Admission| {
        reproduce_stored(run.bytes(), &run.available(), admission, |_, _| panic!("admission precedes execution"))
    };
    let mut edition = now.clone();
    edition.edition = "6cs-draft-9".into();
    assert_eq!(attempt(&edition), Reproduction::Refused(codes::KEY_MISMATCH.into()));
    let mut granted = now.clone();
    granted.grants.push(Grant {
        kind: "file".into(),
        rights: vec!["read".into()],
        scope: "corpus".into(),
        grantee: hex_of("g"),
    });
    assert_eq!(attempt(&granted), Reproduction::Refused(codes::GRANT_MISMATCH.into()));
    let mut policy = now.clone();
    policy.policy = policy_identity(&["research.declassify"]);
    assert_eq!(attempt(&policy), Reproduction::Refused(codes::GRANT_MISMATCH.into()));
    assert!(matches!(reproduce(&run).answer, Reproduction::Same(_)), "the host's own admission re-runs it");
}
