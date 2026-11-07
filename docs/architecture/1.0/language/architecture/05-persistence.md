# Chapter 05: persistence and exchange

Status: **draft for the owner. Not accepted.** Conventions are in [the overview](00-overview.md), section 3.

This chapter describes stored data, its byte boundaries and its passage between versions. Chapters 02, 04, 06, 07
and 09 define the meanings of modules, bindings and recordings, evidence, provenance and reproducibility. This chapter
records those meanings without choosing source syntax, a runtime, an evidence order or a boundary policy. The format
tables describe logical fields. The [EN4 format packet](../format-packet/README.md) assigns candidate wire fields,
identifiers, profiles and limits for the language-neutral core. These are drafts, not a freeze or an owner answer.

Sources: [the partition](../ecosystem-partition.md), sections 7, 8.1, 10.3, 11, 13 and defaults 3, 4, 11, 13, 20;
[chapter 01](01-kernel.md); [DT-L1](../../../../progress/1.0/design-table/topics/language-ecosystem.md), M2, R2-E1,
R2-E3 and the persistence and evidence-storage Fix first items;
[DT-J1](../../../../progress/1.0/design-table/topics/jumble-solver-aids.md) and
[DT-M1](../../../../progress/1.0/design-table/topics/command-keyboard-model.md);
[migration decision of 2 October](../../../../wiki/decisions/owner-decisions-2026-10-02-migration.md),
[its 3 October amendment](../../../../wiki/decisions/owner-decisions-2026-10-03-migration.md),
[the owner decisions of 6 November](../../../../wiki/decisions/owner-decisions-2026-11-06.md),
[migration options](../../../../progress/1.0/migration-options.md),
[exporter re-plan](../../../../progress/1.0/migration-exporter-proposal.md) and
[probe report](../../../../progress/1.0/migration-probe-results.md);
[older package field draft](../../../../progress/0.4.1/exporter-spec-draft.md), only where reused by the re-plan;
[ENG-0J](../../../../progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md), section 4;
[render contract](../../../../../research/jumbling/render-contract.md);
[V03 and V05](../../10_V1_ARCHITECTURE.md), within the limits in the briefing;
[requirements](../../../../progress/1.0/requirements-from-screening.md);
[research handoff](../../../../progress/1.0/jumbling-handoff/research-data.md),
[restoration index](../../../../../research/jumbling/restoration/README.md),
[stage 19](../../../../../research/jumbling/restoration/19-height-depth-framework/README.md) and its
[publication boundary](../../../../../research/jumbling/restoration/19-height-depth-framework/PUBLICATION.md);
[`session.py`](../../../../../session.py), [`log_io.py`](../../../../../log_io.py),
[`core.py`](../../../../../core.py) and the experimental
[jumbling store](../../../../../research/jumbling/differential/candidate/store.py).

## 1. PS1: byte rules and limits

- **PS1.1 [D]** Data uses canonical JSON, gzip where large, SHA-256 over normalised bytes and explicit size limits
  (partition default 4). Source text and existing binary arrays retain their own encodings. Canonical means that a
  format version assigns one byte sequence to a value: object-key order, string escaping, number representation,
  separators, encoding and terminal newline are fixed. Array order remains significant. A digest of stored bytes,
  a digest of decoded content and a mechanical state digest are separate fields.
- **PS1.2 [D]** The existing profiles in the following table remain distinct (defaults 3 and 4). A reader checks an
  archived file against its captured bytes before any parsing or normalisation. Re-encoding a legacy value under a new
  profile produces another content digest and a recorded conversion; it never repairs an old digest in place.

| Existing byte profile | Canonical content or captured bytes | Compression and digest boundary | Source |
| --- | --- | --- | --- |
| Retained JSON | `core.canonical`: sorted object keys, compact comma and colon separators, ASCII escapes for non-ASCII text, `allow_nan=False`; UTF-8 encoding, no terminal newline added | SHA-256 of exactly those bytes where the format calls for canonical JSON | `core.py`, `canonical` and `digest` |
| Retained proof log | Retained JSON profile | `encode_log`: gzip level 5, `mtime=0`; the file digest covers stored gzip bytes, the content digest covers uncompressed bytes | `log_io.py`, `encode_log` |
| Experimental jumbling session | Sorted keys, compact separators, `ensure_ascii=False`, finite numbers, UTF-8 and one terminal LF; exact integer tokens retain all digits | The prototype writes JSON without gzip; binary ID fields have their own digests | `candidate/store.py`, `_json_text`, `dumps`, `encode_ids` |
| Stage 19 public archive | The captured public bytes; its JSON production uses compact separators and `ensure_ascii=False` | JSON over 250,000 bytes uses `gzip.compress(data, compresslevel=9, mtime=0)`; stored, uncompressed and original-source digests are distinct | Stage 19 `PUBLICATION.md` and `IMPORT-MANIFEST.json`; source evidence for the threshold |
| Earlier research records | The bytes identified by that stage's manifest, including original spacing and line endings | The stage's recorded stored and uncompressed digests; no reserialization before checking either | Restoration index, each `IMPORT-MANIFEST.json` |

- **PS1.3 [D]** The candidate profile for new core records is `c600-json-1` in FP.5: scalar-sorted keys, literal
  Unicode, compact separators, UTF-8 without BOM and exactly one terminal LF. Duplicate keys, lone surrogates and
  noncanonical bytes refuse active reading; explicit conversion retains the original bytes. FP.6 gives finite
  display numbers a separate typed bit encoding. This is the EN4 draft for KN1 normalisation, not an adopted
  universal JSON standard. PS1.2's retained no-LF and prototype LF profiles remain distinct and have byte vectors.
- **PS1.4 [B]** Decision fields contain exact values, never floats (INV-4). The store's Q(√5) entry is the integer
  triple `[a,b,d]` for `(a+b√5)/d`, with positive `d` and `gcd(a,b,d)=1`; matrices retain their ordered entries
  (`candidate/store.py`, `check_q5`, `check_matrix`; ENG-0J store section). Its integers are JSON integer tokens of
  arbitrary precision. Reading them through a binary floating-point number is invalid even when the JSON parser
  otherwise accepts the file. Existing decimal-string fields, such as `primitive_count`, keep their representation.
- **PS1.5 [D]** New `Int`/`Nat` fields use canonical decimal strings; free exact values use the typed `int`, `rat`
  and `q5` wrappers of FP.6. Rationals have reduced numerator and positive denominator. Canonical integer text has
  no leading zeroes or plus sign, and zero is `0`, never `-0`. New JSON integer tokens are limited to the exact
  53-bit range; large exact values never pass through floats. The shared digit limit is 10,000 and has a generated
  6,775-digit round-trip vector. A host parser ceiling cannot round or silently change it. Existing formats keep
  their own integer tokens and decimal-string fields (PS1.4).
- **PS1.6 [D]** Every state digest carries its level (KN1.8, default 4). `Digest<labels>` is SHA-256 of all labels in
  canonical slot order as signed little-endian int32, as in `core.state_hash`. `Digest<poses>` follows J1's exact
  `State.digest`: group non-identity pieces by exact pose, sort groups by the JSON matrix key, then hash each key's
  UTF-8 bytes followed by its ascending piece IDs as signed little-endian int32. The key uses J1's JSON spacing,
  including its spaces. `candidate/store.py`, `canonical_digest`, makes the little-endian convention explicit. It is
  not the digest of the store JSON, a pose-ID array, or `poses.f32`. A labels digest never substitutes for a poses
  digest. Off the lattice, pose is the only stored level until a sticker-class digest exists (KN1.8).
- **PS1.7 [B]** Binary arrays retain their declared type, shape and byte order (render contract section 3;
  ENG-0J store section). The render contract uses little-endian int32 pose indices and lattice indices, and float32
  matrices for drawing. The store uses `u32le-base64` for pose IDs and menu-letter codes, with `encoding`, `count`,
  `sha256` and `data`; its SHA-256 covers the decoded bytes. These signed and unsigned carriers do not redefine
  mechanical identity. Decision fields use exact poses and canonical IDs, never the float matrices or an index from
  another pose table.
- **PS1.8 [B]** Hashed assets and proof sources use their pinned repository bytes (R-03). The current
  [`.gitattributes`](../../../../../.gitattributes) disables text conversion. New source and architecture text is
  UTF-8 with LF; existing captured bytes retain their declared profile. Checkout conversion is not a digest
  normalisation step. A reader that receives changed bytes reports a mismatch instead of accepting the old digest.
- **PS1.9 [D]** New records use `c600-gzip-stored-1` (FP.7): fixed header, zero mtime, no optional fields,
  deterministic 65,535-byte DEFLATE stored blocks, CRC32 and size trailer. This candidate chooses portable literal
  blocks rather than a platform compressor version; it provides no compression saving. Records over 250,000 decoded
  bytes use gzip, smaller ones raw. A compressing successor needs its own profile and vectors. Existing level-5 logs,
  level-9 archives and timestamped `encode_export` downloads keep their captured stored and separate decoded digests.
- **PS1.10 [B]** Readers and writers use one limits table, with each write limit at most its read limit (R-01).
  A limit is checked before allocation, expansion or publication that would exceed it. Compression never bypasses
  the decoded limit. The following table distinguishes compatibility limits from limits that have not been chosen.
  Its existing values are source evidence from the named files, not new measurements or product capacity claims.
  FP.9 is the single candidate read/write resource table; the table here records the compatibility obligations
  and the independent EN3 admission dependency that it must not replace.

| Profile and unit | Write limit | Read/admission limit | Source or decision still needed |
| --- | --- | --- | --- |
| Retained interactive log, stored bytes | At most 16 MiB | 16 MiB | `log_io.py`, `MAX_LOG_BYTES`, `encode_log`, `decode_log` |
| Retained interactive log, uncompressed UTF-8 bytes | At most 24 MiB | 24 MiB | `log_io.py`, `MAX_JSON_BYTES` |
| Retained interactive log, transactions | At most 10,000 | 10,000 | `log_io.py`, `MAX_TRANSACTIONS` |
| Retained interactive log, expanded signed-generator primitives over the whole history | At most 2,000,000 | 2,000,000 | `log_io.py`, `MAX_PRIMITIVES`; separate from one recipe's ceiling |
| A 0.4 recipe, steps | 1 to 256 | 1 to 256 | `core.Model.normalize`; partition section 13 |
| A 0.4 recipe word step, signed-generator letters | 1 to 100,000 | 1 to 100,000 | `core.Model.normalize`; representation chunk size |
| A 0.4 recipe, expanded signed-generator primitives over all steps | At most 3,000,000 | 3,000,000 | `core.Model.normalize`; V05.1; unchanged in path B |
| Stored word, mathematical expanded length | No type-level cap | No type-level cap | DT-J1 E3; finite storage and run budgets still apply |
| Experimental store named-part nesting, depth | At most 64 | 64 | `candidate/store.py`, `_check_parts`; applies to that existing carrier |
| Checking, previewing or replaying a menu word, expanded menu letters | At most the X3 admission bound | The same X3 admission bound | Value open; DT-J1 X3 and EN3, PS1.21 |
| Legacy preference validation, characters in `core.canonical(prefs)` | At most 500,000 | 500,000 in the compatibility validator | `session.py`, `_validated_prefs`; this is a character count, not a UTF-8 byte count |
| New JSON, source, graph and binary record profiles | Equal to each FP.9 read limit | `c600-limits-draft-1`: FP.9 | PS1.11; trial resource budgets, not measured capacity |
| Path B package and independent-check bundle | Equal to each FP.9 read limit | FP.9 per-member and cumulative bytes, members, events and replay cost | PS1.11; no inherited interactive-log cap |

- **PS1.11 [D]** FP.9 assigns `c600-limits-draft-1`, with equal read/write bounds for new records and archives,
  and retains the compatibility limits separately. Its analysis cites the PS1.38 corpus and labels the unmeasured
  memory/performance assumptions. Counts are checked before allocation or expansion, including cumulative gzip,
  graph and replay work. The old log cap is not a path B archive cap, and the archive cannot raise a recipe's
  semantic ceiling or replace EN3's X3 bound. An over-limit write preserves the source and returns a typed refusal;
  it never truncates or silently splits an operation (default 10, V05.2). The trial values still need review.

