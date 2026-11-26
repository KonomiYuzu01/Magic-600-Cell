//! [`RenderBackend`]: c600-display's [`Backend`] over a [`CoreAbi`]. The ABI 5 product calls for engine adoption,
//! frame reports and picks are also methods of their own, in the core's terms (its frame IDs and per-frame rows), and
//! the trait's adapter operations (RM.14) map onto them; `counters` has no trait operation.
//!
//! What the bridge decides, because the core's interface and the host's types differ:
//! - A publication becomes per-slot rows: the one retained primitive's visibility and interaction sets, and each
//!   instance on the slot its sticker occupies under the primitive's labels (or the slot it names); or the one posed
//!   primitive (J02), every slot drawn and none pickable. Other primitives, a second sticker primitive, an instance
//!   of a posed primitive, two instances on one slot and instance IDs that do not fit the ID pass's u32 (or are 0)
//!   are refused.
//! - A jumbling engine bundle (render/1 section 7) is adopted as the core's pose path over the home labels: every
//!   sticker keeps its own colour and moves with its piece's pose, as W-J draws it.
//! - The core reports no limit profile and no stages: every supported row carries the profile the bridge was given.
//! - F20 (atomic publication and latest-wins replacement), which the host requires of every publication, is not a
//!   drawing row and the core does not name it. Its backend operations are ABI 4's prepare, poll, activate, cancel
//!   and retire, which the bridge maps, and its replacement rules are the host's (RM.17 to RM.22), so the bridge
//!   reports it supported unless the core names it.
//! - A support report naming a removed device is an error, so the host notices the loss (RM.23).
//! - The frame ID `submit_frame` returns is the core's (its ready fence value), not the stamp's sequence.
//! - The frame's resource set is the activated set of its view, view generation and publication generation; its
//!   camera matrix is the one set for its view and camera version ([`RenderBackend::set_camera_matrix`]); its target
//!   is the one set by [`RenderBackend::set_frame_target`].
//! - ABI 8 (ADR-D02 B18b, a candidate behind the shell's --d02): while panes are set ([`RenderBackend::set_panes`]),
//!   the next frame is a composite frame (`sa2_frame_submit_panes`). The host's stamp names the first pane, whose
//!   view, view generation, publication generation and camera version it must carry; each pane's set and camera
//!   matrix are found as a one-view frame's are, from its own view's bindings. Its report names the first pane and
//!   its vertex evaluations are the panes' sum ([`RenderBackend::read_pane_rows`] gives each pane's).
//! - `cancel` and `retire` cannot return an error through the trait; a failure is kept in [`RenderBackend::fault`].
//! - Recovery (ABI 5, route B; accepted for now by the owner on 18 Nov 2026, pending three owner-machine checks)
//!   forgets every resource set of the old device epoch: the core released them, and RM.23 lets no old resource token
//!   draw on the new device. A frame naming such a set fails until its publication is prepared and activated again;
//!   retiring one is a no-op.
//! - Every core failure is `EV3:busy.renderer_unavailable` except `SA2_E_UNSUPPORTED`, which is
//!   `EV3:input.display_unsupported`; the context names the status and the core's last error.
//! - The trait's adoption answers a bundle at the revision the core already holds (the whole revision reference, not
//!   only its state counter) as adopted, without a second copy: the shell adopts into the core before the host does,
//!   and the core refuses a revision that does not increase. Any other bundle goes to [`RenderBackend::adopt_engine`],
//!   so one at the same state counter with another digest reaches the core, which refuses it.
//! - The trait's frame reports are empty: RM.41's reports name frames that stopped a user stage, and the product path
//!   runs none (`sa2_frame_report.fallback` is always 0). The core's row of every frame stays with
//!   [`RenderBackend::read_frame_reports`].
//! - A pick through the trait asks the ID pass on the frame with the given stamp; the requested kind and level are the
//!   host's to resolve (RPI.25). A frame older than every frame the bridge keeps is refused as stale
//!   (`EV3:stale.display_pick` at `sequence`), so the host drops a passive pick on it; a stamp the bridge never
//!   submitted is invalid, and so is a point outside the frame's viewport, before the core is asked. So is a frame the
//!   core certainly no longer retains: the bridge mirrors the part of the core's rule it sees, a frame with 64 newer
//!   frames whose reports [`RenderBackend::read_frame_reports`] read, or one of an earlier device epoch. The core's
//!   own refusal, for a frame it dropped otherwise (an unread report past its bound) among others, is a core failure:
//!   its status does not tell that case from its pick bound. A pick the core finds stale is refused as
//!   `EV3:stale.display_pick`, naming the field that moved.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use c600_digest::sha256;
use c600_display::array::Elem;
use c600_display::codes;
use c600_display::error::Error;
use c600_display::host::{self, Backend, FrameInput, Poll, Preparation, PublicationStamp, ResourceSet, Ticket};
use c600_display::pick::{FrameStamp, PickBinding, PickReply, Presented, PresentedFrame, Requested};
use c600_display::plan::{CommandId, InstanceId, ObjectId, Primitive, Publication, RevisionRef, ViewId};
use c600_display::receipt::CancelReason;
use c600_display::stage;
use c600_display::validate::{
    validate_engine_bundle, CheckedBundle, EngineBundle, LimitProfile, PlanSubject, Support, SupportReport,
};
use c600_kernel::json::{self, Value};
use c600_kernel::Puzzle;

use crate::abi::*;
use crate::api::{CoreAbi, EngineImage, PoseArrays, PreparationInput};

