//! The view model: views, the view lane, revision binding and the view budget.
//!
//! [`ViewModel`] holds one adopted engine bundle, the views (Global and Local plus further views), one view lane per
//! view and the physical charges of RM.39. It never waits for program output: [`ViewModel::adopt`] installs a
//! complete bundle at once, fences work whose dependencies moved and records the newest invalidation (RPI.2, ONE.64,
//! LI.25). Program work leaves the model as [`Job`]s through [`ViewModel::take_jobs`] and comes back through
//! [`ViewModel::finish`]; a test double or a real runtime can sit behind that seam.

use std::collections::BTreeMap;

use c600_digest::sha256;

use crate::bundle::{validate_engine_bundle, BundleCandidate, CompleteBundle};
use crate::error::{code, BundleFault, Context, Error, Form};
use crate::limits::{Field, LimitProfile, OutputAllowance};
use crate::provenance::{BoundaryClass, Profile, Provenance};
use crate::publication::{Content, PublicationInput};
use crate::receipt::{
    CancelReason, CommandId, Completion, CompletionKind, JobId, JobOutcome, JobResult, ResultRef, Status,
};
use crate::revision::{Base, Depends, InputKey, Revision};
use crate::Digest;

/// The fixed `ViewId` of Global (draft, LI.22).
pub const GLOBAL: &str = "global";
/// The fixed `ViewId` of Local (draft, LI.22).
pub const LOCAL: &str = "local";

/// How many equal-input read results a view keeps for ONE.10's reuse (draft).
pub const CACHE_LIMIT: usize = 4;

/// A view identifier in LI.22's view namespace.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ViewId(String);

impl ViewId {
    /// A view identifier.
    pub fn new(name: &str) -> ViewId {
        ViewId(name.to_string())
    }

    /// Global.
    pub fn global() -> ViewId {
        ViewId::new(GLOBAL)
    }

    /// Local.
    pub fn local() -> ViewId {
        ViewId::new(LOCAL)
    }

    /// The identifier's text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// True for Global and Local, which are always open (RPI.42, RPI.83).
    pub fn is_fixed(&self) -> bool {
        self.0 == GLOBAL || self.0 == LOCAL
    }
}

/// RPI.83's `ViewSourceRef`: what fills a further view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViewSource {
    /// A built-in source by name.
    BuiltIn { name: String },
    /// An overlay row of kind `view`, its checked module's source digest and typed arguments.
    Overlay { row: String, module: Digest, arguments: Vec<u8> },
}

/// Who asks for `view.open` or `view.close` (RPI.83). The command layer supplies it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// A command the solver invoked.
    Solver,
    /// A recomputation started by a revision change. It holds no permission for either row.
    Recomputation,
}

/// The declared per-frame workload of a view (RM.38, RM.39, RPI.71, RPI.82).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Workload {
    /// Vertex evaluations per frame while visible.
    pub vertex_evals: u64,
    /// Weighted operations of one vertex evaluation of the view's user stage chain; 0 for a built-in chain.
    pub stage_ops: u64,
    /// Weighted operations of one fragment invocation of the user stage chain; 0 for a built-in chain.
    pub fragment_ops: u64,
    /// Resident CPU bytes, including the pending generation and staging.
    pub cpu_bytes: u64,
    /// Resident GPU bytes, including the pending generation and staging.
    pub gpu_bytes: u64,
    /// Surface pixels at open.
    pub pixels: u64,
}

impl Workload {
    /// True when the view draws through a user stage chain.
    pub fn has_user_stage(&self) -> bool {
        self.stage_ops > 0 || self.fragment_ops > 0
    }
}

/// What the host declares for a view's source when it opens: its dependencies, its workload, the output allowance of
/// each revision-driven run and, for a built-in source, the boundary class of the tool's manifest (draft, RPI.83,
/// BND.27).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Declared {
    pub depends: Depends,
    pub workload: Workload,
    pub output: OutputAllowance,
    /// The class the host read from a built-in tool's manifest; `None` for a solver's own tool (BND.27).
    pub class: Option<BoundaryClass>,
}

/// A run handed to the runner. Its base is the newest adopted revision at start (RPI.12).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Job {
    pub job: JobId,
    pub command: CommandId,
    pub view: ViewId,
    pub source: ViewSource,
    pub base: Base,
}

/// What a runner reports for a job.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunOutcome {
    /// The run's complete publication. The view model binds it to the job's base.
    Published(Content),
    /// The run was cut off with this code (KN7.10).
    CutOff(&'static str),
    /// The run stopped with an error.
    Error(Error),
}

/// An installed publication: nodes, plans and source maps installed together (RPI.10), with the block the display
/// sink stores (RPI.45, BND.26).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Installed {
    pub command: CommandId,
    pub base: Base,
    pub digest: Digest,
    pub content: Content,
    /// The installed view generation (RM.20's `covered`).
    pub covered: u64,
    pub provenance: Provenance,
}

/// The host's status of a view's program output (RPI.13, charter D2). Each status carries its typed data.
///
/// Precedence (draft): installed output decides first (`Current`, `Empty` or `Stale`); without it, a failure of the
/// newest request is `Error`, a newest request waiting for a slot held by fenced work is `Busy`, any other accepted
/// request is `Loading`, and nothing at all is `Empty`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViewStatus {
    /// Installed output matches the adopted revision.
    Current { base: Base, publication: Digest },
    /// Nothing to show at the adopted revision: the matching installed output is empty, or there is no output and
    /// no request.
    Empty { revision: Revision, base: Option<Base> },
    /// A request for the newest revision is accepted and no output exists yet.
    Loading { command: CommandId, revision: Revision },
    /// The newest request failed or was cut off, and no output exists.
    Error { command: CommandId, error: Error },
    /// Retained output lags the adopted revision; it names its base until matching output arrives (ONE.12).
    Stale { base: Base, adopted: Revision, refresh: Option<CommandId> },
    /// The newest request waits for the runner slot that fenced work still holds (LI.23).
    Busy { waiting: CommandId, held_by: CommandId },
}

