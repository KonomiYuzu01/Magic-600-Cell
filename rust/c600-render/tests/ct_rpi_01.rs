//! CT-RPI-01 (source, part: switching the backend under a display plan leaves the plan's meaning unchanged; not run:
//! the boundary enumeration, which `c600-run/tests/tables.rs` runs): RPI.1's "a backend change preserves the meaning of
//! accepted plans" and RPI.54's adapter boundary, over c600-display's mock backend and over the bridge on the fake core.
//!
//! The comparison helper is [`meaning`]: it publishes one plan on a host over a backend, lets the backend install it,
//! picks every installed instance through the host, and returns what the host holds of the plan ([`Meaning`]). It keeps
//! nothing of the backend: no ticket, resource set, device epoch or frame. Each backend is wrapped in [`Switched`],
//! which records the canonical digest of every plan the backend is handed to prepare, so the comparison also covers
//! what crossed the boundary. Every result is a synthetic fixture outcome, not a GPU run.

mod support;

use c600_display::canonical;
use c600_display::error::Error;
use c600_display::host::{
    Backend, FrameInput, Host, MockBackend, PickPoll, PickTicket, Picked, Poll, Preparation, PublicationStamp,
    ResourceSet, Ticket,
};
use c600_display::pick::{FrameStamp, Partial, PickMode, PresentedFrame, Requested};
use c600_display::plan::*;
use c600_display::produce::{self, full_set};
use c600_display::receipt::{CancelReason, Status};
use c600_display::stage::FrameReport;
use c600_display::validate::{EngineBundle, SupportReport};
use c600_render::{FakeCore, RenderBackend};

use support::*;

/// A backend as the host drives it, plus the canonical digest of every plan it was handed to prepare (RPI.54's
/// `compile_display_plan` input) and the instance a test makes its next ID pass find.
struct Switched<B> {
    inner: B,
    handed: Vec<String>,
    aimed: Option<InstanceId>,
}

impl<B> Switched<B> {
    fn new(inner: B) -> Switched<B> {
        Switched { inner, handed: Vec::new(), aimed: None }
    }
}

impl<B: Backend> Backend for Switched<B> {
    fn describe_backend(&self) -> Result<SupportReport, Error> {
        self.inner.describe_backend()
    }

    fn adopt_engine(&mut self, bundle: &EngineBundle) -> Result<RevisionRef, Error> {
        self.inner.adopt_engine(bundle)
    }

    fn prepare(&mut self, request: Preparation<'_>) -> Result<Ticket, Error> {
        self.handed.push(canonical::request_digest(request.payload));
        self.inner.prepare(request)
    }

    fn poll(&mut self, ticket: Ticket) -> Poll {
        self.inner.poll(ticket)
    }

    fn activate(&mut self, resources: ResourceSet, stamp: &PublicationStamp) -> Result<(), Error> {
        self.inner.activate(resources, stamp)
    }

    fn cancel(&mut self, ticket: Ticket, reason: &CancelReason) {
        self.inner.cancel(ticket, reason)
    }

    fn retire(&mut self, resources: ResourceSet) {
        self.inner.retire(resources)
    }

    fn submit_frame(&mut self, input: FrameInput) -> Result<u64, Error> {
        self.inner.submit_frame(input)
    }

    fn read_presented(&mut self) -> Result<Vec<PresentedFrame>, Error> {
        self.inner.read_presented()
    }

    fn read_frame_reports(&mut self) -> Result<Vec<FrameReport>, Error> {
        self.inner.read_frame_reports()
    }

    fn request_pick(
        &mut self,
        frame: &FrameStamp,
        point: (u32, u32),
        requested: Requested,
    ) -> Result<PickTicket, Error> {
        self.inner.request_pick(frame, point, requested)
    }

    fn poll_pick(&mut self, ticket: PickTicket) -> PickPoll {
        self.inner.poll_pick(ticket)
    }

