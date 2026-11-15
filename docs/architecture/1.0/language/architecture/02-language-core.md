# Chapter 02: the language core

Status: **frozen with the LP1 candidate** (owner sign-off, 15 November 2026;
[decision](../../../../wiki/decisions/owner-decisions-2026-11-15-freeze.md)). Conventions are in
[the overview](00-overview.md), section 3, and its status says what the freeze covers.

The language core supplies the checker and the execution contract used by libraries, commands and views. It has no
mechanics or mutable puzzle state. The interfaces it calls belong to [chapter 01](01-kernel.md). This chapter fixes
requirements on any syntax. Syntax, implementation route and flow-control form wait for their evidence under the
owner's answers of 6 November 2026; section 8 lists what remains open.

Sources: [the partition](../ecosystem-partition.md), especially sections 4, 4.1, 8.3, 9, 11, 12 and 16;
[DT-L1](../../../../progress/1.0/design-table/topics/language-ecosystem.md), especially E1 to E3, M1 to M4, R2-E2 to
R2-E4 and the Fix first items; [state contract A1 to A5](../../../../../research/jumbling/state-contract.md);
[requirements R-01 to R-19](../../../../progress/1.0/requirements-from-screening.md);
[command table H-08](../../../../progress/1.0/command-table.md) and
[DT-M1](../../../../progress/1.0/design-table/topics/command-keyboard-model.md). References to kernel clauses use
chapter 01's vocabulary. Types below name concepts, not final identifiers or concrete syntax.

## 1. LG1: syntax and editions

- **LG1.1 [D]** Each source file has an edition header (partition LG1, PS1). The header selects the rules for parsing
  and source-level meaning: name resolution, typing, effects and lowering to the supported IR contract. A dependency
  does not change the importing file's edition. The header's spelling stays with LQ13.
- **LG1.2 [D]** Editions preserve saved meaning (partition PS2). An unsupported edition is refused before execution;
  the implementation never retries the file under its current default edition. An edition and an IR version are
  separate identifiers: the former fixes source meaning, the latter the execution contract (LG4.1). A migration is an
  explicit, recorded operation owned by chapter 05.

  ```text
  operation parse(source: SourceBytes, edition: Edition) -> ParsedProgram | Diagnostics
    effect:  pure
    errors:  LG1:syntax.edition_missing, LG1:syntax.edition_unsupported, LG1:syntax.invalid_encoding,
             LG1:syntax.invalid_literal, LG1:syntax.unexpected_token
  ```

- **LG1.3 [B]** Source is UTF-8, with line endings pinned to LF before any source digest is taken (partition PS1;
  R-03). A decoder refuses invalid UTF-8 rather than replacing bytes and then hashing the replacement. Such a failure
  uses ERR.10's encoding-error diagnostic with no `span` and with raw-byte context; its raw input digest is not a
  validated source digest. Chapter 05 owns the stored byte rules and digest inputs. Formatting is a source edit;
  a digest of formatted text cannot stand for different source bytes that were executed.
- **LG1.4 [D]** The formatter's output depends only on the parsed program, including its edition and any comments the
  parser retains (partition LG1). It has no dependency on the session, clock, file path, machine locale or an ambient
  style preference. Formatting the output again gives the same bytes, and parsing those bytes preserves the program's
  meaning. This does not select whitespace or comment syntax.
- **LG1.5 [D]** Recovery parsing preserves locations and finds independent declarations after a local syntax error, so
  EV1 can offer diagnostics, completion and navigation in a broken draft (partition EV1). Recovery holes are diagnostic
  values, never executable expressions. The error code remains stable when the diagnostic's explanatory text changes
  (default 5). Recovery does not install a broken draft over the last valid module (LG5.6).
- **LG1.6 [D]** Any syntax provides exact literals for unbounded integers, rationals and Q(√5) values (partition KN3,
  LG2; DT-L1 M4). Their parsed meanings are respectively an integer, a ratio of integers with a nonzero denominator,
  and a pair of rational coefficients of 1 and √5. Parsing and lowering never pass these through a host float. The
  exact arithmetic operations and their costs are KN3's.
- **LG1.7 [B]** An approximate literal has a distinct parsed kind and can never be read as exact (INV-4). It carries a
  declared precision through lowering. Its spelling must distinguish it from every exact literal in the same edition.
  Approximate literals do not provide a second route to an exact integer, rational, rotation or decision (LG2.4,
  LG2.5).
- **LG1.8 [P LQ13]** A syntax candidate enters LX-8 with a parser, recovery diagnostics
  and a lowering description for the same typed IR. The candidate demonstrates the domain families of LG2.12,
  effect-bearing functions, explicit approximation, verdict matching, literal and compressed words, modules, commands
  and views. Shared inputs lower to operations with the same types, effects, literal order and declared charges.
  Source locations may differ. LX-8's human protocol belongs to the owner. The syntax choice waits for LX-8 before
  LP2 builds the parser (owner, 6 November 2026, LQ13). No syntax candidate is chosen here.

## 2. LG2: types

### General types and inference

- **LG2.1 [D]** The language has algebraic data types, generics and inference (partition LG2). A constructor fixes its
  payload types, and matching a typed choice accounts for every alternative. Generic substitution and inference
  preserve model, level, mode, precision, cost unit and effect requirements. Inference cannot resolve an ambiguity by
  discarding an index, assuming a verdict is false or granting a capability. No particular inference algorithm or
  concrete declaration syntax is selected.

### The six disciplines

- **LG2.2 [B]** An identity type binds an object to its model and level (INV-6; KN1.5, KN1.7). A piece of 600-cell-Full
  and a piece of the jumbling puzzle do not unify, even if their stored indices agree. A pose and its sticker class do
  not unify either. The checker refuses a cross-model comparison or call argument and refuses a pose-table
  index or display instance used as a canonical identity.
- **LG2.3 [D]** A change of level uses KN1.9's explicit projection and handles KN0.4's partial result (partition KN1;
  DT-L1 M3, R2-E2). The checker refuses an implicit projection, a fabricated value for a missing projection or a lift
  from a coarser level. Protection verdicts keep their level (KN1.11). Labels are undefined off the lattice, so a rule
  that reads them there refuses; a class is never equality (KN1.17).
- **LG2.4 [B]** Exact and approximate values are separate types (INV-4). Exact-to-approximate conversion is explicit
  and names a precision. There is no language conversion back, including through an ordinary boolean, a cast, a
  generic container or a helper function. Approximate values cannot decide legality, admissibility, protection,
  equality of exact objects, goals, digests or evidence. [D] Under proposed amendment A6, a rotation request reaches a
  program only as an opaque exact value from the host, and programs write exact literals. Menu-mode drag resolution
  is a kernel call (KN3.4, KN3.8), with its exact result recorded for replay; it is never a language conversion.
  The owner chose the exact-request direction (owner, 6 November 2026, decision 8); A1 stays in force until EN1's
  Astra check of A6 and then the owner. The A2 filter remains internal to KN3.5 with its proven bound.
- **LG2.5 [D]** LP1 recommends form 1 of [the working handoff](../closure/HANDOFF.md), H6: an approximate comparison
  returns `ApproxBool`, never `Bool`. Only `display_if` branches on it, in a display expression with pure branches and
  a display result. LG2.19 to LG2.22 give the checking rules. This is an integrator proposal, not an owner decision or
  a syntax choice. Form 2, a checked display region, remains specified in LG2.21. Under both forms a float comparison
  wrapped in an ordinary-boolean helper is refused. These exactness rules apply independently of the flow mechanism
  (capability separation, LQ21; RTE.24); a provenance label or capability grant cannot waive them. Display output
  meets chapter 10 at `publish_view`.
- **LG2.6 [D]** Costs are exact nonnegative integers indexed by puzzle and unit (partition LG2; default 6; DT-L1
  R2-E4). Menu letters, retained generators and UI actions are distinct units. Addition, ordering and a budget argument
  require matching puzzle and unit; the checker refuses adding a menu-letter count to a generator count, or comparing
  counts from different puzzles as one cost. A recipe can report several separately named costs from its expansion.
  Seconds are a display unit only. Cost units travel into chapter 09's reproducibility key and chapter 05's run record.