```text
operation read_record(bytes: Bytes, format: FormatRef, limits: LimitsRef) -> StoredValue | RecordedInput
  effect:  read a granted file, or a command-layer query
  cost:    stored and decoded bytes, parsed values and declared verification work
  errors:  PS1:input.persistence_format, PS2:input.persistence_version, PS1:input.noncanonical_value,
           PS1:resource_limit.persistence_size, PS1:input.digest_mismatch
operation write_record(value: StoredValue, format: FormatRef, limits: LimitsRef) -> Bytes
  effect:  export to a granted file, or a workspace command
  cost:    canonical bytes and declared compression work
  errors:  PS2:input.persistence_version, PS1:resource_limit.persistence_size, PS1:evidence.scope_not_immutable
```

## 2. PS1: format catalogue

- **PS1.12 [D]** Existing format identifiers and versions are reused without renaming (partition default 3).
  The catalogue distinguishes a legacy input, an experimental carrier and a proposed logical record. A slash in an
  existing identifier is part of that identifier. The source extension is `.6cs` (owner, 6 November 2026, decision 6).
  The `c600-` descriptors, including `c600-source-draft`, stay until the format freeze; the name changes no module
  identity bytes. The data-core column follows the scope chosen in LQ10; nothing in this catalogue freezes a format.

| Format | Identifier | Version | Writer | Reader | Data core under LQ10 |
| --- | --- | --- | --- | --- | --- |
| Source | Source bytes: PS1.14; descriptor `c600-source-draft` (FP.4); extension `.6cs` | Source edition | Authoring environment | Chapter 02 front end | No |
| Module manifest | `c600-module-draft` (reserved; FP.4) | Data schema plus source edition reference | Module authoring command | Chapters 02, 03, 09 | No |
| Lockfile | `c600-lock-draft` (reserved; FP.4) | Data schema | Local module resolver | Chapters 02, 09 | No |
| Word and macro | `c600-word-draft`, `c600-macro-draft` (FP.4) | Data schema | Workspace word commands, importer | Kernel replay, macro tools, exporter | Yes |
| Recipe | Existing `word` and `star` steps in the carriers below | Carrier's schema | Workspace commands, importer | Exact engine, oracle adapter | Yes |
| Macro set and protection set | `c600-macro-set-draft`, `c600-protection-set-draft` | Data schema | Workspace commands, importer | Session command layer | Yes |
| Binding and key map | `c600-binding-draft`, `c600-key-map-draft` | Data schema | Key-map commands, importer | Chapter 04, command layer | Yes |
| Layout and declared tool parameters | `c600-layout-draft` (reserved; FP.4) | Data schema | Invoked workspace commands | Chapters 04, 10 | No; their provenance field is in the core |
| Filter | `c600-filter-draft` | Data schema | Filter commands, importer | Filter evaluator | Yes |
| Preference | `c600-preferences-draft` | Data schema | Preference commands, importer | Session and workspace commands | Yes |
| Checkpoint and replay path | `c600-checkpoint-draft`, `c600-replay-path-draft`; member `session/checkpoint-paths.json` | Data schema | Session store, importer | Recovery and replay checker | Yes |
| Recording | `c600-recording-draft` (reserved; FP.4) | Data schema | Chapter 04 recorder | Recording converter and gated playback | No |
| Playback state | `c600-playback-draft` (FP.37) | Data schema | Command layer (PS1.44) | Playback and recovery | No |
| Proof-obligation export | `c600-proof-obligation-draft` (PS1.45) | Export schema and vocabulary version | LB5.18 translator through a granted file capability | External Lean 4 tooling | No |
| Run record | `c600-run-draft` (reserved; FP.4) | Data schema, editions and ReproKey references | Job completion/store command | Reproducer, independent checker | No; referenced envelopes use the core |
| Workspace record | `c600-workspace-draft` | Data schema | Workspace commands, importer | Workspace queries | Provenance field only, plus the core payload kinds above |
| Pick record | `c600-pick-draft` (FP.38) | Data schema | Workspace commands (BND.54) | Provenance adapter (BND.84, BND.85) | Yes |
| Evidence envelope | `c600-evidence-draft` | Data schema and evidence-function editions | EVD.64's named writers, enforced by EVD.65; imports obey EVD.47 | Chapter 06, independent checker | Identity, immutable scope and facts payload |
| Import table | `c600-import-table-draft` | Mapping-table version | Reviewed import-table author | Research importer, coverage checker | Envelope's mapping dependency; packet fixes its boundary |
| Retained session exchange | `C600-STUDIO-SESSION-v1` | v1 | Existing 0.4 exporter | Compatibility reader | Existing input to migration |
| Retained preview certificate | `C600-STUDIO-CERTIFICATE-v1` | v1 | Existing 0.4 preview | Compatibility reader, independent checker | Existing evidence input; no permission |
| Experimental jumbling section | `magic600-jumbling-store/1` | 1 | Experimental store; future store adapter | Store/recovery checker | Word, checkpoint and evidence portions only |
| Jumbling drawing data | `magic600-jumbling-render/1` | 1 | Engine/view-model export | Renderer adapter, fixture readers | No |
| Experimental session wrapper | `magic600-session-proto/1` | 1 | Experimental store | Experimental round-trip reader | No choice of product wrapper follows |
| Path B archive | `C600-MIGRATE-draft`, working package name `.c600migrate` | Draft | 1.0 importer, from staged copies | Migration validator/importer | Carrier of the selected core; still draft |
| Stage 14 certificates and checks | `t3p-certificate/1`, `t3p-check/1` | 1 each | Archived producer/checker | Independent research checker/importer | Archived inputs |
| Stage 14 analyses | `t3p-scan/1`, `t3p-pairs/1`, `t3p-plus/1`, `t3p-levels/1`, `t3p-locality/1`, `t3p-chain/1` | 1 each | Archived research tools | Importer and format-specific checkers | Archived inputs |
| Stage 15 macro records | `macro-certificate/1`, `macro-check/1` | 1 each | Archived producer/checker | Independent macro checker/importer | Archived inputs |
| Stage 15 search records | `pstar-*`, `pstar2-*`, including `pstar-census/1`, `pstar-step/1`, `pstar2-stop/1`, `pstar2/2` | As in each record | Archived search tools | Search-scope checker/importer | Archived inputs |
| Stages 16 and 17 records | `s16-*/1`, including `s16-rr/1`, `s16-pxrr/1`, `s16-transport/1`; `jams.json` may have no format | As in each record, or absent | Archived research tools | Format-specific checker/importer | Archived inputs |
| Step index and word files | `steps.json` and each word file's own header, or no header | As captured | Archived runner | Chain/word checker/importer | Archived inputs |
| Compressed-word certificates | `r22b10/1`, `r22b10/2` | 1 and 2 | Archived certificate builder | Expansion and composition checker | Archived inputs |
| Runner record | `runner-run/3` | 3 | Archived runner | Independent run checker/importer | Archived input |
| Lattice records | `literal-lattice/1`, `lattice-phase-play/1` | 1 each | Archived lattice runner/checker | Independent lattice checker/importer | Archived inputs |
| Long-word binary carrier | `ia16` | As captured; no invented JSON header | Archived word writer | Streaming word checker | Archived input |
| Research publication manifest | `IMPORT-MANIFEST.json`; stage 19 header `sanitized-research-archive/1` | Per manifest | Archive preparation | Integrity checker/importer | Source-byte dependency |
| Stage 19 proofs, tables and receipts | Each original header, including absent headers; no umbrella rename | As captured | Archived research producer/reviewer | Independent checker/importer | Archived inputs |

- **PS1.13 [D]** FP.4/FP.10/FP.11 assign independent version-1 draft records, their wire fields and unknown-field
  outcomes. FP.4 supplies the identifiers now listed in the catalogue; language-dependent payloads remain
  outside the data-core freeze. Unknown ordinary fields refuse active read; optional extensions are preserved
  and cannot alter required meaning, while unknown required extensions or versions refuse activation. FP.24 uses
  `C600-MIGRATE-draft` with `format_version=1`; `c600migrate` is a different draft input requiring an explicit
  converter, not an alias. No experimental session wrapper becomes the product container. Default 11 keeps this
  specification in prose; nothing is added under `schemas/`.

## 3. PS1: source, modules and workspace data

- **PS1.14 [D]** Source stores UTF-8 text with an edition header (partition PS1). The following fields describe the
  source artefact without choosing the spelling or location of that header. Its digest is of the pinned source
  bytes, including the header. A source change creates different bytes; a saved compiled result still cites the
  exact bytes it was built from (R-08).

| Source field | Stored value and validation |
| --- | --- |
| Edition header | Explicit source edition interpreted by chapter 02; absent or unknown edition is never guessed from the current default |
| Source text | Complete UTF-8 text, LF profile, including comments and the edition header |
| Source digest | SHA-256 of those bytes, stored in the referring manifest or record without a self-hash cycle |
| Source linkage | Locations refer to this source digest; chapter 04 defines the location model |

- **PS1.15 [D]** The manifest and lockfile retain the chapter 02 meanings; their stored fields below follow partition
  LG5, LB6, BM2 and INV-10. A display name alone never resolves a dependency. The principle record's content belongs
  to chapter 03; keeping it as manifest data relies on DT-L1 R2-E4, with the additions of R3-M4 (LB6.2).

| Module manifest field | Stored value and validation |
| --- | --- |
| Format/version and edition | Manifest schema and the edition of each source input |
| Module identity and sources | Module identity, source digests and their local resolution entries |
| Dependencies | Chapter 02 dependency declarations; no ambient dependency lookup |
| Capabilities/effects and entry metadata | Declarations and language-defined command data of chapters 02 and 04; no saved grants or execution token |
| Declared parameters | Typed parameter values/defaults used by the module, with exact values on decision paths |
| Boundary and principle references | Chapter 07 boundary declaration; chapter 03 `PrincipleRecord` data and its evidence references |

| Lockfile field | Stored value and validation |
| --- | --- |
| Format/version | Lockfile data schema |
| Root manifest digest | Exact manifest resolved by this lockfile |
| Resolved modules | Every resolved dependency's module/content digest, edition and source inputs |
| Local resolution entries | Locations used only to find bytes; the digest decides identity |
| Resolution/version inputs | Chapter 02 resolver contract version and other inputs it declares; a changed input invalidates reuse |

- **PS1.16 [D]** Workspace records are durable data changed only through invoked workspace commands (INV-2,
  defaults 15 and 16, KN4.4). Their provenance field travels with their payload, including through file export and
  re-import (BM2; DT-L1 R2-E3). The following common form does not create a new source of grants, evidence or state.

| Workspace record field | Stored value and validation |
| --- | --- |
| Format/version, kind, record identity | Schema, payload kind and workspace identity; content digest apart from the record's local name |
| Payload | One typed stored value; nested words and evidence remain references to their exact content |
| Model and dependencies | `IdentityBlock` when model-dependent, plus source/module/content dependencies actually used |
| Provenance field | Versioned chapter 07 `ProvenanceBlock`, as carried by PS1.32 to PS1.34 |
| Durable action/revision reference | Store-owned workspace action and revision information from chapters 01 and 08; no live process handle |

- **PS1.17 [B]** State that must agree is saved in one recoverable transaction (R-06, KN4.2). A failed write or
  rejected input changes neither a workspace payload nor the session's committed assistance provenance (R-12).
  Saving a workspace never saves reusable execution permission (V03.5). Preview tokens, user-action tokens and
  process-epoch grants are outside every durable form here.

## 4. PS1: words, macros, recipes and protection

- **PS1.18 [B]** Stored words keep the literal chronological letters, their alphabet, model identity, order-pinned
  menu identity, contract revision and recorded start digest (DT-J1 E1 and E3; ENG-0J store section). The following
  table is the word form. A macro adds a workspace name and versioned metadata to that word, with no independent
  execution authority. Canonical relative arguments retain every pin that gives them meaning (DT-M1 M3).

