# Language IR line plan: the constructs the IR draft lacks, and the syntax and IR freeze

Status: plan of 26 November 2026, written when the owner started the IR work ("IR的工作可以开始了", 26 November 2026)
and before any of its code. It records the integrator's selections; the owner may overrule any of them. It is no ADR,
freezes nothing and edits no frozen text. The syntax and the IR freeze only when the owner signs the freeze card
(section 7), as one PHS.52 amendment, the way LP3's command table froze
([LP3 plan](language-lp3-plan.md), section 11).

Inputs: LP2's exit record ([LP2 plan](language-lp2-plan.md), section 12, reading 14); the
[LP2 index notes](lp2-ct-index-notes.md); chapter 02 (LG1, LG2, LG4, LG5), chapter 01 (KN0, KN1, KN2.11 to KN2.15),
chapter 08 (ONE.20 to ONE.28, ONE.70), chapter 09 (DET.8 to DET.30, DET.55 to DET.65), chapter 12 (PHS.49, PHS.52,
PHS.62, PHS.63) and chapter 13; `rust/README.md`; the owner's LQ2 answer of 26 November 2026 (base tier BS2, flow rule
on, admitted `own` and `forked_built_in`), whose policy implementation is a separate task: nothing here contradicts it.

Evidence kind for everything this line produces: source (PHS.19). Nothing here is Windows interaction, DirectX, input,
long-session or performance evidence.

## 1. What the line delivers

Reading 14 of LP2's exit (owner, 22 November 2026, item 8) indexed 19 conformance IDs "with the IR's freeze, where the
missing constructs land". The line builds those constructs so that the 19 run whole, or records why a half does not
apply, and then drafts the freeze packet. Each row below was checked against the source on 26 November 2026, not only
against the test markers; where a marker is stale, the row says so.

| ID | What does not run today | Built already (stale marker text) | Construct (section) | Crates |
| --- | --- | --- | --- | --- |
| CT-LG2-01 | Matches over other algebraic types; model, level and mode indices through generics; puzzle-indexed units | Precision, exactness, `Budget` and effects through generics | T1 algebraic types, indices, cost units, the control bit | ir, check, syntax, run |
| CT-LG2-02 | Substitution and unification refusals of identity types | - | T1 identity types | ir, check, syntax |
| CT-LG2-03 | Explicit partial projection, labels refused off the lattice, a class is no equality | The kernel's `project` (M8) | T1, T2 `project`, `compare` | ir, check, syntax, run, jumble-tool |
| CT-LG2-07 | Raw pose data into a state only by validation with a claim; a jumbling witness is no retained reachability | The kernel's `import` and `derive` (M8) | T1, T2 `import`, `derive`, `Reachability<P>` | all of the above |
| CT-LG2-09 | An off-lattice projection is no `unknown`; a goal cannot weaken its level; KN0.5's general verdict | End tests' verdicts (LB3.2) | T1 `Verdict`, `Partial`, goals | ir, check, syntax, run |
| CT-LG2-10 | Goal fixtures over LG2.11's chain; equal labels without equal poses; no retained word from a label checkpoint | The kernel's end tests and `compare` (M8) | T1 goals, `imply`; T2 fixtures | ir, check, syntax, run, jumble-tool |
| CT-LG2-11 | An execution and a derivation binding; a key indexed by puzzle | Proposal and protection bindings | T2 `derive` takes literal letters only; T1 `WordEqualityKey<P>` | ir, check, run |
| CT-LG2-16 | The language half of A6's rotation requests | The kernel half: `c600-kernel`'s `request` and `c600-jumble`'s `rotation` (pull requests 176, 207) | T3 rotation requests | ir, check, syntax, run, runtime, tools |
| CT-LG4-04 | The wall-clock safety stop feeding a record | `c600-runtime`'s deadline (`KN7:resource_limit.safety_stop`), the record's `Ending::SafetyStop`, the session's derived-query cache charged the same cold, warm and evicted (`jobs.rs`) | R1 fault-to-record glue; tags | session, runtime, tools |
| CT-LG5-04 | A model or menu-order check at the run itself; a refused module kept as recorded material | The closure and approval identity checks | M1 the run's identity check | module, tools, runtime |
| CT-LG5-07 | A module signature; the evidence half | Digest, lock, identity and grant checks; FP.10's optional extension | M2 the evidence half, and a reading on signatures (section 4) | evidence, module |
| CT-ONE-21 | Derived values end at a restart; a stored derivation recomputes | Snapshot handles end with the epoch (`EPOCH_CHANGED`); drafts and receipts | T2 value store; R3 restart test | session, run, jumble-tool |
| CT-DET-06 | The run record refuses host entropy and a clock-derived seed | Child stream paths (`child_scopes.rs`), the sealed seed and its digest check (`persist_run.rs`) | R2 seed checks; tags | session, run |
| CT-DET-08 | A value store and its eviction | The derived-query cache cold, warm and evicted | T2 value store | session, run, jumble-tool |
| CT-DET-12 | Other jobs varied around a fixed run | Atomic one-job admission; serial children (DET.60) | R3 scheduling test; reading on child order (section 4) | session |
| CT-DET-13 | An empty completed enumeration read as complete; a host error's record; a runtime fault's record | The four outcomes, `Ending::SafetyStop` | R1 `Ending::Fault` and glue; tests | session, runtime, tools |
| CT-DET-28 | Text order in a program; flow checks | Text equality and decoding priced on scalar lengths | T4 text order; reading on `flow_check` (section 4) | ir, check, syntax, run |
| CT-DET-29 | As CT-DET-06 | As CT-DET-06 | R2; tags | session, run |
| CT-ERR-16 | The typed failed fact of the three KN2 refusals | The parser and the three codes, through `twist.apply` | M3 typed refusal context | kernel, jumble, command |

