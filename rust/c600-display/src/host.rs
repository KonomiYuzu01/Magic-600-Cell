//! The host's publication state machine (RM.17 to RM.26, RM.32, RM.35, RM.36; LI.7/LI.25 standalone and scheduled
//! publication) over the portable adapter operations of RM.14, with a mock backend that issues tickets and lets a
//! test lose the device before acceptance, after acceptance or after installation.
//!
//! A standalone publication is validated at admission: a bad one is `refused(Error)` and supersedes nothing. A
//! scheduled publication is accepted first and runs as a job; its generated payload is validated on the original
//! invocation, and any error is the job's `error(Error)` in form `stop`, which LI.28 maps to `failed(Error)`, with no
//! second admission and no resurrection. Accepted work owns the view's one pending generation; a newer acceptance
//! cancels it as `superseded`, and installation happens only while the command is the newest accepted one and its
//! dependencies and device epoch still match. The first serialized terminal wins; later notifications only release
//! resources (RM.22).
//!
//! The host also drives RM.14's other adapter operations: engine adoption through the backend (RM.16), frame reports
//! that show a fallback on the view (RM.41), and picks, which go request, poll, then RPI.25's resolution.

use std::collections::BTreeMap;

use crate::canonical;
use crate::error::{codes, Error, Form};
use crate::pick::{
    FrameStamp, Partial, PickBinding, PickMode, PickReply, Presented, PresentedFrame, PreviewShown, Projection,
    Requested, Resolution,
};
use crate::plan::{
    CommandId, InstanceId, JobId, ObjectId, PreviewDisplay, Publication, RevisionRef, SourceLink, ViewId,
};
use crate::receipt::{receipt_status, CancelReason, Completion, Covered, JobOutcome, Receipt, ResultRef, Status};
use crate::source::{self, Followed, HostRange};
use crate::stage::{Fallback, FrameReport};
use crate::validate::{self, validate, Context, EngineBundle, LimitProfile, SubjectKind, SupportReport, Validated};

/// A backend ticket for one preparation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ticket(pub u64);

/// A prepared resource set; it contains no device handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceSet(pub u64);

/// RM.14's `PublicationStamp`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicationStamp {
    pub command: CommandId,
    pub view: ViewId,
    pub view_generation: u64,
    pub publication_generation: u64,
    pub device_epoch: u64,
    pub base: RevisionRef,
    pub dependencies: BTreeMap<String, String>,
    pub input_digest: String,
}

/// RM.14's `Preparation`.
pub struct Preparation<'a> {
    pub stamp: &'a PublicationStamp,
    pub payload: &'a Publication,
    pub limits: &'a LimitProfile,
}

/// RM.14's `poll` result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Poll {
    Pending,
    Ready(ResourceSet),
    Cancelled(CancelReason),
    Failed(Error),
}

/// RM.24's `FrameInput`, reduced to what the host decides: the stamp and the engine preview the frame shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameInput {
    pub stamp: FrameStamp,
    pub preview: Option<PreviewShown>,
}

/// RM.14's pick ticket: the backend's handle on one pick request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PickTicket(pub u64);

/// RM.14's `poll_pick` result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PickPoll {
    Pending,
    /// RM.14's `stamped_instances`: what RPI.22's ID pass found at the point, an instance stamped with its frame's
    /// binding (RPI.24), or `None` where no interactive instance was drawn there.
    Stamped(Option<PickReply>),
    Refused(Error),
}

/// The adapter operations of RM.14 that the host drives. The route-specific ABI stays with the binding annex.
pub trait Backend {
    fn describe_backend(&self) -> Result<SupportReport, Error>;
    /// RM.16: installs the bundle's complete CPU-side revision image, independently of any program lane, preparation
    /// or recovery and also while the device is lost, and answers the revision it adopted. It never waits for program
    /// work or a GPU fence. Missing or mismatched engine inputs are refused (`refresh_required`) with
    /// `EV3:input.display_bundle_mismatch`, and nothing is adopted.
    fn adopt_engine(&mut self, bundle: &EngineBundle) -> Result<RevisionRef, Error>;
    fn prepare(&mut self, request: Preparation<'_>) -> Result<Ticket, Error>;
    fn poll(&mut self, ticket: Ticket) -> Poll;
    fn activate(&mut self, resources: ResourceSet, stamp: &PublicationStamp) -> Result<(), Error>;
    fn cancel(&mut self, ticket: Ticket, reason: &CancelReason);
    fn retire(&mut self, resources: ResourceSet);
    fn submit_frame(&mut self, input: FrameInput) -> Result<u64, Error>;
    fn read_presented(&mut self) -> Result<Vec<PresentedFrame>, Error>;
    /// RM.41: each frame that stopped a user stage since the last read, once.
    fn read_frame_reports(&mut self) -> Result<Vec<FrameReport>, Error>;
    /// RPI.22's ID pass at `point` of the frame that `frame` stamps, for the requested kind and level (RM.14). A frame
    /// the backend never submitted is refused as invalid; one it submitted but no longer keeps is refused as stale
    /// (`EV3:stale.display_pick`), so that the host drops a passive pick on it (RPI.25).
    fn request_pick(
        &mut self,
        frame: &FrameStamp,
        point: (u32, u32),
        requested: Requested,
    ) -> Result<PickTicket, Error>;
    fn poll_pick(&mut self, ticket: PickTicket) -> PickPoll;
    fn recover(&mut self) -> Result<SupportReport, Error>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum MockTicket {
    Pending,
    Ready,
    Cancelled(CancelReason),
    Failed(Error),
}

/// A pick the mock backend was asked for, and its answer so far.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MockPick {
    pub frame: FrameStamp,
    pub point: (u32, u32),
    pub requested: Requested,
    pub answer: PickPoll,
}

/// A mock backend: tickets stay pending until [`MockBackend::finish`], picks until [`MockBackend::answer_pick`] or
/// [`MockBackend::refuse_pick`], frames are confirmed unless held, and the device can be lost at any point. Adoption
/// works while the device is lost (RM.16).
#[derive(Debug)]
pub struct MockBackend {
    report: SupportReport,
    lost: bool,
    next: u64,
    tickets: BTreeMap<Ticket, MockTicket>,
    frames: Vec<FrameInput>,
    held: Vec<u64>,
    /// The stamps of the frames submitted on the current device, which a pick names.
    stamps: Vec<FrameStamp>,
    /// The dependencies each publication generation of a view was activated with, for pick bindings.
    activated: BTreeMap<(ViewId, u64), BTreeMap<String, String>>,
    /// The next `prepare` or `activate` fails with this error (an allocation failure, for example).
    pub fail_prepare: Option<Error>,
    pub fail_activate: Option<Error>,
    /// The next `adopt_engine` fails with this error (`refresh_required`, for example).
    pub fail_adopt: Option<Error>,
    /// The next `request_pick` is refused with this error (a frame the backend no longer keeps, for example).
    pub fail_pick: Option<Error>,
    /// Retired resource sets, in order.
    pub retired: Vec<ResourceSet>,
    /// Adopted revisions, in order.
    pub adoptions: Vec<RevisionRef>,
    /// RM.41's reports that the next `read_frame_reports` returns: a test queues the frames that stopped a user stage.
    pub frame_reports: Vec<FrameReport>,
    /// The picks asked for, by ticket.
    pub picks: BTreeMap<PickTicket, MockPick>,
}

impl MockBackend {
    pub fn new(report: SupportReport) -> MockBackend {
        MockBackend {
            report,
            lost: false,
            next: 1,
            tickets: BTreeMap::new(),
            frames: Vec::new(),
            held: Vec::new(),
            stamps: Vec::new(),
            activated: BTreeMap::new(),
            fail_prepare: None,
            fail_activate: None,
            fail_adopt: None,
            fail_pick: None,
            retired: Vec::new(),
            adoptions: Vec::new(),
            frame_reports: Vec::new(),
            picks: BTreeMap::new(),
        }
    }