/// Which chain a visible view draws with (RM.38).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageUse {
    /// The view declares no user stage.
    BuiltIn,
    /// The view's user stage chain.
    User,
    /// A resize took the view over `frame_work`; it draws with the built-in chain and visible status.
    Fallback,
}

/// What a visible view contributes to a frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Draw {
    /// Output whose declared dependencies match the frame's revision (ONE.63), drawn with `stage` and carrying the
    /// block of its producer whichever chain draws it (RPI.45).
    Output { base: Base, publication: Digest, stage: StageUse, provenance: Provenance },
    /// The view's own stale presentation, naming its base. Its overlays do not attach to the new state (RPI.13).
    Stale { base: Base },
    /// No program output. Global and Local still draw the bundle.
    Nothing,
}

/// One visible view in a frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameView {
    pub view: ViewId,
    pub generation: u64,
    /// True for Global and Local, which draw the engine bundle.
    pub bundle: bool,
    pub draw: Draw,
}

/// The inputs of one frame: one engine revision and its bundle, and every visible view (RM.24, without device data).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameInput {
    pub revision: Revision,
    pub bundle: Digest,
    pub views: Vec<FrameView>,
}

/// Work counters of one view (RPI.11, RM.17, RM.25, KN7.8). Every counter only grows; a view that does no work
/// leaves all of them unchanged.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counters {
    /// Host-scheduled recomputations admitted to the view's lane: the first run at open, a run for a revision that
    /// moved its declared dependencies while it was visible, and a run when showing finds it out of date.
    pub recomputations: u64,
    /// Program runs handed to the runner.
    pub program_runs: u64,
    /// Runs charged: program runs plus reused read results, which are charged as misses (KN7.8).
    pub charged_runs: u64,
    /// Read results reused under ONE.10.
    pub cache_hits: u64,
    /// Payloads validated and prepared as display resources of a new generation (RM.17's prepare), installed or not.
    pub preparations: u64,
    /// Prepared payloads installed (RM.17's activate). Each is a display-resource rebuild.
    pub installs: u64,
}

/// The view budget in use (RM.38, RM.39, RPI.82).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Usage {
    /// Vertex evaluations per frame over visible views; a hidden view adds nothing.
    pub view_vertex_evals: u128,
    /// RM.38's weighted frame charge over visible views with an active user stage.
    pub frame_work: u128,
    /// Resident CPU bytes of open, unevicted views plus resources awaiting safe retirement.
    pub adapter_cpu_bytes: u128,
    /// Resident GPU bytes of open, unevicted views plus resources awaiting safe retirement.
    pub adapter_gpu_bytes: u128,
}

/// A set of resources awaiting safe retirement (RM.19).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RetireToken(u64);

struct ViewState {
    source: Option<ViewSource>,
    declared: Declared,
    generation: u64,
    visible: bool,
    evicted: bool,
    fallback: bool,
    pixels: u64,
    invalidated: bool,
    installed: Option<Installed>,
    failure: Option<(CommandId, Error)>,
    counters: Counters,
    cache: Vec<(InputKey, Content)>,
}

impl ViewState {
    fn new(source: Option<ViewSource>, declared: Declared, generation: u64) -> ViewState {
        ViewState {
            source,
            declared,
            generation,
            visible: true,
            evicted: false,
            fallback: false,
            pixels: declared.workload.pixels,
            invalidated: false,
            installed: None,
            failure: None,
            counters: Counters::default(),
            cache: Vec::new(),
        }
    }

    fn frame_work(&self, overdraw: u64) -> u128 {
        let work = &self.declared.workload;
        u128::from(work.vertex_evals) * u128::from(work.stage_ops)
            + u128::from(self.pixels) * u128::from(overdraw) * u128::from(work.fragment_ops)
    }

    fn stage_use(&self) -> StageUse {
        if !self.declared.workload.has_user_stage() {
            StageUse::BuiltIn
        } else if self.fallback {
            StageUse::Fallback
        } else {
            StageUse::User
        }
    }

    fn remember(&mut self, key: InputKey, content: Content) {
        self.cache.retain(|(cached, _)| *cached != key);
        self.cache.push((key, content));
        if self.cache.len() > CACHE_LIMIT {
            self.cache.remove(0);
        }
    }
}

enum Request {
    Run { command: CommandId, output: OutputAllowance },
    Publish { command: CommandId, input: PublicationInput },
}

impl Request {
    fn command(&self) -> CommandId {
        match self {
            Request::Run { command, .. } | Request::Publish { command, .. } => *command,
        }
    }

    fn output(&self) -> OutputAllowance {
        match self {
            Request::Run { output, .. } => *output,
            Request::Publish { input, .. } => input.output,
        }
    }
}

struct Runner {
    job: JobId,
    command: CommandId,
    base: Base,
    output: OutputAllowance,
    fenced: bool,
    taken: bool,
}

/// One view lane: at most one runner and one newest pending request (KN7.2, LI.23).
#[derive(Default)]
struct Lane {
    runner: Option<Runner>,
    pending: Option<Request>,
}

