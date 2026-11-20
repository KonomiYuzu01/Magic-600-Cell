//! RM.14's adapter operations that the host drives besides preparation, over the mock backend: engine adoption
//! through the backend (RM.16), frame reports that mark a view's fallback (RM.41), and picks that go request, poll,
//! then RPI.25's resolution. Every result is a source or synthetic fixture outcome, not a backend run.

mod common;

use c600_display::codes;
use c600_display::error::{Error, Form};
use c600_display::host::{Host, MockBackend, PickPoll, PickTicket, Picked};
use c600_display::pick::{FrameStamp, Partial, PickMode, PickReply, Requested};
use c600_display::plan::*;
use c600_display::produce::{full_set, retained};
use c600_display::receipt::Status;
use c600_display::stage::{Fallback, FrameReport};
use c600_display::validate::{BoundArray, EngineBundle};

use common::*;

const V1: ViewId = ViewId(1);

/// An engine bundle of a Full state at `revision`: the three arrays, each bound to `bound`.
fn bundle(state: &c600_kernel::Snapshot, revision: &RevisionRef, bound: [u64; 3]) -> EngineBundle {
    let set = full_set();
    let input = retained(state, &set, &set, Vec::new());
    let arrays = [("labels", input.labels), ("visibility", input.visibility), ("interaction", input.interaction)];
    EngineBundle {
        revision: revision.clone(),
        arrays: arrays
            .into_iter()
            .zip(bound)
            .map(|((name, array), revision)| BoundArray { name: name.to_owned(), revision, array })
            .collect(),
    }
}

/// RM.41's report of the frame `stamp`: the frame stopped a user stage and its view falls back to `fallback`.
fn report(stamp: &FrameStamp, fallback: Fallback) -> FrameReport {
    FrameReport {
        stamp: stamp.clone(),
        view: stamp.view,
        publication_generation: stamp.publication_generation,
        error: Error::new(codes::STAGE_WORK_EXCEEDED, "the frame's raster bound is over its allowance")
            .in_form(Form::Status),
        fallback,
    }
}

/// After a device loss: the host notices it and recovers, and view 1's retained publication is prepared again on the
/// new device (RM.23).
fn recover_and_rebind(host: &mut Host<MockBackend>) {
    host.pump();
    host.recover().expect("recovers");
    host.pump();
    let ticket = host.rebinding(V1).expect("prepared again on the new device");
    host.backend.finish(ticket);
    host.pump();
}

/// A pick at one point of `frame`, at the occurrence's own level.
fn pick(host: &mut Host<MockBackend>, frame: &FrameStamp, mode: PickMode) -> PickTicket {
    host.request_pick(frame, (1, 1), mode, Requested::Occurrence).expect("a pick").expect("a ticket")
}

#[test]
fn adoption_goes_through_the_backend_and_a_refused_bundle_changes_nothing() {
    let base = revision(12);
    let mut host = host(&base);
    install(&mut host, 1, sheet(1, &base, 2));
    let next = derive(&[1]);
    let thirteen = revision_with(13, next.state.digest());

    let mixed = host.adopt(&bundle(&next.state, &thirteen, [13, 12, 13])).expect_err("a mixed bundle");
    assert_eq!(mixed.code, codes::BUNDLE_MISMATCH);
    assert!(host.backend.adoptions.is_empty(), "the host refuses a mixed bundle before the backend sees it");

    host.backend.fail_adopt = Some(Error::new(codes::BUNDLE_MISMATCH, "the engine inputs moved"));
    let refused = host.adopt(&bundle(&next.state, &thirteen, [13; 3])).expect_err("refresh_required");
    assert_eq!(refused.code, codes::BUNDLE_MISMATCH);
    assert_eq!(host.adopted(), &base, "a bundle the backend refuses is not adopted");
    assert!(!host.stale(V1), "and moves no dependency");
    assert!(host.backend.adoptions.is_empty());

    assert_eq!(host.adopt(&bundle(&next.state, &thirteen, [13; 3])), Ok(thirteen.clone()));
    assert_eq!(host.backend.adoptions, [thirteen.clone()]);
    assert_eq!(host.adopted(), &thirteen);
    assert!(host.stale(V1), "the state dependency moves with the adopted revision");
    assert!(matches!(status(&host, 1), Status::Completed(_)), "no receipt changes");

    host.backend.lose_device();
    host.pump();
    let later = derive(&[2]);
    let fourteen = revision_with(14, later.state.digest());
    let adopted = host.adopt(&bundle(&later.state, &fourteen, [14; 3]));
    assert_eq!(adopted, Ok(fourteen.clone()), "adoption works while the device is lost");
    assert_eq!(host.backend.adoptions, [thirteen, fourteen.clone()]);
    assert_eq!(host.adopted(), &fourteen);
}

