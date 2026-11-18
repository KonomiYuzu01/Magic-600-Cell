//! The display host over the bridge, end to end, on the shipped model: adoption, the support report, a retained
//! publication from preparation to installation, frames, reports, picks resolved through the host, staleness, set
//! retirement, device loss and refused recovery. Each scenario runs over the fake core directly and again through
//! the C function table ([`common::Shimmed`]). Every result is a synthetic fixture outcome, not a GPU run.

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_digest::sha256_hex;
use c600_display::array::Array;
use c600_display::codes;
use c600_display::error::Error;
use c600_display::host::{Backend, FrameInput, Host, Poll, Preparation, PublicationStamp, Ticket};
use c600_display::pick::{FrameStamp, Partial, PickMode, Projection, Requested};
use c600_display::plan::*;
use c600_display::produce::{self, full_set, GeometryInput};
use c600_display::receipt::{CancelReason, Status};
use c600_display::validate::{BoundArray, EngineBundle, LimitProfile, Support};
use c600_kernel::{Derivation, Kernel, ProtectedSet, Snapshot, SLOTS};
use c600_render::backend::subjects_digest;
use c600_render::fake::DEVICE_REMOVED_REASON;
use c600_render::{abi, CoreAbi, FakeCore, PickPoll, RenderBackend};

use common::{Driver, Shimmed};

const V1: ViewId = ViewId(1);
const SIZE: (u32, u32) = (800, 600);
const CAMERA: [f32; 16] = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];
const SOURCE: &str = "ab";

fn assets() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

fn kernel() -> &'static Kernel {
    static KERNEL: OnceLock<Kernel> = OnceLock::new();
    KERNEL.get_or_init(|| Kernel::load(&assets()).expect("the shipped model loads"))
}

fn geometry() -> &'static GeometryInput {
    static GEOMETRY: OnceLock<GeometryInput> = OnceLock::new();
    GEOMETRY.get_or_init(|| GeometryInput::load(&assets()).expect("the shipped geometry loads"))
}

/// The state after one twist from solved, so that stickers sit away from their home slots.
fn twisted() -> &'static Snapshot {
    static STATE: OnceLock<Snapshot> = OnceLock::new();
    STATE.get_or_init(|| {
        let recipe = kernel().word(&[1]).expect("a legal letter");
        match kernel().derive(&kernel().solved(), &recipe, &ProtectedSet::none()).expect("derives") {
            Derivation::Derived(derived) => derived.state,
            Derivation::Refused(refused) => panic!("refused: {:?}", refused.refusal),
        }
    })
}

fn limits() -> LimitProfile {
    LimitProfile::uniform("bridge-fixture/1", 1 << 40)
}

fn bundle(state: &Snapshot, revision: &RevisionRef) -> EngineBundle {
    let set = full_set();
    let input = produce::retained(state, &set, &set, Vec::new());
    let arrays = [("labels", input.labels), ("visibility", input.visibility), ("interaction", input.interaction)];
    EngineBundle {
        revision: revision.clone(),
        arrays: arrays
            .into_iter()
            .map(|(name, array)| BoundArray { name: name.to_owned(), revision: revision.state, array })
            .collect(),
    }
}

/// A one-plan publication for view 1 at `base`: `primitives`, and an instance on primitive 0 per object.
fn publication(base: &RevisionRef, primitives: Vec<Primitive>, objects: &[(u64, ObjectId)]) -> Publication {
    let digest = sha256_hex(SOURCE.as_bytes());
    let instances: Vec<DisplayInstance> = objects
        .iter()
        .map(|(id, object)| DisplayInstance {
            instance: InstanceId(*id),
            plan: PlanId(1),
            object: object.clone(),
            primitive: 0,
            piece: None,
            interaction: true,
        })
        .collect();
    let link = SourceLink::Range { digest: digest.clone(), start: 0, end: 1 };
    let entries =
        instances.iter().map(|i| (i.instance, MapEntry { object: i.object.clone(), source: link.clone() })).collect();
    let plan = DisplayPlan {
        plan: PlanId(1),
        base: base.clone(),
        subject: StateRef::Snapshot(base.clone()),
        derivation: None,
        reachability: None,
        dimension: 4,
        geometry: GeometryBinding { identity: geometry().identity.clone(), digests: geometry().digests() },
        primitives,
        charts: Vec::new(),
        instances,
        source_map: MapId(1),
        precision: "binary32".to_owned(),
        dependencies: BTreeMap::from([("state".to_owned(), base.state.to_string())]),
    };
    let map = SourceMap { map: MapId(1), plan: PlanId(1), base: base.clone(), sources: vec![digest.clone()], entries };
    Publication {
        target: V1,
        base: base.clone(),
        nodes: Vec::new(),
        plans: vec![plan],
        maps: vec![map],
        sources: BTreeMap::from([(digest, Arc::new(SOURCE.as_bytes().to_vec()))]),
    }
}

