//! The 600 Cell Script runtime in a process of its own: route RT1 (LQ9; RTE.21, ONE.69, ADR-L12.9), for LP3.
//!
//! The binary `c600-runtime` is the child. It reads framed messages on standard input and writes framed messages on
//! standard output. It parses, lowers, admits, loads and runs each job with `c600-syntax`, `c600-check` and
//! `c600-run`, and forwards every host call (`tariff`, `perform`, `sink`) to the parent. The [`parent`] module is the
//! other end: [`parent::Runtime`] starts the child, sends it a job, serves its calls from the caller's
//! `c600_run::interp::Host` and returns the run's end or a typed [`parent::Fault`]. [`program::run_in_process`] is the
//! same compile-and-run path inside the caller's process, the reference the tests hold the child to.
//!
//! # Protocol (draft `c600-rt1-wire-1`)
//!
//! - Frame: a little-endian `u32` length and the payload, at most [`wire::MAX_FRAME`] (16 MiB) either way; the
//!   payload starts with the protocol byte [`wire::PROTOCOL`] and a kind byte ([`message::kind`]).
//! - Child to parent: `hello` (protocol, IR version, execution contract) once; `call` (run, call id, acknowledged
//!   call id, and a `tariff`, `perform` or `sink` request); `done` (run, acknowledgement, and the summary, value and
//!   output, or the refusal stage, codes and detail).
//! - Parent to child: `run` (sources, module, entry, arguments, limits, tariffs, seed, record flag, profile, grant,
//!   declassifiers, source-range flag), `answer` (tariff, value, delivered, or refusal code), `cancel` (run, why).
//! - Values, types and summaries use [`codec`]'s canonical tagged encoding: exactly one encoding per value.
//! - KI.5: a new call acknowledges the last answer; the parent answers a repeated delivery of the open call from its
//!   record and never calls the host twice for it.
//! - Faults ([`parent::FaultKind`]): `Spawn`, `Incompatible`, `Lost`, `Malformed`, `Oversized` and `Protocol` end the
//!   job as `error` with `KN7:resource_limit.runtime_lost`; `Deadline` ends it as `cut_off` with
//!   `KN7:resource_limit.safety_stop`. The child is killed and reaped, and the next run starts a fresh one.
//!
//! # Drafts
//!
//! Every choice below fills a hole the frozen architecture leaves open. Each names the clause it touches; none of
//! them is accepted, and each may change with the protocol byte.
//!
//! 1. **Messages and framing** (KI.15, "the concrete ABI or messages"; ONE.69): six message kinds over the child's
//!    standard streams, length-prefixed frames, one protocol byte. `hello` names the IR version and the execution
//!    contract, and the parent refuses a child that names others.
//! 2. **Transfer encoding** (KI.15, "canonical transfer encoding"; DET.59): DET.59's `c600-json-1` unit measures
//!    output and cannot carry closures, functions or receipts, so values cross in [`codec`]'s own encoding:
//!    fixed-width little-endian integers, minimal sign-magnitude big integers, and a refusal of every noncanonical
//!    form (a ratio not in lowest terms, an unnormalised `Q5`, an approximation mantissa out of range, a word letter
//!    outside 1..=1200, a record with a repeated field) on both sides. A closure crosses as its code index and
//!    captures and means something only to the program of the same run. LP2's draft collections cross too: a
//!    list's elements in order, a word key's letters, freely reduced (LG2.18), and a set's elements and a map's
//!    `{ key, value }` entries, strictly increasing in the canonical key order without its secondary key (DET.9), so
//!    a written order never reaches the bytes. A compressed word has no form yet (draft 15).
//! 3. **Acknowledgement and retry** (KI.5; KI.15, "acknowledgement/retry protocol"): every call carries its id and
//!    the id of the last answer the child received. The child is synchronous, so a new call must acknowledge the
//!    last answer and at most one answer is open: the parent keeps the open answer's bytes until the next call or the
//!    `done` acknowledges it, and answers a repeated delivery of the open call, with its request and acknowledgement
//!    as first sent, with identical bytes from that record. Anything else is a protocol fault: a new call that does
//!    not acknowledge the last answer, a repeated id with another request or acknowledgement, an acknowledged id, a
//!    skipped id, or a `done` that leaves the last answer unacknowledged. The child never repeats a call by itself:
//!    over a pipe an answer is lost only with the process. The debug-only `duplicate` rehearsal exercises the repeat
//!    path and `unacked` the acknowledgement rule.
//! 4. **Process-loss reclamation** (KI.15, "process-loss reclamation"; KN7.13; ONE.70): on any fault the parent
//!    kills and reaps the child. The child owns no engine state, so everything it held ends with it; the child also
//!    exits when its input closes. Missing and not built (RTE.21 placement only; no OS isolation was in scope): a
//!    Windows job object with kill-on-close, a restricted or AppContainer token, and filesystem or network
//!    confinement of the child.
//! 5. **Resource bounds** (KI.15, "resource bounds"): [`wire::MAX_FRAME`] both ways, checked before any payload
//!    byte is allocated; [`codec::TYPE_DEPTH`] for transferred IR types; values decode without recursion; every
//!    length is checked against the bytes left before allocation; [`parent::STDERR_TAIL`] bytes of standard error
//!    are kept; the child keeps at most 1024 distinct host refusal codes of at most 256 bytes. A call above the frame
//!    bound stops the run with `KN7:resource_limit.host_exhausted`, and so does an answer above it; a job above it is
//!    refused at the start stage without a code; a result above it ends the child. The parent's output reader holds
//!    at most [`parent::READ_AHEAD`] frames besides the one it reads, and the KI.5 record at most one answer (draft
//!    3). Not bounded yet: the parent's queue of frames to the child, which grows by one answer per call or repeated
//!    delivery while the child sends without reading its input; bounding it without blocking the parent needs a rule
//!    for `cancel` and is deferred. Nor is the time the parent spends checking a decoded set's or map's order, which
//!    compares elements outside any meter: a crafted frame costs time within the frame bound only.
//! 6. **Fault codes** (KN7.13, ERR.19, CT-ERR-14; KN7.10, ERR.16, DET.17): a lost runtime is
//!    `KN7:resource_limit.runtime_lost` with outcome `error`; a passed deadline is `KN7:resource_limit.safety_stop`
//!    inside `cut_off`, measured from the start of the run with host time included and outside the deterministic
//!    key. The parent checks the deadline only between messages, while it waits for the child: a host call that
//!    blocks is not interrupted, and a deadline it passes is noticed only after it returns. A child that does not
//!    start or names another protocol also ends as `runtime_lost`: no clause names a failure to start.
//! 7. **Values that cannot cross** (PS1, `PS1:input.noncanonical_value`): a host value without a canonical form is
//!    answered as a refusal with that code; a call the child cannot encode stops the run with it; a job whose
//!    arguments cannot be encoded is refused at the start stage with it, as `Loaded::run` refuses them in process.
//! 8. **Stop codes** (KN7, ERR chapter 13): a host refusal crosses as text. The child interns it as the
//!    `&'static str` the interpreter needs; the parent resolves a summary's codes against the meter's and the
//!    interpreter's static codes and the codes its host returned during the run, and any other code is malformed.
//! 9. **Cancellation** (DET.56, KN7.6): `cancel` names the run. The child's input thread records each run's number
//!    as it reads the `run`, before the main loop takes the run, and applies a `cancel` at once to the interpreter's
//!    pending interruption, so a `cancel` that follows its run is never lost and an interruption the parent sends
//!    before an answer is observed before that answer is used; a `cancel` for another run is ignored. The parent
//!    makes a run visible to its [`parent::Canceller`]s only once the run's frame is queued, so a `cancel` always
//!    follows its run. A canceller reports whether it reached a run in progress, and one that outlives its child
//!    holds nothing of it.
//! 10. **No authority on the wire** (KN5.15): no message has a field for a user-action token or any other
//!     authority. [`message::SCHEMA`] lists every field, and `tests/protocol.rs` holds the encodings to it. A preview
//!     a run proposes is confirmed outside the runtime, through the host.
//! 11. **What a job carries** (KI.6; ADR-L12.9): source bytes, so the child runs the same compile stage as
//!     [`program::run_in_process`]. Sending checked IR instead (KI.6's checked isolate entry) is open, and so is
//!     where `c600_run::boundary`'s placement and provenance checks sit: the parent leaves them to the caller's host.
//! 12. **Fault rehearsals** (LX-2 is not run here): `C600_RUNTIME_FAULT=<kind>@<answers>` is read only in debug
//!     builds (`cfg(debug_assertions)`); a release build contains no rehearsal code, and the parent removes the
//!     variable from the child's inherited environment unless [`parent::Config::env`] sets it. Rehearsal results
//!     are source and synthetic evidence, not LX-2 evidence.
//! 13. **Closing the output** (rehearsal limit): the standard library has no safe way to close standard output while
//!     the process lives, so the `close` rehearsal exits with status 0 before the result and `truncate` writes part
//!     of a frame and exits.
//! 14. **Type depth** (KI.15, "canonical transfer encoding"; ONE.69; a known difference): an IR type crosses only to
//!     [`codec::TYPE_DEPTH`] (64) levels, because the parent decodes types into `c600_ir::Ty`, which is cloned,
//!     compared, printed and dropped recursively. A request whose type is nested deeper has no transfer encoding: in
//!     the child the call stops the run with `PS1:input.noncanonical_value`, while in process the same call reaches
//!     the host. The two executors differ there until the checker bounds type depth.
//! 15. **Compressed words** (KI.15, "canonical transfer encoding"; LG2.16; a known difference): `c600-run` keeps a
//!     compressed word's parts private, so the value has no transfer form; its type has one. Both executors refuse
//!     it as an argument at the start stage, since it has no DET.59 output form either, and so no run returns or
//!     outputs one. As a host's answer it reaches the run in process, while the parent answers the child's call
//!     with `PS1:input.noncanonical_value` (draft 7). The two differ there until `c600-run` lends a way to read and
//!     rebuild the parts.

#![forbid(unsafe_code)]

pub mod child;
pub mod codec;
pub mod message;
pub mod parent;
pub mod program;
pub mod wire;

pub use parent::{Canceller, Config, Fault, FaultKind, Killer, Runtime};
pub use program::{run_in_process, Ended, Job, JobOptions, Refused, Stage};

/// The variable that selects a fault rehearsal in a debug build of the child: `<kind>@<answers>`.
pub const FAULT_VARIABLE: &str = "C600_RUNTIME_FAULT";