#[test]
fn a_frame_report_marks_the_fallback_of_the_publication_it_names_and_changes_no_receipt() {
    let base = revision(12);
    let mut host = host(&base);
    install(&mut host, 1, sheet(1, &base, 2));
    let first = host.frame(V1).expect("a frame");
    assert_eq!(host.read_frame_reports(), Ok(Vec::new()));
    assert_eq!(host.installed(V1).expect("installed").fallback, None);

    host.backend.frame_reports.push(report(&first, Fallback::BuiltInChain));
    assert_eq!(host.read_frame_reports(), Ok(vec![report(&first, Fallback::BuiltInChain)]));
    assert_eq!(host.installed(V1).expect("installed").fallback, Some(Fallback::BuiltInChain));
    assert!(matches!(status(&host, 1), Status::Completed(_)), "no receipt changes");
    assert_eq!(host.read_frame_reports(), Ok(Vec::new()), "each report is read once");

    install(&mut host, 2, sheet(1, &base, 3));
    assert_eq!(host.installed(V1).expect("installed").fallback, None, "a new publication starts with none");
    host.backend.frame_reports.push(report(&first, Fallback::ErrorStatus));
    assert_eq!(host.read_frame_reports().expect("reports").len(), 1, "a replaced publication's report is returned");
    assert_eq!(host.installed(V1).expect("installed").fallback, None, "and marks nothing");

    let before = host.frame(V1).expect("a frame");
    host.backend.lose_device();
    assert_eq!(host.read_frame_reports().expect_err("lost").code, codes::RENDERER_UNAVAILABLE);
    recover_and_rebind(&mut host);
    host.backend.frame_reports.push(report(&before, Fallback::ErrorStatus));
    assert_eq!(host.read_frame_reports(), Ok(Vec::new()), "a report of an earlier device epoch is dropped");
    assert_eq!(host.installed(V1).expect("installed").fallback, None);
    let after = host.frame(V1).expect("a frame on the new device");
    host.backend.frame_reports.push(report(&after, Fallback::ErrorStatus));
    assert_eq!(host.read_frame_reports(), Ok(vec![report(&after, Fallback::ErrorStatus)]));
    assert_eq!(host.installed(V1).expect("installed").fallback, Some(Fallback::ErrorStatus));
    assert!(matches!(status(&host, 2), Status::Completed(_)));
}

#[test]
fn a_pick_goes_request_poll_then_resolution() {
    let base = revision(12);
    let mut host = host(&base);
    install(&mut host, 1, sheet(1, &base, 3));
    let frame = host.frame(V1).expect("a frame");

    let ticket = host.request_pick(&frame, (10, 20), PickMode::Explicit, Requested::Occurrence);
    let ticket = ticket.expect("a pick").expect("a ticket");
    let asked = &host.backend.picks[&ticket];
    assert_eq!((&asked.frame, asked.point, asked.requested), (&frame, (10, 20), Requested::Occurrence));
    assert_eq!(host.poll_pick(ticket, &Unasked), Ok(Picked::Pending));
    host.backend.answer_pick(ticket, Some(InstanceId(2)));
    let reply = PickReply { binding: host.pick_binding(V1).expect("current"), instance: InstanceId(2) };
    assert_eq!(host.backend.picks[&ticket].answer, PickPoll::Stamped(Some(reply.clone())), "stamped (RPI.24)");
    let Ok(Picked::Done(Some(Partial::Total(resolution)))) = host.poll_pick(ticket, &Unasked) else {
        panic!("a total resolution")
    };
    assert_eq!((resolution.instance, &resolution.target), (InstanceId(2), &ObjectId::Sticker(2)));
    let direct = host.resolve_pick(&reply, PickMode::Explicit, Requested::Occurrence, &Unasked);
    assert_eq!(direct, Ok(Some(Partial::Total(resolution))), "the same as resolving the reply");
    let over = host.poll_pick(ticket, &Unasked).expect_err("the host forgets a pick once it is over");
    assert_eq!(over.code, codes::PLAN_INVALID);

    let empty = pick(&mut host, &frame, PickMode::Explicit);
    host.backend.answer_pick(empty, None);
    assert_eq!(host.poll_pick(empty, &Unasked), Ok(Picked::Done(None)), "no interactive instance at the point");

    let unknown = host.poll_pick(PickTicket(999), &Unasked).expect_err("a ticket the host never gave");
    assert_eq!(unknown.code, codes::PLAN_INVALID);
}