    /// The preparation behind `ticket` finishes. A cancelled ticket stays cancelled: a late completion yields no
    /// usable resources (RM.17).
    pub fn finish(&mut self, ticket: Ticket) {
        if let Some(state @ MockTicket::Pending) = self.tickets.get_mut(&ticket) {
            *state = MockTicket::Ready;
        }
    }

    /// The device is lost: pending tickets and picks fail, frames and frame reports clear, and the backend is
    /// unavailable until `recover`.
    pub fn lose_device(&mut self) {
        self.lost = true;
        for state in self.tickets.values_mut() {
            if matches!(state, MockTicket::Pending | MockTicket::Ready) {
                *state = MockTicket::Failed(Error::unavailable("the device was lost"));
            }
        }
        for pick in self.picks.values_mut() {
            if pick.answer == PickPoll::Pending {
                pick.answer = PickPoll::Refused(Error::unavailable("the device was lost"));
            }
        }
        self.frames.clear();
        self.stamps.clear();
        self.frame_reports.clear();
    }

    /// The ID pass of the pick `ticket` found `instance`, or no interactive instance (`None`): the pick answers it
    /// stamped with its frame's binding and the dependencies its publication was activated with. A pick that has an
    /// answer keeps it.
    pub fn answer_pick(&mut self, ticket: PickTicket, instance: Option<InstanceId>) {
        let Some(pick) = self.picks.get_mut(&ticket).filter(|p| p.answer == PickPoll::Pending) else { return };
        let frame = &pick.frame;
        let binding = PickBinding {
            device_epoch: frame.device_epoch,
            surface: frame.surface,
            surface_generation: frame.surface_generation,
            view: frame.view,
            view_generation: frame.view_generation,
            publication_generation: frame.publication_generation,
            camera: frame.camera,
            viewport: frame.viewport,
            visibility: frame.visibility,
            interaction: frame.interaction,
            dependencies: self.activated.get(&(frame.view, frame.publication_generation)).cloned().unwrap_or_default(),
        };
        pick.answer = PickPoll::Stamped(instance.map(|instance| PickReply { binding, instance }));
    }

    /// The pick `ticket` is refused with `error` (a binding the backend found stale, for example). A pick that has an
    /// answer keeps it.
    pub fn refuse_pick(&mut self, ticket: PickTicket, error: Error) {
        if let Some(pick) = self.picks.get_mut(&ticket).filter(|p| p.answer == PickPoll::Pending) {
            pick.answer = PickPoll::Refused(error);
        }
    }

    /// The frame with that sequence stays unconfirmed until [`MockBackend::release`].
    pub fn hold(&mut self, sequence: u64) {
        self.held.push(sequence);
    }

    pub fn release(&mut self, sequence: u64) {
        self.held.retain(|&s| s != sequence);
    }

    pub fn is_lost(&self) -> bool {
        self.lost
    }
}

impl Backend for MockBackend {
    fn describe_backend(&self) -> Result<SupportReport, Error> {
        if self.lost {
            Err(Error::unavailable("the device is lost"))
        } else {
            Ok(self.report.clone())
        }
    }

    fn adopt_engine(&mut self, bundle: &EngineBundle) -> Result<RevisionRef, Error> {
        if let Some(error) = self.fail_adopt.take() {
            return Err(error);
        }
        self.adoptions.push(bundle.revision.clone());
        Ok(bundle.revision.clone())
    }

    fn prepare(&mut self, _request: Preparation<'_>) -> Result<Ticket, Error> {
        if self.lost {
            return Err(Error::unavailable("the device is lost"));
        }
        if let Some(error) = self.fail_prepare.take() {
            return Err(error);
        }
        let ticket = Ticket(self.next);
        self.next += 1;
        self.tickets.insert(ticket, MockTicket::Pending);
        Ok(ticket)
    }

    fn poll(&mut self, ticket: Ticket) -> Poll {
        match self.tickets.get(&ticket) {
            Some(MockTicket::Pending) | None => Poll::Pending,
            Some(MockTicket::Ready) => Poll::Ready(ResourceSet(ticket.0)),
            Some(MockTicket::Cancelled(reason)) => Poll::Cancelled(reason.clone()),
            Some(MockTicket::Failed(error)) => Poll::Failed(error.clone()),
        }
    }

    fn activate(&mut self, _resources: ResourceSet, stamp: &PublicationStamp) -> Result<(), Error> {
        if self.lost {
            return Err(Error::unavailable("the device is lost"));
        }
        if let Some(error) = self.fail_activate.take() {
            return Err(error);
        }
        self.activated.insert((stamp.view, stamp.publication_generation), stamp.dependencies.clone());
        Ok(())
    }

    fn cancel(&mut self, ticket: Ticket, reason: &CancelReason) {
        if let Some(state) = self.tickets.get_mut(&ticket) {
            if matches!(state, MockTicket::Pending | MockTicket::Ready) {
                *state = MockTicket::Cancelled(reason.clone());
            }
        }
    }

    fn retire(&mut self, resources: ResourceSet) {
        self.retired.push(resources);
    }

    fn submit_frame(&mut self, input: FrameInput) -> Result<u64, Error> {
        if self.lost {
            return Err(Error::unavailable("the device is lost"));
        }
        let sequence = input.stamp.sequence;
        self.stamps.push(input.stamp.clone());
        self.frames.push(input);
        Ok(sequence)
    }