/// The headless view model of one session.
pub struct ViewModel {
    session: Profile,
    profile: LimitProfile,
    bundle: CompleteBundle,
    views: BTreeMap<ViewId, ViewState>,
    order: Vec<ViewId>,
    lanes: BTreeMap<ViewId, Lane>,
    receipts: BTreeMap<CommandId, Status>,
    results: BTreeMap<CommandId, JobResult>,
    jobs: BTreeMap<JobId, ViewId>,
    ready: Vec<JobId>,
    retiring: BTreeMap<RetireToken, (u64, u64)>,
    next_generation: u64,
    next_job: u64,
    next_scheduled: u64,
    next_token: u64,
    next_covered: u64,
}

impl ViewModel {
    /// A view model for a session in profile `session`, with Global and Local open and visible over a first complete
    /// bundle. Their workloads must fit the limit profile together. The session's profile changes no admission rule
    /// here: Global and Local are linked and further views open the same way in every profile (RPI.42, EV3.11).
    pub fn new(
        session: Profile,
        profile: LimitProfile,
        initial: &BundleCandidate,
        global: Workload,
        local: Workload,
    ) -> Result<ViewModel, Error> {
        let bundle = validate_engine_bundle(initial)?;
        let mut model = ViewModel {
            session,
            profile,
            bundle,
            views: BTreeMap::new(),
            order: Vec::new(),
            lanes: BTreeMap::new(),
            receipts: BTreeMap::new(),
            results: BTreeMap::new(),
            jobs: BTreeMap::new(),
            ready: Vec::new(),
            retiring: BTreeMap::new(),
            next_generation: 1,
            next_job: 1,
            next_scheduled: 1,
            next_token: 1,
            next_covered: 1,
        };
        for (view, workload) in [(ViewId::global(), global), (ViewId::local(), local)] {
            let declared =
                Declared { depends: Depends::NONE, workload, output: OutputAllowance::new(0, 0), class: None };
            let state = ViewState::new(None, declared, model.next_generation);
            model.check_view_usage(model.usage_with(&state, true))?;
            model.next_generation += 1;
            model.order.push(view.clone());
            model.views.insert(view, state);
        }
        Ok(model)
    }

    // ----- engine adoption (RPI.2, RPI.9, ONE.60, ONE.64, LI.25)

    /// Adopts a complete engine bundle at once, or returns `refresh_required(EV3:input.display_bundle_mismatch)`.
    ///
    /// Adoption never waits for program output. It fences runs and pending publications whose declared dependencies
    /// moved (`cancelled(replaced_revision)`), and each further view whose declared dependencies moved records the
    /// newest invalidation: a visible view gets one scheduled run at the newest revision, a hidden view only keeps
    /// the invalidation (RPI.11, RPI.12). Re-adopting the identical bundle changes nothing.
    pub fn adopt(&mut self, candidate: &BundleCandidate) -> Result<Revision, Error> {
        let complete = validate_engine_bundle(candidate)?;
        let old = *self.bundle.revision();
        let new = *complete.revision();
        let out_of_order = || {
            Error::refusal(
                code::DISPLAY_BUNDLE_MISMATCH,
                Context::Bundle(BundleFault::OutOfOrder { adopted: old, candidate: new }),
            )
        };
        if new.same_counters(&old) {
            return if complete.digest() == self.bundle.digest() { Ok(new) } else { Err(out_of_order()) };
        }
        if !new.not_older_than(&old) {
            return Err(out_of_order());
        }
        self.bundle = complete;
        let lanes: Vec<ViewId> = self.lanes.keys().cloned().collect();
        for view in &lanes {
            self.fence_on_adoption(view, &new);
        }
        for view in self.order.clone() {
            self.invalidate(&view, &old, &new);
        }
        Ok(new)
    }

    fn fence_on_adoption(&mut self, view: &ViewId, adopted: &Revision) {
        let Some(lane) = self.lanes.get_mut(view) else { return };
        let mut cancelled = Vec::new();
        if let Some(runner) = lane.runner.as_mut() {
            if !runner.fenced && !runner.base.matches(adopted) {
                runner.fenced = true;
                cancelled.push((runner.command, Some(runner.job), runner.output));
            }
        }
        if matches!(&lane.pending, Some(Request::Publish { input, .. }) if !input.base.matches(adopted)) {
            if let Some(request) = lane.pending.take() {
                cancelled.push((request.command(), None, request.output()));
            }
        }
        for (command, job, output) in cancelled {
            self.cancel(command, job, output, CancelReason::ReplacedRevision);
        }
        self.release_untaken(view);
    }

    fn invalidate(&mut self, view: &ViewId, old: &Revision, new: &Revision) {
        let Some(state) = self.views.get_mut(view) else { return };
        if state.source.is_none() || old.same_parts(new, state.declared.depends) {
            self.try_start(view);
            return;
        }
        if !state.visible {
            state.invalidated = true;
            return;
        }
        let output = state.declared.output;
        let has_pending = self.lanes.get(view).is_some_and(|lane| lane.pending.is_some());
        if has_pending {
            self.try_start(view);
        } else {
            self.schedule(view, output);
        }
    }

    /// Admits one host-scheduled recomputation of a further view (LI.25) and counts it.
    fn schedule(&mut self, view: &ViewId, output: OutputAllowance) {
        let command = self.mint();
        if let Some(state) = self.views.get_mut(view) {
            state.counters.recomputations += 1;
        }
        self.admit(view, Request::Run { command, output });
    }

    // ----- further views (RPI.81 to RPI.83, RM.39)

