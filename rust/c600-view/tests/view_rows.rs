//! Contract tests of the view model, each named after its test ID (PHS.62).
//!
//! Every ceiling here is a declared fixture value, not a measured one (RM.35, RM.40). The evidence is source and
//! synthetic only: no device, renderer or timing is involved.

use c600_view::*;

const BIG: u64 = 1_000_000;

/// The command ID the command layer gives a solver's `view.open` in these tests; the view model keeps no receipt for
/// it (RPI.83).
const OPEN: CommandId = CommandId::Caller(BIG);

/// A complete fixture profile: every field generous, `overdraw` 1, then the overrides.
fn profile(overrides: &[(&str, u64)]) -> LimitProfile {
    let entries: Vec<(&str, u64, &str)> = Field::ALL
        .iter()
        .map(|field| {
            let default = if *field == Field::Overdraw { 1 } else { BIG };
            let value = overrides.iter().find(|(name, _)| *name == field.name()).map_or(default, |(_, value)| *value);
            (field.name(), value, field.unit())
        })
        .collect();
    LimitProfile::from_entries("fixture-1", &entries).expect("a complete fixture profile")
}

/// A revision of engine epoch 1.
fn rev(state: u64, workspace: u64, protection: u64, tag: u8) -> Revision {
    rev_in(1, state, workspace, protection, tag)
}

fn rev_in(epoch: u64, state: u64, workspace: u64, protection: u64, tag: u8) -> Revision {
    Revision::new(epoch, state, workspace, protection, [tag; 32])
}

fn part(kind: PartKind, revision: Revision) -> BundlePart {
    let mut digest = [0u8; 32];
    digest[0] = kind as u8;
    digest[1..9].copy_from_slice(&revision.state.to_be_bytes());
    digest[9..17].copy_from_slice(&revision.workspace.to_be_bytes());
    digest[17..25].copy_from_slice(&revision.protection.to_be_bytes());
    BundlePart { kind, revision, digest }
}

fn bundle(revision: Revision) -> BundleCandidate {
    let parts = Mode::JumbleOff.required().iter().map(|kind| part(*kind, revision)).collect();
    BundleCandidate { revision, mode: Mode::JumbleOff, parts }
}

/// A view model for a session in `session`'s profile.
fn model_in(session: Profile, profile: LimitProfile, revision: Revision) -> ViewModel {
    ViewModel::new(session, profile, &bundle(revision), Workload::default(), Workload::default())
        .expect("Global and Local fit")
}

fn model_at(profile: LimitProfile, revision: Revision) -> ViewModel {
    model_in(Profile::Solving, profile, revision)
}

fn workload(vertex_evals: u64, bytes: u64) -> Workload {
    Workload { vertex_evals, cpu_bytes: bytes, gpu_bytes: bytes, ..Workload::default() }
}

fn declared(depends: Depends, workload: Workload) -> Declared {
    Declared { depends, workload, output: OutputAllowance::new(BIG, 16), class: None }
}

fn overlay(name: &str) -> ViewSource {
    ViewSource::Overlay { row: name.to_string(), module: [7; 32], arguments: vec![1, 2, 3] }
}

fn open(model: &mut ViewModel, name: &str, depends: Depends) -> ViewId {
    let view = ViewId::new(name);
    model
        .open_view(OPEN, Origin::Solver, view.clone(), overlay(name), declared(depends, workload(1, 1)))
        .expect("it fits");
    view
}

/// Opens a view and takes its first run, which then holds the view's runner slot.
fn held_view(model: &mut ViewModel, name: &str) -> (ViewId, Job) {
    let view = open(model, name, Depends::STATE);
    let job = model.take_jobs().into_iter().find(|job| job.view == view).expect("the first run");
    (view, job)
}

/// A minimal plan-size descriptor: one plan whose instance count is the count under test, and its source map.
fn plans(instances: u64) -> Content {
    Content {
        nodes: vec![vec![1, 2, 3]],
        plans: vec![PlanDescriptor { plan: 1, source_map: 7, instances, ..PlanDescriptor::default() }],
        maps: vec![MapDescriptor { map: 7, entries: instances }],
        source_bytes: 12,
    }
}

fn content(tag: u8) -> Content {
    Content { nodes: vec![vec![tag; 4]], ..plans(1) }
}

/// Content whose canonical bytes bound to `base` are exactly `len` bytes long.
fn sized(base: &Base, len: usize) -> Content {
    (0..len)
        .map(|n| Content { nodes: vec![vec![9; n]], ..plans(1) })
        .find(|content| content.canonical_bytes(base).len() == len)
        .expect("some node length gives the exact size")
}

/// A standalone publication bound to the adopted revision, with the block a host would fill for a solver's own tool
/// in the session's profile.
fn publication(model: &ViewModel, view: &ViewId, content: Content, output: OutputAllowance) -> PublicationInput {
    PublicationInput {
        generation: model.generation(view).expect("the view is open"),
        base: Base::new(*model.adopted(), Depends::STATE),
        content,
        transport: Transport::Canonical,
        output,
        provenance: Provenance { profile: model.session_profile(), producer: overlay("host"), class: None },
    }
}

fn run_all(model: &mut ViewModel) -> usize {
    model.pump(|job| RunOutcome::Published(content(job.base.revision.state as u8)))
}

fn ceiling(error: &Error) -> (&'static str, u128, u128) {
    match error.context {
        Context::Ceiling { field, requested, ceiling } => (field, requested, ceiling),
        _ => panic!("no ceiling context: {error:?}"),
    }
}

fn permutations(items: Vec<BundlePart>) -> Vec<Vec<BundlePart>> {
    if items.len() <= 1 {
        return vec![items];
    }
    let mut out = Vec::new();
    for index in 0..items.len() {
        let mut rest = items.clone();
        let first = rest.remove(index);
        for mut tail in permutations(rest) {
            tail.insert(0, first);
            out.push(tail);
        }
    }
    out
}

// ----- CT-RPI-03

/// CT-RPI-03, first half: every order of the arrays of revisions 12 and 13 is offered to adoption after each arrival,
/// once as the newest array of each kind (which can mix revisions) and once as the arrays of one revision. Only
/// whole one-revision bundles are adopted, adoption never goes back, and a refresh with the complete newest bundle
/// always lands (RPI.2, RPI.9, ONE.60).
#[test]
fn ct_rpi_03_reordered_arrays_never_form_a_mixed_bundle() {
    let r11 = rev(11, 1, 1, 11);
    let r12 = rev(12, 1, 1, 12);
    let r13 = rev(13, 2, 1, 13);
    let arrivals: Vec<BundlePart> = bundle(r12).parts.into_iter().chain(bundle(r13).parts).collect();
    let orders = permutations(arrivals);
    assert_eq!(orders.len(), 720);
    let mut mixed_refusals = 0;
    for order in orders {
        let mut model = model_at(profile(&[]), r11);
        let mut received: Vec<BundlePart> = Vec::new();
        for arrival in order {
            received.push(arrival);
            let mut newest: Vec<BundlePart> = Vec::new();
            for part in received.iter().rev() {
                if !newest.iter().any(|kept| kept.kind == part.kind) {
                    newest.push(*part);
                }
            }
            let own: Vec<BundlePart> =
                received.iter().filter(|part| part.revision == arrival.revision).copied().collect();
            for parts in [newest, own] {
                let before = *model.adopted();
                let candidate = BundleCandidate { revision: arrival.revision, mode: Mode::JumbleOff, parts };
                match model.adopt(&candidate) {
                    Ok(adopted) => assert_eq!(adopted, arrival.revision),
                    Err(error) => {
                        assert_eq!(error.code, code::DISPLAY_BUNDLE_MISMATCH);
                        if matches!(error.context, Context::Bundle(BundleFault::Mismatched { .. })) {
                            mixed_refusals += 1;
                        }
                        assert_eq!(*model.adopted(), before);
                    }
                }
                let adopted = *model.adopted();
                assert_eq!(model.bundle().parts().len(), 3);
                assert!(model.bundle().parts().iter().all(|part| part.revision == adopted), "a mixed bundle");
                assert!(adopted.not_older_than(&before), "adoption went back");
            }
        }
        assert_eq!(model.adopt(&bundle(r13)), Ok(r13));
        assert_eq!(model.shown_revision(&ViewId::global()), Some(r13));
        assert_eq!(model.shown_revision(&ViewId::local()), Some(r13));
    }
    assert!(mixed_refusals > 0, "the vectors offered mixed candidates");
}

/// CT-RPI-03, second half: a program view whose run never finishes does not delay adoption. Global and Local show
/// each new revision at once, the stalled run is fenced, its late completion installs nothing, and the one pending
/// run starts at the newest revision (RPI.2, ONE.64, LI.23, RM.22).
#[test]
fn ct_rpi_03_stalled_program_view_does_not_block_adoption() {
    let mut model = model_at(profile(&[]), rev(11, 1, 1, 11));
    let (v, stalled) = held_view(&mut model, "v");
    for state in [12, 13] {
        let next = rev(state, 1, 1, state as u8);
        assert_eq!(model.adopt(&bundle(next)), Ok(next));
        assert_eq!(model.shown_revision(&ViewId::global()), Some(next));
        assert_eq!(model.shown_revision(&ViewId::local()), Some(next));
        assert_eq!(model.frame_input().revision, next);
    }
    assert!(model.is_fenced(stalled.job));
    assert_eq!(model.receipt(stalled.command), Some(&Status::Cancelled(CancelReason::ReplacedRevision)));
    let waiting = model.pending(&v).expect("one pending run");
    assert_eq!(model.status(&v), Some(ViewStatus::Busy { waiting, held_by: stalled.command }));
    assert!(model.take_jobs().is_empty());

    let late = model.finish(stalled.job, RunOutcome::Published(content(11)));
    assert_eq!(late, Some(Status::Cancelled(CancelReason::ReplacedRevision)));
    assert!(model.installed(&v).is_none());

    let jobs = model.take_jobs();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].command, waiting);
    assert_eq!(jobs[0].base.revision, rev(13, 1, 1, 13));
    assert!(matches!(model.finish(jobs[0].job, RunOutcome::Published(content(13))), Some(Status::Completed(_))));
    assert_eq!(model.shown_revision(&v), Some(rev(13, 1, 1, 13)));
}