- **LG2.7 [D]** Every state-typed value has the reachability claim of KN2.13 in its named puzzle (partition LG2;
  DT-L1 M3). The claim is witnessed, by theorem without a witness, promised, refuted or open, with the evidence
  reference chapter 06 defines. Form validation does not establish reachability (KN2.14). The checker refuses a
  state without a claim and refuses to substitute witnessed jumbling reachability for retained reachability. A raw
  pose table remains data until it enters through the kernel's derived-state validation.
- **LG2.8 [D]** Session operations use the mode carried by KN0.2's revision reference (partition LG2). Retained-workflow
  tools require Jumble off; applying or proposing a menu word to the session requires Jumble on. The checker refuses
  the wrong mode and requires a check of a dynamically obtained mode before such a call. A stored word does not change
  a session's mode. Mode changes remain command-layer operations, and a previously checked mode does not waive a
  revision or mode recheck at run time.
- **LG2.9 [B]** A decision that may lack evidence uses KN0.5's verdict: yes with evidence, no with evidence, or unknown
  with a reason (INV-5). The checker refuses a verdict in a boolean condition or a match that silently treats unknown
  as no. An off-lattice projection is a typed partial result, distinct from unknown (KN0.4). Neither is an empty set
  or an error disguised as false. Exact booleans remain available for predicates whose definitions are total and
  decided exactly.
- **LG2.10 [D]** A goal is a predicate indexed by puzzle and the level it reads (default 9; KN1.11; DT-L1 M3).
  Application checks those indices and any projection's domain. A goal cannot be replaced by a goal at a weaker level
  because the values happen to look the same. The following goal names remain separate; chapter 03 owns their library
  entries and chapter 06 their claims.

| Goal | Input level and meaning | Source |
| --- | --- | --- |
| Exact pose | Jumbling poses; every labelled piece has its solved pose. It is a contract-equality predicate, not the end of a jumbled solve. | State contract section 2, A3, A5; KN1.6 |
| Sticker test | Labels; every slot has its home label. In the jumbling puzzle it also requires the lattice test before π is available. | State contract A5; KN2.4 |
| Lattice placement | Jumbling poses; the exact lattice test. It is equivalent to height zero only on Γ(Λ) (T3): in free mode ω = (2,0,0) at cap 0 has heights (0,0) and lies off the lattice | State contract section 2, A5; DT-L1 M3, R3-M2 |
| Low height | Jumbling poses; the declared exact height bound of the goal, with the height function named | DT-L1 M3; chapter 03 |
| Retained checkpoint | Jumbling poses on the lattice, witnessed equal to a state reached from solved by a legal retained-generator word | State contract section 5, as amended by A5 |
| Label checkpoint | Lattice-checked jumbling poses and their label projection, with reachability and ν = 0 under the pinned convention; it certifies retained reachability of the labels without producing a retained word | State contract A5; KN2.13 |

- **LG2.11 [D]** The implications available without another lemma are exact pose ⇒ sticker test ⇒ lattice placement
  ⇒ low height for a nonnegative bound (DT-L1 M3). State contract A5 also gives retained checkpoint ⇒ label checkpoint
  and sticker test ⇒ label checkpoint within its reachable-state scope. All use the same puzzle, scope and named goal
  parameters. Raw pose data with open reachability does not discharge that premise. A label checkpoint satisfies
  lattice placement by its definition. These implications provide no inverse, no pose lift and no retained witness
  word. A frontier exit is not a restoration goal (partition 10.3). A further implication needs its own evidence and
  subject-preserving map (KN1.13).

### Domain families and words

- **LG2.12 [D]** The type surface covers the families of partition 4.1. The table routes definitions to their owning
  chapter; it does not supply alternate representations of kernel or evidence values. The user-action token is listed
  for coverage of the operation family; the runtime cannot construct or obtain one (KN5.15).

| Family | Types | Definition or interface |
| --- | --- | --- |
| Model and identity | Model, cell, grip, cap, generator, menu, menu letter, piece, sticker, slot, position, pose, sticker class, region class, orbit, identity levels | KN0.1; KN1.1 to KN1.11; LG2.2, LG2.3 |
| State | Snapshot, derived state, difference | KN0.3; KN2.2, KN2.11 to KN2.14; chapter 08 |
| Words | Literal words of signed retained generators; literal menu words; equality keys; recipes with word and star steps; compressed words; exact expansion length and costs | KN2.7 to KN2.10; LG2.13 to LG2.18; chapter 03 |
| Operations | Proposal, preview, user-action token, receipt, admissible/blocked/invalid/uncertain outcome, typed error | KN2.8; KN5.9 to KN5.24; chapters 08, 13 |
| Claims | Claim, evidence, certificate, premise, scope, domain, level-typed goal, reachability claim | KN0.5; KN2.13; LG2.7, LG2.9 to LG2.11; chapter 06 |
| Representations | Chart, projection, section, layout, atlas, action, embedding, relation, graph, tree, dimension-tagged cell complex, state graph | Chapters 03, 10 |
| Interface | View, panel, control, overlay, display plan, display instance, source map, command, binding, recording | KN5.3; chapters 04, 10 |
| Research | Search space, search record, strategy, rank, budget, run record, branch | KN4.6, KN7.7; chapters 03, 05, 08, 09 |

- **LG2.13 [D]** A word equality key has a type of its own, indexed by the word's puzzle and identity (default 10;
  DT-L1 R2-E2). It is never a subtype of a literal word. No replay, derivation, protection, proposal or commit interface
  accepts it in place of literal letters, and there is no implicit conversion to a literal word. Key equality says
  nothing about equal Strict-protection results or equal peak height. The key is sound, not complete: equal keys give
  equal partial maps, and unequal keys say nothing, because letters at grips that do not interact commute.

  ```text
  operation word_key(word: LiteralWord) -> WordEqualityKey
    effect:  pure
    cost:    deterministic word-processing cost, charged under LG4.7
    errors:  KN7:budget.*, KN1:input.identity_mismatch
  ```

- **LG2.14 [D]** For 600-cell-Full the key uses free reduction. For jumbling it preserves the word's domain as a
  partial map on states (default 10; DT-L1 M4, R2-E2). A same-cap inverse pair at a straddling cap is refused, while the
  empty word is admissible. Merging the pair therefore retains an identity letter's domain test; deleting it is
  invalid. Key agreement requires the same admissible starts and end digest, as R-11's extended property test states.
  No key rule removes an intermediate state from the literal sequence used for execution. The property holds under four
  hypotheses, and is false without them (DT-L1 R3-M2). With key(w) the word that replaces every maximal run of adjacent
  letters at one grip c by the single letter (c, g_m⋯g₁):

  1. Only runs of adjacent letters at one grip merge.
  2. Identity products are kept: (2, id) at a blocked grip measures `blocked`, while the empty word is admissible.
  3. The menu is closed under products, as A4₀, S4₀ and I_a are. For a menu that is not, the key holds group elements and
     is not run through the menu test.
  4. Classification is exact, not a budgeted `uncertain`.
- **LG2.15 [D]** Validation and inverse, conjugate, commutator and composition share one well-formed word domain
  (R-11, read with default 10 and DT-L1 M4). These operations are total on well-formed words within granted resources;
  their state actions remain partial. Forming a conjugate does not assert that it is admissible at the caller's
  revision. Admissibility is a fresh whole-word, prefix-by-prefix kernel verdict (KN2.8). Search reductions are not
  additional word identities.
- **LG2.16 [D]** A word's exact length is an unbounded nonnegative integer that names its alphabet. A compressed word
  is a value, such as a straight-line program, with that exact expanded length known before any expansion (KN2.10).
  Length and cost computation never overflows a host integer or expands the word merely to count it. Kernel admission
  can refuse replay beyond its declared bound; that does not make the compressed value ill-typed or change its length.
