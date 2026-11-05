# Experimental partial facts model, edition 1

This model is only the PS1.38 experiment. It is not a schema proposal or a product
importer. Decision: **open**. Decider: **owner**. It answers no LQ and selects no route,
engine language or syntax. No acceptance of chapters 05 or 06 follows.

## Why the envelopes are partial

The table assigns pieces of six fields to labels. It does not assign statement IDs,
forms, negation pairs or complete identity/scope bindings. A reviewer receipt may
contain several mathematical statements with different scopes. Treating its rows
as one argument would manufacture meaning. This experiment retains those rows
together with their precise source context, and leaves unbound fields null with
typed `EF:missing.*` reasons. A missing binding is not a claim that the underlying
record has no useful fact. More format-specific interpretation would be needed.

Each included record becomes exactly one envelope, including records with zero
labels and metadata records. A facts envelope stores:

- Its source path, Git blob OID, SHA-256 and size at stored, decoded and canonical
  boundaries; stage and frozen source commit; the top-level format or null.
- A pinned shared edition: order source, initially empty invalid-pair table, trial
  threshold U0, import-table digest, chapter digest and function-source digests.
- All label occurrences as exact JSON pointers plus row IDs into the pinned table.
  No normalized `[]` selector substitutes for a concrete array index.
- Same-object fact bundles. H1 reachability rows always have their own bundle.
  A bundle can contain several rows but is a storage grouping, never an AND node.
  The bundle keeps *all* distinct table candidates for scope, basis, check, mode and
  job; these are unbound interpretations, never a stored truth judgement. A table
  row's claim mapping is available by its pinned row reference, not copied as truth.
- Exact pointers to source identity/input/search/proof/check/attribution fields,
  source scope prose, and occurrences of reported 64-character lowercase digests.
  No digest is silently assigned a subject level or a content role.
- Null typed claim fields with reasons. Immutable source references preserve facts
  not interpreted here. There is no session revision, permission or preview token.

For a bundle with a basis candidate, one leaf **slot** is stored, with null key and
covered set and `recorded_only` admission. No slot enters an EVD argument. The graph
contains no invented keys, AND, OR or edges. Thus storage counts of leaf slots and
counts of admitted support differ: the latter are zero. Zero AND/OR counts describe
this incomplete representation, not the eventual mathematical graph. Source pointers
and table/digest references are storage links, not EVD edges.

This deliberately conservative result is a measurement of partial envelope overhead,
not a completed semantic migration. It cannot estimate the cost of a fully interpreted
graph. RECORDS.json lists every incomplete record and its reasons, instead of dropping
hard records from the denominator. The label table's absence of `form` is itself an
open item in chapter 06; adding arbitrary forms here would hide it.

## Missing fields and source context

The fields of PS1.35/EVD.13 are represented even when unbound: identity; statement ID
and form; negation pair; subject level; typed scope; scope membership; search binding;
premise classes; guards; covered set; checker/tool/common-base binding; reachability.
Raw input digests and parameters are source references, separate from typed scope.
Attribution and engineering class are source references when recorded, otherwise
typed absences. `EF:missing.*` codes are experiment-local diagnostics, not proposed
chapter-06 errors. Unknown is not false, and null guard/premise data is not an empty
list asserting there are no guards or premises.

All original source bytes remain available through the manifest. A missing/mismatched
source object fails acceptance. A digest mentioned in a source record can refer to a
state, unavailable private edition, or content. The inventory reports matches at the
stored/decoded byte boundaries only; a missing match remains an opaque source fact,
never a dangling graph edge. Shared digest text proves neither shared tool identity
nor independence. No new archive checker or mathematics is executed.

EVD.52 limits are enforced conservatively: V14 is not a KN6 checker, P13 never creates
transport, K1 never inherits an enclosing check without a covering relation, and H1
never supplies support. Even rows with declared formats retain missing bindings.
Synthetic filename selectors from the old table are metadata, not inferred formats
written into records (EVD.53/PS3.12).

## Display model and exhaustive domain

There are two explicitly distinct variants in this experiment:

1. **Corpus/recorded variant.** Every envelope above has unbound scope/key and no
   admitted support. EVD.36/51/53 keep it recorded, so its derived word is `open`,
   with original labels retained. The validator rejects a newly typed field, admitted
   leaf or argument edge rather than silently switching variants. Its arbitrary
   payload strings, numbers, source pointers, row sets and digests are opaque to the
   display function. All values of those fields are equivalent for display: the
   only admitted word-relevant state is recorded-only. Every one of the 497 instances
   is checked. This is the exhaustive quotient for the actual envelope format.
2. **Synthetic bounded graph variant.** DISPLAY.json enumerates a broader finite
   facts model to exercise the draft rules and expose collisions; these fixtures are
   not new corpus evidence and are never inferred from an incomplete source record.

The synthetic model admits exactly the Cartesian product below. The complete ordered
enumerations are stored in DISPLAY.json. An unknown enum is outside this domain;
there is no claim of support for arbitrary JSON or arbitrary EVD graphs.

| Field | Admitted values |
| --- | --- |
| Graph shape (6) | no leaf; one leaf for S; one leaf for neg S; one for each; a leaf AND one named open premise; failed leaf (removed) |
| Grade (31) | no grade/leaf; every pair of 5 bases and 6 checks in EVD.1 to EVD.3 |
| Statement scope (4) | one_instance, search_set, admitted_domain, every_reachable |
| Statement form (6) | absent, finite, universal, existence, mixed(forall), mixed(exists) |
| Leaf covered set (4) | absent; explicitly identical; strictly narrower/different pinned finite input; different admitted domain |
| Search (7) | absent; complete/closed; incomplete/deterministic budget; incomplete/declared limit; incomplete/wall clock; incomplete/resource stop; incomplete/other |
| Job (7) | absent, completed, cancelled, cut off, error, not run, not stated |
| Membership (2) | missing or checked, for the synthetic refutation instance |
| Check mode (5) | absent, transport, audit, replay, stated not replayed |

