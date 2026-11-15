# EN4 migration format packet

Status: **candidate draft, not accepted and not frozen**. This is packet `p3` of
`docs/architecture/1.0/language/closure/HANDOFF.md`. Clause tags have the meaning in
chapter 00. The migration-format-freeze ruling belongs to Astra; the owner signs
after it. No schema, importer, runtime, UI or model is changed here.

## Candidate and scope

- **FP.1 [D]** Candidate `en4-data-core-draft-1` consists of the files enumerated in
  `candidate.json`. That file records SHA-256 of each candidate and input file and
  the commit the pins were read from. Its inventory digest is SHA-256 of the
  `candidate` array encoded by FP.5. It excludes itself and the closure manifest
  to avoid a self-hash cycle. The closure manifest reports ownership, not format
  identity. `check_vectors.py` verifies every recorded digest. With `frozen:false`,
  a moved chapter under `language/architecture/` is reported with both digests and
  is not fatal. Packet files, vectors and other inputs remain fatal. At the freeze,
  refresh all pins and set `frozen:true`; every mismatch is then fatal. This draft
  allowance is not proof that changed chapter semantics agree with the packet.
  After the freeze, a change to the frozen core's semantics (a frozen clause, vector
  or dependency meaning) follows PHS.52. A governance-only or non-core change, such
  as an owner decision recorded in `AGENTS.md` or the briefing, or an edit to a
  chapter clause outside the data core, needs a recorded pin refresh that names each
  old and new identity and shows every frozen clause, vector and dependency meaning
  unchanged. No mismatched pin is refreshed silently, and a whole-file digest change
  never permits a change to the frozen core (migration-format-freeze ruling,
  question 2).
- **FP.2 [B]** The freeze covers PS2.10's language-neutral data core and the facts
  payload (owner, 6 November 2026, LQ10). This packet consumes H8 for text/output
  accounting, H9 for no-run provenance, H10 for flat menu words and H12 for aliases.
  Those four are working proposals, not owner decisions. The archive preserves source
  syntax as bytes; source editions, syntax, IR, module semantics and renderer data do
  not freeze here. This packet grants no LP2 start or release approval.
- **FP.3 [B] Withdrawn.** Rejected alternatives of LQ10 (owner, 6 November 2026).

## Profiles and inventory

- **FP.4 [D]** The following are independent record identifiers at version 1,
  encoded as integer `version: 1`, or `format_version: 1` in the Path B manifest.
  A row wholly or partly in the freeze has no `-draft` suffix (migration-format-freeze
  ruling, question 1). A row marked No keeps its literal `-draft` suffix, which no
  alias removes. A former `c600-<kind>-draft` identifier of a frozen row is not an
  alias of its frozen identifier: FP.10 refuses it as an unknown format unless an
  explicit recorded conversion (FP.29) applies.
  Every core row uses FP.5; binary members use FP.8. All retained identifiers and
  profiles in PS1.12/PS1.39/PS3.13 remain unchanged. Import never writes new fields
  into an old v1 record.

| Record | Exact `format` | Payload contract | In the freeze |
| --- | --- | --- | --- |
| Word | `c600-word` | FP.13 | Yes |
| Recipe | `c600-recipe` | FP.12 | Yes |
| Macro, macro set | `c600-macro`, `c600-macro-set` | FP.14 | Yes |
| Protection set | `c600-protection-set` | FP.14 | Yes |
| Binding, key map | `c600-binding`, `c600-key-map` | FP.15 | Yes |
| Filter, preferences | `c600-filter`, `c600-preferences` | FP.15 | Yes |
| Checkpoint, replay path | `c600-checkpoint`, `c600-replay-path` | FP.16 | Yes |
| Workspace | `c600-workspace` | FP.12 | Provenance and core payloads only |
| Evidence, import table | `c600-evidence`, `c600-import-table` | FP.19 to FP.22 | Common envelope and facts only |
| Migration | `c600-migration` | FP.29 | Yes |
| Exit, void, transition | `c600-exit`, `c600-void`, `c600-transition` | FP.30 to FP.32 | Durable replay/recovery data |
| Admission, receipt, workspace inverse | `c600-admission`, `c600-receipt`, `c600-workspace-inverse` | FP.31, FP.36 | Durable action/outcome bindings |
| Path B manifest | `C600-MIGRATE` | FP.24 | Yes |
| Path B session record, local publication metadata | `c600-session-record`, `c600-publication` | FP.24, FP.28; their stated wrappers | Yes |
| Research/check bundle manifest | `c600-research-bundle` | FP.23 | Core transport only |
| Pick record | `c600-pick` | FP.38 | Yes |
| Playback state | `c600-playback-draft` | FP.37 | No |
| Source descriptor, module manifest, lockfile | `c600-source-draft`, `c600-module-draft`, `c600-lock-draft` | PS1.14/PS1.15; byte descriptors only | No |
| Layout, recording, run | `c600-layout-draft`, `c600-recording-draft`, `c600-run-draft` | PS1.27/PS1.30/PS1.31 | No |
| Proof-obligation export | `c600-proof-obligation-draft` | PS1.45 | No |

The last three rows reserve draft identifiers only. Their payload contracts need
their own packet before an active reader exists. The language's name and extension
are chosen (600 Cell Script, `.6cs`; owner, 6 November 2026, LQ17), module sharing
is local (LQ12), every identifier here keeps the `c600-` prefix, the rows marked No
keep `-draft` until their own packet, and syntax waits for LQ13.

- **FP.5 [D]** New JSON uses `c600-json-1`: UTF-8 without BOM, sorted object keys
  in Unicode scalar order, no normalization (H8), compact `,` and `:`, and exactly
  one terminal LF. Strings escape quote and backslash, use `\b`, `\t`, `\n`, `\f`,
  `\r` for those controls and lowercase `\u00xx` for other U+0000..U+001F scalars.
  Slash, DEL, non-ASCII and U+2028/U+2029 are literal. Lone surrogates and duplicate
  keys are invalid. JSON number tokens are integers in [-9007199254740991,
  9007199254740991], with no leading zero, plus sign, exponent or negative zero;
  booleans are not integers. Larger exact values use FP.6. A new active read
  requires byte equality with canonical re-encoding. Parseable different bytes
  return `PS1:input.noncanonical_value`; an explicit conversion may retain them
  as source bytes and create a new record. Retained JSON still uses ASCII escapes
  and **no LF**; the prototype keeps literal Unicode and **one LF**. Source bytes
  are never re-encoded to make a digest match.
- **FP.6 [D]** A field typed `Int` is a decimal string matching
  `0|-?[1-9][0-9]*`; `Nat` is its nonnegative subset. The digit count excludes a
  minus sign. The wire for a free exact value is `{"int":"..."}`, a rational is
  `{"rat":["numerator","denominator"]}`, and Q(sqrt(5)) is
  `{"q5":["a","b","d"]}`. Denominators are positive and the gcd of all
  components is one; zero rational is `["0","1"]`. A Q5 triple retains its
  distinct type even when b is zero. Readers refuse unreduced values, never repair
  them silently. Arithmetic and parsing never pass through floats. The 10,000
  digit resource limit admits a 6,775-digit value. A reader may parse in small
  decimal chunks; a host's default decimal-conversion ceiling is not the format.
  Display-only binary64 is `{"f64":"hhhhhhhhhhhhhhhh"}`: exactly 16 lowercase
  hex digits of the IEEE-754 bits, most significant byte first. Reject all exponent
  2047 patterns and negative zero; writers map either zero to positive zero. This
  value cannot occur in an exact decision field. Legacy integer tokens and decimal
  strings keep their existing format's rules.