/// Resolution at the occurrence level asks no projection.
struct Unasked;

impl Projection for Unasked {
    fn slot(&self, _: &StateRef, object: &ObjectId) -> Partial<u32> {
        panic!("a projection was asked for {object:?}")
    }
}

/// The slots the scenario uses: a sticker away from home (its slot and home), a sticker at home and a hidden slot.
struct Slots {
    moved: usize,
    home: u32,
    fixed: usize,
    hidden: usize,
}

fn slots(labels: &[i32]) -> Slots {
    let moved = (0..SLOTS).find(|&s| labels[s] as usize != s).expect("the twist moves a sticker");
    let fixed = (0..SLOTS).find(|&s| labels[s] as usize == s).expect("the twist fixes a sticker");
    let hidden = (0..SLOTS).find(|&s| s != moved && s != fixed).expect("a third slot");
    Slots { moved, home: labels[moved] as u32, fixed, hidden }
}

fn scenario<C: Driver>(core: C) {
    let state = twisted();
    let labels = state.labels();
    let at = slots(labels);
    let base = produce::revision(&geometry().identity, 12, state);

    // Adoption and the support report.
    let mut backend = RenderBackend::new(core, limits());
    assert_eq!(backend.adopt_engine(&bundle(state, &base)), Ok(12));
    assert_eq!(backend.core.fake().adoptions, [(12, labels[0] as u32)]);
    let report = backend.describe_backend().expect("a live core describes itself");
    assert_eq!((report.interface_version, report.device_epoch, report.backend_build.len()), (4, 1, 64));
    for row in ["F01", "F02", "F18", "F20"] {
        match &report.features[row] {
            Support::Supported { layout_version, limits: declared, stages } => {
                assert_eq!((*layout_version, declared, stages.len()), (1, &limits(), 0), "{row}")
            }
            other => panic!("{row}: {other:?}"),
        }
    }
    for row in ["J01", "J02", "F22", "F23", "J13"] {
        let Support::Unsupported(error) = &report.features[row] else { panic!("{row} is supported") };
        assert_eq!((error.code, error.context["feature"].as_str()), (codes::UNSUPPORTED, row));
    }
    let mut ctx = produce::fixture_context(geometry(), limits());
    ctx.support = report;
    let mut host = Host::new(backend, ctx, base.clone());
    host.open_view(V1).expect("view 1 opens");
    host.backend.set_camera_matrix(V1, 0, CAMERA);

    // A retained publication: prepared per slot, installed on completion.
    let mut visibility = full_set();
    visibility[at.hidden] = 0;
    let retained = produce::retained(state, &visibility, &full_set(), Vec::new());
    let objects = [(1, ObjectId::Sticker(at.home)), (2, ObjectId::Slot(at.fixed as u32))];
    let receipt = host.publish(CommandId(1), publication(&base, vec![Primitive::Retained(retained.clone())], &objects));
    assert!(!receipt.status.is_terminal(), "{:?}", receipt.status);
    host.pump();
    assert!(host.installed(V1).is_none(), "the upload is pending");
    host.backend.core.fake().complete();
    host.pump();
    assert!(matches!(host.receipt(CommandId(1)).expect("a receipt").status, Status::Completed(_)));
    let first_set = host.installed(V1).expect("installed").resources;
    let prepared = host.backend.core.fake().prepared.clone();
    assert_eq!(prepared.len(), 1);
    assert_eq!(prepared[0].0, 1);
    assert_eq!(prepared[0].1, BTreeMap::from([(at.moved, 1), (at.fixed, 2)]), "a sticker is drawn where it sits");
    assert_eq!(prepared[0].2, visibility);

    // A frame carries the stamp, the active set and the camera.
    let stamp = host.frame(V1).expect("a frame");
    let row = *host.backend.core.fake().submitted.last().expect("submitted");
    assert_eq!(row.resource_set, first_set.0);
    assert_eq!((row.view, row.view_generation, row.publication_generation), (1, 1, 1));
    assert_eq!((row.engine_revision, row.surface, row.sequence), (12, stamp.surface, stamp.sequence));
    assert_eq!((row.viewport_width, row.viewport_height, row.flags), (SIZE.0, SIZE.1, 0));
    assert_eq!((row.camera_matrix, row.subjects_digest), (CAMERA, subjects_digest(&stamp.subjects)));
    let unconfirmed = host.backend.read_presented().expect_err("ABI 4 confirms no presentation");
    assert_eq!(
        (unconfirmed.code, unconfirmed.context["status"].as_str()),
        (codes::RENDERER_UNAVAILABLE, "E_UNAVAILABLE")
    );
    assert!(host.read_presented().is_empty());
    assert_eq!(host.presented_preview(), None, "the presented-preview latch stays clear");

    // Reports in frame order, with the frame's stamp.
    host.backend.core.fake().complete();
    let reports = host.backend.read_frame_reports().expect("reports");
    assert_eq!(reports.len(), 1);
    assert_eq!((&reports[0].stamp, &reports[0].error, reports[0].view_vertex_evals), (&stamp, &None, 72_000));
    let frame = reports[0].frame;
    assert_eq!(host.backend.counters().expect("counters").frames_reported, 1);

    // Picks: the ID pass's instance resolves through the host to the sticker; a hidden slot answers nothing.
    host.backend.core.fake().place(10, 20, at.moved);
    host.backend.core.fake().place(30, 40, at.hidden);
    let sticker = host.backend.request_pick(frame, 10, 20).expect("a pick");
    let nothing = host.backend.request_pick(frame, 30, 40).expect("a pick");
    assert_eq!(host.backend.poll_pick(sticker), Ok(PickPoll::Pending));
    host.backend.core.fake().complete();
    let Ok(PickPoll::Ready(Some(reply))) = host.backend.poll_pick(sticker) else { panic!("a stamped instance") };
    assert_eq!(reply.instance, InstanceId(1));
    assert_eq!(Ok(&reply.binding), host.pick_binding(V1).as_ref(), "the reply binds the current view");
    match host.resolve_pick(&reply, PickMode::Explicit, Requested::Occurrence, &Unasked) {
        Ok(Some(Partial::Total(resolution))) => assert_eq!(resolution.target, ObjectId::Sticker(at.home)),
        other => panic!("{other:?}"),
    }
    assert_eq!(host.backend.poll_pick(nothing), Ok(PickPoll::Ready(None)));
    let misbound = host.backend.request_pick(frame, 10, 20).expect("a pick");
    host.backend.core.fake().misbind_picks(99);
    host.backend.core.fake().complete();
    let error = host.backend.poll_pick(misbound).expect_err("a pick bound to another publication than its frame");
    assert_eq!((error.code, error.context["status"].as_str()), (codes::RENDERER_UNAVAILABLE, "contradiction"));
    assert_eq!(
        host.backend.request_pick(frame, SIZE.0, 0).map(|_| ()).expect_err("outside").code,
        codes::RENDERER_UNAVAILABLE
    );

    // A reopened view: a pick on the old frame is stale by its surface generation, as the host names it.
    host.close_view(V1);
    host.open_view(V1).expect("view 1 reopens");
    let stale = host.backend.request_pick(frame, 10, 20).expect("a pick");
    host.frame(V1).expect("a frame of the new generation draws no set");
    assert_eq!(host.backend.core.fake().submitted.last().expect("submitted").resource_set, 0);
    host.backend.core.fake().complete();
    assert_eq!(host.backend.poll_pick(stale), Ok(PickPoll::Stale("surface_generation")));

    // A second publication replaces the first, whose set is retired: picks on its frame are stale.
    host.publish(CommandId(2), publication(&base, vec![Primitive::Retained(retained.clone())], &objects));
    host.pump();
    host.backend.core.fake().complete();
    host.pump();
    let second_set = host.installed(V1).expect("installed").resources;
    assert_ne!(second_set, first_set);
    assert_eq!(host.backend.fault, None);
    host.frame(V1).expect("a frame");
    assert_eq!(host.backend.core.fake().submitted.last().expect("submitted").resource_set, second_set.0);
    let retired = host.backend.request_pick(frame, 10, 20).expect("a pick");
    assert_eq!(host.backend.poll_pick(retired), Ok(PickPoll::Stale("publication_generation")));

    // Device loss: the support report fails, pending work fails, frames stop and recovery is refused.
    host.publish(CommandId(3), publication(&base, vec![Primitive::Retained(retained)], &objects));
    host.pump();
    host.backend.core.fake().remove_device();
    let lost = host.backend.describe_backend().expect_err("a removed device");
    assert_eq!((lost.code, lost.context["removed"].as_str()), (codes::RENDERER_UNAVAILABLE, "0x887a0005"));
    host.pump();
    match host.receipt(CommandId(3)).expect("a receipt").status {
        Status::Failed(error) => assert_eq!(error.code, codes::RENDERER_UNAVAILABLE),
        other => panic!("{other:?}"),
    }
    assert!(host.frame(V1).is_err());
    let refused = host.recover().expect_err("ABI 4 cannot recover a device");
    assert_eq!(refused.context["status"], "E_UNAVAILABLE");
    assert!(refused.context["reason"].contains("RPI.46"), "{refused:?}");
    assert_eq!(host.backend.core.device_status(), (abi::Status::DEVICE_REMOVED, DEVICE_REMOVED_REASON));
}

