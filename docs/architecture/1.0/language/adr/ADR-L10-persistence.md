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
- **ADR-L10.5** Every stored word, macro and workspace record carries a versioned provenance field, which stays
  meaningful under either form of flow control (PS1.32, PS1.33). Unknown-field handling cannot drop a label, a
  protection requirement, a literal part or a claim dependency while reporting success (PS2.9).
- **ADR-L10.6** An evidence envelope has the same identity and immutable scope fields under either LQ10 option, and it
  refuses a dangling dependency or a failed digest instead of strengthening a claim (PS1.35, PS1.37).
- **ADR-L10.7** Source has editions, and data has schema versions (PS2.1). A stored meaning never changes silently
  (PS2.2). Migration is one-way and recorded, and it validates the complete input before publishing, keeping the old
  bytes and the recovery path (PS2.3, PS2.4, PS2.5).
- **ADR-L10.8** Path B stands as the owner decided: one-way import from a copy taken under the session lock, with the
  original session directory untouched and no `immutable=1` (PS3.1, PS3.2, PS3.9). Each checkpoint offered for restore
  is verified from its own path (PS3.5), and the importer validates the package and replays it before publishing
  (PS3.8).
- **ADR-L10.9** Research archives import through a versioned table keyed by (format, field, value). An unmapped value
  enters as recorded, with its text kept word for word (PS3.10). Records with no `format` key are accepted (PS3.11).
- **ADR-L10.10** Artefact identity is content digest, never path. The corpus is read from Git blobs through a manifest
  of path, blob digest and size, and no importer or test walks a checkout (PS1.38, PS3.15).
- **ADR-L10.11** Formats stay in prose in this stage, and nothing is added under `schemas/` until the records are
  accepted (default 11).

## What this record does not decide

Open whatever the owner answers, for LP1, EN3 and EN4:

- The canonical byte profile for new formats and the wire form of big integers and rationals (PS1.3, PS1.5).
- A deterministic gzip profile, and the open rows of the limits table (PS1.9, PS1.11).
- Unassigned identifiers, versions and unknown-field rules (PS1.13).
- Flat words or named parts (PS1.20), and the provenance field's wire version (PS1.34).
- Dispatch for ambiguous records with no `format` key (PS3.12), and the physical carrier of research bundles (PS3.16).

Conditional on an owner question:

- **LQ10:** what the migration format freeze fixes. The recommendation freezes only the language-neutral data core, with
  an envelope that stores facts and derives judgements (PS1.36, PS2.6, PS2.10). The alternatives are an envelope that
  stores judgements and waits for LQ3, freezing the syntax too, or leaving language data out (PS2.11). LQ10 also decides
  the word encoding of the data core: one word commit as one journal row citing the literal letters by digest, stored as
  `ia16` chunks (KN4.7).
- **LQ11:** how a recording stores and converts (PS1.30).
- **LQ12:** how modules are shared (LG5.9).
- **LQ19:** when the freeze rulings happen relative to the charter's stage 2.5 exit (PS2.12).

## Why

- Renaming a format would cut the link to data that already exists, and it would gain nothing (default 3).
- The provenance field has to be in the data core from the start. A word stored without it can never recover it, so a
  later boundary decision would have nothing to read.
- A facts envelope lets the freeze go ahead without waiting for LQ3, because the judgements are computed and can change
  as an edition (PS2.6).
- Long words need their own storage. The longest literal word stored flat has 2,352,984 letters, and that is about 649
  million characters as J1 rows against 4.7 MB of `ia16` chunks (R3-E4, arithmetic on the record).
- Path identity has already failed once. On Windows six case pairs collapsed, and a checkout walk missed six records
  (R3-E2).

## Alternatives considered

| Alternative | Why it was turned down |
| --- | --- |
| One new container for everything | Would rename existing formats, against default 3, and the old readers would stay anyway |
| Freeze the source syntax too (LQ10) | Early and risky, because the syntax is decided after LX-8. It stays open as LQ10's alternative |
| An envelope that stores judgements (LQ10) | Needs LQ3 and migrations of saved judgements. It stays open as LQ10's alternative |
| Leave language data out of the freeze (LQ10) | Migration would target an interim format and convert again later. It stays open as LQ10's alternative |
| Identify archives by path | Fails on case-insensitive file systems (R3-E2) |
| Normalise archived research records on import | Would lose what was recorded. Original bytes and labels stay (PS3.10) |

## Consequences

- Several byte profiles live side by side, each with its own reader.
- Old carriers keep their quirks. They document input compatibility and are not extended with language fields (PS1.39).
- A reader has to handle a recorded value it cannot map, and much of the research corpus starts that way.
- Nothing lossy is allowed under any alternative without an explicit owner scope decision (PS2.11).
- The freeze needs EN4's packet (PS2.13), and that has not been delivered. Its ruling belongs to Astra and its sign-off
  to the owner.

## Evidence

Class: source. The corpus baseline of 497 records and 13,412,769 bytes is counted from Git blobs. The envelope
experiment of PS1.38 has not run, so the envelope's size is unknown. The figures for long words are read from recorded
runs, and the comparison with J1 rows is arithmetic. Chapter 05's conformance tests are requirements only, and no 1.0
importer exists.

## What the owner accepts

Accepting this record accepts ADR-L10.1 to ADR-L10.11 and the [D] clauses they cite. It answers none of LQ10, LQ11, LQ12
or LQ19, and it is no freeze ruling.

The owner may overrule this integrator addition: a jumbling delta carries the pose-ID remap, because `_compact` reorders
the pose table (KN4.8).