    fn read_presented(&mut self) -> Result<Vec<PresentedFrame>, Error> {
        if self.lost {
            return Err(Error::unavailable("the device is lost"));
        }
        let (held, confirmed): (Vec<FrameInput>, Vec<FrameInput>) =
            self.frames.drain(..).partition(|f| self.held.contains(&f.stamp.sequence));
        self.frames = held;
        Ok(confirmed
            .into_iter()
            .map(|f| PresentedFrame {
                stamp: f.stamp,
                presented: f.preview.map_or(Presented::NoPreview, |p| Presented::Previews(vec![p])),
            })
            .collect())
    }

    fn read_frame_reports(&mut self) -> Result<Vec<FrameReport>, Error> {
        if self.lost {
            return Err(Error::unavailable("the device is lost"));
        }
        Ok(std::mem::take(&mut self.frame_reports))
    }

    fn request_pick(
        &mut self,
        frame: &FrameStamp,
        point: (u32, u32),
        requested: Requested,
    ) -> Result<PickTicket, Error> {
        if self.lost {
            return Err(Error::unavailable("the device is lost"));
        }
        if let Some(error) = self.fail_pick.take() {
            return Err(error);
        }
        if !self.stamps.contains(frame) {
            return Err(Error::invalid("no frame with that stamp was submitted on this device"));
        }
        let ticket = PickTicket(self.next);
        self.next += 1;
        self.picks.insert(ticket, MockPick { frame: frame.clone(), point, requested, answer: PickPoll::Pending });
        Ok(ticket)
    }

    fn poll_pick(&mut self, ticket: PickTicket) -> PickPoll {
        match self.picks.get(&ticket) {
            Some(pick) => pick.answer.clone(),
            None => PickPoll::Refused(Error::invalid("no such pick")),
        }
    }

    fn recover(&mut self) -> Result<SupportReport, Error> {
        self.lost = false;
        self.report.device_epoch += 1;
        Ok(self.report.clone())
    }
}

/// An installed publication of one view.
#[derive(Clone, Debug, PartialEq)]
pub struct Installed {
    pub command: CommandId,
    pub generation: u64,
    pub view_generation: u64,
    pub publication: Publication,
    pub validated: Validated,
    pub resources: ResourceSet,
    pub device_epoch: u64,
    /// False while the device is lost, and after a recovery until the publication has resources on the new device;
    /// installed CPU data is retained (RM.23).
    pub available: bool,
    /// RM.41: what the view shows since a frame of this publication stopped a user stage, for its visible status;
    /// `None` while no frame report has named it.
    pub fallback: Option<Fallback>,
}

/// RM.23: a retained publication on its way to resources on the device of a recovery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Rebind {
    /// The host asks the backend at the next [`Host::pump`] in which the view has no pending work.
    Due,
    /// The backend prepares it under this ticket.
    Preparing(Ticket),
}

/// A view and its surface.
#[derive(Clone, Debug, PartialEq)]
pub struct View {
    pub open: bool,
    pub generation: u64,
    pub surface: u64,
    pub surface_generation: u64,
    pub publication_generation: u64,
    pub camera: u64,
    pub viewport: (u32, u32),
    pub visibility: u64,
    pub interaction: u64,
    newest: Option<CommandId>,
    pending: Option<CommandId>,
    installed: Option<Installed>,
    preview: Option<PreviewDisplay>,
    rebind: Option<Rebind>,
}

/// A pick the host asked the backend for: how to resolve its answer, and the device epoch it was asked on.
#[derive(Clone, Copy, Debug)]
struct PickRequest {
    mode: PickMode,
    requested: Requested,
    epoch: u64,
}

/// A pick as [`Host::poll_pick`] sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Picked {
    /// The backend has not answered yet.
    Pending,
    /// The pick is over: RPI.25's resolution of the stamped instance, or `None` where no interactive instance was
    /// drawn at the point or a passive pick went stale.
    Done(Option<Partial<Resolution>>),
}

#[derive(Clone, Debug)]
struct Command {
    request: String,
    view: ViewId,
    job: Option<JobId>,
    status: Status,
    outcome: Option<JobOutcome>,
    ticket: Option<Ticket>,
    stamp: Option<PublicationStamp>,
    publication: Option<Publication>,
    validated: Option<Validated>,
    allowance: Option<u64>,
}

/// The host.
pub struct Host<B: Backend> {
    pub backend: B,
    pub ctx: Context,
    views: BTreeMap<ViewId, View>,
    commands: BTreeMap<CommandId, Command>,
    events: Vec<(CommandId, Status)>,
    current: BTreeMap<String, String>,
    adopted: RevisionRef,
    epoch: u64,
    lost: bool,
    next_job: u64,
    next_surface: u64,
    sequence: u64,
    latch: Option<(u64, u64, Presented)>,
    picks: BTreeMap<PickTicket, PickRequest>,
}

impl<B: Backend> Host<B> {
    /// A host whose engine image has adopted `adopted`.
    pub fn new(backend: B, ctx: Context, adopted: RevisionRef) -> Host<B> {
        let epoch = ctx.support.device_epoch;
        let current = [("state".to_owned(), adopted.state.to_string())].into_iter().collect();
        Host {
            backend,
            ctx,
            views: BTreeMap::new(),
            commands: BTreeMap::new(),
            events: Vec::new(),
            current,
            adopted,
            epoch,
            lost: false,
            next_job: 1,
            next_surface: 1,
            sequence: 0,
            latch: None,
            picks: BTreeMap::new(),
        }
    }

    // Views and their inputs.

    /// Opens a view; an open view's ID is in use.
    pub fn open_view(&mut self, id: ViewId) -> Result<u64, Error> {
        let surface = self.next_surface;
        let view = self.views.entry(id).or_insert(View {
            open: false,
            generation: 0,
            surface,
            surface_generation: 0,
            publication_generation: 0,
            camera: 0,
            viewport: (800, 600),
            visibility: 0,
            interaction: 0,
            newest: None,
            pending: None,
            installed: None,
            preview: None,
            rebind: None,
        });
        if view.open {
            return Err(Error::new(codes::VIEW_ID_IN_USE, format!("view {} is open", id.0)));
        }
        if view.surface == surface {
            self.next_surface += 1;
        }
        view.open = true;
        view.generation += 1;
        view.surface_generation += 1;
        Ok(view.generation)
    }

    /// Closes a view: its pending publication ends `cancelled(by_user)`, and a retained publication waiting for
    /// resources on a recovered device (RM.23) waits no longer; identified old data is retained (RM.25).
    pub fn close_view(&mut self, id: ViewId) {
        let pending = self.views.get(&id).and_then(|v| v.pending);
        if let Some(command) = pending {
            self.cancel_with(command, CancelReason::ByUser);
        }
        if let Some(view) = self.views.get_mut(&id) {
            view.open = false;
            view.preview = None;
            if let Some(Rebind::Preparing(ticket)) = view.rebind.take() {
                self.backend.cancel(ticket, &CancelReason::ByUser);
            }
        }
    }