- **FP.7 [D]** New gzip uses `c600-gzip-stored-1`. Header bytes are
  `1f8b08000000000000ff` (DEFLATE, no flags, mtime zero, XFL zero, OS 255). The
  compressor is the following algorithm, not a platform zlib version: split input
  into consecutive blocks of at most 65,535 bytes, emit byte `00` for a non-final
  block or `01` for the final block, uint16-LE LEN and one's-complement NLEN,
  then the literal bytes. Empty input has one empty final block. Append CRC32 of
  decoded bytes and decoded size modulo 2^32, both uint32-LE. These are DEFLATE
  stored blocks; they intentionally buy portability, not compression savings.
  The overhead is 18 + 5*max(1,ceil(n/65535)) bytes. This avoids depending on a
  particular platform compressor; a compressing successor needs a new profile.
  New JSON/binary members of at most 250,000 decoded bytes are raw; larger members
  use this gzip profile. A format vector may exercise gzip below that threshold.
  Readers check the declared encoding, size, CRC and exact profile, reject a second
  member or trailing bytes, and bound output before expansion. Old level-5 logs,
  level-9 archives and timestamped downloads retain their captured stored bytes.
- **FP.8 [D]** SHA-256 values are 64 lowercase hex digits. A `DigestRef` is
  `{"role":role,"sha256":hex}`. Roles are `bytes`, `content`, `labels`, `poses`,
  `source`, `menu`, `menu_order`, `recipe_expansion` and `net`. Content hashes FP.5
  bytes, including LF; bytes hashes captured/stored bytes; source hashes pinned
  source bytes. Mechanical labels and poses use PS1.6 unchanged, including J1's
  spaced JSON matrix keys. Recipe expansion hashes signed int32-LE primitives with
  no prefix, count or separator. Net hashes source int32-LE then destination
  int32-LE, sorted by source. Raw arrays declare `encoding`, `shape` (Nat list),
  `count` (Nat), and a `BlobRef`; type widths and element count must agree before
  allocation. Encodings are `i32le`, `u32le`, `u16le` and display-only `f32le`.
  `BlobRef={stored:DigestRef(bytes),decoded:DigestRef(bytes),stored_bytes:Nat,
  decoded_bytes:Nat,encoding:raw|c600-gzip-stored-1}`. Captured legacy members use
  `encoding:captured` plus their declared legacy decoder, never a new gzip claim.
  Same hexadecimal text with different roles is not interchangeable.

## Shared limits and read outcomes

- **FP.9 [D]** `c600-limits-1` is the following single read/write table.
  Both bounds are inclusive and equal. Counts are exact; before accepting the next
  byte, digit, member, edge or operation the reader/writer checks the bound. Every
  applicable row applies, including decoded totals through nested containers and
  referenced data. Referenced graph nodes are counted once by content; each replay
  of a word counts its expanded letters again. ZIP entries count even if their
  content is shared. Limits are resource refusals, never truncation or semantic
  word-length caps. `PS1:resource_limit.persistence_size` reports row, limit and
  attempted count; word admission instead uses `PS1:resource_limit.word_expansion`.

| Row | Unit / scope | Minimum | Read = write maximum |
| --- | --- | ---: | ---: |
| legacy_log_stored | retained log bytes | 1 | 16777216 |
| legacy_log_decoded | retained log decoded bytes | 1 | 25165824 |
| legacy_log_events | retained log transactions | 0 | 10000 |
| legacy_log_primitives | retained log total primitives | 0 | 2000000 |
| recipe_steps | steps in one retained recipe | 1 | 256 |
| recipe_word_letters | signed generators in one word step | 1 | 100000 |
| recipe_primitives | expanded primitives in one recipe | 1 | 3000000 |
| legacy_part_depth | prototype named-part nesting | 0 | 64 |
| legacy_prefs_chars | characters in retained canonical preferences | 0 | 500000 |
| record_stored | bytes in one new JSON or binary record | 0 | 71303168 |
| record_decoded | bytes in one decoded new record | 0 | 67108864 |
| source_bytes | bytes in one new source input | 0 | 8388608 |
| integer_digits | decimal digits per exact integer | 1 | 10000 |
| json_depth | containers on deepest path; scalar root = 0 | 0 | 64 |
| graph_nodes | nodes in a resolved evidence graph | 0 | 1000000 |
| graph_edges | directed child references in that graph | 0 | 4000000 |
| array_rank | dimensions in one binary array | 1 | 8 |
| array_elements | product of dimensions of one array | 0 | 16777216 |
| bundle_members | non-manifest members in one package | 0 | 60000 |
| bundle_stored | all stored package bytes, framing and manifest included | 0 | 2147483648 |
| bundle_decoded | sum of all decoded members and manifest bytes | 0 | 4294967296 |
| archive_events | distinct replay events in one package | 0 | 1000000 |
| archive_replay | total primitive/menu letters replayed to validate offered paths | 0 | 100000000 |
| ia16_chunk_letters | letters in one nonempty new chunk | 1 | 524288 |

Per-member stored/decoded limits are the record rows, not a second table. New
referenced graphs and part annotations also obey json_depth/record_decoded. A word
can span chunks; one chunk is at most 1,048,576 raw bytes. DT-J1 X3's separate
per-operation admission bound remains an EN3 dependency under JI.22;
archive_replay is not that bound. Legacy input semantics and their existing readers
are not widened by the archive limits.

These are engineering trial budgets, not measured capacities or a tariff. The
PS1.38 source baseline has 497 stored records and 13,412,769 bytes; decoded source
bytes total 58,976,540 and envelope metadata 10,424,730 bytes (RESULT.md). The
64 MiB per-record and 4 GiB aggregate budget leave space for that corpus without
imposing the old 24 MiB log cap on a package. A streaming reader need not allocate
the aggregate: at most one record plus bounded graph indices is required. The
source, digit, nesting, graph and event caps bound otherwise unbounded work, but
their product memory cost and interactive suitability are unmeasured. The graph
experiment contains no admitted edges, so it cannot validate the graph caps.
Exceeding a trial budget preserves the archive and reports the item; larger resource
profiles require their own version and evidence, not an automatic split or retry.

- **FP.10 [D]** Required fields below must be present; fields marked `?` alone may
  be absent. `null` means explicit absence only where stated. Unknown fields outside
  `extensions` give `PS1:input.persistence_format` on active read. Each independent
  record may have `extensions: [{name:Text,required:Bool,value:Value}]`; names are
  unique and sorted by scalar order. An unknown required extension gives
  `PS2:input.persistence_version`; an unknown optional extension is preserved and
  included in the content digest, with no effect on interpretation. No extension
  may redefine identity, provenance, protection, letters, evidence or any required
  field; those changes need a new version. An unknown format/version remains
  recorded byte-for-byte and refuses active use with `PS2:input.persistence_version`.
  Dispatch comes first: a reader checks the `format`/`version` pair of a record, and
  of each Ref before following it and then of the record it resolves to, before any
  version-specific field, expected-Ref or header-mismatch check. An unknown pair gives
  `PS2:input.persistence_version` even with added or missing fields; a header
  without `format` or `version` gives `PS1:input.persistence_format`; the mismatch
  outcomes of FP.11 and of each record contract apply only to known pairs. A former
  draft identifier of a frozen FP.4 row is an unknown format (FP.4).
  In particular, an unknown nested provenance version cannot be hidden as an optional
  extension. A current known field may not be silently omitted during conversion.

## Core wire fields

- **FP.11 [D]** Except for the explicit manifest/session/publication wrappers in
  FP.24 and FP.28, each new independent record has exactly `format`, `version`,
  `identity`, `payload`, `provenance` and optional `extensions`. `identity` uses the
  KN0.1 field names, without redefining that type: the three jumbling-only fields
  are null for Full and required non-null for jumbling. Digest-valued identity
  fields are hex strings, not paths. `provenance` is FP.17; null is allowed only
  for a record that is neither a word, macro nor workspace record. `Ref` is
  `{format:Text,version:integer,content:DigestRef(content)}`; no path determines
  identity. Required references resolve locally before active use, and a resolved
  record's `format` and `version` equal its Ref's. A `Pin` is
  `{version:Text,source:DigestRef(source)}`. `Value` permits null, Bool, Text,
  bounded JSON integer, FP.6 exact/display wrappers, ordered lists and string-keyed
  objects; each field's type still constrains it. All IDs/names are Text, all counts,
  offsets and revisions are Nat strings unless a row says bounded JSON integer.
  Lists preserve order; sets are lists sorted by their FP.5 element bytes without
  duplicates. Choices use `{kind:tag,...fields for that tag}`. No unspecified field
  is an executable command, grant or inferred default.
