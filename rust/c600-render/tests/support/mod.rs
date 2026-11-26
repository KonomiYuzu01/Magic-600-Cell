//! The host scenarios that c600-render's bridge tests run over the fake core, and that `renderer/bridge`'s tests
//! (`c600-render-ffi`) run again through the C function table, by including this file: adoption, the support
//! report, a retained publication from preparation to installation, frames, reports, picks resolved through the
//! host, staleness, set retirement, device loss and refused recovery, and frames with report batches; and on route B
//! (route B, accepted for now by the owner on 18 Nov 2026, pending three owner-machine checks) confirmed presentations and recovery. Every result is a synthetic fixture
//! outcome, not a GPU run.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_digest::sha256_hex;
use c600_display::codes;
use c600_display::host::{Backend, FrameInput, Host};
use c600_display::pick::{FrameStamp, Partial, PickMode, Presented, PreviewShown, Projection, Requested};
use c600_display::plan::*;
use c600_display::produce::{self, full_set, GeometryInput};
use c600_display::receipt::Status;
use c600_display::validate::{BoundArray, EngineBundle, LimitProfile, Support};
use c600_kernel::{Derivation, Kernel, ProtectedSet, Snapshot, SLOTS};
use c600_render::abi::TARGET_SURFACE;
use c600_render::backend::subjects_digest;
use c600_render::fake::{DEVICE_REMOVED_REASON, DISPLAY_TICKS};
use c600_render::{abi, CoreAbi, DisplayTime, FakeCore, PickPoll, RenderBackend};

/// A core whose fake a test can drive (complete work, place picks, remove the device).
pub trait Driver: CoreAbi {
    fn fake(&mut self) -> &mut FakeCore;
}

impl Driver for FakeCore {
    fn fake(&mut self) -> &mut FakeCore {
        self
    }
}

pub const V1: ViewId = ViewId(1);
pub const SIZE: (u32, u32) = (800, 600);
pub const CAMERA: [f32; 16] = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];
pub const SOURCE: &str = "ab";

pub fn assets() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

pub fn kernel() -> &'static Kernel {
    static KERNEL: OnceLock<Kernel> = OnceLock::new();
    KERNEL.get_or_init(|| Kernel::load(&assets()).expect("the shipped model loads"))
}

pub fn geometry() -> &'static GeometryInput {
    static GEOMETRY: OnceLock<GeometryInput> = OnceLock::new();
    GEOMETRY.get_or_init(|| GeometryInput::load(&assets()).expect("the shipped geometry loads"))
}

/// The state after one twist from solved, so that stickers sit away from their home slots.
pub fn twisted() -> &'static Snapshot {
    static STATE: OnceLock<Snapshot> = OnceLock::new();
    STATE.get_or_init(|| {
        let recipe = kernel().word(&[1]).expect("a legal letter");
        match kernel().derive(&kernel().solved(), &recipe, &ProtectedSet::none()).expect("derives") {
            Derivation::Derived(derived) => derived.state,
            Derivation::Refused(refused) => panic!("refused: {:?}", refused.refusal),
        }
    })
}

pub fn limits() -> LimitProfile {
    LimitProfile::uniform("bridge-fixture/1", 1 << 40)
}