/// Frame records kept for reports, presentations and picks: more than the core retains (every unreported frame up
/// to its bound of 1024, and the last 64 reported).
const FRAME_RECORDS: usize = 2048;
/// Reported frames the core retains for picks (`sa2_pick_request`; `RetainedFrames` in the core): a frame with this
/// many newer reported frames is gone.
const REPORTED_RETAINED: usize = 64;
/// Camera matrices kept per view.
const CAMERAS: usize = 8;
/// Display times kept until the caller takes them; the oldest go first.
const DISPLAY_TIMES: usize = 1024;

/// A frame the bridge submitted.
#[derive(Clone, Debug)]
struct FrameRecord {
    id: u64,
    input: FrameInput,
    dependencies: BTreeMap<String, String>,
}

/// ABI 8 (ADR-D02 B18b): one pane of the next composite frame, in the host's terms. The bridge finds its resource set
/// and its camera matrix as it finds a one-view frame's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaneSpec {
    pub view: ViewId,
    pub view_generation: u64,
    /// The installed publication's generation, or 0 for none (every slot drawn, nothing pickable).
    pub publication_generation: u64,
    /// The view's camera version.
    pub camera: u64,
    /// The rectangle in target pixels.
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    /// 0: no style; else a style slot 1 to [`STYLE_SLOTS`].
    pub style: u32,
}

/// An activated resource set.
#[derive(Clone, Debug)]
struct SetRecord {
    view: ViewId,
    view_generation: u64,
    publication_generation: u64,
    dependencies: BTreeMap<String, String>,
}

/// A frame report with the stamp of its frame.
#[derive(Clone, Debug, PartialEq)]
pub struct FrameReport {
    pub frame: u64,
    pub stamp: FrameStamp,
    /// Why the frame drew nothing, if it did not draw.
    pub error: Option<Error>,
    pub gpu_time_ns: u64,
    /// RM.39's measured view vertex evaluations.
    pub view_vertex_evals: u64,
}

/// When the display showed a confirmed frame (route B, accepted for now by the owner on 18 Nov 2026, pending three
/// owner-machine checks): `display_qpc` is the SyncQPCTime of the statistics row that confirmed it, in QPC ticks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisplayTime {
    pub sequence: u64,
    pub device_epoch: u64,
    pub surface: u64,
    pub display_qpc: u64,
}

/// A pick request of [`RenderBackend::request_pick`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PickTicket(pub u64);

/// The state of a pick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PickPoll {
    Pending,
    /// The ID pass answered: a stamped instance, or `None` where no interactive instance was drawn.
    Ready(Option<PickReply>),
    /// The core found the binding stale, naming the RPI.24 field that moved.
    Stale(&'static str),
}

/// The bridge.
pub struct RenderBackend<C: CoreAbi> {
    pub core: C,
    limits: LimitProfile,
    target: u32,
    cameras: BTreeMap<ViewId, VecDeque<(u64, [f32; 16])>>,
    sets: BTreeMap<u64, SetRecord>,
    /// Sets of an earlier device epoch, which the core released at recovery.
    lost_sets: BTreeSet<u64>,
    frames: VecDeque<FrameRecord>,
    /// The frames whose reports the bridge read, the newest [`REPORTED_RETAINED`], in frame order.
    reported: VecDeque<u64>,
    /// The newest frame the core certainly no longer retains for picks, or 0: frames are numbered in submission order
    /// and never reused, so every frame up to it is gone too.
    dropped: u64,
    /// Display times of confirmed frames not yet taken.
    displays: VecDeque<DisplayTime>,
    /// The newest failure of `cancel` or `retire`, which the trait cannot return.
    pub fault: Option<Error>,
    /// The revision of the engine image the core holds: the last bundle it accepted, which it keeps across recovery
    /// (RM.16).
    held: Option<RevisionRef>,
    /// ABI 8: the panes of the next frame ([`RenderBackend::set_panes`]); empty for a one-view frame.
    panes: Vec<PaneSpec>,
}

impl<C: CoreAbi> RenderBackend<C> {
    /// A bridge whose supported rows carry `limits`, the declared ceilings of this backend (RM.11).
    pub fn new(core: C, limits: LimitProfile) -> RenderBackend<C> {
        RenderBackend {
            core,
            limits,
            target: 0,
            cameras: BTreeMap::new(),
            sets: BTreeMap::new(),
            lost_sets: BTreeSet::new(),
            frames: VecDeque::new(),
            reported: VecDeque::new(),
            dropped: 0,
            displays: VecDeque::new(),
            fault: None,
            held: None,
            panes: Vec::new(),
        }
    }

    /// ABI 8 (ADR-D02 B18b): the panes of the next frames, in order, or none for one-view frames. While panes are set,
    /// a frame is a composite frame, and the host's stamp must name the first pane's view, view generation,
    /// publication generation and camera version. 0 or more than [`PANES_MAX`] panes are refused at the frame.
    pub fn set_panes(&mut self, panes: Vec<PaneSpec>) {
        self.panes = panes;
    }

    /// ABI 8: each pane's view, view generation and vertex evaluations of a produced composite frame, in pane order.
    pub fn read_pane_rows(&mut self, frame: u64) -> Result<Vec<Sa2PaneRow>, Error> {
        let mut rows = vec![Sa2PaneRow::new(); PANES_MAX as usize];
        let n = self.core.frame_panes(frame, &mut rows).map_err(|status| self.error(status))?;
        if n > rows.len() {
            return Err(contradiction("a composite frame names more panes than SA2_PANES_MAX"));
        }
        rows.truncate(n);
        Ok(rows)
    }