- **FP.12 [D]** These payload fields are literal wire names. Fields within a choice
  are exclusive. Historical context does not make a live `RevisionRef` or grant.

| Payload | Required fields and representation |
| --- | --- |
| Recipe | `steps`: PS1.22 word/star objects with existing small JSON integers; positive star `sign` is explicit after conversion. `primitive_count`: Nat. `expansion`: DigestRef(recipe_expansion). `contract`: Pin. Model is in the record identity. |
| Workspace | `id`: Text; `kind`: format ID of payload; `value`: Ref; `dependencies`: Ref list; `action`: Text; `revision`: Nat. Noncore values can remain recorded and cannot activate without their own reader. |
| Source descriptor (reservation) | `bytes`: BlobRef; `edition`: Text or null if not interpreted. This describes captured bytes, not an accepted header syntax. |

The recipe digest is over expanded primitives, not JSON. The oracle README's
`[{"kind":"word","moves":[1,-1]}]` must give count 2 and
`b15348c8f462384c01e83b6d499c6faf3f96808f5aa07c6bab4b65b36b4445d4`.
Unknown recipe step kinds are untranslatable, never guessed.

- **FP.13 [D]** Under H10 (KN4.7; owner, 6 November 2026, LQ10), a menu-word payload
  has `alphabet:"menu"`, `start`: DigestRef(poses or labels), `length`: Nat,
  `letters`: DigestRef(bytes), `chunks`: ordered `BlobRef` list, `parts`: list of
  `{id:Text,word:Ref}`, and
  `annotations`: list of `{name:Text,start:Nat,end:Nat,
  relation:null|{kind:same_as|inverse_of,word:{local:Text}|{part:Text}|{ref:Ref}}}`.
  `letters` hashes the concatenated decoded chunks, without chunk boundaries.
  Every decoded chunk is little-endian uint16 codes `60*grip+item`, grip 0..599,
  item 0..59 in the pinned 60-item menu order. No header or count is in the `ia16`
  bytes. Non-final chunks have exactly 524,288 letters; the final chunk has 1 to
  524,288. Empty word has no chunks and the empty-byte digest. Length equals half
  the total decoded bytes. Validate counts, codes, both menu pins against the
  independently pinned current menu and order, and concatenated digest before
  replay; a stored pin is never its own expected value. Annotation spans are half-open expanded positions and do
  not generate letters. Under JI.21/H15.13, `local` names an earlier annotation's
  unique `name` in this ordered list. `part` names a unique entry of `parts`, sorted
  by ID, whose immutable word record is included by digest. `ref` names an included
  word directly. References resolve locally with matching alphabet and identity;
  cycles, missing targets, forward local targets and ambiguous tags refuse with
  `PS1:input.persistence_format`. External relations compare the target's complete
  literal word; local relations compare its span. A relation must match that span
  using the pinned inverse menu; mismatch is `PS1:input.noncanonical_value`. It
  cannot relink a definition or generate replay letters. All chunks are mandatory;
  a compressed source graph is a separate archive object, not a stored flat word.
  `examples.json` encodes JI.23 with A, B and C and its exact literal bytes. A menu not representable in this
  60-item encoding needs a new carrier, never a guessed radix. Full words use
  `alphabet:"signed_generators"`, `start`, and `recipe:Ref` instead. The empty
  signed-generator word has the exclusive payload
  `{alphabet:"signed_generators",start:DigestRef(labels),length:"0",
  letters:DigestRef(recipe_expansion)}`, where `letters` hashes empty bytes and
  there is no recipe reference. It is a materialized word record of literal
  length zero, usable by `solved_witness.word`; it is not a cancelling nonempty
  word. FP.9's recipe minima still apply to every recipe, including imported
  legacy recipes, and forbid an empty recipe or empty recipe word step. This
  empty word variant is not a recipe and does not relax those bounds. No ia16 cast
  is permitted. If EN3 changes H10, chunk/annotation fields and their byte vectors
  need reconciliation; literal chronology and provenance still survive.
- **FP.14 [D]** Macro payload is `{id:Text,name:Text,revision:Nat,value:Ref,
  metadata:Value,evidence:[Ref]}`; value refers to a word or recipe and metadata
  contains no executable authority. Macro-set payload is `{entries:[Ref],origin:
  Ref|null}` with unique macro IDs, in ID order. Protection-set payload is
  `{level:labels|poses,members:[{kind:orbit,id:integer}],mode:net|strict,
  contract:Pin,origin:Ref|null}`; orbit IDs are 0..34, unique and ascending.
  An unsupported protected-object kind or rule cannot be dropped: import stops.
  Future supported objects require a declared version, not an optional extension.
- **FP.15 [D]** Workspace payloads use these fields. Values whose interpretation
  is unavailable stay recorded; preservation is not activation.

| Payload | Required wire fields |
| --- | --- |
| Binding | `key`: ordered Text physical-code sequence; `context`: Text; `command`: Text; `contract`: Pin; `args`: Value object of absolute typed arguments; `fixing`: null or `{identity:KN0.1 wire form,revision:Nat,source:DigestRef}`; `layout`: null or Pin. Relative arguments need all fixing fields. |
| Key map | `bindings`:[Ref]; `contexts`:[Text]; `layouts`:[Pin]; `grip_map`:[`{cap:integer,frame:Value}`]; `aliases`: null or `{version:Text,profile:Text,content_digest:hex}` under the alias table's identity rule below; `conversion`: Ref|null. Cap/frame values use exact canonical object/rotation data. Duplicate/prefix conflicts follow PS1.26. |
| Filter | `contract`: Pin; `rules`:[`{expr:Text,style:hide\|ghost\|solid\|highlight}`]; `named_sets`:[`{name:Text,kind:identity\|position,ids:[integer]}`]; `source`: BlobRef; `conversion`: Ref|null. Preserve rule order; set names and IDs are unique and sorted. |
| Preferences | `contract`: Pin; `entries`:[`{name:Text,value:Value,outcome:converted\|recorded,source:DigestRef(bytes)}`]; `protection`: Ref; `conversion`: Ref|null. Entry names are unique and sorted. Recorded values cannot become active preferences. |

PS1.28's existing preference names all get a per-item outcome, including workspace
data under `layout`. Unsupported key contexts and filter languages are reported.
Alias pins reference `docs/progress/1.0/command-aliases.json` version
`en4-2026-11-05.3`, content digest
`5ef9a34103f9411f6b11eda5d371c34f561df43bcb0cf707c6910d23e042c3c5`.
`profile` is the exact `canonical_form` text declared by that table. Verify using
that identity rule: the table excludes `content_digest` itself and the final LF.
This is not `DigestRef(content)` or FP.5 hashing. Version, profile and digest all
must match before command-key conversion; mismatch gives `PS1:input.digest_mismatch`.
The `after` alias converts to `view.forecast.select` with `phase:cleanup` under
this pin, granting nothing. The pin and its vectors follow the table as it stands
after the reconciliation (H15.15); a later table version needs a new pin.

- **FP.16 [D]** A checkpoint payload is `{id,name,mode,state,root,path,preferences,
  episode,exit,assistance}`. IDs/names are Text; mode is `jumble_off|jumble_on`;
  `state={digest:DigestRef(labels|poses),data:BlobRef}`; root is
  `{kind:solved}` or `{kind:verified,checkpoint:Ref,evidence:Ref}`; path and
  preferences are Ref; episode/exit are Ref or null; assistance is FP.17.
  Labels data is exactly 259,800 int32-LE values, a permutation of 0..259799.
  Pose data is a canonical FP.5 object containing `pose_table` and `pose_ids`:
  a table entry is `{k:integer}` in the pinned K+ order or `{m:[[Q5]]}` with 4x4
  exact entries; IDs are an array descriptor for u32le, one per pinned piece.
  The referenced model fixes piece count; exact PS1.6 pose digest is recomputed.
  No display float enters this object. Replay-path payload is
  `{root:Ref|null,head:Text,events:[{id:Text,parent:Text|null,value:Ref,
  pre:DigestRef,post:DigestRef,preferences:Ref|null}]}`. Root null means solved;
  each event value is a recipe/word. Event IDs are nonempty and unique; `head`
  names one event, or is the empty Text and selects the root itself. The parent graph is
  acyclic and ordered parent-before-child. Every offered checkpoint selects its
  own head; verify its full path and complete state (PS3.5), including abandoned
  branches. References to cached states do not replace the replay witness.

