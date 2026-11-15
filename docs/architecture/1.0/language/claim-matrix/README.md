# FR-B08: claim, checker and dependency matrix

Status: **architecture input, frozen with the LP1 candidate** (owner sign-off, 15 November 2026;
[decision](../../../../wiki/decisions/owner-decisions-2026-11-15-freeze.md)); the `status` string in `matrix.json` keeps
the wording the rulings read. This selects no
route, engine language or syntax, answers no LQ, and changes no chapter. Research
remains stopped. The matrix is an independent-checking **plan**, not a new check
report or mathematical result.

## Method and reading rules

- **CX.1 [D]** `matrix.json` is regenerated from the stage 14-19 Git blobs and
  the committed `work/experiments/language-lx/envelope-facts/` extraction. All
  497 records, 15,210 source-object fact bundles and 11,467 leaf slots are
  accounted for. No leaf is excluded. The matrix also inventories records with
  zero leaf slots: in particular, the envelope label extractor does not create
  leaves for the stored `macro-certificate/1` records. Their certificates remain
  checking obligations, not omissions from coverage.
- **CX.2 [D]** A leaf here is the experiment's **incomplete slot**, identified by
  source file and exact JSON pointer to its fact bundle. It is not an admitted
  EVD leaf or a unique typed mathematical statement. One bundle can describe
  several claims. `groups[].leaves` contains every pointer exactly once;
  `anchors` additionally names whole-record and specifically split obligations.
  A group inherits the assertion, producer leads, checker plan and limits from
  its `families` row. This is grouping for work allocation, not an AND/OR graph,
  a format-dispatch rule or permission to merge statements.
- **CX.3 [D]** The acceptance command reuses `corpus.scan` and
  `envelopes.build_one`; it does not copy their large tables or run their display
  enumeration. It verifies every regenerated envelope against `RECORDS.json`,
  including its original source and edition identity. The research tree at
  `HEAD` must match the experiment's manifest. The label extractor/table are
  read at the Git blob identities recorded by that manifest. Current chapters,
  cards, decisions and briefing text are named below but never pinned as moving
  acceptance inputs. The historical Git objects must remain available locally.
- **CX.4 [D]** `files` is the shared digest catalogue. Every entry has a
  repository-relative path, byte count and SHA-256 that resolves to actual file
  bytes. Research and experiment inputs use `git_blob_bytes`, independent of
  checkout line-ending conversion; the two new Python sources use their exact
  LF `worktree_bytes`. All integer file references index this catalogue. Model,
  exact arithmetic, tree and menu convention files are explicit inputs. They
  are source references for a future implementation, not blanket permission to
  import research construction code into KN6.
- **CX.5 [D]** `dependencies` records every file-named hash and explicit file
  reference found by the declared JSON walk. Each edge names its source file
  and JSON pointer, presence, resolution, and target when available. Record
  rows carry direct and transitive edge indices; traversal follows available
  recorded JSON targets and terminates on cycles. Source-only coupling witnesses
  are separately pinned in `source_walk`. They demonstrate disqualifying shared
  construction paths, not an assertion that every Python import was resolved.
- **CX.6 [D]** An expected historical hash is retained at `digest_at` in its
  pinned source, rather than copied into the catalogue as a fictitious resolved
  file hash. `exact_stored` and `exact_decoded` resolve that value at the stated
  boundary. `public_correspondence` follows `IMPORT-MANIFEST.json`'s explicit
  original-to-public association and checks both public byte digests; it marks
  the original bytes absent. A current file with another hash is
  `dependency_mismatch`, never equality by filename. A path without a recorded
  hash may select a newly pinned current input, but cannot prove historical
  byte identity. Missing material is not fetched or reconstructed by this tool.
- **CX.7 [D]** Bare state, support-array, content and source hashes are not
  automatically file dependencies. Every occurrence counted by the envelope
  extractor is retained in the immutable source and counted on its record as a
  declared file reference, exact byte match, public-source correspondence, or
  untyped/unresolved value. A byte match alone does not create an evidence edge
  or an independence claim. Prose, numeric assertions and non-label fields
  remain within the whole-record anchors; this inventory does not claim to
  formalize every English sentence or infer unrecorded dependency semantics.
- **CX.8 [D]** Each group's `required_files` names the immutable inputs selected
  for its **new** checking plan. `missing_inputs` names a required certificate,
  input or binding and keeps `present:false` and `digest:null` when bytes have
  never been supplied. The named source plus pointer/anchor identifies the
  exact pending obligation. No checksum is invented for an absent checker or
  future certificate. Recorded historical dependencies remain on the source
  record even where a new checker can recompute a cache from full tables.