    pub fn view(&self, id: ViewId) -> Option<&View> {
        self.views.get(&id)
    }

    pub fn installed(&self, id: ViewId) -> Option<&Installed> {
        self.views.get(&id).and_then(|v| v.installed.as_ref())
    }

    pub fn set_camera(&mut self, id: ViewId) {
        self.with_view(id, |v| v.camera += 1);
    }

    pub fn set_viewport(&mut self, id: ViewId, size: (u32, u32)) {
        self.with_view(id, |v| v.viewport = size);
    }

    pub fn set_visibility(&mut self, id: ViewId) {
        self.with_view(id, |v| v.visibility += 1);
    }

    pub fn set_interaction(&mut self, id: ViewId) {
        self.with_view(id, |v| v.interaction += 1);
    }

    fn with_view(&mut self, id: ViewId, f: impl FnOnce(&mut View)) {
        if let Some(view) = self.views.get_mut(&id) {
            f(view);
        }
    }

    // Engine adoption and dependencies.

    /// RM.16: adopts a complete engine bundle independently of any program lane, even while the device is lost. The
    /// host validates the bundle and hands it to the backend, which installs its CPU-side image; the host's revision
    /// moves to the revision the backend adopted. A bundle the host or the backend refuses changes nothing.
    pub fn adopt(&mut self, bundle: &EngineBundle) -> Result<RevisionRef, Error> {
        validate::validate_engine_bundle(bundle)?;
        let adopted = self.backend.adopt_engine(bundle)?;
        self.set_dependency("state", &adopted.state.to_string());
        self.adopted = adopted;
        Ok(self.adopted.clone())
    }

    pub fn adopted(&self) -> &RevisionRef {
        &self.adopted
    }

    /// A dependency changes: accepted work that read the old value ends `cancelled(replaced_revision)` (RM.22);
    /// installed output that read it becomes stale (RPI.13) and keeps its receipt, and is no longer prepared again
    /// for a recovered device (RM.23).
    pub fn set_dependency(&mut self, key: &str, value: &str) {
        self.current.insert(key.to_owned(), value.to_owned());
        let moved: Vec<CommandId> = self
            .commands
            .iter()
            .filter(|(_, c)| !c.status.is_terminal())
            .filter(|(_, c)| c.stamp.as_ref().is_some_and(|s| s.dependencies.get(key).is_some_and(|v| v != value)))
            .map(|(id, _)| *id)
            .collect();
        for command in moved {
            self.cancel_with(command, CancelReason::ReplacedRevision);
        }
        for view in self.views.values_mut() {
            let read =
                view.installed.as_ref().is_some_and(|i| i.validated.dependencies.get(key).is_some_and(|v| v != value));
            if let Some(Rebind::Preparing(ticket)) = view.rebind.take_if(|_| read) {
                self.backend.cancel(ticket, &CancelReason::ReplacedRevision);
            }
        }
    }

    fn moved(&self, dependencies: &BTreeMap<String, String>) -> Option<String> {
        dependencies.iter().find(|(k, v)| self.current.get(*k).is_some_and(|c| c != *v)).map(|(k, _)| k.clone())
    }

    /// Whether the view's installed output depends on a value that has since changed (RPI.13).
    pub fn stale(&self, id: ViewId) -> bool {
        self.installed(id).is_some_and(|i| self.moved(&i.validated.dependencies).is_some())
    }

    // Receipts.

    pub fn receipt(&self, command: CommandId) -> Option<Receipt> {
        self.commands.get(&command).map(|c| Receipt {
            command,
            request: c.request.clone(),
            job: c.job,
            status: c.status.clone(),
        })
    }

    /// KN5.23's lookup: the same terminal for the same command and request, while it is retained.
    pub fn outcome(&self, command: CommandId, request: &str) -> Option<Receipt> {
        self.receipt(command).filter(|r| r.request == request)
    }

    /// The job outcome of a scheduled publication (LI.28).
    pub fn job_outcome(&self, command: CommandId) -> Option<&JobOutcome> {
        self.commands.get(&command).and_then(|c| c.outcome.as_ref())
    }

    pub fn ticket(&self, command: CommandId) -> Option<Ticket> {
        self.commands.get(&command).and_then(|c| c.ticket)
    }

    /// The ticket under which the backend prepares the view's retained publication again after a recovery (RM.23).
    pub fn rebinding(&self, id: ViewId) -> Option<Ticket> {
        match self.views.get(&id)?.rebind? {
            Rebind::Preparing(ticket) => Some(ticket),
            Rebind::Due => None,
        }
    }

    /// Every terminal transition, in serialized order.
    pub fn events(&self) -> &[(CommandId, Status)] {
        &self.events
    }

    /// Sets a terminal status unless one is already set. Returns whether it was set.
    fn finish(&mut self, command: CommandId, status: Status) -> bool {
        let Some(entry) = self.commands.get_mut(&command) else { return false };
        if entry.status.is_terminal() {
            return false;
        }
        entry.status = status.clone();
        let view = entry.view;
        self.events.push((command, status));
        if let Some(v) = self.views.get_mut(&view) {
            if v.pending == Some(command) {
                v.pending = None;
            }
        }
        true
    }

    /// Ends accepted work with a job outcome mapped by LI.28, or with the status itself when there is no job.
    fn end(&mut self, command: CommandId, outcome: JobOutcome) {
        let Some(entry) = self.commands.get_mut(&command) else { return };
        if entry.status.is_terminal() {
            return;
        }
        if entry.job.is_some() {
            entry.outcome = Some(outcome.clone());
        }
        self.finish(command, receipt_status(&outcome));
    }

    /// Accepted work fails: the error takes the contextual `stop` form (RM.22, H15.9).
    fn fail(&mut self, command: CommandId, error: Error) {
        self.end(command, JobOutcome::Error(error.in_form(Form::Stop)));
    }

    fn cancel_with(&mut self, command: CommandId, reason: CancelReason) {
        let Some(entry) = self.commands.get(&command) else { return };
        if entry.status.is_terminal() {
            return;
        }
        if let Some(ticket) = entry.ticket {
            self.backend.cancel(ticket, &reason);
        }
        self.end(command, JobOutcome::Cancelled(reason));
    }

    /// The solver cancels accepted work: `cancelled(by_user)`.
    pub fn cancel(&mut self, command: CommandId) -> Option<Receipt> {
        self.cancel_with(command, CancelReason::ByUser);
        self.receipt(command)
    }

    fn duplicate(&self, command: CommandId, request: &str) -> Option<Receipt> {
        let existing = self.receipt(command)?;
        if existing.request == request {
            return Some(existing);
        }
        let error = Error::new(codes::COMMAND_ID_REUSED, "the command ID was used with other content");
        Some(Receipt { command, request: request.to_owned(), job: None, status: Status::Refused(error) })
    }