Each construct's acceptance check is its CT rows running whole under `cargo test --locked --offline` in `rust/`, with the
PHS.62 markers changed from "part" to whole (or to the note of section 4), `tools/language/ct_index.py` agreeing, and
the checks of section 5 passing.

## 2. T1: the identity tower and the domain families (largest risk, built first)

The kernel half exists: `c600-jumble`'s `levels` module (M8) has KN1.7's levels and maps, KN0.4's typed partial
result, KN0.5's verdict, KN1.9's `project`, ONE.25's `compare`, KN2.11's `derive`, KN2.13's five reachability kinds and
KN2.14's `import`; `c600-kernel` holds 600-cell-Full at labels. What is missing is the language half: the types, the
checker's rules, the source forms, the interpreter's values and the host rows that reach those kernel calls. Every name
below is a draft until the freeze.

**Indices.** `Puzzle` is `Full` or `Jumbling` (KN0.1). `Level` is KN1.7's eight levels with its maps, as
`c600-jumble`'s `Level` names them; 600-cell-Full holds `labels` and `position` only (KN1.7's table). A level index
is static wherever the program names it; a level that a puzzle does not hold is `LG2:type.level_mismatch` statically
and `KN1:input.level_not_held` at run time.

**Types** (each exact, with no display qualifier of its own):

| Type | Meaning | Clauses |
| --- | --- | --- |
| `Piece<P>`, `Sticker<P>`, `Slot<P>` | Canonical IDs bound to a puzzle; key-ordered, with a canonical form | KN1.5, LG2.2 |
| `Pose` | The exact key of one piece's rotation (jumbling); exact equality | KN1.5 |
| `PoseTableIndex` | A per-revision index; never an identity, so it fits no slot that takes one | KN1.5, LG2.2, KN2.9 |
| `Digest<P, L>` | A digest that names its level; compares only with a digest of the same puzzle and level | KN1.8 |
| `LevelValue<P, L>` | A state's value at one level, held by the host | KN1.9 |
| `State<P>` | KN0.3's state reference, a snapshot or a derived state, with its reachability claim | KN0.3, LG2.7, KN2.11 |
| `PoseTable` | Raw pose data: data only, never a state | KN2.14, LG2.7 |
| `Reachability<P>` | KN2.13's claim: `witnessed`, `by_theorem`, `promised`, `refuted` or `open`, each with its evidence | KN2.13, LG2.7 |
| `Evidence` | An evidence envelope reference; a program can pass one on but never make one | KN0.5, chapter 06 |
| `Verdict<P, L>` | KN0.5's `yes(Evidence)`, `no(Evidence)` or `unknown(Text)`, carrying its level | KN0.5, KN1.11, LG2.9 |
| `Partial<T>` | KN0.4's `total(T)`, `partial(T, Missing)` or `undefined(Text)` | KN0.4, LG2.3, LG2.9 |
| `Holds<G>` | A certificate that goal G holds of a state, for LG2.10's six goals, each at its puzzle and level | LG2.10, LG2.11 |
| `Cost<P, U>` | A cost in unit U (`generator`, `menu_letter`, `ui_action`) of puzzle P | LG2.6 |
| `WordEqualityKey<P>` | The existing key, now indexed by its word's puzzle | LG2.13 |