| Word field | Stored value and validation |
| --- | --- |
| Format/version and alphabet | Retained signed generators, or menu letters; the alphabet is never inferred from integer ranges |
| Identity | Chapter 01 `IdentityBlock`; jumbling includes both menu identity and order digest and the contract revision |
| Literal letters | Ordered signed generator IDs, or ordered `(grip,item)` letters against that identity; no replacement by an equality key |
| Start digest | Digest and level of the input at which it was recorded; an origin, not a certificate for later states |
| Named parts | Optional named literal parts, `same_as` and `inverse_of` relations, with their pinned definitions |
| Exact expanded length | Big integer counting letters in the declared alphabet, computed before expansion |
| Content references | Under JI.18's flat draft, mandatory literal chunk references plus optional pinned annotation definitions; their agreement is checked |
| Provenance field | Versioned block from chapter 07, retained on every copy and conversion |

| Macro field | Stored value and validation |
| --- | --- |
| Format/version, workspace identity and name | Stable saved-entry identity and name, separate from word content identity |
| Word or retained recipe | Exact word reference or recipe value, preserving its identity and literal semantics |
| Named parts and length | The word's part definitions and exact expanded length; a name does not substitute for a definition |
| Metadata/evidence references | Recorded origins, full collateral claims and certificate digests, with their scope |
| Provenance field | The macro's and payload's labels remain attached through copying, renaming and import |

- **PS1.19 [D]** Under JI.18's flat draft, stored menu words carry all literal chunks. Named parts are annotations
  over half-open spans; they never generate replay letters. FP.13 encodes JI.21's local and pinned targets. Admission
  checks closure, rejects cycles and compares `same_as` and `inverse_of` with the actual literal spans under the
  pinned menu. A language/source part graph may expand lazily, but its exact length and expanded cost are checked
  before producing the stored word. Retaining that graph does not admit or store its expanded word.
- **PS1.20 [O]** JI.18 recommends flat ia16 storage with named annotations; FP.13 gives its draft wire form. The
  carrier's adoption stays open for EN3 and the format-freeze ruling. Under that form, chunks are mandatory and
  storage budgets count every literal byte, even when a small source graph generates them. A closed referenced-part
  alternative would need its own reviewed byte profile, closure/expansion rules and streaming vectors. Either form
  preserves chronology, exact length,
  dependencies and pre-replay admission; neither imposes a mathematical length cap.
- **PS1.21 [B]** A word has no type-level length cap, while expansion beyond DT-J1 X3's bound is refused with its
  exact length reported (DT-J1 E3; KN2.10). A word containing a jumble letter is refused while Jumble is off
  (DT-J1 E1). Storing a word does not establish that every prefix is admissible at any later revision. Checking,
  preview, protection and commit replay its literal letters at the applicable revision; an equality key never
  supplies the executed path (partition default 10).
- **PS1.22 [D]** The recipe form below preserves the existing meaning of `core.Model.normalize` and `expand`
  (partition section 13). The recipe alphabet counts expanded signed-generator primitives, not menu letters or
  recipe steps. The limits in PS1.10 apply to the entire recipe, including every star expansion.

| Recipe field | Stored value and validation |
| --- | --- |
| Model identity and carrier version | Exact retained model and schema that interpret the steps |
| Ordered steps | 1 to 256 `word` or `star` steps |
| Word step | `kind=word`, `moves`: 1 to 100,000 signed generator IDs from ±1 to ±1200, excluding zero |
| Star step | `kind=star`, `orbit`, `node`, `sign`; sign is ±1, with the existing default +1 accepted during compatibility conversion |
| Expansion | For a positive star: setup inverse, relocation inverse, seed, relocation, setup; a negative star inverts this complete word |
| Exact total | Expanded primitive count; at most 3,000,000 over every step, regardless of chunks |
| Provenance reference | The enclosing macro, event or workspace block, preserved by conversion |

- **PS1.23 [B]** Recipe normalisation retains the executed steps and their literal words (`core.Model.normalize`;
  R-11 as qualified by partition default 10). It never free-reduces away prefixes before a Strict protection check.
  Conversion to streamed chunks leaves total expansion, chronological effect, protection and one operation's
  transaction boundary unchanged (V05.1 and partition section 13).
- **PS1.24 [D]** Macro sets and protection sets are separate data-core records (partition section 13). Importing a
  set grants no execution permission, and an unconvertible protection requirement cannot be dropped (path B,
  migration-options item 4).

| Macro-set field | Stored value and validation |
| --- | --- |
| Format/version and identity | Carrier schema, retained model or each entry's explicit puzzle identity |
| Entries | Macro records or digest references with their names, exact payloads and provenance fields |
| Import origin | Source digest and conversion record; zero entries remain zero entries |

| Protection-set field | Stored value and validation |
| --- | --- |
| Format/version, identity and level | Model and protection level; label-based in Full, pose-based inside Jumble, as KN1.11 |
| Members | Protected orbits and any other supported canonical protected objects; no inferred visual selection |
| Protection contract | Net or Strict and the relevant protection-rule version, as defined by the kernel |
| Origin and conversion | Original requirement and source digest; failure to represent it stops activation/import |

```text
operation expand_stored_word(word: StoredWord, bound: Budget) -> LiteralWordStream
  effect:  pure, plus granted reads for digest-pinned parts
  cost:    exact expanded length in the word's alphabet and the declared memory/output charge
  errors:  KN1:input.identity_mismatch, PS1:input.noncanonical_value, PS1:resource_limit.word_expansion,
           PS1:resource_limit.persistence_size, PS1:capability.provenance_incompatible
```

## 5. PS1: bindings and other workspace forms

- **PS1.25 [B]** Binding and key-map storage follow DT-M1's settled K5, K6 and absolute-argument rule. The following
  fields store the chapter 04 `Binding` meaning. Every relative value retains its fixing identity and revision;
  stale list-row numbers, key labels and camera-relative descriptions are never reinterpreted as absolute objects.

| Binding field | Stored value and validation |
| --- | --- |
| Format/version, key and context | Key sequence, routing context and binding schema |
| Command | Command ID and contract reference, never executable pixels or a control's label |
| Absolute arguments | Resolved cap, position, slot, piece, orbit, exact rotation or word; any retained relative form includes its fixing pins |
| Physical codes and layout relation | Portable physical codes; a character-based binding also records its keyboard layout and conversion dependency |
| Provenance reference | Workspace record and any referenced word's provenance; no saved token |

| Key-map field | Stored value and validation |
| --- | --- |
| Format/version and bindings | Ordered or explicitly keyed binding collection; schema fixes its canonical order |
| Contexts and layouts | Routing classes and each layout against which character-derived bindings were checked |
| Applied grip map | Stored absolute caps and transported frames of the applied map, including model dependencies |
| Alias/conversion record | DT-M1 alias-table version and source content digest for migrated command bindings |
| Workspace provenance | PS1.16 provenance field; imports never discard nested word labels |

- **PS1.26 [B]** Key-map checks reject duplicates and check the prefix-code property within each routing class;
  a group prefix is not also a complete binding (DT-M1 K6). Character-based bindings are checked per recorded layout.
  Camera, filter, twist and focus changes do not rewrite the applied grip map; only the rebind command changes it
  (DT-M1 K2, XM2). Saving the map records data, with no decision about its interface presentation.
- **PS1.27 [D]** Layouts and declared tool parameters store the data consumed by chapters 04 and 10 (partition PS1).
  Their common form follows INV-4, INV-6 and default 16. It specifies neither presentation nor the available views.

| Layout field | Stored value and validation |
| --- | --- |
| Format/version and provider | Data schema, provider module/source digests and edition |
| Declared parameters | Typed parameter/default values; exact values wherever a decision reads them |
| Object and representation references | Canonical object IDs with model/level, and chapter 10 representation dependencies |
| Display-only data | Separately typed approximate values with declared precision; no mechanical input or authority |
| Workspace/provenance reference | Owning workspace record and its provenance block |

- **PS1.28 [D]** Filters and preferences use distinct typed records even when an old carrier nests them under
  `prefs` (partition section 13). Existing field names below come from `session.py`; their presence does not carry
  the old frontend into 1.0. Filter interpretation and preference conversion are explicit versioned dependencies.

| Filter field | Stored value and validation |
| --- | --- |
| Format/version and model | Filter contract and model of its canonical objects |
| Ordered rules | Existing `rules` contain `expr` and `style`; compatible conversion preserves rule order |
| Named sets and context inputs | Existing `named_sets` distinguish `identity` and `position`, with canonical IDs and fixed model dependencies |
| Expression contract and source | Filter-language/version reference, original expression bytes or digest, and conversion outcome |
| Provenance reference | Owning workspace record; a filter's visibility never changes mechanics |

| Preference field | Stored value and validation |
| --- | --- |
| Format/version and typed entries | Preference schema and declared values; unknown entries have an explicit import outcome |
| Existing core preferences | `orbit`, `protected`, `selected`, `rules`, `pin_safety`, `inspection`, `focus_color` where supported by conversion |
| Existing workspace-bearing entries | `layout`, `keybinds`, `presets`, `bookmarks`, `named_sets`, `macro_library`, `filter_sequence`, `sequence_index`, `assistance_profile`, `input_profile` |
| Existing display data | `camera` and `view`, retained only as typed display data; no float used to decide a mechanical claim |
| Dependencies and provenance | Model/contract references and enclosing workspace provenance; protection requirements remain separately validated |

## 6. PS1: checkpoints, recordings and runs

- **PS1.29 [B]** Checkpoints retain an independently checkable replay path (partition section 13; path B
  migration-options item 5; ENG-0J A5). The table distinguishes a replayed retained witness from a label checkpoint
  with its certificate and exact exit poses. Labels alone cannot supply exit poses or establish reachability.

| Checkpoint field | Stored value and validation |
| --- | --- |
| Format/version, name and identity | Checkpoint identity, model, mode and contract references |
| State and digest | Complete labels or exact pose table and IDs as applicable, with a digest carrying its level |
| Replay root | Solved root, or a previously verified root/exit with its immutable digest and reachability evidence |
| Replay path | Parent-linked events with each event's literal recipe/word, pre/post digests and identities; every offered checkpoint has its own path |
| Checkpoint preferences | Validated preferences captured at this checkpoint, separately from event replay |
| Jumbling episode | Start witness or resumed exit, episode journal, menu identity/order, contract revision, end witness or label certificate |
| Label-crossing certificate | Pose digest, labels digest, ν=0, C5 convention, certificate-program SHA-256 and contract revision; exact exit pose state retained |
| Recovery/dependencies | Durable store dependencies and applicable recorded assistance; no preview or user-action token |

- **PS1.30 [D]** A recording stores EV4.10's ordered `RecordedStep` values in the form below. Its three uses are
  conversion to a word draft or source for one complete preview and one user commit, stepwise playback with one
  user action per commit, and unattended playback after one starting user action. View and workspace steps may
  play unattended; commits stay manual (owner, 6 November 2026, LQ11; owner, 6 November 2026, decision 3).
  EV4.17 to EV4.23 govern eligibility, outcomes, recovery and conversion. No stored field grants commit permission
  under INV-3, and no screen coordinate identifies an object.

| Recording field | Stored value and validation |
| --- | --- |
| Format/version, source identity and edition | Recorder contract and optional generated-source dependency |
| Ordered steps | EV4.10's RecordedStep values in invocation order, including refusals and unresolved terminal outcomes under EV4.11 |
| Revision bindings | Recorded revisions and immutable input digests; a process-bound revision is historical context, not a live grant |
| Word and workspace references | Exact content references and their provenance fields |
| Outcome linkage | Each step's originating invocation and terminal receipt reference, or explicit absence; missing knowledge follows EV4.19 |

| RecordedStep field | Stored value and validation |
| --- | --- |
| `row` | Stable command-table row ID, distinct from the invocation's CommandId |
| `arguments` | Resolved canonical arguments with canonical object identities; no token and no operation preview ID (KN5.9). A `FilterPreviewRef` argument stays as recorded for EV4.14's check |
| `word` | WordRef or none; operation.execute retains the word behind its original preview, never that preview ID |
| `depends` | Ordered List<RevisionRef> of the revisions the step read, retained as recorded dependencies |
| `module` | Digest<source> for an overlay step, or none; no substitution by another module with the same name |
| `action` | ActionNumber or none, naming the durable action the invocation took (LI.9); undo/redo conversion matches this number to the recorded hint.target |
| `command` | The originating invocation's CommandId under KN5.21 |
| `request` | Digest<request> binding that original invocation; it cannot authorize a new one |
| `outcome` | Terminal receipt reference or none, preserving KN5.21's actual status and EV4.19's unknown-outcome treatment |

  These are versioned data fields under PS1.1 and PS1.13. The common byte profile, exact-number encodings and
  digest roles apply; optional absence is explicit. A replayed child has a new command/request binding and its
  receipt is tracked by PS1.44, without overwriting the captured invocation. Capture excludes `recording.*`.