- **LG2.17 [B]** A proposal uses KN5.9's literal witness interface, including a recipe reference where that interface
  permits one (R-10; KN2.7). Compression supplies a stream of the original literal sequence, not its equality key.
  Exact expanded length and cost precede execution, and no program is split silently (default 10). The engine owns the
  preview witness; caller changes cannot alter it. The language supplies no state or end digest for adoption.

  ```text
  operation propose(base: RevisionRef, word: LiteralWord | RecipeRef, provenance: ProvenanceBlock) -> Receipt
    effect:  prepare a preview
    lane:    as specified by KN5.9
    cost:    the exact length of the literal word, as specified by KN5.9
    errors:  KN5:stale.*, KN5:input.*, KN5:capability.*, KN2:resource_limit.*
  ```

- **LG2.18 [D]** The proposed `word-key/1` recognizes exactly equality of the following normalized sequences, under
  the same LG2.23 header. For Full, scan chronological signed generators with a stack, cancelling only adjacent
  inverse pairs until none remains. No generator-order relation, commuting swap or other group relation is used.
  For jumbling, replace each maximal adjacent run at grip c by `(c, g_m ... g_1)`, using exact products in chronological
  action order. Keep one token even when its product is identity. The token retains the admissibility test at c;
  it is not the empty sequence. A product outside the menu is still an exact group element in a key, never an
  executable menu item. No commuting swap, identity deletion, minimality or dynamic-target reduction is part of this
  key. Its partial-map assertion has LG2.14's four hypotheses. LB1.10 defines a separate trace key, without composing
  these two rewrite systems. Equal keys recognize a sufficient equivalence; unequal keys do not refute equality of
  actions. Keys never replace literal execution, prefix protection, peak-height checks or witness bytes.

- **LG2.19 [D]** Form 1 uses a display qualifier in addition to the model, level, precision and effect indices.
  `Approx<p>`, `ApproxBool` and chapter 10 display values have that qualifier. Comparison of approximate operands
  returns `ApproxBool`; boolean operations on it return `ApproxBool`. There is no cast, equality test, pattern match
  or serialization operation that extracts an exact boolean, integer, text, tag or identity from such a value.
  Explicit exact-to-approximate conversion names p. Exact constants may be embedded in display data; that embedding
  has no inverse. The following rules are conservative even when a programmer believes a dependency irrelevant:

  | Boundary | Checking rule |
  | --- | --- |
  | Function | An approximate/display argument or captured environment requires a display-qualified result and a pure display body. Return annotations cannot erase the qualifier. An ordinary `Bool` return from an approximate comparison is `LG2:type.exactness_violation`. |
  | Generic or aggregate | Substitution preserves the qualifier recursively. A container with a display element, a tag chosen by `display_if`, or a display-dependent shape is display-qualified as a whole. Projection, matching, length, indexing and equality cannot extract an exact result from it. |
  | Closure or higher-order call | A closure's type includes its captures and callable effect/result contract. Capturing `ApproxBool` makes it display-qualified; generic wrappers and function arguments preserve this. An unresolved callable cannot be admitted as exact or pure. |
  | Storage | Display values may be stored only in typed display fields. Decoding retains the qualifier, precision and contract version; untyped imported bytes cannot assert an exact value. No display-dependent bytes or digest become a word, exact parameter, evidence input or budget. |
  | Module or export | Exported signatures and the checked dependency closure retain qualifiers and captures. An importer checks them against the executed digest. A host adapter or foreign export must prove the same rule, or is refused. |

  Exact fields already computed outside display evaluation may be supplied to a display constructor. This does not
  permit extracting those fields from its result or selecting a canonical object through an approximate branch.
  Chapter 10 owns the fixed canonical associations for picking; display coordinates cannot manufacture them.

- **LG2.20 [D]** Ordinary `if`, match guards, loop bounds, recursion control, short-circuit exact booleans and exact
  collection indices accept exact inputs only. `display_if(c, a, b)` requires `c: ApproxBool`; a and b are lazy pure
  display expressions with compatible display types and precision. Only the selected branch is evaluated. Its
  display context propagates into every callee, local binding and nested branch: even an exact literal computed
  there cannot escape as an exact result. No branch performs a kernel decision, state read, capability request,
  workspace/file write, preview or evidence construction. Exact snapshot facts can be read before entering it.
  Approximate data cannot bound iteration or recursion. LG4/DET meter the conditional and the branch actually taken
  as H6 requires; this clause sets no tariff.

  Display evaluation is a terminal consumer of already computed exact inputs. Its grant is fixed from exact inputs
  before evaluation. Its used budget, stop, exception and publication outcome are display/host observations, with no
  continuation back into exact computation or an exact budget pool. A later exact computation cannot read them or
  reclaim a branch-dependent remainder. Thus a cheaper approximate branch cannot select an exact word by making
  extra exact work affordable. Failure leaves the exact inputs unchanged and publishes no partial exact result.
  The deterministic outcome, code, coverage and granted/used budget fields still belong to DET.6's exact
  reproduction payload, including peak used memory under DET.58. Terminal isolation does not remove DET.64's
  predicate and size agreement requirements. Removing them would require an explicit versioned split of that
  payload; no such split is specified.
  The host may publish the display value through `publish_view` only, subject to chapter 10 and BND's independent
  policy. LG2.4 keeps the exact-request direction under proposed amendment A6 and the A2 filter at their kernel
  boundaries; A1 stays in force until EN1's Astra check and the owner.

- **LG2.21 [D]** Form 2 remains an alternative: a separately checked display region marks every approximate datum,
  condition and value computed under its program-counter dependence as display-dependent. Joins propagate that
  dependence through calls, captures, generic substitution, aggregate tags and shape, storage and exports as in
  LG2.19. A comparison may use a region-local boolean representation, but it is not an exact `Bool` at the boundary.
  Every region exit has a display type and only the `publish_view` sink; exact results and effects cannot leave it.
  LG2.20's pure, terminal evaluation, budget isolation and reproduction rule still apply. Choosing this form replaces
  `ApproxBool` and `display_if` admission with region and exit checks; it does not permit the ordinary-boolean helper or change
  LQ21. The checker and route must advertise the selected exactness-contract version before LP2.

  Conditional on this form, `lp1-exec-region-draft-1` uses DET.54's contract with this lowering: `display_enter`
  opens a logical display scope before any region computation; each conditional evaluates its condition, then
  `display_branch` selects only its taken continuation; `display_exit` closes the scope on normal completion.
  Entry borrows the already computed inputs and creates no function frame or payload owner. Retaining a value,
  computing a predicate, calling a helper and constructing a closure use the ordinary charged operations. The exit
  moves an already owned display result to the terminal publication continuation and releases other local owners
  under DET.58; a borrowed result first needs an explicit charged owner. It creates no exact continuation.
  Publication is a separate `publish_view` operation; an exit alone
  publishes nothing. Nested conditionals add branches, not implicit region entries or exits. Each explicit nested
  region has its own pair; its result remains inside the enclosing display computation. DET.64 supplies charges.
  Generated entry and exit operations name the region's originating EV1.10 range; a branch names its conditional's
  range. A denied exit leaves publication unperformed and follows DET.58's terminal cleanup. The execution identity
  occupies DET.54's existing version fields and changes the key; it is not a new model identity or an owner selection.

- **LG2.22 [D]** The LP1 vectors are semantic notation, not LQ13 syntax. `approx_less(x,y)` has approximate operands
  and a display condition. Direct use as an ordinary `if` condition, a helper declared to return `Bool`, a closure
  capturing it and returning an exact budget, `Box<ApproxBool>` projected to an exact boolean, and a saved display
  condition decoded as an exact goal all yield `LG2:type.exactness_violation`. Exporting that helper does not change
  the refusal. Pure display branches choosing two colours through `display_if` yield a display colour; exporting or
  storing that display value with its qualifier is admissible, subject to the other checks. A display-region version
  of the same examples has the same permitted sink and exactness refusals. CT-LG2-05 covers both mechanisms under
  capability separation (LQ21; RTE.24); these are specified results, not executions of a checker.

  At an exact-use site, `LG2:type.exactness_violation` takes precedence for a display condition (`ApproxBool` in
  form 1), a value computed or selected under approximate control, or qualifier erasure through LG2.19's helper,
  closure, aggregate, storage or export boundaries. Otherwise, a mismatch consisting solely of a direct approximate
  scalar at an exact numeric/geometric operand or an attempted scalar conversion back to exact uses
  `LG2:type.exact_required`. This is a diagnostic distinction; both uses are refused. Thus `approx_less(x,y)` in
  an ordinary condition and a helper returning it as `Bool` both yield `LG2:type.exactness_violation`; a direct
  `Approx<p>` operand to exact addition yields `LG2:type.exact_required`. EV1 forwards these codes unchanged.

