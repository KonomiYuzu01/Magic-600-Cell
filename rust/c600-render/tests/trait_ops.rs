//! RM.14's adapter operations through c600-display's `Backend` trait, over the bridge and the fake core: adoption in
//! the shell's order (the bridge's own call, then the host's) and through the host alone, frame reports, and picks
//! that the host requests, polls and resolves. Every result is a synthetic fixture outcome, not a GPU run.

mod support;

use c600_display::codes;
use c600_display::host::{Backend, Host, PickTicket, Picked};
use c600_display::pick::{FrameStamp, Partial, PickMode, Requested};
use c600_display::plan::*;
use c600_display::produce::{self, full_set};
use c600_render::{FakeCore, RenderBackend};

use support::*;

/// The newest frames the bridge keeps for picks and reports (its `FRAME_RECORDS`).
const KEPT: usize = 2048;

/// The host over the bridge, with `base` adopted in the core first, as the shell starts it, and view 1 open.
fn started(base: &RevisionRef) -> Host<RenderBackend<FakeCore>> {
    let mut backend = RenderBackend::new(FakeCore::new(SIZE), limits());
    assert_eq!(backend.adopt_engine(&bundle(twisted(), base)), Ok(base.state));
    let mut ctx = produce::fixture_context(geometry(), limits());
    ctx.support = backend.describe_backend().expect("a live core describes itself");
    let mut host = Host::new(backend, ctx, base.clone());
    host.open_view(V1).expect("view 1 opens");
    host.backend.set_camera_matrix(V1, 0, CAMERA);
    host
}

fn adopted_in_core(host: &Host<RenderBackend<FakeCore>>) -> Vec<u64> {
    host.backend.core.adoptions.iter().map(|(revision, _)| *revision).collect()
}

/// The ticket of a pick at (10, 20) of `frame`, at the occurrence's own level.
fn pick(host: &mut Host<RenderBackend<FakeCore>>, frame: &FrameStamp, mode: PickMode) -> PickTicket {
    host.request_pick(frame, (10, 20), mode, Requested::Occurrence).expect("a pick").expect("a ticket")
}

#[test]
fn the_host_adopts_through_the_bridge_in_the_shells_order_and_alone() {
    let state = twisted();
    let revision = |n| produce::revision(&geometry().identity, n, state);
    let mut host = started(&revision(12));

    // The shell's order: the bridge's own adoption, then the host's, which makes no second copy.
    assert_eq!(host.backend.adopt_engine(&bundle(state, &revision(13))), Ok(13));
    assert_eq!(host.adopt(&bundle(state, &revision(13))), Ok(revision(13)));
    assert_eq!(adopted_in_core(&host), [12, 13]);

    // The host alone: the trait's adoption reaches the core, and frames carry the revision the core holds.
    assert_eq!(host.adopt(&bundle(state, &revision(14))), Ok(revision(14)));
    assert_eq!(adopted_in_core(&host), [12, 13, 14]);
    let stamp = host.frame(V1).expect("a frame at the adopted revision");
    assert_eq!(stamp.engine_revision, 14);
    assert_eq!(host.backend.core.submitted.last().expect("submitted").engine_revision, 14);

    // A revision the core refuses is not adopted.
    let error = host.adopt(&bundle(state, &revision(13))).expect_err("the core's revision does not go back");
    assert_eq!(error.code, codes::RENDERER_UNAVAILABLE);
    assert_eq!(host.adopted(), &revision(14));
    assert_eq!(adopted_in_core(&host), [12, 13, 14]);

    // The same state counter with another digest is not the image the core holds: it reaches the core, which
    // refuses it.
    let solved = kernel().solved();
    let other = produce::revision(&geometry().identity, 14, &solved);
    let error = host.adopt(&bundle(&solved, &other)).expect_err("another image at the same state counter");
    assert_eq!(error.code, codes::RENDERER_UNAVAILABLE);
    assert_eq!(host.adopted(), &revision(14));
    assert_eq!(adopted_in_core(&host), [12, 13, 14]);

    // Adoption also works while the device is removed (RM.16).
    host.backend.core.remove_device();
    assert_eq!(host.adopt(&bundle(state, &revision(15))), Ok(revision(15)));
    assert_eq!(adopted_in_core(&host), [12, 13, 14, 15]);
}