    fn recover(&mut self) -> Result<SupportReport, Error> {
        self.inner.recover()
    }
}

/// What differs between the two backends: how a view gets its camera, how a preparation completes, and how the ID pass
/// is made to find an instance and answer.
trait Drive: Backend {
    fn camera(&mut self);
    fn complete(&mut self, ticket: Ticket);
    /// The next ID pass at `point` finds `instance`, which the plan draws at `slot`.
    fn aim(&mut self, point: (u32, u32), instance: InstanceId, slot: usize);
    fn answer(&mut self, ticket: PickTicket);
}

impl Drive for Switched<MockBackend> {
    fn camera(&mut self) {}

    fn complete(&mut self, ticket: Ticket) {
        self.inner.finish(ticket);
    }

    fn aim(&mut self, _point: (u32, u32), instance: InstanceId, _slot: usize) {
        self.aimed = Some(instance);
    }

    fn answer(&mut self, ticket: PickTicket) {
        self.inner.answer_pick(ticket, self.aimed.take());
    }
}

impl Drive for Switched<RenderBackend<FakeCore>> {
    fn camera(&mut self) {
        self.inner.set_camera_matrix(V1, 0, CAMERA);
    }

    fn complete(&mut self, _ticket: Ticket) {
        self.inner.core.complete();
    }

    fn aim(&mut self, point: (u32, u32), _instance: InstanceId, slot: usize) {
        self.inner.core.place(point.0, point.1, slot);
    }

    fn answer(&mut self, _ticket: PickTicket) {
        self.inner.core.complete();
    }
}

/// The meaning of an accepted plan as the host holds it once a backend installed it (RPI.1): the plans the backend was
/// handed, the installed publication's canonical digest, the validated publication (its digest and charged bytes, the
/// counted fields, the subjects, the shown nodes, the dependencies and the chart properties), the installed instances
/// and what an explicit pick of each resolves to.
#[derive(Debug, PartialEq, Eq)]
struct Meaning {
    handed: Vec<String>,
    request: String,
    validated: String,
    instances: Vec<InstanceId>,
    picks: Vec<String>,
}

/// The comparison helper: publishes `publication` at `base` on a host over `backend`, lets the backend install it,
/// picks each instance of `drawn` (an instance and the slot the plan draws it at) through the host, and returns the
/// plan's meaning.
fn meaning<B>(mut backend: Switched<B>, publication: &Publication, drawn: &[(InstanceId, usize)]) -> Meaning
where
    Switched<B>: Drive,
{
    let base = publication.base.clone();
    assert_eq!(backend.adopt_engine(&bundle(twisted(), &base)), Ok(base.clone()));
    let mut ctx = produce::fixture_context(geometry(), limits());
    ctx.support = backend.describe_backend().expect("a live backend describes itself");
    let mut host = Host::new(backend, ctx, base);
    host.open_view(V1).expect("view 1 opens");
    host.backend.camera();

    let receipt = host.publish(CommandId(1), publication.clone());
    assert_eq!(receipt.status, Status::Accepted, "{:?}", receipt.status);
    host.pump();
    let ticket = host.ticket(CommandId(1)).expect("the plan was prepared");
    host.backend.complete(ticket);
    host.pump();
    let status = host.receipt(CommandId(1)).expect("a receipt").status;
    assert!(matches!(status, Status::Completed(_)), "{status:?}");

    let installed = host.installed(V1).expect("installed");
    let v = &installed.validated;
    let request = canonical::request_digest(&installed.publication);
    let validated =
        format!("{:?}", (&v.digest, v.output_bytes, &v.counts, &v.subjects, &v.nodes, &v.dependencies, &v.properties));
    let instances = host.instances(V1);

    let frame = host.frame(V1).expect("a frame");
    let mut picks = Vec::new();
    for (n, (instance, slot)) in drawn.iter().enumerate() {
        let point = (10 + 20 * n as u32, 20);
        host.backend.aim(point, *instance, *slot);
        let ticket = host
            .request_pick(&frame, point, PickMode::Explicit, Requested::Occurrence)
            .expect("a pick")
            .expect("a ticket");
        host.backend.answer(ticket);
        match host.poll_pick(ticket, &Unasked) {
            Ok(Picked::Done(Some(Partial::Total(resolution)))) => picks.push(format!("{resolution:?}")),
            other => panic!("{other:?}"),
        }
    }
    Meaning { handed: host.backend.handed.clone(), request, validated, instances, picks }
}