**Algebraic types.** LG2.1 asks for algebraic data types, and the draft has records and literal matches only. The
line adds sum types declared in a module (constructors with fixed payload types, LG2.1), the built-in sum types
`Verdict`, `Partial`, `Reachability` and `Derivation`, and a match with constructor patterns whose arms account for
every constructor; the `_` arm stays, and it may not take `unknown` together with another constructor of a verdict,
as it may not for an end test's verdict today (LG2.9). Generic substitution keeps every index (LG2.1), so `id` of a
`Piece<Full>` is a `Piece<Full>`. The function type gains the approximate-control bit that `rust/README.md` lists as
M1's open limit (LG2.22), so a callable known only by its declared type keeps `LG2:type.exactness_violation`.

**Operations** (host rows; the language calls the kernel, never the reverse, KN0.7):

| Operation | Result | Clauses |
| --- | --- | --- |
| `snapshot<P>()` | `State<P>` at the run's revision; the mode is the puzzle's (Jumble on for `Jumbling`), refused at invocation after a mode change | KN2.2, LG2.8 |
| `project(s, L)` | `Partial<LevelValue<P, L>>`; off the lattice `partial` with the off count, never `unknown` | KN1.9, LG2.3, LG2.9 |
| `digest(v)` | `Digest<P, L>` of a level value | KN1.8 |
| `compare(a, b, L)` | `Partial<Verdict<P, L>>` | ONE.25 |
| `derive(s, w)` | `Derivation<P>`: `derived(State<P>)` or `refused(...)` with the first refused prefix | KN2.11, ONE.20 to ONE.22 |
| `import(t, r)` | `State<Jumbling>` from a `PoseTable` with an `open` or `promised` claim | KN2.14, ONE.27 |
| `reachability(s)` | `Reachability<P>` | KN2.13, ONE.28 |
| `check(g, s)` | `Verdict<P, L>` for goal g at its own level, with `Holds<g>` as the evidence of `yes` | LG2.10 |
| `imply(h, G)` | `Holds<G>` along LG2.11's chain only: exact pose ⇒ sticker test ⇒ lattice placement ⇒ low height (nonnegative bound); retained checkpoint ⇒ label checkpoint; sticker test ⇒ label checkpoint with a reachability premise; label checkpoint ⇒ lattice placement | LG2.11 |

**Checker rules and codes** (all registered in chapter 13): another puzzle in an argument or a comparison is
`LG2:type.identity_mismatch` (LG2.2); a wrong level, an implicit projection, a lift, a `Partial` used as a total value
and a goal at a weaker level are `LG2:type.level_mismatch` (LG2.3, LG2.10); a `PoseTable` or record where a `State` is
expected is `LG2:type.reachability_missing` (LG2.7); a verdict as a boolean, or a match that lets `unknown` read as
`no`, is `LG2:type.verdict_required` (LG2.9); mixed cost indices are `LG2:type.cost_unit_mix` (LG2.6); a wrong
statically known mode is `LG2:type.mode_mismatch` (LG2.8); a word equality key where `derive` takes a word is
`LG2:type.word_key_not_executable` (LG2.13, CT-LG2-11). `imply` has no edge from a label checkpoint to a retained
checkpoint and no operation gives a `Word` from a label checkpoint (CT-LG2-10); no operation lifts (KN1.10).

**Values and hosts.** A state, level value, pose table and evidence are host handles in the interpreter, charged at the
logical size their row declares (KN2.12, DET.24); a derived state lives in a value store that is not durable (ONE.70),
keyed by its derivation (ONE.21), whose eviction and recomputation change no result and no charge (KN2.15, DET.16).
The jumbling rows are served by `c600-jumble-tool`'s bridge over M8's `levels` module, the Full rows by `c600-tools`'
bridge over `c600-kernel`. H-08's `reads` (CM.18) froze at LP3 without `project`, `compare`, `derive`, `import` and
`reachability`: they are built as drafts and proposed on the freeze card as an H-08 amendment (section 7).

**Source forms.** Candidate A's grammar stands; the new types and operations are written as the draft's other types
and reserved calls are (`State<Jumbling>`, `project(s, labels)`), and a sum type is declared `type T { A(Int); B; }`
at column 1. The lowering refusals that name "M4, M5" go.

## 3. The other constructs

- **T2, the derived-state row and the value store** (CT-LG2-07, CT-LG2-11, CT-DET-08, CT-ONE-21). `derive` takes a
  `State<P>` and a literal `Word` (Full) or `MenuWord` (jumbling), never a key; `import` takes a `PoseTable` and an
  `open` or `promised` claim. The host keeps derived values in a value store that is not durable, keyed by ONE.21's
  derivation (base by `RevisionRef` or digest, and `Digest<literal word>`), charges each at its logical size on a hit
  and a miss alike, may evict and recompute with no change of result or charge, and ends every value at a restart
  while the stored derivation still recomputes.
