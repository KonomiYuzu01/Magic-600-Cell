//! Errors of the view model: chapter 13's registry codes with their contextual form (H15.9, LI.28).
//!
//! Every code here is an existing registry code. A standalone request that fails validation gets the `refusal` form
//! before acceptance; an accepted scheduled invocation whose final validation fails gets the `stop` form inside its
//! job's `error(Error)`, which LI.28 maps to `failed(Error)` (RM.12, RM.20, RM.35).

use crate::bundle::PartKind;
use crate::revision::{Base, Revision};

/// The registry codes this crate emits (chapter 13).
pub mod code {
    /// `view.open` names an open view (RPI.83).
    pub const VIEW_ID_IN_USE: &str = "EV3:input.view_id_in_use";
    /// `view.close` names an unknown view, Global or Local (RPI.83).
    pub const VIEW_NOT_CLOSABLE: &str = "EV3:input.view_not_closable";
    /// A fixed plan ceiling, or RM.39's view ceilings at open or show (RM.12, RM.39, RPI.82).
    pub const DISPLAY_PLAN_CEILING: &str = "EV3:resource_limit.display_plan_ceiling";
    /// An incomplete or mismatched engine bundle; install none and refresh (RPI.9, ONE.60).
    pub const DISPLAY_BUNDLE_MISMATCH: &str = "EV3:input.display_bundle_mismatch";
    /// A malformed publication: duplicate IDs or a dangling source-map reference (RM.20).
    pub const DISPLAY_PLAN_INVALID: &str = "EV3:input.display_plan_invalid";
    /// A publication that no longer matches its target view (RPI.10, RPI.12).
    pub const STALE_DISPLAY_PLAN: &str = "EV3:stale.display_plan";
    /// The declared output allowance ends (KN7.10, RM.13, LI.28).
    pub const OUTPUT: &str = "KN7:budget.output";
    /// Requested authority is absent: a recomputation names `view.open` or `view.close` (draft, RPI.83).
    pub const NOT_GRANTED: &str = "LG3:capability.not_granted";
}

/// The contextual form of an error occurrence (H15.9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    /// Before acceptance: the request is `refused(Error)` and changes nothing.
    Refusal,
    /// After acceptance: the job ends with `error(Error)` or `cut_off(code)`.
    Stop,
}

/// Why an engine bundle is not one complete revision (RPI.9, ONE.60).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BundleFault {
    /// A required part is absent.
    Missing(PartKind),
    /// A part kind occurs twice.
    Duplicate(PartKind),
    /// A part kind the bundle's mode does not use.
    Unexpected(PartKind),
    /// A part belongs to another revision than the bundle.
    Mismatched { kind: PartKind, part: Revision, bundle: Revision },
    /// The bundle is older than the adopted one (an older engine epoch, or a lower counter in the adopted epoch), or
    /// reuses its epoch and counters with other content.
    OutOfOrder { adopted: Revision, candidate: Revision },
}

/// Why a publication is malformed (RM.20).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanFault {
    /// Two plans share a `DisplayPlanId`.
    DuplicatePlan(u64),
    /// Two source maps share a `SourceMapId`.
    DuplicateMap(u64),
    /// A plan names a source map the publication does not carry.
    DanglingMap { plan: u64, map: u64 },
}

/// Typed context of an error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Context {
    /// No further context.
    None,
    /// The view the request named.
    View(String),
    /// RM.12: the field, the requested count and the ceiling.
    Ceiling { field: &'static str, requested: u128, ceiling: u128 },
    /// RPI.9: what makes the bundle incomplete.
    Bundle(BundleFault),
    /// RM.20: what makes the publication malformed.
    Plan(PlanFault),
    /// RPI.10: the publication's base no longer matches the adopted revision.
    Stale { base: Base, adopted: Revision },
    /// RPI.10, RM.20: the publication names another generation of its target view.
    Generation { requested: u64, current: Option<u64> },
    /// RM.13: canonical bytes required and the allowance that remained.
    Output { required: u64, remaining: u64 },
}

/// An error with its registry code, form and context.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub code: &'static str,
    pub form: Form,
    pub context: Context,
}

impl Error {
    /// A refusal before acceptance.
    pub fn refusal(code: &'static str, context: Context) -> Error {
        Error { code, form: Form::Refusal, context }
    }

    /// A stop after acceptance.
    pub fn stop(code: &'static str, context: Context) -> Error {
        Error { code, form: Form::Stop, context }
    }

    /// The same code and context in another form (LI.28's contextual forms).
    pub fn with_form(mut self, form: Form) -> Error {
        self.form = form;
        self
    }
}