#[test]
fn the_host_runs_over_the_fake_core() {
    scenario(FakeCore::new(SIZE));
}

#[test]
fn the_host_runs_through_the_c_function_table() {
    scenario(Shimmed::new(FakeCore::new(SIZE), 5000));
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

fn frame_input(sequence: u64, camera: u64, publication_generation: u64) -> FrameInput {
    FrameInput {
        stamp: FrameStamp {
            device_epoch: 1,
            surface: 1,
            surface_generation: 1,
            sequence,
            engine_revision: 1,
            view: V1,
            view_generation: 1,
            publication_generation,
            subjects: Vec::new(),
            camera,
            viewport: SIZE,
            visibility: 0,
            interaction: 0,
        },
        preview: None,
    }
}

#[test]
fn frames_need_a_camera_and_a_set_and_reports_come_in_batches() {
    let mut backend = RenderBackend::new(Shimmed::new(FakeCore::new(SIZE), 0), limits());
    let labels: Vec<u32> = (0..SLOTS as u32).collect();
    let image = c600_render::EngineImage { revision: 1, labels: &labels, pose: None, digest: [0; 32] };
    assert_eq!(backend.core.engine_adopt(&image), Ok(1));

    let missing = backend.submit_frame(frame_input(1, 0, 0)).expect_err("no camera matrix");
    assert_eq!(missing.code, codes::PLAN_INVALID);
    backend.set_camera_matrix(V1, 0, CAMERA);
    let unset = backend.submit_frame(frame_input(1, 0, 3)).expect_err("no set holds publication 3");
    assert_eq!(unset.code, codes::RENDERER_UNAVAILABLE);
    let mut nan = CAMERA;
    nan[0] = f32::NAN;
    backend.set_camera_matrix(V1, 1, nan);
    let refused = backend.submit_frame(frame_input(1, 1, 0)).expect_err("a non-finite camera");
    assert_eq!(refused.context["status"], "E_INVALID_ARGUMENT");
    assert!(refused.context["detail"].contains("non-finite camera"), "{refused:?}");

    let ids: Vec<u64> = (1..=70).map(|s| backend.submit_frame(frame_input(s, 0, 0)).expect("a frame")).collect();
    backend.core.fake().complete();
    let reports = backend.read_frame_reports().expect("reports");
    assert_eq!(reports.iter().map(|r| r.frame).collect::<Vec<_>>(), ids, "every report, in frame order");
    assert_eq!(reports.iter().map(|r| r.stamp.sequence).collect::<Vec<_>>(), (1..=70).collect::<Vec<_>>());
    assert!(backend.read_frame_reports().expect("none left").is_empty());
}