- **T3, rotation requests** (CT-LG2-16). `RotationRequest` is an opaque exact type like `Receipt`; a literal
  `rotation("LX9B1|...")` holds the request's bytes, which the host validates (`KN2:input.rotation_request_malformed`);
  no operation makes one from `Approx`, `Text`, `Bytes` or a number, through a helper or a generic either; `resolve(r)`
  is the host row that calls KN3.8's kernel resolution under the run's pinned menu and gives a one-letter `MenuWord`
  whose exact result is recorded for replay (LG2.4, KN3.9). A program still cannot commit (LG3.4).
- **T4, text order** (CT-DET-28). `exact_less` takes two `Text` values and orders them by scalar value (DET.62),
  priced at one step plus a tariff on both scalar lengths; DET.65's `text-comparison` trace runs as a program.
- **R1, faults into records** (CT-LG4-04, CT-DET-13). The record gains an ending for a runtime fault that leaves no
  summary; glue turns `c600-runtime`'s deadline fault into `Ending::SafetyStop` and a lost runtime into `error`; tests
  for an empty completed enumeration that reads complete and a host refusal that reads `error` with no negative
  claim (DET.17, DET.30).
- **R2, seed checks** (CT-DET-06, CT-DET-29). The record pins the generator's algorithm and a 32-byte seed, and
  refuses a key whose randomness grant has no sealed seed, so no seed comes from host entropy or a clock (DET.11,
  DET.12, DET.61).
- **R3, scheduling and restart tests** (CT-DET-12, CT-ONE-21). A fixed run keeps its payload and coverage while other
  jobs are cut off, cancelled or run beside it; derived values end at a restart (with T2).
- **M1, the run's identity check** (CT-LG5-04). A run names the session's identity; a loaded entry is refused with
  `KN1:input.identity_mismatch` (LG5.5, KN1.4) when its approval's model, menu or menu order differs from the
  session's or from the run's pinned menu, so its literal menu words are never read in another order. LG5.5 offers
  "refused ... or retained as recorded material": the line refuses; recording stays an option for the owner.
- **M2, the evidence half of CT-LG5-07**. In `c600-evidence`, a module whose manifest carries an unknown optional
  extension (FP.10), such as one a signing tool might add, creates no evidence and no grant, and its keys read open.
- **M3, the typed failed fact** (CT-ERR-16, ERR.8). `c600-kernel`'s `Refusal` gains a typed context (the malformed
  field; which menu digest differs, with the expected and held values; the search bounds against the limit), carried
  through `c600-command`'s error and the receipt; a test asserts code and fact, that nothing changed, and that a
  corrected request then succeeds (`after_user_change`).

## 4. Rows that close by a recorded decision, not by code

These halves get an index note (`not applicable` or `indexed elsewhere`) that cites the record; none of them counts as
a pass (PHS.63). The first three are recorded decisions; the last three are readings of frozen text that the freeze
card puts to the owner.