// ----- CT-RPI-06

/// CT-RPI-06: a hidden view runs no program and rebuilds nothing while revisions pass; showing it checks its
/// dependencies once and runs once, at the newest revision (RPI.11, RPI.12, RM.25).
#[test]
fn ct_rpi_06_hidden_view_does_no_work_and_showing_runs_once_at_the_newest_revision() {
    let mut model = model_at(profile(&[]), rev(11, 1, 1, 11));
    let v = open(&mut model, "v", Depends::STATE);
    assert_eq!(run_all(&mut model), 1);
    let before = model.counters(&v).expect("open");
    assert_eq!((before.program_runs, before.installs), (1, 1));

    assert!(model.hide(&v));
    for state in 12..=15 {
        model.adopt(&bundle(rev(state, 1, 1, state as u8))).expect("adopted");
    }
    assert!(model.take_jobs().is_empty());
    assert_eq!(model.pending(&v), None);
    assert_eq!(model.counters(&v), Some(before));
    assert!(model.frame_input().views.iter().all(|view| view.view != v));

    assert_eq!(model.show(&v), Ok(true));
    let jobs = model.take_jobs();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].base.revision, rev(15, 1, 1, 15));
    model.finish(jobs[0].job, RunOutcome::Published(content(15)));
    assert!(model.take_jobs().is_empty());
    let after = model.counters(&v).expect("open");
    assert_eq!((after.program_runs, after.installs), (2, 2));
}

/// CT-RPI-06: a view whose declared dependencies did not move does no work, visible or after hide and show.
#[test]
fn ct_rpi_06_unchanged_view_does_no_work() {
    let mut model = model_at(profile(&[]), rev(11, 1, 1, 11));
    let v = open(&mut model, "v", Depends::STATE);
    run_all(&mut model);
    let before = model.counters(&v).expect("open");

    model.adopt(&bundle(rev(11, 2, 1, 11))).expect("a workspace change");
    model.adopt(&bundle(rev(11, 3, 2, 11))).expect("a protection change");
    assert!(model.take_jobs().is_empty());
    assert_eq!(model.counters(&v), Some(before));
    assert!(matches!(model.status(&v), Some(ViewStatus::Current { .. })));

    assert!(model.hide(&v));
    assert_eq!(model.show(&v), Ok(true));
    assert!(model.take_jobs().is_empty());
    assert_eq!(model.counters(&v), Some(before));
}

/// CT-RPI-06 and KN7.2: a burst of revisions and requests keeps one runner and one newest pending request; each
/// newer request supersedes the older one, and the view runs once more, at the newest revision (RPI.12, LI.23).
#[test]
fn ct_rpi_06_a_burst_runs_once_at_the_newest_revision() {
    let mut model = model_at(profile(&[]), rev(11, 1, 1, 11));
    let (v, held) = held_view(&mut model, "v");
    let mut pending = Vec::new();
    for state in 12..=20 {
        model.adopt(&bundle(rev(state, 1, 1, state as u8))).expect("adopted");
        pending.push(model.pending(&v).expect("one pending request"));
    }
    pending.dedup();
    assert_eq!(pending.len(), 1, "one pending request, never a queue per revision");

    let requests: Vec<CommandId> = (1..=5).map(CommandId::Caller).collect();
    for command in &requests {
        assert_eq!(model.invoke(*command, &v, OutputAllowance::new(BIG, 16)), Status::Accepted);
    }
    assert_eq!(model.receipt(pending[0]), Some(&Status::Cancelled(CancelReason::Superseded { by: requests[0] })));
    for pair in requests.windows(2) {
        assert_eq!(model.receipt(pair[0]), Some(&Status::Cancelled(CancelReason::Superseded { by: pair[1] })));
    }
    assert_eq!(model.pending(&v), Some(requests[4]));

    model.finish(held.job, RunOutcome::Published(content(11)));
    let mut bases = Vec::new();
    model.pump(|job| {
        bases.push((job.command, job.base.revision.state));
        RunOutcome::Published(content(20))
    });
    assert_eq!(bases, vec![(requests[4], 20)]);
    assert_eq!(model.counters(&v).map(|counters| counters.program_runs), Some(2));
}

// ----- CT-RPI-07

/// CT-RPI-07: adoption makes mismatched output stale. It is drawn only as its view's stale presentation naming its
/// base, never as output over the new revision, and a publication bound to the old base cannot attach (RPI.13,
/// ONE.12, ONE.63).
#[test]
fn ct_rpi_07_adoption_makes_mismatched_output_stale() {
    let r12 = rev(12, 1, 1, 12);
    let r13 = rev(13, 1, 1, 13);
    let mut model = model_at(profile(&[]), r12);
    let v = open(&mut model, "v", Depends::STATE);
    run_all(&mut model);
    let old = model.installed(&v).expect("installed").clone();
    let old_input = publication(&model, &v, content(2), OutputAllowance::new(BIG, 16));

    model.adopt(&bundle(r13)).expect("adopted");
    let refresh = model.runner(&v).map(|(_, command)| command);
    assert!(refresh.is_some());
    assert_eq!(model.status(&v), Some(ViewStatus::Stale { base: old.base, adopted: r13, refresh }));
    let frame = model.frame_input();
    assert_eq!(frame.revision, r13);
    for view in &frame.views {
        if let Draw::Output { base, .. } = &view.draw {
            assert!(base.matches(&r13), "output drawn over a revision it does not match");
        }
    }
    let drawn = frame.views.iter().find(|view| view.view == v).expect("visible");
    assert_eq!(drawn.draw, Draw::Stale { base: old.base });

    let Status::Refused(error) = model.publish(CommandId::Caller(1), &v, old_input) else { panic!("attached") };
    assert_eq!(error.code, code::STALE_DISPLAY_PLAN);
    assert_eq!(error.context, Context::Stale { base: old.base, adopted: r13 });

    run_all(&mut model);
    assert!(matches!(model.status(&v), Some(ViewStatus::Current { base, .. }) if base.revision == r13));
}

/// CT-RPI-07: each of charter D2's statuses, with its typed data (RPI.13).
#[test]
fn ct_rpi_07_every_d2_status_has_typed_data() {
    let r11 = rev(11, 1, 1, 11);
    let mut model = model_at(profile(&[]), r11);
    assert_eq!(model.status(&ViewId::global()), Some(ViewStatus::Empty { revision: r11, base: None }));

    let loading = open(&mut model, "loading", Depends::STATE);
    let (_, first) = model.runner(&loading).expect("started");
    assert_eq!(model.status(&loading), Some(ViewStatus::Loading { command: first, revision: r11 }));

    let busy = open(&mut model, "busy", Depends::STATE);
    let (_, held) = model.runner(&busy).expect("started");
    assert_eq!(model.take_jobs().len(), 2);
    let newer = CommandId::Caller(1);
    model.invoke(newer, &busy, OutputAllowance::new(BIG, 16));
    assert_eq!(model.status(&busy), Some(ViewStatus::Busy { waiting: newer, held_by: held }));
    assert_eq!(model.status(&loading), Some(ViewStatus::Loading { command: first, revision: r11 }));

    let failing = open(&mut model, "error", Depends::STATE);
    let (_, failed) = model.runner(&failing).expect("started");
    model.pump(|_| RunOutcome::Error(Error::stop(code::DISPLAY_PLAN_INVALID, Context::None)));
    let error = Error::stop(code::DISPLAY_PLAN_INVALID, Context::None);
    assert_eq!(model.status(&failing), Some(ViewStatus::Error { command: failed, error }));

    let empty = open(&mut model, "empty", Depends::STATE);
    model.pump(|_| RunOutcome::Published(Content::default()));
    let base = Base::new(r11, Depends::STATE);
    assert_eq!(model.status(&empty), Some(ViewStatus::Empty { revision: r11, base: Some(base) }));

    let current = open(&mut model, "current", Depends::STATE);
    model.pump(|_| RunOutcome::Published(content(1)));
    let publication = model.installed(&current).expect("installed").digest;
    assert_eq!(model.status(&current), Some(ViewStatus::Current { base, publication }));

    let r12 = rev(12, 1, 1, 12);
    model.adopt(&bundle(r12)).expect("adopted");
    let refresh = model.runner(&current).map(|(_, command)| command);
    assert_eq!(model.status(&current), Some(ViewStatus::Stale { base, adopted: r12, refresh }));
}

// ----- CT-RPI-22, limit half

/// CT-RPI-22, limit half: a plan one over a ceiling installs nothing, standalone or scheduled; the adopted bundle
/// stays as it was, the view keeps its earlier output, and a view without output shows an error, never an empty
/// result (RPI.48 to RPI.50, RM.12). The chart half needs the display-plan validator and is not built.
#[test]
fn ct_rpi_22_one_over_limit_plan_installs_nothing() {
    let mut model = model_at(profile(&[("instances", 4)]), rev(11, 1, 1, 11));
    let v = open(&mut model, "v", Depends::STATE);
    model.pump(|_| RunOutcome::Published(plans(4)));
    let kept = model.installed(&v).expect("four instances pass").clone();
    let bundle_before = model.bundle().clone();

    let input = publication(&model, &v, plans(5), OutputAllowance::new(BIG, 16));
    let Status::Refused(error) = model.publish(CommandId::Caller(1), &v, input) else { panic!("admitted") };
    assert_eq!((error.code, error.form), (code::DISPLAY_PLAN_CEILING, Form::Refusal));
    assert_eq!(ceiling(&error), ("instances", 5, 4));
    assert_eq!(model.installed(&v), Some(&kept));

    assert_eq!(model.invoke(CommandId::Caller(2), &v, OutputAllowance::new(BIG, 16)), Status::Running(JobId(2)));
    model.pump(|_| RunOutcome::Published(plans(5)));
    let Some(Status::Failed(error)) = model.receipt(CommandId::Caller(2)).cloned() else { panic!("installed") };
    assert_eq!((error.code, error.form), (code::DISPLAY_PLAN_CEILING, Form::Stop));
    assert_eq!(model.installed(&v), Some(&kept));
    assert_eq!(model.bundle(), &bundle_before);

    let w = open(&mut model, "w", Depends::STATE);
    model.pump(|_| RunOutcome::Published(plans(5)));
    assert!(
        matches!(model.status(&w), Some(ViewStatus::Error { error, .. }) if error.code == code::DISPLAY_PLAN_CEILING)
    );
    assert!(model.installed(&w).is_none());
}

