//! Module closures, lockfiles and their approval binding for 600 Cell Script (LP3; chapter 02 LG5, chapter 05 PS1.14
//! and PS1.15, the format packet's FP.4, FP.5, FP.10 and FP.11).
//!
//! - [`Manifest`] and [`Lock`] are the stored manifest and lockfile (LG5.2, LG5.3, PS1.15). They are FP.11 records in
//!   FP.5's canonical JSON, under the identifiers FP.4 reserves, `c600-module-draft` and `c600-lock-draft`.
//! - [`resolve`] finds a manifest's dependency closure among local files, by content digest only, and writes its lock
//!   (LG5.3, LG5.9). [`verify`] checks a manifest, its lock and the local bytes and gives the [`Closure`]: every
//!   module's checked bytes, parsed and lowered, and the closure digest (LG5.1, LG5.3, LG5.5, LG5.7).
//! - [`approve`] checks the closure with `c600-check` for one entry, author, profile, grant and policy, refuses dynamic
//!   code first, and binds an [`Approval`] to the closure digest. This is RTE.45's `source_bound` variant for route RT1:
//!   the checked bytes are the executed bytes, over the transitive dependency closure (LG5.6).
//! - [`load`] refuses a closure whose digest differs from the approval's, so substituted dependency bytes never run
//!   (LG5.1, LG5.3, CT-RTE-08). [`Ready::run`] runs the approved entry through `c600-run`'s boundary
//!   ([`c600_run::boundary::Boundary`]) and keeps capability separation at the preview point (LG3.13, BND.39).
//!
//! Every refusal carries a stable code of chapter 13 ([`code`]). The crate reads and writes local bytes only: it opens
//! no file, network or process itself (LG5.9).
//!
//! # Drafts
//!
//! The forms below are the smallest drafts that CT-RTE-07 and CT-RTE-08 need. They freeze with the module formats,
//! which stay later freezes (LG5.10, LQ10); none of them is a frozen format.
//!
//! - **Manifest form** (FP.4, FP.11, PS1.15, LG5.2). `c600-module-draft` version 1: `{format, version, identity,
//!   payload, provenance}` with payload `{module, edition, source, dependencies, capabilities, boundary, principle,
//!   parameters, supports}`. `source` is the root source's `DigestRef(source)`; `dependencies` is the set of the root's
//!   direct imports `{alias, source}`; `capabilities` is the set of the header's `requires` names; `boundary` is a
//!   built-in's reviewed class or null; `principle` is a `DigestRef(content)` or null; `parameters` is always empty
//!   (declared parameters are not drafted, and a non-empty list is refused with `LG5:input.module_contract_mismatch`);
//!   `supports` is the set of KN0.1 identity blocks the module claims (LG5.2, LG5.5).
//! - **Identity null** (FP.11, LG5.2, LG5.5). A manifest and a lock are not model-dependent records themselves, so
//!   their FP.11 `identity` is null and the model identities sit in `payload.supports`. FP.11 allows null only where
//!   stated; this null is a draft gap for the format review. `provenance` is null: neither record is a word, macro or
//!   workspace record (FP.11). The draft forms carry no extensions: a required extension is refused with
//!   `PS2:input.persistence_version`, an optional one is kept in the bytes and has no effect (FP.10).
//! - **Locations in the lock only** (PS1.15, LG5.1, LG5.9). PS1.15 lists local resolution entries in both records. The
//!   manifest names its sources by digest only, and the lock holds every location, so moving a file changes the lock's
//!   bytes but never the closure digest. An approval survives a moved file: [`load`] compares the closure and manifest
//!   digests, not the lock's bytes.
//! - **No dependency manifest** (LG5.2, LG5.7). A dependency needs no manifest of its own: its source header declares
//!   its capabilities and `c600-check` checks them transitively against each importer and the grant.
//! - **Lock form** (FP.4, PS1.15, LG5.3). `c600-lock-draft` version 1, payload `{root, modules, locations, resolver}`.
//!   `root` is the manifest's `DigestRef(content)`; `modules` is the set `{module, source, edition, ir: {version,
//!   contract}}`; `locations` is the set `{path, source}`; `resolver` is `{contract: "c600-resolver-draft-1"}`.
//! - **Closure digest** (LG5.1, LG5.3, LG5.4). The SHA-256 of the canonical lock payload without `locations`: the
//!   root manifest, every module digest with its edition and IR contract, and the resolver contract. A changed module,
//!   edition, contract or resolver changes it; a moved file does not.
//! - **Approval record** (RTE.45, LG5.6). `c600-approval-draft` version 1 has no FP.4 row and is never stored: it
//!   lives in memory for one session. Its canonical bytes name the session identity, the closure, lock and manifest
//!   digests, every module digest, the entry, author, profile, grant, enabled declassifiers, checker and contract pins,
//!   the IR contract and the separation facts.
//! - **Dynamic code** (RTE.45, LG2.19, LG3.2). An unresolved call in any function of the closure is dynamic code. It is
//!   refused before any other finding with `LG3:capability.not_granted`, the nearest code: `policy.json` of LX-3 lists
//!   `dynamic_code` as an open code gap. The surface cannot write one, since lowering refuses an unbound name.
//! - **Built-in tools** (BND.37, BND.56). `c600-check::admit` checks the own-tool column only. A built-in's approval
//!   reads `c600-check::check`'s diagnostics, the same capability rows as `admit` (a never-granted or ungranted
//!   capability is `LG3:capability.not_granted`) and `c600-check::separation`, with no static separation refusal.
//!   Author `unknown` is never approved (BND.21).
//! - **Separation at the preview point** (LG3.13, BND.39, BND.42). In the solving profile a proposal whose block may
//!   read state is refused with `LG3:capability.not_granted` after the boundary's preview admission, which keeps its
//!   own codes; the program still gets the fixed acknowledgement `{base, id}`. The block reads state when the entry's
//!   static separation does (callees included), when the run read a snapshot, or when a stored part may. BND.39 binds
//!   the split; `c600-check` calls the refusal of a program that does both the flow rule (BND.33, BND.41), which stays
//!   off, and `LG3:capability.flow_refused` stays reserved (LX-3's `flow_code_reserved` gap). The refusal stays here
//!   as a draft until C5's host bridge, which decides whether the command layer's preview admission applies it
//!   instead (integrator, 16 November 2026).
//! - **Refusal codes** (LG5.11, ERR). A syntax diagnostic keeps its own `LG1` code; a lowering error is
//!   `LG4:input.ir_rejected`; a session identity outside `supports` is `LG5:input.model_not_supported`.
//! - **CT-RTE-07's canonical result** (RTE.46). [`Outcome::canonical`] writes the outcome, code, stop position, used
//!   amounts, output bytes, shell answers (refusal codes only, and whether the split refused) and captures, in FP.5's
//!   canonical JSON. Timing, host and path fields are not in it. A key's ten-run digest hashes these bytes for each of
//!   its runs (a refusal by its code), the engine side and the store.
//! - **CT-RTE-07's fixture mapping** (RTE.46, LX-3). The frozen keys use letter names and fixed-order lists. The
//!   tests map the letters `a, b, c, d, D` to `+1, +2, +3, +4, -4`, and read a negated letter `-n`, which is how the
//!   IR writes the inverse of the letter `n` (LG2.15), as the inverse `fixture.json` names for that letter (`a`, `b`
//!   and `c` are their own inverses), so A17's `-1, +4` is the corpus's `a d`. They map fixed-order lists to records
//!   with fields `c0..c2`, `i0..i2` and `s0..s3`, and the corpus's workspace rows to `read_snapshot` and
//!   `write_workspace`, which the boundary serves from its store. The corpus's `emit` maps to `publish_view`, the one
//!   output besides the result. A fixed-length list function (a sum, a first minimum, a table look-up) is unrolled
//!   into nested `if` and `exact_*` calls over the record's fields, since the IR draft has no loop; a loop over the
//!   corpus's fixed word list is unrolled over its literal words, and a row with a literal argument, such as `score`
//!   of the word `a`, is the query `score/a`. The synthetic engine charges each row its `fixture.json` cost. The IR
//!   draft has no network read or process call, so those two probes only declare the capability. The session
//!   identity is Full's KN0.1 block; the fixture data stay synthetic and are not Full's mechanics.