| Half | IDs | Record | Note |
| --- | --- | --- | --- |
| RT2 and RT3 | CT-DET-06, CT-DET-28, CT-DET-29 | Reading 1 (owner, 22 November 2026, item 8): not applicable while RT1 is the only admitted route (LQ9) | As CT-DET-10 and CT-DET-26 have it |
| A row-answer cache's warm, evicted, shared and other states | CT-LG4-04, CT-DET-08 (the host's row cache only) | 1.0's host keeps no row-answer cache (owner, 20 November 2026, item 1; B12) | The session's derived-query cache (KN7.8, `c600-session`'s `jobs.rs`) and the new value store run whole; only a row cache would add states |
| Form 2's halves | none of the 19 (CT-LG2-05, CT-DET-32) | Reading 8: form 2 stays out until it is built | Proposed "not in 1.0" on the freeze card (section 7) |
| The run-time `flow_check` operation | CT-DET-28 | DET.55 ties it to the flow-label alternative, which LQ21 did not choose (owner, 14 November 2026; RTE.24); LQ2's flow rule of 26 November 2026 is capability separation's static rule (BND.41), not this operation | Not applicable while capability separation stands; a reading for the card |
| Child completion order | CT-DET-12 | DET.60 admits no speculative or concurrent child work, so a child's completion order is its invocation order | Not applicable while DET.60 stands; a reading for the card |
| A module signature | CT-LG5-07 | No frozen text defines a signature format; LG5.9 shares modules only as local files with digests, with no key distribution; a real signature would need a format change (FP.4, PHS.52) and a cryptographic dependency (RTE.70) | The row's claim is negative: with M2, nothing a manifest carries supplies evidence or a capability. A reading for the card: 1.0 has no module signature |
| A refused module kept as recorded material | CT-LG5-04 | LG5.5 and KN1.4 say "refused or recorded" | M1 refuses, which meets the row; a reading for the card |
| The headless Linux leg | CT-DET-06, CT-DET-29 | The owner's answer of 16 November 2026: CI's Linux Rust job runs them (LP2 exit record, "What stays open") | Stale marker text; corrected, no note |

## 5. Order of work, commits and checks

Three streams, so that no two writers touch the same files:

| Stream | Pieces, in order | Where |
| --- | --- | --- |
| Tower | T1 (IR and checker, then source forms, then the interpreter), T2, T3, T4 | This line's worktree, commit by commit |
| Records | R1, R2, R3's scheduling test | A subagent in a worktree of its own; its commits are applied here |
| Modules and refusals | M1, M2, M3 | A subagent in a worktree of its own; its commits are applied here |

R3's restart test follows T2. Then the line updates the markers, [the index notes](lp2-ct-index-notes.md) and
`rust/README.md`, and drafts the freeze packet (section 7). Everything lands in one pull request with its commits kept
apart, queued for the top integrator.

Checks for every commit that touches code: `cargo test --locked --offline` in `rust/` with the line's own target
directory (doctests that stop with E0463 rerun with `--doc`), `cargo fmt --check -p` per changed crate,
`python tests/test_language_scans.py`, `python tools/language/ct_index.py --summary`; for documents,
`python tools/wiki/lint.py` and `python tests/test_wiki_lint.py`. A change to the kernel, model or protection logic
(`c600-kernel`, `c600-jumble`) gets one read of its diff by a fresh Claude subagent on a smaller model, with the
acceptance check, before it is committed. No Codex call.

The owner's quiet-machine hold of 26 to 27 November 2026 stops every build and test from 20:45 UTC until the top
integrator says the measured chain has ended; work committed and pushed before 20:45 waits.

Size: the tower is the large part, about 8 to 12 agent days (T1 about 5, T2 about 3, T3 about 1.5, T4 about 0.5); the
records and modules streams about 2 to 3 days each, in parallel; the markers, notes and freeze packet about 1.

## 6. Stop conditions

- A construct that cannot follow a frozen clause stops: the line sends the clause, the conflict and the options to the
  top integrator and builds nothing around it.
- No file under `docs/architecture/1.0/language/` changes, the format packet included; a needed change becomes an
  item of the freeze card.
- No `-draft` identifier is renamed before the owner signs (section 7).
- A product dependency, an "Ask the owner" item or a change to H-08 beyond the freeze card's proposal stops for the
  owner, through the top integrator.

## 7. The freeze card (drafted at the end of the line)

The line drafts the packet; the top integrator puts the card to the owner. The owner signs on one card, as for LP3's
command table: one PHS.52 amendment that freezes

- the syntax (`6cs-draft-1`, candidate A as built) and the IR (`c600-ir-draft-1`, with the constructs above);
- the renames at the freeze, after the signature and not before: `c600-ir-draft-1`, `6cs-draft-1`,
  `lp1-exec-draft-1`, `lp1-exec-region-draft-1` and `sha256-counter-draft-1` (`c600-log-draft` is not tied to it);
- the settle list: `trace-key/1`, the word-key contract (PHS.55), the draft precedence of a base type before
  exactness, the frame layout, the stop locator, `DraftTariffs`, the LEB128 stand-in, the draft codes, and DET.65's
  three amendment candidates (`generator-nested-same-name`, `text-comparison`, `abstract-trace-charges`);
- H-08's `reads` amendment for the new host rows (`project`, `compare`, `derive`, `import`, `reachability`,
  `resolve`), each with its puzzle, level, argument, answer, tariff and codes (CM.18's form);
- the readings of section 4 that are not yet recorded.

The packet also states the CT index on the frozen candidate, what stays open (LQ2's policy implementation, form 2,
the post-freeze on-screen line's owner-machine halves), and form 2's proposed disposition: not in 1.0. Form 2
(LG2.21, `lp1-exec-region-draft-1`) is not built by this line; building it later would add `ApproxBool` regions,
region entries and exits in the IR, the checker's region and exit rules, a source form, an executor with DET.64's
charges and DET.65's five traces (about 5 to 10 agent days), and would run CT-LG2-05's form-2 half and CT-DET-32.