#[test]
fn a_pick_goes_through_the_host_to_the_id_pass_and_back() {
    let state = twisted();
    let at = slots(state.labels());
    let base = produce::revision(&geometry().identity, 12, state);
    let mut host = started(&base);
    let retained = produce::retained(state, &full_set(), &full_set(), Vec::new());
    let objects = [(1, ObjectId::Sticker(at.home)), (2, ObjectId::Slot(at.fixed as u32))];
    host.publish(CommandId(1), publication(&base, vec![Primitive::Retained(retained)], &objects));
    host.pump();
    host.backend.core.complete();
    host.pump();
    let frame = host.frame(V1).expect("a frame");
    host.backend.core.place(10, 20, at.moved);

    // Request, poll, then resolution: the ID pass's instance is the sticker drawn at the point.
    let ticket = pick(&mut host, &frame, PickMode::Explicit);
    assert_eq!(host.poll_pick(ticket, &Unasked), Ok(Picked::Pending));
    host.backend.core.complete();
    match host.poll_pick(ticket, &Unasked) {
        Ok(Picked::Done(Some(Partial::Total(resolution)))) => {
            assert_eq!((resolution.instance, &resolution.target), (InstanceId(1), &ObjectId::Sticker(at.home)))
        }
        other => panic!("{other:?}"),
    }

    // A point outside the frame's viewport and a frame the bridge never submitted are the caller's errors, refused
    // before the core is asked (the core would answer a point outside as a failure, `EV3:busy.renderer_unavailable`),
    // so even a passive pick returns them.
    let outside = host.request_pick(&frame, (SIZE.0, 0), PickMode::Explicit, Requested::Occurrence);
    assert_eq!(outside.expect_err("outside").code, codes::PLAN_INVALID);
    let below = host.request_pick(&frame, (0, SIZE.1), PickMode::Passive, Requested::Occurrence);
    assert_eq!(below.expect_err("below").code, codes::PLAN_INVALID);
    let unknown = FrameStamp { sequence: frame.sequence + 1, ..frame.clone() };
    let refused = host.request_pick(&unknown, (10, 20), PickMode::Explicit, Requested::Occurrence);
    assert_eq!(refused.expect_err("never submitted").code, codes::PLAN_INVALID);

    // A reopened view: the core finds a pick on the old frame stale, which an explicit pick names and a passive one
    // drops (RPI.25).
    host.close_view(V1);
    host.open_view(V1).expect("view 1 reopens");
    let explicit = pick(&mut host, &frame, PickMode::Explicit);
    let passive = pick(&mut host, &frame, PickMode::Passive);
    let reopened = host.frame(V1).expect("a frame of the new generation");
    host.backend.core.complete();
    let error = host.poll_pick(explicit, &Unasked).expect_err("stale");
    assert_eq!((error.code, error.context["field"].as_str()), (codes::STALE_PICK, "surface_generation"));
    assert_eq!(host.poll_pick(passive, &Unasked), Ok(Picked::Done(None)));

    // RM.41's reports through the trait are empty, since no frame stopped a user stage; the core's own rows remain.
    assert_eq!(host.read_frame_reports(), Ok(Vec::new()));
    assert_eq!(host.backend.read_frame_reports().expect("the core's rows").len(), 2);

    // A frame with 64 newer frames reported is one the core no longer retains: it is stale at its sequence before the
    // core is asked (the core would answer only as a failure, `EV3:busy.renderer_unavailable`), so an explicit pick
    // says so and a passive one gets no ticket. The oldest of the 64 is still retained, and picked.
    let newer: Vec<FrameStamp> = (0..64).map(|_| host.frame(V1).expect("a frame")).collect();
    host.backend.core.complete();
    assert_eq!(host.backend.read_frame_reports().expect("the core's rows").len(), 64);
    let error = host.request_pick(&reopened, (10, 20), PickMode::Explicit, Requested::Occurrence).expect_err("gone");
    assert_eq!((error.code, error.context["field"].as_str()), (codes::STALE_PICK, "sequence"));
    assert_eq!(host.request_pick(&reopened, (10, 20), PickMode::Passive, Requested::Occurrence), Ok(None));
    pick(&mut host, &newer[0], PickMode::Passive);

    // A frame older than every frame the bridge keeps is stale at its sequence before the core is asked: an explicit
    // pick says so, and a passive one gets no ticket.
    for _ in 0..KEPT {
        host.frame(V1).expect("a frame");
    }
    let error = host.request_pick(&reopened, (10, 20), PickMode::Explicit, Requested::Occurrence).expect_err("old");
    assert_eq!((error.code, error.context["field"].as_str()), (codes::STALE_PICK, "sequence"));
    assert_eq!(host.request_pick(&reopened, (10, 20), PickMode::Passive, Requested::Occurrence), Ok(None));
}
