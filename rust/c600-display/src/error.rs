//! The error envelope subset of ERR.8/ERR.9 that the display contract uses, and the codes of chapter 10 section 10
//! (RPI.64) it emits. Tests assert the code (ERR.6); the summary is for a reader.

use std::collections::BTreeMap;
use std::fmt;

/// The codes this crate emits or names.
pub mod codes {
    pub const PLAN_INVALID: &str = "EV3:input.display_plan_invalid";
    pub const UNSUPPORTED: &str = "EV3:input.display_unsupported";
    pub const STALE_PICK: &str = "EV3:stale.display_pick";
    pub const STALE_PLAN: &str = "EV3:stale.display_plan";
    pub const PLAN_CEILING: &str = "EV3:resource_limit.display_plan_ceiling";
    /// Reserved for a separately declared producer plan budget (RM.13, ERR.25); this crate never emits it.
    pub const PLAN_LIMIT: &str = "KN7:budget.display_plan_limit";
    pub const OUTPUT: &str = "KN7:budget.output";
    pub const HOST_EXHAUSTED: &str = "KN7:resource_limit.host_exhausted";
    pub const STALE_SOURCE: &str = "EV3:stale.source_link";
    pub const BUNDLE_MISMATCH: &str = "EV3:input.display_bundle_mismatch";
    pub const RENDERER_UNAVAILABLE: &str = "EV3:busy.renderer_unavailable";
    pub const VIEW_ID_IN_USE: &str = "EV3:input.view_id_in_use";
    pub const COMMAND_ID_REUSED: &str = "KN5:input.command_id_reused";
    pub const EXPOSED_GEOMETRY_UNAVAILABLE: &str = "KN2:input.exposed_geometry_unavailable";
    pub const FIXED_NU_NONZERO: &str = "KN2:consistency.fixed_nu_nonzero";
    /// RPI.48: a required chart property or precision check failed; the checked status is withheld.
    pub const CHART_CHECK_FAILED: &str = "EV3:evidence.chart_check_failed";
    /// LB4.2, RPI.30: approximate data offered to an exact chart-property check (03-libraries, LB4's catalogue).
    pub const APPROXIMATE_PROPERTY_INPUT: &str = "LB4:type.approximate_property_input";
    /// LG2.24, RPI.69: a stage outside the stage rules; its registered form is `diagnostic`.
    pub const STAGE_RULE_VIOLATION: &str = "LG2:type.stage_rule_violation";
    /// KN1.8, KN1.10, RPI.23: a slot projection asked for a subject at a level it does not hold. A digest of one level
    /// never compares with a digest of another and nothing lifts labels into poses, so the slot is missing with this
    /// reason, distinct from `off_lattice` (chapter 13: "distinct from partial off-lattice data").
    pub const LEVEL_NOT_HELD: &str = "KN1:input.level_not_held";
}

/// The `form` field of ERR.8.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    Diagnostic,
    Refusal,
    Stop,
    Freeze,
    Status,
}

/// An error: code, form, context and summary of ERR.8. The other envelope fields belong to the command layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub code: &'static str,
    pub form: Form,
    pub context: BTreeMap<&'static str, String>,
    pub summary: String,
}

impl Error {
    /// An error in form `refusal`; a code registered with another form gets it from [`Error::in_form`] where it is
    /// raised.
    pub fn new(code: &'static str, summary: impl Into<String>) -> Error {
        Error { code, form: Form::Refusal, context: BTreeMap::new(), summary: summary.into() }
    }

    /// `EV3:input.display_plan_invalid`.
    pub fn invalid(summary: impl Into<String>) -> Error {
        Error::new(codes::PLAN_INVALID, summary)
    }

    /// `EV3:input.display_unsupported`, naming the feature row or stage.
    pub fn unsupported(feature: &str, summary: impl Into<String>) -> Error {
        Error::new(codes::UNSUPPORTED, summary).with("feature", feature)
    }

    /// `EV3:resource_limit.display_plan_ceiling` with RM.12's field, requested count and ceiling.
    pub fn ceiling(field: &'static str, requested: u64, ceiling: u64) -> Error {
        Error::new(codes::PLAN_CEILING, format!("{field}: {requested} is over the ceiling {ceiling}"))
            .with("field", field)
            .with("requested", requested)
            .with("ceiling", ceiling)
    }

    /// `EV3:busy.renderer_unavailable`.
    pub fn unavailable(summary: impl Into<String>) -> Error {
        Error::new(codes::RENDERER_UNAVAILABLE, summary)
    }

    pub fn with(mut self, key: &'static str, value: impl ToString) -> Error {
        self.context.insert(key, value.to_string());
        self
    }

    /// The same error in another form, as H15.9's contextual forms allow for an existing code.
    pub fn in_form(mut self, form: Form) -> Error {
        self.form = form;
        self
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.summary)
    }
}

impl std::error::Error for Error {}
