//! The display host over the bridge and the fake core, end to end, on the shipped model. The scenarios live in
//! `support`, which `renderer/bridge`'s tests run again through the C function table. Every result is a synthetic
//! fixture outcome, not a GPU run.

mod support;

use std::collections::BTreeMap;

use c600_digest::sha256_hex;
use c600_display::array::Array;
use c600_display::codes;
use c600_display::error::Error;
use c600_display::host::{Backend, Poll, Preparation, PublicationStamp, Ticket};
use c600_display::plan::*;
use c600_display::produce::{self, full_set};
use c600_display::receipt::CancelReason;
use c600_kernel::SLOTS;
use c600_render::{CoreAbi, FakeCore, RenderBackend};

use support::*;

#[test]
fn the_host_runs_over_the_fake_core() {
    scenario(FakeCore::new(SIZE));
}

/// A bridge over a fresh fake core, and a stamp for a preparation of `publication`.
fn prepared(publication: &Publication) -> Result<Ticket, Error> {
    let mut backend = RenderBackend::new(FakeCore::new(SIZE), limits());
    let stamp = PublicationStamp {
        command: CommandId(1),
        view: V1,
        view_generation: 1,
        publication_generation: 1,
        device_epoch: 1,
        base: publication.base.clone(),
        dependencies: BTreeMap::new(),
        input_digest: sha256_hex(b"fixture input"),
    };
    backend.prepare(Preparation { stamp: &stamp, payload: publication, limits: &limits() })
}

fn refusal(publication: &Publication) -> (&'static str, String) {
    let error = prepared(publication).expect_err("refused");
    (error.code, error.context.get("feature").cloned().unwrap_or_default())
}

#[test]
fn preparation_refuses_what_the_core_cannot_draw_or_stamp() {
    let state = twisted();
    let at = slots(state.labels());
    let base = produce::revision(&geometry().identity, 12, state);
    let retained = || Primitive::Retained(produce::retained(state, &full_set(), &full_set(), Vec::new()));
    let sticker = [(1, ObjectId::Sticker(at.home))];
    assert!(prepared(&publication(&base, vec![retained()], &sticker)).is_ok());
    assert!(prepared(&publication(&base, Vec::new(), &[])).is_ok(), "an empty publication draws nothing");
    assert_eq!(refusal(&publication(&base, Vec::new(), &sticker)).0, codes::PLAN_INVALID, "an instance, nothing drawn");

    let invalid = (codes::PLAN_INVALID, String::new());
    let unsupported = |row: &str| (codes::UNSUPPORTED, row.to_owned());
    assert_eq!(refusal(&publication(&base, vec![retained(), retained()], &sticker)), unsupported("F01"));
    let posed = PosedInput {
        pose_index: Array::i32s(vec![1], &[0]),
        poses: Array::f32s(vec![1, 16], &CAMERA),
        pose_lattice: Array::i32s(vec![1], &[0]),
    };
    assert_eq!(refusal(&publication(&base, vec![Primitive::Posed(posed)], &[])), unsupported("J02"));
    let layout = LayoutInput {
        instances: vec![InstanceId(1)],
        positions: Array::f32s(vec![1, 2], &[0.0, 0.0]),
        edges: None,
        triangles: None,
        kind: LayoutKind::Sheet,
    };
    assert_eq!(refusal(&publication(&base, vec![Primitive::Layout(layout)], &sticker)), unsupported("F06"));
    assert_eq!(refusal(&publication(&base, vec![retained()], &[(1, ObjectId::Piece(0))])), unsupported("F18"));
    assert_eq!(refusal(&publication(&base, vec![retained()], &[(0, ObjectId::Sticker(at.home))])), invalid);
    assert_eq!(refusal(&publication(&base, vec![retained()], &[(1 << 32, ObjectId::Sticker(at.home))])), invalid);
    let shared = [(1, ObjectId::Sticker(at.home)), (2, ObjectId::Slot(at.moved as u32))];
    assert_eq!(refusal(&publication(&base, vec![retained()], &shared)), invalid, "two instances on one slot");
    assert_eq!(refusal(&publication(&base, vec![retained()], &[(1, ObjectId::Slot(SLOTS as u32))])), invalid);
}

#[test]
fn tickets_cancel_and_faults_are_kept() {
    let mut backend = RenderBackend::new(FakeCore::new(SIZE), limits());
    let input = vec![0u8; SLOTS];
    let instances = vec![0u32; SLOTS];
    let request = c600_render::PreparationInput {
        command: 1,
        view: 1,
        view_generation: 1,
        publication_generation: 1,
        device_epoch: 1,
        base_revision: 12,
        visibility: &input,
        interaction: &input,
        instances: &instances,
        input_digest: [0; 32],
    };
    let ticket = backend.core.prepare(&request).expect("a ticket");
    assert_eq!(backend.poll(Ticket(ticket)), Poll::Pending);
    backend.cancel(Ticket(ticket), &CancelReason::Superseded { by: CommandId(7) });
    assert_eq!(backend.poll(Ticket(ticket)), Poll::Cancelled(CancelReason::Superseded { by: CommandId(7) }));
    assert_eq!(backend.fault, None);
    backend.retire(c600_display::host::ResourceSet(99));
    let fault = backend.fault.clone().expect("a refused retirement is kept");
    assert_eq!((fault.code, fault.context["status"].as_str()), (codes::RENDERER_UNAVAILABLE, "E_WRONG_STATE"));
    assert!(fault.context["detail"].contains("retired set"), "{fault:?}");
    let Poll::Failed(error) = backend.poll(Ticket(999)) else { panic!("an unknown ticket fails") };
    assert_eq!(error.context["status"], "E_WRONG_STATE");

    // A ticket the core failed names its own status, not the text of the core's last failing call.
    let request = c600_render::PreparationInput { command: 2, publication_generation: 2, ..request };
    let doomed = backend.core.prepare(&request).expect("a ticket");
    backend.core.remove_device();
    let Poll::Failed(error) = backend.poll(Ticket(doomed)) else { panic!("a lost device fails the ticket") };
    assert_eq!(error.context["status"], c600_render::abi::Status::DEVICE_REMOVED.name());
    assert!(!error.context.contains_key("detail"), "{error:?}");
}

#[test]
fn frames_need_a_camera_and_a_set_and_reports_come_in_batches() {
    frames_and_batches(FakeCore::new(SIZE));
}