fn mock() -> Switched<MockBackend> {
    Switched::new(MockBackend::new(produce::fixture_context(geometry(), limits()).support))
}

fn bridge() -> Switched<RenderBackend<FakeCore>> {
    Switched::new(RenderBackend::new(FakeCore::new(SIZE), limits()))
}

/// A retained publication of the twisted state at revision 12 that draws `objects`.
fn plan(objects: &[(u64, ObjectId)]) -> Publication {
    let state = twisted();
    let base = produce::revision(&geometry().identity, 12, state);
    let retained = produce::retained(state, &full_set(), &full_set(), Vec::new());
    publication(&base, vec![Primitive::Retained(retained)], objects)
}

#[test]
fn ct_rpi_01_switching_the_backend_leaves_an_accepted_plans_meaning_unchanged() {
    let at = slots(twisted().labels());
    let objects = [(1, ObjectId::Sticker(at.home)), (2, ObjectId::Slot(at.fixed as u32))];
    let drawn = [(InstanceId(1), at.moved), (InstanceId(2), at.fixed)];
    let publication = plan(&objects);

    let on_mock = meaning(mock(), &publication, &drawn);
    let on_bridge = meaning(bridge(), &publication, &drawn);
    assert_eq!(on_mock, on_bridge, "the backend changed the plan's meaning");

    // Each backend was handed the plan the program published, once, and the host installed that plan.
    let published = canonical::request_digest(&publication);
    assert_eq!(on_mock.handed, [published.clone()]);
    assert_eq!(on_mock.request, published);
    assert_eq!(on_mock.instances, [InstanceId(1), InstanceId(2)]);
    // The picks resolve to the plan's objects: the sticker where it sits, and the slot.
    assert!(on_mock.picks[0].contains(&format!("target: {:?}", ObjectId::Sticker(at.home))), "{:?}", on_mock.picks);
    assert!(
        on_mock.picks[1].contains(&format!("target: {:?}", ObjectId::Slot(at.fixed as u32))),
        "{:?}",
        on_mock.picks
    );
}

#[test]
fn ct_rpi_01_the_meaning_follows_the_plan_and_not_the_backend() {
    let at = slots(twisted().labels());
    let first = meaning(
        mock(),
        &plan(&[(1, ObjectId::Sticker(at.home)), (2, ObjectId::Slot(at.fixed as u32))]),
        &[(InstanceId(1), at.moved), (InstanceId(2), at.fixed)],
    );

    // Another plan: instance 2 is another slot. The helper's observable changes with the plan, the same way on both
    // backends, so the equality above is no constant.
    let objects = [(1, ObjectId::Sticker(at.home)), (2, ObjectId::Slot(at.hidden as u32))];
    let drawn = [(InstanceId(1), at.moved), (InstanceId(2), at.hidden)];
    let on_mock = meaning(mock(), &plan(&objects), &drawn);
    let on_bridge = meaning(bridge(), &plan(&objects), &drawn);
    assert_eq!(on_mock, on_bridge);
    assert_ne!(on_mock.request, first.request);
    assert_ne!(on_mock.validated, first.validated);
    assert_eq!(on_mock.picks[0], first.picks[0], "instance 1 is the same sticker in both plans");
    assert_ne!(on_mock.picks[1], first.picks[1]);
}