- **PS1.44 [D]** `PlaybackState` is a durable workspace record in PS1.16's carrier, outside the undo order, with
  FP.37's wire form. Its payload has this shape; `Nat`, digests, references and receipts retain their existing byte
  contracts under PS1.3, PS1.5, PS1.13 and KN5.21:

  ```text
  PlaybackState = {
    playback:  PlaybackId
    recording: Digest<recording>
    step:      Nat                                   # next step
    pending:   { command: CommandId, request: Digest<request> } | none
    actions:   List<{ recorded: ActionNumber, played: ActionNumber }>   # sorted by recorded; each recorded once
    status:    running | waiting(step) | stopped(PlaybackStop) | finished
    last:      Receipt?
  }
  PlaybackStop = code(Code) | outcome(refused | failed | cancelled | unknown_outcome)
  ```

  The command layer writes `pending` durably before dispatch, including a shell manual-step action. Cursor,
  pending, status and last receipt are updated together in a recoverable write; the cursor creates no undo target.
  Recovery looks up the pending command/request through KN5.23 before continuation, reconciles a known receipt
  at most once and stops on `unknown_outcome` without a fresh retry (EV4.19, EV4.20). An explicit `from`
  follows EV4.22's table after pending reconciliation; an unrelated invocation cannot move the cursor. The recording
  digest prevents a saved cursor from silently applying to edited recording bytes.
  `actions` holds one entry for each played step that took a durable action: the step's recorded `action`
  (EV4.10) and the action number the played step took. Undo and redo steps enter it like any other step, and the
  entry is saved with the cursor. EV4.18 reads it before an undo or redo.
  `stopped(code)` carries a stop with a code of its own. `stopped(outcome)` carries EV4.19's typed child outcome;
  any code that outcome has stays in the unchanged receipt in `last`, and no code is invented for it.

- **PS1.45 [D]** `c600-proof-obligation-draft` is the export format for LB5.18's `research.obligation.export`
  (owner, 6 November 2026, LQ15; owner, 6 November 2026, decision 4; owner, 6 November 2026, decision 9).
  It carries a typed statement (PS1.46) and its Lean 4 source over a restricted, versioned vocabulary, with the
  statement and translation dependency
  pins. Text is UTF-8/LF and its source bytes retain their digest; the export is outside the language-neutral data
  core. An untranslatable statement refuses with `LB5:input.obligation_untranslatable` and writes nothing. Export
  uses a granted file capability and supplies no process or network access (default 2).

  1.0 has no return format or proof-record import row and adds no check mode. `formal proof` remains in EVD.3
  with no writer in 1.0 (EVD.64); imports cannot assert it (EVD.47). Export promotes no claim and touches no commit
  gate (RTE.28). A failed Lean run is no refutation and enters no record. The translation prelude lives in the
  repository outside the shipped software. Lean is neither shipped nor a runtime dependency and is not on the
  allowlist; a project check that runs it needs the owner.
- **PS1.46 [D]** Export reads one typed statement: a chapter 06 `Statement` (section 2, EVD.8, EVD.9) whose StatementId
  has FP.19's typed form. Its bytes must resolve and carry the body `{hypotheses:[Prop],conclusion:Prop}`. A named ID
  has no body to translate, so it refuses with `LB5:input.obligation_untranslatable`. The body uses vocabulary
  `c600-obligation-vocabulary-draft-1`:

  ```text
  Sort = Nat | Int | Rat | named(name)
  Term = var(name) | nat(n) | int(n) | rat(n, d) | sym(name, args)
  Prop = true | false | eq(a, b) | le(a, b) | lt(a, b) | not(p) | and(p, q) | or(p, q) | implies(p, q)
       | forall(name, sort, p) | exists(name, sort, p) | pred(name, args)
  ```

  `sym`, `pred` and `named` names come from a versioned symbol table. Each row maps a name to a Lean identifier of the
  translation prelude, with its arity, and the table's digest is among the export's pins. Premises and guards arrive
  as hypotheses of the body, and each becomes a named hypothesis of the Lean statement. A name outside the table, a
  wrong arity or a form outside the vocabulary refuses with the same code and writes nothing. The output states the
  theorem with no proof (`by sorry`). The `obligations` vectors of `format-packet/` hold a supported statement with
  its exact Lean text and the refusals.

- **PS1.31 [D]** A `RunRecord` stores the inputs and result of chapter 09's `ReproKey`, using the following fields
  (partition section 13, INV-10, KN7.3). It records an incomplete result without turning its stop reason into a
  negative claim. A completed run and a check of its result are separate records.

| Run-record field | Stored value and validation |
| --- | --- |
| Format/version and identity | Data schema and chapter 01 `IdentityBlock` |
| Snapshot digest | Immutable input snapshot digest with its level; derived inputs also retain derivation and reachability references |
| Source, edition and modules | Source/module digests, source edition, IR version when applicable, and lockfile dependencies |
| Parameters and seeds | Complete exact parameter values and explicit seeds; no ambient randomness or clock input |
| Budgets | Granted and used deterministic `Budget` fields, with their cost-contract versions |
| Outcome and stop reason | Chapter 01 `JobResult` outcome, named exhausted budget/error/cancellation and declared coverage |
| Evidence and outputs | Output content digests, `EvidenceEnvelope` references and checker traces or their re-derivation inputs |
| Reproducibility key | Complete chapter 09 `ReproKey` plus its version and digest; not a shorter substitute omitting inputs |
| Safety-stop observation | If applicable, separate wall-clock stop metadata; such a run is incomplete and not reproducible |
| Provenance | Profile, producer/module origins and assistance references defined by chapter 07 |

## 7. PS1: the provenance field

- **PS1.32 [D]** Every stored word, macro and workspace record carries a versioned provenance field (BM2,
  partition section 13; DT-L1 R2-E3). The field carries chapter 07's `ProvenanceBlock`; this chapter defines only
  its persistence obligations. Copying, renaming, converting to source, or exporting and re-importing a value retains
  its labels and dependency pins. A value selected through a state-derived branch cannot become unlabelled merely
  because its literal letters came from an own macro. Chapter 07 defines propagation and sink checks.

| Provenance field | Common stored obligation | Flow labels, if LQ21 chooses them | Capability separation, if LQ21 chooses it |
| --- | --- | --- | --- |
| Version and form | Explicit block version and the form that produced it | Static or run-time flow-label contract and checker/executed-source pins | Capability-contract version and producer's recorded state-read fact |
| Author and producer | Chapter 07 `AuthorClass`, program/module digests or an explicit manual/import origin | Same common fields | Same common fields |
| Profile and origin | Producing profile, origin content digest and import/conversion record | Original labels survive the profile boundary | Original state-read/profile classification survives the boundary |
| Dependencies | Every dependency the selected form records, including stored inputs | Data and control dependencies; indirect choice retains its labels | Coarse fact of whether the producing program read state-derived data; no invented fine-grained trace |
| Explicit pick | Chapter 07's record of the solver's pick and the content/target it fixed | Pick/declassification reference checked by policy | The same fixed exception checked by policy |
| Declassification references | Only fixed entries enabled by chapter 07 policy; no program-declared declassifier | Flow evidence required by that contract | Exception evidence required by that contract |
| Legacy import | Original labels and source digest, including missing legacy detail | No fabricated branch history or checker approval | No fabricated assertion that a legacy program did not read state |

- **PS1.33 [D]** Provenance remains meaningful under either LQ21 form by recording which form/version produced it
  (BM2; DT-L1 R2-E3). A reader lacking that form's required information retains the bytes as recorded and refuses
  a policy-gated use that it cannot justify; it does not downgrade to an unlabelled word. A switch of forms is a
  recorded migration, not a label reset. 0.4 macros enter labelled as the solver's own, imported, with the captured
  source digest. That records their import origin and grants no research-to-solving exception.
- **PS1.34 [D]** FP.17 assigns independent nested provenance version 1, with explicit form and contract/checker
  pins, complete field spellings and compatibility outcomes under both LQ21 forms. H9/BND.23 fixes the required
  no-run `profile` as the explicit value `none`, only for manual/import producers. Hand-typed words have empty flow
  labels or `read_state(no)`; imported 0.4 macros keep `depends: not_recorded` and the source digest. Missing fields
  still read `unknown`, never this explicit no-run value. FP.18's four complete examples fabricate neither a run
  nor permission. Unknown versions/forms are preserved and cannot justify policy-gated use; changing form is a
  recorded migration retaining the old block. This storage draft decides no LQ21 policy or BND.74 question.

## 8. PS1: evidence and existing carriers

- **PS1.35 [D]** An `EvidenceEnvelope` has the identity and scope fields below under LQ10's facts envelope
  (partition section 13; DT-L1 M2, R2-E1, R3-M1). Every scope field of a stored claim is immutable, and none is
  revision-bound context. A scope need not hold an input digest: a theorem and a bound on D[k] have none (EVD.10).
  Transient check context from 0.4 state C is never persisted as evidence: revision, current role, goal and protection
  context do not become a durable permission. Chapter 06 owns claims, statement IDs, scope, basis, checks and every ordering
  rule. The envelope retains the data those rules need; it does not define a parallel evidence model.

| Common envelope field | Stored value and validation |
| --- | --- |
| Format/version and payload form | Envelope schema and explicit facts form under LQ10; derived caches name all fact inputs and function editions |
| Identity | Model, puzzle, menu identity and order digest, contract revision; chapter 01 identity vocabulary |
| Statement and scope references | Chapter 06 statement ID and typed scope; every digest states its level or content role, at least as fine as the subject level (EVD.11) |
| Statement form and negation pair | Chapter 06 statement form, and the statement it negates where it has one (EVD.9) |
| Subject level | The identity level the statement is about |
| Immutable inputs | Input digest(s) where the statement has them, source/module digests, declared family/alphabet/bounds or admitted-domain parameters covered by the statement |
| Scope-membership evidence | The evidence that the object lies in the scope, kept apart from the claim itself (EVD.13) |
| Search record | Completeness or closure of a search, stored apart from the job outcome (EVD.13) |
| Covered set of a leaf | `covered` of chapter 06, computed from the leaf's own record (EVD.59) |
| Premise class | The class of each premise, with its statement ID (EVD.13) |
| Guards of a statement | The class and name of each guard (EVD.36); a guard is part of the statement, so its ID binds it (EVD.8) |
| Sources and original labels | Source content digests, record/field locations and labels preserved word for word, with stage and source version |
| Attribution | Author, reproducer, scope setter and review class when recorded; absence stays absent |
| Engineering evidence class | Source/fixture, synthetic, windows, performance or human where applicable; separate from mathematical basis/check |
| Certificate/check dependencies | Kind, certificate byte digest, checker source digest, report/trace references, recorded check mode, and the tool digest of each check with the declared common base (EVD.13) |
| Reachability reference | Named-puzzle reachability envelope when the input is a derived state; raw pose validation does not establish it |

- **PS1.36 [D]** The payload stores facts and derives judgements (owner, 6 November 2026, LQ10; DT-L1 R2-E1).
  Its fields are below. Their meaning is checked by chapter 06; no numerical evidence ranking is introduced here.
  Original labels and immutable scope survive. Field writers are EVD.64's, enforced at the record sink by EVD.65;
  retaining a field is no permission for a program or import to assert it.

| Facts payload |
| --- |
| Leaf facts, their source labels and scope/statement references |
| AND/OR nodes and edges, preserving the distinction between one argument's inputs and independent arguments |
| Certificate, exact-computation, exhaustive-set, sample and proof-source facts, as actually present |
| Check identity, result and mode: reused exact transport, fresh exact audit or actual replay, without inventing an order between modes |
| Versions/digests of the functions for the partial order, summaries and display words |
| Optional derived caches with all recorded inputs; facts remain available for recomputation |

