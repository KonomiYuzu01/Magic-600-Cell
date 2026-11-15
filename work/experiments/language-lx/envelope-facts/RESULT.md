# PS1.38 facts-envelope measurement

| CT-RTE-05 field | Record |
| --- | --- |
| LX card | PS1.38, supplementary evidence card E-2.4-LE; CARD.md |
| Experiment ID | E-2.4-LE / PS1.38 |
| Owner allowance | docs/wiki/decisions/owner-decisions-2026-11-05.md, decision 2; packet PS1.38 |
| Time box | what can be done now; one wrapper call of at most two hours |
| Frozen candidate | CANDIDATE.json; canonical inventory SHA-256 `d47c2b0021af0a045c98fc5d0c232efaad1701ae945ebe8b32ca140c4b90b0b8` |
| Frozen inputs | Git `1e7d0074df96cbeb05f55282cf8f6c03fbf4a4ff`; MANIFEST.json; record inventory SHA-256 `f0e35b4dce8512ea11c8544de89294681df30e845b7fa832c37c0211e5a4b0f4` |
| Commands | `python work/experiments/language-lx/envelope-facts/check.py --write`, then `python work/experiments/language-lx/envelope-facts/check.py` |
| Tools | CPython 3.15.1; git version 2.55.0.windows.5; zlib compile 1.3.1.zlib-ng, runtime 1.3.1.zlib-ng |
| Evidence class | Source: corpus counts and serialization; synthetic: finite display cases. No new mathematical evidence. |
| Kill criteria | Wrong baseline, missing/duplicate record, label coverage mismatch, unsupported fact/edge, non-unique display result, stale artifact, or time limit: fail. |
| Actual result | 497 partial envelopes; 0 complete typed envelopes; every source record and 22,513 label occurrences accounted for. |
| Decision / decider | open / owner; no LQ answered and neither chapter accepted |
| Cannot show | A product migration format, lossless judgements, complete mathematical argument graphs, new verification/replay, an accepted display policy, performance, or any route/engine-language/syntax choice. |

## Corpus identity and equal inclusion

The source stored baseline is **497 records, 13,412,769 bytes**, comprising 417 `.json`
and 80 `.json.gz` members, no JSONL. Both the source and envelope columns include all
497 members, including lists, absent-format records, publication metadata and records
with no mapped label. No corpus file is read from the checkout or materialized by name.
Git `ls-tree` supplies paths and object IDs; `cat-file --batch` supplies verified bytes.

The full stage-14-to-19 manifest (632 files, including non-records) is
`00693f6c3b4c1f04c30f0fd15744c21be155da50c28b9c66c4b4511797aac35d`. It **matches** the README's
post-rename digest at `8af824a4`; there is no corpus change to explain at this commit.
The older label table's `e2b388be...` digest predates the six certificate renames and
the stage-16 README note. It uses the same 497 record contents. The current manifest
has no case-colliding paths. Its digest uses the extractor's path-NUL-OID-LF encoding;
the separate record-only digest includes sizes and SHA-256 boundaries and is not comparable
to that full-corpus digest. The recorded label table stays frozen, including its old paths.

## Nodes, fields and missing semantics

The envelopes store **15,210 source-object fact bundles** assembled from
multiple rows where present, with **11,467 leaf slots**, **0 AND nodes,
0 OR nodes and 0 edges**. These are incomplete leaf slots, not admitted support:
**0 leaves have a complete statement/scope key**. No artificial OR groups unrelated
claims, and no AND is inferred from a reused/check-mode label. Graph storage links
(row references, digest references) are counted separately from evidence edges.

All **497 records cannot form a complete typed envelope with these rules**. Every path
and every missing-field reason is listed in RECORDS.json. The common gaps include
statement identity and form, negation pair, subject level, typed identity/scope,
membership, search-to-statement binding, premise/guard classes, per-leaf covered set,
checker/tool/common-base bindings and reachability bindings. These are missing
*typed interpretations*, not claims that the raw records contain no relevant data.
Exact source pointers retain input tables, options, certificates, proof text, checks,
attribution and engineering classes where present; absent attribution stays absent.

The table has 387 triples: 298 mapped, 89 recorded with reasons, zero without a rule.
UNMAPPED.json lists all 89, with their original values, counts and reasons. The 24
external/code-declared rows are outside this corpus and create no envelopes. Prose,
payload booleans and numbers remain in pinned source bytes; prose scope strings also
remain beside the record's facts. All 22,513 label occurrences are verified against
exact JSON pointers and the frozen table's per-triple and per-file counts.

V14 remains a table interpretation of model review; it is never KN6 evidence. P13
and other mode labels add no transport edges. K1 gets no inherited checker without
a demonstrated covering relation. H1 rows have separate reachability bundles and
supply no support. A same-object bundle may describe several statements, so candidate
field values are retained as alternatives, never merged into a judgement. In particular
a failing guard and a completed job cannot turn into one fabricated contradiction.

## Bytes and sharing