#[test]
fn a_stale_pick_is_explained_when_explicit_and_dropped_when_passive() {
    let base = revision(12);
    let mut host = host(&base);
    install(&mut host, 1, sheet(1, &base, 3));

    let frame = host.frame(V1).expect("a frame");
    let explicit = pick(&mut host, &frame, PickMode::Explicit);
    let passive = pick(&mut host, &frame, PickMode::Passive);
    host.set_camera(V1);
    host.backend.answer_pick(explicit, Some(InstanceId(1)));
    host.backend.answer_pick(passive, Some(InstanceId(1)));
    let error = host.poll_pick(explicit, &Unasked).expect_err("the camera moved after the frame");
    assert_eq!((error.code, error.context["field"].as_str()), (codes::STALE_PICK, "camera"));
    assert_eq!(host.poll_pick(passive, &Unasked), Ok(Picked::Done(None)), "a stale passive pick is quiet");

    let frame = host.frame(V1).expect("a frame");
    let explicit = pick(&mut host, &frame, PickMode::Explicit);
    let passive = pick(&mut host, &frame, PickMode::Passive);
    let stale = || {
        Error::new(codes::STALE_PICK, "the pick is stale: surface_generation changed")
            .with("field", "surface_generation")
    };
    host.backend.refuse_pick(explicit, stale());
    host.backend.refuse_pick(passive, stale());
    assert_eq!(host.poll_pick(explicit, &Unasked), Err(stale()), "the backend found it stale");
    assert_eq!(host.poll_pick(passive, &Unasked), Ok(Picked::Done(None)));
    host.backend.fail_pick = Some(stale());
    let dropped = host.request_pick(&frame, (1, 1), PickMode::Passive, Requested::Occurrence);
    assert_eq!(dropped, Ok(None), "refused as stale before it started: a passive pick gets no ticket");
    host.backend.fail_pick = Some(stale());
    let refused = host.request_pick(&frame, (1, 1), PickMode::Explicit, Requested::Occurrence);
    assert_eq!(refused, Err(stale()), "and an explicit one is explained");
    host.backend.fail_pick = Some(Error::invalid("no frame with that stamp was submitted"));
    let refused = host.request_pick(&frame, (1, 1), PickMode::Passive, Requested::Occurrence);
    assert_eq!(refused.expect_err("any other refusal is returned").code, codes::PLAN_INVALID);
    let other = pick(&mut host, &frame, PickMode::Passive);
    host.backend.refuse_pick(other, Error::invalid("the point is outside the surface"));
    let error = host.poll_pick(other, &Unasked).expect_err("any other refusal is returned");
    assert_eq!(error.code, codes::PLAN_INVALID);

    let explicit = pick(&mut host, &frame, PickMode::Explicit);
    let passive = pick(&mut host, &frame, PickMode::Passive);
    host.backend.lose_device();
    host.pump();
    let error = host.request_pick(&frame, (1, 1), PickMode::Explicit, Requested::Occurrence).expect_err("lost");
    assert_eq!(error.code, codes::RENDERER_UNAVAILABLE);
    let error = host.poll_pick(explicit, &Unasked).expect_err("asked before the device was lost");
    assert_eq!((error.code, error.context["field"].as_str()), (codes::STALE_PICK, "device_epoch"));
    recover_and_rebind(&mut host);
    assert_eq!(host.poll_pick(passive, &Unasked), Ok(Picked::Done(None)), "asked on the old device");

    let asked = host.backend.picks.len();
    let error = host.request_pick(&frame, (1, 1), PickMode::Explicit, Requested::Occurrence).expect_err("old");
    assert_eq!((error.code, error.context["field"].as_str()), (codes::STALE_PICK, "device_epoch"));
    let dropped = host.request_pick(&frame, (1, 1), PickMode::Passive, Requested::Occurrence);
    assert_eq!(dropped, Ok(None), "a frame of the old device is not one to pick from");
    assert_eq!(host.backend.picks.len(), asked, "and neither pick reached the backend");
    let fresh = host.frame(V1).expect("a frame on the new device");
    let ticket = pick(&mut host, &fresh, PickMode::Explicit);
    host.backend.answer_pick(ticket, Some(InstanceId(3)));
    let resolved = host.poll_pick(ticket, &Unasked);
    assert!(
        matches!(&resolved, Ok(Picked::Done(Some(Partial::Total(r)))) if r.instance == InstanceId(3)),
        "{resolved:?}"
    );
}
