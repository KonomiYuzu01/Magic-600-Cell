//! The headless part of the user-stage contract (RPI.68 to RPI.77; RM.26, RM.41; rows F05 and F26).
//!
//! The display takes a stage up through LP2's stage checker: [`take_up`] reads a [`StageDescription`] from a stage that
//! `c600_check::admit_stage` admitted, and returns each refusal of the checker with the checker's own code. The
//! checker's entry rule guards a stage's outputs; [`check_outputs`] is the display's own output rule on a description,
//! the rule that `c600-repr`'s chart path also calls on descriptions of its own until it reads admitted stages.
//! [`compose`] orders a frame's chain, preview and host statuses under the presented-preview identity rule.
//! [`Quarantine`] keeps RPI.72's frame stop, RPI.77's quarantine and RM.41's frame reports over a raster bound that
//! [`crate::raster`] computes from evaluated positions.
//!
//! Not here, since they need a backend (C9): HLSL compilation and its compile record (RPI.78), the vertex evaluation,
//! projection and raster bound on a device and the skipping of a stopped frame's work (RPI.72), device faults
//! (RPI.77) and composition on a device. The support report still names F05 and F26 unsupported, so no user stage
//! installs in [`crate::host::Host`], and [`Quarantine`] is not wired into its publications or frames.

use std::collections::BTreeSet;

use c600_digest::sha256_hex;

use crate::error::{codes, Error, Form};
use crate::pick::{FrameStamp, Presented, PreviewShown};
use crate::plan::{Primitive, Publication, StageKind, ViewId};

/// An output a stage may declare.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Output {
    /// A vertex stage's position.
    Position,
    /// A vertex stage's typed coverage code where the map is undefined (RPI.69).
    CoverageCode,
    /// A fragment stage's opaque colour.
    Rgb,
    /// Transparency, a later row (RPI.75).
    Alpha,
    Discard,
    Depth,
    Stencil,
    /// A change to the sample coverage mask.
    Coverage,
}

impl Output {
    pub fn name(self) -> &'static str {
        match self {
            Output::Position => "position",
            Output::CoverageCode => "coverage_code",
            Output::Rgb => "rgb",
            Output::Alpha => "alpha",
            Output::Discard => "discard",
            Output::Depth => "depth",
            Output::Stencil => "stencil",
            Output::Coverage => "coverage",
        }
    }
}

/// A description of a stage: its kind, digest and declared outputs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StageDescription {
    pub kind: StageKind,
    pub digest: String,
    pub outputs: Vec<Output>,
}

/// A stage whose declared outputs passed [`check_outputs`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckedStage {
    kind: StageKind,
    digest: String,
}

impl CheckedStage {
    pub fn kind(&self) -> StageKind {
        self.kind
    }

    pub fn digest(&self) -> &str {
        &self.digest
    }
}

/// `LG2:type.stage_rule_violation` in its registered form, a diagnostic (chapter 13; CT-ERR-17).
fn violation(summary: String) -> Error {
    Error::new(codes::STAGE_RULE_VIOLATION, summary).in_form(Form::Diagnostic)
}

/// The output rule. A vertex stage returns a position or a coverage code and declares exactly those two (RPI.68,
/// RPI.69); a fragment stage writes opaque RGB only, with no discard and no depth, stencil, coverage or alpha output
/// (RPI.75). Anything else, a repeated output or a digest that is not 64 lowercase hex digits is the diagnostic
/// `LG2:type.stage_rule_violation`, before anything reaches a device.
pub fn check_outputs(stage: &StageDescription) -> Result<CheckedStage, Error> {
    let (kind, allowed): (&str, &[Output]) = match stage.kind {
        StageKind::Vertex => ("vertex", &[Output::Position, Output::CoverageCode]),
        StageKind::Fragment => ("fragment", &[Output::Rgb]),
    };
    let hex =
        stage.digest.len() == 64 && stage.digest.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    if !hex {
        return Err(violation(format!("a {kind} stage needs a SHA-256 digest")));
    }
    for (i, output) in stage.outputs.iter().enumerate() {
        if !allowed.contains(output) {
            return Err(violation(format!("a {kind} stage cannot declare a {} output", output.name()))
                .with("output", output.name()));
        }
        if stage.outputs[..i].contains(output) {
            return Err(violation(format!("a {kind} stage declares its {} output twice", output.name()))
                .with("output", output.name()));
        }
    }
    if let Some(output) = allowed.iter().find(|o| !stage.outputs.contains(o)) {
        return Err(violation(format!("a {kind} stage must declare its {} output", output.name()))
            .with("output", output.name()));
    }
    Ok(CheckedStage { kind: stage.kind, digest: stage.digest.clone() })
}