    /// `view.open {view, source}`: opens a further view, visible, and schedules its first run.
    ///
    /// Only a solver-invoked command may open a view; a recomputation gets `LG3:capability.not_granted` (draft).
    /// Naming an open view, Global or Local included, is `EV3:input.view_id_in_use`. Opening past `frame_work`,
    /// `view_vertex_evals`, `adapter_cpu_bytes` or `adapter_gpu_bytes` is refused with
    /// `EV3:resource_limit.display_plan_ceiling`; nothing is queued. Returns the view's generation.
    pub fn open_view(
        &mut self,
        origin: Origin,
        view: ViewId,
        source: ViewSource,
        declared: Declared,
    ) -> Result<u64, Error> {
        if origin != Origin::Solver {
            return Err(Error::refusal(code::NOT_GRANTED, Context::View(view.0)));
        }
        if self.views.contains_key(&view) {
            return Err(Error::refusal(code::VIEW_ID_IN_USE, Context::View(view.0)));
        }
        let state = ViewState::new(Some(source), declared, self.next_generation);
        self.check_view_usage(self.usage_with(&state, true))?;
        self.next_generation += 1;
        let generation = state.generation;
        self.views.insert(view.clone(), state);
        self.order.push(view.clone());
        self.schedule(&view, declared.output);
        Ok(generation)
    }

    /// `view.close {view}`: closes a further view.
    ///
    /// The close is a newer request on the view's lane, so unfinished work there ends
    /// `cancelled(superseded(by: command))` (LI.23). The view's resources move to safe retirement and stay charged
    /// until [`ViewModel::complete_retirement`] (RM.19). Global, Local and unknown views give
    /// `EV3:input.view_not_closable`; a recomputation gets `LG3:capability.not_granted` (draft). Returns the
    /// retirement token, or `None` when an eviction already retired the view's resources.
    pub fn close_view(
        &mut self,
        command: CommandId,
        origin: Origin,
        view: &ViewId,
    ) -> Result<Option<RetireToken>, Error> {
        if origin != Origin::Solver {
            return Err(Error::refusal(code::NOT_GRANTED, Context::View(view.0.clone())));
        }
        if view.is_fixed() || !self.views.contains_key(view) {
            return Err(Error::refusal(code::VIEW_NOT_CLOSABLE, Context::View(view.0.clone())));
        }
        self.cancel_lane(view, CancelReason::Superseded { by: command });
        let Some(state) = self.views.remove(view) else { return Ok(None) };
        self.order.retain(|id| id != view);
        let token = (!state.evicted).then(|| self.retire(&state.declared.workload));
        self.restore_fallbacks();
        Ok(token)
    }

    /// Hides a further view. Its unfinished requests end `cancelled(by_user)` and it keeps its invalidation; it runs
    /// no program and rebuilds nothing while hidden, and its allocations stay charged (RM.25, RM.39, RPI.11).
    /// Returns false, changing nothing, for Global, Local, an unknown or an already hidden view.
    pub fn hide(&mut self, view: &ViewId) -> bool {
        if !self.views.get(view).is_some_and(|state| state.source.is_some() && state.visible) {
            return false;
        }
        let cancelled = self.cancel_lane(view, CancelReason::ByUser);
        if let Some(state) = self.views.get_mut(view) {
            state.visible = false;
            state.fallback = false;
            state.invalidated |= cancelled;
        }
        self.restore_fallbacks();
        true
    }

    /// Shows a hidden further view after charging it again (RM.38, RM.39); over a ceiling it is refused with
    /// `EV3:resource_limit.display_plan_ceiling` and stays hidden. Showing checks the dependencies once and runs at
    /// most once, at the newest revision (RPI.11, RPI.12). Returns `Ok(false)` for Global, Local, an unknown or a
    /// visible view.
    pub fn show(&mut self, view: &ViewId) -> Result<bool, Error> {
        let Some(state) = self.views.get(view) else { return Ok(false) };
        if state.source.is_none() || state.visible {
            return Ok(false);
        }
        self.check_view_usage(self.usage_with(state, state.evicted))?;
        let adopted = *self.bundle.revision();
        let has_pending = self.lanes.get(view).is_some_and(|lane| lane.pending.is_some());
        let Some(state) = self.views.get_mut(view) else { return Ok(false) };
        state.visible = true;
        state.evicted = false;
        let needs_run =
            state.invalidated || state.installed.as_ref().is_none_or(|installed| !installed.base.matches(&adopted));
        state.invalidated = false;
        let output = state.declared.output;
        if has_pending {
            self.try_start(view);
        } else if needs_run {
            self.schedule(view, output);
        }
        Ok(true)
    }

    /// Evicts a hidden further view: its output and retained read results are dropped and its resources move to safe
    /// retirement, where they stay charged (RM.19, RM.39). Showing it again needs fresh resources beside the retiring
    /// ones and runs the program again (KN7.8).
    pub fn evict(&mut self, view: &ViewId) -> Option<RetireToken> {
        let state = self.views.get_mut(view)?;
        if state.source.is_none() || state.visible || state.evicted {
            return None;
        }
        state.evicted = true;
        state.installed = None;
        state.cache.clear();
        state.invalidated = true;
        let workload = state.declared.workload;
        Some(self.retire(&workload))
    }

    /// The backend proved that a retired resource set is no longer used; its bytes stop counting (RM.19).
    pub fn complete_retirement(&mut self, token: RetireToken) -> bool {
        self.retiring.remove(&token).is_some()
    }