    /// The camera matrix set for `view` at camera version `camera`.
    fn camera_matrix(&self, view: ViewId, camera: u64) -> Result<[f32; 16], Error> {
        self.cameras
            .get(&view)
            .and_then(|kept| kept.iter().find(|(version, _)| *version == camera))
            .map(|(_, matrix)| *matrix)
            .ok_or_else(|| Error::invalid(format!("view {} has no camera matrix for version {camera}", view.0)))
    }

    /// The activated set of `view`'s publication at its view and publication generations, with its declared
    /// dependencies; set 0 for publication generation 0.
    fn activated(
        &self,
        view: ViewId,
        view_generation: u64,
        publication_generation: u64,
    ) -> Result<(u64, BTreeMap<String, String>), Error> {
        if publication_generation == 0 {
            return Ok((0, BTreeMap::new()));
        }
        self.sets
            .iter()
            .find(|(_, r)| {
                (r.view, r.view_generation, r.publication_generation) == (view, view_generation, publication_generation)
            })
            .map(|(set, r)| (*set, r.dependencies.clone()))
            .ok_or_else(|| Error::unavailable("no activated resource set holds the frame's publication"))
    }

    /// The frame target of the next frames: on the Godot-texture route, the registered ring slot; on route B (a
    /// `sa2_create` context; accepted for now by the owner on 18 Nov 2026, pending three owner-machine checks),
    /// [`TARGET_SURFACE`].
    pub fn set_frame_target(&mut self, target: u32) {
        self.target = target;
    }

    /// The 4D rotation of `view` at camera version `camera`, column-major as W1..W4 use it.
    pub fn set_camera_matrix(&mut self, view: ViewId, camera: u64, matrix: [f32; 16]) {
        let kept = self.cameras.entry(view).or_default();
        kept.retain(|(version, _)| *version != camera);
        kept.push_back((camera, matrix));
        while kept.len() > CAMERAS {
            kept.pop_front();
        }
    }

    /// ABI 6: the shape `view`'s frames and their ID passes draw with, or `None` for the defaults (ABI 5's constants).
    /// The core keeps it across a recovery. A value out of the header's range is refused before the core is asked.
    pub fn set_view_shape(&mut self, view: ViewId, shape: Option<Sa2ViewShape>) -> Result<(), Error> {
        let shape = shape.map(|s| Sa2ViewShape { struct_size: Sa2ViewShape::new().struct_size, reserved: 0, ..s });
        if shape.is_some_and(|s| !s.in_range()) {
            return Err(Error::invalid("a view shape value is not finite or lies outside its range"));
        }
        self.core.view_shape_set(view.0, shape.as_ref()).map_err(|status| self.error(status))
    }

    /// ABI 10: the layout `view`'s frames and their ID passes draw with, or `None` for the chain. The core checks the
    /// Hopf rings against its own cells and keeps the layout across a recovery.
    pub fn set_view_layout(&mut self, view: ViewId, layout: Option<&Sa2ViewLayout>) -> Result<(), Error> {
        if layout.is_some_and(|l| !l.in_range()) {
            return Err(Error::invalid("a view layout value is not finite or lies outside its range"));
        }
        let row = layout.map(|l| Sa2ViewLayout { struct_size: Sa2ViewLayout::new().struct_size, ..*l });
        self.core.view_layout_set(view.0, row.as_ref()).map_err(|status| self.error(status))
    }

    /// ABI 9: the focus `view`'s frames and their ID passes draw with, or `None` for the defaults. The core keeps it
    /// across a recovery. A value out of the header's range is refused before the core is asked.
    pub fn set_view_focus(&mut self, view: ViewId, focus: Option<Sa2ViewFocus>) -> Result<(), Error> {
        let focus = focus.map(|f| Sa2ViewFocus { struct_size: Sa2ViewFocus::new().struct_size, reserved: 0, ..f });
        if focus.is_some_and(|f| !f.in_range()) {
            return Err(Error::invalid("a view focus value is not finite or lies outside its range"));
        }
        self.core.view_focus_set(view.0, focus.as_ref()).map_err(|status| self.error(status))
    }

    /// ABI 7: one channel of the adopted revision's piece style (`PIECES` bytes), or `None` to clear it. A new
    /// adoption drops it in the core, so the caller sets it again for each revision. Bytes with bits outside the
    /// channel's are refused before the core is asked.
    pub fn set_piece_style(&mut self, revision: u64, piece_style: Option<&[u8]>, channel: u32) -> Result<(), Error> {
        if piece_style.is_some_and(|bytes| !piece_style_valid(bytes, channel))
            || (channel != STYLE_CHANNEL_FILL && channel != STYLE_CHANNEL_OUTLINE)
        {
            return Err(Error::invalid("a piece style is PIECES bytes with only its channel's bits"));
        }
        self.core.style_set(revision, piece_style, channel).map_err(|status| self.error(status))
    }

    /// ABI 7: style slot 1 to 4 (a one-view frame uses slot 1), or `None` to clear it. Out-of-range values are
    /// refused before the core is asked.
    pub fn set_view_style(&mut self, slot: u32, style: Option<Sa2ViewStyle>) -> Result<(), Error> {
        let style = style.map(|s| Sa2ViewStyle { struct_size: Sa2ViewStyle::new().struct_size, reserved: 0, ..s });
        if !(1..=STYLE_SLOTS).contains(&slot) || style.is_some_and(|s| !s.in_range()) {
            return Err(Error::invalid("a style slot is 1..4 and its values lie inside their ranges"));
        }
        self.core.view_style_set(slot, style.as_ref()).map_err(|status| self.error(status))
    }