- **CX.9 [D]** Source-table bindings use the explicit source catalogue, matching
  declared state digests, certificate/check pairs and stored chronological
  words. A derived start requires the complete table and word or jam chain and
  macros. The eventual checker must recompute and compare the actual state;
  this inventory only establishes that its named input bytes are available.
  It neither validates pose digests nor trusts a producer's success flags.
- **CX.10 [D]** Data readiness and actual fresh-check availability are separate:

| Field/value | Meaning |
| --- | --- |
| `data_status:self_contained` | The scoped new check has supplied data, model/conventions and explicit reconstruction inputs; no historical producer is needed to obtain these inputs. It is not a claim that a checker exists or that the result passes. |
| `data_status:needs_input` | The row names missing data, a complete finite coverage certificate, or an exact binding needed for its full stated family. Some narrower positive checks may already be possible from the same record. |
| `data_status:record_only` | This anchor is an input/publication record or an unresolved global obligation; it supplies no mathematical certificate of the requested kind. |
| `fresh_status:self_contained` | A qualified, mutation-gated independent checker and all typed inputs exist now. **There are zero such rows.** |
| `fresh_status:needs_input` | A new independent check is planned, pending the named checker, typed claim binding and any listed data. |
| `fresh_status:record_only` | No supported mathematical check is available for this metadata/global anchor; retain it with `unsupported_claim`. |
| `historical_reproduction:record_only` | The exact historical procedure/edition has unavailable dependencies, including unavailable original bytes or producer material. A different new checker must have its own identity; it cannot be presented as a rerun of that procedure. |
| `historical_reproduction:not_qualified` | No declared historical dependency was found absent, but neither KN6 independence nor a fresh run has been established. |

- **CX.11 [D]** Every mathematical group currently yields library preflight
  `unavailable`, with `no_independent_checker` and `claim_unmapped` (the EVD
  statement/scope/covered binding is still untyped), plus its data or historical
  gate blockers. `historical_outcome` separately includes missing or mismatching
  historical dependencies. These are LB5.14 reason atoms, not new error codes
  and not KN6 `CheckReport` verdicts. Unknown format remains absent. Readiness
  would not imply a pass even after these blockers are removed.
- **CX.12 [D]** All planned checkers require separation from producer/search
  code and the KN6.5 wrong-input/per-claim mutation gate. The source walk pins
  `macro.py -> t3p.py -> blockmap.py -> restore.py`, the literal replay path into
  the same construction code, and further stage 16/18/runner witnesses.
  Historical tool paths are leads, not reconstructed authorship. Stage 19 has
  no archived executable producer/checker. Original labels, scope, attribution
  and reused/audit/replay modes stay in their exact sources. No new evidence
  leaf, permission, completed negative or refutation is produced.
- **CX.13 [D]** The plan preserves distinctions within a record. T3P chain
  `/stop` fields have finite-coverage rows separate from the chain witnesses.
  The trimmed stage-19 q11 lower bound has its own row, separate from frontier
  replay and finite rank arithmetic. Stage-18 expansion, composed strict-prime
  and complete literal replay have separate rows. T6 steps 1-3 retain their
  recorded replay; T6 step 4 and all three rule stages retain
  `historical_check_incomplete`. No complete span chain is inferred from a
  composed result or a partial set of chunks.

## Counts

- **CX.14 [D]** Counts below are mechanically compared with the stored matrix.
  They count planning groups, not independent arguments or newly proved claims.
  A record can appear in several families; the unique record total remains
  497. The additional `19/global_open` row is the stage README's existing
  remaining-obligations section and is outside that record total. The complete
  pointer inventory stays below 2 MB, so no leaf table is omitted or summarized
  by a non-resolvable digest. There are zero out-of-scope leaves, zero admitted
  support leaves and zero ready fresh checks.

  There are 37 stage/family rows and 510 groups: 291 with self-contained data,
  213 needing data/bindings, and 6 record-only metadata/global anchors. Fresh
  availability is 504 `needs_input` and 6 `record_only`, never ready. The 681
  declared reference occurrences resolve as 171 exact stored-byte matches,
  53 exact decoded-byte matches, 162 public/source correspondences, 82 selected
  current paths without historical file hashes, 211 missing references and 2
  mismatching editions. These are occurrences, not counts of distinct files.
  The 697 file pins all resolve. The stored matrix is 1,403,436 bytes.