- **LG2.23 [D]** Key identity includes KN1's identity block, the alphabet, key kind and rewrite version. Jumbling
  also pins the exact menu mapping and order already required by that block. Trace keys additionally name LB1.10's
  independence contract and digest. Equality requires matching headers and token sequences, not only matching
  hashes. `word-key/1` orders Full tokens by absolute generator ID then sign (inverse before forward). It orders
  jumbling tokens by grip ID then their exact matrix entries in row-major order, each entry's canonical PS1.4 triple
  compared lexicographically as integers. Sequences use lexicographic order, with a proper prefix first. No machine
  address or host hash order participates. Headers are compared for compatibility before token ordering; incompatible
  keys return `LB1:input.key_contract_mismatch`, rather than an equality or order result. Chapter 05 owns their bytes.
  A persisted key keeps its version and is only a cache alongside its literal word. A new rule, independence relation
  or order gets a new version. Recompute explicitly from the original literal word; never relabel an old key or infer
  execution letters from it. An unsupported version or missing literal source prevents recomputation.

- **LG2.24 [B]** The language includes user shader stages (owner, 6 November 2026, LQ7).
  [D] A stage module is a module kind with its own checker and no fourth profile. Its rules live in chapter 10,
  RPI.68 to RPI.76; a broken stage rule or bound gives `LG2:type.stage_rule_violation`. It may be used under any profile.

## 3. LG3: effects and capabilities

- **LG3.1 [D]** Function types carry effects (partition LG3). The following effect families describe what a function
  can request. Pure means that none of the external effects is requested; deterministic accounting still applies.
  None grants direct access to the engine, store or renderer.

| Effect | Permits | Boundary |
| --- | --- | --- |
| Pure | Construct and compute typed immutable values, including exact algebra and word values | No ambient inputs or session mutation |
| Read a snapshot | Open and read revision-bound snapshots, query stored records, derive immutable states and invoke checks through the kernel interfaces | KN0.3, KN2, KN4.3; chapter 08 |
| Change the view | Publish view-model nodes and display plans with their input revision or derivation | Chapters 04, 10; no renderer call |
| Change the workspace | Request a workspace command's change | LG3.5; KN4.4 |
| Prepare a preview | Request the engine's literal-word preview with provenance | LG3.6; KN5.9 |
| Use a granted file | Read or write within the host's explicit file grant and command rules | Chapters 05, 09; no session-file access |
| Seeded randomness | Draw from the run's explicitly supplied random stream | INV-10; chapter 09 |
| Budgets | Inspect and spend the run's granted deterministic resources | KN7.7 to KN7.11; cannot raise the grant |

- **LG3.2 [D]** Effects and capability requirements are inferred, checked and inherited through calls (partition LG3).
  A function's type includes the effects of every callee it may invoke, including a function passed as an argument.
  A generic instantiation preserves them. An annotation can restrict what checks successfully; it cannot erase an
  inferred effect. The checker refuses a call whose requirements exceed the caller's context and refuses to treat an
  unresolved callable as pure. Module boundaries preserve these summaries (LG5.7).

  ```text
  FunctionType = { arguments: Types, result: Type, effects: EffectSet, capabilities: Requirements }
  ```

- **LG3.3 [D]** An invoked language command has KN5.3's overlay row, kind, contexts and permitted kernel rows. A
  workspace effect does not turn a query into a workspace command. The checker supplies the static verdict and row
  requirements used by KN5.7's availability rule (default 17). Registering a command does not run it. Bindings name
  the command and absolute arguments under DT-M1; chapter 04 owns their definition.
- **LG3.4 [B]** There is no program effect that commits (INV-3). Only the shell's explicit user action supplies the
  commit token (KN5.14, LI.6). A program cannot mint, receive, persist or forward that token, register a commit
  command or obtain a commit through another effect. A preview effect is not commit permission.
- **LG3.5 [D]** Views have no workspace-write effect, including through callees (default 16; DT-L1 E3). A workspace
  change occurs only inside a command the solver invokes, as one undoable action. Its action order and stale-undo
  check are KN5.19's, following DT-M1 Q4 (a). Re-evaluating a view therefore cannot add workspace actions that supersede
  the undo hint. The checker refuses a view that calls a workspace-writing helper.
- **LG3.6 [D]** Preparing a preview also occurs only inside a command the solver invokes. Views have no preview
  effect, including through callees. This is a new integrator default that follows from BM1's explicit action gate
  and default 16's separation of reactive views from invoked workspace changes; the owner may overrule it. It prevents
  a view from replacing a shown preview on its own. It does not answer KN5.12's question about an invoked command.
- **LG3.7 [O]** Whether an invoked command may replace a preview already on screen remains KN5.12's open dependency
  (DT-L1 E2). Refusing replacement and allowing replacement under a fresh preview ID both retain KN5.14 to KN5.16's
  binding to the shown preview and literal digest. The owner and integrator settle the replacement permission when
  that question is taken up; this chapter does not resume the design line.
- **LG3.8 [B]** Files, clock and randomness are capabilities granted explicitly, with no ambient authority (INV-10).
  Network and process access are not granted in 1.0. A function that reads a clock declares that requirement even
  though the effect families above contain no general system-access effect. A clock grant cannot make seconds a
  deterministic budget or introduce an unrecorded input to a reproducible run. Chapter 09 owns the grants and replay
  treatment of permitted external inputs.
- **LG3.9 [B]** There is one language with three profiles: solving, tool authoring and research (owner,
  6 November 2026, LQ1).

  [D] A module declares its required capabilities; the host grants them per profile and per run (partition
  LG3, BM3). A declaration requests authority and never supplies it. A missing grant is a typed refusal. Importing a
  module does not add to the run's grant. Chapter 09 owns the grant table, and chapter 07 owns its boundary-sensitive
  rows, including the meaning of `Profile` and `BoundaryClass`. No language file grants itself access to session files.
- **LG3.10 [B]** Static success never replaces the engine's run-time revalidation (partition LG3; INV-2). A mode,
  identity, revision, permission or availability checked earlier can be stale at invocation. Every kernel row keeps
  its gates, and commit keeps all of KN5.16's rechecks. The runtime cannot turn a static verdict into engine authority.
- **LG3.11 [D]** Under either form of BM2, proposals and stored words retain their provenance through calls and
  persistence (partition section 11; DT-L1 R2-E3). Chapter 07 defines `ProvenanceBlock`, author classes and fixed
  policy-enabled declassifiers. A program cannot declare a declassifier. Rejected input changes no session provenance
  (R-12); preparing a value or preview is not a commit. Chapter 05 stores the provenance field so that a later program
  cannot erase state dependence by loading a saved word.
- **LG3.12 [D]** If LQ21 selects flow labels, LG supplies the checker rules for both data dependence and control
  dependence, including choices between words and targets, module calls and stored values (BM2; DT-L1 R2-E3). A
  preview's outcome carries the state label. Static-only labels conservatively summarize possible branches and may
  refuse harmless tools; they add no run-time label propagation cost. Static labels with run-time tracking also label
  executor values and branch context, allowing fewer conservative refusals at a charged cost per step. In either
  variant, approval covers exactly the source and dependency bytes executed. RT2 and RT4 need a checker over an
  approved host subset that refuses dynamic code. Run-time labels additionally require owned executor values,
  compiled label fields or a modified borrowed VM. Chapter 07 defines the flow rule and permitted sinks. This clause
  records the unselected alternative and binds nothing (owner, 14 November 2026, LQ21; RTE.24).