- **PS1.37 [D]** An envelope rejects a dangling dependency, contradictory identity/scope, failed content digest or AND
  node with no children (EVD.40, I8) instead of silently strengthening its evidence (INV-5, INV-7; partition 8.1, DT-L1 M2 and R2-E1). A source label
  such as `measured`, `reported` or `passed` is data to interpret through PS3's table, not an evidence verdict by
  spelling. A job outcome is separate from basis and check; a cut-off job yields no negative claim, while a checked
  witness can still support its own claim (KN7.4).
- **PS1.38 [D]** The facts-envelope experiment constructs envelopes for the stage 14 to 19 corpus and
  reports node counts and bytes against the baseline of **497 records, 13,412,769 bytes at Git HEAD**: 417 `.json`
  files and 80 `.json.gz` files counted as stored, with no JSONL (partition section 13; DT-L1 R3-E2, source evidence
  counted there; the earlier 13,387,282 came from a Windows checkout in which six case pairs collapse). The corpus is
  read from Git blobs through a manifest of path, blob digest and size, and no importer or test walks a checkout
  (default 13). The report counts leaf/AND/OR nodes and edges, separates canonical uncompressed bytes from stored
  compressed bytes, and accounts for shared dependencies and unmapped labels. It reports original records and
  envelopes under the same inclusion rule. It also enumerates all stored-field combinations admitted by the
  chosen model: each gives exactly one display word or a typed inconsistency. The unreviewed source experiment in
  `work/experiments/language-lx/envelope-facts/RESULT.md` now reports 497 partial envelopes and zero complete typed
  envelopes: 15,210 fact bundles, 11,467 incomplete leaf slots, no AND/OR nodes or edges, and 10,424,730 canonical
  metadata bytes. Its complete accounting, serializer pins and display collisions are retained there; those bytes
  are not the FP.7 profile and establish no complete lossless typed mapping. FP.33 states the evidence limits.
  No timing result is claimed. Experiment code is allowed (owner, 6 November 2026, LQ18); these first-pass results
  do not establish a complete typed mapping or decide any waiting question.
- **PS1.39 [D]** The following tables retain the fields of existing carriers (default 3). They document input
  compatibility, not an instruction to extend a v1 file with new language fields or to treat it as the complete
  migration package. Additional language data uses a versioned enclosing record under PS1.13.

| `C600-STUDIO-SESSION-v1` field | Existing meaning and import boundary |
| --- | --- |
| `format`, `model_id`, `root` | v1, retained model identity, `labelled_identity` replay root |
| `events` | Current ancestor chain, chronological; event `id`, `recipe`, `pre`, `post`, `primitive_count`, `stars`, `note`, `assistance` |
| `final_state` | Labels digest after replaying the entire chain |
| `prefs` | Existing source preferences; ordinary 0.4 log import validates the object but does not apply them |
| `native_windows_equivalence` | Existing false field; not evidence of native equivalence |

| `C600-STUDIO-CERTIFICATE-v1` field | Existing meaning and import boundary |
| --- | --- |
| `format`, `model_id`, `recipe` | Exact retained model and normalised recipe from `Session.preview` |
| `source_to_destination_sha256` | SHA-256 of source int32-LE bytes followed by destination int32-LE bytes |
| `primitive_count`, `star_count`, `support` | Decimal-string primitive count, star count, full collateral support per orbit |
| `pre_state`, `post_state` | Labels digests at the preview's input and result |
| `evidence`, `native_windows_equivalence` | Recorded evidence text and false native-equivalence field |
| `certificate_id` | Digest of the initial canonical public object before later diagnostic/context fields are added, exactly as `session.py` computes it |
| Later public fields | `conflicts`, moved counts, `preview_ms`, `note`, `assistance`; diagnostics and context, not additional inputs to that old `certificate_id` |

- **PS1.40 [B]** The versioned optional jumbling section stores exact states and episodes (ENG-0J store section,
  A5). With no Jumble episode it is absent; that session stores the same retained data it would without jumbling
  support (KN4.5). An interrupted crossing restores an episode as on or rolls back to its last committed revision.
  The prototype's representation validation alone proves neither admissibility nor certificate truth.

| `magic600-session-proto/1` field | Existing representation and boundary |
| --- | --- |
| `format`, `revision` | Experimental wrapper version and nonnegative session revision |
| `retained` | Opaque retained JSON, preserved without a replacement retained-session schema |
| Optional `jumbling` | The section below; absent for never-on sessions, as in the prototype |

| `magic600-jumbling-store/1` field | Existing representation and required check |
| --- | --- |
| `format`, `episodes`, `words`, `scrambles` | Optional versioned section; scramble journals remain separate from the solver's own twists |
| Episode `identities` | Model asset digests, `menu`, `menu_order`, `contract_revision`, adapted explicitly to chapter 01 identity fields |
| Episode `start` | Retained `word` from solved, or earlier label-exit episode and `retained_moves` |
| Episode `journal`, `state`, `end` | Exact twist records, state with revision, and open/witness/label exit; full replay checks semantic validity |
| State `pose_table`, `pose_ids`, `digest` | Exact table entries `k` or Q(√5) matrix `m`; `u32le-base64` IDs; independently recomputed poses digest |
| Label exit | `certificate` and `exit_state`; certificate pose digest equals exit state's and episode state's digest |
| Certificate fields | `pose_digest`, `labels_digest`, `certificate_program_sha256`, `c5_convention`, `contract_revision`, 45 exact zero ν coordinates |
| Stored word | `letters`, `menu_size`, `identities`, `recorded_at`, optional `name` and `parts`; added provenance needs its declared enclosing version |
| Scramble | Separate identities, journal and optional exact integer seed; no relabelling as manual input |

| `magic600-jumbling-render/1` field | Existing representation and boundary |
| --- | --- |
| `format`, `dimension`, `pieces`, `poses`, `revision`, `digest` | Render header and engine poses digest; dimension unsupported by a reader is refused |
| `model_identity`, `menu_identity`, `contract_revision` | Existing engine identity fields; order-dependent word references retain their separate order pin |
| `files` | Array SHA-256, shape and type; `pose_index.i32`, `poses.f32`, `pose_lattice.i32` |
| Other engine-provided records | Exact/corresponding approximate certificate data and revision dependencies, as specified by the render contract |

- **PS1.41 [D]** The render export is a drawing carrier, not a checkpoint, reachability certificate or exact pose
  store (INV-4, INV-8; render contract section 1). A float render array is never an import path to mechanical state.
  Its array checks and engine digest checks are separate. This chapter adds no presentation rule to that contract.

## 9. PS2: editions and recorded migration

- **PS2.1 [D]** Source has editions; data has schema versions (partition PS2). A manifest or run record can reference
  both. An engine epoch or workspace revision from chapter 01 is neither of these. A schema change cannot silently
  select a newer source edition, a new word alphabet, another menu order or another exact-arithmetic convention.
- **PS2.2 [D]** A stored meaning never changes silently (partition PS2). A reader dispatches by the recorded format
  and version before interpreting fields. It checks the `IdentityBlock`, digest levels and required dependencies
  before using a decoded value. An unknown/newer version is refused for execution, activation or evidence reuse;
  its original bytes may remain recorded. Treating it as the nearest known version would violate INV-7.
- **PS2.3 [D]** A migration is one-way and recorded (partition PS2). The following record follows from the need to
  identify both the saved meaning and its conversion. FP.29 assigns its candidate field spellings under PS1.13.

| Migration-record field | Stored value and validation |
| --- | --- |
| Source | Old format/version/edition, exact source content digest and identity |
| Target | New format/version/edition, resulting content digest and identity |
| Converter | Converter identity/source digest, conversion-contract version and declared inputs |
| Outcome | Converted, retained as recorded, or refused, with typed reasons per affected item |
| Preservation evidence | Replay/hash checks, label and protection preservation checks, and explicit semantic-change notes if any |
| Dependencies | Mapping-table/function versions and digests used in conversion |

- **PS2.4 [D]** Conversion validates complete input before publishing new durable data, retaining the old bytes and
  recovery path (PS2, R-06, `AGENTS.md` persistence rule). Failure leaves the active generation unchanged. A
  migration that changes meaning identifies the change explicitly and cannot claim to preserve old evidence.
  Reopening an old file with a new reader does not itself authorise a model-identity change.
- **PS2.5 [D]** Stored words keep their meaning across editions (partition section 13). Migration preserves the
  alphabet, literal order, identities, part definitions and expanded length, recorded start digest and provenance.
  A changed equality-key algorithm can be re-derived and versioned; it never changes the executed letters. Relinking
  a named part to a newer definition is a changed word requiring a different content identity.
- **PS2.6 [D]** Changing the partial-order, summary or display-word function is an
  edition with a recorded note (partition section 13; DT-L1 R2-E1). It records old and new function editions/digests,
  unchanged fact-input digest and the resulting derived-output digests or inconsistencies. The old interpretation
  stays reproducible. This follows the facts envelope chosen by the owner (owner, 6 November 2026, LQ10).
- **PS2.7 [B]** Artefact reuse requires equality of all recorded inputs, checked after the build (R-08). For stored
  compiled modules this includes source, manifest/lockfile, edition and compiler/IR dependencies defined by chapter
  02. For derived evidence it includes fact graph, mapping table and function versions. For replay it includes
  input digest/level, literal word, model/menu/order/contract and the check's declared dependencies. Equal output
  hashes do not excuse changed inputs. A mismatch causes recomputation or a typed refusal.
- **PS2.8 [D]** A cached check is about its immutable inputs, not about a current session merely because its labels
  happen to agree (KN1.8, KN2.9; partition 8.1, DT-L1 R2-E1). Reuse at a new revision needs a new check or a recorded
  exact transport justified by the relevant chapter 06 lemma. Import does not create such a transport. Caches fail
  closed when a recorded input or function version is unavailable.
- **PS2.9 [D]** Future extension/unknown-field handling cannot discard a label, a protection requirement, a literal
  part or a claim dependency while reporting successful preservation (INV-5, INV-7, BM2). FP.10 gives the candidate
  extension mechanism: unknown optional inert data is preserved, unknown required extensions refuse active use,
  and changes to required meaning need a new version. An uninterpretable record remains recorded or is refused
  rather than partially activated.

```text
operation migrate_record(input: RecordedInput, target: FormatRef, converter: ConverterRef) -> MigrationResult
  effect:  granted file reads and writes, or an invoked workspace/import command
  cost:    complete validation, conversion, replay and output charges
  errors:  PS2:input.persistence_version, KN1:input.identity_mismatch, PS1:resource_limit.persistence_size,
           PS1:input.digest_mismatch, PS1:capability.provenance_incompatible
```

## 10. PS3: path B import of 0.4

- **PS3.1 [B]** Path B imports 0.4 one way (owner decisions of 2 and 3 October). It copies the database under the
  session lock, never modifies the original session directory and never opens a user database with `immutable=1`.
  The 3 October amendment permits only the locked byte copy of `session.sqlite3` and `-wal` through read-only
  handles under the write guard, with SQLite recovery and subsequent backup confined to that copy. `-shm` is never
  opened for that copy operation. No live-source content is read before the lock or after its release.
- **PS3.2 [B]** The importer locks the existing `engine.lock` without creating or writing it, and refuses an active
  0.4 engine or a missing required lock (migration-options items 1 and 2; amended exporter re-plan). The whole
  source-copy interval uses the preservation guard and watch contract of probe report section 7. Acceptance needs
  an uninterrupted, drained watch with no event, overflow or error, and equal snapshots and file digests; it makes
  no timing-based attribution of a writer. The copied file digests equal those taken under the lock.
- **PS3.3 [D]** The importer reads the staged copy into the following targets (partition section 13; migration-options
  items 4 to 6; DT-M1 K6). The report names every target's converted, recorded or refused outcome. The ordinary
  `C600-STUDIO-SESSION-v1` head-chain export is insufficient for these targets because it omits checkpoints and
  carries workspace data only as untyped preferences. The re-plan reuses the older exporter draft's ZIP layout and
  field inventory with its recorded changes. Its candidate manifest spelling is fixed by PS1.13/FP.24. The package is a
  migration carrier around existing data. The reused draft's member checks exclude absolute/traversal paths,
  links, duplicates and encrypted entries; PS1.10 and PS1.11 give the shared limits obligation. The following tables
  give its fields and target conversion checks without silently changing session v1.