<!-- family-counts:start -->
| Stage/family | Records | Groups | Leaves | Data self-contained | Data needs input | Record only |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 14/finite_search | 28 | 28 | 2798 | 0 | 28 | 0 |
| 14/input_catalogue | 2 | 2 | 0 | 0 | 0 | 2 |
| 14/literal | 9 | 9 | 49 | 9 | 0 | 0 |
| 14/relations | 19 | 19 | 118 | 19 | 0 | 0 |
| 14/t3p | 16 | 16 | 8 | 16 | 0 | 0 |
| 15/finite_search | 2 | 2 | 1382 | 0 | 2 | 0 |
| 15/macro | 55 | 55 | 0 | 55 | 0 | 0 |
| 15/relations | 3 | 3 | 3 | 2 | 1 | 0 |
| 15/selectors | 27 | 27 | 821 | 0 | 27 | 0 |
| 16/input_catalogue | 1 | 1 | 0 | 0 | 0 | 1 |
| 16/literal | 4 | 4 | 0 | 4 | 0 | 0 |
| 16/macro | 48 | 48 | 0 | 48 | 0 | 0 |
| 16/reductions | 41 | 41 | 91 | 0 | 41 | 0 |
| 16/relations | 6 | 6 | 0 | 6 | 0 | 0 |
| 16/selectors | 9 | 9 | 32 | 0 | 9 | 0 |
| 17/macro | 20 | 20 | 0 | 20 | 0 | 0 |
| 17/rr | 36 | 36 | 62 | 0 | 36 | 0 |
| 18/composed | 2 | 2 | 0 | 0 | 2 | 0 |
| 18/expansion | 35 | 35 | 0 | 35 | 0 | 0 |
| 18/lattice_run | 58 | 58 | 5 | 58 | 0 | 0 |
| 18/static_tail | 5 | 5 | 0 | 0 | 5 | 0 |
| 18/tail_replay | 2 | 2 | 0 | 0 | 2 | 0 |
| 19/admission | 2 | 2 | 0 | 0 | 2 | 0 |
| 19/controlled_bound | 2 | 2 | 2 | 0 | 2 | 0 |
| 19/finite_extra | 20 | 20 | 5193 | 0 | 20 | 0 |
| 19/frontier | 10 | 10 | 5 | 10 | 0 | 0 |
| 19/global_open | 1 | 1 | 0 | 0 | 0 | 1 |
| 19/high | 4 | 4 | 2 | 0 | 4 | 0 |
| 19/indexed_reviews | 2 | 2 | 6 | 0 | 2 | 0 |
| 19/input_catalogue | 2 | 2 | 0 | 0 | 0 | 2 |
| 19/lattice_bound | 4 | 4 | 36 | 0 | 4 | 0 |
| 19/matching | 5 | 5 | 0 | 5 | 0 | 0 |
| 19/profiles | 19 | 19 | 2 | 0 | 19 | 0 |
| 19/pump | 3 | 3 | 1 | 0 | 3 | 0 |
| 19/q11 | 1 | 1 | 1 | 0 | 1 | 0 |
| 19/ranks | 4 | 4 | 0 | 4 | 0 | 0 |
| 19/shortening | 3 | 3 | 850 | 0 | 3 | 0 |
<!-- family-counts:end -->

## Concrete availability cases

- **CX.15 [D]** The matching family has the full synthetic input/end tables,
  reference table, finite bijection/gauge certificate and explicit nine-letter
  forward/inverse witness. Its new check's data are self-contained. Its fresh
  preflight is still unavailable pending a qualified checker and typed binding;
  synthetic reachability remains unknown. `type-lex-trimmed-independent-results.json`
  also contains the q11 lower-bound claim, whose historical declarations include
  absent `type-lex-successor-base-cache.npz`. The q11 row requires that complete
  evidence or a new independent holder/cut certificate. The stored three-letter
  upper witness alone cannot discharge it. Neither case loses its existing
  certificate or historical label on import.
- **CX.16 [D]** Two declared historical proof hashes currently resolve to a
  different available edition: the decoder record's references to
  `GUARDED_OPTIMAL_RETURN.md` and `PREFIX_COMPLETE_OPTIMAL_RETURN.md`.
  They remain `dependency_mismatch` on the historical dependency walk; the
  original declarations are addressed by exact source pointers. Public/source
  associations elsewhere are not converted to exact original-byte matches.
  These are archival availability findings, not mathematical refutations.
- **CX.17 [D]** Fixed-rank arithmetic can be independently derived from the
  supplied type counts and block vectors, without inventing the absent
  `type-height-weight-inputs.json`. It is a new derivation, and it must check
  those counts against the model. Length/selector/invariant-domain premises
  remain. B11 stays a symbolic bound; indexed references are not distinct fibres
  or fresh replays; full family membership does not prove reachability or
  transfer sticker goals. No unavailable result reduces promised coverage.

