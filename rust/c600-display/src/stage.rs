//! The headless part of the user-stage contract (RPI.68, RPI.69, RPI.75, RPI.76; RM.26; rows F05 and F26): a stage's
//! declared outputs, the frame's composition order and the presented-preview identity rule, over fixture stage
//! descriptions. A description stands for what the stage checker would read from a checked stage module. The parser and
//! checker of a module's body, HLSL compilation (RPI.78), the raster bound (RPI.72), quarantine (RPI.77) and every
//! device step are not built, and the support report still names F05 and F26 unsupported, so no user stage installs.

use crate::error::{codes, Error, Form};
use crate::pick::{Presented, PreviewShown};
use crate::plan::StageKind;

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

/// A fixture description of a stage: its kind, digest and declared outputs.
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