    fn record(&mut self, command: CommandId, request: String, view: ViewId, job: Option<JobId>, status: Status) {
        let terminal = status.is_terminal();
        self.commands.insert(
            command,
            Command {
                request,
                view,
                job,
                status: status.clone(),
                outcome: None,
                ticket: None,
                stamp: None,
                publication: None,
                validated: None,
                allowance: None,
            },
        );
        if terminal {
            self.events.push((command, status));
        }
    }

    /// Acceptance: the view's pending generation passes to `command`, and pending older work is superseded.
    fn accept(&mut self, command: CommandId, request: String, view: ViewId, job: Option<JobId>) {
        if let Some(older) = self.views.get(&view).and_then(|v| v.pending) {
            self.cancel_with(older, CancelReason::Superseded { by: command });
        }
        let status = job.map_or(Status::Accepted, Status::Running);
        self.record(command, request, view, job, status);
        if let Some(v) = self.views.get_mut(&view) {
            v.pending = Some(command);
            v.newest = Some(command);
        }
    }

    fn open_target(&self, view: ViewId) -> Result<(), Error> {
        match self.views.get(&view) {
            Some(v) if v.open => Ok(()),
            _ => Err(Error::invalid(format!("the target view {} is not open", view.0))),
        }
    }

    // Publication.

    /// A standalone `publish_view`: validated at admission; a bad request is refused and supersedes nothing.
    pub fn publish(&mut self, command: CommandId, publication: Publication) -> Receipt {
        let request = canonical::request_digest(&publication);
        if let Some(receipt) = self.duplicate(command, &request) {
            return receipt;
        }
        let view = publication.target;
        let admitted = self.admit(&publication);
        let validated = match admitted {
            Ok(validated) => validated,
            Err(error) => {
                self.record(command, request, view, None, Status::Refused(error.in_form(Form::Refusal)));
                return self.receipt(command).expect("recorded");
            }
        };
        self.accept(command, request, view, None);
        self.prepare(command, publication, validated);
        self.receipt(command).expect("recorded")
    }

    fn admit(&self, publication: &Publication) -> Result<Validated, Error> {
        self.open_target(publication.target)?;
        if self.lost || self.backend.describe_backend().is_err() {
            return Err(Error::unavailable("the renderer is unavailable"));
        }
        let validated = validate(publication, &self.ctx)?;
        if let Some(key) = self.moved(&validated.dependencies) {
            return Err(Error::new(codes::STALE_PLAN, format!("the publication read an old {key}")));
        }
        Ok(validated)
    }

    /// A scheduled view: the job is accepted first, with the run's remaining output allowance in canonical bytes.
    pub fn schedule(&mut self, command: CommandId, view: ViewId, allowance: u64) -> Receipt {
        let request = c600_digest::sha256_hex(format!("schedule:{}:{allowance}", view.0).as_bytes());
        if let Some(receipt) = self.duplicate(command, &request) {
            return receipt;
        }
        if let Err(error) = self.open_target(view) {
            self.record(command, request, view, None, Status::Refused(error));
            return self.receipt(command).expect("recorded");
        }
        let job = JobId(self.next_job);
        self.next_job += 1;
        self.accept(command, request, view, Some(job));
        if let Some(entry) = self.commands.get_mut(&command) {
            entry.allowance = Some(allowance);
        }
        self.receipt(command).expect("recorded")
    }

    /// The scheduled job delivers its generated payload: final validation on the original invocation (RM.20). An
    /// invocation already fenced keeps its cancellation.
    pub fn deliver(&mut self, command: CommandId, mut publication: Publication) -> Option<Receipt> {
        let entry = self.commands.get(&command)?;
        if entry.status.is_terminal() || entry.job.is_none() {
            return self.receipt(command);
        }
        publication.target = entry.view;
        let allowance = entry.allowance.unwrap_or(0);
        if self.lost || self.backend.describe_backend().is_err() {
            self.fail(command, Error::unavailable("the renderer is unavailable"));
            return self.receipt(command);
        }
        let validated = match validate(&publication, &self.ctx) {
            Ok(validated) => validated,
            Err(error) => {
                self.fail(command, error);
                return self.receipt(command);
            }
        };
        if self.moved(&validated.dependencies).is_some() {
            self.cancel_with(command, CancelReason::ReplacedRevision);
            return self.receipt(command);
        }
        if validated.output_bytes > allowance {
            let error = Error::new(codes::OUTPUT, "the run's output allowance cannot hold this publication")
                .with("requested", validated.output_bytes)
                .with("remaining", allowance)
                .in_form(Form::Stop);
            self.end(command, JobOutcome::CutOff(error));
            return self.receipt(command);
        }
        if let Some(entry) = self.commands.get_mut(&command) {
            entry.allowance = Some(allowance - validated.output_bytes);
        }
        self.prepare(command, publication, validated);
        self.receipt(command)
    }

    fn prepare(&mut self, command: CommandId, publication: Publication, validated: Validated) {
        let Some(entry) = self.commands.get(&command) else { return };
        let Some(view) = self.views.get(&entry.view) else { return };
        let stamp = PublicationStamp {
            command,
            view: entry.view,
            view_generation: view.generation,
            publication_generation: view.publication_generation + 1,
            device_epoch: self.epoch,
            base: publication.base.clone(),
            dependencies: validated.dependencies.clone(),
            input_digest: validated.digest.clone(),
        };
        let limits = self.ctx.limits.clone();
        let prepared = self.backend.prepare(Preparation { stamp: &stamp, payload: &publication, limits: &limits });
        let entry = self.commands.get_mut(&command).expect("present");
        entry.stamp = Some(stamp);
        entry.publication = Some(publication);
        entry.validated = Some(validated);
        match prepared {
            Ok(ticket) => entry.ticket = Some(ticket),
            Err(error) => self.fail(command, error),
        }
    }

    /// Polls the backend: notices device loss, installs ready preparations and ends failed or cancelled ones; then
    /// moves retained publications on to resources on a recovered device (RM.23).
    pub fn pump(&mut self) {
        if self.backend.describe_backend().is_err() && !self.lost {
            self.device_lost();
        }
        let active: Vec<(CommandId, Ticket)> = self
            .commands
            .iter()
            .filter(|(_, c)| !c.status.is_terminal())
            .filter_map(|(id, c)| c.ticket.map(|t| (*id, t)))
            .collect();
        for (command, ticket) in active {
            match self.backend.poll(ticket) {
                Poll::Pending => {}
                Poll::Ready(resources) => self.install(command, resources),
                Poll::Cancelled(reason) => self.end(command, JobOutcome::Cancelled(reason)),
                Poll::Failed(error) => self.fail(command, error),
            }
        }
        self.rebind();
    }

