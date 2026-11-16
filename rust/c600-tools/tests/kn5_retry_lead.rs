//! A lead on KN5.21 to KN5.23 (the receipt, the retry and the outcome lookup), tested from `c600-tools` against the
//! command layer as it stands; `c600-command` is not changed here. The test records what the layer does today. It is
//! a finding for KN5's one implementation, not a conformance pass.

mod process_support;

use c600_command::{code, draft_args, Caller, Invocation, Secret, Status, Token};
use c600_kernel::json::{self, Value as Json};
use c600_tools::tool::ROW;
use process_support::{chunked, fingerprint, secret, session};

/// The lead, confirmed. The layer's request digest covers the session, the epoch, the command ID, the row, the
/// arguments and the token's binding; it covers neither the token's authenticator nor the caller. The layer stores
/// every answer, refusals included, as the outcome of its command ID. So an envelope that reuses a genuine
/// `operation.execute` envelope's command ID, arguments and binding, and is refused before it authenticates, takes
/// the genuine envelope's place in the retry record:
///
/// - a shell envelope whose token was minted with another secret is refused `KN5:auth.token_invalid`;
/// - a runtime envelope is refused `KN5:capability.commit_row_refused`.
///
/// In both cases nothing changes, the refusal has the genuine envelope's request digest, and the genuine envelope,
/// sent afterwards for the first time, is answered by the outcome lookup with that stored refusal: it never commits
/// under its command ID, however often it is retried. The token is not spent and the preview stays presented, so
/// the shell's next invocation, with a fresh command ID, commits. The control forges the same token under another
/// command ID: the genuine envelope then commits.
///
/// KN5.21 to KN5.23, LI.5, LI.6 and LI.30 (CT-KN5-04's "a retried delivery commits at most once" holds; what fails
/// is that the retry record of a command ID can be written by an envelope that never authenticated). When KN5's
/// implementation closes this, the two forged cases answer the genuine envelope with a commit, and this test's
/// expectation for them changes with it.
#[test]
fn kn5_21_lead_a_refused_forgery_under_a_genuine_command_id_answers_the_genuine_envelope() {
    let recipe = chunked(&[&[1, -2]]);
    for case in ["forged token", "runtime caller", "control"] {
        let (mut layer, mut shell) = session("lead");
        let admitted = shell.invoke(&mut layer, ROW, draft_args(&recipe)).unwrap();
        let Status::Running(id) = admitted.status else { panic!("{case}: {admitted:?}") };
        assert_eq!(layer.run_jobs(), 1);
        shell.present(&mut layer, id).unwrap();
        let before = fingerprint(&layer);
        let args = json::object([("preview", Json::Int(id as i64))]);
        let genuine = shell.prepare(&layer, "operation.execute", args.clone()).unwrap();
        let token = genuine.token.clone().expect("the shell mints a token for a commit");
        assert!(token.verify(&secret()));
        let other = Token::mint(&Secret::fixed([6; 32]), token.binding.clone());
        assert!(!other.verify(&secret()), "a token minted with another secret does not authenticate");

        let mut forged = Invocation { token: Some(other), ..genuine.clone() };
        let expected = match case {
            "forged token" => code::TOKEN_INVALID,
            "runtime caller" => {
                forged.caller = Caller::Runtime { run: "forger".into() };
                code::COMMIT_ROW_REFUSED
            }
            _ => {
                forged.command = "another-command".into();
                code::TOKEN_INVALID
            }
        };
        let refusal = layer.invoke(forged);
        assert_eq!(refusal.status.code(), Some(expected), "{case}");
        assert_eq!(fingerprint(&layer), before, "{case}: a refused envelope changes nothing");

        let answer = shell.send(&mut layer, genuine.clone());
        if case == "control" {
            assert!(matches!(answer.status, Status::Committed(_)), "{case}: {answer:?}");
            continue;
        }
        assert_eq!(refusal.request, answer.request, "{case}: the forged envelope has the genuine request digest");
        assert_eq!(answer, refusal, "{case}: the genuine envelope is answered with the forged envelope's refusal");
        assert_eq!(layer.outcome(&genuine.command, &answer.request), Some(refusal.clone()), "{case}");
        assert_eq!(shell.send(&mut layer, genuine), refusal, "{case}: every retry gets the stored refusal");
        assert_eq!(fingerprint(&layer), before, "{case}: the genuine envelope committed nothing");

        let fresh = shell.invoke(&mut layer, "operation.execute", args).unwrap();
        assert!(matches!(fresh.status, Status::Committed(_)), "{case}: a fresh command ID commits: {fresh:?}");
    }
}