- **LG3.13 [B]** The owner selected capability separation (owner, 14 November 2026, LQ21; RTE.24): a program in the
  solving profile either reads state-derived data or prepares previews, never both apart from chapter 07's enabled
  declassifiers (BM2; DT-L1 R2-E3). A previewing
  program cannot read its previews' outcomes, including the end state, N and protection verdict. Call requirements and
  imported or stored provenance keep that separation across helpers, modules, workspace records and files. This form
  works in every route. It refuses a harmless own tool that both reads state and previews, and records coarser
  provenance than flow labels. It does not require labels on every executor value.
- **LG3.14 [B]** LQ21 chose LG3.13, capability separation, over LG3.12's flow labels, static-only or with run-time
  labels (owner, 14 November 2026, LQ21; RTE.24). LX-3's two false refusals are met by fixed enabled declassifiers or
  by splitting the tool, and LX-3's decisive check is part of LP3 acceptance (RTE.24). LQ2 still chooses which flows,
  author classes, sinks and exceptions the policy admits. The first-pass records in `work/experiments/language-lx/`
  were leads, not a ruling. Chapter 07 owns the policy, and chapter 11 owns the route decision.
- **LG3.15 [D]** Both forms retain BM1 to BM5, exactness, deterministic budgets, model identity and the engine's gates
  (partition INV-11; section 11). Neither proves authorship or understanding, detects every solving procedure or
  prevents a person retyping output. Data and branch checks do not close termination or preview-emission channels;
  which display sinks are checked remains LQ2 (DT-L1 R2-E3). A capability or label check is not evidence that a computed
  mathematical claim is true.
- **LG3.16 [D]** Until the joint boundary setting, shipped tools and library entries keep the 2 October boundary and
  the 17 October repair-list exception (partition section 11's interim reading). Research-profile output never becomes
  a solving-session preview, and no solver-tool solving preview is enabled in advance of that setting. The mechanisms
  above do not select BS1 to BS4 or authorize shipped solving functions. The enumerated tier and boundary rulings belong
  to chapters 03 and 07.
- **LG3.17 [D]** A stage module has only the `pure` effect. Its separate stage checker enforces RPI.68 to RPI.76;
  LG3.1 gains no effect family.

## 4. LG4: typed IR and interpreter

- **LG4.1 [D]** The typed IR is the versioned execution contract shared by source, GUI-originated operations and keys
  in RT1 to RT3 (partition LG4, section 12). It preserves types, effect requirements, literal word order, source
  associations and the meaning of resource charges. An IR change that changes saved meaning or budget stopping points
  requires a recorded version change. An unsupported version is refused, with migration owned by chapter 05. Sharing
  the contract does not route every key through the interpreter (LG4.13).
- **LG4.2 [D]** Lowering produces only operations whose operands, results, effects and required capabilities check
  against the following families (partition LG4 and section 9). DET.54 to DET.64 give their draft semantic operations
  and continuations; they select no opcode encoding or serialized IR form. No family
  provides a raw store write, authoritative-state replacement, commit token, renderer call, process or network call.

| IR family | Meaning and required boundary |
| --- | --- |
| Values and exact computation | Construct algebraic values and indexed identities, call exact arithmetic and library operations, perform explicit exact-to-approximate conversion |
| Calls and control flow | Typed functions, generic instantiations, matching, iteration and conditions that preserve effects, verdict alternatives, exactness and the selected BM2 discipline |
| State and checks | Snapshot queries, explicit partial projections, immutable derivation, kernel batches and independent checker requests through chapter 01 |
| Words and recipes | Construct literal sequences, compute separate keys, compose compressed values and stream literal expansion |
| Commands and receipts | Invoke permitted command rows, request previews and read permitted receipts; no commit operation |
| View output | Construct revision-bound view nodes, display plans and source associations defined by chapters 04 and 10 |
| Granted input and output | Scoped file operations, explicit seeded randomness and any permitted clock use under chapter 09's grants |
| Resource control | Deterministic accounting, cooperative cancellation and pointwise contracts |

- **LG4.3 [D]** Evaluation is deterministic and has one thread per run (partition 8.4; INV-10). Observable traversal,
  reduction and output order never depend on addresses, hash-table iteration order or worker completion order. A
  collection operation that produces an ordered result specifies an order in the IR or library contract. Concurrent
  jobs use KN7's lanes and immutable inputs; they do not introduce concurrent evaluation within one run.
- **LG4.4 [D]** The starting executor is an interpreter (partition LG4). No machine-code backend, own VM or JIT is
  introduced before measurement supports it. A borrowed executor under RT2 or RT3 still meets the semantic and resource
  contract; its existence is not performance evidence. That sentence records an unselected alternative and binds
  nothing. Chapter 11 owns route admission and the experiments. The owner chose route RT1, an own interpreter over the
  typed IR with the runtime in a process of its own, and Rust for the engine and the interpreter's host (owner,
  14 November 2026, LQ8, LQ9; RTE.17, RTE.21).
- **LG4.5 [D]** Whole-state operations lower to KN2.3's kernel batches (partition LG4; DT-L1 R2-E2). A program never
  walks every sticker through per-sticker interpreter calls. Projections, protection, end tests, ν and heights use the
  typed decision kernels and their oracle agreement tests (KN2.4). Small exact algebra remains library code. Lowering
  cannot replace the kernel's prefix checks with an unverified composed fast path.
- **LG4.6 [D]** Compressed words stream to the kernel in its declared chunks (KN2.10). The interpreter retains the
  compressed structure and expansion cursor without first materializing the expanded word. Expansion emits the
  original chronological letters and preserves their positions for diagnostics and checker traces. The known exact
  length permits admission and cost reporting before replay. Chunking changes neither the operation's protection
  scope nor its literal digest, and cancellation does not wait for full expansion.
- **LG4.7 [D]** Evaluation enforces KN7's deterministic budget throughout execution (partition LG4; default 6). The
  table states the charges and check locations. A denied charge stops work before that operation exceeds the grant.
  Cache hits and recomputation after eviction leave the logical charges unchanged (KN7.8).

| Budget | Charge | Check location |
| --- | --- | --- |
| Steps | One per semantic operation, including each loop test, call, return, pointwise assertion and applicable flow check (DET.55) | Debit before the operation; DET.56 fixes failure precedence and the charge retained on failure |
| Kernel cost | Pinned input-size tariff for an exact primitive, kernel row or declared streaming chunk (DET.63) | Preflight after the step and before executing work; secondary charges are atomic together |
| Memory | KN7.9's state payload units and DET.57's language layout, charged per logical owning handle, including aliases | Before allocation, growth or frame entry; deterministic release under DET.58, never host GC |
| Output | Canonical uncompressed bytes under chapter 05's pinned profile, as HANDOFF H8 requires | Before each complete output unit is exposed; DET.59 separates terminal reserve and retained output |
| Wall-clock safety stop | No deterministic charge; this is a safety stop only | Host safety checks; a stopped run is incomplete and not reproducible (KN7.10) |

- **LG4.8 [D]** The LP1 draft recommends uniform logical steps with size-dependent kernel tariffs, specified by
  `lp1-exec-draft-1` in DET.54 to DET.64, with LG2.21's distinct identity for form 2. One operation has one step;
  variable-size work has explicit continuations or a declared input-size tariff. This keeps instruction scheduling
  separate from operand size and gives a small
  reference meter. The alternative remains a fixed versioned weight table over logical primitives. That alternative
  changes stopping points and needs a different execution identity, compatibility treatment and the owner's route
  decision under DET.22. Neither form can use cache residency or native instruction counts. This is a draft
  recommendation, not an accepted ADR, route or freeze. The reference vectors use test tariffs only; no production
  tariff, preset or default budget is claimed as measured.