/// The description of a stage that LP2's stage checker admitted (RPI.68). Its outputs are those the checker's entry
/// rule leaves: a vertex entry returns `record { x, y, z }` of binary32 and, where its map is undefined, a coverage
/// code (`nonfinite`, or `singular_neighbourhood` from `exclude`); a fragment entry returns `record { r, g, b }` of
/// binary32 and nothing else (RPI.75).
///
/// The digest is a draft: the SHA-256 of the module name, the entry name and the checked definition's Rust debug text,
/// one per line. It stands in for a digest of the IR's canonical encoding, which LP2 does not have before the IR
/// freeze (PHS.49), and it never depends on the host's limits.
pub fn describe(stage: &c600_check::CheckedStage) -> StageDescription {
    let (kind, outputs) = match stage.kind() {
        c600_ir::StageKind::Vertex => (StageKind::Vertex, vec![Output::Position, Output::CoverageCode]),
        c600_ir::StageKind::Fragment => (StageKind::Fragment, vec![Output::Rgb]),
    };
    let text = format!("{}\n{}\n{:?}", stage.module(), stage.entry(), stage.definition());
    StageDescription { kind, digest: sha256_hex(text.as_bytes()), outputs }
}

/// A finding of the stage checker as a display error with the checker's code, in that code's registered form
/// (`refusal` for `LG4:input.ir_version_unsupported` and the checker's two LG3 capability codes, `diagnostic` for the
/// others), naming the module, the function, if any, and the path, with the expected and supplied types of a mismatch.
fn finding(diagnostic: &c600_check::Diagnostic) -> Error {
    let refusals = [c600_check::IR_VERSION_UNSUPPORTED, c600_check::NOT_GRANTED, c600_check::AMBIENT_READ];
    let form = if refusals.contains(&diagnostic.code) { Form::Refusal } else { Form::Diagnostic };
    let mut error = Error::new(diagnostic.code, diagnostic.note.clone())
        .in_form(form)
        .with("module", &diagnostic.module)
        .with("path", &diagnostic.path);
    if !diagnostic.function.is_empty() {
        error = error.with("function", &diagnostic.function);
    }
    if let Some(mismatch) = &diagnostic.mismatch {
        error = error.with("expected", &mismatch.expected).with("supplied", &mismatch.supplied);
    }
    error
}

/// The display's take-up of the stage checker's answer (RPI.68, RPI.69). A stage the checker admitted with no finding
/// becomes a [`CheckedStage`] through its [`describe`]d outputs. Otherwise every finding comes back, in the checker's
/// order, with the checker's code ([`finding`]); nothing of the stage reaches the display.
pub fn take_up(admission: &c600_check::StageAdmission) -> Result<CheckedStage, Vec<Error>> {
    match &admission.stage {
        // Defensive only: `describe` sets the outputs from the stage kind, so `check_outputs` cannot refuse them
        // here. The checker's entry rule, which admitted the stage, is the real guard on what a stage returns.
        Some(stage) if admission.diagnostics.is_empty() => check_outputs(&describe(stage)).map_err(|e| vec![e]),
        _ => Err(admission.diagnostics.iter().map(finding).collect()),
    }
}

/// A host status, in RPI.76's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum HostStatus {
    Preview,
    Stale,
    ProtectionBreakthrough,
    Selection,
}

