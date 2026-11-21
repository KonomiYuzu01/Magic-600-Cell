//! CT-ONE-24, a pinned job's result is not shown against a newer revision (ONE.19), joined across the session store and
//! the view model. Source evidence: no host, device or renderer is involved.

mod common;

use std::sync::{Arc, OnceLock};

use c600_check::check;
use c600_run::interp::{load, Loaded};
use c600_session::{code, Input, JobKind, JobSpec, Lane, Receipt, RevisionRef, Row, Session, Status};
use c600_syntax::{lower, parse};
use c600_view as view;
use common::{commit, finished, job, session, shell};

const BIG: u64 = 1_000_000;

/// Job programs in one module `m`: `tree` does 2^limit calls, so `slow` takes a while and `endless` never ends.
const PROGRAMS: &str = "\
edition \"6cs-draft-1\";
module m requires [];

fn tree(level: Nat, limit: Nat) -> Nat effects [] {
    return match exact_less(level, limit) {
        true => exact_add(tree(exact_add(level, 1), limit), tree(exact_add(level, 1), limit));
        _ => 1;
    };
}

export fn slow() -> Nat effects [snapshot] {
    let s = read_snapshot<Nat>(\"state\");
    return exact_add(s, tree(0, 16));
}

export fn endless() -> Nat effects [snapshot] {
    let s = read_snapshot<Nat>(\"state\");
    return exact_add(s, tree(0, 60));
}
";

fn programs() -> Arc<Loaded> {
    static LOADED: OnceLock<Arc<Loaded>> = OnceLock::new();
    Arc::clone(LOADED.get_or_init(|| {
        let file = parse(PROGRAMS.as_bytes());
        assert!(file.diagnostics.is_empty(), "{:#?}", file.diagnostics);
        let lowered = lower(&[file]).unwrap_or_else(|e| panic!("{e}"));
        let report = check(&lowered.program).unwrap_or_else(|e| panic!("{e}"));
        assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
        Arc::new(load(&lowered.program).unwrap_or_else(|r| panic!("{r:?}")))
    }))
}

/// A pinned job of one function above, on the current revision.
fn pinned(function: &str, lane: Lane) -> JobSpec {
    JobSpec { program: programs(), lane, inputs: vec![Input::Current], ..job(function, Vec::new(), JobKind::Pinned) }
}

fn submit(session: &Session, command: &str, spec: JobSpec) -> Receipt {
    let admitted = shell(session, command, Row::JobSubmit(spec));
    assert!(matches!(admitted.status, Status::Running(_)), "{command}: {admitted:?}");
    admitted
}

/// A session digest in hex as the view model's digest bytes.
fn bytes(hex: &str) -> view::Digest {
    assert_eq!(hex.len(), 64, "a SHA-256 in hex");
    let mut digest = [0u8; 32];
    for (byte, pair) in digest.iter_mut().zip(hex.as_bytes().chunks(2)) {
        *byte = std::str::from_utf8(pair).ok().and_then(|pair| u8::from_str_radix(pair, 16).ok()).expect("hex");
    }
    digest
}

/// The view model's revision of a session's, converted by hand: the types are the two crates' own.
fn revision(of: &RevisionRef) -> view::Revision {
    view::Revision::new(of.epoch, of.state, of.workspace, of.protection, bytes(&of.digest))
}

fn bundle(revision: view::Revision) -> view::BundleCandidate {
    let parts = view::Mode::JumbleOff
        .required()
        .iter()
        .map(|&kind| {
            let digest = c600_digest::sha256(format!("{kind:?} of {:?}", revision.digest).as_bytes());
            view::BundlePart { kind, revision, digest }
        })
        .collect();
    view::BundleCandidate { revision, mode: view::Mode::JumbleOff, parts }
}

fn profile() -> view::LimitProfile {
    let entries: Vec<(&str, u64, &str)> = view::Field::ALL
        .iter()
        .map(|field| (field.name(), if *field == view::Field::Overdraw { 1 } else { BIG }, field.unit()))
        .collect();
    view::LimitProfile::from_entries("fixture-1", &entries).expect("a complete fixture profile")
}

fn content(tag: u8) -> view::Content {
    view::Content {
        nodes: vec![vec![tag; 4]],
        plans: vec![view::PlanDescriptor { plan: 1, source_map: 7, instances: 1, ..view::PlanDescriptor::default() }],
        maps: vec![view::MapDescriptor { map: 7, entries: 1 }],
        source_bytes: 12,
    }
}

/// CT-ONE-24 (part: a pinned session job finishes after a commit on its original input, the result is refused against
/// the newer revision, and a view model whose base is the job's input refuses to publish it at the newer revision and
/// draws that view as stale; not run: a host that binds a job's result to a view (LP3), which this test stands in for
/// by hand). ONE.19 says a pinned job keeps its input after a commit and its handle stays open until the job ends; the
/// test asserts those two, and with them what KN7.3 and KN7.5 say of the digest, the clause's neighbours. ONE.19 says
/// nothing of showing the result after an undo, so the view half asserts nothing there.
#[test]
fn a_pinned_jobs_result_is_not_shown_against_the_newer_revision() {
    let (session, _disk) = session();
    commit(&session, "A", &[1]);
    let start = session.overview().revision;

    let endless = submit(&session, "J.endless", pinned("endless", Lane::Exclusive));
    let slow = submit(&session, "J.slow", pinned("slow", Lane::Program));

    // The view model at `start`, with one further view whose first run published at `start`.
    let mut model = view::ViewModel::new(
        view::Profile::Solving,
        profile(),
        &bundle(revision(&start)),
        view::Workload::default(),
        view::Workload::default(),
    )
    .expect("Global and Local fit");
    let id = view::ViewId::new("job.result");
    let declared = view::Declared {
        depends: view::Depends::STATE,
        workload: view::Workload { vertex_evals: 1, cpu_bytes: 1, gpu_bytes: 1, ..view::Workload::default() },
        output: view::OutputAllowance::new(BIG, 16),
        class: None,
    };
    let source = view::ViewSource::Overlay { row: "job.result".to_owned(), module: [7; 32], arguments: vec![1] };
    model
        .open_view(view::CommandId::Caller(1 << 20), view::Origin::Solver, id.clone(), source.clone(), declared)
        .expect("the view opens");
    model.pump(|_| view::RunOutcome::Published(content(1)));
    assert_eq!(model.adopted(), &revision(&start));

    // A commit; the host's model adopts the newer revision and does not run the view again yet.
    commit(&session, "C", &[2]);
    let newer = session.overview().revision;
    assert_ne!(newer.digest, start.digest);
    assert_eq!(model.adopt(&bundle(revision(&newer))).expect("a complete bundle"), revision(&newer));
    assert_eq!(model.adopted(), &revision(&newer));

    // ONE.19: the pinned job kept its input and ends on it; the other handle stays open.
    assert!(matches!(finished(&session, &slow).status, Status::Completed(_)));
    let result = session.wait_job(slow.job.unwrap()).unwrap();
    assert_eq!(result.input, start.digest, "the pinned job finished on its original input");
    assert_eq!(session.job_result(endless.job.unwrap()).unwrap(), None, "the handle stays open until the job ends");
    assert_eq!(session.job_result_at(slow.job.unwrap(), &newer).unwrap_err().code, code::JOB_INPUT_DIGEST);
    assert_eq!(session.job_result_at(slow.job.unwrap(), &start).unwrap(), result);

    // Showing it: the base is the result's input at `start`'s counters. The view model refuses it at the newer
    // revision and draws the view as stale, not as output.
    let base =
        view::Base::new(view::Revision { digest: bytes(&result.input), ..revision(&start) }, view::Depends::STATE);
    let input = view::PublicationInput {
        generation: model.generation(&id).expect("the view is open"),
        base,
        content: content(2),
        transport: view::Transport::Canonical,
        output: view::OutputAllowance::new(BIG, 16),
        provenance: view::Provenance { profile: model.session_profile(), producer: source, class: None },
    };
    let refused = model.publish(view::CommandId::Caller(7), &id, input);
    assert!(matches!(&refused, view::Status::Refused(e) if e.code == view::code::STALE_DISPLAY_PLAN), "{refused:?}");
    let frame = model.frame_input();
    assert_eq!(frame.revision, revision(&newer));
    let drawn = frame.views.iter().find(|v| v.view == id).expect("the view is visible");
    assert!(
        matches!(&drawn.draw, view::Draw::Stale { base: shown } if shown.revision == revision(&start)),
        "{:?}",
        drawn.draw
    );

    // An undo back to a state whose digest is `start`'s: the session's result applies to it (KN7.3, KN7.5).
    let hint = session.overview().undo.expect("an action to undo");
    shell(&session, "U", Row::Undo { hint, confirm: false });
    let back = session.overview().revision;
    assert_eq!(back.digest, start.digest);
    assert_eq!(session.job_result_at(slow.job.unwrap(), &back).unwrap(), result);

    let stopped = shell(&session, "stop", Row::JobStop { job: endless.job.unwrap() });
    assert!(matches!(stopped.status, Status::Completed(_)), "{stopped:?}");
    assert!(matches!(finished(&session, &endless).status, Status::Cancelled(_)));
}
