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
use c600_display::jumble;
use c600_display::plan::*;
use c600_display::produce::{self, full_set};
use c600_display::receipt::CancelReason;
use c600_display::validate::EngineBundle;
use c600_kernel::{Puzzle, SLOTS};
use c600_render::abi::PIECES;
use c600_render::{CoreAbi, FakeCore, RenderBackend};

use support::*;

#[test]
fn the_host_runs_over_the_fake_core() {
    scenario(FakeCore::new(SIZE));
}

/// Route B (accepted for now by the owner on 18 Nov 2026, pending three owner-machine checks): presentations and recovery.
#[test]
fn route_b_confirms_presentations_and_recovers() {
    route_b(FakeCore::on_surface(SIZE));
}

/// A bridge over a fresh fake core, and a stamp for a preparation of `publication`.
fn prepared(publication: &Publication) -> Result<Ticket, Error> {
    prepare_on(&mut RenderBackend::new(FakeCore::new(SIZE), limits()), publication)
}

fn prepare_on(backend: &mut RenderBackend<FakeCore>, publication: &Publication) -> Result<Ticket, Error> {
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
    let posed = || {
        Primitive::Posed(PosedInput {
            pose_index: Array::i32s(vec![1], &[0]),
            poses: Array::f32s(vec![1, 16], &CAMERA),
            pose_lattice: Array::i32s(vec![1], &[0]),
        })
    };
    assert!(prepared(&publication(&base, vec![posed()], &[])).is_ok(), "a posed plan draws the adopted poses");
    assert_eq!(refusal(&publication(&base, vec![posed()], &sticker)), unsupported("J13"), "no posed instance");
    assert_eq!(refusal(&publication(&base, vec![retained(), posed()], &[])), unsupported("F01"));
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

/// A Jumble-on bundle at `revision` with the given pose table (render/1 section 7).
fn posed_bundle(revision: u64, index: &[i32], poses: &[f32], lattice: &[i32]) -> EngineBundle {
    let mut base = produce::revision(&geometry().identity, revision, twisted());
    base.identity.puzzle = Puzzle::Jumbling;
    let posed = PosedInput {
        pose_index: Array::i32s(vec![index.len() as u64], index),
        poses: Array::f32s(vec![(poses.len() / 16) as u64, 4, 4], poses),
        pose_lattice: Array::i32s(vec![lattice.len() as u64], lattice),
    };
    jumble::bundle(&base, &posed)
}

#[test]
fn a_posed_bundle_is_adopted_over_the_home_labels_and_its_plan_draws_every_slot() {
    // Even pieces at the identity, odd pieces turned a quarter in the (x, y) plane, off the lattice.
    let quarter = [0.0, -1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];
    let index: Vec<i32> = (0..PIECES).map(|piece| (piece % 2) as i32).collect();
    let poses = [CAMERA, quarter].concat();
    let bundle = posed_bundle(12, &index, &poses, &[0, -1]);
    let mut backend = RenderBackend::new(FakeCore::new(SIZE), limits());
    assert_eq!(backend.adopt_engine(&bundle), Ok(12));
    let digest = |name: &str| bundle.arrays.iter().find(|a| a.name == name).expect(name).array.desc.digest.clone();
    assert_eq!(backend.core.adoptions, [(12, 0)], "the home labels: slot 0 shows sticker 0");
    assert_eq!(backend.core.posed, [(12, 2, [digest("pose_index"), digest("poses"), digest("pose_lattice")])]);

    let base = bundle.revision.clone();
    let posed = Primitive::Posed(PosedInput {
        pose_index: Array::i32s(vec![PIECES as u64], &index),
        poses: Array::f32s(vec![2, 4, 4], &poses),
        pose_lattice: Array::i32s(vec![2], &[0, -1]),
    });
    prepare_on(&mut backend, &publication(&base, vec![posed], &[])).expect("a posed plan prepares");
    let (view, instances, visibility) = backend.core.prepared[0].clone();
    // ADR-D02 B17: every slot is pickable, stamped in the ID pass as its home slot + 1.
    assert_eq!((view, instances.len()), (1, SLOTS), "every slot is pickable");
    assert!(instances.iter().all(|(&slot, &id)| id as usize == slot + 1), "each as its home slot + 1");
    assert!(visibility.iter().all(|&v| v == 1), "every slot is drawn where its piece's pose puts it");

    // Shapes the bridge refuses before the core sees them, then values the core refuses.
    let shape = |bundle: &EngineBundle| backend_refusal(bundle).0;
    assert_eq!(shape(&posed_bundle(13, &index[1..], &poses, &[0, -1])), codes::PLAN_INVALID, "a piece short");
    assert_eq!(shape(&posed_bundle(13, &index, &poses, &[0])), codes::PLAN_INVALID, "two poses, one entry");
    assert_eq!(shape(&posed_bundle(13, &index, &[], &[])), codes::PLAN_INVALID, "no pose");
    let core = |index: &[i32], poses: &[f32], lattice: &[i32]| {
        let (code, status) = backend_refusal(&posed_bundle(13, index, poses, lattice));
        assert_eq!(code, codes::RENDERER_UNAVAILABLE);
        status
    };
    assert_eq!(core(&index, &[quarter, CAMERA].concat(), &[0, -1]), "E_INVALID_ARGUMENT", "pose 0 is the identity");
    let mut beyond = index.clone();
    beyond[7] = 2;
    assert_eq!(core(&beyond, &poses, &[0, -1]), "E_INVALID_ARGUMENT", "an index past the table");
    assert_eq!(core(&index, &poses, &[0, 7200]), "E_INVALID_ARGUMENT", "a lattice entry past K+");
    assert_eq!(core(&index, &poses, &[1, -1]), "E_INVALID_ARGUMENT", "entry 0 is the identity's");
    let mut infinite = poses.clone();
    infinite[20] = f32::INFINITY;
    assert_eq!(core(&index, &infinite, &[0, -1]), "E_INVALID_ARGUMENT", "a pose entry is finite");
}

/// The code and core status of a refused adoption of `bundle` by a fresh bridge.
fn backend_refusal(bundle: &EngineBundle) -> (&'static str, String) {
    let mut backend = RenderBackend::new(FakeCore::new(SIZE), limits());
    let error = backend.adopt_engine(bundle).expect_err("refused");
    assert!(backend.core.adoptions.is_empty() && backend.core.posed.is_empty(), "nothing was adopted");
    (error.code, error.context.get("status").cloned().unwrap_or_default())
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