    /// Changes a view's surface size. `frame_work` is charged again before the change takes effect. A resize cannot
    /// be refused, so a view that it takes over the ceiling falls back to the built-in chain with visible status and
    /// no quarantine; fallen-back stages return, in open order, once a recharge fits (RM.38).
    pub fn resize(&mut self, view: &ViewId, pixels: u64) {
        let Some(state) = self.views.get_mut(view) else { return };
        state.pixels = pixels;
        let active = state.visible && !state.fallback && state.declared.workload.has_user_stage();
        if active && self.usage().frame_work > u128::from(self.profile.get(Field::FrameWork)) {
            if let Some(state) = self.views.get_mut(view) {
                state.fallback = true;
            }
        }
        self.restore_fallbacks();
    }

    fn restore_fallbacks(&mut self) {
        let ceiling = u128::from(self.profile.get(Field::FrameWork));
        let overdraw = self.profile.get(Field::Overdraw);
        for view in self.order.clone() {
            let Some(state) = self.views.get(&view) else { continue };
            if !(state.visible && state.fallback) {
                continue;
            }
            if self.usage().frame_work + state.frame_work(overdraw) <= ceiling {
                if let Some(state) = self.views.get_mut(&view) {
                    state.fallback = false;
                }
            }
        }
    }

    // ----- requests on the view lane (RPI.10, LI.22 to LI.25, RM.20, RM.21)

    /// A standalone host publication to `view`. Its payload exists at admission, so it is checked first: the target
    /// view and generation and the dependencies of its base (`EV3:stale.display_plan`), its structure
    /// (`EV3:input.display_plan_invalid`) and the fixed ceilings (`EV3:resource_limit.display_plan_ceiling`). A
    /// failed check is `refused(Error)` and supersedes nothing. An admitted publication replaces the view's pending
    /// request and fences its runner (LI.23); it is prepared synchronously once the runner slot is free.
    pub fn publish(&mut self, command: CommandId, view: &ViewId, input: PublicationInput) -> Status {
        if let Err(error) = self.check_standalone(view, &input) {
            let status = Status::Refused(error);
            self.receipts.insert(command, status.clone());
            return status;
        }
        self.admit(view, Request::Publish { command, input });
        self.receipt(command).cloned().unwrap_or(Status::Accepted)
    }

    fn check_standalone(&self, view: &ViewId, input: &PublicationInput) -> Result<(), Error> {
        let current = self.views.get(view).map(|state| state.generation);
        if current != Some(input.generation) {
            return Err(Error::refusal(
                code::STALE_DISPLAY_PLAN,
                Context::Generation { requested: input.generation, current },
            ));
        }
        let adopted = *self.bundle.revision();
        if !input.base.matches(&adopted) {
            return Err(Error::refusal(code::STALE_DISPLAY_PLAN, Context::Stale { base: input.base, adopted }));
        }
        input.content.admit(&input.base, &self.profile, Form::Refusal).map(|_| ())
    }

    /// A solver-invoked run of a further view's source. It is accepted at once, replaces the pending request and
    /// fences the runner (LI.23); its output is validated when it finishes, and any validation error is
    /// `failed(original Error)` on this same command (RM.20, RM.35). A run naming Global, Local or no open view is
    /// refused with `EV3:stale.display_plan` (draft).
    pub fn invoke(&mut self, command: CommandId, view: &ViewId, output: OutputAllowance) -> Status {
        if !self.views.get(view).is_some_and(|state| state.source.is_some()) {
            let status = Status::Refused(Error::refusal(code::STALE_DISPLAY_PLAN, Context::View(view.0.clone())));
            self.receipts.insert(command, status.clone());
            return status;
        }
        self.admit(view, Request::Run { command, output });
        self.receipt(command).cloned().unwrap_or(Status::Accepted)
    }

    /// Hands every started job to the runner. A started job is fenced if a newer request or an adoption replaced it;
    /// the runner may check [`ViewModel::is_fenced`] and stop at a chunk boundary.
    pub fn take_jobs(&mut self) -> Vec<Job> {
        let mut jobs = Vec::new();
        for job in std::mem::take(&mut self.ready) {
            let Some(view) = self.jobs.get(&job).cloned() else { continue };
            let Some(runner) = self.lanes.get_mut(&view).and_then(|lane| lane.runner.as_mut()) else { continue };
            if runner.job != job || runner.fenced {
                continue;
            }
            let Some(state) = self.views.get_mut(&view) else { continue };
            let Some(source) = state.source.clone() else { continue };
            runner.taken = true;
            state.counters.program_runs += 1;
            state.counters.charged_runs += 1;
            jobs.push(Job { job, command: runner.command, view, source, base: runner.base });
        }
        jobs
    }

    /// Ends a taken job and releases its runner slot. A fenced job's late completion installs nothing and keeps its
    /// first terminal (RM.21, RM.22). Otherwise a publication is validated, checked against the adopted revision,
    /// charged and installed whole (RM.13, RM.20, LI.25). Returns the command's status, or `None` for an unknown job.
    pub fn finish(&mut self, job: JobId, outcome: RunOutcome) -> Option<Status> {
        let view = self.jobs.get(&job)?.clone();
        let lane = self.lanes.get_mut(&view)?;
        if lane.runner.as_ref().map(|runner| runner.job) != Some(job) {
            return None;
        }
        let runner = lane.runner.take()?;
        self.jobs.remove(&job);
        self.ready.retain(|ready| *ready != job);
        if !runner.fenced {
            match outcome {
                RunOutcome::Published(content) => {
                    let run = (runner.command, Some(job), runner.base, runner.output);
                    self.finalize(&view, run, content, None, true)
                }
                RunOutcome::CutOff(cut) => {
                    let outcome = JobOutcome::CutOff(cut);
                    let status = outcome.receipt(None, CancelReason::Budget);
                    self.settle_result(&view, runner.command, Some(job), outcome, status, runner.output);
                }
                RunOutcome::Error(error) => {
                    self.fail(&view, runner.command, Some(job), runner.output, error.with_form(Form::Stop))
                }
            }
        }
        let status = self.receipts.get(&runner.command).cloned();
        self.try_start(&view);
        status
    }