- **LG4.9 [B]** Cancellation is cooperative and checked at interpreter continuations and kernel chunk boundaries
  (partition LG4; KN7.6). A run ends as completed, cancelled, cut off or error with its covered input and used budget
  (KN7.3). A cut-off run gives no negative claim; a witness counts only after checking (KN7.4). A commit or undo cancels
  jobs reading the revision it replaces under DT-M1 Q5 (a). Digest-pinned research jobs finish after a commit under
  KN7.5 (owner, 6 November 2026, LQ4); their results stay bound to their original input.
  DET.56 orders a pending interruption before a new step; DET.63 fixes semantic chunks separately from transport
  chunks. Cancellation does not refund admitted work or claim the coverage of an unfinished chunk.
- **LG4.10 [D]** Pointwise contracts run in the interpreter and consume the step budget. A whole-state contract runs
  once per revision or as an agreement test in CI, never once per call inside a loop (partition LG4; DT-L1 R2-E4).
  Changing the input revision invalidates the per-revision result. A contract assertion is not an independent checker
  pass and cannot promote its own evidence. Decision-path batches retain KN2.4's independent oracle agreement checks.
  Predicate evaluation uses ordinary operations, followed by one `assert` step. An applicable run-time flow check
  is a separate `flow_check` step at the checked boundary (DET.55); static checking creates no run-time charge.
- **LG4.11 [D]** Views evaluate incrementally for changed inputs at a revision and skip to the newest requested
  revision (partition LG4, section 9; DT-L1 E1). Superseded work does not build an unbounded queue. Its output keeps the
  revision or derivation it read and is not applied to a different input. Until current output is ready, its base
  revision and stale status remain available to the view model (chapter 08). This evaluation has neither workspace nor
  preview effects (LG3.5, LG3.6).
- **LG4.12 [D]** IR operations retain associations to their source module digest and source location, including call
  sites and generated operations' originating declarations (partition EV2, EV3). EV2 can step the executed version,
  and EV3 can link an output to that version's source. A compiler or borrowed executor preserves these associations
  through lowering; the current draft is not substituted for the executed source. Chapter 10 owns `SourceMap`, and
  chapter 04 owns the editor and debugger services. A kernel batch can point to its call site and trace references
  without emitting a source-map entry for every sticker.
- **LG4.13 [D]** A key bound to a kernel command row dispatches without an interpreter step or a program job
  (partition section 9; DT-L1 E1). The renderer adopts engine revisions without waiting for language output. No program
  or synchronous engine call runs on the UI thread (R-14). Runtime failure, cancellation or slow views do not prevent a
  commit or undo (KN7.12, KN7.13). These are requirements to verify in LX-2 and LP3, not measured responsiveness claims.
- **LG4.14 [D]** RT1 to RT3 all retain LG4.1's typed IR, deterministic results, bulk calls, streaming, budget charges,
  cancellation and source associations (partition section 12). RT1's interpreter supplies them directly. RT2 needs an
  adapter and the required checker around its admitted host subset. RT3's compiler preserves them in the target VM;
  target fuel alone is insufficient unless it implements the declared logical accounting. Flow-control obligations
  are LG3.13's, capability separation (owner, 14 November 2026, LQ21; RTE.24). The RT2, RT3 and RT4 sentences record
  unselected alternatives and bind nothing (owner, 14 November 2026, LQ9; RTE.21).

  RT4, if selected, must replace the IR contract with an equally versioned contract over the admitted host program:
  domain types and exactness, transitive effects and capabilities, deterministic ordering, streaming and batches,
  logical accounting, interruption and source linkage. Host types or conventions alone do not establish these. Static
  flow labels require the approved-subset checker bound to executed bytes; run-time labels require a modified host
  runtime, while capability separation does not (BM2; DT-L1 R2-E3, as narrowed by the partition's plan check). Chapter
  11 owns the replacement's admission and evidence; this chapter does not select RT4 or silently exempt it.
  For LB6.2 and EV5.4, RT4 either lowers to the common IR or supplies a checked host decomposition contract covering
  transitive calls, effects and executed-source identity. Indirect helper calls must be resolved to the checked
  closure; omitted calls invalidate admission. Its meter maps every continuation, lifetime and output unit to
  DET.54 to DET.64 before it claims the same key (RTE.71). A compatible host signature alone is insufficient.

## 5. LG5: modules, packages and versions

- **LG5.1 [D]** A module version is addressed by its content digest (partition LG5; default 13). A file path or display
  name locates bytes and never establishes identity. The bytes covered by module identity, and the stored manifest
  and lockfile forms, belong to chapter 05. Loaded content is checked against its expected digest before it is used;
  different bytes do not replace the addressed module. Source files use `.6cs` (owner, 6 November 2026, LQ17).
- **LG5.2 [D]** A manifest is a declaration of compatibility and requirements (partition LG5, LB6, BM5). The table
  fixes the meaning of its entries. Declaring a capability, boundary class or principle record does not grant
  permission or prove the module's claims.

| Entry | Meaning |
| --- | --- |
| Supported model identities | The identities under which this module claims compatibility, including the jumbling menu identity, order digest and contract revision where relevant (KN0.1) |
| Capabilities | The module's required authority, checked against transitive calls and the host grant (LG3.2, LG3.9) |
| Edition | The source edition required for its files; a mismatch with their headers is refused |
| Dependencies | The modules required to check and run this module; resolved content identities are fixed by the lockfile |
| Boundary class, where applicable | The class chapter 07 defines, including each built-in tool's reviewed class; it is data for the policy, not an effect that bypasses it |
| Principle record for a built-in tool | Chapter 03's record, carried as manifest data with its fixture, decomposition and evidence references (default 18; DT-L1 R2-E4) |

- **LG5.3 [D]** A lockfile fixes the resolved dependency closure by content digest, with the editions and IR contracts
  needed to interpret it (partition LG5; INV-10; PS2). A run does not resolve a dependency again by a mutable name or
  silently take a newer local file. Missing content, a digest mismatch or an unsupported contract causes a refusal.
  Locking dependencies gives no additional file or network capability.
- **LG5.4 [D]** A run record names the actual source and module digests of its dependency closure (partition LG5,
  section 13). It also records the edition and IR version through chapter 09's `ReproKey`, with the model identity,
  inputs, parameters, seeds, budgets and cost units. Chapter 05 owns `RunRecord` and its bytes. A replay that lacks a
  required version reports the missing content or unsupported version; it does not claim to reproduce the run with a
  substitute module.
- **LG5.5 [B]** If a module's supported identity does not match the session, it is refused for execution or retained
  as recorded material for inspection (INV-7; KN1.4). Its words, evidence and constants are never reinterpreted for the
  session's identity. A migration, where one exists, produces a separately identified result under chapter 05. Matching
  a model digest alone does not waive a mismatched menu-order digest or contract revision.
- **LG5.6 [D]** The last valid module version keeps running while its draft is broken (partition EV1). A draft's
  source, manifest or dependency edits cannot mutate that active content-addressed version. Parsing, typing, effects,
  identity and capability requirements are checked for the candidate before it can replace the valid version. A run
  and its diagnostics distinguish the active module digest from the draft's digest, so a broken edit cannot mix a new
  body with an old approval. Each run continues to name the version it executed.
- **LG5.7 [D]** A dependency's exported types carry its inferred effects and capability requirements (partition LG2,
  LG3). The importer checks them against its own declarations and the run's grants. An import neither resets provenance
  nor makes an effect pure, and a module cannot hide a state read behind a declared preview-only interface. Both forms
  of BM2 apply across the dependency closure (LG3.12, LG3.13). Separate source editions interoperate only through
  compatible typed contracts; the importer never reparses a dependency under its own edition.
- **LG5.8 [D]** A fork inherits source and the recorded derivation of the fork, with author classification owned by
  chapter 07. It inherits no evidence (default 18; DT-L1 R2-E4). A changed module has a new content digest. Citations
  to the parent remain citations about their original subjects; they are not checks of the fork. Running a fork still
  requires the host's grants, and its principle record and claims use chapters 03 and 06's checks.