    /// RM.20's atomic installation: a bounded compare-and-swap while the command is the newest accepted one and its
    /// dependencies, view generation and device epoch still match.
    fn install(&mut self, command: CommandId, resources: ResourceSet) {
        let entry = self.commands.get(&command).expect("present").clone();
        let (Some(stamp), Some(publication), Some(validated)) = (entry.stamp, entry.publication, entry.validated)
        else {
            return;
        };
        let view = self.views.get(&entry.view).expect("view").clone();
        if view.newest != Some(command) {
            self.backend.retire(resources);
            let by = view.newest.unwrap_or(command);
            self.end(command, JobOutcome::Cancelled(CancelReason::Superseded { by }));
            return;
        }
        if self.moved(&stamp.dependencies).is_some() {
            self.backend.retire(resources);
            self.end(command, JobOutcome::Cancelled(CancelReason::ReplacedRevision));
            return;
        }
        if view.generation != stamp.view_generation || !view.open {
            self.backend.retire(resources);
            self.end(command, JobOutcome::Cancelled(CancelReason::ByUser));
            return;
        }
        if self.lost || stamp.device_epoch != self.epoch {
            self.backend.retire(resources);
            self.fail(command, Error::unavailable("the device epoch changed before installation"));
            return;
        }
        if let Err(error) = self.backend.activate(resources, &stamp) {
            self.backend.retire(resources);
            self.fail(command, error);
            return;
        }
        let generation = view.publication_generation + 1;
        let completion = Completion {
            revision: publication.base.clone(),
            result: ResultRef {
                job: entry.job,
                digest: validated.digest.clone(),
                covered: Covered {
                    view: entry.view,
                    view_generation: view.generation,
                    publication_generation: generation,
                },
            },
        };
        let installed = Installed {
            command,
            generation,
            view_generation: view.generation,
            publication,
            validated,
            resources,
            device_epoch: self.epoch,
            available: true,
            fallback: None,
        };
        let v = self.views.get_mut(&entry.view).expect("view");
        v.publication_generation = generation;
        let rebind = v.rebind.take();
        let old = v.installed.replace(installed);
        if let Some(old) = old {
            self.backend.retire(old.resources);
        }
        if let Some(Rebind::Preparing(ticket)) = rebind {
            self.backend.cancel(ticket, &CancelReason::Superseded { by: command });
        }
        self.end(command, JobOutcome::Completed(Box::new(completion)));
    }

    // Device loss and recovery.

    /// RM.23: the device epoch is invalid; pending work fails with `EV3:busy.renderer_unavailable` unless already
    /// terminal; installed data is retained but unavailable; picks, reports and the presented-preview latch clear,
    /// and a preparation of retained data for an earlier recovery ends with its epoch.
    pub fn device_lost(&mut self) {
        self.lost = true;
        self.latch = None;
        let pending: Vec<CommandId> =
            self.commands.iter().filter(|(_, c)| !c.status.is_terminal()).map(|(id, _)| *id).collect();
        for command in pending {
            self.fail(command, Error::unavailable("the device was lost before installation"));
        }
        for view in self.views.values_mut() {
            view.rebind = None;
            if let Some(installed) = view.installed.as_mut() {
                installed.available = false;
            }
        }
    }

    /// RM.23's recovery: a fresh epoch and fresh surfaces. Old resource tokens cannot draw on the new device, so every
    /// retained publication stays unavailable until it has new resources. One that is shown in its open view,
    /// revalidates and is not stale is due to be prepared again under its own command and generations; [`Host::pump`]
    /// asks the backend and activates the result. A stale or invalid one waits for a new publication. No receipt
    /// changes and nothing is retried.
    pub fn recover(&mut self) -> Result<(), Error> {
        let report = self.backend.recover()?;
        self.epoch = report.device_epoch;
        self.ctx.support = report;
        self.lost = false;
        let ctx = self.ctx.clone();
        let current = self.current.clone();
        for view in self.views.values_mut() {
            view.surface_generation += 1;
            view.rebind = None;
            if let Some(installed) = view.installed.as_mut() {
                installed.available = false;
                let shown = view.open && installed.view_generation == view.generation;
                let valid = validate(&installed.publication, &ctx).is_ok();
                let fresh = installed.validated.dependencies.iter().all(|(k, v)| current.get(k).is_none_or(|c| c == v));
                if shown && valid && fresh {
                    view.rebind = Some(Rebind::Due);
                }
            }
        }
        Ok(())
    }

    /// RM.23: prepares each due retained publication on the new device once its view has no pending work (a newer
    /// publication on its way replaces it when it installs), and activates a finished preparation under the
    /// publication's own command and generations on the current epoch; the old resources are retired once the new
    /// ones are active. A refused, failed or cancelled preparation, or one whose dependencies moved, leaves the
    /// publication unavailable until a new one replaces it. No receipt changes.
    fn rebind(&mut self) {
        if self.lost {
            return;
        }
        let views: Vec<ViewId> = self.views.iter().filter(|(_, v)| v.rebind.is_some()).map(|(id, _)| *id).collect();
        for id in views {
            let view = &self.views[&id];
            let (Some(rebind), Some(installed)) = (view.rebind, view.installed.as_ref()) else { continue };
            let stamp = PublicationStamp {
                command: installed.command,
                view: id,
                view_generation: installed.view_generation,
                publication_generation: installed.generation,
                device_epoch: self.epoch,
                base: installed.publication.base.clone(),
                dependencies: installed.validated.dependencies.clone(),
                input_digest: installed.validated.digest.clone(),
            };
            let (next, active) = match rebind {
                Rebind::Due if view.pending.is_some() => continue,
                Rebind::Due => {
                    let limits = self.ctx.limits.clone();
                    let preparation = Preparation { stamp: &stamp, payload: &installed.publication, limits: &limits };
                    (self.backend.prepare(preparation).ok().map(Rebind::Preparing), None)
                }
                Rebind::Preparing(ticket) => match self.backend.poll(ticket) {
                    Poll::Pending => continue,
                    Poll::Ready(resources) => {
                        if self.moved(&stamp.dependencies).is_none() && self.backend.activate(resources, &stamp).is_ok()
                        {
                            (None, Some(resources))
                        } else {
                            self.backend.retire(resources);
                            (None, None)
                        }
                    }
                    Poll::Cancelled(_) | Poll::Failed(_) => (None, None),
                },
            };
            let epoch = self.epoch;
            let view = self.views.get_mut(&id).expect("listed");
            view.rebind = next;
            if let (Some(resources), Some(installed)) = (active, view.installed.as_mut()) {
                let old = std::mem::replace(&mut installed.resources, resources);
                installed.available = true;
                installed.device_epoch = epoch;
                self.backend.retire(old);
            }
        }
    }

    // Frames and presentation.