| Path B draft manifest field | Stored value and validation |
| --- | --- |
| `format`, `format_version` | Candidate `C600-MIGRATE-draft`, integer 1; FP.24 distinguishes the older exporter spelling |
| `producer` | Producer name/version/source identity; the re-plan moves production to the 1.0 importer |
| `model_id` | Exact retained model; disagreement refuses import |
| `created_utc` | Recorded creation metadata, separate from replay inputs and evidence strength |
| `head_state_hash`, `head_event_id` | Archived head and its labels digest |
| `entries` | Each non-manifest member's path, byte count, SHA-256 and role; no unlisted member |
| `options`, `privacy` | Explicit log-inclusion choice and the draft's privacy contract; no machine paths in public metadata |

| Path B draft member | Stored fields and validation |
| --- | --- |
| `session/record.json` | Neutral event chain, checkpoint metadata and label-file references, preferences outside workspace, attempt/timer record |
| `session/checkpoints/<n>.labels` | Complete little-endian int32 labels for current state/checkpoints; exact permutation and replay checks |
| `session/checkpoint-paths.json` | Added by the re-plan: each offered checkpoint's recipe/pre/post path, including abandoned branches |
| `workspace/macros.json`, `workspace/keymap.json` | Existing exchange records, interpreted through their own declared formats; empty macro member may be absent |
| `workspace/workspace.json` | Remaining typed/versioned workspace sections with explicit import outcomes |
| `raw/session.sqlite3` | Staged consistent copy for recovery/evidence, mandatory for unsupported work under PS3.6; not the logical import input |
| `logs/<name>` | Optional selected saved-log bytes with recorded digests; no execution of contents |

| Target | Read from staged 0.4 data | Becomes | Checks before publication |
| --- | --- | --- | --- |
| Recipes | Event recipes and saved recipes, including `word` and `star` steps | Retained recipes with literal semantics and conversion records | Normalise, check all step/primitive limits, replay with the 1.0 oracle-matched engine; compare full labels and pre/post hashes |
| Macro sets | Macro/workspace sections, including `prefs.layout.magic600_experiment` and `macro_library` | Typed saved macros/sets with provenance labelled own, imported | Validate every recipe, identity and full collateral claim; A/B evidence is recorded pending fresh verification; empty libraries stay empty |
| Protected orbits | `prefs.protected` and workspace protection requirements | Explicit protection-set records | Exact canonical members and rule conversion; any failure stops import, with no weakened protection |
| Filters | `rules`, named sets, filter sequences and typed workspace filter sections | Versioned filter records | Validate expressions, referenced sets and rule order under the declared conversion; unknown/invalid sections follow PS3.6 |
| Preferences | `meta.prefs` and typed workspace sections | Versioned preferences/workspace records | Validate schema and protection requirements; separate display-only data and durable work; no interpretation of a layout field as disposable UI data |
| Checkpoints | Snapshot labels/hash/head/preferences plus `session/checkpoint-paths.json` built from staged events | Checkpoints with their own replay paths and captured preferences | Validate label permutation and model; replay each path from root with every recipe/pre/post hash; match complete labels, not just a self-reported hash |
| 0.4 twist keys | The literal signed-generator word each key typed under its original key-set context | Binding to that exact word, with its origin and absolute arguments | Resolve old meaning before conversion; replay equality, identity and new key-map conflict checks; no translation by displayed twist name |
| 0.4 command keys | Old command ID, arguments and set context | Binding translated by DT-M1's reviewed alias table | Pin alias-table version; check destination row/arguments and duplicates/prefixes; untranslatable or removed set scope is reported, never guessed |

- **PS3.4 [B]** Recipes remain 1 to 256 word/star steps and at most 3,000,000 expanded primitives in total;
  100,000-letter chunks cannot raise that ceiling (partition section 13; `core.py`; V05.1). The oracle acceptance
  replays every fixture recipe before and after conversion and compares its full replay hash. Star steps retain
  model-bound seed, relocation and setup semantics. The smaller interactive-log history limit in PS1.10 is not
  an amendment to recipe semantics or a reason to truncate the path B archive.
- **PS3.5 [B]** Every checkpoint offered for restore is verified from its own root-to-checkpoint path, including an
  abandoned branch (migration-options item 5; exporter re-plan F10 and F11). Shared prefixes may be stored once.
  Each event's recipe and `pre`/`post` hashes remain available in `session/checkpoint-paths.json`. A path that cannot
  be recovered or replayed leaves that checkpoint unoffered and named in the report, recoverable in the raw copy.
  Checkpoint preferences are validated and retained as captured; replay cannot reconstruct them from event history.
- **PS3.6 [B]** Known valid workspace sections are imported. Unknown or invalid sections are named and retained in
  the raw copy; without that copy the import stops and publishes nothing. Failed protection conversion always
  stops import (migration-options item 4). The fresh-session case omits an empty `workspace/macros.json` and imports
  zero macros, rather than using exchange validators to invent a nonempty library (migration-options item 6).
- **PS3.7 [D]** Imported certificate states obey partition PS3 and 8.1 (DT-L1 R2-E1). The following lifetimes remain
  separate. A recorded certificate never enables a commit or bypasses protection.

| 0.4 state | Import treatment | What can change it |
| --- | --- | --- |
| A: reference map verified | Recorded claim, retaining model/manifest/reference version and source/destination frames | A fresh 1.0 reference-map check against those exact inputs |
| B: emitted net action verified | Recorded claim retaining A's dependencies and source/emitted recipe digests; old proof metadata remains untrusted | Independent 1.0 re-verification of the emitted net action for its immutable scope |
| C: contextual check | Never persisted or resurrected as evidence | A new in-memory check at the current revision, orbit, role, protection and goal |
| D: preview token | Outside all evidence fields; never persisted or imported as permission | A new engine preview and explicit user-action token through chapter 01 |

- **PS3.8 [B]** Before publication, the importer validates the package from the package alone and replays the head
  and every offered checkpoint with the matching 1.0 engine (exporter re-plan stages 7 and 8). All complete labels
  agree with their recorded states; model mismatch is refused. The 0.4 chain and checkpoints become verified
  archived history. Live undo begins at the imported head, stops there, and after one 1.0 operation can return to
  that head (path B invariant 5, fixture F19). No 0.4 history is silently continued as live 1.0 undo.
- **PS3.9 [B]** Publication writes a fresh generation, validates it, makes every file and the retained archive
  durable, then switches the active pointer (migration-options item 7; probe report sections 6 and 7). The previous
  generation and recovery checkpoint remain available as that contract requires. A failure before the switch leaves
  the previous generation active. An uncertain switch is reconciled before retry; it is not reported as proof of
  non-publication. This chapter cites the publication contract, and makes no new Windows, power-loss or performance
  claim from its source-only evidence.

## 11. PS3: research archives and independent exchange

- **PS3.10 [D]** Research import uses a versioned table keyed by **(format, field, value)** (partition default 20;
  DT-L1 R2-E1). A value the table does not map enters as recorded, with its text kept word for word. Each envelope
  cites its source bytes by digest. Format recognition, integrity, interpretation and mathematical rechecking are
  separate results; a familiar status word cannot bypass any of them.

| Import-table field | Stored value and validation |
| --- | --- |
| Table format/version and content digest | The exact mapping edition used; referenced by every converted envelope |
| Source selector | Exact format identifier or explicit absent-format case; stage/source identity when an ambiguous case needs it |
| Field selector | Precise source field location, including nested fields, with no match by display label alone |
| Value | Original value with case and text unchanged |
| Target facts | Chapter 06 interpretation under LQ10's facts envelope, without inventing evidence for omitted fields |
| Scope and prerequisites | Required input digests/levels, declared family/bounds and check dependencies for that interpretation |
| Unmapped outcome | Original field/value, source digest and reason retained as recorded |

- **PS3.11 [D]** Records without a `format` key are accepted as archive inputs (default 13; DT-L1 R2-E1).
  The absent key is represented as absence in the import inventory, never silently filled from a filename or status
  word. **156 of the 417 JSON files** have no top-level `format` key, and 146 have none at any depth (source evidence,
  DT-L1 R3-E2). Coverage checks enumerate every actual (format, field, value), including absent-format cases, and
  require an exact mapping or an explicit unmapped entry. Corpus membership and file digests accompany that report.
  EVD.48 defines field, value and the enumeration rule (DT-L1 R3-E3).
- **PS3.12 [D]** FP.22 uses an explicit manifest/source-byte-digest/JSON-pointer discriminator for absent-format
  mappings. An exact match with its prerequisites may produce the table's facts; no match stays recorded/unmapped
  and conflicting matches stay recorded/ambiguous. A filename or status word cannot identify the format. Absence
  and original bytes remain intact, with any interpretation recorded separately. The complete typed discriminator
  table is still an integration dependency; without it records remain recorded under EVD.53. This fixes dispatch,
  not the missing mathematical interpretations.
- **PS3.13 [D]** The following format-specific field views supplement the catalogue without renaming research
  formats (defaults 3 and 20; partition 10.3). They retain original fields and their exact byte reference; an
  absent field stays absent. They are not replacement schemas that force different family members into one
  shape. Additional source fields remain in the captured record and the mapping inventory.

| Research format | Original fields/data retained | Independent interpretation/check boundary |
| --- | --- | --- |
| `t3p-certificate/1` | `table_digest`, `rounds` | Exact start table, each chronological round and its certificate claims |
| `t3p-check/1` | `setup`, `prefix_word`, `rounds`, `digest_after`, `passed`, `failures` | Checker inputs and actual replay coverage; `passed` alone is insufficient |
| `t3p-scan/1` | `setup`, `digest`, `prefix_word`, `rows`, `targets`, `family_exhausted`, `exhausted`, `tie_break` | Negative scoped to the declared scan family; retain best-certificate references |
| `t3p-pairs/1` | `setup`, `digest`, `prefix_word`, `metric_scope`, tested items/rows and counts | Exact declared pair family, parameters and finite coverage |
| `t3p-plus/1` | `setup`, `digest`, `prefix_word`, `instances`, `limit`, tested items/pairs, `exhausted` | A limit is not a completed negative over an undeclared family |
| `t3p-levels/1` | `setup`, `digest`, `levels`, `lambda`, witnesses, histogram and iterations | Greatest-fixed-point/locality data, under its exact starting input |
| `t3p-locality/1` | `delta_encoding`, `NN`, `R2`, `kappa`, shells and partner counts | Exact Q(√5) values and declared locality encoding |
| `t3p-chain/1` | `table`, start/end digests, rounds, `word`, policy and stop reason | Chronological replay, joins and stated stopping scope |
| `macro-certificate/1` | `start`, `table_digest`, `prefix`, `macros`, `excursion`; per-macro support/delta/Φ/digest/F/D claims | Recompute full collateral and prefix observables claim by claim |
| `macro-check/1` | `setup`, `digest`, `macros`, `replay`, `phi`, `passed`, `failures` | Distinguish recomputation, reused checks and literal replay |
| `pstar-census/1`, `pstar-step/1` | Starting digest/prefix/setup, options, families/candidates, choice/certificate and exhaustion fields where present | Complete declared search sets and reductions, separate from selected witnesses |
| `pstar2-stop/1`, `pstar2/2`, other `pstar-*`/`pstar2-*` | Original version, table digest, parameters, segments/words, status/reason and stop options | Preserve route and limits; no cross-format interpretation of `exhausted` |
| `s16-*/1`, including RR, PXRR and transport | `seed`, `table_digest`, `word`, `digest`, `phi`, `options`; family-specific counts, segments and completion fields | Scope to that route, alphabet and declared reductions; retain helpers and bracket chronology |
| `jams.json` and other absent-header records | Complete source object, original key names and field locations | Explicit absent-format dispatch under PS3.12 |
| `steps.json` and per-step words | Original index/order, word reference or `word`, `input_digest`, `output_digest`, part tree, counts and checks when present | Join adjacent steps by their digests; check full literal expansion before calling it replayed |
| `r22b10/1` | Original node graph, root `V`, `V_length`, state/target digests and orbit/certificate data | Exact straight-line-program length, generator action and literal expansion checks remain separate |
| `r22b10/2` | Version-1 data plus recorded seed words and orbit order where supplied | Interpret the new seed/order contract; do not replay under the version-1 defaults |
| `runner-run/3` | `menu`, `start`, `config`, `phases`, `steps`, `stuck`, `lattice`, `result`, `error`, `acceptance`, `totals` | Job/run outcome separate from scope and certificate validity |
| `literal-lattice/1`, `lattice-phase-play/1` | Original start/word/chunk/check fields and exact label-result references | Preserve the distinction between composed evaluation and played chunks |
| `ia16` | Captured gzip member and uncompressed little-endian uint16 codes `60*grip+item` | SHA-256 of uncompressed bytes; decode only with the pinned menu order, never as retained generator IDs |
| Stage 19 pose tables, proofs and receipts | Original header or absence, all exact matrices, word/profile/rank data, input and candidate digests, source references, scope, attribution and check-mode facts | Regional-family transport covers only its stated subject; reachability promises and open global coverage remain visible |