// ----- CT-RPI-23

fn built_in(name: &str) -> ViewSource {
    ViewSource::BuiltIn { name: name.to_string() }
}

/// RPI.28's two chart evaluators as a view's declared chain, with the stage a frame reports for each: the built-in
/// stage, and a checked vertex stage (RPI.68). No stage checker is built, so the vertex stage is a declared fixture
/// chain; its `frame_work` is 10 * 3 + 10 * 1 * 1 = 40 at `overdraw` 1.
fn evaluators() -> [(Workload, StageUse); 2] {
    let built_in = Workload { vertex_evals: 10, cpu_bytes: 1, gpu_bytes: 1, pixels: 10, ..Workload::default() };
    [(built_in, StageUse::BuiltIn), (Workload { stage_ops: 3, fragment_ops: 1, ..built_in }, StageUse::User)]
}

/// What the next frame draws for `view`, when it is visible.
fn draw_of(model: &ViewModel, view: &ViewId) -> Option<Draw> {
    model.frame_input().views.into_iter().find(|drawn| drawn.view == *view).map(|drawn| drawn.draw)
}

/// CT-RPI-23, profiles: under each profile's fixture, Global and Local stay open, visible together and linked to the
/// adopted revision after every adoption, and neither can be hidden or closed; further views with a built-in source
/// and with an overlay source open, run and show each revision the same way in every profile (RPI.42, RPI.81, EV3.11,
/// ONE.65).
#[test]
fn ct_rpi_23_global_and_local_stay_linked_and_further_views_open_under_every_profile() {
    let fixed = [ViewId::global(), ViewId::local()];
    let further = [(ViewId::new("orbit"), built_in("orbit")), (ViewId::new("mine"), overlay("mine"))];
    let mut traces = Vec::new();
    for session in Profile::ALL {
        let mut model = model_in(session, profile(&[]), rev(11, 1, 1, 11));
        assert_eq!(model.session_profile(), session);
        let mut trace = Vec::new();
        for ((view, source), (work, _)) in further.iter().zip(evaluators()) {
            let opened =
                model.open_view(OPEN, Origin::Solver, view.clone(), source.clone(), declared(Depends::STATE, work));
            trace.push(format!("open {view:?}: {opened:?}"));
        }
        for state in 12..=14 {
            let next = rev(state, 1, 1, state as u8);
            model.adopt(&bundle(next)).expect("adopted");
            for view in &fixed {
                assert_eq!(model.shown_revision(view), Some(next), "{view:?} is not linked in {session:?}");
                assert!(model.is_visible(view));
                assert!(!model.hide(view));
                let error = model.close_view(CommandId::Caller(state), Origin::Solver, view).expect_err("fixed");
                assert_eq!(error.code, code::VIEW_NOT_CLOSABLE);
            }
            let frame = model.frame_input();
            assert_eq!(frame.revision, next);
            let linked: Vec<&ViewId> =
                frame.views.iter().filter(|drawn| drawn.bundle).map(|drawn| &drawn.view).collect();
            assert_eq!(linked, vec![&fixed[0], &fixed[1]]);
            run_all(&mut model);
            for (view, _) in &further {
                assert_eq!(model.shown_revision(view), Some(next));
                trace.push(format!("{view:?}: {:?} {:?}", model.generation(view), model.status(view)));
            }
            trace.push(format!("{:?}", model.usage()));
        }
        let open: Vec<ViewId> = fixed.iter().chain(further.iter().map(|(view, _)| view)).cloned().collect();
        assert_eq!(model.views(), open.as_slice());
        traces.push(trace);
    }
    assert!(traces.windows(2).all(|pair| pair[0] == pair[1]), "a profile changed linkage or admission");
}

/// CT-RPI-23, boundary: built-in output keeps the interim boundary under either chart evaluator. In every profile,
/// for every class of the interim reading (BND.69) and with the built-in stage or a checked vertex stage, a built-in
/// source's output is installed and drawn with the block the host filled: the session's profile, the built-in source
/// and its manifest class. The runner returns content only and cannot set the block; the vertex stage, and its
/// fallback to the built-in chain, change the drawing chain and never the block. A standalone publication keeps the
/// block it carries, so output of another profile is not relabelled (RPI.45, RPI.81, BND.22, BND.26, BND.27).
#[test]
fn ct_rpi_23_built_in_output_keeps_the_interim_boundary_under_either_chart_evaluator() {
    let view = ViewId::new("tool");
    let out = OutputAllowance::new(BIG, 16);
    let drawn = |model: &ViewModel, expected: StageUse, block: &Provenance| {
        assert_eq!(model.installed(&view).map(|installed| &installed.provenance), Some(block));
        let Some(Draw::Output { stage, provenance, .. }) = draw_of(model, &view) else { panic!("no output drawn") };
        assert_eq!((stage, &provenance), (expected, block));
    };
    for session in Profile::ALL {
        for (work, stage) in evaluators() {
            for class in BoundaryClass::ALL {
                let mut model = model_in(session, profile(&[("frame_work", 40)]), rev(11, 1, 1, 11));
                let decl = Declared { class: Some(class), ..declared(Depends::STATE, work) };
                model.open_view(OPEN, Origin::Solver, view.clone(), built_in("repair"), decl).expect("fits");
                let block = Provenance { profile: session, producer: built_in("repair"), class: Some(class) };
                model.pump(|job| RunOutcome::Published(content(job.job.0 as u8)));
                drawn(&model, stage, &block);
                model.adopt(&bundle(rev(12, 1, 1, 12))).expect("adopted");
                model.pump(|_| RunOutcome::Published(content(0xEE)));
                drawn(&model, stage, &block);

                let mut stage = stage;
                if stage == StageUse::User {
                    model.resize(&view, 20);
                    assert!(model.is_fallback(&view), "50 does not fit 40");
                    stage = StageUse::Fallback;
                    drawn(&model, stage, &block);
                }

                let other = Profile::ALL.into_iter().find(|other| *other != session).expect("three profiles");
                let carried = Provenance { profile: other, ..block.clone() };
                let input =
                    PublicationInput { provenance: carried.clone(), ..publication(&model, &view, content(3), out) };
                assert!(matches!(model.publish(CommandId::Caller(1), &view, input), Status::Completed(_)));
                drawn(&model, stage, &carried);
            }
        }
    }
}

// ----- CT-RPI-26, counter half

/// CT-RPI-26, the counter half: hidden and unchanged views add no rebuilds while program views are active (RPI.11,
/// RPI.62, RM.25). Each frame adopts a new state revision, runs what is due and takes the frame input. The visible
/// view that depends on the state recomputes, runs, prepares and installs once per frame. A hidden view (with a host
/// publication waiting), an evicted hidden view, visible views whose declared dependencies did not move, and Global
/// and Local leave every counter unchanged. Frames without a new revision rebuild nothing anywhere, and showing the
/// hidden view then rebuilds it once, at the newest revision. Charter E2 and the R-19 gate captures are the
/// owner-machine half (C9); nothing here is timed.
#[test]
fn ct_rpi_26_counter_half_hidden_and_unchanged_views_add_no_rebuilds() {
    const FRAMES: u64 = 8;
    let snapshot = |model: &ViewModel, views: &[ViewId]| -> Vec<Counters> {
        views.iter().map(|view| model.counters(view).expect("open")).collect()
    };
    let rebuilds =
        |counters: Counters| [counters.recomputations, counters.program_runs, counters.preparations, counters.installs];
    let grown = |after: Counters, before: Counters| -> Vec<u64> {
        rebuilds(after).iter().zip(rebuilds(before)).map(|(after, before)| after - before).collect()
    };
    let mut model = model_at(profile(&[]), rev(11, 1, 1, 11));
    let active = open(&mut model, "active", Depends::STATE);
    let hidden = open(&mut model, "hidden", Depends::STATE);
    let evicted = open(&mut model, "evicted", Depends::STATE);
    let unchanged = open(&mut model, "unchanged", Depends::of(&[Part::Workspace, Part::Protection]));
    let constant = open(&mut model, "constant", Depends::NONE);
    assert_eq!(run_all(&mut model), 5);
    let quiet = [hidden.clone(), evicted.clone(), unchanged, constant, ViewId::global(), ViewId::local()];
    let quiet_before = snapshot(&model, &quiet);
    let active_before = model.counters(&active).expect("open");

    assert!(model.hide(&hidden) && model.hide(&evicted));
    model.evict(&evicted).expect("evicted");
    let waiting = CommandId::Caller(1);
    let input = publication(&model, &hidden, content(1), OutputAllowance::new(BIG, 16));
    assert_eq!(model.publish(waiting, &hidden, input), Status::Accepted);
    let mut newest = *model.adopted();
    for frame in 1..=FRAMES {
        newest = rev(11 + frame, 1, 1, 11 + frame as u8);
        model.adopt(&bundle(newest)).expect("adopted");
        run_all(&mut model);
        let input = model.frame_input();
        assert_eq!(input.revision, newest);
        assert!(input.views.iter().all(|drawn| drawn.view != hidden && drawn.view != evicted));
        assert_eq!(snapshot(&model, &quiet), quiet_before, "a hidden or unchanged view rebuilt in frame {frame}");
    }
    assert_eq!(grown(model.counters(&active).expect("open"), active_before), vec![FRAMES; 4]);
    assert_eq!(model.receipt(waiting), Some(&Status::Cancelled(CancelReason::ReplacedRevision)));

    let all = model.views().to_vec();
    let settled = snapshot(&model, &all);
    for _ in 1..=FRAMES {
        assert_eq!(model.adopt(&bundle(newest)), Ok(newest));
        assert_eq!(run_all(&mut model), 0);
        assert_eq!(model.frame_input().revision, newest);
    }
    assert_eq!(snapshot(&model, &all), settled, "a frame without a new revision rebuilt");

    let before = model.counters(&hidden).expect("open");
    assert_eq!(model.show(&hidden), Ok(true));
    let mut bases = Vec::new();
    model.pump(|job| {
        bases.push(job.base.revision);
        RunOutcome::Published(content(1))
    });
    assert_eq!(bases, vec![newest]);
    assert_eq!(grown(model.counters(&hidden).expect("open"), before), vec![1; 4]);
}