- **LG5.9 [B]** 1.0 shares modules only through local files the user chooses, with digests and no registry or network
  (owner, 6 November 2026, LQ12). [D] The same manifest, lock and grant checks apply. Network and process access
  remain absent from 1.0 programs under INV-10.
- **LG5.10 [D]** Updating source or dependencies creates a new addressed version, and changing an edition or IR meaning
  follows the recorded migration rules (partition PS2). It never overwrites the meaning of a saved word, run or
  evidence reference. Retaining the active valid module during a broken edit is LG5.6's behavior; silently changing
  the version named by an existing run is refused. Source syntax, IR and module formats remain later freezes under
  the data-core-only freeze (owner, 6 November 2026, LQ10), rather than being fixed by this chapter's conceptual tables.
- **LG5.12 [D]** `stage` is a module kind under LG2.24, with the separate checker and rules of RPI.68 to RPI.76.
  It uses the same syntax family selected after LX-8 and the `.6cs` source extension of LG5.1.

## 6. Error codes added by this chapter

- **LG5.11 [D]** The codes below use default 5's categories and give stable diagnostic distinctions. Chapter 13 owns
  their final spelling and registry. Errors retain the source and IR location where available. A syntax, type or
  effect refusal does not execute a candidate; a run-time refusal keeps chapter 01's receipt and job-outcome rules.
  Kernel error families in the signatures above are inherited rather than redefined here.

| Code | Meaning |
| --- | --- |
| `LG1:syntax.edition_missing` | No edition header selects the source rules |
| `LG1:syntax.edition_unsupported` | The requested source edition is not supported |
| `LG1:syntax.invalid_encoding` | Source bytes are not valid under the UTF-8 and pinned-byte contract |
| `LG1:syntax.invalid_literal` | A literal has no value of its declared kind, including a zero rational denominator |
| `LG1:syntax.unexpected_token` | Parsing failed at a token or required structure; recovery locations accompany the code |
| `LG2:type.identity_mismatch` | An argument or comparison uses another puzzle or model identity |
| `LG2:type.level_mismatch` | A level is wrong or a partial projection was used as a total value |
| `LG2:type.exactness_violation` | ApproxBool misuse or display-dependent control/result escape, with precedence under LG2.22 |
| `LG2:type.exact_required` | Direct approximate scalar at an exact operand or conversion, delimited by LG2.22 |
| `LG2:type.cost_unit_mix` | Puzzle or cost unit indices do not agree |
| `LG2:type.reachability_missing` | A state lacks its typed reachability claim |
| `LG2:type.mode_mismatch` | A session operation requires the other mode |
| `LG2:type.verdict_required` | A decision loses unknown or uses an evidence-bearing verdict as a boolean |
| `LG2:type.word_key_not_executable` | An equality key is supplied where literal execution data is required |
| `LG2:type.stage_rule_violation` | A stage module breaks a stage rule or bound; LG2.24 and RPI.69 to RPI.71 |
| `LG4:input.ir_version_unsupported` | The executor cannot preserve the requested IR contract |
| `LG3:effect.undeclared` | A function's transitive effects exceed its context, including a view writing workspace or preparing a preview |
| `LG3:capability.not_granted` | A declared or inferred capability lacks the required run grant |
| `LG3:capability.flow_refused` | The flow-refusal code under capability separation: the boundary mechanism refuses a flow or a combination of capabilities (owner, 14 November 2026, LQ21; RTE.24; ERR.24) |
| `LG3:effect.flow_refused` | Retired before first use and never emitted: the unselected flow-label spelling (ERR.24) |
| `LG5:input.module_digest_mismatch` | Loaded module content differs from its expected digest |
| `LG5:input.module_dependency_missing` | Content required by the lockfile is unavailable |
| `LG5:input.module_contract_mismatch` | Manifest, file edition or dependency contracts do not agree |

## 7. Conformance tests

These are required tests for later phases, not results. None exists yet. The classes describe the proposed evidence;
no entry claims a Windows, DirectX, input, long-session or performance result. Chapter 12 assigns the tests to phases.
Tests of open mechanisms use the selected option's contract and cannot decide the owner question by themselves.

| Test | Checks | Class |
| --- | --- | --- |
| CT-LG1-01 | A missing or unsupported per-file edition is refused without trying another grammar; mixed supported dependency editions keep their meanings | source |
| CT-LG1-02 | R-03 fixtures yield the pinned UTF-8/LF digest bytes from LF and CRLF checkouts; malformed UTF-8 is refused | source |
| CT-LG1-03 | The formatter gives identical output from the same parsed program in different host contexts, is idempotent and preserves parsed meaning | source |
| CT-LG1-04 | A local syntax failure retains locations and later independent declarations; recovered holes never execute and stable error codes survive diagnostic-text changes | source |
| CT-LG1-05 | Large integer, rational and Q(√5) literals lower without float rounding; approximate literals remain a different kind and a zero denominator is refused | source |
| CT-LG1-06 | LX-8 candidates lower the shared semantic corpus to the same typed operations, effects, literal order and charges before the owner's learnability protocol runs | source |
| CT-LG2-01 | Algebraic matches account for every constructor; generics and inference cannot erase a model, level, mode, precision, unit or effect requirement | source |
| CT-LG2-02 | A Full piece cannot substitute for a jumbling piece; pose, sticker class, label digest and pose-table index are not interchangeable | source |
| CT-LG2-03 | Partial projection is handled explicitly and no typed operation lifts labels to poses; a read of labels off the lattice refuses, and no test treats a class as equality | source |
| CT-LG2-04 | Explicit exact-to-approximate conversion names precision; direct casts and helper-mediated approximate-to-exact conversions are refused | source |
| CT-LG2-05 | Under each LG2.5 working form, approximate comparisons cannot steer a word, target, exact goal, evidence or budget through branches, generics, closures, storage or module calls; LG2.22 determines diagnostic precedence and LG2.20 isolates display accounting without removing DET.6 reproduction | source |
| CT-LG2-06 | Addition, ordering and budget use refuse mixed puzzle or unit indices for menu letters, retained generators and UI actions; seconds cannot be a deterministic budget | source |
| CT-LG2-07 | Raw pose data cannot enter as a state without validation and a reachability claim; a jumbling witness does not assert retained reachability | source |
| CT-LG2-08 | Wrong-mode session calls fail statically where known and at invocation after a mode change; a stored menu word never changes the session mode | source |
| CT-LG2-09 | Unknown cannot pass as false, an off-lattice projection cannot pass as unknown, and a goal cannot silently weaken its required level | source |
| CT-LG2-10 | Goal fixtures check the sourced implication chain and the checkpoint implications; equal labels do not require equal poses and a label checkpoint supplies no retained word | source |
| CT-LG2-11 | No execution, derivation, protection or proposal binding accepts a WordEqualityKey as a LiteralWord | source |
| CT-LG2-12 | R-11's operations accept the validated word domain; on jumbling fixtures that meet LG2.14's four hypotheses the chosen key preserves admissible starts and end digests, while literal prefix protection and peak height stay unchanged | source |
| CT-LG2-13 | A same-cap inverse pair at a straddling cap retains its domain test in the key; the empty word cannot substitute for its literal witness | source |
| CT-LG2-14 | A compressed word computes an exact length beyond machine-integer range without expansion; streaming positions and letters equal an independently expanded small fixture | source |
| CT-LG2-15 | A stage module is checked under RPI.68 to RPI.76 in each of the three profiles; a violated stage rule or bound yields `LG2:type.stage_rule_violation` and no unchecked stage reaches the adapter | source |
| CT-LG2-16 | Under proposed A6, programs receive rotation requests only as opaque exact host values and write exact literals; helpers and generic casts cannot convert approximate data into a request, and menu drags resolve through KN3.8's kernel call | source |
| CT-LG3-01 | The function and import call graph preserves transitive effects and capability requirements, including higher-order and generic calls | source |
| CT-LG3-02 | No program effect or overlay row can commit or obtain a token; a helper cannot hide that request under another effect | source |
| CT-LG3-03 | Views refuse workspace and preview effects through direct and indirect calls; an invoked workspace change contributes one undoable action under DT-M1 Q4 (a) | source |
| CT-LG3-04 | Undeclared or ungranted file, clock and randomness requests fail; network, process and session-file access fail even when requested in a manifest | source |
| CT-LG3-05 | Static success cannot bypass a run-time stale revision, mode, model, availability or protection refusal | source |
| CT-LG3-06 | The flow-label variant follows data and branch choices through a module and a saved word, labels preview outcomes and binds checker approval to executed bytes; static-only and run-time variants are checked separately | source |
| CT-LG3-07 | Capability separation refuses a state-reading preview program, preview-outcome reads and laundering through a second module or saved word; only policy-enabled fixed declassifiers cross the boundary | source |
| CT-LG3-08 | LX-3's must-refuse cases include selection by a state fact, search then rebuild from own words, the ν correction solve and persistence laundering; false refusals are recorded under capability separation, and LX-3's two are met by fixed enabled declassifiers or by splitting the tool (RTE.24) | source |
| CT-LG3-09 | R-12 rejected input preserves session provenance, and a fork, imported word or research result cannot acquire a new author class or solving permission by being saved | source |
| CT-LG3-10 | A stage module admits only `pure`; direct or transitive snapshot, workspace, preview, file, randomness or other effect requests fail the stage checker | source |
| CT-LG4-01 | Repeated runs and host collection layouts give identical result bytes and deterministic stop points for one key; admitted routes preserve the logical contract | source |
| CT-LG4-02 | Lowered whole-state operations call kernel batches; no per-sticker interpreter bridge is present, and kernel decision calls retain oracle agreement coverage | source |
| CT-LG4-03 | Compressed replay streams under a small memory grant, cancels at declared boundaries and preserves literal digests and prefix checks without silent splitting | source |
| CT-LG4-04 | Exhaust each deterministic resource separately; cache hits and eviction do not change charges, and a wall-clock safety stop is incomplete rather than a reproducible negative | source |
| CT-LG4-05 | Pointwise contracts consume steps; whole-state contracts run once per revision or in CI and are not inserted in per-call loops | source |
| CT-LG4-06 | Revision bursts supersede queued view work; results keep their input and cannot publish for another revision or alter workspace and previews | source |
| CT-LG4-07 | IR stepping and output linkage resolve the executed module's source, including compiled call sites, while an invalid draft remains separate | source |
| CT-LG4-08 | A direct kernel binding dispatches without language execution; runtime faults leave the journal and state hash intact as required by LX-2 | source |
| CT-LG4-09 | LX-2 and LP3 check the required responsiveness with programs and views active under the owner's measurement protocol | performance |
| CT-LG5-01 | Content changes under a module path fail the expected digest; an unavailable locked dependency fails without resolving a replacement | source |
| CT-LG5-02 | Manifest compatibility, edition, transitive requirements and boundary declarations are checked as data and never grant authority themselves | source |
| CT-LG5-03 | A run record identifies the actual active dependency closure and contracts; replay refuses missing or unsupported versions | source |
| CT-LG5-04 | A model or menu-order mismatch leaves the module refused or recorded; execution never reinterprets its words or constants | source |
| CT-LG5-05 | Broken source, manifest and dependency drafts leave the last valid version running with its own digest and approval | source |
| CT-LG5-06 | A changed fork gets a new content digest and requires run grants, inherits no evidence, and cannot attach the parent's check to its changed subject | source |
| CT-LG5-07 | Sharing through local files preserves digest, lock, identity and grant checks; a signature alone supplies neither evidence nor capability | source |
| CT-LG5-08 | A `.6cs` module of kind `stage` reaches the stage checker, shares the selected syntax family and introduces no fourth profile; module digests, locks and grants keep their existing checks | source |