    /// RM.16's adoption into the core: a validated bundle at its revision. A Full bundle gives its labels. A jumbling
    /// bundle gives its pose index, pose rows and lattice entries over the home labels (each slot shows its own
    /// sticker, which its piece's pose moves); a pose table that is not i32 [PIECES], f32 [P, 4, 4] and i32 [P] with
    /// P from 1 to `MAX_POSES` is refused before the core sees it, and the core checks the values.
    pub fn adopt_engine(&mut self, bundle: &EngineBundle) -> Result<u64, Error> {
        validate_engine_bundle(bundle)?;
        self.adopt_valid(bundle)
    }

    /// [`RenderBackend::adopt_engine`] of a bundle that [`validate_engine_bundle`] accepted already.
    fn adopt_valid(&mut self, bundle: &EngineBundle) -> Result<u64, Error> {
        let array = |name: &str| &bundle.arrays.iter().find(|a| a.name == name).expect("validated").array;
        let digest = hex32(&bundle.revision.digest)
            .ok_or_else(|| Error::invalid("the revision digest is not a SHA-256 in hexadecimal"))?;
        let revision = bundle.revision.state;
        let adopted = match bundle.revision.identity.puzzle {
            Puzzle::Full => {
                let labels = array("labels").as_u32();
                self.core.engine_adopt(&EngineImage { revision, labels: &labels, pose: None, digest })
            }
            Puzzle::Jumbling => {
                let count = array("pose_lattice").expect("pose_lattice", Elem::I32, &[None])?;
                array("pose_index").expect("pose_index", Elem::I32, &[Some(PIECES as u64)])?;
                array("poses").expect("poses", Elem::F32, &[Some(count), Some(4), Some(4)])?;
                if count == 0 || count > MAX_POSES as u64 {
                    return Err(Error::invalid("a posed bundle holds 1 to SA2_MAX_POSES poses"));
                }
                let (pose_index, poses, pose_lattice) =
                    (array("pose_index").as_i32(), array("poses").as_f32(), array("pose_lattice").as_i32());
                let labels: Vec<u32> = (0..SLOTS as u32).collect();
                let pose = Some(PoseArrays { pose_index: &pose_index, poses: &poses, pose_lattice: &pose_lattice });
                self.core.engine_adopt(&EngineImage { revision, labels: &labels, pose, digest })
            }
        };
        let adopted = adopted.map_err(|status| self.error(status))?;
        self.held = Some(bundle.revision.clone());
        Ok(adopted)
    }

    /// The display times of the frames [`Backend::read_presented`] confirmed since the last call, in frame order;
    /// at most the newest 1024 are kept.
    pub fn take_display_times(&mut self) -> Vec<DisplayTime> {
        self.displays.drain(..).collect()
    }

    /// The core's frame reports, in frame order, each with the stamp of its frame. A row that names other bindings
    /// than its frame fails the whole read, and the rows read before it are dropped with it: a core that
    /// contradicts its own frames is trusted for none of them. Each frame read counts toward the core's retention of
    /// frames for picks, which [`Backend::request_pick`] mirrors.
    pub fn read_frame_reports(&mut self) -> Result<Vec<FrameReport>, Error> {
        let mut reports = Vec::new();
        loop {
            let mut rows = vec![Sa2FrameReport::new(); 64];
            let n = self.core.frame_reports(&mut rows).map_err(|status| self.error(status))?;
            for row in &rows[..n] {
                let record = self.frame(row.frame_id)?;
                let stamp = &record.input.stamp;
                let same = (row.device_epoch, row.surface, row.surface_generation, row.sequence, row.engine_revision)
                    == (
                        stamp.device_epoch,
                        stamp.surface,
                        stamp.surface_generation,
                        stamp.sequence,
                        stamp.engine_revision,
                    )
                    && (row.view, row.view_generation, row.publication_generation)
                        == (stamp.view.0, stamp.view_generation, stamp.publication_generation);
                if !same {
                    return Err(contradiction("a frame report names other bindings than its frame"));
                }
                reports.push(FrameReport {
                    frame: row.frame_id,
                    stamp: stamp.clone(),
                    error: (row.error != 0).then(|| row_error(Status(row.error))),
                    gpu_time_ns: row.gpu_time_ns,
                    view_vertex_evals: row.view_vertex_evals,
                });
                // The core reports in frame order and has dropped a reported frame, at the latest, once 64 newer ones
                // are reported.
                self.reported.push_back(row.frame_id);
                while self.reported.len() > REPORTED_RETAINED {
                    self.dropped = self.reported.pop_front().unwrap_or(self.dropped);
                }
            }
            if n < rows.len() {
                return Ok(reports);
            }
        }
    }

    /// RPI.22's ID pass at pixel `(x, y)` of a submitted frame.
    pub fn request_pick(&mut self, frame: u64, x: u32, y: u32) -> Result<PickTicket, Error> {
        self.core.pick_request(frame, x, y).map(PickTicket).map_err(|status| self.error(status))
    }

