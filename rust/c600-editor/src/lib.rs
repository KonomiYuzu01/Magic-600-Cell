//! EV1's editor services for 600 Cell Script (LP4-1; chapter 04, EV1.1 to EV1.12).
//!
//! The services read source and checker data and run no user program or callback (EV1.9): the crate depends on
//! `c600-syntax`, `c600-check`, `c600-ir` and `c600-digest` only, with no path to the interpreter or the command layer
//! (`tests/dependencies.rs`). A shell may link it as a library; program execution stays outside the UI thread and off
//! the kernel-key dispatch path.
//!
//! - A [`Document`] is immutable bytes named by their SHA-256, with the units the host declared when it opened them
//!   (EV1.1, EV1.10). Every request names the digest it was built against, and a request or an edit for other bytes
//!   is refused with `EV1:stale.editor_source`. Every response carries a [`Subject`]: the source digest, the edition
//!   and the module context's digest.
//! - [`SourceRange`] is H7's: UTF-8 byte offsets on scalar boundaries, half-open, bound to the source digest.
//!   [`Document::range`], [`Document::offset`] and [`Document::host`] convert from and to the declared `utf8`, `utf16`
//!   or `scalar` units without rounding; a bad endpoint, undeclared units or unpinned bytes are
//!   `EV1:input.source_range_invalid` (EV1.10).
//! - [`parse`] gives the recovered structure, an outline and the diagnostics even for an incomplete draft (EV1.2).
//! - [`complete`] gives candidates from the recovered scope with their replace range, type, effects, identity and
//!   documentation reference (EV1.3).
//! - [`diagnose`] gives typed diagnostics with ranges and the static verdict (EV1.4).
//! - [`hover`] and [`definition`] resolve names in the digest-pinned dependency graph (EV1.5, EV1.6).
//! - [`rename`] proposes digest-bound edits and the check of the resulting draft; [`apply_edits`] applies them
//!   (EV1.6).
//! - Every diagnostic and refusal is an [`Envelope`]: the code, the service, a typed [`Reason`] and a typed
//!   [`NextAction`] (EV1.12). Handling never reads its summary.
//!
//! The API is plain owned data (strings, vectors, enums, byte offsets) so that a C ABI host library (LP4-3) can wrap
//! it. `rust/README.md` lists what is not built.

#![forbid(unsafe_code)]

mod analysis;
mod complete;
mod context;
mod diagnose;
mod document;
mod envelope;
mod index;
mod navigate;
mod parse;
mod rename;

pub use complete::{complete, Candidate, CandidateKind, Completion, CompletionContext, DeclIdentity, Missing};
pub use context::{Dependency, IdentityField, ModelIdentity, ModuleContext, Obligation, ObligationRole};
pub use diagnose::{diagnose, CheckedCandidate, Diagnosis, Stage, Verdict};
pub use document::{Document, HostRange, SourceRange, Subject, Units};
pub use envelope::{code, Envelope, Form, IdentitySubject, NextAction, Reason, Retry, Service};
pub use index::NameKind;
pub use navigate::{
    definition, hover, Because, Exactness, Hover, HoverField, Inferred, Permission, Resolved, StaticCheck, Unresolved,
};
pub use parse::{parse, OutlineItem, OutlineKind, Parse};
pub use rename::{apply_edits, rename, Edit, Rename};
