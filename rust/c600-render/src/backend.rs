//! [`RenderBackend`]: c600-display's [`Backend`] over a [`CoreAbi`], and the ABI 5 product calls the trait has no
//! method for (engine adoption, frame reports, picks, counters) as methods of their own.
//!
//! What the bridge decides, because the core's interface and the host's types differ:
//! - A publication becomes per-slot rows: the one retained primitive's visibility and interaction sets, and each
//!   instance on the slot its sticker occupies under the primitive's labels (or the slot it names). Posed plans,
//!   other primitives, a second retained primitive, two instances on one slot and instance IDs that do not fit the
//!   ID pass's u32 (or are 0) are refused; J01 and J02 are reported unsupported until posed plans are mapped.
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
//! - `cancel` and `retire` cannot return an error through the trait; a failure is kept in [`RenderBackend::fault`].
//! - Recovery (ABI 5, route B; accepted for now by the owner on 18 Nov 2026, pending three owner-machine checks)
//!   forgets every resource set of the old device epoch: the core released them, and RM.23 lets no old resource token
//!   draw on the new device. A frame naming such a set fails until its publication is prepared and activated again;
//!   retiring one is a no-op.
//! - Every core failure is `EV3:busy.renderer_unavailable` except `SA2_E_UNSUPPORTED`, which is
//!   `EV3:input.display_unsupported`; the context names the status and the core's last error.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use c600_digest::sha256;
use c600_display::error::Error;
use c600_display::host::{Backend, FrameInput, Poll, Preparation, PublicationStamp, ResourceSet, Ticket};
use c600_display::pick::{FrameStamp, PickBinding, PickReply, Presented, PresentedFrame};
use c600_display::plan::{CommandId, InstanceId, ObjectId, Primitive, Publication, ViewId};
use c600_display::receipt::CancelReason;
use c600_display::validate::{validate_engine_bundle, EngineBundle, LimitProfile, PlanSubject, Support, SupportReport};
use c600_kernel::json::{self, Value};
use c600_kernel::Puzzle;

use crate::abi::*;
use crate::api::{CoreAbi, EngineImage, PreparationInput};

/// Frame records kept for reports, presentations and picks: more than the core retains (every unreported frame up
/// to its bound of 1024, and the last 64 reported).
const FRAME_RECORDS: usize = 2048;
/// Camera matrices kept per view.
const CAMERAS: usize = 8;

/// A frame the bridge submitted.
#[derive(Clone, Debug)]
struct FrameRecord {
    id: u64,
    input: FrameInput,
    dependencies: BTreeMap<String, String>,
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
    /// The newest failure of `cancel` or `retire`, which the trait cannot return.
    pub fault: Option<Error>,
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
            fault: None,
        }
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

    /// RM.16's adoption into the core: a validated Full bundle's labels at its revision. Jumbling bundles wait for
    /// the posed mapping.
    pub fn adopt_engine(&mut self, bundle: &EngineBundle) -> Result<u64, Error> {
        validate_engine_bundle(bundle)?;
        if bundle.revision.identity.puzzle != Puzzle::Full {
            return Err(Error::unsupported("J02", "the bridge adopts 600-cell-Full labels only; posed states wait"));
        }
        let labels = bundle.arrays.iter().find(|a| a.name == "labels").expect("validated").array.as_u32();
        let digest = hex32(&bundle.revision.digest)
            .ok_or_else(|| Error::invalid("the revision digest is not a SHA-256 in hexadecimal"))?;
        let image = EngineImage { revision: bundle.revision.state, labels: &labels, pose: None, digest };
        self.core.engine_adopt(&image).map_err(|status| self.error(status))
    }

    /// The core's frame reports, in frame order, each with the stamp of its frame. A row that names other bindings
    /// than its frame fails the whole read, and the rows read before it are dropped with it: a core that
    /// contradicts its own frames is trusted for none of them.
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
        let camera = self
            .cameras
            .get(&s.view)
            .and_then(|kept| kept.iter().find(|(version, _)| *version == s.camera))
            .map(|(_, matrix)| *matrix)
            .ok_or_else(|| {
                Error::invalid(format!("view {} has no camera matrix for version {}", s.view.0, s.camera))
            })?;
        let (set, dependencies) = if s.publication_generation == 0 {
            (0, BTreeMap::new())
        } else {
            self.sets
                .iter()
                .find(|(_, r)| {
                    (r.view, r.view_generation, r.publication_generation)
                        == (s.view, s.view_generation, s.publication_generation)
                })
                .map(|(set, r)| (*set, r.dependencies.clone()))
                .ok_or_else(|| Error::unavailable("no activated resource set holds the frame's publication"))?
        };
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
        let id = self.core.frame_submit(&row).map_err(|status| self.error(status))?;
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
        Ok(presented)
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
    let mut retained = None;
    for plan in &publication.plans {
        for (index, primitive) in plan.primitives.iter().enumerate() {
            match primitive {
                Primitive::Retained(input) if retained.is_none() => retained = Some((plan, index as u32, input)),
                Primitive::Retained(_) => {
                    return Err(Error::unsupported("F01", "the core draws one retained sticker set per view"));
                }
                Primitive::Posed(_) => {
                    return Err(Error::unsupported("J02", "the bridge does not map posed plans yet"));
                }
                other => {
                    return Err(Error::unsupported(other.feature(), "the S-A2 core draws retained stickers only"));
                }
            }
        }
    }
    let drawn = |p| retained.is_some_and(|(plan, _, _)| std::ptr::eq(plan, p));
    if publication.plans.iter().any(|p| !p.instances.is_empty() && !drawn(p)) {
        return Err(Error::invalid("an instance names a primitive the plan does not draw"));
    }
    let Some((plan, index, input)) = retained else { return Ok(rows) };
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
            Some("supported") if matches!(row.as_str(), "J01" | "J02") => {
                Support::Unsupported(Error::unsupported(row, "the bridge does not map posed plans yet"))
            }
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
