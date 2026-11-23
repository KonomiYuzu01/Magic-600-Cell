//! Typed diagnostic and refusal data (EV1.4, EV1.12): chapter 13's envelope (ERR.8 to ERR.12) with what happened (the
//! service and the code), why (a typed [`Reason`]) and an available next action (a typed [`NextAction`]). A host
//! decides by `code`, `reason` and `next`, never by `summary` (ERR.6); final messages wait for the design line.

use c600_syntax::RawContext;

use crate::context::{IdentityField, ObligationRole};
use crate::document::{SourceRange, Units};

/// The codes the services raise or forward. Forwarded codes keep their own prefix (ERR.7): a syntax diagnostic stays
/// `LG1`, a checker finding `LG2` or `LG3`.
pub mod code {
    pub use c600_check::{
        AMBIENT_READ, EFFECT_UNDECLARED, EXACTNESS_VIOLATION, EXACT_REQUIRED, NOT_GRANTED, TYPE_MISMATCH,
    };
    pub use c600_syntax::code::{
        EDITION_MISSING, EDITION_UNSUPPORTED, INVALID_ENCODING, INVALID_LITERAL, RECOVERY_GAP, UNEXPECTED_TOKEN,
    };

    /// `EV1:stale.editor_source`: a request or an edit names bytes that are no longer its target (EV1.1, EV1.6).
    pub const STALE_EDITOR_SOURCE: &str = "EV1:stale.editor_source";
    /// `EV1:input.source_range_invalid`: an endpoint, a host unit or a conversion is invalid (EV1.10).
    pub const SOURCE_RANGE_INVALID: &str = "EV1:input.source_range_invalid";
    /// `EV1:evidence.obligation_unmet`: a required claim or check is absent (EV1.4).
    pub const OBLIGATION_UNMET: &str = "EV1:evidence.obligation_unmet";
    /// `KN1:input.identity_mismatch`: a model, menu, order digest or contract revision differs (KN1.4, LG5.5).
    pub const IDENTITY_MISMATCH: &str = "KN1:input.identity_mismatch";
    /// `LG4:input.ir_rejected`: lowering refuses the draft, as `c600-module` reports a lowering error (LG5.11).
    pub const IR_REJECTED: &str = "LG4:input.ir_rejected";
    /// `LG5:input.module_dependency_missing`: an import names content the module context does not hold (LG5.3).
    pub const DEPENDENCY_MISSING: &str = "LG5:input.module_dependency_missing";
}

/// Where a code occurs (ERR.9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    /// Attached to source; the diagnosed draft did not run.
    Diagnostic,
    /// Returned to the caller; the refused request changed nothing.
    Refusal,
}

/// Chapter 13's retry values (ERR.8, ERR.11). Nothing retries automatically.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Retry {
    Never,
    AfterRefresh,
    AfterUserChange,
    AfterReconnect,
    AfterUserRequest,
}

/// The service that produced an envelope: what the host asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Service {
    /// Opening a document with its declared units.
    Open,
    /// A conversion between source ranges and host units (EV1.10).
    Range,
    Parse,
    Complete,
    Diagnose,
    Hover,
    Definition,
    Rename,
    /// Applying proposed edits to source bytes (EV1.6).
    ApplyEdits,
}

/// Whose identity differs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IdentitySubject {
    /// The draft's own manifest claims (LG5.2).
    Module,
    /// A pinned dependency's claims, reached through the import `alias`.
    Dependency { alias: String, digest: String },
}