// ----- CT-RPI-36

/// CT-RPI-36: `view.open` and `view.close` need a solver-invoked command; a recomputation holds no permission;
/// Global and Local cannot be closed; both EV3 codes (RPI.81, RPI.83).
#[test]
fn ct_rpi_36_view_rows_need_a_solver_command_and_global_and_local_stay_open() {
    let mut model = model_at(profile(&[]), rev(11, 1, 1, 11));
    let fixed = [ViewId::global(), ViewId::local()];
    let v = ViewId::new("v");
    let decl = declared(Depends::STATE, workload(1, 1));

    let error = model.open_view(OPEN, Origin::Recomputation, v.clone(), overlay("v"), decl).expect_err("no permission");
    assert_eq!((error.code, error.form), (code::NOT_GRANTED, Form::Refusal));
    assert_eq!(model.views(), &fixed);

    model.open_view(OPEN, Origin::Solver, v.clone(), overlay("v"), decl).expect("a solver command opens it");
    let (_, first) = model.runner(&v).expect("its first run");
    for name in ["v", "global", "local"] {
        let source = ViewSource::BuiltIn { name: "orbit".to_string() };
        let error = model.open_view(OPEN, Origin::Solver, ViewId::new(name), source, decl).expect_err("in use");
        assert_eq!(error.code, code::VIEW_ID_IN_USE);
        assert_eq!(error.context, Context::View(name.to_string()));
    }

    let error = model.close_view(CommandId::Caller(9), Origin::Recomputation, &v).expect_err("no permission");
    assert_eq!(error.code, code::NOT_GRANTED);
    assert!(model.views().contains(&v));
    for name in ["global", "local", "unknown"] {
        let error = model.close_view(CommandId::Caller(10), Origin::Solver, &ViewId::new(name)).expect_err("fixed");
        assert_eq!(error.code, code::VIEW_NOT_CLOSABLE);
    }
    assert!(!model.hide(&ViewId::global()));
    assert!(!model.hide(&ViewId::local()));
    assert!(model.is_visible(&ViewId::global()) && model.is_visible(&ViewId::local()));

    let close = CommandId::Caller(11);
    assert!(model.close_view(close, Origin::Solver, &v).expect("closed").is_some());
    assert_eq!(model.receipt(first), Some(&Status::Cancelled(CancelReason::Superseded { by: close })));
    assert_eq!(model.views(), &fixed);
    assert_eq!(model.generation(&v), None);

    let generation =
        model.open_view(OPEN, Origin::Solver, v.clone(), overlay("v"), decl).expect("the ID is free again");
    assert!(generation > model.generation(&ViewId::local()).expect("open"));
}

// ----- CT-RPI-37

/// CT-RPI-37: RM.40's view vectors. Hidden views add no vertex evaluations; hidden allocations stay charged until
/// safe release; opening or showing past a ceiling is refused with `display_plan_ceiling`, and nothing is queued
/// (RPI.82, RM.39).
#[test]
fn ct_rpi_37_view_budget_vectors() {
    let mut model = model_at(profile(&[("view_vertex_evals", 100), ("adapter_gpu_bytes", 100)]), rev(11, 1, 1, 11));
    let v = ViewId::new("v");
    let w = ViewId::new("w");
    let heavy =
        declared(Depends::STATE, Workload { vertex_evals: 60, cpu_bytes: 10, gpu_bytes: 40, ..Workload::default() });
    model.open_view(OPEN, Origin::Solver, v.clone(), overlay("v"), heavy).expect("fits");
    assert_eq!(model.usage().view_vertex_evals, 60);

    for _ in 0..100 {
        let error = model.open_view(OPEN, Origin::Solver, w.clone(), overlay("w"), heavy).expect_err("over");
        assert_eq!((error.code, error.form), (code::DISPLAY_PLAN_CEILING, Form::Refusal));
        assert_eq!(ceiling(&error), ("view_vertex_evals", 120, 100));
    }
    assert_eq!(model.views().len(), 3);
    assert_eq!(model.pending(&w), None);
    assert_eq!(model.generation(&w), None);

    assert!(model.hide(&v));
    assert_eq!(model.usage().view_vertex_evals, 0);
    assert_eq!(model.usage().adapter_gpu_bytes, 40);
    model.open_view(OPEN, Origin::Solver, w.clone(), overlay("w"), heavy).expect("fits beside the hidden view");
    assert_eq!(model.usage().view_vertex_evals, 60);
    assert_eq!(model.usage().adapter_gpu_bytes, 80);

    let error = model.show(&v).expect_err("over");
    assert_eq!(ceiling(&error), ("view_vertex_evals", 120, 100));
    assert!(!model.is_visible(&v));

    let token = model.close_view(CommandId::Caller(1), Origin::Solver, &v).expect("closed").expect("retiring");
    assert_eq!(model.usage().adapter_gpu_bytes, 80);
    assert_eq!(model.retiring(), 1);
    assert!(model.complete_retirement(token));
    assert_eq!(model.usage().adapter_gpu_bytes, 40);
}

// ----- CT-RM-02

/// CT-RM-02, instances: ceiling 4 passes 4. Five are refused at standalone admission with A left pending, and fail
/// the original scheduled invocation, which superseded A, with the same field and counts (RM.11, RM.12, RM.35).
/// Compression changes nothing.
#[test]
fn ct_rm_02_instance_ceiling_standalone_and_scheduled() {
    let mut model = model_at(profile(&[("instances", 4)]), rev(11, 1, 1, 11));
    let (v, held) = held_view(&mut model, "v");
    let out = OutputAllowance::new(BIG, 16);
    let a = CommandId::Caller(1);
    assert_eq!(model.publish(a, &v, publication(&model, &v, plans(1), out)), Status::Accepted);
    assert_eq!(model.pending(&v), Some(a));

    for transport in [Transport::Canonical, Transport::Compressed { frame_bytes: 9 }] {
        let input = PublicationInput { transport, ..publication(&model, &v, plans(5), out) };
        let Status::Refused(error) = model.publish(CommandId::Caller(2), &v, input) else { panic!("admitted") };
        assert_eq!((error.code, error.form), (code::DISPLAY_PLAN_CEILING, Form::Refusal));
        assert_eq!(ceiling(&error), ("instances", 5, 4));
        assert_eq!(model.pending(&v), Some(a));
        assert_eq!(model.receipt(a), Some(&Status::Accepted));
    }

    let b = CommandId::Caller(3);
    let generation = model.generation(&v);
    assert_eq!(model.invoke(b, &v, out), Status::Accepted);
    assert_eq!(model.receipt(a), Some(&Status::Cancelled(CancelReason::Superseded { by: b })));
    model.finish(held.job, RunOutcome::Published(plans(1)));
    let jobs = model.take_jobs();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].command, b);
    let Some(Status::Failed(error)) = model.finish(jobs[0].job, RunOutcome::Published(plans(5))) else {
        panic!("not failed")
    };
    assert_eq!((error.code, error.form), (code::DISPLAY_PLAN_CEILING, Form::Stop));
    assert_eq!(ceiling(&error), ("instances", 5, 4));
    let result = model.job_result(b).expect("settled");
    assert_eq!((result.command, result.job), (b, Some(jobs[0].job)));
    assert_eq!(result.outcome, JobOutcome::Error(error.clone()));
    assert_eq!(result.outcome.receipt(None, CancelReason::ByUser), Status::Failed(error));
    assert_eq!(model.receipt(a), Some(&Status::Cancelled(CancelReason::Superseded { by: b })));
    assert_eq!(model.pending(&v), None);
    assert!(model.take_jobs().is_empty());
    assert!(model.installed(&v).is_none());
    assert_eq!(model.generation(&v), generation);

    let input = PublicationInput {
        transport: Transport::Compressed { frame_bytes: 9 },
        ..publication(&model, &v, plans(4), out)
    };
    assert!(matches!(model.publish(CommandId::Caller(4), &v, input), Status::Completed(_)));
    model.invoke(CommandId::Caller(5), &v, out);
    let jobs = model.take_jobs();
    assert!(matches!(model.finish(jobs[0].job, RunOutcome::Published(plans(4))), Some(Status::Completed(_))));
}