## Conflicts

- **CX.18 [D]** Historical conflict, corrected in the draft: chapter 06's worked
  example E1 formerly assigned `replay_word.py` the grade
  `(exact computation, independent checker)` and allowed the `macro.py` path
  that grade from a tool-digest fact. EVD.63 now keeps both replay paths
  unqualified, agreeing with this matrix, LB5.10/LB5.15 and KN6.2/KN6.5.
  The pinned source walk shows imports into construction code. Independent
  checker qualification still requires code separation and the mutation gate;
  a source digest or declared common base alone establishes neither.
- **CX.19 [D]** Historical conflict, corrected in the draft: LB5.17 and
  `closure/manifests/p5.json` now acknowledge this folder's per-leaf inventory
  and plan. FR-B08 remains partial: typed statement/scope/input bindings,
  independent checker implementations and their mutation gates remain owed,
  with missing data and unresolved premises retained. The inventory is no
  closure ruling or fresh mathematical check; the resulting evidence still
  needs integrator review.
  No other chapter-03/06 conflict is asserted by this source-only inventory;
  the envelope experiment's display semantics are not rerun or adjudicated.

## Reproduction and remaining work

- **CX.20 [D]** From the repository root, the acceptance command is:

  ```text
  python docs/architecture/1.0/language/claim-matrix/check_matrix.py
  ```

  It regenerates only under this folder's `build/`, compares the exact stored
  matrix and this README's family counts, checks leaf coverage and file digest
  resolution, and runs the embedded negative controls. `build/` is removed on
  success and failure. It never refreshes an artifact, writes bytecode, runs a
  research checker or imports research producer code. The development-only
  command `python docs/architecture/1.0/language/claim-matrix/check_matrix.py --write`
  creates the stored matrix and prints its count table; it is not the acceptance
  command. Sources are `check_matrix.py` and `plan.py`; their digests and the
  reused experiment-input digests are recorded in the matrix. Tools used here:
  CPython 3.15.1 and Git 2.55.0.windows.5; Python standard library only.
- **CX.21 [D]** The negative controls cover a missing leaf, a duplicate leaf,
  a corrupted file digest, the complete matching-data/unavailable-checker split,
  q11's unmet lower-bound inputs, the absent successor cache, public versus
  original bytes, the four unfinished tail gates, refusal to invent a ready
  checker, and the six chain-stop coverage obligations. These are inventory
  checks, not KN6 mutation gates. No mathematical certificate, replay, research
  search, GPU, native, WSL or performance check is run by this packet.
  A deliberate stored-byte alteration was also rejected with
  `FAIL: Stored matrix differs`; the acceptance command removed `build/` on that
  failure, and the original artifact bytes were restored and compared.
- **CX.22 [D]** Missing inventory families: **none in the pinned corpus and
  LB5.15/LB5.16 plan**. Open implementation work is explicit in the rows: EN4
  typed statement/scope/input binding; each named independent checker and its
  mutation gate; the missing data or replacement certificates; and unresolved
  mathematical premises. Generic `needs_input` certificate descriptions are
  obligations on those exact source anchors, not claims that unavailable files
  already exist. This does not make the 497 partial envelopes complete or
  authorize restarting stopped research.
- **CX.23 [D]** After applying and reviewing the candidate, the integrator must
  rerun the acceptance command above. After reconciling all closure drafts,
  run `python -B docs/architecture/1.0/language/closure/check_closure.py --all`
  (**not run here**, outside this packet). No external hardware command is
  required for this source inventory. Fresh KN6 commands do not exist yet and
  are not invented. Integrator review, reconciliation of the conflict and the
  FR-B08 disposition remain open; this folder selects no route, accepts no ADR
  and freezes no architecture.

Named documents read, without moving digest pins: `AGENTS.md`;
`docs/development-guide/AGENT_BRIEFING.md`;
`docs/wiki/decisions/owner-decisions-2026-11-05.md`;
`docs/architecture/1.0/language/closure/findings.json` and `closure/manifests/p5.json`;
architecture chapters 01 (KN6), 03 (LB5 and its tables), 06 (claims/certificates)
and 13 (error vocabulary); `docs/architecture/1.0/language/format-packet/README.md`;
`work/experiments/language-lx/README.md` and `envelope-facts/RESULT.md`, inventory
outputs, `corpus.py`, `envelopes.py` and `check.py`; stage 14-19 README, source,
manifest, run and check records named in the matrix.