- **PS3.14 [D]** A manifest binds both stored and decoded content, without confusing either with the captured
  private-source edition (partition 10.3, default 13; stage 19 `PUBLICATION.md`). The `IMPORT-MANIFEST.json` field
  table below gives the existing stage 19 spelling. Older manifests retain their own schema and equivalent
  recorded boundaries. A check of sanitized bytes establishes only the public-byte relation it actually checks.

| Manifest field | Existing value and check |
| --- | --- |
| `format`, `source_commit`, `archive_date` | Manifest version and pinned source edition/date |
| `evidence_kind`, `runtime_of_original_mathematical_checks`, `production`, `hash_note` | Scope of archival production and historical checks; no fabricated fresh replay |
| `files[].original_name`, `original_sha256` | Captured source name and digest; separate from public bytes |
| `files[].path`, `sha256`, `stored_bytes` | Locator and SHA-256/size of the exact stored public member |
| `files[].uncompressed_sha256`, `uncompressed_bytes` | SHA-256/size after bounded decompression; equals the stored boundary for uncompressed members |
| `files[].transformation` | Recorded metadata transformation; original and public mathematical data retain their documented relation |

- **PS3.15 [D]** Artefact identity is content digest, never path (default 13). Import preserves case-distinct source
  names as metadata and resolves their bytes independently. It never extracts two members onto a case-insensitive
  path and overwrites one. The **six pairs** of case-only names are source evidence from stage 19 `PUBLICATION.md`.
  An incomplete checkout is reported as missing input; a filename collision cannot be treated as equal content
  or silently removed from a coverage denominator.
- **PS3.16 [D]** FP.23 chooses digest-named ZIP STORE members with original names kept in a source-name manifest.
  It fixes framing, header fields, traversal/duplicate checks, stored/decoded digest boundaries and FP.9 cumulative
  limits. Case-only source names stay distinct metadata and are never extraction paths. Path B keeps PS3.3's ZIP
  paths with FP.24's candidate manifest, rather than adopting the research bundle's object paths. The carrier is
  a draft and executes no archived producer.
- **PS3.17 [D]** Independent-check exports include the evidence envelope, original-label mapping version, exact input
  tables/words/certificates or their available digest-pinned bytes, and the checker identity plus the instructions
  and dependencies needed to check them (partition PS3, KN6). A bundle manifest records member digests and missing
  dependencies. A dependency absent from the stage 19 archive stays declared as absent; a historic PASS receipt
  does not imply a new portable end-to-end reproduction. Source bytes, exact computation, review and replay keep
  their distinct scopes. Network and process access are not granted in 1.0 (default 2); exchange is local through
  explicitly granted files, and import never executes an archived producer.

```text
operation import_archive(input: GrantedFiles, table: ImportTableRef, limits: LimitsRef) -> ImportReport
  effect:  read granted files; publication only through an invoked import command
  cost:    stored/decoded bytes, mapped fields, exact checks and declared replay work
  errors:  PS1:resource_limit.persistence_size, PS1:input.digest_mismatch, PS1:input.persistence_format,
           PS3:input.checkpoint_replay, PS3:input.source_preservation_unverified, PS3:busy.legacy_session_locked,
           PS3:unknown_outcome.import_publication
operation export_check_bundle(envelopes: List<EvidenceEnvelopeRef>, limits: LimitsRef) -> CheckBundle
  effect:  write granted files
  cost:    envelope/member bytes and declared verification work
  errors:  PS1:resource_limit.persistence_size, PS1:evidence.scope_not_immutable, PS1:input.digest_mismatch
```

## 12. Relation to the migration format freeze

- **PS2.10 [B]** When its gate passes, the migration format freeze fixes only the language-neutral data core:
  words/recipes, macro/protection sets, filters/preferences, checkpoints with replay paths, bindings/key maps,
  the provenance field of words/macros/workspace records and
  the envelope's identity, immutable scope and facts payload (owner, 6 November 2026, LQ10; partition section 13;
  DT-L1 R2-E1).
  It fixes the selected byte profiles, identifiers, versions, limits and lossless import mappings needed to read
  that core. Source syntax, IR and module formats freeze later and evolve through recorded editions. A lossy
  mapping, loss of provenance, changed word meaning or omission of a path B target needs an explicit owner scope
  decision. No part of this chapter is a freeze ruling.
- **PS2.11 [B] Withdrawn.** The owner chose the facts envelope and language-neutral core, removing these alternatives (owner, 6 November 2026, LQ10).
- **PS2.12 [B]** Both Astra freeze rulings sit at LP1's exit, then the owner signs off and LP2 to LP5 may start
  before the design-system ADR (owner, 6 November 2026, LQ19; owner, 6 November 2026, decision 7). LQ8, LQ9 and LQ21
  need their evidence first, so the rulings follow the decisive runs. Language development is complete when LP2
  to LP5 are done apart from LP4's editor look; the design line resumes then. Its design-system ADR conditions
  LP4's editor look and LP6. The charter's stage 2.5 exit retains all three parts in this amended order for the
  language line; no gate is relaxed. Nothing here resumes the design line or sets a day count.
- **PS2.13 [D]** EN4's candidate [format packet](../format-packet/README.md) supplies the following fields and
  reference vectors, following partition LP1, section 13 and the [EN4 handoff](../../../../progress/1.0/replan-2026-10-22.md).
  FP.35 and closure/manifests/p3.json record its open integration dependencies, including the typed corpus mapping,
  EN3 word/admission rules and alias pins. Delivery of this draft is not freeze readiness. Its ruling belongs to
  Astra and its sign-off to the owner.

| Requested packet item | Needed content | Clauses/dependency |
| --- | --- | --- |
| Candidate identity and scope | Exact source/candidate digests; LQ10's facts core; LQ19's amended ordering and owner authorisations | PS2.10 to PS2.12; FP.1 to FP.3 |
| Format inventory | Every path B target and core format, identifiers/versions, wire fields and optional/unknown-field rules | PS1.12, PS1.13, PS3.3; FP.4, FP.10 to FP.16, FP.24 |
| Byte and digest profiles | Canonical JSON/numbers, pinned text bytes, binary types/order, gzip framing and each digest level/role | PS1.1 to PS1.9; KN1 normalisation; FP.5 to FP.8 |
| Shared limits | One read/write table including archive members, decompression, expanded words and graph bounds | PS1.10, PS1.11; R-01; FP.9 |
| Provenance | Versioned common field and both LQ21 forms/compatibility outcomes; no laundering across file/workspace boundaries | PS1.32 to PS1.34; chapter 07; FP.17, FP.18 |
| Evidence envelope and import table | Immutable scope, original labels/source digests, facts payload, EVD.64's field writers and mapping/coverage reports | PS1.35 to PS1.38, PS3.10 to PS3.14; chapter 06; FP.19 to FP.22, FP.33 |
| 0.4 conversion and replay | Recipe oracle equality, protection preservation, every offered checkpoint path, key-word and alias conversions | PS3.3 to PS3.8; FP.25, FP.26 |
| Source preservation and publication | Amended locked-copy contract, guard/watch rules, generation/recovery protocol and matching evidence references | PS3.1, PS3.2, PS3.9; FP.27, FP.28 |
| Editions and recovery | One-way migration records, retained originals, forward-version refusals and meaning-preservation proof | PS2.1 to PS2.9; FP.29 to FP.32 |
| Conformance and open dependencies | Tests for the current candidate, evidence classes, all remaining open choices; no structural check substituted for content review | Section 15 and section 16; FP.34, FP.35 |

- **PS1.43 [D]** FP.30 to FP.32 encode HANDOFF H11's durable exit lineage. Exact exit poses, the crossing's
  before/after positions and certificate are retained. The append-only action chain records head changes separately
  from the mechanical event tree. Position and current Context are separate (H11); exit and entry have their own
  before/after and entry binds its derivation. Workspace actions join the chain with equal Position and an inverse.
  One transaction stores the transition, command/request/admission/receipt bindings, new RevisionRef and any void
  facts. FP.36 retains durable outcome lookup after undo and epoch loss. Reset/import/restore void prior exits
  durably, including inactive branches. Recovery derives validity
  from committed actions, selected parent path and voids, then replays retained moves from the stored poses. Equal
  labels never select or revive an exit; an incomplete crossing transaction leaves the complete before position.
  This storage proposal consumes H11 and does not change A5's refusals. Branches follow the engine-owned journal
  policy chosen by the owner (owner, 6 November 2026, LQ4).

## 13. Source differences and evidence limits

The 2 October migration decision and migration-options still describe online backup without a file copy. The
3 October owner amendment supersedes that restriction for the locked database/WAL copy only. The exporter re-plan's
stage 4 wording still describes a model read through a source connection; its amended invariant 2 and the probe
report instead confine SQLite recovery/opening to the copy. Its stage 5 writer attribution is superseded by the
probe report's MPR-06 decision without attribution. PS3.1 and PS3.2 use the amended contracts.

The older exporter draft's 0.4.1 vehicle, live-export option and head-chain-only checkpoint treatment are superseded
by path B and the checkpoint-path additions. FP.24 distinguishes its old manifest spelling from the candidate
`C600-MIGRATE-draft`. Its existing ZIP layout remains the referenced package proposal.

The retained JSON serializer, prototype store serializer and stage 19 publication serializer are different byte
profiles, as PS1.2 records. `encode_export` also differs from deterministic `encode_log` in gzip timestamp and byte
limit enforcement. The R-01 table describes the required compatible read/write contract, not a claim that every
old download already meets it. The 2,000,000-primitive interactive history limit and the 3,000,000-primitive recipe
limit apply to different objects.

KN2.10 lists the recorded word lengths: 2,352,984 literal letters stored flat, 871,582 in the T6 step, 672,417 in
ENG-0J's earlier observation, and longer words only composed. The research handoff, section 3.5, distinguishes the T6
step's incomplete literal replay from composed checks. Those are source records, not new measurements. No such number
sets a storage cap or substitutes for DT-J1 X3. The stage 18 admission figure of 10⁷ letters is likewise a research
bound, not a chosen product admission bound.

The chapter remains a draft. FP.34 now supplies source/reference byte and conversion checks and synthetic durable-lineage
checks for this candidate. The broader conformance tests below remain requirements, not a report that they all ran. DT-L1 R2-E1, R2-E3 and R2-E4 dependencies are named where used, as round 3 corrected them (overview
section 8). No field or migration here depends on labels off the lattice.

## 14. Error codes added by this chapter

- **PS1.42 [D]** The following stable-code candidates use the categories of V06 and partition default 5. The
  signatures above use them; chapter 13 owns their final spelling. An unmapped research label is a recorded import
  outcome, not a failed check or an empty search result. A refusal publishes no partial active record.