/// CT-RM-02, output: an allowance of 100 bytes admits a 100-byte publication and cuts off a 101-byte one without
/// installing it, charging it or touching the terminal reserve, standalone or scheduled; compression changes
/// neither outcome (RM.13, LI.28).
#[test]
fn ct_rm_02_output_allowance_admits_100_and_cuts_off_101() {
    let allowance = OutputAllowance::new(100, 16);
    let mut outcomes = Vec::new();
    for transport in [Transport::Canonical, Transport::Compressed { frame_bytes: 40 }] {
        let mut model = model_at(profile(&[]), rev(11, 1, 1, 11));
        let v = open(&mut model, "v", Depends::STATE);
        run_all(&mut model);
        let kept = model.installed(&v).cloned();
        let base = Base::new(*model.adopted(), Depends::STATE);
        let mut statuses = Vec::new();

        let input = PublicationInput { transport, ..publication(&model, &v, sized(&base, 101), allowance) };
        let status = model.publish(CommandId::Caller(1), &v, input);
        assert_eq!(status, Status::Cancelled(CancelReason::Budget));
        let result = model.job_result(CommandId::Caller(1)).expect("settled");
        assert_eq!(result.outcome, JobOutcome::CutOff(code::OUTPUT));
        assert_eq!(result.output, allowance);
        assert_eq!(model.installed(&v).cloned(), kept);
        statuses.push(status);

        let input = PublicationInput { transport, ..publication(&model, &v, sized(&base, 100), allowance) };
        let status = model.publish(CommandId::Caller(2), &v, input);
        assert!(matches!(status, Status::Completed(_)));
        assert_eq!(
            model.job_result(CommandId::Caller(2)).map(|result| result.output),
            Some(OutputAllowance::new(0, 16))
        );
        statuses.push(status);

        for (index, len) in [101, 100].into_iter().enumerate() {
            let command = CommandId::Caller(10 + index as u64);
            let installed = model.installed(&v).cloned();
            model.invoke(command, &v, allowance);
            model.pump(|job| RunOutcome::Published(sized(&job.base, len)));
            let status = model.receipt(command).cloned().expect("settled");
            let output = model.job_result(command).expect("settled").output;
            if len == 101 {
                assert_eq!(status, Status::Cancelled(CancelReason::Budget));
                assert_eq!(output, allowance);
                assert_eq!(model.installed(&v).cloned(), installed);
            } else {
                assert!(matches!(status, Status::Completed(_)));
                assert_eq!(output, OutputAllowance::new(0, 16));
            }
            statuses.push(status);
        }
        outcomes.push(statuses);
    }
    assert_eq!(outcomes[0], outcomes[1], "compression changed an outcome");
}

// ----- CT-RM-09, accounting rows

/// CT-RM-09, RM.40 row 1: with `adapter_gpu_bytes` ceiling 100, a hidden view retains 70 and opening another view
/// that needs 70 is refused at 140 until retirement completes; hiding freed nothing (RPI.82, RM.39).
#[test]
fn ct_rm_09_row_1_hidden_bytes_stay_charged() {
    let mut model = model_at(profile(&[("adapter_gpu_bytes", 100)]), rev(11, 1, 1, 11));
    let need = declared(Depends::STATE, Workload { gpu_bytes: 70, ..Workload::default() });
    let v = ViewId::new("v");
    let w = ViewId::new("w");
    model.open_view(OPEN, Origin::Solver, v.clone(), overlay("v"), need).expect("fits");
    assert!(model.hide(&v));
    assert_eq!(model.usage().adapter_gpu_bytes, 70);

    let error = model.open_view(OPEN, Origin::Solver, w.clone(), overlay("w"), need).expect_err("over");
    assert_eq!((error.code, error.form), (code::DISPLAY_PLAN_CEILING, Form::Refusal));
    assert_eq!(ceiling(&error), ("adapter_gpu_bytes", 140, 100));

    let token = model.close_view(CommandId::Caller(1), Origin::Solver, &v).expect("closed").expect("retiring");
    let error = model.open_view(OPEN, Origin::Solver, w.clone(), overlay("w"), need).expect_err("still retiring");
    assert_eq!(ceiling(&error), ("adapter_gpu_bytes", 140, 100));
    assert!(model.complete_retirement(token));
    model.open_view(OPEN, Origin::Solver, w.clone(), overlay("w"), need).expect("fits after retirement");
    assert_eq!(model.usage().adapter_gpu_bytes, 70);
}

/// CT-RM-09, RM.40 row 2: a hidden, evicted view whose retirement is delayed cannot come back early. Fresh resources
/// must fit beside the retiring bytes or the show or reopen is refused; there is no early free or reuse (RM.19,
/// RM.39).
#[test]
fn ct_rm_09_row_2_retiring_bytes_block_early_reuse() {
    let need = declared(Depends::STATE, Workload { gpu_bytes: 70, ..Workload::default() });
    let v = ViewId::new("v");

    let mut model = model_at(profile(&[("adapter_gpu_bytes", 100)]), rev(11, 1, 1, 11));
    model.open_view(OPEN, Origin::Solver, v.clone(), overlay("v"), need).expect("fits");
    run_all(&mut model);
    assert!(model.hide(&v));
    let token = model.evict(&v).expect("evicted");
    assert!(model.installed(&v).is_none());
    assert_eq!(model.usage().adapter_gpu_bytes, 70);

    let error = model.show(&v).expect_err("over");
    assert_eq!(ceiling(&error), ("adapter_gpu_bytes", 140, 100));
    assert!(!model.is_visible(&v));
    assert_eq!(model.close_view(CommandId::Caller(1), Origin::Solver, &v), Ok(None));
    let error = model.open_view(OPEN, Origin::Solver, v.clone(), overlay("v"), need).expect_err("over");
    assert_eq!(ceiling(&error), ("adapter_gpu_bytes", 140, 100));
    assert!(model.complete_retirement(token));
    model.open_view(OPEN, Origin::Solver, v.clone(), overlay("v"), need).expect("fits after retirement");
    assert_eq!(model.usage().adapter_gpu_bytes, 70);

    let mut model = model_at(profile(&[("adapter_gpu_bytes", 140)]), rev(11, 1, 1, 11));
    model.open_view(OPEN, Origin::Solver, v.clone(), overlay("v"), need).expect("fits");
    run_all(&mut model);
    assert!(model.hide(&v));
    model.evict(&v).expect("evicted");
    assert_eq!(model.show(&v), Ok(true));
    assert_eq!(model.usage().adapter_gpu_bytes, 140);
    assert_eq!(run_all(&mut model), 1);
    assert!(matches!(model.status(&v), Some(ViewStatus::Current { .. })));
}

/// CT-RM-09, RM.40 row 7: view V has pending publication A and view W has pending publication C; standalone B names
/// V. B supersedes A and C runs on, because each publication binds its own target (RPI.10, LI.25).
#[test]
fn ct_rm_09_row_7_a_publication_to_one_view_leaves_another_alone() {
    let mut model = model_at(profile(&[]), rev(11, 1, 1, 11));
    let (v, v_job) = held_view(&mut model, "v");
    let (w, w_job) = held_view(&mut model, "w");
    let out = OutputAllowance::new(BIG, 16);
    let (a, b, c) = (CommandId::Caller(1), CommandId::Caller(2), CommandId::Caller(3));
    assert_eq!(model.publish(a, &v, publication(&model, &v, content(1), out)), Status::Accepted);
    assert_eq!(model.publish(c, &w, publication(&model, &w, content(3), out)), Status::Accepted);
    assert_eq!(model.publish(b, &v, publication(&model, &v, content(2), out)), Status::Accepted);

    assert_eq!(model.receipt(a), Some(&Status::Cancelled(CancelReason::Superseded { by: b })));
    assert_eq!(model.receipt(c), Some(&Status::Accepted));
    assert_eq!(model.pending(&w), Some(c));

    model.finish(w_job.job, RunOutcome::Published(content(9)));
    assert!(matches!(model.receipt(c), Some(Status::Completed(_))));
    model.finish(v_job.job, RunOutcome::Published(content(9)));
    assert!(matches!(model.receipt(b), Some(Status::Completed(_))));
    assert_eq!(model.installed(&v).map(|installed| installed.command), Some(b));
    assert_eq!(model.installed(&w).map(|installed| installed.command), Some(c));
}

/// CT-RM-09, RM.40 row 9: `frame_work` ceiling 600, vertex term 100, `fragment_ops` 1, `overdraw` 4. Growing the
/// surface from 100 to 200 pixels recharges 500 to 900; the view falls back with visible status and no quarantine,
/// and its stage returns when a recharge fits (RM.38, RM.39).
#[test]
fn ct_rm_09_row_9_resize_recharges_frame_work() {
    let mut model = model_at(profile(&[("frame_work", 600), ("overdraw", 4)]), rev(11, 1, 1, 11));
    let v = ViewId::new("v");
    let work = Workload { vertex_evals: 10, stage_ops: 10, fragment_ops: 1, pixels: 100, ..Workload::default() };
    model.open_view(OPEN, Origin::Solver, v.clone(), overlay("v"), declared(Depends::STATE, work)).expect("fits");
    run_all(&mut model);
    assert_eq!(model.frame_work_of(&v), Some(500));
    assert_eq!(model.usage().frame_work, 500);

    model.resize(&v, 200);
    assert_eq!(model.frame_work_of(&v), Some(900));
    assert!(model.is_fallback(&v));
    assert_eq!(model.usage().frame_work, 0);
    let stage = |model: &ViewModel| match model.frame_input().views.into_iter().find(|view| view.view == v) {
        Some(FrameView { draw: Draw::Output { stage, .. }, .. }) => Some(stage),
        _ => None,
    };
    assert_eq!(stage(&model), Some(StageUse::Fallback));
    let input = publication(&model, &v, content(1), OutputAllowance::new(BIG, 16));
    assert!(matches!(model.publish(CommandId::Caller(1), &v, input), Status::Completed(_)));

    model.resize(&v, 150);
    assert!(model.is_fallback(&v), "700 does not fit 600");
    model.resize(&v, 125);
    assert!(!model.is_fallback(&v));
    assert_eq!(model.usage().frame_work, 600);
    assert_eq!(stage(&model), Some(StageUse::User));
}

