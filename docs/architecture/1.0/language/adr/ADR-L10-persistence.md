# ADR-L10: persistence formats, the data core and editions

Status: **Proposed.** Not accepted. The owner accepts or rejects it ([`AGENTS.md`](../../../../../AGENTS.md), "Ask the
owner"). Accepting it freezes no format. The migration format freeze is a separate gate.

Rests on: partition INV-7, PS1 to PS3, section 13 and defaults 3, 4, 11, 13 and 20; [chapter
05](../architecture/05-persistence.md); [chapter 01](../architecture/01-kernel.md) KN4. Round table: DT-L1 E6, M2,
R2-E1, R3-E1, R3-E2, R3-E3, R3-E4; explainer P-E9.

## Context

A file outlives the program that wrote it. Solvers' sessions, macros, checkpoints and bindings have to open in every
later version, and the research archives have to stay readable as evidence. A reader written first would turn its
accidents into the format. So the layout is agreed first, and whatever the format cannot record is lost for good when
old data is imported.

The project does not start from nothing. Several formats are already on disk: the 0.4 session and certificate exchange
files, the experimental jumbling store and render files, and the research corpus of stages 14 to 19. Path B also fixes
how 0.4 data enters 1.0.

## Decision

- **ADR-L10.1** New data uses canonical JSON, gzip where large, SHA-256 over normalised bytes and explicit size limits.
  Source text and existing binary arrays keep their own encodings (PS1.1). Readers and writers use one limits table,
  with each write limit at most its read limit (PS1.10).
- **ADR-L10.2** Existing format identifiers and versions are reused, not renamed, and existing byte profiles stay
  distinct (PS1.2, PS1.12). A reader checks an archived file against its captured bytes before any parsing or
  normalisation.
- **ADR-L10.3** Decision fields hold exact values, never floats (PS1.4), and every state digest carries its level
  (PS1.6).
- **ADR-L10.4** Stored words keep their literal chronological letters, alphabet, model identity, menu identity, contract
  revision and start digest (PS1.18). A word has no type-level length cap, and expansion beyond the bound is refused
  with the exact length reported (PS1.21).
  KN4.7 records one word commit as one journal row citing the literal letters by digest. The word encoding belongs to
  the data core (owner, 6 November 2026, LQ10); its `ia16` chunks are an integrator default (KN4.7, PS1.20).
- **ADR-L10.5** Every stored word, macro and workspace record carries a versioned provenance field, which stays
  meaningful under either form of flow control (PS1.32, PS1.33). Unknown-field handling cannot drop a label, a
  protection requirement, a literal part or a claim dependency while reporting success (PS2.9).
  A recording may become a word draft or source through `recording.convert`, or play stepwise, or play view and workspace
  steps unattended; commits stay manual (owner, 6 November 2026, LQ11). PS1.30 and EV4.17 to EV4.23 define eligibility,
  outcomes and per-target conversion. The durable playback cursor is outside the undo order and reconciles a pending
  command through its receipt before advancing; each step keeps its row's undo class (PS1.44, EV4.21).
- **ADR-L10.6** An evidence envelope has one identity and immutable scope fields, and it refuses a dangling dependency
  or a failed digest instead of strengthening a claim (PS1.35, PS1.37).
  PS1.36 stores facts and derives judgements (owner, 6 November 2026, LQ10).
  Evidence-field writers follow EVD.64 (owner, 6 November 2026, decision 11). Proof obligations export to Lean through
  LB5.18 and PS1.45's `c600-proof-obligation-draft` (owner, 6 November 2026, LQ15). In 1.0 EVD.47 and EVD.64 provide
  no proof-record import or writer for `formal proof` (owner, 6 November 2026, decision 9).
- **ADR-L10.7** Source has editions, and data has schema versions (PS2.1). A stored meaning never changes silently
  (PS2.2). Migration is one-way and recorded, and it validates the complete input before publishing, keeping the old
  bytes and the recovery path (PS2.3, PS2.4, PS2.5).
  The migration format freeze covers only the language-neutral data core, including all path B import targets and
  provenance; syntax and IR freeze later under PS2.6 and PS2.10 (owner, 6 November 2026, LQ10).
- **ADR-L10.8** Path B stands as the owner decided: one-way import from a copy taken under the session lock, with the
  original session directory untouched and no `immutable=1` (PS3.1, PS3.2, PS3.9). Each checkpoint offered for restore
  is verified from its own path (PS3.5), and the importer validates the package and replays it before publishing
  (PS3.8).
- **ADR-L10.9** Research archives import through a versioned table keyed by (format, field, value). An unmapped value
  enters as recorded, with its text kept word for word (PS3.10). Records with no `format` key are accepted (PS3.11).
- **ADR-L10.10** Artefact identity is content digest, never path. The corpus is read from Git blobs through a manifest
  of path, blob digest and size, and no importer or test walks a checkout (PS1.38, PS3.15).
  Module sharing in 1.0 under LG5.9 is local, through user-chosen files with digests, with no registry or network
  (owner, 6 November 2026, LQ12).
- **ADR-L10.11** Formats stay in prose in this stage, and nothing is added under `schemas/` until the records are
  accepted (default 11).
  Both freeze rulings sit at LP1's exit. LP2 to LP5 follow those rulings and the owner's sign-off; the design-system ADR
  conditions LP4's editor look and LP6. No gate is relaxed under PS2.12 (owner, 6 November 2026, decision 7).

## What this record does not decide

The owner answered LQ1 to LQ21 on 6 November 2026. LQ8, LQ9, LQ13 and LQ21 wait for their evidence, and LQ2's option set
waits for the joint setting.

Open mechanisms, for LP1, EN3 and EN4:

- Review, implementation and matching evidence for the supplied byte and numeric drafts: `c600-json-1` (PS1.3/FP.5)
  and canonical decimal strings with typed `int`, `rat` and `q5` wrappers (PS1.5/FP.6).
- Review and implementation of `c600-gzip-stored-1` (PS1.9/FP.7) and the trial limits (PS1.11/FP.9). The literal
  gzip blocks provide no compression saving, and the limits do not establish measured product capacity.
- Review and implementation of the draft identifiers, versions and unknown-field rules (PS1.13/FP.4/FP.10/FP.11);
  language-dependent payload contracts remain outside the data-core freeze (owner, 6 November 2026, LQ10).
- Adoption of flat words with named annotations, or a separately reviewed referenced-part alternative (PS1.20).
  PS1.34/FP.17 supply nested provenance version 1 under both LQ21 forms; review, implementation and compatibility
  evidence remain owed, and no form or boundary policy is selected.
- Complete typed discriminator bindings for absent-format records (PS3.12/FP.22); the dispatch rule is drafted but
  unmatched records stay recorded. PS3.16/FP.23 supply the digest-named ZIP STORE research carrier draft, still
  requiring review, implementation and matching evidence.

## Why

- Renaming a format would cut the link to data that already exists, and it would gain nothing (default 3).
- The provenance field has to be in the data core from the start. A word stored without it can never recover it, so a
  later boundary decision would have nothing to read.
- A facts envelope keeps stored data independent of judgement editions, because the judgements are computed and can
  change as an edition (PS2.6).
- Long words need their own storage. The longest literal word stored flat has 2,352,984 letters, and that is about 649
  million characters as J1 rows against 4.7 MB of `ia16` chunks (R3-E4, arithmetic on the record).
- Path identity has already failed once. On Windows six case pairs collapsed, and a checkout walk missed six records
  (R3-E2).

## Alternatives considered

| Alternative | Why it was turned down |
| --- | --- |
| One new container for everything | Would rename existing formats, against default 3, and the old readers would stay anyway |
| Freeze the source syntax too (LQ10) | Rejected alternative of LQ10 (owner, 6 November 2026). Early and risky, because the syntax is decided after LX-8 |
| An envelope that stores judgements (LQ10) | Rejected alternative of LQ10 (owner, 6 November 2026). Would couple the freeze to LQ3 and migrations of saved judgements |
| Leave language data out of the freeze (LQ10) | Rejected alternative of LQ10 (owner, 6 November 2026). Migration would target an interim format and convert again later |
| Identify archives by path | Fails on case-insensitive file systems (R3-E2) |
| Normalise archived research records on import | Would lose what was recorded. Original bytes and labels stay (PS3.10) |

## Consequences

- Several byte profiles live side by side, each with its own reader.
- Old carriers keep their quirks. They document input compatibility and are not extended with language fields (PS1.39).
- A reader has to handle a recorded value it cannot map, and much of the research corpus starts that way.
- Nothing lossy is allowed without an explicit owner scope decision (PS2.10).
- PS2.13 identifies the supplied [EN4 format packet](../format-packet/README.md), candidate `en4-data-core-draft-1`
  with its inventory in `candidate.json`. Delivery is not freeze readiness: FP.35 retains typed corpus coverage,
  integration checks, implementation/evidence and review dependencies. Its ruling belongs to Astra and its sign-off
  to the owner.

## Evidence

Class: source. The corpus baseline of 497 records and 13,412,769 bytes is counted from Git blobs. PS1.38's source
report, which one Sol review sampled without a finding on it, contains 497 partial envelopes and zero complete typed
envelopes: 15,210 fact bundles, 11,467 incomplete leaf slots, no AND/OR nodes or edges, and 10,424,730 canonical
metadata bytes. Those sizes use the experiment's
serializers, not FP.7; they establish neither complete lossless typed coverage nor the size of a complete typed
envelope. FP.33 retains those limits, and no timing result is claimed. The figures for long words are read from
recorded runs, and the comparison with J1 rows is arithmetic. Chapter 05's conformance tests are requirements only,
and no 1.0 importer exists.

## What the owner accepts

Accepting this record accepts ADR-L10.1 to ADR-L10.11 and the [D] clauses they cite. It answers no owner question and
is no freeze ruling. LQ3, LQ10, LQ11, LQ12, LQ15 and LQ19 were answered on 6 November 2026; those answers accepted no
ADR and froze nothing.

The owner may overrule this integrator addition: a jumbling delta carries the pose-ID remap, because `_compact` reorders
the pose table (KN4.8).