/// Why: the typed facts of one envelope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reason {
    /// The request names bytes other than the document's (EV1.1, EV1.6).
    StaleSource { requested: String, held: String },
    /// A position lies outside the document.
    OutOfBounds { offset: usize, units: Units, length: usize },
    /// A position falls inside a scalar or a surrogate pair, and is never rounded (EV1.10).
    InsideScalar { offset: usize, units: Units },
    /// A range starts after it ends.
    Reversed { start: usize, end: usize },
    /// The host names units it did not declare when it opened the document, or no units this protocol knows.
    UndeclaredUnits { declared: Option<Units>, requested: String },
    /// The bytes are not pinned UTF-8/LF source, so they have no source range at all, not even an empty one (EV1.10;
    /// ERR.10's raw context).
    NotPinned { raw: RawContext },
    /// Two proposed edits overlap.
    OverlappingEdits { first: SourceRange, second: SourceRange },
    /// ERR.10's encoding failure: the raw input context and no span.
    Encoding { raw: RawContext },
    /// A syntax diagnostic of LG1, forwarded unchanged.
    Syntax,
    /// A recovered gap, which supplies no executable value (LG1.5, EV1.2).
    RecoveryGap,
    /// The request names an edition this parser does not read (LG1.2).
    EditionUnsupported { requested: String, supported: String },
    /// A checker finding, forwarded unchanged with its typed context (ERR.7): the IR position and, for exactness and
    /// type findings, the fields the checker reports.
    Check {
        module: String,
        function: String,
        path: String,
        value_type: Option<String>,
        display_control: bool,
        qualifier_erasure: bool,
        expected: Option<String>,
        supplied: Option<String>,
    },
    /// Lowering refused the draft (LG4.12): the IR draft cannot hold it, or a name does not resolve.
    Lowering { module: Option<String> },
    /// An import names content the module context does not hold (LG5.3). Nothing stands in for it.
    DependencyMissing { alias: String, digest: String },
    /// A model or menu identity differs (KN1.4, LG5.5): the first differing field, the session's value and the
    /// claimed value nearest to it.
    IdentityMismatch { subject: IdentitySubject, field: IdentityField, expected: Option<String>, held: Option<String> },
    /// The context requires a claim or check that the declaration does not supply (EV1.4); `declared` says whether the
    /// declaration exists.
    ObligationUnmet { declaration: String, role: ObligationRole, declared: bool },
}

/// An available next action, as data: the kind of action, never a choice made for the solver (ERR.13, ERR.15).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NextAction {
    /// Read the current bytes and send the request again.
    Refresh,
    /// Name a position or range on scalar boundaries inside the document.
    ChoosePosition,
    /// Name positions in the units declared when the document was opened, or open it with known units.
    UseDeclaredUnits,
    /// Save the source as UTF-8 with LF line ends and no byte-order mark.
    Reencode,
    /// Edit the source at the envelope's span.
    EditSource,
    /// Add the bytes with this digest to the module context.
    SupplyDependency { digest: String },
    /// Select a module context whose identity the module supports, or change the manifest's claims.
    SelectContext,
    /// Supply the required claim or check in the named declaration.
    SupplyObligation { declaration: String },
    /// Propose edits that do not overlap.
    SeparateEdits,
}

/// One diagnostic or refusal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Envelope {
    pub code: &'static str,
    pub form: Form,
    pub service: Service,
    pub reason: Reason,
    pub next: NextAction,
    /// The source range of a diagnostic, absent for ERR.10's encoding failure and for a refusal without a source
    /// position (ERR.10).
    pub span: Option<SourceRange>,
    /// A human account for display only. Handling never reads it (ERR.6).
    pub summary: String,
}

impl Envelope {
    pub(crate) fn new(code: &'static str, form: Form, service: Service, reason: Reason, next: NextAction) -> Envelope {
        Envelope { code, form, service, reason, next, span: None, summary: String::new() }
    }

    pub(crate) fn at(mut self, span: Option<SourceRange>) -> Envelope {
        self.span = span;
        self
    }

    pub(crate) fn saying(mut self, summary: impl Into<String>) -> Envelope {
        self.summary = summary.into();
        self
    }

    /// The code's category, the part between the prefix and the slug (ERR.2).
    pub fn category(&self) -> &'static str {
        let rest = self.code.split_once(':').map_or(self.code, |(_, rest)| rest);
        rest.split_once('.').map_or(rest, |(category, _)| category)
    }

    /// Whether a corrected or renewed request is possible (ERR.14). Every category these services raise is.
    pub fn recoverable(&self) -> bool {
        !matches!(self.category(), "consistency" | "unknown_outcome")
    }

    /// The category's default retry value (ERR.14): a stale binding after a refresh, every other category here after
    /// a change by the user.
    pub fn retry(&self) -> Retry {
        match self.category() {
            "stale" => Retry::AfterRefresh,
            "consistency" | "unknown_outcome" => Retry::Never,
            "auth" => Retry::AfterReconnect,
            "busy" | "cancelled" => Retry::AfterUserRequest,
            _ => Retry::AfterUserChange,
        }
    }
}