// ----- CT-ONE-30

/// CT-ONE-30: a further view keeps its one-revision base, its `ViewId` and its source linkage (source and source
/// maps) after adoption until it is republished, while Global and Local both show the new revision (ONE.65, RPI.81).
#[test]
fn ct_one_30_further_view_keeps_its_base_identity_and_source() {
    let r12 = rev(12, 1, 1, 12);
    let r13 = rev(13, 1, 1, 13);
    let mut model = model_at(profile(&[]), r12);
    let v = ViewId::new("v");
    let source = overlay("v");
    let generation = model
        .open_view(OPEN, Origin::Solver, v.clone(), source.clone(), declared(Depends::STATE, workload(1, 1)))
        .expect("fits");
    assert_eq!(model.shown_revision(&ViewId::global()), Some(r12));
    assert_eq!(model.shown_revision(&ViewId::local()), Some(r12));
    run_all(&mut model);
    let old = model.installed(&v).expect("installed").clone();
    assert_eq!(model.shown_revision(&v), Some(r12));

    model.adopt(&bundle(r13)).expect("adopted");
    assert_eq!(model.shown_revision(&ViewId::global()), Some(r13));
    assert_eq!(model.shown_revision(&ViewId::local()), Some(r13));
    assert_eq!(model.shown_revision(&v), Some(r12));
    assert!(
        matches!(model.status(&v), Some(ViewStatus::Stale { base, adopted, .. }) if base == old.base && adopted == r13)
    );
    assert_eq!(model.source(&v), Some(&source));
    assert_eq!(model.generation(&v), Some(generation));
    assert_eq!(model.installed(&v).map(|installed| &installed.content.maps), Some(&old.content.maps));
    assert!(model.views().contains(&v));
    let frame = model.frame_input();
    assert_eq!(frame.revision, r13);
    let fixed: Vec<&ViewId> = frame.views.iter().filter(|view| view.bundle).map(|view| &view.view).collect();
    assert_eq!(fixed, vec![&ViewId::global(), &ViewId::local()]);

    run_all(&mut model);
    assert_eq!(model.shown_revision(&v), Some(r13));
    assert_eq!(model.source(&v), Some(&source));
    assert_eq!(model.generation(&v), Some(generation));
    assert_eq!(model.shown_revision(&ViewId::global()), model.shown_revision(&ViewId::local()));
}

// ----- CT-EV3-08, admission semantics

/// A view-lane receipt status. The match is exhaustive, so a commit status or completion kind would not compile here
/// (RPI.14, RPI.81).
fn view_lane_status(status: &Status) -> &'static str {
    match status {
        Status::Accepted | Status::Running(_) => "open",
        Status::Completed(Completion { kind: CompletionKind::ViewPublished, .. }) => "view_published",
        Status::Refused(_) | Status::Failed(_) | Status::Cancelled(_) => "ended",
    }
}

/// What a runner can return. The match is exhaustive, so a commit outcome would not compile here (RPI.14).
fn runner_outcome(outcome: &RunOutcome) -> &'static str {
    match outcome {
        RunOutcome::Published(_) => "published",
        RunOutcome::CutOff(_) => "cut off",
        RunOutcome::Error(_) => "error",
    }
}

/// Runs every startable job, recording its command, and publishes content tagged with the job's state revision.
fn run_recording(model: &mut ViewModel, commands: &mut Vec<CommandId>) -> usize {
    model.pump(|job| {
        commands.push(job.command);
        let outcome = RunOutcome::Published(content(job.base.revision.state as u8));
        assert_eq!(runner_outcome(&outcome), "published");
        outcome
    })
}

/// CT-EV3-08, the admission semantics (EV3.11, EV3.13): in every profile, further views are admitted under RPI.81 to
/// RPI.83 with identity (their `ViewId` and generation), one base revision, source linkage and no program commit
/// path; Global and Local remain linked and cannot be closed; hidden allocations remain charged; and opening or
/// showing a view past RM.39's ceilings is refused with `display_plan_ceiling` and queues nothing. The ceilings are
/// declared fixture values: RM.39's measured ceilings are the renderer-freeze half (C9) and are not run.
#[test]
fn ct_ev3_08_admission_semantics_in_every_profile() {
    let (r11, r12) = (rev(11, 1, 1, 11), rev(12, 1, 1, 12));
    let (v, w, x) = (ViewId::new("v"), ViewId::new("w"), ViewId::new("x"));
    let fixed = [ViewId::global(), ViewId::local()];
    let heavy =
        declared(Depends::STATE, Workload { vertex_evals: 60, cpu_bytes: 30, gpu_bytes: 40, ..Workload::default() });
    let light = declared(Depends::STATE, Workload { gpu_bytes: 30, ..Workload::default() });
    let mut traces = Vec::new();
    for session in Profile::ALL {
        let limits = profile(&[("view_vertex_evals", 100), ("adapter_cpu_bytes", 100), ("adapter_gpu_bytes", 100)]);
        let mut model = model_in(session, limits, r11);
        let mut commands = Vec::new();
        let mut trace = Vec::new();

        // RPI.83: only a solver command opens a view; a recomputation holds no permission.
        let refused = model.open_view(OPEN, Origin::Recomputation, v.clone(), built_in("orbit"), heavy);
        assert_eq!(refused.map_err(|error| error.code), Err(code::NOT_GRANTED));
        assert_eq!(model.views(), &fixed);

        // RPI.81: identity is a `ViewId` in the view namespace and a generation; an ID in use is refused.
        let generation = model.open_view(OPEN, Origin::Solver, v.clone(), built_in("orbit"), heavy).expect("admitted");
        for name in ["v", "global", "local"] {
            let error =
                model.open_view(OPEN, Origin::Solver, ViewId::new(name), overlay(name), light).expect_err("in use");
            assert_eq!(error.code, code::VIEW_ID_IN_USE);
        }
        run_recording(&mut model, &mut commands);

        // RPI.81: one base revision and source linkage (the source, and plans that resolve through installed maps).
        let installed = model.installed(&v).expect("installed").clone();
        assert_eq!(installed.base.revision, r11);
        assert_eq!(model.source(&v), Some(&built_in("orbit")));
        let maps = &installed.content.maps;
        assert!(installed.content.plans.iter().all(|plan| maps.iter().any(|map| map.map == plan.source_map)));
        trace.push(format!("{generation} {:?}", model.status(&v)));

        // ONE.65: Global and Local stay linked and cannot be closed; the further view keeps its one base until it is
        // republished, and is drawn only as its stale presentation meanwhile.
        model.adopt(&bundle(r12)).expect("adopted");
        for view in &fixed {
            assert_eq!(model.shown_revision(view), Some(r12));
            let error = model.close_view(CommandId::Caller(1), Origin::Solver, view).expect_err("fixed");
            assert_eq!(error.code, code::VIEW_NOT_CLOSABLE);
            assert!(!model.hide(view));
        }
        assert_eq!(model.shown_revision(&v), Some(r11));
        assert_eq!(draw_of(&model, &v), Some(Draw::Stale { base: installed.base }));
        run_recording(&mut model, &mut commands);
        assert_eq!(model.shown_revision(&v), Some(r12));
        trace.push(format!("{:?}", model.status(&v)));

        // RM.39, RPI.82: opening past a ceiling is refused, naming field, request and ceiling, and queues nothing.
        let error = model.open_view(OPEN, Origin::Solver, w.clone(), overlay("w"), heavy).expect_err("over");
        assert_eq!((error.code, ceiling(&error)), (code::DISPLAY_PLAN_CEILING, ("view_vertex_evals", 120, 100)));
        assert_eq!((model.generation(&w), model.pending(&w)), (None, None));

        // RM.39: a hidden view adds no vertex evaluations and its allocations stay charged.
        assert!(model.hide(&v));
        let usage = model.usage();
        assert_eq!((usage.view_vertex_evals, usage.adapter_cpu_bytes, usage.adapter_gpu_bytes), (0, 30, 40));
        model.open_view(OPEN, Origin::Solver, w.clone(), overlay("w"), heavy).expect("fits beside the hidden view");
        let error = model.show(&v).expect_err("over");
        assert_eq!(ceiling(&error), ("view_vertex_evals", 120, 100));
        assert!(!model.is_visible(&v));
        let error = model.open_view(OPEN, Origin::Solver, x.clone(), overlay("x"), light).expect_err("over");
        assert_eq!(ceiling(&error), ("adapter_gpu_bytes", 110, 100));
        trace.push(format!("{:?}", model.usage()));

        // RM.19: eviction and close move the bytes to safe retirement, still charged until it completes.
        let token = model.evict(&v).expect("evicted");
        let refused = model.close_view(CommandId::Caller(2), Origin::Recomputation, &v);
        assert_eq!(refused.map_err(|error| error.code), Err(code::NOT_GRANTED));
        assert_eq!(model.close_view(CommandId::Caller(3), Origin::Solver, &v), Ok(None));
        let error = model.open_view(OPEN, Origin::Solver, x.clone(), overlay("x"), light).expect_err("still retiring");
        assert_eq!(ceiling(&error), ("adapter_gpu_bytes", 110, 100));
        assert!(model.complete_retirement(token));
        model.open_view(OPEN, Origin::Solver, x.clone(), overlay("x"), light).expect("fits after retirement");

        // RM.25: a reopened ID is a new identity.
        let reopened = model.open_view(OPEN, Origin::Solver, v.clone(), built_in("orbit"), light).expect("free again");
        assert!(reopened > generation);
        assert_eq!(run_recording(&mut model, &mut commands), 3);
        trace.push(format!("{reopened} {:?} {:?}", model.usage(), model.views()));

        // RPI.14: no program commit path. Every request ended on the view lane, and view work never moved the engine
        // revision or the adopted bundle.
        assert_eq!(commands.len(), 5);
        for command in &commands {
            assert_eq!(model.receipt(*command).map(view_lane_status), Some("view_published"));
        }
        assert_eq!(*model.adopted(), r12);
        assert_eq!(model.bundle(), &validate_engine_bundle(&bundle(r12)).expect("complete"));
        traces.push(trace);
    }
    assert!(traces.windows(2).all(|pair| pair[0] == pair[1]), "a profile changed admission");
}