| Boundary | Bytes |
| --- | ---: |
| Original members as stored (baseline) | 13,412,769 |
| Original members decoded, before canonicalization | 58,976,540 |
| Original values, canonical uncompressed | 54,282,023 |
| Original canonical values, gzip every member | 6,273,978 |
| Envelope metadata, canonical uncompressed | 10,424,730 |
| Envelope metadata, gzip every member | 1,156,098 |
| Envelope package including common artifacts, canonical/captured bytes | 13,242,586 |
| Envelope package including common artifacts, gzip bytes | 1,637,387 |
| Captured original members plus gzip envelope package | 15,050,156 |

SUMMARY.md gives stage totals and every component. Envelope-only bytes are **not** a
replacement for the records. The source-preserving figure keeps the captured inputs
once, so compression/formatting changes cannot erase their original digest boundaries.
These sizes measure this partial facts model; a future complete argument graph may be larger.

The import table, chapter/edition pins and other common input artifacts are referenced
497 times and counted once in the package. DEPENDENCIES.json accounts for reported
digest reuse: **1,453 digests occur in multiple records**;
**189** have an exact stored/decoded byte match in the scoped
inputs, and **4,070** have no such match. No match is
invented from a filename. A reported digest can identify a state, an unavailable private
source edition or content; equal text is not automatically a shared evidence leaf or a
tool dependency. No independence or common failure-point claim is inferred from it.

Envelope-array canonical SHA-256: `c37c6a57208a51d974d45110b4720041cd1d2fac01ef63d3d642f0d19e4d0f2c`
(10,425,228 bytes). Ten full envelopes are in SAMPLES.json;
all 497 canonical and compressed digests are in RECORDS.json. The full dependency
catalogue digest is `aab524d0b64cee3445d09295c5fdf46a388dec5f6669b07183b10fbba0846797`.
The full arrays are regenerated in memory by the acceptance command, not committed.

## Display enumeration and open rules

DISPLAY.json records **8,749,440** combinations of the explicitly
bounded synthetic facts model in envelope_model.md, including all five mode states.
Each has one word or one typed inconsistency. The output-stream SHA-256 is
`aa2e7c4d6fddd115df1ca07af7bc05e329eaa99cf4ffe28261d73ed9cde03790`. The 504 EVD.32 node-level cases also run,
alongside 31 fixed checks. Every actual partial envelope derives
`open` with its source labels retained, because a typed scope/key has not been bound.

- **Open collision:** `EF:collision.not_run_vs_budget`. With no standing support,
  incomplete search plus a deterministic/declared limit meets EVD.32 row 5, but
  EVD.47 says `not run` reads row 7. The probe reports a typed collision instead of
  resolving it; 146,120 combinations expose it.
- **Open collision:** `EF:collision.existence_refutation_basis`. An exact-computation
  leaf for the negation of an existence statement can pass the equal-coverage I7 test
  and reach EVD.32 row 1, but EVD.42 requires a proof or complete certificate. The
  probe retains this as a typed collision (1,600 combinations);
  it does not invent a basis conversion or claim that the counterexample is sound.
- **Open job/support combinations:** `EF:collision.not_run_vs_support` and
  `EF:collision.negative_job_without_completion` occur in
  123,000 and
  28,500 combinations respectively.
  In these fixtures the job belongs to the supporting leaf. Standing support reaches
  rows 1-4 despite a not-run job (EVD.47), or a search-record-free negative reaches
  row 1 despite its producing job failing to complete (EVD.43). I5 already catches
  search-backed negatives. These cases need a rule or an admission restriction;
  the probe preserves typed collisions. It makes no assertion about support from a
  different, already completed historical job. DISPLAY.json gives concrete tuples
  for every collision so the binding assumption is reviewable.
- **Open edition:** LQ3 has not chosen the threshold or invalid pairs. The synthetic
  run pins trial U0 and the chapter's initially empty invalid-pair table, not a product
  choice. A stricter threshold may change supported words. LQ15 remains open too.
- **Missing facts:** an exhaustive leaf without its search record, a cut-off search
  with no stop category, and a mixed-universal refutation without its required witness
  have experiment-local typed inconsistencies. They do not add chapter-06 error codes.
- **Resolved priority, not a collision:** a budget-limited search and an error/cancelled
  job can match rows 5 and 6; EVD.32 explicitly says first match wins. The probe uses row 5.
- **Unmapped semantics:** absent-format dispatch (PS3.12/EVD.53), source-to-statement
  bindings, transport relations, guard/premise classes and covered-set construction
  remain open. The source facts and typed reasons preserve these gaps.

## Validation and limits

The acceptance command rebuilds the Git manifest and all outputs, checks the frozen
baseline, per-label counts, per-record accounting, pointers, digests and absence of
invented support, exhausts the bounded display model, and compares every frozen artifact.
It writes no build directory, bytecode cache or binaries, even on failure. `--write`
refreshes only the listed experiment outputs. No timing result is claimed.

Not run here: full chapter-06 conformance on arbitrary graphs, general whole-record
statement interpretation, judgement-option mapping, historical mathematical checkers,
WSL, GPU/native device runs and decisive performance measurements. They are not required
by this source/synthetic packet, and no executable command for the unimplemented
semantic work is claimed. There is no required external hardware command for PS1.38.
The integrator must review this candidate and rerun exactly
`python work/experiments/language-lx/envelope-facts/check.py` after applying it. This run does not accept chapters 05/06,
freeze a migration format, answer LQ3/LQ10, or select a route, engine language or syntax.