## 8. Open items

The owner answered LQ1 to LQ21 on 6 November 2026 and chose LQ8, LQ9 and LQ21 on 14 November 2026: the engine and the
interpreter's host in Rust, route RT1 with the runtime in a process of its own, and capability separation (RTE.17,
RTE.21, RTE.24). LQ13 waits for its evidence, and LQ2's option set waits for the joint setting.

| Owner question | Clauses and dependency |
| --- | --- |
| LQ2 Boundary policy | LG3.11 to LG3.16. Chapter 07 owns option sets, permitted flows, declassifiers, display sinks and author classes. |
| LQ8 Engine language | LG1.6, LG4.4, LG4.14. [B] Answered: the engine is written in Rust (owner, 14 November 2026, LQ8; RTE.17). |
| LQ9 Route and placement | LG3.12 to LG3.14, LG4.1 to LG4.14. [B] Answered: route RT1, runtime in a process of its own (owner, 14 November 2026, LQ9; RTE.21). |
| LQ13 Syntax | LG1.1 to LG1.8. [B] The choice waits for LX-8 before LP2 builds the parser (owner, 6 November 2026, LQ13). |
| LQ21 Flow-control form | LG3.12 to LG3.14, LG4.14. [B] Answered: capability separation (owner, 14 November 2026, LQ21; RTE.24). |

Open mechanisms and dependencies:

- **Approximate comparison and control flow (LG2.5, [D]).** The draft recommends H6 form 1, specifies form 2 and
  records exactness vectors in LG2.22. LP2 still owes checker execution; LQ21 (answered) remains independent of it.
- **Canonical jumbling keys (LG2.18, [D]).** The draft specifies adjacent merging with retained domain tests and
  versions in LG2.23. LB1.10 separately specifies trace equality. Implementations and their conformance are still owed.
- **Invoked preview replacement (LG3.7, [O]).** KN5.12 remains unresolved. Either permission keeps binding to the shown
  preview; reactive views have no preview effect under LG3.6.
- **Flow-control policy (LG3.14, [B]; enabled rules [O]).** LQ21 chose capability separation (RTE.24); LQ2 still
  decides the enabled rules. The choice was not made by selecting a syntax or host runtime.
- **Deterministic charge schedule (LG4.8, [D]; DET.22, DET.28 [O]).** DET.54 to DET.64 specify the draft operations,
  input-size tariff form, owning-handle lifetimes, canonical-byte output and logical frames. LG2.21 supplies the
  distinct display-region lowering. The weighted-step alternative remains separate. Executor mappings, production
  tariffs and limits, conformance evidence and the owner freeze remain outstanding; no timing is supplied here.
- **New integrator default LG3.6.** Preview preparation is confined to commands the solver invokes, and views have no
  preview effect. This follows from BM1 and default 16; it is additional to the partition's explicit effect defaults
  and the owner may overrule it.
- **DT-L1 obligations.** Round 3 cleared them (overview section 8). LG2.3, LG2.13 and LG2.14 carry its corrections, and
  the uses of R2-E2 to R2-E4 stay cited in their clauses.
- **EN3, EN4 and LP1 interfaces.** KN2.3's batches, KN5's overlay and query rows, undo during jobs, lanes and final
  command contexts remain chapter 01's dependencies (LG3.3, LG3.5, LG4.5, LG4.9, LG4.13). This chapter does not repair
  the current H-08 draft or choose its final binding shapes.
- **Source reconciliations.** R-11's normal-form wording is used with default 10's literal execution rule
  (LG2.13 to LG2.15). State contract A5 amends section 5's retained-checkpoint-only handoff and A3's checkpoint wording
  while retaining exact pose equality (LG2.10, LG2.11). Partition 4.1's derived-state base revision is lineage, not a
  session revision assigned to that value, following INV-1 and KN2.12 (ONE.21, LG2.12, LG4.11). H-08's separate
  workspace-undo description is read with DT-M1 Q4 (a)'s one durable action order (LG3.5). None is a new mechanics rule.
- **Other chapter contracts.** Chapters 03, 04, 05, 06, 07, 09 and 10 supply the definitions routed in LG2.12 and LG5.2.
  Chapter 05 finalizes digest bytes, chapter 09 capability grants and external-input replay, and chapter 13 final error
  spelling. This chapter supplies no schema and assumes no unpublished clause numbers.