pub fn bundle(state: &Snapshot, revision: &RevisionRef) -> EngineBundle {
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
pub fn publication(base: &RevisionRef, primitives: Vec<Primitive>, objects: &[(u64, ObjectId)]) -> Publication {
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
pub struct Unasked;

impl Projection for Unasked {
    fn slot(&self, _: &StateRef, object: &ObjectId) -> Partial<u32> {
        panic!("a projection was asked for {object:?}")
    }
}

/// The slots the scenario uses: a sticker away from home (its slot and home), a sticker at home and a hidden slot.
pub struct Slots {
    pub moved: usize,
    pub home: u32,
    pub fixed: usize,
    pub hidden: usize,
}

pub fn slots(labels: &[i32]) -> Slots {
    let moved = (0..SLOTS).find(|&s| labels[s] as usize != s).expect("the twist moves a sticker");
    let fixed = (0..SLOTS).find(|&s| labels[s] as usize == s).expect("the twist fixes a sticker");
    let hidden = (0..SLOTS).find(|&s| s != moved && s != fixed).expect("a third slot");
    Slots { moved, home: labels[moved] as u32, fixed, hidden }
}

pub fn scenario<C: Driver>(core: C) {
    let state = twisted();
    let labels = state.labels();
    let at = slots(labels);
    let base = produce::revision(&geometry().identity, 12, state);

    // Adoption and the support report.
    let mut backend = RenderBackend::new(core, limits());
    assert_eq!(backend.adopt_engine(&bundle(state, &base)), Ok(12));
    assert_eq!(backend.core.fake().adoptions, [(12, labels[0] as u32)]);
    let report = backend.describe_backend().expect("a live core describes itself");
    assert_eq!((report.interface_version, report.device_epoch, report.backend_build.len()), (8, 1, 64));
    for row in ["F01", "F02", "F18", "F20", "J01", "J02"] {
        match &report.features[row] {
            Support::Supported { layout_version, limits: declared, stages } => {
                assert_eq!((*layout_version, declared, stages.len()), (1, &limits(), 0), "{row}")
            }
            other => panic!("{row}: {other:?}"),
        }
    }
    for row in ["F22", "F23", "J13"] {
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

pub fn preview(base: &RevisionRef, id: u64) -> PreviewDisplay {
    PreviewDisplay {
        preview: PreviewId(id),
        base: base.clone(),
        literal: format!("{id:064x}"),
        effect: "e".repeat(64),
        primitives: Vec::new(),
    }
}

/// The core frame ID of each stamp, from the frame reports (every frame must have drawn).
fn frame_ids<C: Driver>(host: &mut Host<RenderBackend<C>>, stamps: &[&FrameStamp]) -> Vec<u64> {
    host.backend.core.fake().complete();
    let reports = host.backend.read_frame_reports().expect("reports");
    stamps
        .iter()
        .map(|stamp| {
            let report = reports.iter().find(|r| &&r.stamp == stamp).expect("a report of the frame");
            assert_eq!(report.error, None);
            report.frame
        })
        .collect()
}

/// Route B (route B, accepted for now by the owner on 18 Nov 2026, pending three owner-machine checks): on a `sa2_create` context with a surface the
/// core confirms presentations under the exact rule and recovers its own device; the old epoch's sets never draw.
pub fn route_b<C: Driver>(core: C) {
    let state = twisted();
    let at = slots(state.labels());
    let base = produce::revision(&geometry().identity, 12, state);
    let mut backend = RenderBackend::new(core, limits());
    assert_eq!(backend.adopt_engine(&bundle(state, &base)), Ok(12));
    let report = backend.describe_backend().expect("a live core describes itself");
    assert_eq!((report.interface_version, report.device_epoch), (8, 1));
    for row in ["F01", "F02", "F18", "F20", "F22", "F23"] {
        assert!(matches!(report.features[row], Support::Supported { .. }), "{row}");
    }
    let mut ctx = produce::fixture_context(geometry(), limits());
    ctx.support = report;
    let mut host = Host::new(backend, ctx, base.clone());
    host.open_view(V1).expect("view 1 opens");
    host.backend.set_camera_matrix(V1, 0, CAMERA);

    // A ring slot is not this context's target.
    let wrong = host.frame(V1).expect_err("a ring slot on a surface context");
    assert_eq!(wrong.context["status"], "E_WRONG_STATE");
    host.backend.set_frame_target(TARGET_SURFACE);

    // A retained publication, installed.
    let retained = produce::retained(state, &full_set(), &full_set(), Vec::new());
    let objects = [(1, ObjectId::Sticker(at.home))];
    host.publish(CommandId(1), publication(&base, vec![Primitive::Retained(retained.clone())], &objects));
    host.pump();
    host.backend.core.fake().complete();
    host.pump();
    let first_set = host.installed(V1).expect("installed").resources;

    // Four frames: no preview, then preview 7 twice, then no preview. Only shown frames are confirmed, once each and
    // in frame order; a frame the display passed is never confirmed (the exact rule).
    let none = host.frame(V1).expect("a frame");
    host.show_preview(V1, Some(preview(&base, 7))).expect("a preview");
    let shown = host.frame(V1).expect("a frame");
    let later = host.frame(V1).expect("a frame");
    host.show_preview(V1, None).expect("no preview");
    let cleared = host.frame(V1).expect("a frame");
    let submitted = &host.backend.core.fake().submitted;
    assert!(submitted.iter().all(|row| row.target == TARGET_SURFACE && row.resource_set == first_set.0));
    let ids = frame_ids(&mut host, &[&none, &shown, &later, &cleared]);
    assert!(host.read_presented().is_empty(), "nothing is confirmed before the display shows a frame");
    assert!(host.backend.core.fake().show(ids[1]));
    assert!(!host.backend.core.fake().show(ids[0]), "a passed frame is never confirmed");
    let read = host.read_presented();
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].stamp, shown);
    let shown_at = DisplayTime {
        sequence: shown.sequence,
        device_epoch: shown.device_epoch,
        surface: shown.surface,
        display_qpc: DISPLAY_TICKS,
    };
    assert_eq!(host.backend.take_display_times(), [shown_at], "the confirming row's display time reaches the caller");
    assert!(host.backend.take_display_times().is_empty(), "each display time is taken once");
    let expected = PreviewShown { preview: PreviewId(7), literal: format!("{:064x}", 7), base: 12 };
    assert_eq!(read[0].presented, Presented::Previews(vec![expected.clone()]));
    assert_eq!(host.presented_preview(), Some(expected.clone()), "ONE.41 may name the presented preview");
    assert!(host.read_presented().is_empty(), "each frame is confirmed once");
    assert!(host.backend.core.fake().show(ids[2]));
    assert_eq!(host.read_presented()[0].stamp, later);
    assert_eq!(host.presented_preview(), Some(expected));
    assert!(host.backend.core.fake().show(ids[3]));
    assert_eq!(host.read_presented()[0].presented, Presented::NoPreview);
    assert_eq!(host.presented_preview(), None, "a frame without a preview clears the latch");
    let times = host.backend.take_display_times();
    assert_eq!(times.iter().map(|t| t.sequence).collect::<Vec<_>>(), [later.sequence, cleared.sequence]);
    assert!(times[0].display_qpc < times[1].display_qpc);

    // A presented row that names other bindings than its frame is a contradiction, and the latch clears.
    host.show_preview(V1, Some(preview(&base, 8))).expect("a preview");
    let misbound = host.frame(V1).expect("a frame");
    let id = frame_ids(&mut host, &[&misbound])[0];
    assert!(host.backend.core.fake().show(id));
    host.backend.core.fake().misbind_presented(misbound.sequence + 100);
    let error = host.backend.read_presented().expect_err("a contradicting row");
    assert!(host.backend.take_display_times().is_empty(), "a contradicting read keeps no display time");
    assert_eq!((error.code, error.context["status"].as_str()), (codes::RENDERER_UNAVAILABLE, "contradiction"));
    let again = host.frame(V1).expect("a frame");
    let id = frame_ids(&mut host, &[&again])[0];
    assert!(host.backend.core.fake().show(id));
    assert_eq!(host.read_presented().len(), 1);
    assert!(host.presented_preview().is_some());

    // Device loss: one frame's report is unread when the device goes. Recovery succeeds with epoch 2.
    let pending = host.backend.core.fake().prepared.len();
    host.frame(V1).expect("a frame before the loss");
    host.backend.core.fake().remove_device();
    host.device_lost();
    assert_eq!(host.presented_preview(), None, "loss clears the latch (RM.23)");
    assert!(host.frame(V1).is_err());
    assert!(!host.backend.core.fake().show(id + 2), "a removed device confirms nothing");
    host.recover().expect("route B recovers its own device");
    assert_eq!(host.ctx.support.device_epoch, 2);
    assert_eq!(host.backend.core.device_status(), (abi::Status::OK, 0));
    assert_eq!(host.backend.counters().expect("counters").reports_dropped, 1, "the unread report is dropped");
    let old = host.backend.core.poll(1).expect_err("an old epoch's ticket is gone");
    assert_eq!(old, abi::Status::WRONG_STATE);
    let not_removed = host.backend.recover().expect_err("a live device has nothing to recover");
    assert_eq!(not_removed.context["status"], "E_WRONG_STATE");
    // The old epoch's frames are gone in the core, so the bridge finds a pick on one stale before the core is asked
    // (the host refuses it earlier, at its device epoch).
    let error = Backend::request_pick(&mut host.backend, &again, (10, 20), Requested::Occurrence).expect_err("old");
    assert_eq!((error.code, error.context["field"].as_str()), (codes::STALE_PICK, "sequence"));

    // The old epoch's set never draws on the new device (RM.23): the frame fails before it reaches the core.
    let count = host.backend.core.fake().submitted.len();
    let refused = host.frame(V1).expect_err("the installed set belongs to the old device");
    assert_eq!(refused.code, codes::RENDERER_UNAVAILABLE);
    assert_eq!(host.backend.core.fake().submitted.len(), count);

    // Prepared and activated again on the new device, the publication draws; the old set's retirement is a no-op.
    host.publish(CommandId(2), publication(&base, vec![Primitive::Retained(retained)], &objects));
    host.pump();
    host.backend.core.fake().complete();
    host.pump();
    assert_eq!(host.backend.core.fake().prepared.len(), pending + 1);
    let second_set = host.installed(V1).expect("installed").resources;
    assert_ne!(second_set, first_set);
    assert_eq!(host.backend.fault, None, "retiring the old epoch's set does not reach the core");
    let fresh = host.frame(V1).expect("a frame on the new device");
    assert_eq!(fresh.device_epoch, 2);
    let row = *host.backend.core.fake().submitted.last().expect("submitted");
    assert_eq!((row.device_epoch, row.resource_set, row.target), (2, second_set.0, TARGET_SURFACE));
    let id = frame_ids(&mut host, &[&fresh])[0];
    assert!(host.backend.core.fake().show(id));
    assert_eq!(host.read_presented()[0].stamp, fresh);
    let times = host.backend.take_display_times();
    assert_eq!(
        times.iter().map(|t| (t.sequence, t.device_epoch)).collect::<Vec<_>>(),
        [(again.sequence, 1), (fresh.sequence, 2)]
    );
    assert_eq!(host.presented_preview().map(|p| p.preview), Some(PreviewId(8)));
}

pub fn frame_input(sequence: u64, camera: u64, publication_generation: u64) -> FrameInput {
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

/// Frames need a camera matrix and an installed set; their reports come back in batches, in frame order.
pub fn frames_and_batches<C: Driver>(core: C) {
    let mut backend = RenderBackend::new(core, limits());
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