/// What draws a plan or a preview: the built-in chain, or checked user stages in chain order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Chain {
    BuiltIn,
    User(Vec<CheckedStage>),
}

impl Chain {
    /// Every user-stage digest of the chain, once each, in chain order.
    pub fn digests(&self) -> Vec<&str> {
        let mut digests: Vec<&str> = Vec::new();
        if let Chain::User(stages) = self {
            for stage in stages {
                if !digests.contains(&stage.digest()) {
                    digests.push(stage.digest());
                }
            }
        }
        digests
    }
}

/// One pass of a composed frame, in drawing order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Layer {
    BuiltIn,
    /// A user stage, by digest.
    User(String),
    /// An engine preview drawn by the built-in chain; the presented report names it.
    Preview(PreviewShown),
    /// A preview drawn through user stages: an extra display with no presented-preview identity (RPI.76).
    ExtraPreview,
    Status(HostStatus),
}

/// A composed frame and its presented-preview report (RM.26).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Composition {
    pub layers: Vec<Layer>,
    pub presented: Presented,
}

impl Composition {
    /// The preview a commit token may bind to (ONE.41): only one the presented report names.
    pub fn commit_binding(&self) -> Option<&PreviewShown> {
        match &self.presented {
            Presented::Previews(previews) => previews.first(),
            Presented::NoPreview => None,
        }
    }
}

fn passes(chain: &Chain) -> Vec<Layer> {
    match chain {
        Chain::BuiltIn => vec![Layer::BuiltIn],
        Chain::User(stages) => stages.iter().map(|s| Layer::User(s.digest.clone())).collect(),
    }
}

/// Composes a frame: the view's chain, then the preview by the chain that draws it, then the host statuses in RPI.76's
/// order, after every user stage. Only a preview drawn by the built-in chain has presented-preview identity; one drawn
/// through a user stage is an extra display that the report does not name and a commit cannot bind to (RPI.76).
pub fn compose(view: &Chain, preview: Option<(&PreviewShown, &Chain)>, statuses: &[HostStatus]) -> Composition {
    let mut layers = passes(view);
    let presented = match preview {
        Some((shown, Chain::BuiltIn)) => {
            layers.push(Layer::Preview(shown.clone()));
            Presented::Previews(vec![shown.clone()])
        }
        Some((_, chain)) => {
            layers.extend(passes(chain));
            layers.push(Layer::ExtraPreview);
            Presented::NoPreview
        }
        None => Presented::NoPreview,
    };
    let mut statuses = statuses.to_vec();
    statuses.sort();
    statuses.dedup();
    layers.extend(statuses.into_iter().map(Layer::Status));
    Composition { layers, presented }
}

/// What a view shows in place of a stopped or faulted user-stage chain (RPI.77, RM.41).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fallback {
    /// The built-in chain, with visible status.
    BuiltInChain,
    /// RPI.13's error status, for a plan with no built-in chain.
    ErrorStatus,
}

/// RM.41's `FrameReport`: a frame that stopped a user stage. Its error's form is `status`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameReport {
    pub stamp: FrameStamp,
    pub view: ViewId,
    pub publication_generation: u64,
    pub error: Error,
    pub fallback: Fallback,
}

/// A frame's outcome under RPI.72's raster bound and RPI.77's quarantine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameOutcome {
    /// The frame draws its chain.
    Runs,
    /// No user-stage rasterization or fragment work of the frame runs; the view shows the fallback.
    Stopped(Fallback),
}

/// One session's stage quarantine and its frame reports (RPI.72 step 4, RPI.77, RM.41). The host keeps one per
/// session; quarantine lasts for the session and nothing lifts it.
///
/// Drafts: the context of `EV3:resource_limit.stage_work_exceeded` names the chain's digests in one entry, `digests`,
/// comma-separated in chain order. RM.41 reports each frame that stopped a user stage. A frame whose chain names a
/// digest that is already quarantined runs the fallback, so it runs no user stage and stops none, and gets no report.
/// A device fault gets no report: RM.41's report needs an error, and the registry names no code for a device fault.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Quarantine {
    digests: BTreeSet<String>,
    reports: Vec<FrameReport>,
}