    /// Runs every startable job through `run` until none is left. Returns the number of jobs run.
    pub fn pump(&mut self, mut run: impl FnMut(&Job) -> RunOutcome) -> usize {
        let mut count = 0;
        loop {
            let jobs = self.take_jobs();
            if jobs.is_empty() {
                return count;
            }
            for job in jobs {
                let outcome = run(&job);
                self.finish(job.job, outcome);
                count += 1;
            }
        }
    }

    fn admit(&mut self, view: &ViewId, request: Request) {
        let by = request.command();
        self.receipts.insert(by, Status::Accepted);
        if let Some(state) = self.views.get_mut(view) {
            state.failure = None;
        }
        let lane = self.lanes.entry(view.clone()).or_default();
        let replaced = lane.pending.replace(request);
        let mut fenced = None;
        if let Some(runner) = lane.runner.as_mut() {
            if !runner.fenced {
                runner.fenced = true;
                fenced = Some((runner.command, runner.job, runner.output));
            }
        }
        if let Some(old) = replaced {
            self.cancel(old.command(), None, old.output(), CancelReason::Superseded { by });
        }
        if let Some((command, job, output)) = fenced {
            self.cancel(command, Some(job), output, CancelReason::Superseded { by });
        }
        self.release_untaken(view);
        self.try_start(view);
    }

    fn try_start(&mut self, view: &ViewId) {
        loop {
            if !self.views.get(view).is_some_and(|state| state.visible) {
                return;
            }
            let Some(lane) = self.lanes.get_mut(view) else { return };
            if lane.runner.is_some() {
                return;
            }
            let Some(request) = lane.pending.take() else { return };
            match request {
                Request::Run { command, output } => {
                    if self.start_run(view, command, output) {
                        return;
                    }
                }
                Request::Publish { command, input } => {
                    let request = (command, None, input.base, input.output);
                    self.finalize(view, request, input.content, Some(input.provenance), false);
                }
            }
        }
    }

    /// Starts a run at the newest revision. Returns true when a job now holds the runner slot.
    fn start_run(&mut self, view: &ViewId, command: CommandId, output: OutputAllowance) -> bool {
        let adopted = *self.bundle.revision();
        let Some(state) = self.views.get_mut(view) else { return false };
        let base = Base::new(adopted, state.declared.depends);
        if matches!(command, CommandId::Scheduled(_)) {
            let key = base.inputs();
            let hit = state.cache.iter().find(|(cached, _)| *cached == key).map(|(_, content)| content.clone());
            if let Some(content) = hit {
                state.counters.cache_hits += 1;
                state.counters.charged_runs += 1;
                self.finalize(view, (command, None, base, output), content, None, false);
                return false;
            }
        }
        let job = JobId(self.next_job);
        self.next_job += 1;
        let lane = self.lanes.entry(view.clone()).or_default();
        lane.runner = Some(Runner { job, command, base, output, fenced: false, taken: false });
        self.jobs.insert(job, view.clone());
        self.ready.push(job);
        if !self.receipts.get(&command).is_some_and(Status::is_terminal) {
            self.receipts.insert(command, Status::Running(job));
        }
        true
    }

    /// Validates and prepares, rechecks dependencies, charges output and installs (RM.13, RM.17, RM.20, LI.25). `request`
    /// is the command, its job, its base and its output allowance. `provenance` is the block a standalone publication
    /// carries; `None` means a run of the view's source, whose block the host fills from the session's profile, the
    /// view's source and its declared class (BND.22). `remember` keeps a run's read result for ONE.10's reuse; host
    /// publications and reused results are not kept again.
    fn finalize(
        &mut self,
        view: &ViewId,
        request: (CommandId, Option<JobId>, Base, OutputAllowance),
        content: Content,
        provenance: Option<Provenance>,
        remember: bool,
    ) {
        let (command, job, base, mut output) = request;
        let session = self.session;
        let provenance = match (provenance, self.views.get(view)) {
            (Some(provenance), _) => provenance,
            (None, Some(ViewState { source: Some(source), declared, .. })) => {
                Provenance { profile: session, producer: source.clone(), class: declared.class }
            }
            (None, _) => return,
        };
        let bytes = match content.admit(&base, &self.profile, Form::Stop) {
            Ok(bytes) => bytes,
            Err(error) => return self.fail(view, command, job, output, error),
        };
        if let Some(state) = self.views.get_mut(view) {
            state.counters.preparations += 1;
        }
        if !base.matches(self.bundle.revision()) {
            return self.cancel(command, job, output, CancelReason::ReplacedRevision);
        }
        if output.charge(bytes.len() as u64).is_err() {
            let outcome = JobOutcome::CutOff(code::OUTPUT);
            let status = Status::Cancelled(CancelReason::Budget);
            return self.settle_result(view, command, job, outcome, status, output);
        }
        let covered = self.next_covered;
        self.next_covered += 1;
        let digest = sha256(&bytes);
        let Some(state) = self.views.get_mut(view) else { return };
        if remember {
            state.remember(base.inputs(), content.clone());
        }
        state.installed = Some(Installed { command, base, digest, content, covered, provenance });
        state.counters.installs += 1;
        state.failure = None;
        let completion = Completion {
            kind: CompletionKind::ViewPublished,
            revision: base.revision,
            result: ResultRef { digest, covered, job },
        };
        self.settle_result(view, command, job, JobOutcome::Completed, Status::Completed(completion), output);
    }

