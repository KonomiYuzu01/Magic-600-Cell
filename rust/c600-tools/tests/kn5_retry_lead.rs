//! A lead on KN5.21 to KN5.23 (the receipt, the retry and the outcome lookup), tested from `c600-tools` against the
//! command layer. LP3 found it before KN5's one implementation, which closed it under ruling 4 (in `c600-command`'s
//! documentation; an owner decision under PHS.52 since 16 November 2026); the test now checks that it stays closed. It
//! is a lead test, not a conformance pass.

mod process_support;

use c600_command::{code, draft_args, Caller, Invocation, Secret, Status, Token};
use c600_kernel::json::{self, Value as Json};
use c600_tools::tool::ROW;
use process_support::{chunked, fingerprint, secret, session};

/// The lead, closed. Before KN5's implementation the layer's request digest covered neither the token's authenticator
/// nor the caller, and the layer stored every answer, refusals included, as the outcome of its command ID. So an
/// envelope that reused a genuine `operation.execute` envelope's command ID, arguments and binding, and was refused
/// before it authenticated, took the genuine envelope's place in the retry record, and the genuine envelope never
/// committed under its command ID. Under ruling 4 the host stamps the caller from the channel an envelope came on,
/// the request digest covers that caller, command IDs are scoped per caller, and the refusal of an envelope that does
/// not authenticate is kept for the epoch under the envelope's full digest, authenticator included, and never takes the
/// command ID:
///
/// - a shell envelope whose token was minted with another secret is refused `KN5:auth.token_invalid`;
/// - the same envelope sent on a run's channel is refused `KN5:capability.commit_row_refused`.
///
/// In both cases nothing changes and the refusal is not filed under the genuine envelope's request digest. The genuine
/// envelope, sent afterwards, commits under its command ID; a retry of it gets the same receipt, the forged envelope
/// sent again gets its own refusal again, and neither commits anything more. The control forges the same token under
/// another command ID.
///
/// KN5.21 to KN5.23, LI.5, LI.6 and LI.30 (CT-KN5-04's "a retried delivery commits at most once", and the retry record
/// of a command ID is written only by an envelope that authenticated).
#[test]
fn kn5_21_lead_closed_a_refused_forgery_leaves_the_command_id_to_the_genuine_envelope() {
    let recipe = chunked(&[&[1, -2]]);
    for case in ["forged token", "runtime caller", "control"] {
        let (mut layer, mut shell) = session("lead");
        let admitted = shell.invoke(&mut layer, ROW, draft_args(&recipe)).unwrap();
        let Status::Running(id) = admitted.status else { panic!("{case}: {admitted:?}") };
        assert_eq!(layer.run_jobs(), 1);
        shell.present(&mut layer, id).unwrap();
        let before = fingerprint(&layer);
        let args = json::object([("preview", Json::Int(id as i64))]);
        let genuine = shell.prepare(&layer, "operation.execute", args).unwrap();
        let token = genuine.token.clone().expect("the shell mints a token for a commit");
        assert!(token.verify(&secret()));
        let other = Token::mint(&Secret::fixed([6; 32]), token.binding.clone());
        assert!(!other.verify(&secret()), "a token minted with another secret does not authenticate");

        let mut forged = Invocation { token: Some(other), ..genuine.clone() };
        let mut channel = Caller::Shell;
        let expected = match case {
            "forged token" => code::TOKEN_INVALID,
            "runtime caller" => {
                channel = Caller::Runtime { run: "forger".into() };
                code::COMMIT_ROW_REFUSED
            }
            _ => {
                forged.command = "another-command".into();
                code::TOKEN_INVALID
            }
        };
        let refusal = layer.invoke(&channel, forged.clone());
        assert_eq!(refusal.status.code(), Some(expected), "{case}");
        assert_eq!(fingerprint(&layer), before, "{case}: a refused envelope changes nothing");

        let answer = shell.send(&mut layer, genuine.clone());
        assert!(matches!(answer.status, Status::Committed(_)), "{case}: the genuine envelope commits: {answer:?}");
        assert_ne!(refusal.request, answer.request, "{case}: the refusal is not filed under the genuine digest");
        assert_eq!(layer.outcome(&genuine.command, &answer.request), Some(answer.clone()), "{case}");
        let after = fingerprint(&layer);
        assert_ne!(after, before, "{case}: the commit moved the session");
        assert_eq!(shell.send(&mut layer, genuine), answer, "{case}: a retry gets the same receipt");
        assert_eq!(layer.invoke(&channel, forged), refusal, "{case}: the forged envelope gets its own refusal again");
        assert_eq!(fingerprint(&layer), after, "{case}: neither retry commits anything more");
    }
}