The product is 8,749,440 tuples. The loop visits every tuple, including all modes;
it does not randomly sample or merely deduplicate observed corpus tuples. It hashes
one outcome-index byte per tuple in the declared product order. Types and opaque
identities are fixed, equal and pinned in this synthetic model; invalid identities,
arbitrary discharge/negation cycles and scope-digest levels are **outside** it.
No general I1/I2/I3 validator or CT-EVD-02 to CT-EVD-23 suite is claimed.

The two leaf positions share the selected grade and covered variant; every tuple
still has one independent field assignment. A positive position and a negative
position are separate support leaves, not a stored claim value. Negative positions
use the negated statement form. The open premise has its own fixed key and is never
discharged. A present job is the producing job of these fixture leaves, not an
unrelated later job. Trial U0 supplies the standing threshold. A failed check removes support;
job outcomes and modes supply none. An unused field (e.g. grade on the empty shape)
is retained but has no effect. `same` explicitly represents equality of independently
specified fixture values; real leaf coverage is never copied from its statement.

The probe implements the applicable I4-I7 tests and EVD.41's inert absences before
the word decision list. Contradictions can overlap; one typed diagnostic suffices
for PS1.38 totality and does not assert a new normative diagnostic priority. Scope
phrases are nonempty for all four typed scope kinds; guard phrase templates are
checked separately for reachability, open hypothesis and model-assumption classes.
Synthetic fixtures fix an empty known guard list; the corpus variant has unknown
guards and remains recorded. The 504 node-level tuples (six pass states, four scope
kinds, three search classes, seven job outcomes) run separately from stored facts.

Open cases are not silently decided:

- `EF:collision.not_run_vs_budget` represents EVD.32 row 5 versus EVD.47's row-7
  instruction, when no support stands and a not-run job has an incomplete bounded
  search record. It is an experiment-local typed inconsistency pending clarification.
- `EF:collision.existence_refutation_basis` reports an exact-computation leaf that
  passes the equal-coverage I7 test for neg S and would reach EVD.32 row 1 for an
  existence (or mixed-exists) S, although EVD.42 requires a proof or complete
  certificate. The probe does not settle whether to reject the stored combination
  or require an additional argument. Completed exhaustive negatives retain EVD.43's
  explicit rule. Synthetic certificate leaves in negative positions represent
  complete negative certificates as a fixed fixture fact, not an inference from the
  basis label of any corpus record.
- `not_run_vs_support` reports standing support from its own not-run job: EVD.32
  rows 1-4 versus EVD.47. `negative_job_without_completion` reports a negative from
  its own cancelled/error/cut-off job with no search record: row 1 versus EVD.43.
  I5 already catches the search-backed cases. An admission restriction might exclude
  these tuples, but this experiment does not silently invent that restriction.
  Support from an unrelated completed historical job is outside these fixtures.
- `EF:inconsistency.missing_search_record` covers an exhaustive leaf with no search
  record, rather than assuming completion. `unknown_cutoff_reason` requires the stop
  category when a cut-off search would otherwise be ambiguous.
- `missing_mixed_refutation_witness` preserves EVD.42's missing witness/complete-negative
  facts for a mixed-universal refutation. The bounded shape cannot construct them.
- LQ3's threshold and invalid-pair choices, LQ15 and the absent-format dispatcher
  remain open. Pinning trial U0 and the initially empty table is a probe condition.

Rows 5 and 6 can both match a budget-limited search with an error/cancelled job.
EVD.32 expressly gives priority to row 5, so that overlap is not an open collision.
The collision probes do not assert that `not run` erases independent historical
support; the same-producing-job binding above is part of their counterexample.

## Serialization and comparison boundaries

The experimental canonical profile is Python JSON with sorted keys, compact comma/
colon separators, ASCII escapes, finite numbers, UTF-8, no terminal newline: the
retained JSON profile in PS1.2. It is not the unchosen new-format profile in PS1.3.
Original bytes are digested *before* decompression/parsing; canonicalization creates
a new digest and never changes the original. Gzip is level 9, mtime 0, every member
separately; VERSIONS.json pins Python and zlib. No cross-platform compressor identity
is asserted. Committed readable JSON has indentation and a terminal LF, so file sizes
are different from the measured canonical bytes by design.

SUMMARY.md reports stored and decoded source bytes, canonical source bytes, and the
same canonical profile compressed for every source member. Envelope metadata is
measured both canonical and compressed. The package adds its full shared dependency
catalogue, pinned common input artifacts and canonical manifest exactly once. A
source-preserving package additionally keeps the captured 497 source members once.
The common input artifacts use their captured bytes and their separate gzip sizes;
the row is labelled canonical/captured, not falsely wholly canonical JSON.

The payload measurement excludes the human reports, sample selection and executable
source package; their digests are pinned as provenance. It includes no unavailable
external dependency bytes. Source and envelope comparisons always include the same
497 record paths; the auxiliary corpus (scripts/Markdown/word binaries) is pinned
but not accidentally added to either 497-record baseline. The auxiliary bytes are
not a complete independent-check export and their archival claims are not rerun.

`check.py --write` regenerates only small retained artifacts. The ordinary acceptance
command regenerates the full envelopes and catalogue in memory and compares digests,
counts, tables and samples with the frozen outputs. No build files, temporary files,
bytecode or binaries are written. Ten samples favor small records and rule coverage;
they are not a statistical sample or a substitute for the full-corpus size totals.