    fn fail(&mut self, view: &ViewId, command: CommandId, job: Option<JobId>, output: OutputAllowance, error: Error) {
        let status = Status::Failed(error.clone());
        self.settle_result(view, command, job, JobOutcome::Error(error), status, output);
    }

    fn settle_result(
        &mut self,
        view: &ViewId,
        command: CommandId,
        job: Option<JobId>,
        outcome: JobOutcome,
        status: Status,
        output: OutputAllowance,
    ) {
        let failure = match (&outcome, &status) {
            (JobOutcome::Error(error), _) => Some(error.clone()),
            (JobOutcome::CutOff(cut), _) => Some(Error::stop(cut, Context::None)),
            _ => None,
        };
        if self.settle(command, status) {
            self.results.insert(command, JobResult { command, job, outcome, output });
            if let (Some(error), Some(state)) = (failure, self.views.get_mut(view)) {
                state.failure = Some((command, error));
            }
        }
    }

    fn cancel(&mut self, command: CommandId, job: Option<JobId>, output: OutputAllowance, reason: CancelReason) {
        if self.settle(command, Status::Cancelled(reason)) {
            self.results.insert(command, JobResult { command, job, outcome: JobOutcome::Cancelled, output });
        }
    }

    /// KN5.21: the first serialized terminal wins; later ones change nothing.
    fn settle(&mut self, command: CommandId, status: Status) -> bool {
        if self.receipts.get(&command).is_some_and(Status::is_terminal) {
            return false;
        }
        self.receipts.insert(command, status);
        true
    }

    /// Cancels the view's pending request and fences its runner. Returns true when anything was cancelled.
    fn cancel_lane(&mut self, view: &ViewId, reason: CancelReason) -> bool {
        let Some(lane) = self.lanes.get_mut(view) else { return false };
        let mut cancelled = Vec::new();
        if let Some(request) = lane.pending.take() {
            cancelled.push((request.command(), None, request.output()));
        }
        if let Some(runner) = lane.runner.as_mut() {
            if !runner.fenced {
                runner.fenced = true;
                cancelled.push((runner.command, Some(runner.job), runner.output));
            }
        }
        let any = !cancelled.is_empty();
        for (command, job, output) in cancelled {
            self.cancel(command, job, output, reason);
        }
        self.release_untaken(view);
        any
    }

    /// A fenced runner that no runner took yet did no work, so its slot is released at once.
    fn release_untaken(&mut self, view: &ViewId) {
        let Some(lane) = self.lanes.get_mut(view) else { return };
        if !matches!(&lane.runner, Some(runner) if runner.fenced && !runner.taken) {
            return;
        }
        if let Some(runner) = lane.runner.take() {
            self.jobs.remove(&runner.job);
            self.ready.retain(|job| *job != runner.job);
        }
    }

    fn mint(&mut self) -> CommandId {
        let command = CommandId::Scheduled(self.next_scheduled);
        self.next_scheduled += 1;
        command
    }

    fn retire(&mut self, workload: &Workload) -> RetireToken {
        let token = RetireToken(self.next_token);
        self.next_token += 1;
        self.retiring.insert(token, (workload.cpu_bytes, workload.gpu_bytes));
        token
    }

    // ----- the view budget (RM.38, RM.39, RPI.82)

    /// The view budget in use now.
    pub fn usage(&self) -> Usage {
        let overdraw = self.profile.get(Field::Overdraw);
        let mut usage = Usage::default();
        for state in self.views.values() {
            if !state.evicted {
                usage.adapter_cpu_bytes += u128::from(state.declared.workload.cpu_bytes);
                usage.adapter_gpu_bytes += u128::from(state.declared.workload.gpu_bytes);
            }
            if state.visible {
                usage.view_vertex_evals += u128::from(state.declared.workload.vertex_evals);
                if !state.fallback {
                    usage.frame_work += state.frame_work(overdraw);
                }
            }
        }
        for (cpu, gpu) in self.retiring.values() {
            usage.adapter_cpu_bytes += u128::from(*cpu);
            usage.adapter_gpu_bytes += u128::from(*gpu);
        }
        usage
    }

    /// The usage after `state` becomes visible with its user stage; `fresh` adds its resident bytes as well.
    fn usage_with(&self, state: &ViewState, fresh: bool) -> Usage {
        let mut usage = self.usage();
        let work = &state.declared.workload;
        usage.view_vertex_evals += u128::from(work.vertex_evals);
        usage.frame_work += state.frame_work(self.profile.get(Field::Overdraw));
        if fresh {
            usage.adapter_cpu_bytes += u128::from(work.cpu_bytes);
            usage.adapter_gpu_bytes += u128::from(work.gpu_bytes);
        }
        usage
    }

    /// RM.39: the first excess in RM.11's table order is refused (draft order).
    fn check_view_usage(&self, usage: Usage) -> Result<(), Error> {
        self.profile.check_all(
            &[
                (Field::FrameWork, usage.frame_work),
                (Field::ViewVertexEvals, usage.view_vertex_evals),
                (Field::AdapterCpuBytes, usage.adapter_cpu_bytes),
                (Field::AdapterGpuBytes, usage.adapter_gpu_bytes),
            ],
            Form::Refusal,
        )
    }

    // ----- reading the model

    /// The session's profile (LG3.9).
    pub fn session_profile(&self) -> Profile {
        self.session
    }

    /// The fixed admission ceilings in use.
    pub fn profile(&self) -> &LimitProfile {
        &self.profile
    }

    /// The adopted revision.
    pub fn adopted(&self) -> &Revision {
        self.bundle.revision()
    }