// ----- supporting clauses

/// ONE.10, KN7.3, KN7.8: after an undo returns to an equal state digest with larger counters, the view's equal read
/// result comes back without a program run and is charged as a miss, output included.
#[test]
fn one_10_an_equal_read_result_after_undo_is_reused_and_charged_as_a_miss() {
    let mut model = model_at(profile(&[]), rev(11, 1, 1, 0xA));
    let v = open(&mut model, "v", Depends::STATE);
    model.pump(|_| RunOutcome::Published(content(0xA)));
    model.adopt(&bundle(rev(12, 1, 1, 0xB))).expect("adopted");
    model.pump(|_| RunOutcome::Published(content(0xB)));
    let before = model.counters(&v).expect("open");

    let undo = rev(13, 1, 1, 0xA);
    model.adopt(&bundle(undo)).expect("adopted");
    assert!(model.take_jobs().is_empty(), "no program run");
    let after = model.counters(&v).expect("open");
    assert_eq!(after.program_runs, before.program_runs);
    assert_eq!(after.cache_hits, before.cache_hits + 1);
    assert_eq!(after.charged_runs, before.charged_runs + 1);
    let installed = model.installed(&v).expect("installed").clone();
    assert_eq!(installed.base.revision, undo);
    assert_eq!(installed.content, content(0xA));
    let len = content(0xA).canonical_bytes(&installed.base).len() as u64;
    assert_eq!(model.job_result(installed.command).map(|result| result.output.remaining), Some(BIG - len));
}

/// RM.11: a profile with a missing field, a missing or wrong unit, an unknown or repeated field or no version is
/// invalid; nothing defaults to unlimited.
#[test]
fn rm_11_a_missing_field_or_unit_makes_the_profile_invalid() {
    let full: Vec<(&str, u64, &str)> = Field::ALL.iter().map(|field| (field.name(), 1, field.unit())).collect();
    assert!(LimitProfile::from_entries("p", &full).is_ok());
    assert_eq!(LimitProfile::from_entries("", &full), Err(ProfileError::MissingVersion));
    let missing: Vec<_> = full.iter().filter(|(name, ..)| *name != "frame_work").copied().collect();
    assert_eq!(LimitProfile::from_entries("p", &missing), Err(ProfileError::Missing(Field::FrameWork)));
    let mut no_unit = full.clone();
    no_unit[2].2 = "";
    assert_eq!(LimitProfile::from_entries("p", &no_unit), Err(ProfileError::MissingUnit(Field::Instances)));
    let mut wrong = full.clone();
    wrong[0].2 = "bytes";
    let expected = ProfileError::WrongUnit { field: Field::Nodes, unit: "bytes".to_string() };
    assert_eq!(LimitProfile::from_entries("p", &wrong), Err(expected));
    let mut repeated = full.clone();
    repeated.push(full[0]);
    assert_eq!(LimitProfile::from_entries("p", &repeated), Err(ProfileError::Duplicate(Field::Nodes)));
    let mut unknown = full.clone();
    unknown.push(("unlimited", 0, "records"));
    assert_eq!(LimitProfile::from_entries("p", &unknown), Err(ProfileError::Unknown("unlimited".to_string())));
}

/// RM.12: the admission ceiling is the componentwise minimum of the host/profile and backend ceilings; equality
/// passes and one more is refused with field, requested count and ceiling.
#[test]
fn rm_12_the_admission_ceiling_is_the_componentwise_minimum() {
    let host = profile(&[("instances", 4), ("nodes", 50)]);
    let backend = profile(&[("instances", 9), ("nodes", 20)]);
    let admission = host.admission_ceiling(&backend);
    assert_eq!(admission.get(Field::Instances), 4);
    assert_eq!(admission.get(Field::Nodes), 20);
    assert!(admission.check(Field::Nodes, 20, Form::Refusal).is_ok());
    let error = admission.check(Field::Nodes, 21, Form::Refusal).expect_err("over");
    assert_eq!(ceiling(&error), ("nodes", 21, 20));
}

/// RM.20, RM.35: malformed output keeps its original code: a standalone publication is refused with
/// `display_plan_invalid`, and a scheduled run fails with it in the stop form, without reclassification.
#[test]
fn rm_35_malformed_output_keeps_its_code() {
    let mut model = model_at(profile(&[]), rev(11, 1, 1, 11));
    let v = open(&mut model, "v", Depends::STATE);
    let mut duplicate = plans(1);
    duplicate.plans.push(duplicate.plans[0].clone());
    let mut dangling = plans(1);
    dangling.plans[0].source_map = 99;

    let cases = [(duplicate, PlanFault::DuplicatePlan(1)), (dangling, PlanFault::DanglingMap { plan: 1, map: 99 })];
    for (round, (malformed, fault)) in cases.into_iter().enumerate() {
        let jobs = model.take_jobs();
        assert_eq!(jobs.len(), 1);
        model.finish(jobs[0].job, RunOutcome::Published(malformed.clone()));
        let Some(Status::Failed(error)) = model.receipt(jobs[0].command).cloned() else { panic!("not failed") };
        assert_eq!((error.code, error.form), (code::DISPLAY_PLAN_INVALID, Form::Stop));
        assert_eq!(error.context, Context::Plan(fault));

        let input = publication(&model, &v, malformed, OutputAllowance::new(BIG, 16));
        let command = CommandId::Caller(10 + round as u64);
        let Status::Refused(error) = model.publish(command, &v, input) else { panic!("admitted") };
        assert_eq!((error.code, error.form), (code::DISPLAY_PLAN_INVALID, Form::Refusal));
        assert_eq!(error.context, Context::Plan(fault));
        model.invoke(CommandId::Caller(20 + round as u64), &v, OutputAllowance::new(BIG, 16));
    }
    assert!(model.installed(&v).is_none());
}

/// RPI.9, ONE.60: incomplete, duplicated, foreign or mixed arrays are `refresh_required(display_bundle_mismatch)`
/// and install nothing; an older bundle or a counter reuse with other content is refused; the identical bundle
/// again changes nothing; Jumble on needs all six arrays.
#[test]
fn rpi_09_incomplete_or_mismatched_bundles_install_nothing() {
    let r11 = rev(11, 1, 1, 11);
    let r12 = rev(12, 1, 1, 12);
    let mut model = model_at(profile(&[]), r11);
    let mut missing = bundle(r12);
    missing.parts.pop();
    let mut duplicate = bundle(r12);
    duplicate.parts.push(duplicate.parts[0]);
    let mut foreign = bundle(r12);
    foreign.parts.push(part(PartKind::Lattice, r12));
    let mut mixed = bundle(r12);
    mixed.parts[1] = part(PartKind::Labels, r11);
    let cases = [
        (missing, BundleFault::Missing(PartKind::Styles)),
        (duplicate, BundleFault::Duplicate(PartKind::Metadata)),
        (foreign, BundleFault::Unexpected(PartKind::Lattice)),
        (mixed, BundleFault::Mismatched { kind: PartKind::Labels, part: r11, bundle: r12 }),
    ];
    for (candidate, fault) in cases {
        let error = validate_engine_bundle(&candidate).expect_err("incomplete");
        assert_eq!((error.code, error.context.clone()), (code::DISPLAY_BUNDLE_MISMATCH, Context::Bundle(fault)));
        assert_eq!(model.adopt(&candidate), Err(error));
        assert_eq!(*model.adopted(), r11);
    }

    assert_eq!(model.adopt(&bundle(r11)), Ok(r11));
    let mut reused = bundle(r11);
    reused.parts[0].digest = [0xEE; 32];
    let error = model.adopt(&reused).expect_err("counter reuse");
    assert_eq!(error.context, Context::Bundle(BundleFault::OutOfOrder { adopted: r11, candidate: r11 }));
    model.adopt(&bundle(r12)).expect("adopted");
    let error = model.adopt(&bundle(r11)).expect_err("older");
    assert_eq!(error.context, Context::Bundle(BundleFault::OutOfOrder { adopted: r12, candidate: r11 }));

    let r13 = rev(13, 1, 1, 13);
    let jumble = Mode::JumbleOn.required().iter().map(|kind| part(*kind, r13)).collect();
    assert!(model.adopt(&BundleCandidate { revision: r13, mode: Mode::JumbleOn, parts: jumble }).is_ok());
    let short =
        BundleCandidate { revision: rev(14, 1, 1, 14), mode: Mode::JumbleOn, parts: bundle(rev(14, 1, 1, 14)).parts };
    let error = model.adopt(&short).expect_err("incomplete");
    assert_eq!(error.context, Context::Bundle(BundleFault::Missing(PartKind::PoseIndices)));
    assert_eq!(*model.adopted(), r13);
}

// ----- L11's leads