    /// A pick's answer. A ready reply carries its frame's binding and the declared dependencies of the frame's set.
    pub fn poll_pick(&mut self, ticket: PickTicket) -> Result<PickPoll, Error> {
        let row = self.core.pick_poll(ticket.0).map_err(|status| self.error(status))?;
        match row.state {
            PICK_PENDING => Ok(PickPoll::Pending),
            PICK_STALE => Ok(PickPoll::Stale(match row.stale {
                PICK_STALE_SURFACE => "surface_generation",
                PICK_STALE_VIEW => "view_generation",
                PICK_STALE_TARGET => "viewport",
                PICK_STALE_ENGINE => "dependencies",
                PICK_STALE_PUBLICATION => "publication_generation",
                _ => return Err(contradiction("a stale pick names no known field")),
            })),
            PICK_READY if row.instance == 0 => Ok(PickPoll::Ready(None)),
            PICK_READY => {
                let record = self.frame(row.frame_id)?;
                let stamp = &record.input.stamp;
                if (row.device_epoch, row.surface, row.surface_generation, row.view, row.view_generation)
                    != (
                        stamp.device_epoch,
                        stamp.surface,
                        stamp.surface_generation,
                        stamp.view.0,
                        stamp.view_generation,
                    )
                    || row.publication_generation != stamp.publication_generation
                {
                    return Err(contradiction("a pick names other bindings than its frame"));
                }
                let binding = PickBinding {
                    device_epoch: row.device_epoch,
                    surface: row.surface,
                    surface_generation: row.surface_generation,
                    view: ViewId(row.view),
                    view_generation: row.view_generation,
                    publication_generation: row.publication_generation,
                    camera: row.camera,
                    viewport: (row.viewport_width, row.viewport_height),
                    visibility: row.visibility,
                    interaction: row.interaction,
                    dependencies: record.dependencies.clone(),
                };
                Ok(PickPoll::Ready(Some(PickReply { binding, instance: InstanceId(u64::from(row.instance)) })))
            }
            _ => Err(contradiction("a pick row has an unknown state")),
        }
    }

    pub fn counters(&self) -> Result<Sa2CounterValues, Error> {
        self.core.counters().map_err(|status| self.error(status))
    }

    fn frame(&self, id: u64) -> Result<&FrameRecord, Error> {
        self.frames.iter().find(|f| f.id == id).ok_or_else(|| contradiction("the core names a frame never submitted"))
    }

    fn error(&self, status: Status) -> Error {
        let detail = self.core.last_error();
        let error = if status == Status::UNSUPPORTED {
            Error::unsupported("core", format!("the renderer core does not support the call: {detail}"))
        } else {
            Error::unavailable(format!("the renderer core answered {}: {detail}", status.name()))
        };
        error.with("status", status.name()).with("detail", detail)
    }
}

impl<C: CoreAbi> Backend for RenderBackend<C> {
    fn describe_backend(&self) -> Result<SupportReport, Error> {
        let text = self.core.describe().map_err(|status| self.error(status))?;
        support_report(&text, &self.limits)
    }

    /// A bundle at the revision the core already holds is adopted without a second copy; any other goes to
    /// [`RenderBackend::adopt_engine`].
    fn adopt_engine(&mut self, bundle: &EngineBundle) -> Result<RevisionRef, Error> {
        if self.held.as_ref() == Some(&bundle.revision) {
            validate_engine_bundle(bundle)?;
        } else {
            // The inherent method: a path names it before this trait method of the same name, so this is no recursion.
            RenderBackend::adopt_engine(self, bundle)?;
        }
        Ok(bundle.revision.clone())
    }

    /// [`Backend::adopt_engine`] without the bundle's second check: [`CheckedBundle::check`] made it.
    fn adopt_checked(&mut self, bundle: &CheckedBundle) -> Result<RevisionRef, Error> {
        let bundle = bundle.bundle();
        if self.held.as_ref() != Some(&bundle.revision) {
            self.adopt_valid(bundle)?;
        }
        Ok(bundle.revision.clone())
    }

    fn prepare(&mut self, request: Preparation<'_>) -> Result<Ticket, Error> {
        let rows = slot_rows(request.payload)?;
        let stamp = request.stamp;
        let input_digest = hex32(&stamp.input_digest)
            .ok_or_else(|| Error::invalid("the input digest is not a SHA-256 in hexadecimal"))?;
        let input = PreparationInput {
            command: stamp.command.0,
            view: stamp.view.0,
            view_generation: stamp.view_generation,
            publication_generation: stamp.publication_generation,
            device_epoch: stamp.device_epoch,
            base_revision: stamp.base.state,
            visibility: &rows.visibility,
            interaction: &rows.interaction,
            instances: &rows.instances,
            input_digest,
        };
        self.core.prepare(&input).map(Ticket).map_err(|status| self.error(status))
    }

    fn poll(&mut self, ticket: Ticket) -> Poll {
        let row = match self.core.poll(ticket.0) {
            Ok(row) => row,
            Err(status) => return Poll::Failed(self.error(status)),
        };
        match row.state {
            TICKET_PENDING => Poll::Pending,
            TICKET_READY => Poll::Ready(ResourceSet(row.resource_set)),
            TICKET_CANCELLED => Poll::Cancelled(match row.reason {
                CANCEL_SUPERSEDED => CancelReason::Superseded { by: CommandId(row.superseded_by) },
                CANCEL_REPLACED_REVISION => CancelReason::ReplacedRevision,
                CANCEL_BUDGET => CancelReason::Budget,
                _ => CancelReason::ByUser,
            }),
            TICKET_FAILED => Poll::Failed(row_error(Status(row.reason))),
            _ => Poll::Failed(contradiction("a ticket row has an unknown state")),
        }
    }