    /// The adopted bundle.
    pub fn bundle(&self) -> &CompleteBundle {
        &self.bundle
    }

    /// The open views in open order, Global and Local first.
    pub fn views(&self) -> &[ViewId] {
        &self.order
    }

    /// The revision a view shows: the adopted bundle's for Global and Local (ONE.65), the installed output's base for
    /// a further view.
    pub fn shown_revision(&self, view: &ViewId) -> Option<Revision> {
        let state = self.views.get(view)?;
        if state.source.is_none() {
            return Some(*self.bundle.revision());
        }
        state.installed.as_ref().map(|installed| installed.base.revision)
    }

    /// The view's status (RPI.13, charter D2).
    pub fn status(&self, view: &ViewId) -> Option<ViewStatus> {
        let state = self.views.get(view)?;
        let adopted = *self.bundle.revision();
        let lane = self.lanes.get(view);
        let pending = lane.and_then(|lane| lane.pending.as_ref()).map(Request::command);
        let runner = lane.and_then(|lane| lane.runner.as_ref());
        let newest = pending.or(runner.filter(|runner| !runner.fenced).map(|runner| runner.command));
        if let Some(installed) = &state.installed {
            if !installed.base.matches(&adopted) {
                return Some(ViewStatus::Stale { base: installed.base, adopted, refresh: newest });
            }
            if installed.content.is_empty() {
                return Some(ViewStatus::Empty { revision: adopted, base: Some(installed.base) });
            }
            return Some(ViewStatus::Current { base: installed.base, publication: installed.digest });
        }
        if let Some((command, error)) = &state.failure {
            return Some(ViewStatus::Error { command: *command, error: error.clone() });
        }
        if let (Some(waiting), Some(held)) = (pending, runner.filter(|runner| runner.fenced)) {
            return Some(ViewStatus::Busy { waiting, held_by: held.command });
        }
        if let Some(command) = newest {
            return Some(ViewStatus::Loading { command, revision: adopted });
        }
        Some(ViewStatus::Empty { revision: adopted, base: None })
    }

    /// The inputs of the next frame. Only output whose declared dependencies match the adopted revision is drawn;
    /// stale output appears only as its view's stale presentation (ONE.12, ONE.63, RPI.13).
    pub fn frame_input(&self) -> FrameInput {
        let revision = *self.bundle.revision();
        let views = self
            .order
            .iter()
            .filter_map(|view| {
                let state = self.views.get(view).filter(|state| state.visible)?;
                let draw = match &state.installed {
                    Some(installed) if installed.base.matches(&revision) => Draw::Output {
                        base: installed.base,
                        publication: installed.digest,
                        stage: state.stage_use(),
                        provenance: installed.provenance.clone(),
                    },
                    Some(installed) => Draw::Stale { base: installed.base },
                    None => Draw::Nothing,
                };
                Some(FrameView { view: view.clone(), generation: state.generation, bundle: view.is_fixed(), draw })
            })
            .collect();
        FrameInput { revision, bundle: *self.bundle.digest(), views }
    }

    /// The receipt status of a command.
    pub fn receipt(&self, command: CommandId) -> Option<&Status> {
        self.receipts.get(&command)
    }

    /// The settled job result of an accepted command (LI.28).
    pub fn job_result(&self, command: CommandId) -> Option<&JobResult> {
        self.results.get(&command)
    }

    /// The installed publication of a view.
    pub fn installed(&self, view: &ViewId) -> Option<&Installed> {
        self.views.get(view)?.installed.as_ref()
    }

    /// The source of a further view.
    pub fn source(&self, view: &ViewId) -> Option<&ViewSource> {
        self.views.get(view)?.source.as_ref()
    }

    /// The generation of an open view; a close and reopen changes it (RM.25).
    pub fn generation(&self, view: &ViewId) -> Option<u64> {
        self.views.get(view).map(|state| state.generation)
    }

    /// True when the view is open and visible.
    pub fn is_visible(&self, view: &ViewId) -> bool {
        self.views.get(view).is_some_and(|state| state.visible)
    }

    /// True when a resize made the view fall back to its built-in chain (RM.38).
    pub fn is_fallback(&self, view: &ViewId) -> bool {
        self.views.get(view).is_some_and(|state| state.fallback)
    }

    /// The `frame_work` charge of a view's user stage chain at its current surface, fallen back or not (RM.38).
    pub fn frame_work_of(&self, view: &ViewId) -> Option<u128> {
        self.views.get(view).map(|state| state.frame_work(self.profile.get(Field::Overdraw)))
    }

    /// The work counters of a view.
    pub fn counters(&self, view: &ViewId) -> Option<Counters> {
        self.views.get(view).map(|state| state.counters)
    }

    /// The command of the view lane's pending request.
    pub fn pending(&self, view: &ViewId) -> Option<CommandId> {
        self.lanes.get(view)?.pending.as_ref().map(Request::command)
    }

    /// The job and command holding the view lane's runner slot.
    pub fn runner(&self, view: &ViewId) -> Option<(JobId, CommandId)> {
        self.lanes.get(view)?.runner.as_ref().map(|runner| (runner.job, runner.command))
    }

    /// True when the job was fenced: its results can no longer publish (LI.23).
    pub fn is_fenced(&self, job: JobId) -> bool {
        let Some(view) = self.jobs.get(&job) else { return true };
        self.lanes
            .get(view)
            .and_then(|lane| lane.runner.as_ref())
            .is_none_or(|runner| runner.job != job || runner.fenced)
    }

    /// The number of resource sets awaiting safe retirement.
    pub fn retiring(&self) -> usize {
        self.retiring.len()
    }
}