    /// Shows an engine preview in a view (RM.9); the host takes it from the engine, never from a program.
    pub fn show_preview(&mut self, id: ViewId, preview: Option<PreviewDisplay>) -> Result<(), Error> {
        if let Some(preview) = &preview {
            validate::validate_preview(preview, &self.ctx)?;
        }
        self.with_view(id, |v| v.preview = preview);
        Ok(())
    }

    /// Builds the view's frame stamp and submits the frame; returns the stamp (RM.24). While the view's publication
    /// has no resources on the current device (RM.23), the frame is refused and takes no sequence number.
    pub fn frame(&mut self, id: ViewId) -> Result<FrameStamp, Error> {
        if self.lost {
            return Err(Error::unavailable("the device is lost"));
        }
        let view = self.views.get(&id).ok_or_else(|| Error::invalid("no such view"))?;
        let installed = view.installed.as_ref().filter(|i| i.view_generation == view.generation);
        if installed.is_some_and(|i| !i.available) {
            return Err(Error::unavailable("the view's publication has no resources on this device yet"));
        }
        self.sequence += 1;
        let stamp = FrameStamp {
            device_epoch: self.epoch,
            surface: view.surface,
            surface_generation: view.surface_generation,
            sequence: self.sequence,
            engine_revision: self.adopted.state,
            view: id,
            view_generation: view.generation,
            publication_generation: installed.map_or(0, |i| i.generation),
            subjects: installed.map_or_else(Vec::new, |i| i.validated.subjects.clone()),
            camera: view.camera,
            viewport: view.viewport,
            visibility: view.visibility,
            interaction: view.interaction,
        };
        let preview = view.preview.as_ref().map(|p| PreviewShown {
            preview: p.preview,
            literal: p.literal.clone(),
            base: p.base.state,
        });
        self.backend.submit_frame(FrameInput { stamp: stamp.clone(), preview })?;
        Ok(stamp)
    }

    /// Reads confirmed presentations. Within one surface and device epoch, only a newer confirmed frame replaces the
    /// latch, and a `no_preview` report clears it (RM.26). Unavailable confirmation clears it too.
    pub fn read_presented(&mut self) -> Vec<PresentedFrame> {
        let frames = match self.backend.read_presented() {
            Ok(frames) => frames,
            Err(_) => {
                self.latch = None;
                return Vec::new();
            }
        };
        for frame in &frames {
            let stamp = &frame.stamp;
            if stamp.device_epoch != self.epoch {
                continue;
            }
            let newer =
                self.latch.as_ref().is_none_or(|(epoch, seq, _)| *epoch != stamp.device_epoch || *seq < stamp.sequence);
            if newer {
                self.latch = Some((stamp.device_epoch, stamp.sequence, frame.presented.clone()));
            }
        }
        frames
    }

    /// The preview ONE.41 may name: the one in the newest confirmed presentation, if any.
    pub fn presented_preview(&self) -> Option<PreviewShown> {
        match &self.latch {
            Some((_, _, Presented::Previews(previews))) => previews.first().cloned(),
            _ => None,
        }
    }

    /// RM.41's frame reports, read from the backend: each frame that stopped a user stage, once. A report of the
    /// current device epoch whose publication is still installed in its view marks that publication's fallback,
    /// which the view's visible status shows. No receipt or job outcome changes (RM.22). Reports of an earlier device
    /// epoch are dropped (RM.23).
    pub fn read_frame_reports(&mut self) -> Result<Vec<FrameReport>, Error> {
        let mut reports = self.backend.read_frame_reports()?;
        let epoch = self.epoch;
        reports.retain(|report| report.stamp.device_epoch == epoch);
        for report in &reports {
            let installed = self.views.get_mut(&report.view).and_then(|v| v.installed.as_mut());
            if let Some(installed) = installed.filter(|i| {
                i.generation == report.publication_generation && i.view_generation == report.stamp.view_generation
            }) {
                installed.fallback = Some(report.fallback);
            }
        }
        Ok(reports)
    }

    // Picking (RPI.22 to RPI.25, RM.10, RM.24).