#![forbid(unsafe_code)]

mod approval;
mod closure;
mod form;
mod run;

pub use approval::{admit_program, approve, Admitted, Approval, Entry};
pub use c600_run::interp::Refusal;
pub use closure::{resolve, verify, Closure, Files};
pub use form::{Dependency, Identity, Location, Lock, LockedModule, Manifest};
pub use run::{load, Outcome, Ready, RunRequest};

/// The manifest form FP.4 reserves (draft).
pub const MANIFEST_FORMAT: &str = "c600-module-draft";
/// The lockfile form FP.4 reserves (draft).
pub const LOCK_FORMAT: &str = "c600-lock-draft";
/// The in-memory approval record; FP.4 has no row for it (draft).
pub const APPROVAL_FORMAT: &str = "c600-approval-draft";
/// The version of every draft form here (FP.10).
pub const FORM_VERSION: i64 = 1;
/// The resolver contract a lock records (PS1.15, LG5.3).
pub const RESOLVER: &str = "c600-resolver-draft-1";
/// 600-cell-Full's model identity, as `assets/manifest.json` and the format packet's examples give it (KN0.1).
pub const FULL_MODEL_ID: &str = "58c83f336e451aa64147fe1b8db2d60d7d43806aa3a35122f8403d5af51253f9";

/// The stable codes this crate refuses with (chapter 13).
pub mod code {
    /// Module bytes differ from the expected content or lockfile digest (LG5.1, LG5.3).
    pub const DIGEST_MISMATCH: &str = "LG5:input.module_digest_mismatch";
    /// Content the locked closure needs is unavailable (LG5.3).
    pub const DEPENDENCY_MISSING: &str = "LG5:input.module_dependency_missing";
    /// The manifest, source editions or dependency contracts disagree (LG5.2, LG5.3, LG5.7).
    pub const CONTRACT_MISMATCH: &str = "LG5:input.module_contract_mismatch";
    /// The manifest does not list the session's model identity (LG5.2, LG5.5).
    pub const MODEL_NOT_SUPPORTED: &str = "LG5:input.model_not_supported";
    /// The IR fails validation at load (LG4.1, LG4.2).
    pub const IR_REJECTED: &str = c600_run::interp::code::IR_REJECTED;
    /// A record's field structure is invalid for an active read (FP.10, PS1.13).
    pub const PERSISTENCE_FORMAT: &str = "PS1:input.persistence_format";
    /// An unknown format and version pair, or a required extension (FP.10, PS2).
    pub const PERSISTENCE_VERSION: &str = "PS2:input.persistence_version";
    /// Bytes that are not FP.5's canonical form, or a set out of order (FP.5, FP.11).
    pub const NONCANONICAL_VALUE: &str = c600_run::interp::code::NONCANONICAL_VALUE;
    /// A capability the profile, class or policy does not grant (LG3, BND.84).
    pub const NOT_GRANTED: &str = c600_check::NOT_GRANTED;
}

fn refuse(code: &'static str, detail: impl Into<String>) -> Refusal {
    Refusal { code, detail: detail.into() }
}