    fn activate(&mut self, resources: ResourceSet, stamp: &PublicationStamp) -> Result<(), Error> {
        self.core.activate(resources.0, &stamp_digest(stamp)).map_err(|status| self.error(status))?;
        self.sets.insert(
            resources.0,
            SetRecord {
                view: stamp.view,
                view_generation: stamp.view_generation,
                publication_generation: stamp.publication_generation,
                dependencies: stamp.dependencies.clone(),
            },
        );
        Ok(())
    }

    fn cancel(&mut self, ticket: Ticket, reason: &CancelReason) {
        let (code, by) = match reason {
            CancelReason::ByUser => (CANCEL_BY_USER, 0),
            CancelReason::Superseded { by } => (CANCEL_SUPERSEDED, by.0),
            CancelReason::ReplacedRevision => (CANCEL_REPLACED_REVISION, 0),
            CancelReason::Budget => (CANCEL_BUDGET, 0),
        };
        if let Err(status) = self.core.cancel(ticket.0, code, by) {
            self.fault = Some(self.error(status));
        }
    }

    fn retire(&mut self, resources: ResourceSet) {
        if self.lost_sets.remove(&resources.0) {
            return;
        }
        self.sets.remove(&resources.0);
        if let Err(status) = self.core.retire(resources.0) {
            self.fault = Some(self.error(status));
        }
    }

    fn submit_frame(&mut self, input: FrameInput) -> Result<u64, Error> {
        let s = &input.stamp;
        let camera = self.camera_matrix(s.view, s.camera)?;
        let (set, dependencies) = self.activated(s.view, s.view_generation, s.publication_generation)?;
        // ABI 8: a composite frame's panes, each from its own view's bindings; the stamp names the first.
        let mut panes = Vec::with_capacity(self.panes.len());
        for (i, pane) in self.panes.iter().enumerate() {
            let bindings = (pane.view, pane.view_generation, pane.publication_generation, pane.camera);
            if i == 0 && bindings != (s.view, s.view_generation, s.publication_generation, s.camera) {
                return Err(Error::invalid("the frame's stamp does not name the first pane's bindings"));
            }
            let (resource_set, _) = self.activated(pane.view, pane.view_generation, pane.publication_generation)?;
            panes.push(Sa2Pane {
                view: pane.view.0,
                view_generation: pane.view_generation,
                resource_set,
                x: pane.x,
                y: pane.y,
                width: pane.width,
                height: pane.height,
                camera_matrix: self.camera_matrix(pane.view, pane.camera)?,
                style: pane.style,
                flags: 0,
            });
        }
        let mut row = Sa2FrameInput {
            target: self.target,
            device_epoch: s.device_epoch,
            surface: s.surface,
            surface_generation: s.surface_generation,
            sequence: s.sequence,
            engine_revision: s.engine_revision,
            view: s.view.0,
            view_generation: s.view_generation,
            publication_generation: s.publication_generation,
            camera: s.camera,
            viewport_width: s.viewport.0,
            viewport_height: s.viewport.1,
            visibility: s.visibility,
            interaction: s.interaction,
            resource_set: set,
            subjects_digest: subjects_digest(&s.subjects),
            camera_matrix: camera,
            ..Sa2FrameInput::new()
        };
        if let Some(preview) = &input.preview {
            row.flags = FRAME_PREVIEW;
            row.preview = preview.preview.0;
            row.preview_base = preview.base;
            row.preview_literal_digest = sha256(preview.literal.as_bytes());
        }
        let submitted =
            if panes.is_empty() { self.core.frame_submit(&row) } else { self.core.frame_submit_panes(&row, &panes) };
        let id = submitted.map_err(|status| self.error(status))?;
        // B17's pick on a composite frame will keep the panes in the record to resolve the pane a point lies in.
        self.frames.push_back(FrameRecord { id, input, dependencies });
        while self.frames.len() > FRAME_RECORDS {
            self.frames.pop_front();
        }
        Ok(id)
    }

    fn read_presented(&mut self) -> Result<Vec<PresentedFrame>, Error> {
        let mut rows = vec![Sa2PresentedRow::new(); 64];
        let n = self.core.presented(&mut rows).map_err(|status| self.error(status))?;
        let mut presented = Vec::with_capacity(n);
        for row in &rows[..n] {
            let record = self.frame(row.frame_id)?;
            let stamp = &record.input.stamp;
            if (row.device_epoch, row.surface, row.sequence) != (stamp.device_epoch, stamp.surface, stamp.sequence) {
                return Err(contradiction("a presented row names other bindings than its frame"));
            }
            let shown = match (row.previews, &record.input.preview) {
                (0, _) => Presented::NoPreview,
                (1, Some(preview)) if row.preview == preview.preview.0 => Presented::Previews(vec![preview.clone()]),
                _ => return Err(contradiction("a presented row names a preview its frame did not show")),
            };
            presented.push(PresentedFrame { stamp: stamp.clone(), presented: shown });
        }
        for row in &rows[..n] {
            self.displays.push_back(DisplayTime {
                sequence: row.sequence,
                device_epoch: row.device_epoch,
                surface: row.surface,
                display_qpc: row.display_qpc,
            });
        }
        while self.displays.len() > DISPLAY_TIMES {
            self.displays.pop_front();
        }
        Ok(presented)
    }

    /// None: the product path runs no user stage, so no frame stopped one. The core's row of every frame is
    /// [`RenderBackend::read_frame_reports`]'s.
    fn read_frame_reports(&mut self) -> Result<Vec<stage::FrameReport>, Error> {
        Ok(Vec::new())
    }