    /// The view's current pick binding, or why there is none.
    pub fn pick_binding(&self, id: ViewId) -> Result<PickBinding, &'static str> {
        if self.lost {
            return Err("device_epoch");
        }
        let view = self.views.get(&id).filter(|v| v.open).ok_or("view_generation")?;
        let installed = view.installed.as_ref().ok_or("publication_generation")?;
        if installed.view_generation != view.generation {
            return Err("view_generation");
        }
        if !installed.available {
            return Err("device_epoch");
        }
        Ok(PickBinding {
            device_epoch: self.epoch,
            surface: view.surface,
            surface_generation: view.surface_generation,
            view: id,
            view_generation: view.generation,
            publication_generation: installed.generation,
            camera: view.camera,
            viewport: view.viewport,
            visibility: view.visibility,
            interaction: view.interaction,
            dependencies: installed.validated.dependencies.clone(),
        })
    }

    /// RPI.25's `resolve_pick`. Bindings are checked first: a stale explicit request returns
    /// `EV3:stale.display_pick` naming the changed field, and a stale passive reply is dropped (`Ok(None)`). A current
    /// reply resolves through the installed map only; an off-lattice slot request is a partial answer, not a stale one.
    pub fn resolve_pick(
        &self,
        reply: &PickReply,
        mode: PickMode,
        requested: Requested,
        projection: &dyn Projection,
    ) -> Result<Option<Partial<Resolution>>, Error> {
        let stale = match self.pick_binding(reply.binding.view) {
            Err(field) => Some(field.to_owned()),
            Ok(current) => reply
                .binding
                .differs(&current)
                .map(str::to_owned)
                .or_else(|| self.moved(&reply.binding.dependencies).map(|k| format!("dependency {k}"))),
        };
        if let Some(field) = stale {
            return stale_pick(mode, field);
        }
        let installed = self.installed(reply.binding.view).expect("binding checked");
        let found = installed.publication.plans.iter().find_map(|plan| {
            let instance = plan.instances.iter().find(|i| i.instance == reply.instance)?;
            let map = installed.publication.maps.iter().find(|m| m.map == plan.source_map)?;
            Some((plan, instance, map.entries.get(&reply.instance)?))
        });
        let Some((plan, instance, entry)) = found else {
            return Err(Error::invalid(format!("instance {} is not in the installed map", reply.instance.0)));
        };
        if !instance.interaction {
            return Ok(None);
        }
        let resolution = |target: ObjectId| Resolution {
            target,
            occurrence: entry.object.clone(),
            instance: reply.instance,
            plan: plan.plan,
            subject: plan.subject.clone(),
            lineage: plan.derivation.clone(),
            source: entry.source.clone(),
        };
        let object = &entry.object;
        let piece = match object {
            ObjectId::Piece(p) => Some(*p),
            ObjectId::Point { witness, .. } => Some(*witness),
            ObjectId::Sticker(_) => instance.piece,
            _ => None,
        };
        let none = |what: &str| Partial::Undefined(format!("the occurrence has no explicit {what} target"));
        Ok(Some(match requested {
            Requested::Occurrence => Partial::Total(resolution(object.clone())),
            Requested::Sticker => match object {
                ObjectId::Sticker(_) => Partial::Total(resolution(object.clone())),
                _ => none("sticker"),
            },
            Requested::Piece => match piece {
                Some(p) => Partial::Total(resolution(ObjectId::Piece(p))),
                None => Partial::Undefined("no declared piece association; nothing is inferred".to_owned()),
            },
            Requested::Slot => {
                let at = match (object, piece) {
                    (ObjectId::Sticker(_) | ObjectId::Slot(_), _) => object.clone(),
                    (_, Some(p)) => ObjectId::Piece(p),
                    _ => object.clone(),
                };
                match projection.slot(&plan.subject, &at) {
                    Partial::Total(slot) => Partial::Total(resolution(ObjectId::Slot(slot))),
                    Partial::Partial { missing, .. } => Partial::Partial { covered: resolution(at), missing },
                    Partial::Undefined(reason) => {
                        Partial::Partial { covered: resolution(at), missing: vec![format!("slot: {reason}")] }
                    }
                }
            }
            Requested::Cell => match object {
                ObjectId::Cell(_) => Partial::Total(resolution(object.clone())),
                _ => none("cell"),
            },
            Requested::Grip => match object {
                ObjectId::Grip(_) => Partial::Total(resolution(object.clone())),
                ObjectId::Point { grip, .. } => Partial::Total(resolution(ObjectId::Grip(*grip))),
                _ => none("grip"),
            },
            Requested::Cap => match object {
                ObjectId::Cap(_) => Partial::Total(resolution(object.clone())),
                _ => none("cap"),
            },
            Requested::Orbit => match object {
                ObjectId::Orbit(_) => Partial::Total(resolution(object.clone())),
                _ => none("orbit"),
            },
        }))
    }

    /// RM.14's `request_pick`: asks the backend for RPI.22's ID pass at `point` of the frame that `frame` stamps, for
    /// the requested kind and level. `mode` says how a stale pick is treated (RPI.25), here and in
    /// [`Host::poll_pick`]: a frame of an earlier device epoch, or one the backend refuses as stale, makes an explicit
    /// pick return `EV3:stale.display_pick` and drops a passive one, which gets no ticket. While the device is lost
    /// there is no pick.
    pub fn request_pick(
        &mut self,
        frame: &FrameStamp,
        point: (u32, u32),
        mode: PickMode,
        requested: Requested,
    ) -> Result<Option<PickTicket>, Error> {
        if self.lost {
            return Err(Error::unavailable("the device is lost"));
        }
        if frame.device_epoch != self.epoch {
            stale_pick(mode, "device_epoch".to_owned())?;
            return Ok(None);
        }
        match self.backend.request_pick(frame, point, requested) {
            Ok(ticket) => {
                self.picks.insert(ticket, PickRequest { mode, requested, epoch: self.epoch });
                Ok(Some(ticket))
            }
            Err(error) if error.code == codes::STALE_PICK && mode == PickMode::Passive => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// RM.14's `poll_pick`, then [`Host::resolve_pick`] of a stamped instance: [`Picked::Pending`] until the backend
    /// answers. A pick asked before a device loss is stale at `device_epoch`: an explicit pick returns
    /// `EV3:stale.display_pick` naming it and a passive one is dropped. A refusal is returned, except that a passive
    /// pick the backend found stale (`EV3:stale.display_pick`) is dropped. The host forgets a pick once it is over, so
    /// polling it again is an error.
    pub fn poll_pick(&mut self, ticket: PickTicket, projection: &dyn Projection) -> Result<Picked, Error> {
        let request = *self.picks.get(&ticket).ok_or_else(|| Error::invalid("no such pick"))?;
        if self.lost || request.epoch != self.epoch {
            self.picks.remove(&ticket);
            return stale_pick(request.mode, "device_epoch".to_owned()).map(Picked::Done);
        }
        let answer = self.backend.poll_pick(ticket);
        if answer != PickPoll::Pending {
            self.picks.remove(&ticket);
        }
        match answer {
            PickPoll::Pending => Ok(Picked::Pending),
            PickPoll::Stamped(None) => Ok(Picked::Done(None)),
            PickPoll::Stamped(Some(reply)) => {
                self.resolve_pick(&reply, request.mode, request.requested, projection).map(Picked::Done)
            }
            PickPoll::Refused(error) if error.code == codes::STALE_PICK && request.mode == PickMode::Passive => {
                Ok(Picked::Done(None))
            }
            PickPoll::Refused(error) => Err(error),
        }
    }

    /// Navigation from an installed occurrence: opens the producing version pinned with the publication (RM.10).
    pub fn follow(&self, id: ViewId, link: &SourceLink) -> Result<Followed, Error> {
        let installed = self.installed(id).ok_or_else(|| Error::invalid("nothing is installed"))?;
        source::follow(link, &installed.publication.sources)
    }

    /// A source action from an explicit pick, in RM.10's order: the pick bindings first, then the source digest
    /// against `current`, then the conversion to host units.
    pub fn pick_source(
        &self,
        reply: &PickReply,
        current: &[u8],
        projection: &dyn Projection,
    ) -> Result<Option<HostRange>, Error> {
        match self.resolve_pick(reply, PickMode::Explicit, Requested::Occurrence, projection)? {
            Some(Partial::Total(resolution)) => source::apply(&resolution.source, current).map(Some),
            _ => Ok(None),
        }
    }

    /// The instance IDs installed in a view, in plan order.
    pub fn instances(&self, id: ViewId) -> Vec<InstanceId> {
        self.installed(id)
            .map(|i| i.publication.plans.iter().flat_map(|p| p.instances.iter().map(|x| x.instance)).collect())
            .unwrap_or_default()
    }

    /// The subject kinds of the installed plans.
    pub fn subjects(&self, id: ViewId) -> Vec<SubjectKind> {
        self.installed(id).map(|i| i.validated.subjects.iter().map(|s| s.kind).collect()).unwrap_or_default()
    }
}

/// RPI.25 for a pick that is stale at `field`: an explicit pick returns `EV3:stale.display_pick` naming the field, and
/// a passive one is dropped quietly.
fn stale_pick(mode: PickMode, field: String) -> Result<Option<Partial<Resolution>>, Error> {
    match mode {
        PickMode::Passive => Ok(None),
        PickMode::Explicit => {
            Err(Error::new(codes::STALE_PICK, format!("the pick is stale: {field} changed")).with("field", field))
        }
    }
}