## Provenance and evidence

- **FP.17 [D]** The nested provenance block has independent wire version 1,
  irrespective of the containing record's version. It encodes BND.21 to BND.24
  and H9, without redefining the policy. Fields are exactly:

| Field | Wire value |
| --- | --- |
| `block` | `{version:1,form:flow_static\|flow_runtime\|capability_separation,contract:Pin,checker:Pin\|null}`; flow_static requires a checker for program output; manual/import has null checker |
| `author` | `built_in\|own\|forked_built_in\|imported\|unknown` |
| `producer` | `{kind:manual}` or `{kind:import,origin:DigestRef(bytes)}` or `{kind:program,module:DigestRef(source),entry:Text}` |
| `profile` | `solving\|tool_authoring\|research\|none`; `none` exactly for manual/import; program requires a real producing run profile |
| `depends` | For flow forms `{labels:[Text]}` with unique scalar-sorted atoms; for capability form `{read_state:yes\|no}`; either may use `{kind:not_recorded}` only for imported/unknown legacy detail |
| `parts` | Ordered DigestRef(content) list of component provenance blocks |
| `pick` | null or `{record:Ref,chosen:DigestRef(content)}` binding chapter 07's recorded pick, a `c600-pick` record (FP.38) |
| `declassified` | List of `{entry:Text,contract:Pin,record:Ref,output:DigestRef(content)}`; each requires the chapter 07 check, never a declaration of permission |
| `legacy` | null or `{source:DigestRef(bytes),labels:[Value],missing:[Text]}`; original labels retained, not reclassified |

Missing/unreadable fields read `unknown` under BND.21 and the source bytes remain
recorded. They never default to `profile:none`, `author:own` or empty dependencies.
An unknown form/version/atom or unavailable policy/checker pin refuses policy-gated
use with `PS1:capability.provenance_incompatible`. Importing a valid block gives
no permission either. Switching between flow and capability forms preserves the
original block and records a migration; no fabricated fine-grained flow or
`read_state:no` is inferred. The current policy must justify a use from the
available facts, or refuse. LQ21 chose capability separation (owner, 14 November 2026; RTE.24); BND.74's other
questions remain open.

- **FP.18 [D]** `examples.json` contains four complete provenance blocks: a
  hand-typed word and an imported 0.4 macro in each LQ21 form. The pins are hashes
  of explicitly supplied **fixture contract bytes**, not invented producing runs
  or accepted policy editions. Their active policy outcome is refusal; their
  structural storage outcome is valid. Manual has own/manual/none, empty labels
  or read_state:no, no parts, no pick, no declassification and no legacy. Imported
  macro has own/import/none, not_recorded, and the SHA-256 of the exact retained
  recipe source bytes in both producer.origin and legacy.source. Both examples
  satisfy H9; neither asserts who thought of the word or grants an exception.
  The flow example uses flow_static; changing it to flow_runtime preserves the
  no-run rule and is checked as a further vector.