    /// The ID pass on the frame that `frame` stamps. The ID pass stamps instances only; the host resolves the
    /// requested kind and level. A frame older than every frame the bridge keeps is stale at its sequence, and so is
    /// one the core certainly no longer retains (64 newer frames reported, or an earlier device epoch); a stamp the
    /// bridge never submitted is invalid, and so is a point outside the viewport the frame was submitted with.
    fn request_pick(
        &mut self,
        frame: &FrameStamp,
        point: (u32, u32),
        _requested: Requested,
    ) -> Result<host::PickTicket, Error> {
        let Some(record) = self.frames.iter().rev().find(|record| &record.input.stamp == frame) else {
            let gone = self.frames.front().is_some_and(|oldest| frame.sequence < oldest.input.stamp.sequence);
            return Err(if gone {
                Error::new(codes::STALE_PICK, "the pick is stale: the bridge no longer keeps its frame")
                    .with("field", "sequence")
            } else {
                Error::invalid("the bridge submitted no frame with this stamp")
            });
        };
        // The core would refuse the frame only with a status that also means its pick bound: a core failure.
        if record.id <= self.dropped {
            return Err(Error::new(codes::STALE_PICK, "the pick is stale: the core no longer retains its frame")
                .with("field", "sequence"));
        }
        // The viewport `submit_frame` gave the core for this frame: a point outside it is the caller's error, which
        // the core would answer only as a failure.
        let (width, height) = record.input.stamp.viewport;
        if point.0 >= width || point.1 >= height {
            return Err(Error::invalid("the point lies outside the frame's viewport"));
        }
        let id = record.id;
        RenderBackend::request_pick(self, id, point.0, point.1).map(|ticket| host::PickTicket(ticket.0))
    }

    fn poll_pick(&mut self, ticket: host::PickTicket) -> host::PickPoll {
        match RenderBackend::poll_pick(self, PickTicket(ticket.0)) {
            Ok(PickPoll::Pending) => host::PickPoll::Pending,
            Ok(PickPoll::Ready(reply)) => host::PickPoll::Stamped(reply),
            Ok(PickPoll::Stale(field)) => host::PickPoll::Refused(
                Error::new(codes::STALE_PICK, format!("the pick is stale: {field} changed")).with("field", field),
            ),
            Err(error) => host::PickPoll::Refused(error),
        }
    }

    fn recover(&mut self) -> Result<SupportReport, Error> {
        let (status, text) = self.core.recover();
        if status != Status::OK {
            let reason = json::parse(text.as_bytes())
                .ok()
                .and_then(|v| v.get("reason").and_then(Value::as_str).map(str::to_owned))
                .unwrap_or_default();
            return Err(self.error(status).with("reason", reason));
        }
        let lost = std::mem::take(&mut self.sets);
        self.lost_sets.extend(lost.into_keys());
        // The old epoch's frames are gone; frame numbers keep counting.
        self.dropped = self.frames.back().map_or(self.dropped, |newest| newest.id);
        self.reported.clear();
        self.describe_backend()
    }
}

/// The error a ticket or frame row names. It carries the status alone: the core's last error text belongs to its
/// last failing call, not to this row.
fn row_error(status: Status) -> Error {
    let error = if status == Status::UNSUPPORTED {
        Error::unsupported("core", "the renderer core does not support the work")
    } else {
        Error::unavailable(format!("the renderer core answered {}", status.name()))
    };
    error.with("status", status.name())
}

/// The per-slot rows of `sa2_preparation`.
struct SlotRows {
    visibility: Vec<u8>,
    interaction: Vec<u8>,
    instances: Vec<u32>,
}

fn slot_rows(publication: &Publication) -> Result<SlotRows, Error> {
    let mut rows = SlotRows { visibility: vec![0; SLOTS], interaction: vec![0; SLOTS], instances: vec![0; SLOTS] };
    let mut stickers = None;
    for plan in &publication.plans {
        for (index, primitive) in plan.primitives.iter().enumerate() {
            match primitive {
                Primitive::Retained(_) | Primitive::Posed(_) if stickers.is_none() => {
                    stickers = Some((plan, index as u32, primitive))
                }
                Primitive::Retained(_) | Primitive::Posed(_) => {
                    return Err(Error::unsupported("F01", "the core draws one sticker set per view"));
                }
                other => {
                    return Err(Error::unsupported(
                        other.feature(),
                        "the S-A2 core draws retained or posed stickers only",
                    ));
                }
            }
        }
    }
    let drawn = |p| stickers.is_some_and(|(plan, _, _)| std::ptr::eq(plan, p));
    if publication.plans.iter().any(|p| !p.instances.is_empty() && !drawn(p)) {
        return Err(Error::invalid("an instance names a primitive the plan does not draw"));
    }
    let Some((plan, index, primitive)) = stickers else { return Ok(rows) };
    let Primitive::Retained(input) = primitive else {
        // J02's posed row: every sticker is drawn where its piece's pose puts it, and none is pickable.
        if !plan.instances.is_empty() {
            return Err(Error::unsupported("J13", "the core's ID pass stamps no instance of a posed primitive"));
        }
        rows.visibility.fill(1);
        return Ok(rows);
    };
    let labels = input.labels.as_u32();
    if labels.len() != SLOTS || input.visibility.bytes.len() != SLOTS || input.interaction.bytes.len() != SLOTS {
        return Err(Error::invalid("a retained primitive carries one label, visibility and interaction per slot"));
    }
    rows.visibility.copy_from_slice(&input.visibility.bytes);
    rows.interaction.copy_from_slice(&input.interaction.bytes);
    let mut at = vec![u32::MAX; SLOTS];
    for (slot, &label) in labels.iter().enumerate() {
        let entry = at.get_mut(label as usize).ok_or_else(|| Error::invalid("a label is out of range"))?;
        if *entry != u32::MAX {
            return Err(Error::invalid("the labels repeat a sticker, so a sticker has no single slot"));
        }
        *entry = slot as u32;
    }
    for instance in &plan.instances {
        let id = u32::try_from(instance.instance.0)
            .ok()
            .filter(|&id| id != 0)
            .ok_or_else(|| Error::invalid("the ID pass carries instance IDs from 1 to 2^32 - 1"))?;
        if instance.primitive != index {
            return Err(Error::invalid("an instance names a primitive the plan does not draw"));
        }
        let slot = match &instance.object {
            ObjectId::Sticker(home) => at.get(*home as usize).copied().filter(|&s| s != u32::MAX),
            ObjectId::Slot(slot) => Some(*slot).filter(|&s| (s as usize) < SLOTS),
            _ => return Err(Error::unsupported("F18", "the core's ID pass stamps sticker slots only")),
        }
        .ok_or_else(|| Error::invalid("an instance names no slot of the labels"))? as usize;
        if rows.instances[slot] != 0 {
            return Err(Error::invalid("two instances share one slot of the ID pass"));
        }
        rows.instances[slot] = id;
    }
    Ok(rows)
}