| Code | Meaning |
| --- | --- |
| `PS1:input.persistence_format` | Unknown carrier, invalid declared field structure, unsupported dimension or missing required format information for an active read |
| `PS2:input.persistence_version` | Unsupported data version/source edition or unavailable migration path; original may remain recorded |
| `PS1:input.noncanonical_value` | Exact value, part graph or byte profile violates the selected format's rules |
| `PS1:resource_limit.persistence_size` | A shared stored/decoded/cumulative size or parsing bound would be exceeded |
| `PS1:resource_limit.word_expansion` | Exact expanded word length exceeds its declared admission bound; length is retained in the refusal |
| `PS1:input.digest_mismatch` | Stored, decoded, input, state, member or recorded dependency digest disagrees |
| `PS1:evidence.scope_not_immutable` | Attempt to persist a claim with revision-bound context in its scope (EVD.58) |
| `PS1:capability.provenance_incompatible` | Required provenance form, label or dependency cannot justify a policy-gated use |
| `PS3:input.checkpoint_replay` | A checkpoint's own path, labels or pre/post checks cannot be verified |
| `PS3:input.source_preservation_unverified` | Source-copy preservation conditions fail or watch coverage is inconclusive; nothing is published |
| `PS3:busy.legacy_session_locked` | The 0.4 session lock cannot be acquired without writing or creating source data |
| `PS3:unknown_outcome.import_publication` | Pointer-switch outcome cannot be authoritatively reconciled; recover before retry |
| `LB5:input.obligation_untranslatable` | Forwarded from LB5.18: a statement is outside the versioned export vocabulary; PS1.45 writes nothing |

## 15. Conformance tests

These are requirements for LP1 and later phases, with the class meanings of overview section 3.4. The closure check
verifies shape and invokes FP.34's narrower reference vectors. Those vectors cover parts of these requirements;
they do not establish full cross-platform, importer or mechanical conformance.

| Test | Checks | Class |
| --- | --- | --- |
| CT-PS1-01 | Fixed canonical-profile vectors give identical bytes/digests on Windows and headless Linux; Unicode, escaping, key order and terminal newline cases included | source |
| CT-PS1-02 | Exact integers beyond 2^64, reduced rationals and the stage 19 6,775-digit B11 bound round-trip without conversion through floats; digit-resource refusal loses no source | source |
| CT-PS1-03 | Invalid Q(√5) denominators/reduction, duplicate keys and a float in a decision field are refused under the selected profile | source |
| CT-PS1-04 | `Digest<labels>` matches `core.state_hash`; `Digest<poses>` matches J1 and the store helper; confusing levels or hashing render floats is refused | source |
| CT-PS1-05 | R-03 pinned-byte checkout fixture and captured proof/source digests survive checkout without line-ending reinterpretation | source |
| CT-PS1-06 | Gzip vectors use the selected complete deterministic header/compressor profile; stored and decoded digests are separately checked | source |
| CT-PS1-07 | R-01 each limits-table row round-trips at its boundary and refuses one unit above; cumulative gzip/part expansion cannot bypass it | source |
| CT-PS1-08 | Catalogue preserves existing identifiers and fields; unknown/newer format data cannot activate under an older version | source |
| CT-PS1-09 | Under the flat draft, mandatory chunks retain the literal word; same_as and inverse_of annotations match their pinned spans, generate no letters, and dangling/cyclic targets refuse admission (FP.13) | source |
| CT-PS1-10 | A small source/program graph may be archived within its own budget; its expanded stored menu word requires all literal chunks and refuses PS1:resource_limit.persistence_size above storage limits. A stored word above X3 refuses PS1:resource_limit.word_expansion before replay, without truncation | source |
| CT-PS1-11 | Reversed I_a item order, another contract/model and jumble letters with Jumble off are refused without reinterpretation | source |
| CT-PS1-12 | Recipe normalisation preserves literal prefixes; 100,000-letter chunking cannot exceed the 3,000,000-primitive total or change Strict protection | source |
| CT-PS1-13 | Binding/key-map round-trip preserves absolute objects/words; duplicates, non-prefix codes and unsupported layout conversions are reported | source |
| CT-PS1-14 | Every checkpoint carries a verifiable own path; A5 exit poses/certificate and resumed retained moves reproduce the poses and labels digests | source |
| CT-PS1-15 | PS1.30 recording round-trip retains every RecordedStep field, canonical arguments, word/action references, revisions, module and receipt bindings, and provenance; all commit playback needs a new explicit action and token | source |
| CT-PS1-16 | RunRecord reproduces chapter 09 key inputs and exact result bytes; cut-off and safety-stopped runs never gain a negative claim | source |
| CT-PS1-17 | State-derived word survives workspace save/read, rename, source conversion and file round-trip with its labels, under both LQ21 forms | source |
| CT-PS1-18 | A legacy/unsupported provenance block stays recorded and cannot be read as an unlabelled solving word; 0.4 origin remains own, imported | source |
| CT-PS1-19 | The facts envelope rejects transient-only scope and keeps every digest's level, original source label and check mode | source |
| CT-PS1-20 | Facts-envelope graph experiment reports nodes/edges and stored/decoded bytes against its digest-pinned stage 14 to 19 baseline | source |
| CT-PS1-21 | Every admitted stored-field combination gives exactly one derived display word or typed inconsistency under each recorded function edition | source |
| CT-PS1-22 | Optional jumbling section is absent for never-on sessions; interrupted episodes recover on or at the last committed revision, with no half crossing | source |
| CT-PS1-23 | Legacy certificate IDs use the original pre-diagnostic field set; imported timing/context fields cannot upgrade A/B or recreate C/D | source |
| CT-PS1-24 | PS1.30: recorded completed, committed, refused, failed and cancelled receipts round-trip unchanged; a missing terminal stays absent and stops play/conversion under EV4.19. No token or operation preview ID is stored, and a playback child's new binding never replaces the captured invocation | source |
| CT-PS1-25 | PS1.44: crash after pending is saved before dispatch, after a durable child completes before cursor advance, and during a manual continuation. Reopen performs outcome lookup first, advances each known success at most once, stops unknown outcomes without retry, saves explicit from and never offers the cursor as an undo target | synthetic |
| CT-PS1-26 | PS1.45, PS1.46: a supported typed statement exports pinned Lean 4 source and vocabulary through a granted file capability; an unsupported symbol or a named statement gives LB5:input.obligation_untranslatable and writes nothing. No proof-record return format/import row, check mode, process/network grant or evidence promotion appears | source |
| CT-PS1-27 | PS1.12, PS1.36: the catalogue's envelope writer resolves to EVD.64. A program or imported record cannot use storage to assert a forbidden basis/check or formal proof; permitted facts round-trip with their source and check dependencies and violations retain EVD.65's code | source |
| CT-PS2-01 | Migration record binds source/target/converter and all versions; injected validation/write failure leaves old bytes and active generation recoverable | source |
| CT-PS2-02 | Edition change preserves stored literal word, alphabet, identities, part pins, length and provenance; changed referenced content is not silently relinked | source |
| CT-PS2-03 | Facts-function edition change retains old facts and interpretation, records new outputs/inconsistencies and invalidates stale derived caches | source |
| CT-PS2-04 | R-08 inputs changed during build/conversion cause reuse refusal even if cached output hashes match; check occurs after build | source |
| CT-PS2-05 | Newer schema/edition, unavailable extension rules and missing provenance versions cannot partially activate a record | source |
| CT-PS2-06 | PS1.12, PS2.10: the source extension is .6cs while c600-source-draft and the other c600- descriptors remain until freeze; the freeze inventory covers every path B data-core target and facts envelope but excludes syntax, IR, module semantics, recordings and proof-obligation export | source |
| CT-PS2-07 | PS2.12, PS2.13: both rulings and owner sign-off precede LP2 to LP5; evidence-dependent choices precede the rulings, and the design-system ADR still conditions LP4's editor look and LP6. The packet states the facts core and amended order without treating this draft as either ruling | source |
| CT-PS3-01 | Path B fixture matrix: guarded, locked database/WAL copy includes WAL-only commits, never uses `immutable=1`, and never opens source SQLite | source |
| CT-PS3-02 | Every oracle fixture recipe has the same replay hash after path B conversion, with exact word/star semantics and full collateral | source |
| CT-PS3-03 | F10/F11: abandoned-branch checkpoints verify by their own paths; exchanged labels with recomputed hashes fail; separate captured preferences restore correctly | source |
| CT-PS3-04 | Known/unknown/invalid workspace sections follow the raw-copy outcome table; failed protection conversion never publishes a weakened set; empty macro library stays empty | source |
| CT-PS3-05 | Old twist-key words replay equally after conversion; command keys cite the alias-table version; obsolete set-scoped bindings are reported without guessed meaning | source |
| CT-PS3-06 | Imported states A/B stay recorded until rechecked, C is absent from persistent evidence, D grants no permission | source |
| CT-PS3-07 | Every actual corpus (format, field, value), including absent-format and differently cased labels, is mapped or listed as unmapped with exact original text/digest | source |
| CT-PS3-08 | Tampered stored gzip, decoded content and manifest digests fail separately; source-edition digest cannot be mistaken for sanitized public bytes | source |
| CT-PS3-09 | Case-only member pairs remain distinct by digest without path overwrite; missing checkout members are reported as missing and remain in inventory | source |
| CT-PS3-10 | Independent bundle checks without network, refuses execution of producers, and reports missing archive dependencies without asserting a fresh replay | source |
| CT-PS3-11 | Buffered-write-loss model and interruption fixtures recover previous or complete new generation; an uncertain pointer switch is reconciled before retry | source |
| CT-PS3-12 | Imported head is a live undo boundary; one 1.0 move undoes exactly to it, including after reopening | source |

## 16. Open items

The owner answered LQ1 to LQ21 on 6 November 2026. LQ8, LQ9, LQ13 and LQ21 wait for their evidence, and LQ2's option set
waits for the joint setting.

- **LQ2, LQ21.** Chapter 07 owns the joint option set, competence capability gates if any, display sinks and the
  choice of flow form. LQ21 is decided with LQ9 on LX-1's label cost and LX-3's refusal counts before the freeze rulings.
  Common provenance survives either form; FP.17/FP.18 specify candidate version/compatibility (PS1.15, PS1.32 to PS1.34).
  Built-in tools keep the 2 October boundary and 17 October repair-list exception until the joint setting.
- **LQ8, LQ9.** Engine language, route and placement affect module/IR dependencies and exact codecs; recorded inputs
  remain explicit (PS1.4 to PS1.7, PS1.15, PS1.31, PS2.7). LX-5/LX-6 feed LQ8, then LX-1 to LX-5 feed LQ9 with LQ21.
  First-pass results in `work/experiments/language-lx/` are leads; decisive runs wait for the owner's window
  (owner, 6 November 2026, decision 15).
- **LQ13.** Source syntax and header spelling wait for LX-8's human test before LP2 builds the parser
  (PS1.13, PS1.14, PS2.1). The source-preparation result is no human learnability result.
- **Candidate wire draft review.** FP.5 to FP.10 now specify JSON, exact/display numbers, gzip, shared limits and
  unknown-field rules (PS1.3, PS1.5, PS1.9, PS1.11, PS1.13). FP.17/FP.18 specify independent provenance version 1
  and no-run values (PS1.34). These are engineering drafts, not unresolved owner choices or an accepted freeze.
- **EN3 word carrier and admission.** FP.13 consumes JI.18's flat ia16 working form; EN3 still pins DT-J1
  X3's replay admission bound (PS1.20, PS1.21). A storage/chunk limit cannot replace X3.
- **Typed corpus interpretation.** FP.22 fixes absent-format dispatch (PS3.12); complete statement/scope,
  coverage and checker bindings and a reviewed typed mapping table still need the integrator/chapter 06. The
  PS1.38 experiment stays partial; EVD.62 records draft corrections to its four collisions without rerunning it.
  Derived judgements need those complete facts and chapter 06's versioned functions; the owner's evidence model is settled.
- **Archive carrier and alias pins.** FP.23 fixes digest-named research ZIP members (PS3.16); FP.24 keeps path B's
  paths. FP.15 pins the current alias identity under its own profile; the integrator refreshes it after p7-table.
  Untranslatable keys remain reported.
- **EN3/EN4 reconciliation.** FP.30 to FP.32 store H11's exit/void/action facts. EN3 and p2b reconcile their
  transition/command rules and H13 traces with these durable fields. The candidate's source identity, requirement
  rows, checks and external commands are in FP.1/FP.34/FP.35 and closure/manifests/p3.json.
- **Source discrepancies.** Section 13 records the migration amendments, the serializer differences and the recorded
  word lengths. Round 3 cleared DT-L1's obligations (overview section 8).