/// L11, lead (a): five revisions in a burst while the view is blocked leave at most one pending request, and the view
/// moves to the newest. A taken run that does not finish holds the view's runner slot, with a host publication
/// waiting behind it. After each adoption the stalled run still holds the slot, exactly one admitted request is
/// unsettled, and Global, Local and the frame show the adopted revision. The waiting publication ends
/// `cancelled(replaced_revision)`, and the whole burst admits one recomputation, whose base binds when it starts. The
/// late completion installs nothing, and the one pending run starts and installs at the newest revision (KN7.2,
/// LI.23, LI.25, ONE.64, RPI.12, RM.22). Not confirmed: the crate already behaved this way.
#[test]
fn l11_a_a_burst_of_five_revisions_while_blocked_leaves_one_pending_request_at_the_newest() {
    let mut model = model_at(profile(&[]), rev(11, 1, 1, 11));
    let (v, held) = held_view(&mut model, "v");
    let waiting = CommandId::Caller(1);
    let input = publication(&model, &v, content(1), OutputAllowance::new(BIG, 16));
    assert_eq!(model.publish(waiting, &v, input), Status::Accepted);
    let before = model.counters(&v).expect("open");

    let mut admitted = vec![waiting];
    for state in 12..=16 {
        let next = rev(state, 1, 1, state as u8);
        assert_eq!(model.adopt(&bundle(next)), Ok(next));
        assert_eq!(model.runner(&v), Some((held.job, held.command)), "the stalled run still holds the slot");
        let pending = model.pending(&v).expect("one pending request");
        if !admitted.contains(&pending) {
            admitted.push(pending);
        }
        let unsettled: Vec<CommandId> = admitted
            .iter()
            .copied()
            .filter(|command| !model.receipt(*command).is_some_and(Status::is_terminal))
            .collect();
        assert_eq!(unsettled, vec![pending], "one unsettled request after revision {state}");
        for view in [ViewId::global(), ViewId::local()] {
            assert_eq!(model.shown_revision(&view), Some(next));
        }
        assert_eq!(model.frame_input().revision, next);
    }
    let newest = rev(16, 1, 1, 16);
    assert_eq!(*model.adopted(), newest);
    assert_eq!(admitted.len(), 2, "the publication and one recomputation for the whole burst");
    let scheduled = admitted[1];
    assert!(matches!(scheduled, CommandId::Scheduled(_)));
    assert_eq!(model.receipt(waiting), Some(&Status::Cancelled(CancelReason::ReplacedRevision)));
    assert_eq!(model.status(&v), Some(ViewStatus::Busy { waiting: scheduled, held_by: held.command }));
    assert_eq!(model.counters(&v).map(|counters| counters.recomputations), Some(before.recomputations + 1));
    assert!(model.take_jobs().is_empty());

    let late = model.finish(held.job, RunOutcome::Published(content(11)));
    assert_eq!(late, Some(Status::Cancelled(CancelReason::Superseded { by: waiting })));
    assert!(model.installed(&v).is_none());
    let jobs = model.take_jobs();
    assert_eq!(jobs.len(), 1);
    assert_eq!((jobs[0].command, jobs[0].base.revision), (scheduled, newest));
    assert!(matches!(model.finish(jobs[0].job, RunOutcome::Published(content(16))), Some(Status::Completed(_))));
    assert_eq!(model.shown_revision(&v), Some(newest));
    let after = Counters {
        recomputations: before.recomputations + 1,
        program_runs: before.program_runs + 1,
        charged_runs: before.charged_runs + 1,
        cache_hits: before.cache_hits,
        preparations: before.preparations + 1,
        installs: before.installs + 1,
    };
    assert_eq!(model.counters(&v), Some(after));
}

/// L11, lead (b): output from an older engine epoch never attaches at the same revision numbers. In epoch 1, view v
/// (state) shows revision 3, and a producer prepared a publication for v at epoch 1, revision 4, which the host never
/// adopted because the engine stopped. Epoch 2 recovers the session at a revision with the same numbers and other
/// content: the prepared publication is refused as `stale.display_plan` and installs nothing. A run taken in epoch 2
/// is still out when the engine restarts again, and epoch 3 recovers at the same numbers and content, so the bundles
/// differ only in the epoch. Adoption fences the run, its late completion installs nothing, v's old output is drawn
/// only as its stale presentation, and v runs again in epoch 3. View c declares no dependencies, and its epoch-1 base
/// matches nothing in epoch 2 either: its equal read result comes back bound to epoch 2, as a new install charged as a
/// miss. A bundle of an older epoch is then out of order whatever its counters, and a newer epoch may start its
/// counters again (KN0.2: counters never decrease within an epoch; ONE.10, ONE.63, ONE.70, RPI.9, KN7.8).
#[test]
fn l11_b_output_from_an_older_engine_epoch_never_attaches_at_the_same_revision_numbers() {
    let first = rev_in(1, 3, 1, 1, 3);
    let mut model = model_at(profile(&[]), first);
    let v = open(&mut model, "v", Depends::STATE);
    let c = open(&mut model, "c", Depends::NONE);
    assert_eq!(run_all(&mut model), 2);
    let unadopted = Base::new(rev_in(1, 4, 1, 1, 4), Depends::STATE);
    let prepared =
        PublicationInput { base: unadopted, ..publication(&model, &v, content(0xE1), OutputAllowance::new(BIG, 16)) };

    let second = rev_in(2, 4, 1, 1, 0x24);
    assert_eq!(model.adopt(&bundle(second)), Ok(second));
    assert_eq!(model.installed(&c).map(|installed| installed.base), Some(Base::new(second, Depends::NONE)));
    let reuse = model
        .counters(&c)
        .map(|counters| (counters.program_runs, counters.charged_runs, counters.cache_hits, counters.installs));
    assert_eq!(reuse, Some((1, 2, 1, 2)));
    let refused = Error::refusal(code::STALE_DISPLAY_PLAN, Context::Stale { base: unadopted, adopted: second });
    assert_eq!(model.publish(CommandId::Caller(1), &v, prepared), Status::Refused(refused));
    let old = Base::new(first, Depends::STATE);
    assert_eq!(model.installed(&v).map(|installed| installed.base), Some(old));
    assert_eq!(draw_of(&model, &v), Some(Draw::Stale { base: old }));

    let out = model.take_jobs().into_iter().find(|job| job.view == v).expect("v runs in epoch 2");
    assert_eq!(out.base.revision, second);
    let third = rev_in(3, 4, 1, 1, 0x24);
    assert_eq!(model.adopt(&bundle(third)), Ok(third));
    assert_eq!(*model.adopted(), third);
    assert_ne!(model.frame_input().bundle, *validate_engine_bundle(&bundle(second)).expect("complete").digest());
    assert!(model.is_fenced(out.job));
    assert_eq!(model.receipt(out.command), Some(&Status::Cancelled(CancelReason::ReplacedRevision)));
    let late = model.finish(out.job, RunOutcome::Published(content(4)));
    assert_eq!(late, Some(Status::Cancelled(CancelReason::ReplacedRevision)));
    assert_eq!(model.installed(&v).map(|installed| installed.base), Some(old));
    assert_eq!(draw_of(&model, &v), Some(Draw::Stale { base: old }));
    let jobs = model.take_jobs();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].base.revision, third);
    assert!(matches!(model.finish(jobs[0].job, RunOutcome::Published(content(4))), Some(Status::Completed(_))));
    assert_eq!(model.shown_revision(&v), Some(third));

    let older = rev_in(2, 9, 1, 1, 9);
    let error = model.adopt(&bundle(older)).expect_err("an older epoch");
    assert_eq!(error.context, Context::Bundle(BundleFault::OutOfOrder { adopted: third, candidate: older }));
    assert_eq!(*model.adopted(), third);
    let restarted = rev_in(4, 1, 1, 1, 0x41);
    assert_eq!(model.adopt(&bundle(restarted)), Ok(restarted));
    let jobs = model.take_jobs();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].base.revision, restarted);
}

/// L11, lead (c): a recomputation that a revision starts cannot open or close a view. After two adoptions, v's first
/// recomputation holds the runner slot and the next one waits; each is a `CommandId::Scheduled` that the view model
/// minted. Naming either as the command of `view.close` or `view.open` is refused with `LG3:capability.not_granted`,
/// whatever origin the caller states: v stays open with its lane as it was, nothing opens, nothing is cancelled and
/// nothing is charged. A solver's own command then closes v, and the waiting recomputation ends superseded by it
/// (RPI.83, LI.25).
#[test]
fn l11_c_a_recomputation_a_revision_starts_cannot_open_or_close_a_view() {
    let mut model = model_at(profile(&[]), rev(11, 1, 1, 11));
    let v = open(&mut model, "v", Depends::STATE);
    run_all(&mut model);
    model.adopt(&bundle(rev(12, 1, 1, 12))).expect("adopted");
    let running = model.take_jobs().into_iter().find(|job| job.view == v).expect("a recomputation runs");
    model.adopt(&bundle(rev(13, 1, 1, 13))).expect("adopted");
    let waiting = model.pending(&v).expect("the next recomputation waits");
    assert!(matches!(running.command, CommandId::Scheduled(_)) && matches!(waiting, CommandId::Scheduled(_)));

    let snapshot = |model: &ViewModel| {
        let receipts = (model.receipt(running.command).cloned(), model.receipt(waiting).cloned());
        let lane = (model.runner(&v), model.pending(&v), model.generation(&v));
        (model.views().to_vec(), receipts, lane, model.usage(), model.retiring())
    };
    let before = snapshot(&model);
    let not_granted = |name: &str| Error::refusal(code::NOT_GRANTED, Context::View(name.to_string()));
    let w = ViewId::new("w");
    for command in [running.command, waiting] {
        for origin in [Origin::Solver, Origin::Recomputation] {
            let closed = model.close_view(command, origin, &v);
            assert_eq!(closed, Err(not_granted("v")), "{command:?} closed v as {origin:?}");
            let decl = declared(Depends::STATE, workload(1, 1));
            let opened = model.open_view(command, origin, w.clone(), overlay("w"), decl);
            assert_eq!(opened, Err(not_granted("w")), "{command:?} opened w as {origin:?}");
            assert_eq!(snapshot(&model), before);
        }
    }

    let close = CommandId::Caller(1);
    assert!(model.close_view(close, Origin::Solver, &v).expect("a solver command closes v").is_some());
    assert_eq!(model.receipt(waiting), Some(&Status::Cancelled(CancelReason::Superseded { by: close })));
    assert_eq!(model.views(), &[ViewId::global(), ViewId::local()]);
}