- **FP.19 [D]** Evidence payload, under FP.2's facts envelope, is `{form:facts,status:complete|recorded,
  statements:[Statement],scopes:[Scope],nodes:[Node],roots:[Text],sources:[Source],
  editions:{order:Pin|null,summary:Pin|null,display:Pin|null,invalid_pairs:Pin|null,
  threshold:Pin|null,import:Pin},
  caches:[Cache],missing:[{field:Text,reason:Text,source:DigestRef(bytes)}]}`.
  The common identity is FP.11. `Statement`, `Scope`, `SearchRecord`, `Covered`,
  `ScopeMembership` and premise/guard classes mean exactly chapter 06's definitions,
  EVD.13 and EVD.59. Their wire fields retain the chapter's names; variant rules
  are given below. No revision/current protection/goal enters a scope. A complete
  payload has no missing structural or claim dependency; a recorded payload can
  hold missing interpretations and supplies **no admitted support**. `order` pins
  the grade-order table and the partial-order function; `summary` and `display` pin
  the summary and display-word functions, so a word recomputes with no cache, and a
  change to one is PS2.6's edition. Unavailable
  display-function editions leave facts stored without a derived verdict. This
  separates archival admission from a check's ability to establish a claim.

| Embedded type | Exact wire structure |
| --- | --- |
| Statement | `{id:StatementId,form:{kind:finite\|universal\|existence}\|{kind:mixed,outer:forall\|exists},negation:StatementId\|null,subject:Text,premises:[Text],guards:[{class:Text,name:Text}]}` |
| StatementId | `{kind:named,stage:Text,commit:Text,name:Text}` or `{kind:typed,content:DigestRef(content)}`; typed statement bytes must resolve and bind all EVD.8 parameters |
| Scope | `{id:Text,covers:Covers,identity:KN0.1 wire form,contract:Text,input:DigestRef\|null,sources:[DigestRef(source)]}` |
| Covers | `{kind:one_instance}`; `{kind:search_set,family:Text,alphabet:Value,bounds:Value}`; `{kind:admitted_domain,name:Text,parameters:Value}`; `{kind:every_reachable,puzzle:Text}` |
| Key | `{statement:StatementId,scope:Text}`; scope names an entry in this payload |
| SearchRecord | `{target:StatementId,declared:{family:Text,alphabet:Value,bounds:Value},complete:yes\|no,stop:{kind:closed\|deterministic_budget\|declared_limit}\|{kind:other,reason:Text},job:completed\|cancelled\|cut_off\|error\|not_run\|not_stated}` |
| ProducingJob | EVD.61's fact: `{source:DigestRef(bytes),pointer:Text,id:Text,outcome:completed\|cancelled\|cut_off\|error\|not_run\|not_stated}`; pointer is an exact JSON pointer into the captured source |
| Source | `{bytes:BlobRef,format:Text\|null,pointer:Text,labels:[{pointer:Text,value:Value}],stage:Text,source_version:Text\|null,attribution:Value\|null,engineering:source\|synthetic\|windows\|performance\|human\|null}` |
| CheckFact | `{tool:DigestRef(source),common_base:[DigestRef(source)],report:BlobRef,kind:Text,result:Value,mode:reused_exact_transport\|fresh_exact_audit\|actual_replay,transport:Ref\|null}` |
| Grade | `{basis:theorem\|certificate\|exhaustive_computation\|exact_computation\|sample,check:none\|producer\|independent_checker\|model_review\|human_review\|formal_proof}`; EVD.14, not spelling, orders grades |
| Covered | `{covers:Covers,input:DigestRef\|null}` derived from the leaf's own record |
| Membership | `{key:Key,member:Ref,check:Grade}` |
| Cache | `{inputs:[DigestRef],functions:[Pin],output:BlobRef}`; reuse only on equality of every input and edition |

- **FP.20 [D]** Facts nodes encode one of these exact shapes, under FP.2's facts
  envelope. Node IDs are unique Text, nodes sorted by ID, references by node ID. A leaf is
  `{id,kind:leaf,key:Key,grade:Grade,identity:KN0.1 wire form,covered:Covered|null,
  record:BlobRef,checks:[CheckFact],membership:[Membership],search:SearchRecord|null,
  producing_job:ProducingJob|null,
  reachability:Ref|null}`. An argument is `{id,kind:and,children:[Text]}`; a
  statement root is `{id,kind:or,key:Key,children:[Text]}`; a premise is
  `{id,kind:premise,key:Key,class:reachability_promise|open_hypothesis|model_assumption}`.
  Children are unique and sorted. Empty AND is refused (I8); an empty OR remains
  unsupported. Validate reference closure, mutual negation, immutable scope,
  identity and coverage under chapter 06 before admitting support. The producing-job
  source must resolve to captured bytes in `sources`, and its pointer must identify
  that job. The checked source interpretation must attribute this leaf to it; a job
  elsewhere in the source is not its producer. Null preserves explicit absence;
  missing attribution stays recorded with a reason. Search/job agreement, not-run
  inertness and negative completion follow EVD.40/EVD.41/EVD.61 before either standing
  pass. Human review and formal proof round-trip without changing EVD.3's admission
  conditions; `formal proof` has no writer in 1.0 (EVD.64), and archived proof-assistant
  facts authorize no live checker. Do not infer
  support edges from repeated words such as passed, reused or independent.
  Cyclic premise dependencies use EVD.22's least fixed point; they are not a proof
  from themselves. Storage introduces no second evidence order.
- **FP.21 [D]** Import-table payload is `{edition:Text,corpus:DigestRef(content),
  rules:[{selector:Selector,field:Text,value:Value,outcome:recorded|facts,
  facts:Value|null,prerequisites:[Text],reason:Text}],coverage:BlobRef}`.
  `field` is an exact JSON pointer (RFC 6901 escaping); occurrence inventory keeps
  array indices. Rule selectors may use the evidence-import enumeration path
  only with the pinned enumeration contract. Rules are sorted by canonical
  selector/field/value bytes. Equal keys with differing outcomes refuse the table;
  there is no first-match guess. Facts are fragments of FP.19/FP.20, never a complete
  claim inferred from a label. An unmet prerequisite yields recorded with its reason.
  Every occurrence gets exactly one rule outcome or an explicit unmapped record.
  Mapping coverage, mathematical validation and source integrity are separate.
  The existing `evidence-import/corpus-labels.json` is a pinned source table, not
  this newly typed wire table; its conversion and coverage record retain its original
  digest, enumeration rule and 387 triples/22,513 occurrences (RESULT.md).
- **FP.22 [D]** `Selector` is `{kind:declared,format:Text}` or
  `{kind:absent,manifest:DigestRef(bytes),source:DigestRef(bytes),pointer:Text}`.
  An absent-format interpretation requires an exact manifest/source/pointer match
  and all rule prerequisites. Zero matches yields recorded/unmapped; two matches
  with different interpretations yields recorded/ambiguous. No filename heuristic,
  synthetic path label or status word supplies a missing format. Original absence
  stays null in Source; an interpretation belongs to the conversion record. Until
  reviewed typed rules exist, all such records remain recorded (EVD.53).

## Carriers, conversion and publication

- **FP.23 [D]** A new research or independent-check bundle is a ZIP archive with
  one `manifest.json` (FP.4/FP.11) and digest-named members
  `objects/<stored-sha256>`. ZIP method is STORE, flags are UTF-8 only, no encryption,
  symlinks, directory entries, comments or extras; DOS timestamp is 1980-01-01
  00:00:00. Version needed/made-by is 20, attributes zero, no data descriptor,
  CRC/size in both headers, no ZIP64. Order is manifest then objects in ASCII
  digest order. Raw/gzip member content follows its BlobRef; ZIP itself adds no
  compression. Duplicate names, case aliases, unlisted entries, absolute/traversal
  paths, local/central header disagreement and trailing data refuse the bundle.
  Manifests exclude themselves from member hashes; the carrier's external stored
  digest covers the whole ZIP. Payload is `{members:[{name:Text,blob:BlobRef,
  role:Text}],sources:[{name:Text,stored:DigestRef(bytes),original:DigestRef(bytes)|null,
  transformation:Text}],envelopes:[Ref],table:Ref,checkers:[Pin],missing:[DigestRef]}`.
  Members sorted by name, source rows by canonical bytes; source names are metadata,
  never extraction paths. Case-only names may refer to distinct digests without
  overwrite. A missing dependency remains listed and prevents the check requiring
  it; bundle integrity alone does not imply replay. A reader resolves bytes in
  memory or by safe digest names and never executes an archived producer.
- **FP.24 [D]** Path B retains PS3.3's named ZIP members and safety rules and adds
  the two digest-named carriers below; its metadata/header rules and limits are
  FP.23/FP.9. Its manifest uses
  `format:"C600-MIGRATE",format_version:1`, not FP.11's common wrapper.
  `c600migrate`/`format_version:1` in the old exporter proposal is a distinct draft
  input; an explicit converter may recognize it, but it is not an alias.
  Manifest fields are exactly PS3.3's `producer`, `model_id`, `created_utc`,
  `head_state_hash`, `head_event_id`, `entries`, `options`, `privacy`, plus
  `limits:"c600-limits-1"` and `conversion:Ref`.
  Producer is `{name,version,source:DigestRef(source)}`; time is UTC Text metadata;
  head hash is labels hex and event ID Text; entries are
  `{path:Text,bytes:Nat,sha256:hex,role:Text}` for every other member, sorted by
  path in byte order, and the ZIP holds `manifest.json` first, then the members in
  entry order. Roles are `session`, `paths`, `labels`, `macros`, `keymap`,
  `workspace`, `raw` and `log` for PS3.3's members, and `record` and `blob` for the
  carriers. Options is `{include_logs:Bool,raw_copy:Bool}`; `logs/` and
  `raw/session.sqlite3` members are present exactly when these say so. Privacy is
  `{publication:private}`. Raw SQLite
  and logs remain private and are not research-publication inputs. Entries are
  checked before reading logical content. `session/record.json` has
  `{format:"c600-session-record",version:1,head:Ref,checkpoints:[Ref],
  preferences:Ref,attempt:Value,timer:Value,unmapped:[{item:Text,reason:Text}]}`;
  `head` and `checkpoints` name checkpoint records, and `preferences` names the
  session's preferences outside the workspace.
  Checkpoint label files retain their raw bytes; `session/checkpoint-paths.json`
  is a replay-path record. Workspace members contain the corresponding FP.4
  record or retained exchange bytes with a conversion record; no empty macro
  record is invented. Unsupported attempt/timer meanings stay recorded.

  Dependency carriage (migration-format-freeze ruling, MFF-01). The entry points are
  the manifest's `conversion`, `session/record.json`, `session/checkpoint-paths.json`
  and each workspace member that holds an FP.4 record. A workspace member holds
  retained exchange bytes exactly when an `outcomes` item of the `conversion` record
  names its path with status `recorded` or `refused`; its DigestRef(bytes) is then
  among that record's `inputs`, and its bytes are checked against their entry and
  never read as a record. Any other workspace member holds the FP.4 record of its
  kind: `c600-macro-set` in `workspace/macros.json`, `c600-key-map` in
  `workspace/keymap.json` and `c600-workspace` in `workspace/workspace.json`.
  The entry points' content Refs, followed
  through every resolved record, form the closure. Each closure record is carried
  once: by the named member that holds it, otherwise as `records/<hex>.json` with
  role `record`, where `<hex>` is the Ref's `content.sha256` and the bytes are
  exactly the record's FP.5 bytes. A Ref resolves to the one record carrier (a
  `records/` member or a named member above) whose entry `sha256` is its digest;
  FP.10's dispatch, FP.5's canonical check and FP.11's format/version equality
  follow. A checkpoint's labels `state.data` is a raw BlobRef whose carrier is its
  label member: `session/checkpoints/head.labels` for `head` and
  `session/checkpoints/<i>.labels` for entry `i` of `checkpoints`, counted from 0
  in decimal. FP.7's raw threshold does not apply to these retained uncompressed
  members. Every other BlobRef in the closure is carried once as `blobs/<hex>` with
  role `blob`, where `<hex>` is its `stored.sha256`; its stored size and digest,
  then its decoded size and digest under its encoding, are checked. A bare DigestRef
  or Pin names bytes and is not a dependency. Raw and log members are never
  resolution inputs, so validation and replay give the same result with `raw_copy`
  true or false. A missing carrier, a second record or blob carrier of one digest,
  or a `records/` or `blobs/` member outside the closure refuses the package with
  `PS1:input.persistence_format`; a carrier whose name and entry disagree gives
  `PS1:input.digest_mismatch`. No cache, path guess, raw copy or log is consulted.

  The head checkpoint's `path` is `session/checkpoint-paths.json`, whose `head` is
  `head_event_id`. Each offered checkpoint's path record has the same `root` and
  `events` and selects its own `head`; every event lies on the head path or on an
  offered checkpoint's path. Each path is replayed from its root to its head (PS3.5,
  PS3.8). Every event's `pre`/`post`, the replayed labels, the label member,
  `state.digest` and, for the head, `head_state_hash` must agree, otherwise
  `PS3:input.checkpoint_replay`. Each checkpoint restores the preferences record its
  own `preferences` names.
- **FP.25 [D]** Conversion walks every target in PS3.3 before publication:
  recipes retain word/star chronology and exact counts; macros retain their source
  and FP.17 provenance; protection requires complete rule/member translation;
  filters/preferences retain every item with its outcome; checkpoints retain each
  path and captured preferences; twist keys retain the old literal word; command
  keys require H12's version/digest. Unknown workspace data needs the retained raw
  database or the entire import refuses. Failed protection always refuses. A/B
  certificates stay recorded pending fresh checks; C is not persisted as evidence
  and D is never permission. No source log is mistaken for a complete path B archive.
- **FP.26 [D]** `check_vectors.py` converts in-memory retained recipes and replays
  their primitive permutations from `assets/primitives.npz` over all 259,800 labels.
  It checks a noncommuting chronological word and a word with a star, full labels,
  per-prefix hashes, net source/destination digest and the oracle expansion vector.
  The data is read with a small standard-library NPY reader; no engine or database
  is started. This is a reference byte/conversion check, **not** evidence that the
  future 1.0 engine matches the oracle or that every migration fixture passed.
  Integration still needs O01 to O07 and each offered checkpoint with the actual
  candidate engine. A reversed path or wrong pre/post digest must refuse import.
  It also builds FP.24's golden Path B packages, a fresh session (F12) and an
  abandoned-branch checkpoint with its own preferences (F10), each with `raw_copy`
  true and false, matches each against its recorded package digest, and then, from
  the archive bytes alone, resolves every closure Ref and BlobRef, replays both
  heads and compares the preferences. Its mutation vectors refuse a missing record
  or blob carrier, an extra, second or misnamed carrier, and exchanged labels. These
  are synthetic packages, not importer output. The golden family holds recipe events
  from the solved root, `jumble_off` checkpoints and workspace members that hold FP.4
  records; it has no key map, logs or retained exchange bytes, and the reference
  reader refuses shapes outside that family instead of implementing them.
- **FP.27 [B]** Source preservation follows PS3.1/PS3.2's amended locked-copy
  contract: existing read-only engine.lock, guarded byte copy of database/WAL,
  no source SQLite open, no immutable=1, no source -shm access, no source writes.
  Recovery and backup happen only on the copy. The uninterrupted drained watch,
  snapshots and copied digests must all pass before using the copy. A watch event,
  gap, error or overflow refuses publication without attributing a writer by timing.
  The historical P1/P2/P3 evidence is `migration-probe-results.md` for its recorded
  source and platform only. This packet performs no new Windows durability probe.
- **FP.28 [D]** Publication follows PS3.9: complete validation and replay from the
  package alone, durable new generation/archive and directory entries, then durable
  active-pointer switch. Retain old generation and recovery checkpoint. Durable
  publication metadata is `{format:"c600-publication",version:1,id:Text,
  previous:DigestRef(content)|null,next:DigestRef(content),archive:DigestRef(bytes),
  validation:Ref}`; the active pointer identifies exactly this metadata's digest.
  It is stored locally, not a reusable grant. A missing dependency, or a failed
  durability barrier before the pointer switch, leaves the previous generation
  active. A failure after a possible switch is reconciled as below, not assumed to
  leave either generation active. On an uncertain switch,
  read the authoritative pointer and validate the named generation before retry;
  otherwise return `PS3:unknown_outcome.import_publication`. Live undo begins at
  the imported head. Publication metadata does not replace a platform's durability
  contract or prove power-loss safety.
- **FP.29 [D]** Migration payload is `{source:{format:Text,version:Value,
  edition:Text|null,bytes:DigestRef(bytes)},target:Ref|null,converter:Pin,
  inputs:[DigestRef],outcomes:[{item:Text,status:converted|recorded|refused,
  reason:Text}],checks:[Ref],semantic_changes:[Text]}`. Original bytes remain
  available. A refused/recorded item has no fabricated target. A target's optional
  conversion backlink is null if it would form a content-hash cycle; the separate
  migration record binds source to target, and the import report indexes that
  record. No record embeds its own digest. Schema and source
  editions are independent. Changing facts-function pins creates a new derived
  record with old and new editions/input/output digests. Missing/newer dependencies
  refuse activation and retain the original; equal output hashes alone cannot
  authorize reuse (PS2.7).

## Durable exit lineage

- **FP.30 [D]** H11 defines validity, Position and Context; this is their wire
  encoding, not another navigation rule. Position has `head:Text`,
  `mode:jumble_on|jumble_off`, `episode:Text|null`, `state:DigestRef(labels|poses)`.
  Context has `protection:Ref` and `preferences:Ref`. The exit payload is
  `{id:Text,episode:Text,crossing:Text,before:Position,after:Position,
  poses:BlobRef,pose_digest:DigestRef(poses),labels:DigestRef(labels),
  certificate:Ref,contract:Pin}`. Before is the pre-exit pose position; after is
  the label position. Exact exit poses and certificate remain immutable when
  inactive. The exit has no authoritative `valid:true` flag. A never-jumbled
  session stores no exit/void section (KN4.5).
  Recovery requires `pose_digest` to equal `before.state`, resolves the pose
  blob and checks its bytes against that digest, and requires both recorded
  Positions to equal the exit transition's corresponding sides.
- **FP.31 [D]** The H11 transition payload is `{id:Text,sequence:Nat,previous:Ref|null,
  action:retained_move|jumble_move|undo|redo|exit|entry|branch|reset|import|restore|workspace,
  before:Position,after:Position,context_before:Context,context_after:Context,
  event:Event,exit:Ref|null,entrance:Entrance|null,voids:[Ref],command:Text,
  request:DigestRef(content),admission:Ref,receipt:Ref,revision:RevisionRef}`.
  RevisionRef uses KN0.2's field names, FP.11 identity, Nat counters and a typed
  state digest. CommandId is Text. The chain is append-only and gap-free;
  `before` and `context_before` equal the previous `after` and `context_after`.
  The first row binds a complete session root. Undo appends a row while moving
  the journal head; it does not erase history or decrease revisions.

  A mechanical word commit while Jumble is on uses `jumble_move`, including
  `jumble.twist.menu` and every word commit at pose level. A word commit while
  Jumble is off uses `retained_move`. Both append journal events with their exact
  literal words and before/after states; only `retained_move` may occur in an
  effective exit suffix. Undo/redo of a `jumble_move` uses its own recorded pose
  sides and episode, with current Context preserved under the rules below.

  `Event` is `{kind:journal,id:Text|null,target:Ref|null,
  path:[{direction:undo|redo,transition:Ref}]}` or
  `{kind:workspace,target:Ref|null,inverse:Ref|null,
  changed:[{before:Ref|null,after:Ref|null}]}`. A journal ID names its mechanical
  event; `target` identifies the original action for undo/redo, otherwise null.
  Branch `path` is the proved chronological inverse/forward navigation; other
  journal actions have an empty path. Workspace inverse payload is the same
  `changed` list with before/after swapped, with provenance under FP.11. Null is
  creation/deletion, never both null; changed records have distinct workspace IDs.
  Inverse data contains no reference back to the transition, avoiding a hash cycle.
  A workspace action or its inverse has equal before/after Position, advances
  workspace versions and leaves state revision unchanged (LI.9, KN0.2).
  `action` classifies the effect, not the command row kind. `inverse` is null
  exactly on a non-undoable durable action: `session.checkpoint.save` is a commit
  row with `action=workspace`, a `committed` receipt with equal pre and post
  digests and no eligible inverse (LI.9, JI.27). It still advances the action
  order; a hint that names it as target gives `KN5:stale.undo_superseded`.

  `Entrance` is `{kind:exit_root,exit:Ref,crossing:Text,suffix:DigestRef(content)}`
  or `{kind:solved_witness,word:Ref}`. The suffix digest identifies an included
  chronological list of retained-word references on the proved selected path,
  including the empty list. Entry validates that binding and replay agreement
  under JI.25/JI.27 before committing its own two sides and new episode. It may
  use a solved witness with no exit. That word and its dependencies are included
  records: resolve them, validate the signed-generator encoding and pins, and
  replay from solved to check agreement with the entry's labels and resulting
  poses before entry or recovery admits the binding. FP.13 supplies the empty
  witness after solved reset without a legacy recipe. A missing/null word or
  missing target is `PS3:input.checkpoint_replay` during recovery.
  `exit` is non-null only on an exit action;
  `entrance` is non-null only on an entry. Undo/redo targets preserve these facts
  by reference to the original transition, never by reusing an exit's sides for
  an entry. Redo revalidates the original binding and exit identity; a voided
  binding refuses `KN5:input.exit_record_voided` (H15.16).

  Crossing, branch navigation and journal undo/redo restore Position only.
  Current preferences and protection remain, except a crossing's own recorded
  protection-level change (JI.27), which is reversed/reapplied with that crossing.
  Changed protection still passes the command layer's current checks; this is no
  historical permission. Workspace inverses restore only their changed records.
  Every navigation row records the resulting current Context, so both continuity
  checks hold even when preferences changed after the original crossing.

  One transaction writes the transition, affected journal/workspace/exit/void
  records, complete Position and Context, new RevisionRef, action pointer and
  FP.36 terminal receipt with its admission binding. A retained inverse selects
  the moves in effect at the new head and retains the exit poses as replay root.
  Branch eligibility follows H11's selected path, never a search by equal labels.
- **FP.32 [D]** A void payload is `{exit:Ref,transition:Text,
  reason:reset|import|restore}`. A break transaction voids every earlier exit in
  that session lineage, including one on an inactive branch, so undoing a head
  change or selecting an old branch cannot resurrect it. The void is monotone
  durable history, not part of the undoable state. Recovery loads only complete
  committed transactions, checks references and the action chain, folds voids,
  finds the crossing on the selected parent path, and checks that the suffix has
  only `retained_move` actions under FP.31; a `jumble_move` in that suffix cannot
  supply an `exit_root`. It replays those retained moves from the stored exit poses and
  compares state digests before any resumed Jumble entry. No matching chain permits
  no exit-based resume, even when labels match another exit. Without a separate
  retained witness allowed by A5, a labels-only request gives
  `KN5:input.jumble_on_labels_only`; an already-on position needs no new entry.
  The labels-only refusal vectors supply no alternative witness; the separate
  solved-witness entry vectors explicitly supply one. A partially written crossing
  transaction recovers the entire before position; a committed one recovers its
  entire after position. A corrupt/missing chain is `PS3:input.checkpoint_replay`,
  never a label-based reconstruction. This is the durable encoding of H11, not a
  new authority to relax A5. Recovery checks both Position and Context continuity
  and FP.36 bindings, and retains monotone voids even across inverse break actions.
  An on-mode chain containing entry, `jumble_move` and checkpoint save recovers
  its exact pose head and episode; undo and redo of the move remain journal
  actions even though save itself has no eligible inverse.
  A subsequent exit retains that moved pose. Recovery checks each retained-move,
  Jumble-move and exit event's kind and parent against its transition's action
  and before head. For an exit-root entry or its redo, it compares the recorded
  after state with replay from the bound exit over the entire effective retained
  suffix; an empty suffix preserves the exit pose digest.

- **FP.36 [D]** Durable admissions and receipts encode KN5.21/KN5.23 and LI.30,
  without storing grants or tokens. Admission payload is
  `{command:Text,request:DigestRef(content),session:Text,epoch:Text,row:Text,
  input:BlobRef,base:RevisionRef}`. `input` is FP.5 bytes of LI.5's resolved request:
  `{row:Text,args:Value,guards:Value,identities:[DigestRef],budget:Value|null,
  grants:[Ref],token_binding:Value|null}`. It contains no secret authenticator or
  reusable permission. Request hashes these bytes; the row and request binding
  agree with the admission. All fields required by the row enter `args`/`guards`,
  including the action hint and resolved lane target where applicable.
  Before recovering a terminal, resolve and decode the input bytes, validate
  that request against its row's schema and bindings, and recompute its request
  digest. Validate `base` as the complete pre-action RevisionRef, including
  session, epoch, identity, mode, head, counters and typed state digest. Agreement
  between copied request fields or rehashed enclosing records is insufficient;
  a missing or inconsistent input/base gives `PS3:input.checkpoint_replay`.
  Undo/redo targets resolve to an already recovered transition matching the
  event target; a legacy head names its recorded destination side. Journal
  inverses/forwards restore that side. An explicit branch path matches the
  recorded directions and transitions, walks contiguous Positions and ends at
  the row's after Position. Entry kind, workspace preferences/protection/checkpoint
  changes, and any requested crossing protection match their recorded bindings.

  Receipt payload keeps KN5.21's exact `command`, `request`, `job` and `status`
  fields; job is Text or null, and status uses FP.11 tagged choices. `committed`
  carries `record:{event:Text,pre:DigestRef,post:DigestRef,revision:RevisionRef}`;
  `completed` carries `completion:{kind:workspace,revision:RevisionRef,
  result:null|{job:Text|null,digest:DigestRef(content),covered:Value|null}}` for the
  durable workspace case, encoding KN5.21's ResultRef. The other retained terminals
  are `{kind:failed,error:Error}` and `{kind:cancelled,reason:CancelReason}`, using
  KN5.21/chapter 13 fields with no rewritten code. All
  state digests preserve their level. A workspace inverse invoked through
  session.undo/session.redo is committed with equal pre/post puzzle digests
  (H15.1); a workspace row itself completes with kind workspace.

  The admission is durable before accepted work can publish. Effects, terminal
  receipt and transition become durable atomically, and their command/request,
  revision and typed pre/post digests must agree. Receipts refer to event IDs,
  not transition content, so references are acyclic. Undo, restart and notification
  loss retain the original receipt and its original revision. Lookup joins the
  retained admission and receipt by command/request even when no transition was
  published; an index is only a cache of these records. Current-state
  equality supplies no outcome. LI.30 governs lookup, including ID/request reuse.
  A surviving admission without a transition can acquire
  `failed(KN5:resource_limit.engine_lost)` only after recovery proves rollback
  and no publication. Persist that terminal against the admission; create no
  state transition. Incomplete proof remains unknown_outcome/read-only recovery.
  No automatic retry or archived token accompanies either result.

- **FP.37 [D]** `PlaybackState` (PS1.44) is a `c600-playback-draft` record: an FP.11
  record outside the data core, with null `provenance`, saved in the workspace
  carrier (PS1.16) and outside the undo order. The carrier's workspace record
  (FP.12) has `kind` `c600-playback-draft` and `value` a Ref to this record, and it
  keeps its own provenance block. The payload is
  `{playback:Text,recording:DigestRef(content),step:Nat,pending:null|{command:Text,
  request:DigestRef(content)},actions:[{recorded:Nat,played:Nat}],status:Status,
  last:null|Ref}`. `Status` is `{kind:running}`, `{kind:waiting,step:Nat}`,
  `{kind:stopped,stop:Stop}` or `{kind:finished}`. `Stop` is `{kind:code,code:Text}`
  or `{kind:outcome,outcome:refused|failed|cancelled|unknown_outcome}`, and a child's
  code stays in the unchanged receipt in `last`, the Ref of an FP.36 receipt. FP.10's
  refusal of an unknown format or version comes first; after it, a resolved record
  whose `format` or `version` differs from its Ref refuses with
  `PS1:input.persistence_format`.
  `actions` is sorted by the number `recorded`, each recorded number once; unsorted or
  duplicate entries refuse with `PS1:input.noncanonical_value`.
  `c600-proof-obligation-draft` (PS1.45) is an export schema outside the data core.
  The `playback` vectors check EV4.18's map under CT-EV4-20; `playback_from`,
  `playback_trace`, `playback_arguments` and `playback_storage` check EV4.22's table,
  EV4.14 and this wire form.
- **FP.38 [D]** `c600-pick` is chapter 07's `PickRecord` (BND.54): an FP.11
  record with null `provenance`. The payload is
  `{list:DigestRef(content),fixed:Bool,options:DigestRef(content),
  chosen:DigestRef(content),target:Value}`. `chosen` digests the option and `target`
  holds the target or content it fixed. FP.17's `pick` names it, and the two `chosen`
  fields are equal. FP.10's refusal of an unknown format or version comes first. A
  pick whose record does not resolve, whose admitted record's `format` or `version`
  differs from its Ref, or whose `chosen` differs, refuses policy-gated use with
  `PS1:capability.provenance_incompatible` (BND.21, BND.85). The `picks` vectors hold
  a built-in pick under each LQ21 form.

## Evidence, conformance and remaining work

- **FP.33 [D]** The PS1.38 result is input evidence, not reviewed format proof:
  `work/experiments/language-lx/envelope-facts/RESULT.md` reports 497 partial
  envelopes, zero complete typed envelopes, 15,210 fact bundles, 11,467 leaf slots
  and no admitted graph edges. Its compressed sizes use that experiment's recorded
  serializer/zlib, not FP.7. EVD.62 maps the four stored display collisions to
  draft corrections; the experiment is not rerun here. Missing statement, scope,
  covered-set, membership, checker/common-base and reachability bindings remain
  with the integrator/chapter 06. No complete corpus interpretation or
  lossless judgement mapping is claimed. The synthetic 6,775-digit vector tests
  width; it is not asserted to be the historical B11 bound. No performance or tariff
  is measured here.
- **FP.34 [D]** Run `python -B docs/architecture/1.0/language/format-packet/check_vectors.py`
  and `python docs/architecture/1.0/language/closure/check_closure.py --packet p3`.
  They use standard-library code, repository inputs and memory only; they do not
  write caches, reports or refreshed vectors. `vectors.json` supplies concrete
  given/expected values. Checks recompute canonical bytes and hashes, exact-number
  round trips, deterministic gzip, both provenance forms, unknown-field outcomes,
  version-first dispatch, ia16 chunks and relations, alias identity, retained
  conversion, golden Path B packages, producing-job
  facts, the four evidence corrections, durable-history/outcome examples, playback
  records and `from` transitions, picks, evidence editions and the obligation
  translator. Limit vectors
  exercise the count admission at the exact boundary and one past it without
  allocating a 4 GiB archive; they are not large-file throughput tests. Every
  expected digest is compared with freshly computed bytes. Changing an expected
  value alone cannot make a mismatched byte vector pass.
  Durable vectors model a small synthetic journal and pose transform, with
  full-label retained replay for solved witnesses. Synthetic move bytes are
  FP.5's `{before:pose_digest,letters:literal_letters}`; nonempty retained replay
  bytes are `{before:exit.pose_digest,suffix:chronological_word_refs}`. The empty
  suffix preserves the exit's bytes. Builder and recovery walk the parent
  events to the crossing, including paths after undo and with multiple moves.
  Exit blobs store those actual synthetic bytes. Requested `pose_root`
  observations while on report the exit pose bound to the resulting Position:
  entry binds its head, Jumble moves carry that root, and undo/redo or explicit
  branch paths select the root at their resulting head. A solved-witness entry
  has no exit root;
  `valid_exit` still describes off-mode resume eligibility. Older head-only
  branch fixtures select predefined positions; explicit paths are walked.
  These vectors do not establish product command admission, jumbling-engine
  pose replay or filesystem durability. The
  paired evidence vectors enumerate a tiny declared fixture language; they are
  not general evidence-graph conformance or the separate mathematical ruling.
  In these fixtures, EVD.61 agreement and I5 precede structural I2/I7 checks,
  then EVD.41 withholds inert support before I6. Absent coverage and no-form
  coverage flags leave mutual-negation and leaf/scope identity checks active.

| Test | Checks | Class |
| --- | --- | --- |
| CT-FP-01 | Three JSON profiles, canonical numbers, gzip, all limit rows and unknown fields; version-first dispatch of records and Refs with added or missing fields, and refusal of former draft identifiers | source |
| CT-FP-02 | Complete manual/import blocks under both forms, a non-null built-in pick under each form, a pick with no resolving record, missing profile, incompatible form and no permission | source |
| CT-FP-03 | ia16 chunk round trip, independent menu identity/order pins, invalid code/length and retained conversion over the full label array; empty signed word with legacy recipe minima preserved | source |
| CT-FP-04 | Exit and both materialized entry forms (including empty witness), Jumble move/save/recovery/inverses, current protection members across crossing-level deltas, monotone voids, outcome lookup validating resolved request input/base after rehashed corruptions, including a corrupted revision, and a complete playback record with its workspace wrapper, pending binding, actions, last receipt and `from` transitions; golden Path B packages (fresh session, abandoned-branch checkpoint, raw copy on and off) resolved and replayed from archive bytes alone, with missing, extra and misnamed carriers and exchanged labels refused | synthetic |
| CT-FP-05 | Source-discriminated absent-format dispatch and empty-AND refusal | source |
| CT-FP-06 | Human/formal grades and source-bound producing jobs round-trip; EVD.62 collisions and EVD.42's paired finite exhaustive negative with each guard broken run on decoded facts under a trial threshold only; with no cache, a display-function change keeps the old pin and records the new edition | synthetic |
| CT-FP-07 | JI.23 local/part/ref relations, missing/cyclic targets, changed inverse and mandatory flat chunks; graph archival is distinct from word admission | source |
| CT-FP-08 | Exact alias version/profile/digest conversion; a final LF, changed version or changed table fails the pin | source |
| CT-FP-09 | Draft chapter drift is reported; packet-file drift and frozen chapter drift stay fatal | source |

- **FP.35 [O]** Open dependencies are deliberately not called closed. The owner's
  answers of 6 November 2026 settle the envelope, the freeze scope, playback, module
  sharing, gate order and formal-proof use (LQ3, LQ10, LQ11, LQ12, LQ15, LQ19).
  LQ21 chose capability separation (owner, 14 November 2026; RTE.24); BND.74's other questions still select policy,
  and EN3 pins X3. The integrator
  refreshes FP.15 and candidate inputs after p7-table lands and checks H11 traces.
  The integrator closes the remaining chapter-06 typed binding gaps and converts the complete corpus
  table into typed rules (or explicitly records remaining unmapped data), reviews
  the PS1.38 candidate and reruns
  `python -B work/experiments/language-lx/envelope-facts/check.py` outside this
  packet. After assembling all drafts, run
  `python -B docs/architecture/1.0/language/closure/check_closure.py --all` and H13's
  four reconciliation traces, and
  `python -B docs/architecture/1.0/language/closure/check_p7.py --all`.
  With the engine environment, run
  `tools/.venv/engine/Scripts/python.exe -B tests/test_oracle.py`; this is reference
  oracle regression only, not a future candidate-engine verdict. Importer and
  Windows preservation/publication fixtures need the eventual implementation;
  no executable acceptance command for that absent importer is invented. A current
  candidate review and the separate Astra migration-format-freeze gate still
  precede owner sign-off. No reviewer call is made by this network-free packet.