fn fallback(built_in: bool) -> Fallback {
    if built_in {
        Fallback::BuiltInChain
    } else {
        Fallback::ErrorStatus
    }
}

impl Quarantine {
    pub fn new() -> Quarantine {
        Quarantine::default()
    }

    /// True when `digest` is quarantined in this session.
    pub fn is_quarantined(&self, digest: &str) -> bool {
        self.digests.contains(digest)
    }

    /// RPI.72 step 4 for the frame `stamp` of a view drawn by `chain`, with the raster bound U that the adapter
    /// computed before any fragment work and the frame's allowance, surface pixels times `overdraw`
    /// ([`crate::raster`]). `built_in` says whether the plan has a built-in chain to fall back to.
    ///
    /// A frame of the built-in chain runs: the bound applies to frames with a user stage in the chain. A frame whose U
    /// is at most the allowance runs. Above it, no user-stage work of the frame runs: every user-stage digest of the
    /// chain is quarantined, the view falls back, and a frame report with `EV3:resource_limit.stage_work_exceeded` in
    /// form `status` names the digests, the bound and the allowance. No receipt changes (RM.41).
    pub fn frame(
        &mut self,
        stamp: &FrameStamp,
        chain: &Chain,
        built_in: bool,
        bound: u128,
        allowance: u128,
    ) -> FrameOutcome {
        let digests = chain.digests();
        if digests.is_empty() {
            return FrameOutcome::Runs;
        }
        if digests.iter().any(|d| self.is_quarantined(d)) {
            return FrameOutcome::Stopped(fallback(built_in));
        }
        if bound <= allowance {
            return FrameOutcome::Runs;
        }
        let error = Error::new(
            codes::STAGE_WORK_EXCEEDED,
            format!("the frame's raster bound {bound} is over its allowance {allowance}"),
        )
        .in_form(Form::Status)
        .with("digests", digests.join(","))
        .with("bound", bound)
        .with("allowance", allowance);
        self.digests.extend(digests.iter().map(|d| d.to_string()));
        self.reports.push(FrameReport {
            stamp: stamp.clone(),
            view: stamp.view,
            publication_generation: stamp.publication_generation,
            error,
            fallback: fallback(built_in),
        });
        FrameOutcome::Stopped(fallback(built_in))
    }

    /// RPI.77: a device fault in a frame that used a user stage quarantines every user-stage digest of its expanded
    /// chain, and the view falls back. Detecting the fault is the backend's.
    pub fn device_fault(&mut self, chain: &Chain, built_in: bool) -> FrameOutcome {
        let digests = chain.digests();
        if digests.is_empty() {
            return FrameOutcome::Runs;
        }
        self.digests.extend(digests.iter().map(|d| d.to_string()));
        FrameOutcome::Stopped(fallback(built_in))
    }

    /// RPI.77, RM.20: a publication whose user-stage row names a quarantined digest ends with
    /// `EV3:input.stage_quarantined` in `form`, a refusal before acceptance and a stop after it, naming the plan and
    /// the digest. Rows are read in plan and primitive order; the first quarantined digest is named.
    pub fn admit(&self, publication: &Publication, form: Form) -> Result<(), Error> {
        for plan in &publication.plans {
            for primitive in &plan.primitives {
                let Primitive::UserStage(stage) = primitive else { continue };
                if self.is_quarantined(&stage.digest) {
                    let summary = format!("plan {} names a stage quarantined in this session", plan.plan.0);
                    return Err(Error::new(codes::STAGE_QUARANTINED, summary)
                        .in_form(form)
                        .with("plan", plan.plan.0)
                        .with("digest", &stage.digest));
                }
            }
        }
        Ok(())
    }

    /// RM.41's `read_frame_reports`: each frame that stopped a user stage since the last read, in frame order.
    pub fn read_frame_reports(&mut self) -> Vec<FrameReport> {
        std::mem::take(&mut self.reports)
    }
}