/// The support report the core describes, with the bridge's declared limits on every supported row.
fn support_report(text: &str, limits: &LimitProfile) -> Result<SupportReport, Error> {
    let malformed = || contradiction("the core's support report is malformed");
    let value = json::parse(text.as_bytes()).map_err(|_| malformed())?;
    let version = value.get("interface_version").and_then(Value::as_i64).ok_or_else(malformed)?;
    if version != i64::from(ABI_VERSION) {
        return Err(Error::unavailable(format!("the core speaks interface version {version}, not {ABI_VERSION}")));
    }
    if let Some(removed) = value.get("device_removed").and_then(Value::as_str) {
        return Err(Error::unavailable("the renderer core's device is removed").with("removed", removed));
    }
    let build = value.get("backend_build").and_then(Value::as_str).ok_or_else(malformed)?;
    let epoch = value.get("device_epoch").and_then(Value::as_i64).ok_or_else(malformed)?;
    let mut features = BTreeMap::new();
    for (row, entry) in value.get("features").and_then(Value::as_object).ok_or_else(malformed)? {
        let support = match entry.get("status").and_then(Value::as_str) {
            Some("supported") => {
                let layout = entry.get("layout_version").and_then(Value::as_i64).ok_or_else(malformed)?;
                Support::Supported {
                    layout_version: u32::try_from(layout).map_err(|_| malformed())?,
                    limits: limits.clone(),
                    stages: Vec::new(),
                }
            }
            Some("unsupported") => {
                let reason = entry.get("reason").and_then(Value::as_str).unwrap_or("unsupported by the core");
                Support::Unsupported(Error::unsupported(row, reason))
            }
            _ => return Err(malformed()),
        };
        features.insert(row.clone(), support);
    }
    features.entry("F20".to_owned()).or_insert_with(|| Support::Supported {
        layout_version: 1,
        limits: limits.clone(),
        stages: Vec::new(),
    });
    let evidence = value
        .get("evidence")
        .and_then(Value::as_array)
        .ok_or_else(malformed)?
        .iter()
        .map(|e| e.as_str().map(str::to_owned).ok_or_else(malformed))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SupportReport {
        interface_version: ABI_VERSION,
        backend_build: build.to_owned(),
        device_epoch: u64::try_from(epoch).map_err(|_| malformed())?,
        features,
        evidence,
    })
}

/// The digest `sa2_activate` stores: the SHA-256 of the stamp's fields, one per line, dependencies sorted by key.
pub fn stamp_digest(stamp: &PublicationStamp) -> [u8; 32] {
    let mut text = format!(
        "command={}\nview={}\nview_generation={}\npublication_generation={}\ndevice_epoch={}\nbase={}:{}\n\
         input_digest={}\n",
        stamp.command.0,
        stamp.view.0,
        stamp.view_generation,
        stamp.publication_generation,
        stamp.device_epoch,
        stamp.base.state,
        stamp.base.digest,
        stamp.input_digest
    );
    for (key, value) in &stamp.dependencies {
        text.push_str(&format!("dependency {key}={value}\n"));
    }
    sha256(text.as_bytes())
}

/// The digest of a frame's subjects: the SHA-256 of plan, kind, digest and comparison base, one subject per line.
pub fn subjects_digest(subjects: &[PlanSubject]) -> [u8; 32] {
    let text: String = subjects
        .iter()
        .map(|s| format!("plan={} kind={:?} digest={} base={}\n", s.plan.0, s.kind, s.digest, s.comparison_base))
        .collect();
    sha256(text.as_bytes())
}

/// A SHA-256 in lowercase or uppercase hexadecimal.
fn hex32(text: &str) -> Option<[u8; 32]> {
    let bytes = text.as_bytes();
    if bytes.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, pair) in bytes.chunks(2).enumerate() {
        let digit = |b: u8| (b as char).to_digit(16);
        out[i] = (digit(pair[0])? * 16 + digit(pair[1])?) as u8;
    }
    Some(out)
}

fn contradiction(summary: &str) -> Error {
    Error::unavailable(summary.to_owned()).with("status", "contradiction")
}
