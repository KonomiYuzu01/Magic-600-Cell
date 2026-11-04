# The language's ecosystem: partition and owner questions

Status: **draft for the owner, step 1 of the language kickoff of 3 November 2026, revised after rounds 1 to 4 of the
round table. Not accepted.** Nothing on this page is an ADR, a frozen architecture or a chosen syntax. The page divides
the ecosystem of the project's own language into components with boundaries. It also maps every capability the kickoff
names to a component and lists the decisions the owner takes. The owner confirmed the direction, and the full text
follows it in [`architecture/`](architecture/00-overview.md) and `adr/`. Section 19 lists what round 3 corrected here.
Accepting ADRs and freezing the architecture are the owner's decisions.

- Round table: [DT-L1](../../../progress/1.0/design-table/topics/language-ecosystem.md) (the mathematician and the
  engineer advisor). Its cards use the IDs of the first draft; section 18 maps them to the IDs on this page.
- Working name: C600 Script (LQ17). "The language" below means this language.
- Words such as `Snapshot` or `Preview` name concepts, not final identifiers.
- Every ID on this page carries a prefix of its own (section 18). A reference to another document always names that
  document, for example "charter E2", "state contract A1", "DT-M1 Q5 (a)" or "DT-J1 rule 6".

## 1. What binds the language

### 1.1 Sources in force

The authority order is the one in `AGENTS.md`: owner decisions, then `AGENTS.md` and `CLAUDE.md`, then accepted ADRs
and contracts, then source and test evidence, then the wiki.

| Source | What it fixes for the language |
| --- | --- |
| [Owner, 3 November 2026](../../../wiki/decisions/owner-decisions-2026-11-03.md) | Language first. Scope is the complete ecosystem and architecture. The language and what restoration and the research need are in 1.0. The handoff's syntax, example tools and projection prototype are references only. Sets the principle for tools a solver develops; built-in tools keep the 2 October boundary until the exact boundary is set. |
| [Owner, 2 October 2026](../../../wiki/decisions/owner-decisions-2026-10-02.md) | The human-solve boundary; Windows first with platform-independent layers; the design decisions (Global and Local only, soft disclosure, competence checks) |
| [Owner, 17 October 2026](../../../wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md) | The repair list at a cap the solver names, in neutral order, without ranking |
| [Owner, 27 October 2026](../../../wiki/decisions/owner-decisions-2026-10-27-jumbling-handoff.md) | Research data only. The jumbling line is a puzzle to be solved, and restoring any depth-3000 scramble is open. |
| [Charter 2.0](../../../progress/1.0/charter-2.0-draft.md) | Six layers. Evidence, score and permission are never merged (bar M3). Quality bars E1 to E7, M1 to M6 and D1 to D8. Stage 2.5 ends with the architecture freeze and migration format freeze gate rulings, before implementation. |
| [Command table (H-08)](../../../progress/1.0/command-table.md) | Command kinds, gates, preview rule, undo class, contexts, repeat, jobs |
| [Requirements R-01 to R-19](../../../progress/1.0/requirements-from-screening.md) | One permission path. No commit outside the normal path. A preview owns an immutable witness. One normal form for words. No engine call on the UI thread. Atomic job admission. Key repeat never repeats a commit. |
| [State contract A1 to A5](../../../../research/jumbling/state-contract.md), [render contract](../../../../research/jumbling/render-contract.md) | Exact jumbling mechanics, admissibility certificates, sticker identity, menus, label checkpoints. The engine is the source of truth; floats are for drawing only. |
| [DT-M1](../../../progress/1.0/design-table/topics/command-keyboard-model.md), [DT-J1](../../../progress/1.0/design-table/topics/jumble-solver-aids.md) | The command and keyboard model and the jumble solver aids, both settled on 22 October 2026. Among them: DT-M1 Q4 (a), one order of actions across the journal and the workspace; DT-M1 Q5 (a), a commit cancels the jobs that read the revision it replaces, while file jobs finish; DT-J1 rule 6, which facts a tool may show. |
| [Glossary](../../../progress/1.0/glossary/glossary.md) | One term per concept, checked by script (charter bar M1) |
| `assets/manifest.json` and the J1 identity | The model boundaries |

### 1.2 Invariants

Every component below is built on these twelve invariants. INV-1 to INV-11 each restate a binding source, and none is a
new decision. INV-12 is a proposal built on the owner's 3 November principle: no owner text names a principle record.

- **INV-1 One mechanics, one authoritative state.**
  - Each session has exactly one authoritative puzzle state. The engine owns it and the session store records it.
  - The language holds no second mutable puzzle state and has no mechanics of its own.
  - A state a program computes, such as a "what if" or a research configuration, is an immutable derived value:
    - the same exact kernels compute it;
    - it carries its derivation, its model identity and a reachability claim in a named puzzle;
    - it has no session revision;
    - it lives in the kernel's value store by digest, charged to the program's memory budget.
  - A derived state can be compared, and displayed through a display plan that names its derivation. It is never adopted
    as the session state.
  - The one state a commit adopts is the end state of the engine's own preview object, when the revision and the digest
    of its literal letters are unchanged (ENG-0J). A program never supplies the committed state or its digest.
- **INV-2 One mutation path.**
  - Every change to a session goes through the command layer and its gates, whether the change is mechanical, to the
    workspace or to the view.
  - The runtime has no write access to the engine or the store. It reads the store only through the command layer's
    query rows.
- **INV-3 Execution only by the solver's action.**
  - A program can at most prepare a preview. Only an explicit user action commits it.
  - The commit takes one action on a fresh preview (charter S4, DT-M1 C4), and key repeat never repeats a commit (R-18).
  - A live twist has no preview: it is an immediate complete operation under the active protection, taken by one explicit
    solver action (solving workflow 4.1, DT-M1 C4; chapter 08 reads charter S4 for it).
  - The commit names the preview on screen. The shell mints the user-action token from a non-repeating key-down and
    binds it to that preview's ID and to the digest of its literal letters, together with the model identity, the menu
    identity, the menu-order digest and the contract revision. The token never binds a normal form (4.1). The engine
    refuses a mismatch as stale.
  - Which programs may prepare which previews is the boundary policy (INV-11).
- **INV-4 Exact decisions, floating-point display.**
  - Every decision is exact: legality, admissibility, protection, equality, N, ν, digests and evidence.
  - Floats appear in only three places:
    - display plans;
    - the state contract's A2 filter, with its proven bound;
    - the float requests that the state contract's A1 input map turns into exact rotations, and the drags that menu
      mode turns into menu letters. The journal records the exact result, and replay uses it. On Windows CPython the
      cosine and sine behind the A1 map are not correctly rounded for 484 and 436 of 13,600 requests (DT-L1 R2-E2;
      synthetic evidence), so another maths library can return the neighbouring value. Whether the choice is the same
      under every runtime, so that a run stays reproducible (INV-10), is spike LX-9.
  - Types keep exact and approximate values apart. Nothing converts implicitly from approximate to exact.
- **INV-5 Evidence is not score and not permission** (charter bar M3).
  - A claim is a typed value with evidence. Unknown is never false, and sampling never upgrades a universal claim.
  - Evidence, score and permission are separate fields.
- **INV-6 Identity is canonical, never visual.**
  - Objects are addressed by canonical IDs bound to a model and to a level of the identity tower (KN1).
  - Screen positions, pose-table indices and display instances are never identities.
  - Every display instance maps back to a canonical ID.
- **INV-7 Model identity is immutable.**
  - Programs, words, records and evidence carry the model identity. For jumbling they also carry the menu identity, the
    menu-order digest and the contract revision.
  - On a mismatch they are refused or marked as recorded, never reinterpreted.
  - A derived model, such as a quotient, a restriction or a simplified model, is a value with a derivation. It is never
    a new authoritative puzzle. A claim moves to or from it only along a map that preserves the claim's subject.
- **INV-8 Renderer independence.** The language produces view-model content and display plans, and a renderer adapter
  draws them. No language feature depends on which renderer is selected.
- **INV-9 Platform independence.**
  - The kernel, runtime, libraries and checkers run headless on Linux.
  - They give the same result bytes on Windows and on Linux (LX-3).
  - Windows code stays behind platform interfaces.
- **INV-10 Determinism and no ambient authority.**
  - A run is reproducible from its source and module digests, model identity, inputs, seeds and budgets.
  - Budgets are deterministic: steps, memory in logical units, output size and the cost each kernel call declares.
    Wall-clock time is only a safety stop, and a run it stops is incomplete and not reproducible.
  - Files, clock and randomness are capabilities granted explicitly.
  - Network and process access are not granted in 1.0.
- **INV-11 The boundary is a policy over fixed mechanisms.**
  - The mechanisms BM1 to BM5 of section 11 exist under every option: the execution gate, provenance and flow control,
    profiles, labelled records and boundary classes.
  - Flow control takes one of two forms, flow labels or capability separation. The form changes what each route must
    build (section 12), so it is owner question LQ21.
  - The owner and the integrator set the policy together.
  - Until then, built-in tools keep the 2 October boundary and the 17 October exception.
- **INV-12 The means of understanding are a deliverable.**
  - Every built-in tool ships with viewable source and a principle record:
    - an exact specification: the function, its domain, and what an empty or negative answer means;
    - the lemma or certificate it rests on, with its evidence;
    - one small instance on the real geometry whose steps a person can check by hand from named givens;
    - its decomposition into base parts;
    - the parts a person writes: a step map, the reason, the failure of a dropped hypothesis, what an answer does not
      mean, the derivation of the instance, the prerequisites and the leaf specifications.
  - Every result the runtime produces can be traced to primitives and evidence. An imported result traces to its stored
    bytes and source label, and the trace says that its producer is absent.
  - The 3 November principle presumes that a solver understands their tool's principle. The program can supply only the
    means.

## 2. The partition at a glance

There are five layers and five pillars that cross them. The kernel is not new: it is the lower half of the charter's six
layers (engine, session store, command layer), with what the language needs from it. The other four layers are new.

```text
EV  Environment          editor services · debugger and replay · source-UI linkage · commands, bindings, macros, recording · docs and learning
LB  Libraries and tools  base · 600-cell-Full · jumbling · representations · research · built-in tools · user modules   (written in the language)
LG  Language core        syntax and editions · types · effects and capabilities · typed IR and interpreter · modules and packages
KN  Kernel               model identity · exact engines · exact arithmetic · session store · command layer · checkers · jobs and budgets
PS  Persistence          formats · editions and migration · import and export
    Pillars              evidence · human-solve boundary · one state · determinism, budgets and sandbox · renderer and platform independence
```

How the layers sit in the charter's six layers:

- The runtime (LG, running LB) is a client of the command layer (KN5). It reads revision-bound snapshots through queries
  and changes the session only through command kinds.
- The runtime stays off a turn's critical path (section 9). A key bound to a kernel command row never waits for the
  interpreter, and the renderer never waits for program output.
- The runtime publishes view-model content (EV3). A renderer adapter draws it, and the language never calls the renderer.
- Each layer uses only the layers below it. The kernel never calls the language, and an independent checker (KN6) never
  imports the producer it checks.

## 3. KN: the kernel

Users cannot program the kernel. It is written in the engine language and kept small. The differential oracles gate it:
the J1 harness in `research/jumbling/differential/`, and the 600-cell-Full oracle in `tools/oracle/` (reference cases
O01 to O07 from the 0.4 reference, an independent primitive replay and the comparator, with `tests/test_oracle.py`),
specified in `docs/progress/1.0/oracle/`. No 1.0 candidate writes oracle traces yet. The language adds requirements to
the kernel, not a second engine.

| Component | Owns | Never |
| --- | --- | --- |
| KN1 Model registry and identity | Model identities (the 600-cell-Full manifest, the jumbling puzzle's J1 identity); menu identity, menu-order digest and contract revision; the canonical ID scheme for cell, grip, cap, generator, menu letter, piece, sticker, slot, position, pose and orbit. The identity tower, which differs by puzzle and whose levels form a partial order, not a chain: the jumbling puzzle has pose, sticker class g_A·S_A (the first draft's "visible pose class"), labels (defined on the lattice only), region class g_A·R_A, position (lattice only), coset, tree data and heights, and shape, with the exact maps between the levels and their kernels; 600-cell-Full holds labels only, and positions derive from them. Snapshots, derived states, digests and predicates name their puzzle and level. A projection to a coarser level is an explicit kernel call with a typed partial result ("off the lattice: k pieces"), never a sentinel. Records of derived models; one normalisation of digest byte conventions. | Changes a model without a new identity and a migration; reinterprets a record of another identity; moves a claim along a map that does not preserve its subject; lifts a value from a coarser level to a finer one |
| KN2 Exact engines | 600-cell-Full: the label permutation of 259,800 slots by 1,200 generators; words and recipes (word and star steps); collateral support; Net and Strict protection. Jumbling: exact poses over Q(√5); admissibility with certificates; the outcomes admissible, blocked, invalid and uncertain. Batch kernels over words, compressed words and all pieces. The calls and per-revision batches that decide something are typed by puzzle and level, each with an oracle agreement test: projections, protection, the end tests, ν, and heights with domain IDs per distinct pose (equal to LB3 and to the research's `trees.py`). A value store for derived states, by digest. | Uses floats on a decision path; changes state on a refusal; reuses a result at another revision unless its input digest is the same or an exact transport justifies it (KN7) |
| KN3 Exact arithmetic | Unbounded integers (pose entries exceed 2^64), rationals, Q(√5) and the finite fields F2 and F5. Exact matrices: Cayley rotations; the state contract's A1 input map and the menu-mode drag choice (LX-9). The state contract's A2 filtered sign test with its proven bound. Exact polytope operations. The small exact algebra above this (quaternions, the icosian order and its tree, rows over F2 and F5) is library code (LB1). | Rounds silently |
| KN4 Session store | The journal (parent-linked events, branches kept), revisions, snapshots and checkpoints, jumble episodes, recovery, atomic writes, workspace records. Only the engine opens the session files. | Exposes a writable state to a program; is opened by another process |
| KN5 Command layer | The command table (kind, gates, preview, undo class, contexts, repeat, job). Previews that own an immutable witness (R-10). A commit's user-action token is bound to the shown preview's ID and to the digest of its literal letters, with the model identity, menu identity, menu-order digest and contract revision (INV-3). At commit the revision, the literal-letter digest, the full-state hash and protection are rechecked. Durable command IDs with a request digest, at most one commit per ID, and an "unknown outcome" result. `invoke` returns a receipt at once, and admission is one compare-and-set per lane, keyed by the command ID. One availability table per revision, computed by the engine. For a command the language defines, availability combines the checker's static verdict with the engine's availability of the kernel rows the command may invoke, so the engine runs no language code. Query rows for the journal, revisions and checkpoints. Typed errors in ten categories. | Commits without a token bound to the shown preview; binds a token to a normal form; adopts a state or digest a program supplies; repeats a commit on key repeat; reuses a preview across revisions |
| KN6 Independent checkers | One small checker per certificate kind: admissibility and straddlers, macro certificates, compressed-word expansion, ν and label checkpoints, research archives. A checker recomputes every claim letter by letter without caches. It shares no search or construction code with the producer; it uses the kernel's exact engines and model files (KN1, KN2), which the oracles test separately. It emits a per-claim trace that the debugger steps through (EV2). Its mutation gate feeds tampered records and wrong start tables, which must fail. | Imports the producer's search or construction code; trusts a producer's verdict |
| KN7 Jobs and budgets | Atomic admission (R-16) and cooperative cancellation. Deterministic budgets (INV-10). Outcomes typed as completed, cancelled, cut off or error, bound to an input digest and a revision. A job that did not complete yields no negative claim, and a witness it found counts once checked. A commit or undo cancels the jobs that read the revision it replaces (DT-M1 Q5 (a)). Whether a job whose input is pinned by digest finishes like a file job is owner question LQ4. Its result reaches a newer revision only by a re-run at that revision or by an exact transport that a lemma justifies. | Runs on the UI thread (R-14); reports "empty" for a job that did not complete; runs two mutations at once; refuses undo while a program runs (DT-M1 fix-first item 3, still open in EN4) |

## 4. LG: the language core

| Component | Contents |
| --- | --- |
| LG1 Syntax and editions | Surface syntax with an edition header per file, a formatter and error recovery. The style is open (LQ13). The typed IR is the contract, so the syntax can be chosen by a learnability test. |
| LG2 Types | Algebraic data types, generics and inference, plus the domain families of 4.1. The checker enforces six kinds of discipline. (1) Identity types bound to a model and to a level of the identity tower: a 600-cell-Full piece and a jumbling piece are different types, and so are a pose and its sticker class. (2) Exact and approximate values kept apart. (3) Cost units typed by puzzle that never mix: menu letters, retained generators and UI actions. Cost units are part of the reproducibility key and are written into every run record, so "twist" gets one meaning per puzzle in the glossary before the data core freezes. Seconds are a display unit, never a budget unit. (4) A reachability claim on every state. (5) Mode-typed operations: retained-workflow tools need Jumble off, and menu words need Jumble on. (6) Verdicts with evidence where a boolean would hide Unknown, and goals as predicates typed by level. |
| LG3 Effects and capabilities | Effects: pure, read a snapshot, change the view, change the workspace, prepare a preview, use a granted file, seeded randomness, budgets. Effects are inferred, checked and inherited through calls. Rules: no program effect commits, because a commit comes only from the shell's token (INV-3). Views get no workspace-write effect, so a view never makes an undo stale (DT-M1 Q4 (a)). A workspace change happens only inside a command the solver invokes, as one undoable action. Flow control in the solving profile (BM2) takes the form that owner question LQ21 chooses: provenance labels that follow data flow and control flow, or capability separation, under which a program either reads state-derived data or prepares previews. Modules declare capabilities, and the host grants them per profile and per run. Static checks never replace the engine's run-time revalidation. |
| LG4 Typed IR and interpreter | A typed domain IR shared by code, GUI and keys, versioned as the stable contract. A deterministic interpreter, with no machine code, own VM or JIT before measurement. Bulk operations lower to kernel batch calls, so a program never walks every sticker in the interpreter. Compressed words stream. Views evaluate incrementally per revision and skip to the newest revision (latest wins). Budgets and cancellation points are enforced. Pointwise contracts run in the interpreter and are charged to the step budget; a contract over a whole state runs once per revision or as an agreement test in CI, never per call inside a loop. |
| LG5 Modules, packages and versions | Modules addressed by content digest. A manifest (supported model identities, capabilities, edition, dependencies) and a lockfile. Every run record names its module digests. Sharing in 1.0 is owner question LQ12 (recommended: local only). |

### 4.1 Domain type families

These are placeholders to show coverage. The names are not final.

- Model and identity: model, cell, grip, cap, generator, menu, menu letter (grip, item), piece, sticker, slot, position,
  pose, sticker class, region class, orbit; the levels of the identity tower.
- State: snapshot (read-only and bound to a revision), derived state (with derivation, base revision and reachability
  claim), difference.
- Words:
  - 600-cell-Full words of signed generators, where free reduction gives the equality key;
  - jumbling words of menu letters, which are partial maps on states: their equality key keeps each word's domain, so
    letters merge and never vanish;
  - for both puzzles a normal form is only an equality key. Reducing or merging deletes the states between letters, and
    Strict protection and the peak height are evaluated on exactly those states, so preview, protection, commit and
    the token use the literal letters;
  - recipes (word and star steps);
  - compressed words (straight-line programs);
  - expansion with exact length and cost.
- Operations: proposal, preview (the engine's witness), user-action token, receipt, outcome (admissible, blocked,
  invalid, uncertain), typed error.
- Claims:
  - claim, evidence, certificate, premise, scope, domain;
  - goals: exact pose, sticker test, lattice placement, low height, retained checkpoint and label checkpoint, which are
    different predicates with stated implications;
  - reachability claim: witnessed, by theorem without a witness, promised, refuted or open.
- Representations: chart, projection, section, layout, atlas, action, embedding, relation, graph, tree, cell complex,
  state graph.
- Interface: view, panel, control, overlay, display plan, display instance, source map, command, binding, recording.
- Research: search space, search record, strategy, rank, budget, run record, branch.

## 5. LB: libraries and tools written in the language

Libraries are written in the language over kernel calls. Where a decision-path function needs speed, it becomes a kernel
batch operation with an oracle agreement test, and the library function that calls it stays the documented, viewable
entry.

| Component | Contents |
| --- | --- |
| LB1 Base library | Collections and relations; graphs, trees and AND/OR graphs. Fixed points over finite lattices. Exact linear algebra over Z, Q, Q(√5), F2 and F5. The quaternion algebra over Q(√5) with the icosian order, its 2-adic valuation and the 5-regular tree, for heights, branches and domains (cosets γK⁺). Permutation groups (orbits, stabilisers, cycle types, Schreier tables, primitivity tests). Words, normal forms (trace normal forms included), conjugates, commutators and powers; on jumbling words these are total on words and partial on states. Straight-line programs. Exact bipartite matching and integral flows. Big integers and symbolic bounds (binomials, digit counts). Budgeted enumeration with a completeness report. |
| LB2 600-cell-Full library | Orbits and stage order. Seeds, stars, setups and buffers (guarded); frames as exact slot tuples. Collateral and support. Protection: Net, Strict, block and position locks; an unchecked part is never shown as safe. Residuals, blocks and work sheets. Macro analysis. The orbit-first workflow of [solving-workflow.md](../../../progress/1.0/solving-workflow.md). |
| LB3 Jumbling library | Menus (A4₀ ⊆ Λ, I_a). Admissibility classes and certificates; straddlers; blocked twists. N, D and Φ; heights and n₂, n₄, n₆, branches and domains through the icosian tree (LB1; the per-revision batch is KN2's); units; clean steps; excursion along a word. ν over F2 and F5, and the 36 corrections; solving for a correction combination is research tier only (section 11). The three separate end tests: retained checkpoint, label checkpoint, sticker test. The Jumble on and off rules. Protection verdicts that carry their level: pose-based inside Jumble, label-based in 600-cell-Full, never crossing over. The repair list at a cap the solver names (17 October). Shown facts follow DT-J1 rule 6. |
| LB4 Representations library | Exact geometry and display geometry. Charts with a declared domain, inverse and loss. A chart's declared properties are decided exactly at exact camera poses (the identity or an element of K⁺): domain by an exact region test (KN3's polytope operations), injectivity by a theorem or an exact region test, and section membership on the posed vertices, to which convexity reduces it. A per-vertex test can refute a region property and cannot establish one (DT-L1 R3-M2). Between those poses, whether a point lies in the chart's domain is a display decision that decides nothing (INV-4). Per-object layouts are evaluated per revision; how projection charts are evaluated is LQ7 (8.5). Projections: perspective, stereographic, local chart, Hopf rings, two hemispheres. True sections, slabs and clips, decided exactly and then drawn. Atlases; trees; cell complexes with dimension-tagged incidence; state graphs. Declared properties checked separately: homomorphism, injectivity, embedding, isometry. Linked selection across representations by canonical ID. |
| LB5 Research library | Restricted search with deterministic budgets. A search record names its family, alphabet, bounds, beam, seed and stop reason. A negative result is scoped to its declared search, with closure-set certificates where they exist. Search reductions such as minimality and dynamic targets are recorded as lemmas, so a reduced search's negative reaches its full family only through the lemma. Strategies, selectors and ranks, with checked strict decrease. Theorem schemas: hypotheses decided on an instance and the conclusion replayed. Conditional models: regional profiles, quotients, partial actions. Admitted domains, with rejection outside them. Family recognition by exact matching. Experiment branches and run records. Certificate builders. Replay and import of the research formats (10.3) through PS3's import table. |
| LB6 Built-in tools | The tools 1.0 ships, written with LB1 to LB5 where possible. Each is viewable and forkable, ships its principle record (INV-12) as data in its module manifest and declares its boundary class (BM5). The record's instance, whose steps a person can check by hand from named givens, runs as a headless fixture. Its decomposition is generated from the IR's call graph and kernel calls, and its lemma or certificate resolves to an evidence envelope. A person writes what a generated graph cannot give: a step map, the reason, the failure of a dropped hypothesis, what an answer does not mean, the derivation, the prerequisites and the leaf specifications. A fork inherits no evidence. |
| LB7 User modules | The solver's tools, macros, recipes, layouts and bindings, and forks of built-in tools, in a local workspace. |

## 6. EV: the environment

| Component | Contents |
| --- | --- |
| EV1 Editor services | Parsing with recovery, completion, diagnostics (types, effects, exactness, evidence, model identity), hover with docs and evidence, go to definition, rename. The last valid version of a tool keeps running while its draft is broken. |
| EV2 Debugger, replay and profiler | Stepping through the IR and inspecting values. Deterministic replay of a run from its record. The provenance graph of a result, expanded down to primitives on demand; a long step is cited by word digest and positions. A step-through of the independent checker's per-claim trace. A cost and budget profile. Read-only travel through the journal. |
| EV3 Source and UI linkage | Declarative views, panels, controls and overlays as view-model nodes ("code grows into UI"). Program output carries its base revision and shows the stale state (charter D2) until it is recomputed. A derived state appears only through a display plan that names its derivation and base revision. Picking returns a display instance, which maps to a canonical ID and a source location. Round trip only for declared parts: parameters, layouts and defaults. The relationship strip's statements are typed relations of the same grammar, so code, strip and theory book share one grammar. Each relation carries its level and is computed per revision, never per frame (R-19), and "none: off the lattice" is a value distinct from "unknown". The strip's data contract can join EN3's view-model fields now; its design waits for the design line. |
| EV4 Commands, bindings, macros, recording | A command the language defines becomes a command-table entry and appears in the palette (charter E5). Bindings follow DT-M1: (key, context) maps to a command ID with absolute arguments, and a key bound to a kernel command row dispatches without the interpreter. Portable layers hold physical key codes only, with prefix-code checks and checks per layout. Only camera, list-movement and preview-scrub keys repeat. Macros are words with model and menu identity, in one macro base for both puzzles. A recording stores command IDs, arguments and canonical IDs, never pixels, and can be turned into source. |
| EV5 Docs, explanations, learning path | Reference docs generated from source. The principle record of each built-in tool, with a provenance view down to primitives and stage-qualified lemmas. The learning path: use tools and read their results, change a parameter, save layouts and bindings, compose recipes and macros, read a principle record and state your tool's principle, write tools, maintain modules and evidence. Competence checks with soft disclosure. Terms from the glossary, checked by script (charter bar M1). The glossary first gains an entry for every language term: position, pose, sticker, word, recipe, certificate, menu, the evidence labels, the jumbling terms, and "twist" for each puzzle. The mathematician oversees this component. |

## 7. PS: persistence and exchange

| Component | Contents |
| --- | --- |
| PS1 Formats | Source (UTF-8 text with an edition header); module manifest and lockfile; word and macro; recipe; binding and key map; layout; filter; preference; checkpoint with its replay path; recording; run record; evidence and certificate envelope. Words, macros and workspace records carry a provenance field (BM2). Data is canonical JSON, gzip-compressed where large, with SHA-256 digests over normalised bytes and size limits. |
| PS2 Editions and migration | Editions for source and schema versions for data. One-way, recorded migrations. A saved meaning never changes silently. |
| PS3 Import and export | The 0.4 import (path B) of every user-data target (section 13). Imported 0.4 evidence enters as recorded: macro-certificate states A and B are recorded claims until the 1.0 engine re-verifies them; state C is never persisted, as in 0.4; state D is the preview token and stays outside the evidence fields. 0.4 macros enter labelled as the solver's own, imported (BM2). Research archives are read and checked independently, through a versioned import table keyed by (format, field, value): a value the table does not map is imported as recorded, with its text kept word for word, and each envelope cites its source bytes by digest. Evidence bundles exported for independent checking. No network. |

## 8. Pillars across the layers

### 8.1 Evidence and certificates

This is the recommended option of LQ3, from the mathematician's cards M1 and M2. The model keeps four questions about a
result apart, and records how the job that produced it ended as a fifth, separate fact.

| Field | Values |
| --- | --- |
| Claim: how true | proved; conditional (named premises); refuted; open |
| Scope: about what | The set the statement covers: one finite instance; a declared search set (family, alphabet, bounds); an admitted domain such as D[k]; or every reachable state of a named puzzle. Its identity: model identity, menu identity and menu-order digest, contract revision, input digest, where the statement has one, with the level it covers (labels or poses; off the lattice poses only, until a sticker-class digest exists), source and module digests. Every scope field is immutable, and a digest is at least as fine as the statement's subject level. The context of a check (revision, protection set, goal and role) is not scope: it serves only claims in memory and the display, and it is never stored. |
| Basis: on what | theorem (a cited proof, qualified by stage and pinned to a source commit); certificate (kind and digest); exhaustive computation over a declared set; exact computation on finite inputs (the research's "measured"); sample |
| Check: who confirmed | none; producer; independent checker (KN6); model review; human review; formal proof. Each check records its mode: reused through exact transport, fresh exact audit or actual replay. The modes are facts, not a strength order. |
| Job outcome (KN7, not evidence) | completed; cancelled; cut off; error |

Rules (draft; the full text states them, with proofs where they need them):

- Evidence is stored as an AND/OR provenance graph.
  - A stated partial order on (basis, check) pairs compares its leaves.
  - One argument is summarised by its set of weakest leaves.
  - Independent arguments for one statement all stay unless one dominates another, so a statement's summary is a set of
    alternatives. Nothing adds up.
  - A stored record also holds seven facts (DT-L1 R3-M1): statement form, negation pair, subject level, scope-membership
    evidence, completeness of a search apart from the job outcome, premise class, and the tool digest of each check with
    the declared common base.
  - What the stored envelope holds is part of LQ10: facts, with the order, the summaries and the five words derived as
    versioned functions (DT-L1 R2-E1), or the judgements themselves (DT-L1 M2). Section 13 gives both.
- A premise is discharged only by a claim with the identical statement and scope. Identity is by statement ID: a
  stage-qualified, commit-pinned name or the digest of a typed statement. An equivalent statement under another ID is
  refused.
- Refutation is asymmetric.
  - One exactly checked counterexample refutes a universal statement, however it was found, given checked scope
    membership and identical identity fields.
  - An existence statement is refuted only by a proof or a complete certificate.
  - A mixed statement is refuted only by a witness plus a complete negative at that witness.
- A negative is proved only for its declared set, by a completed exhaustive search, a closure-set certificate or a
  theorem.
  - A cut-off search reads open and keeps its search record.
  - `OUTSIDE_CERTIFIED_DOMAIN(k)` refutes membership in D[k]. It says nothing about reachability or solvability.
- A job that did not complete yields no negative claim. A witness it found counts once it is checked. A job that
  completed is not a search that completed: completeness or closure is stored apart.
- Sampling never upgrades a universal claim. A checked witness found by sampling proves an existence statement, because
  its basis is exact computation or certificate.
- A claim about a pinned input reaches another revision only by a re-run at that revision or by an exact transport
  that a lemma justifies.
- A result on a derived state keeps that state's reachability claim.

Display labels. The owner's five words are computed from these fields for display: proved, conditional, finite
instance, not found within budget, incomplete. Refuted and open are shown as well.

Provenance travels with the evidence as data: author, reproducer, scope setter and review class, as stage 18 records
them per theorem.

Existing labels map onto these fields without loss only by format, not by word. The same word means different things in
different records. Of the 417 `.json` files of stages 14 to 19, 156 have no top-level `format` key and 146 have none at
any depth; counted with the 80 `.json.gz` files, 232 of 497 records have no top-level `format` (DT-L1 R3-E2). The import
table of PS3 maps by (format, field, value), and the full text gives the mapping.

These label families all go through that table, and a word can map differently by format:

- The research labels: proved, certificate, computed, measured (an exact computation on finite inputs) and open.
- The stage 14 to 18 labels: computed, lead, reported and conditional. "Reported" (a program that is not committed) is a
  replayability fact and stays visible.
- The command layer's states: verified, recorded, unknown and unchecked.
- The 0.4 macro-certificate states:
  - states A and B map onto the fields as recorded claims until the 1.0 engine re-verifies them;
  - state C's context (state, orbit, role, protection set, goal) is a check context, held in memory and never
    persisted, as in 0.4;
  - state D is the preview token, a permission, and stays outside the evidence fields (INV-5).

The `AGENTS.md` evidence kinds for engineering claims (source or fixture, synthetic, actual Windows/DirectX, performance)
are a separate field.

### 8.2 Human-solve boundary

Every option shares the mechanisms of section 11. Built-in tools keep the 2 October boundary and the 17 October
exception, and DT-J1 binds as settled:

- No built-in tool chooses, ranks or lists solving twists, except the repair list at a named cap.
- None shows unlock lists, existence answers, correction combinations, a target cap the program picks or a default sort
  of caps. Every item that DT-J1's outcome put out by the filter stays out. This includes a "next cap" key that skips
  caps with an empty list, a job started for a cap the solver has not named (a prefetch along the solver's order is a
  search across caps), and hot-cold signals.
- A fact a built-in tool shows follows DT-J1 rule 6.
  - It is static: defined at one revision, with no quantifier, minimum or argmax over twists or words.
  - It is shown per cap or for what the solver names, never as the result of the method's own selection (largest, top,
    first).

### 8.3 One state

- Programs receive read-only snapshot handles bound to a revision.
- A derived state is computed by the same kernels and held in the kernel's value store. It has no session revision,
  appears only through a display plan that names its derivation, and is never adopted as the session state.
- A commit adopts only the engine's own preview object, named by the token (INV-3).
  - A headless test swaps the pending preview between its display and the commit.
  - The commit must be refused.
- Every result names the revision it read. After a commit it shows the stale state (charter D2), and a preview is
  rechecked at commit.
- The view model has one mode and holds no puzzle copy.
- Branches are an owner question (LQ4).

### 8.4 Determinism, budgets and sandbox

- The reproducibility key is: source and module digests, edition, IR version, model identity, menu identity and order,
  contract revision, input snapshot digest, parameters, seeds and budgets. The same key gives the same exact result,
  bit for bit, on Windows and on Linux.
- Budgets are deterministic (INV-10). Only such budgets give a negative a reproducible scope, so a run stopped by
  wall-clock time reads open.
- Display floats are held only to their declared precision. Display plans depend on the maths library too (INV-4), so
  LX-3 compares them across platforms at that precision.
- Builders follow the research's practice: sealed seeds, no clock, one thread, an isolated interpreter, outputs outside
  the source tree, digest-keyed caches that fail closed, and atomic writes.
- Capabilities are granted per profile.
- "Empty", "not found within budget" and "not run" are different results.

### 8.5 Renderer and platform independence

- Display plans are declarative: objects, charts, overlays, token colours. The renderer adapter compiles them.
- A user chart in the projection chain needs the GPU (DT-L1 E5).
  - During a camera rotation in four dimensions or a turn animation, every vertex gets new input in every frame.
  - At full detail that is 18,288,000 vertices per frame, counted from `assets/mesh.json` (source).
- The recommended option of LQ7 follows from this: a chart subset that is pure and total, uses float32, has bounded
  loops and has no recursion, allocation or big integers. It compiles to portable HLSL, and the CPU interpreter is its
  reference evaluator.
- A chart's declared properties are decided exactly at exact camera poses, by an exact region test or a theorem (LB4).
  Between poses, domain membership is a display decision.
- The shader test compares the GPU output with an exact or interval evaluation on the declared domain and excludes a
  stated neighbourhood of singular sets. PresentMon in LX-7 stays the performance evidence.
- Per-object layouts, such as sheets, atlases, trees and state graphs, are evaluated on the CPU per revision.
- Picking uses display instance IDs and source maps, never chart inverses.
- Headless runs produce the same display plans for tests.
- The display-plan primitives go to the renderer line as a handoff before its freeze (LQ20).
- The dimension is a parameter, and only d = 4 is used in 1.0.

## 9. The runtime's interfaces to the six layers

| Layer | The runtime uses | The runtime never |
| --- | --- | --- |
| Engine | Read-only snapshot handles at a revision; batch queries; derived states from pure word application, in the value store; certificates; checkers | Calls the engine from the UI thread; holds its mutable state |
| Session store | The journal, revisions and checkpoints, through the command layer's query rows; workspace records through workspace commands | Opens the session files; writes directly |
| Command layer | Proposal with its provenance (BM2), then the engine's preview, then a commit by the shell's token bound to that preview; undo through the journal; receipts; typed errors; availability | Bypasses a gate; commits; supplies a state or digest to commit |
| View model | Publishes view nodes, display plans and source maps per revision, each with its base revision; shows "applied, display out of date" (R-02) | Holds a puzzle copy; delays the adoption of an engine revision |
| Shell | Hosts the editor (EV1 to EV3) and key bindings (EV4); may link the checker as a library for editor latency | Runs programs on the UI thread |
| Renderer | Nothing directly; display plans reach it through the adapter | Decides legality; is called by a program; waits for program output |

The runtime stays off the turn's critical path (DT-L1 E1):

- A turn's interval (charter E2) runs from the key through the command layer, the engine's apply, the store's durable
  transaction and the bundle's adoption to the frame. The runtime is not on it.
- A key bound to a kernel command row dispatches without the interpreter or its jobs.
- The renderer adopts each engine revision bundle without waiting for program output.
- Program views that recompute per revision skip to the newest revision, so a burst of twists never queues view work.

Where the runtime runs is open and is decided with evidence (LQ9, LX-2). There are three options:

- an isolate inside the engine process;
- a process of its own;
- the shell process. This carries the most risk: user programs would share the crash and memory domain of the window
  and the renderer. It is admitted only if it passes LX-2's fault-injection run.

Change notification stays advisory, so a client converges by reading the revision delta (`10_V1_ARCHITECTURE.md`).

## 10. Coverage

### 10.1 The kickoff's questions

| Kickoff item | Components | Owner question |
| --- | --- | --- |
| Language core, types and effects | LG1 to LG5 | LQ1, LQ13 |
| Standard library | LB1 to LB5 | LQ16 |
| Built-in tools in the language | LB6 | LQ5 |
| User modules, packages and versions | LB7, LG5, PS2 | LQ12 |
| Editor: completion, diagnostics, debugging, source and UI linkage | EV1 to EV3 | LQ6 |
| Macros, bindings, recording | EV4 | LQ11 |
| Docs and learning path | EV5 | LQ14 |
| Interfaces to engine, store, command layer, view model, renderer | KN1 to KN7, section 9 | LQ9, LQ20 |
| Preview and commit, undo, cancellation, async work, budgets | KN5, KN7, LG3, LG4 | LQ4 |
| One authoritative state | INV-1, 8.3 | LQ4 |
| Exact decisions apart from float display | INV-4, KN3, LG2 | none |
| Identities of piece, sticker, slot, position, pose | INV-6, KN1, LG2 | none |
| Evidence levels; certificates and independent checking | 8.1, KN6 | LQ3 |
| Linked representations; custom maps and charts | LB4, EV3, 8.5 | LQ6, LQ7 |
| Restricted search, strategies, conditional models | LB5, section 11 | LQ2 |
| Experiment branches and reproducible records | LB5, KN4, 8.4 | LQ4 |
| Stage 14 to 19 techniques and formats | 10.3 | LQ16 |
| Boundary mechanism for tools a solver develops | section 11 | LQ2, LQ21 |
| Persistence and compatibility; migration format freeze | PS1 to PS3, section 13 | LQ10, LQ19 |
| Implementation route and engine language | section 12 | LQ8, LQ9, LQ18, LQ21 |
| Phased route | section 14 | LQ19 |

### 10.2 What restoration needs

| Need | Source | Components |
| --- | --- | --- |
| Words of signed generators ±1 to ±1200 in chronological order; recipes of word and star steps (1 to 256 steps, at most 3,000,000 primitives in total, chunks of at most 100,000) | `core.py`, `10_V1_ARCHITECTURE.md` | KN2, LG2, LB2 |
| Stars (setup⁻¹, relocation⁻¹, seed, relocation, setup); seeds certified with full collateral; guarded buffers; exact slot-tuple frames | `core.py`, `research/PUZZLE_THEORY.md` | LB2, KN6 |
| Collateral support per orbit; Net and Strict protection (Strict is a predicate over the executed prefix, so a program is never split silently); block and position locks | `session.py`, glossary | KN2, KN5, LB2 |
| Exact length and cost before any run | `10_V1_ARCHITECTURE.md`, ENG-0J | LG4, KN5 |
| Orbit-first block building, work sheets, residuals | `solving-workflow.md` | LB2, LB6 |
| Menu letters (grip, item) against a pinned menu identity, order digest and contract revision | state contract, ENG-0J | KN1, LB3 |
| Four outcomes with certificates; a refusal changes nothing; straddler sets with independent certificate checks | `research/jumbling/sim/`, `research/jumbling/differential/` | KN2, KN6, LB3 |
| Jumbling words as partial maps: a free reduction can widen a word's domain, so the equality key merges letters and never deletes them; execution uses the literal letters | state contract section 3 | LG2, LB1 |
| Poses apart from stickers: π is not injective on poses; protection inside Jumble is pose-based; a solve ends on the sticker test, not on pose equality | state contract A3 and A5 | KN1, LG2, LB3 |
| N, D, Φ, heights, units, clean steps; ν and label checkpoints; the Jumble on and off crossing | research data, state contract A5 | LB3 |
| ν and the 36 corrections over F2 and F5; heights, branches and domains through the icosian tree | state contract A5, `research/jumbling/theory/restoration.md` | KN2, LB1, LB3 |
| The repair list at a named cap | owner, 17 October | LB3, LB6 |
| Words of any length (2,352,984 literal letters stored flat on record, 224,683,850 composed; DT-L1 R3-E4); scrambles as jobs, because most attempted twists are blocked | ENG-0J | LG4, KN7 |

### 10.3 What the research needs (stages 14 to 19)

Stage 19, from [its archive](../../../../research/jumbling/restoration/19-height-depth-framework/README.md):

| Technique or format | What the language must express | Components |
| --- | --- | --- |
| Full reference pose tables; families Γ(Y) with type-preserving bijections and gauges in K⁺; recognition by exact matching | Pose tables as data; group elements of K⁺; exact bipartite matching; family recognition from the current state. Γ(Y) transfers legality claims only. | LB1, LB5, KN3 |
| Blocks as chronological (cap, menu item) letter lists; continuations; "frontier exits, not restored states" | Words; goals as predicates, so a frontier exit cannot pass as restored | LG2, LB3 |
| Ranks R3, S and R_L as weighted sums with strict decrease on every block; lexicographic descent | Ranks with checked strict-decrease certificates | LB5, KN6 |
| Conditional bounds with named obligations; the reachability promise | Conditional claims whose premises accumulate and are discharged only by identical claims | 8.1 |
| The admitted domain D[k]; finite legal enumeration; rejection `OUTSIDE_CERTIFIED_DOMAIN(k)`, which is not "unreachable" | Domains as scopes; typed rejections that differ from refutations of reachability | LG2, LB5, 8.1 |
| Theoretical bounds of thousands of digits (B11 has 6,775) | Exact big integers and symbolic bounds; words of such length only as straight-line programs | LB1 |
| Peak height along a word (no overshoot) | Prefix-wise observables along words | LB3 |
| Synthetic validations whose reachability is unknown | Reachability claims on derived states | LG2, 8.3 |
| 73 indexed references that are not 73 distinct fibres; reused checks against fresh audits against actual replay | Counting with identity; the check modes of 8.1 | LB1, 8.1 |
| `IMPORT-MANIFEST.json` with stored and uncompressed digests; deterministic gzip; source digest apart from the public bytes | Import with digest checks | PS3, KN6 |
| No executable producers in the archive | Import and check of data now; producers can be written in the language later | LB5, PS3 |

Stages 14 to 18, from their directories under `research/jumbling/restoration/`:

| Stage | Techniques | Formats | What the language must express | Components |
| --- | --- | --- | --- | --- |
| 14 Blocking structure | Exact locality values Δ(c,d) in Q(√5); the precedence peel T3P with its tests; unlock levels as a greatest fixed point; plateau certificates; lemma chains; letter-by-letter replay without caches | The `t3p-*/1` family (certificate, check, scan, pairs, plus, levels, locality, chain); replay rows (letter, applied, n₂, n₄, n₆, maximum height, Φ, off, digest) | Q(√5) values; fixed points; certificates with exact key sets; observables per letter | LB1, LB3, LB5, KN6 |
| 15 Lowering machine | Chronological macro certificates (Theorem MC: one certificate per admissible word; the stored `macro-certificate/1` format accepts only descending macros ending in a jumble); the release structure as an AND/OR graph; a good-set controller; the declared search P* (families, exhaustive depth, beam); closure-set negative certificates; termination | `macro-certificate/1` (claims supports, delta, phi, digests, excursion, F, D), `macro-check/1`, the `pstar-*` records, `pstar2-stop/1` and `pstar2/2` | Φ and excursion along words; certificates recomputed claim by claim; search records; scoped negatives | LB3, LB5, KN6, 8.1 |
| 16 Six jams | Retained transport (Theorem X) with reductions; the budget ledger B = D − F; cover tests; conjugated releases; schema bounds; results that depend on the route; negatives over a stated alphabet; minimality and dynamic targets as search reductions | `s16-*/1` records, `jams.json`, whole-descent `macro-certificate/1` runs; about 61 records without a `format` key | Theorem schemas; results tied to their route; reductions recorded as lemmas; import of records without a format key | LB5, PS3 |
| 17 Routing in the driver | The routing family RR (routing words, helpers, releases, five tail forms); reduced searches gated against brute force; repeated macros bounded by Φ/2 | `s16-rr/1`, `s16-pxrr/1`, `s16-transport/1`; the RR word language | Word languages as values; gates comparing a reduced search with brute force | LB5, KN6 |
| 18 Tail completion | Whole-cap steps X W V′ W⁻¹; hole profiles; a static plan theorem with a layered integral flow; a group lift; the WC-PLAN rule with exact length admission at most 10⁷; Schreier tables and Jordan witnesses; obstructions; provenance per theorem | `steps.json` with per-step word files; `r22b10/1` and `/2`, `runner-run/3`, `literal-lattice/1`, `lattice-phase-play/1` | Integral flows; the permutation-group toolkit; chunked, chain-verified replay of words up to 10⁷ letters; provenance as data; conditional entries | LB1, LB5, KN6, 8.1 |

Needs shared by every stage:

- **An independent check path.** Checkers import no construction code. Mutation gates must fail on tampered input.
- **Scoped negatives.** A negative result holds only for its family, alphabet, excursion or declared search. A resource
  limit means open.
- **Deterministic runs** (8.4).
- **Content identity.** An artifact is identified by its content digest, not its path. Six pairs of file names that differ
  only in case cannot coexist in a Windows checkout
  (`research/jumbling/restoration/19-height-depth-framework/PUBLICATION.md`).
- **A conformance corpus.** The existing certificates and records must still check under the language's checkers.

## 11. The boundary for tools a solver develops: option sets

The principle (owner, 3 November 2026): a tool the solver develops themselves, through the built-in base tools and while
understanding its principle, that assists solving does not count as the program outputting a solution by itself. The
owner and the integrator set the exact boundary together. Until then, built-in tools keep the 2 October boundary and the
17 October exception.

**The facts every option faces** (DT-L1 M5):

- A general-purpose language with read access to the state can express any solving procedure, and a solver can type a
  word computed elsewhere.
- Whether the program chose a word is not a function of (word, state): the identical pair can be the solver's named
  macro or a search's output. For programs without limits, no checker decides whether every output is a solving step
  (Rice's theorem). A budgeted program always stops, so one run is decided by running it, and only a check over all
  states stays out of reach. Budgets make each run finite, but they do not make "searches" a property of what a program
  computes.
- Even a static fact can answer a question over twists. When N_c equals its floor, no twist of cap c alone lowers N_c,
  and nothing was tried. A piece's domain at height 2 fixes the twists at each own holder that lower that piece.
- A composition of shipped functions can answer one too. Generic elimination over F2 and F5, applied to the ν on screen
  and the shipped correction rows, returns the correction combination that DT-J1's filter put out; the research's
  `solve(b2, b5)` already does this (DT-L1 R2-E3). No class of "non-searching" functions defines a safe tier, because
  "searches" is intensional: a lookup table computes the same function. A list that leaves a part out stops the shipped
  composition. What a solver writes from base parts, only flow control stops.
- Flow control covers data and branches, not whether a run ends or emits a preview. A state-dependent stop can select
  among up to 36,000 pending previews, about 15.1 bits per run, and repeated runs add up (DT-L1 R3-M3; chapter 07 gives
  three fixes). A tool that shows a state-derived word for the solver to retype passes any preview check, and DT-J1's
  filter also covers what is shown, so display sinks are a policy question too.
- So the boundary concerns who chose a word, not what the word does. The part of that question software can check is a
  flow property. "Themselves" and "understanding" are not checkable, because two identical flows can differ in what the
  person understood.
- Software can control what it ships, how execution happens, which capabilities a profile grants, which flows inside the
  sandbox reach a preview, and how the records the system writes are labelled. It cannot establish authorship or
  understanding, and nothing it does binds a person who retypes a word.

**Mechanisms present in every option**, so the policy can be set later without a redesign:

- **BM1 Execution gate.**
  - A commit needs an explicit user action on the shown preview (INV-3).
  - One action commits one previewed operation.
  - Nothing commits in the background.
- **BM2 Provenance and flow control.**
  - Every preview names:
    - the program and its module digest;
    - its author class: built-in, own, forked built-in or imported;
    - what it may depend on, as far as the form of flow control records it.
  - An explicit pick by the solver is recorded as a pick.
  - Stored words, macros and workspace records carry a provenance field (section 13), so a word laundered through the
    workspace or a saved file keeps its label. 0.4 macros enter labelled as the solver's own, imported.
  - Declassifiers are fixed kernel or library entries that the policy enables: setup search for a named target, the
    repair pair the solver picks, the solver's own pick, and the author classes the policy admits. No program can
    declare one.
  - A preview's outcome (its end state, N, its protection verdict, a refusal and its error category) is state-derived,
    and so is a read of the budget used. A missing provenance field reads unknown, never own.
  - Flow control takes one of two forms, owner question LQ21:
    - **Flow labels** follow data flow and control flow: statically in the checker, with more refusals and no run-time
      cost, or also at run time, with fewer refusals and a cost per step. A record can say where each previewed word
      came from, indirect choices included, and a preview's outcome carries the state label. Static labels need a
      checker over exactly the source that runs: RT1 and RT3 have one, and RT2 or RT4 can add one over an approved
      subset of the host language that refuses dynamic code, with the approval bound to the executed bytes. Run-time
      labels need the project to own the executor's values: RT1, RT3's compiled label fields, or a modified borrowed
      VM (section 12).
    - **Capability separation** in the solving profile works in every route: a program either reads state-derived data
      or prepares previews, never both, apart from the declassifiers, and a program that prepares previews does not
      read their outcomes. It is coarse: a solver's own tool cannot both read the state and preview, and a record says
      only whether a program read the state.
  - A policy may switch on a flow rule. In the solving profile, a preview's word and target may then depend on the state
    or on computed results only through the declassifiers.
  - Either form is conservative and refuses some harmless tools.
- **BM3 Profiles.**
  - There are three profiles: solving, tool authoring and research.
  - Each grants a set of capabilities and attaches a label.
  - Research on derived states never changes the session state.
- **BM4 Labelled records.** A solve record states which assistance classes the system recorded.
- **BM5 Boundary classes.**
  - Each built-in tool declares its class under the 2 October boundary, and the class is reviewed when the tool ships.
  - The classes are tracking, analysis, checking, protection, view aid, setup search for a named target, and the repair
    list.

**Option sets.** Each keeps BM1 to BM5, the built-in tools' 2 October boundary and the 17 October exception.

| Set | What a solver's own tool may do in a solving session | How "themselves", "base tools" and "understanding" are read | For | Against |
| --- | --- | --- | --- | --- |
| BS1 Provenance | Prepare a preview of any word it computes; the solver commits each one; BM2 records what the preview may depend on (under capability separation, only whether the program read the state) | Author class (BM2): created or forked locally. It records how the bytes arrived, not who thought of them. | Simple; most freedom; honest labels | Base tools and understanding are not checked; a copied solver counts after a trivial fork; the line rests on labels |
| BS2 Base tier | As BS1, but in the solving profile the tool may call only the base tier. The base tier is an enumerated list of shipped functions, each with a recorded definition and a boundary ruling. It follows DT-J1 rule 6 (static facts shown per cap or for what the solver names; no selection by the method; no target the program picks; no existence answer), plus exact application and composition of named words. It ships no function that searches, minimises, ranks or selects over twists, words or caps. The solver may build such a function from base parts. A research tier (ready-made search, strategies), if LQ16 ships one, runs in the research profile on derived states. | Author class as in BS1, plus a capability boundary (LG3) enforced with either form of flow control (BM2); building from base parts is the solver's own development | Matches the principle's wording: the solving profile ships no solver, and the solver builds one from base parts; research stays free | A determined user can rebuild any search from base parts; understanding is not checked; every listed function needs the mathematician's ruling, including shipped tables (certified setup tables, seeds, the 36 corrections) and static facts that a theorem turns into answers about twists; listed functions compose into forbidden outputs (the F2 and F5 solve over ν), so flow control must also cover what a tool shows, or the list must leave such parts out |
| BS3 Understanding | As BS2. In addition, the capability to prepare solving previews opens per principle, after the solver passes the competence check on that principle's record (INV-12) | Author class as in BS1, base tier and competence checks | Closest to "while understanding its principle"; ties into the learning path | Gating conflicts with soft disclosure unless the checks only label; checks can be gamed; writing the checks is substantial work |
| BS4 Named targets | A solver's own tool may prepare previews only for a target the solver names, which extends the 2 October setup-search allowance to own tools; flow control (BM2) checks that the target came from the solver. Anything else runs in the research profile. | Author class as in BS1; naming the target is the solver's own act | Smallest step from 2 October; easy to explain; enforceable with either form: flow labels check that the target came from the solver, and under capability separation the tool reads no state and reaches it only through the shipped setup search for the named target | Restricts assistance more than the principle requires; "named target" needs a definition per tool kind |

Questions for the joint setting of the boundary:

- Does "assists solving" exclude a tool whose single preview completes the puzzle?
- Is a tool written by another person treated as built-in, as own, or as a class of its own?
- Is the flow rule switched on, and which author classes does it admit?
- Which display sinks does flow control check: previews only, or also what a tool shows for the solver to retype?
- Do shipped tables and theorem-backed static facts count as static facts?
- Are labels shown during a solve, or only in records?
- Is the research profile part of the shipped product?

**Interim reading until the joint setting.** This is the integrator's reading; the owner may change it.

- The policy table starts at the 2 October boundary for everything that ships, library functions included.
- In a solving session, no shipped function searches, minimises, ranks or selects (largest, top, first) over twists,
  words or caps, none picks a target, and no shipped job starts for a cap the solver has not named. Shown facts follow
  DT-J1 rule 6. The two exceptions are setup search for a
  target the solver names and the repair list at a named cap.
- Research-profile output never becomes a preview in a solving session.
- No solver-tool solving preview is enabled until the boundary is set.

**Integrator's leaning, for discussion only.** Build BM1 to BM5 in any case, with the form of flow control that LQ21
chooses. Start the joint discussion from BS2 with the flow rule available. The solving profile then ships no solver,
while the solver may build one from base parts, and the line rests on capabilities and flow control rather than on
guessing what a program computes. It does not gate learning. This is not a decision, and it presets neither LQ21 nor the
route.

## 12. Implementation route and engine language

The kickoff asks for options and the evidence each needs, with no preset conclusion.

| Route | What it is | For | Against | Evidence needed |
| --- | --- | --- | --- | --- |
| RT1 Own language | Own parser, checker (types, effects, exactness, evidence, flow labels), typed IR and interpreter, written in the engine language | Full control of the domain types, determinism, budgets, flow labels and messages; bulk kernels; no foreign runtime | Largest effort; editor and debugger from scratch; interpreter speed unmeasured | LX-1, LX-4, effort from a vertical slice |
| RT2 Embedded runtime | An existing scripting runtime with a domain library. Candidates: Luau, Lua 5.4, a JavaScript engine, CPython or a restricted subset, Rhai, a WebAssembly VM. | Mature VM, docs and editors; faster start | Types, effects and evidence become conventions or need a separate checker; run-time flow labels need a modified VM, because a native branch on a plain value is invisible from outside, and static flow labels need a separate checker over an approved subset, whose false refusals are unmeasured (LX-3, LQ21); sandboxing and determinism vary (hard for CPython); host semantics leak (floats by default, integer overflow); licences | LX-2, LX-3, LX-5 |
| RT3 Hybrid | Own front end (syntax, checker, IR) compiled to an existing VM, such as WebAssembly with fuel metering | Own discipline with a mature sandbox and metering; flow labels in the own front end, with run-time labels compiled as explicit fields | Two layers; source mapping for the debugger; target limits (big integers, memory management) | LX-1 to LX-4 |
| RT4 Host-language library | The language as a typed library inside the engine language or Python | Fastest start; all host tooling | No static effect discipline for user code without a separate checker over a subset of the host language, as in RT2; run-time flow labels only with a modified host runtime (LQ21); hard sandbox; users must learn the host language; tied to the engine choice | LX-3 |

- The interpreter's host normally follows the engine language, for zero-copy access to snapshots and kernels.
- The engine language is open: Rust is proposed in
  [engine-language-rust](../../../wiki/questions/engine-language-rust.md), and its spike has not run.
- So the engine language is decided first (LQ8), then the route (LQ9).
- Flow control changes what each route must build. Static flow labels need a checker over the source that runs, which
  RT1 and RT3 have and RT2 or RT4 must add over an approved subset of the host language. Run-time labels need RT1, RT3
  or a modified VM. Capability separation works in every route. Flow checks added after LP2 would refuse saved tools
  that used to run, so LQ21 is decided before LP2 builds the checker and the IR.
- In RT1 to RT3 the typed IR is the contract, so the executor below it can change later without changing what saved work
  means. The front end is the choice that is hard to reverse.
- A proof assistant as the language (for example Lean 4) is a separate question (LQ15).

Evidence spikes run only if the owner allows them (LQ18). Each gets the stage 2 protocol's experiment card before it
runs: the question, the decision it feeds, kill criteria and the evidence class. The decision rules below are sketches
from DT-L1 E4; nothing has been measured.

| Spike | Question | Feeds | Decision rule (sketch) | Evidence class |
| --- | --- | --- | --- | --- |
| LX-1 Interpreter cost | Cost per IR step and per kernel call for the three LP3 tools in each route, with flow labels off, static only, and at run time | LQ9, LQ21 | A headless lead only; a function timer never stands in for charter E2. The decisive check is LP3's end-to-end E2 measurement. | Source (lead) |
| LX-2 Placement | Kernel-call round trip in and out of process at LX-1's call mix; faults (memory exhaustion, deep recursion, an endless loop, a killed runtime); startup; transfer of 1,039,200 bytes of labels and 708,480 bytes of pose IDs per revision; a preview request carries its label set to the command layer | LQ9 | A placement passes when, during each fault, the journal and state hash are unchanged, the shell stays responsive and a commit completes within charter E2. Startup stays within the 3 s that charter E3 proposes. | Performance (owner's machine) |
| LX-3 Sandbox, determinism and refusals | Budgets, interrupts, no ambient input or output; display plans across platforms at their declared precision; must-refuse cases for flow control (caps chosen by largest N_c, a search followed by a rebuild from base parts, the F2 and F5 solve over ν, a word laundered through the workspace, a choice between two words by a state fact, a flow through a module call, a stop-selected preview, refusal-driven selection, a one-item pick, accumulation across runs), run under capability separation, static labels, run-time labels and, for RT2 and RT4, a variant whose static checker over an approved subset refuses dynamic code and binds its approval to the executed bytes | LQ9, LQ21 | One reproducibility key gives identical result bytes over repeated runs on Windows and on headless Linux, and display plans agree at their declared precision; planted clock, file and network reads are refused; every must-refuse case is refused under each form and variant, and the count of false refusals over the built-in tools and the solving-workflow tools, for each form and variant, goes to the owner with LQ21 | Source |
| LX-4 Editor latency | Feedback latency on realistic modules | LQ9 | Bars set in the full text | Performance (owner's machine) |
| LX-5 Exact arithmetic | Speed and licences of big-number and Q(√5) candidates at these sizes: synthetic pose entries beyond 2^64, literal words of 672,417 and 2,352,984 letters, 13,231,266 letters as one stream, the 10⁷ admission bound | LQ8 | A licence list goes to the owner; a copyleft dependency needs the owner | Source; performance (owner's machine) |
| LX-6 Engine language | The deferred spike of the wiki question: port the hot path, check oracle equality, compare speed with NumPy | LQ8 | Oracle equality is source evidence; the speed comparison counts only from the owner's machine | Source; performance (owner's machine) |
| LX-7 User charts | A chart stage generated from the subset against the built-in perspective stage | LQ7 | PresentMon at W3 on the owner's machine, three cold runs per candidate, inside the selection gate; GPU and CPU evaluations agree within the declared display precision (headless) | Performance (owner's machine); source |
| LX-8 Syntax learnability | Two or three syntax candidates over one IR, tested with non-authors | LQ13 | A protocol the owner sets | Human test |
| LX-9 Input-map determinism | The state contract's A1 input map and the menu-mode drag choice, in two steps (DT-L1 M4, R2-E2): the 13,600 requests of R2-E2 on Linux CPython and on the LX-6 engine candidate, comparing target bits with Windows CPython; then requests within one ulp of exactly computed bisectors, comparing the chosen rotations | INV-10 | If the choices differ, the request is recorded as a dyadic Cayley vector read exactly, and a drag compares its 60 candidates exactly in Q(√5). That amends the state contract: EN1's Astra check, then the owner. | Synthetic, on both platforms |

Order: LX-5 and LX-6 feed LQ8. LX-3, LX-7, LX-8 and LX-9 do not wait for LQ8; LX-9's run on the engine candidate
follows LX-6. LX-1, LX-2 and LX-4 depend on the interpreter's host language, so they follow LQ8. LX-1 and LX-3 feed
LQ21 as well.

Performance figures count only from the owner's machine. Headless and cloud timings are not performance evidence.

## 13. Persistence and the migration format freeze

**What the language persists:**

- source with its module manifest and lockfile;
- words and macros, with their provenance field (BM2), under DT-J1:
  - the literal letters, model identity, the order-pinned menu identity, contract revision and start digest;
  - named parts, and the exact expanded length as a big integer;
  - no length cap, but a refusal beyond the bound from DT-J1's experiment X3;
  - jumble letters are refused while Jumble is off;
- recipes;
- bindings and key maps;
- layouts and the declared parameters of tools;
- workspace records, with their provenance field;
- filters, preferences, and checkpoints with their replay paths;
- recordings;
- run records: snapshot digest, module digests, parameters, seeds, budgets, stop reason, evidence;
- evidence and certificate envelopes.

**Relation to the migration format freeze.**

- Under path B, a 1.0 importer reads a locked copy of the 0.4 database. Its targets are:
  - recipes, macro sets and protected orbits;
  - filters and preferences;
  - checkpoints with their replay paths (`session/checkpoint-paths.json`: each event's recipe with its pre and post
    hashes, replayed by the 1.0 engine);
  - 0.4 key maps: twist keys by the word each key typed, commands through DT-M1's alias table.
- 0.4 semantics stay: recipes of word and star steps, 1 to 256 steps, at most 3,000,000 primitives in total, a ceiling
  that 100,000-letter chunks cannot raise. Test: every recipe in the oracle fixtures gives the same replay hash after
  conversion.
- **Proposal (LQ10).** At the migration format freeze, freeze only a language-neutral data core:
  - words and recipes, macro sets and protection sets;
  - filters, preferences, and checkpoints with replay paths;
  - bindings and key maps;
  - the provenance field of stored words, macros and workspace records (BM2);
  - the evidence envelope with its identity and scope fields.

  The freeze must never fix a lossy mapping. LQ10 chooses how:
  - **Facts** (DT-L1 R2-E1). The envelope stores facts and derives judgements: the leaf facts, the AND/OR edges, the
    scope with the level of every digest, and each source's label word for word. The partial order, the summaries and
    the owner's five words are versioned functions over those facts, and changing one is an edition with a recorded
    note. The envelope can then freeze before LQ3 is answered. A test runs through every combination of stored fields
    and checks that each gets exactly one display word or a typed inconsistency. An experiment builds envelopes for the
    stage 14 to 19 corpus (497 records, 13,412,769 bytes at Git HEAD) and reports their nodes and bytes.
  - **Judgements** (DT-L1 M2). The envelope stores the claim fields of 8.1 and freezes only after LQ3 and the rules of
    8.1 are settled.
  - Under either option every scope field of a stored claim is immutable, and none is revision-bound context (8.1).
- The source syntax, the IR and the module formats freeze later and evolve by editions with recorded migrations.
- A stored word keeps its meaning across editions.

Existing formats are reused, not renamed:

- `C600-STUDIO-SESSION-v1` and `C600-STUDIO-CERTIFICATE-v1`;
- `magic600-jumbling-store/1` and `magic600-jumbling-render/1`;
- the research formats of 10.3.

## 14. Phases

The phases have no day counts (the earlier schedule is set aside), and no gate is relaxed. The order is acyclic:

- LP0 settles the direction and allows the spikes (LQ18).
- The decisions that need spike evidence are taken inside LP1, after their spikes and before LP1's freeze gates: LQ8,
  then LQ9 and LQ21. LQ13 follows LX-8, before LP2 builds the parser.
- The charter puts implementation after the whole stage 2.5 exit: the architecture-freeze and migration-format-freeze
  gate rulings, which sit at LP1's exit, and the accepted design-system ADR. Starting LP2 before the design-system ADR
  needs a charter amendment by the owner (LQ19).
- The table follows the recommendations. Under LQ18's alternative the spikes, and with them LQ8, LQ9 and LQ21, move
  after LP1's gates and before LP2.

| Phase | Products | Acceptance | Depends on |
| --- | --- | --- | --- |
| LP0 Contracts | This partition; the architecture document; ADR drafts; glossary entries for every language term. The owner's answers to LQ1 to LQ21; for LQ8, LQ9, LQ13 and LQ21 the answer may be how each is decided, and under the recommendations the choice follows its evidence (LP1; LQ13 before LP2's parser). LQ2's option set follows in the joint setting, before LP6. Once LQ18 is answered, the spikes may start during LP0. | Astra review in at least two shards with no open blocker or major; the owner accepts the ADR drafts that need no spike evidence: every draft of section 17 except the form of flow control in ADR 6, ADR 7 and ADR 12, which follow the route inside LP1 | none |
| LP1 Interfaces and evidence | The spikes LX-1 to LX-9 as experiment code (LQ18), in the order of section 12, and the choices they feed: LQ8 after LX-5 and LX-6, then LQ9 and LQ21 after LX-1 to LX-5, with the form of flow control in ADR 6, ADR 7 and ADR 12 for the owner to accept. The kernel interface specification: snapshot handles, batch queries, derived states and the value store, previews and token binding, receipts, availability, query rows, checkers, jobs and deterministic budgets. Canonical IDs and the identity tower; digest normalisation; the data core formats. The runtime's command-layer needs folded into EN4's final H-08. The display-plan primitives handed to the renderer line (LQ20). | LQ8, LQ9 and LQ21 answered with their spike evidence, and the remaining ADRs accepted; then the architecture-freeze gate ruling (Astra), with the command table, the layer interfaces and the renderer feature list; the migration-format-freeze gate ruling (Astra); the owner's sign-off | LP0, and LQ18 for the spikes. EN3: command rows, view-model fields, jumbling interfaces. EN4: final H-08 with undo available while a job runs, layer interfaces, migration-format packet. LQ10, and LQ3 if LQ10 chooses judgements. |
| LP2 Headless core | The kernel API over the engine and over J1, with a public J1 API in place of the adapters' private members; a candidate adapter through which the existing 600-cell-Full oracle (`tools/oracle/`, cases O01 to O07) runs on the kernel API, with the cases the language adds; parser, checker, IR and interpreter with budgets and cancellation; the conformance suite | The J1 harness and the 600-cell-Full oracle pass on the API. Conformance and determinism tests give identical bytes on Windows and Linux. No float on a decision path, checked. The preview-swap test is refused. Repeated budgeted runs stop at the same step. The centre-0 case is refused at pose level inside Jumble and passes at label level after the crossing. Heights and domain IDs equal the library and `trees.py` on ENG-0J acceptance item 8's states and the H6030 witness. R-11's property test passes with its jumbling case under default 10's hypotheses, and the label-lift conformance test passes with its S, R and σ ≠ id cases. | LP1's gates. The design-system ADR, or the charter amendment of LQ19. The syntax (LQ13) before the parser. |
| LP3 First closed loop | Three built-in tools in the language (600-cell-Full, jumbling, representations) running end to end: source, check, run, view nodes, preview, user commit, receipt, undo | The same result through source, command and key. Oracle equality of every committed step. Charter E2 measured end to end on the owner's machine, p95 at most 100 ms over at least 100 observations (the B4-12 table in `10_V1_ARCHITECTURE.md`), with those tools' views active. | LP2; the frozen H-08; the renderer freeze for the on-screen half |
| LP4 Authoring environment | Editor services, debugger and replay, source and UI linkage, bindings and recording, docs, principle records and the learning path, first edition | Editor latency bars; tests with non-authors in a form the owner sets | LP3; the shell. The editor's look waits for the design line, which resumes after the language (owner, 3 November). |
| LP5 Research expressiveness | Research library; import and independent check of the stage 14 to 19 archives; branches and run records; family recognition; ranks; symbolic bounds | Archived certificates and verdicts re-checked by the language's checkers and matched; a run record reproduced bit for bit; no new search counts as acceptance | LP2 and LP3; the research results in the repository (the research stays stopped) |
| LP6 Product | The 0.4 import of every path B target; packaging; complete docs; the boundary policy as decided; performance gates on the owner's machine | Owner sign-off | Session store and migration line; renderer selected and frozen; the design-system ADR after the design line (LQ19); owner gates |

## 15. Owner questions

Each question gives a recommendation and the alternatives. The owner decides. Items the integrator can default are in
section 16.

- **LQ1 One language or several.** Recommended: one language with three profiles (solving, tool authoring, research)
  that differ only in granted capabilities and labels, so research and product share types, evidence and identities.
  Alternatives: a separate research language (for example Python notebooks) beside a small product language; or a
  product-only language with research left outside.
- **LQ2 The boundary for tools a solver develops.** Recommended: build BM1 to BM5 now, with the form of flow control
  that LQ21 chooses, and choose an option set together after the round table has explained them; the joint setting also
  decides which display sinks flow control checks. Meanwhile the interim reading of section 11 holds. Alternatives:
  choose BS1, BS2, BS3 or BS4 now; or change the interim reading.
- **LQ3 Evidence model.** Recommended: the fields of 8.1. The claim is proved, conditional, refuted or open; scope, basis,
  check and the job's outcome are kept apart; evidence is a provenance graph; the owner's five words are computed from
  these fields as display labels. Alternatives: one claim axis with the five words plus refuted and open (the first
  draft); or one linear scale.
- **LQ4 Branches, the single state and research runs.** Recommended: an experiment branch is a journal branch the engine
  owns; exactly one line per session is active; other branches can be viewed and compared but not changed by programs;
  derived states are values. A research job whose input is pinned by digest finishes after a commit, as file jobs do
  under DT-M1 Q5 (a); its result reaches a newer revision only by a re-run or an exact transport that a lemma
  justifies. Alternatives: no branches in solving sessions, so research uses separate research sessions; or pinned jobs
  cancelled like the others.
- **LQ5 Built-in tools in the language.** Recommended: every tool above the kernel is written in the language, with
  viewable and forkable source; the exact engines and checkers stay in the kernel. Alternatives: native built-in tools
  for speed, with the language for user tools only; or a per-tool rule.
- **LQ6 Views and charter D8.** Charter D8 says Global and Local are the only two views. Recommended: in the solving
  profile, user charts and overlays live inside Global and Local; in the research profile, research views (for example a
  cell sheet) may open in a research workspace, each with its own gate run. Alternatives: D8 strictly everywhere; or free
  views everywhere, which reopens an owner design decision.
- **LQ7 User charts and the renderer.** Recommended: define now a chart subset that compiles to portable HLSL (pure,
  total, float32, bounded loops), with the CPU interpreter as its reference and exact checks of each chart's declared
  properties. Put a generated projection stage into the renderer feature list before the renderer freeze, and enable
  user charts when LX-7 passes; until then, parameterised built-in charts. Alternatives: built-in parameterised charts
  only in 1.0, each measured in H-06; or a shader language for users.
- **LQ8 Engine language.** Recommended: run LX-5 and LX-6 and decide the engine language inside LP1, before the route,
  because the interpreter's host follows it; LX-6's speed comparison counts only from the owner's machine.
  Alternatives: decide Rust now; or keep the exact Python and NumPy reference as the 1.0 engine.
- **LQ9 Implementation route and runtime placement.** Recommended: keep the typed IR as the stable contract, and decide
  among RT1 to RT4, and where the runtime runs, inside LP1 after LX-1 to LX-5 and together with LQ21, because the form
  of flow control changes what each route must build; the shell process is admitted only if it passes LX-2's
  fault-injection run. Alternatives: decide now for an own interpreter (most control) or an embedded runtime (fastest
  start).
- **LQ10 Persistence and the migration format freeze.** Recommended: at the migration format freeze, freeze only the
  language-neutral data core of section 13, which covers every path B import target and the provenance field, with an
  evidence envelope that stores facts and derives judgements (DT-L1 R2-E1), so the freeze need not wait for LQ3; syntax
  and IR freeze later. Alternatives: an envelope that stores judgements and freezes only after LQ3 (DT-L1 M2); freeze
  the source syntax as well (early and risky); or leave language data out of the freeze, so migration targets an
  interim format.
- **LQ11 Recording.** Recommended: a recording stores command IDs, arguments, canonical IDs and revisions. It becomes
  either a macro (one commit) or a step-by-step playback with one user action per commit, and never plays commits
  unattended. Alternatives: allow unattended playback of view and workspace steps only.
- **LQ12 Sharing modules in 1.0.** Recommended: local only, through files the user chooses, with digests; no registry
  and no network. Alternatives: a curated registry after 1.0; or signed packages.
- **LQ13 Syntax style.** Recommended: decide after the learnability test LX-8, before LP2 builds the parser.
  Alternatives: choose now between an expression-oriented, a Python-like or an ML-like syntax.
- **LQ14 Learning gates.** Recommended: soft disclosure as in the charter; competence checks guide and do not lock,
  unless the LQ2 decision uses them as gates (BS3). Alternatives: hard gates per level.
- **LQ15 Formal proofs.** Recommended: no proof assistant in 1.0; certificates and independent checkers carry the
  evidence; a proof-assistant link is a 1.x option. Alternatives: Lean 4 as the research language; or export of proof
  obligations to Lean.
- **LQ16 The language's 1.0 cut.** Recommended: 1.0 ships the five layers as partitioned here, including the research
  library and the checkers for the research archives (in scope since 3 November); module sharing and formal proofs come
  after 1.0. Alternatives: a smaller cut without the research library (this conflicts with the 3 November scope); or a
  larger one. The cut also settles whether the theory book, limited to what the interface shows (EV5.11), covers the
  language's libraries.
- **LQ17 Name.** Recommended: keep "C600 Script" with the extension `.c6s` as a working name until the syntax test; the
  owner names the language. Alternatives: name it now.
- **LQ18 Evidence spikes in this stage.** Recommended: allow LX-1 to LX-9 as experiment code under `work/experiments/`,
  each with an experiment card and the decision rules of section 12, decisive timings on the owner's machine, no product
  code and subscription quota only; they may start during LP0. Alternatives: no spikes before the architecture freeze,
  so the frozen interfaces must hold under every route and both forms of flow control, and LQ8, LQ9 and LQ21 are
  decided after the freeze, before LP2.
- **LQ19 The freeze gates and the charter's stage 2.5 exit.** The charter's exit has three parts: the
  architecture-freeze ruling (with the command table, the layer interfaces and the renderer feature list), the
  migration-format-freeze ruling and the accepted design-system ADR; implementation follows the whole exit. Under the
  3 November hold the design line waits until the language's development is complete, so the design-system ADR cannot
  come before LP2 unless one of the two changes. Recommended: hold both rulings at LP1's exit, and amend the charter so
  that LP2 to LP5 may start after the two rulings. The design line then resumes when LP2 to LP5 are done apart from
  LP4's editor look, which reads "the language's development is complete" as that point, and its design-system ADR is
  the condition for LP4's editor look and for LP6. This moves part of the stage 2.5 exit, so only the owner can approve
  it, and the owner decides whether it keeps "no gate is relaxed". Alternatives: read "the language's development is complete" as LP1's
  exit, so the design line resumes then and LP2 waits for the design-system ADR; or freeze after LP3, which changes the
  charter's order further.
- **LQ20 A handoff to the renderer line.** Recommended: add a handoff row now. It gives the renderer line the
  display-plan primitives of 8.5 (objects, charts, overlays, token colours, derived-state display) and, if LQ7 is
  accepted, the generated projection stage, so that they enter the renderer feature list before its freeze.
  Alternatives: no handoff, so display plans fit the frozen renderer as it is.
- **LQ21 Flow control in the solving profile.** Mechanism BM2 needs one of two forms, and the choice changes what each
  route must build (LQ9). Recommended: decide it together with LQ9 inside LP1, on LX-1's label cost and LX-3's refusal
  counts, before LP1's freeze gates and so before LP2 builds the checker and the IR. Alternatives: flow labels now,
  which needs a checker over the source that runs in every route (RT2 and RT4 add one over an approved subset) and, for
  run-time labels, RT1, RT3 or a modified VM; or capability separation now, which works in every route but is coarse
  (a solver's own tool cannot both read the state and preview).

## 16. Integrator defaults

The integrator takes these defaults unless the owner overrules them.

1. The kernel is not programmable, and the language never re-implements mechanics. Every kernel fast path has an oracle
   agreement test.
2. Programs get no network or process access in 1.0, and files only through granted, scoped capabilities.
3. Existing formats are reused, not renamed (section 13).
4. Data formats are canonical JSON, gzip-compressed where large, with SHA-256 over normalised bytes and size limits.
   The digest byte conventions (int32, uint32, little-endian) are normalised once, in KN1.
5. Errors reuse the ten typed categories and add language categories with stable codes: syntax, type, effect,
   capability, budget, evidence.
6. Every job has an explicit, deterministic budget, and seconds are a display unit, never a budget unit. "Not found
   within budget", "empty" and "not run" are distinct results, and a job that did not complete yields no negative
   claim.
7. Picking returns display instances that map to canonical IDs. Pixels never identify objects.
8. Library and documentation terms come from the glossary, checked by script (charter bar M1). Every language term gets
   a glossary entry first.
9. The end predicates stay separate: retained checkpoint, label checkpoint and sticker test. There is no single "solved".
   Goals are predicates typed by their level of the identity tower.
10. Every word has one normal form (R-11), used only as an equality key: free reduction for 600-cell-Full words, and
    domain-preserving merging for jumbling words. Preview, protection, commit and the token use the literal letters.
    R-11's property test gains a jumbling case: the key's word is admissible at a state exactly when the literal word
    is, with the same end digest. That property holds under four hypotheses: only runs of adjacent letters at one grip
    merge, identity products are kept, the menu is closed under products, and classification is exact (DT-L1 R3-M2). A
    word's exact expanded length and cost are shown before any run. A program is never split silently.
11. Nothing is added under `schemas/` in this stage. Formats are described in prose until the ADRs are accepted.
12. A public J1 API replaces the adapters' use of private members before the language binds to J1.
13. Artifacts are identified by content digest, never by path. Importers accept research records without a `format` key
    and file names that differ only in case. The corpus is read from Git blobs through a manifest of path, blob digest
    and size; no importer or test walks a checkout.
14. A commit names the preview on screen (INV-3), and the engine adopts only its own preview object.
15. The runtime stays off a turn's critical path and reads the store only through query rows (section 9).
16. Views get no workspace-write effect. A workspace change happens only inside a command the solver invokes.
17. A language-defined command's availability combines the checker's static verdict with the engine's availability of
    the kernel rows it may invoke.
18. Every built-in tool ships a principle record, and a fork inherits no evidence.
19. IDs on this page carry their own prefixes (section 18), and references to other documents name the source. Stored
    identifiers (error codes, budgets, evidence citations) carry a source prefix too: charter bar, partition component,
    contract amendment, spike or research item. A lint fails on a bare, ambiguous identifier in formats and
    architecture pages.
20. Research records are imported by a versioned table keyed by (format, field, value). A value the table does not map
    is imported as recorded, with its text kept word for word. A test checks that every (format, field, value) of the
    corpus is mapped or listed as unmapped.

## 17. What the full text adds

After the owner confirms the direction, the full text follows:

- **The architecture document**, one chapter per layer and pillar, with interface contracts, the exact evidence mapping
  and partial order, error codes and conformance tests.
- **ADR drafts**, each for the owner to accept:
  1. One mechanics and one authoritative state: derived states and the preview exception.
  2. The runtime as a client of the command layer, off the critical path.
  3. Exactness, the numeric tower and the algebras.
  4. Canonical identity, the identity tower and source maps.
  5. The evidence model and certificates.
  6. Effects, capabilities and flow control (the form of flow control after LX-1 and LX-3).
  7. The typed IR as the stable contract (with the route, because section 12 makes the IR the contract in RT1 to RT3).
  8. Jobs and deterministic budgets.
  9. Boundary mechanisms, with the policy left to the joint setting.
  10. Persistence formats, the data core and editions.
  11. Display plans and user charts.
  12. Implementation route and runtime placement (after the evidence).
- **DT-L1's open obligations**, now cleared: the label-lift lemma off the lattice (note N1), and the mathematician's
  check of what this revision adopted from the engineer advisor's round 2 cards (R3-M1 to R3-M4). Section 19 lists the
  corrections.
- **Plain-language explainers** from the round table on the key concepts, for the owner.

## 18. Identifiers

The first draft used IDs that collided with the charter's bars, the state contract's amendments and other topics' items
(DT-L1 M6). The round table's cards keep the old IDs.

| IDs on this page | What they name | IDs in the first draft and in DT-L1 rounds 1 and 2 |
| --- | --- | --- |
| INV-1 to INV-12 | Invariants | I-1 to I-12 |
| KN1 to KN7 | Kernel components | A1 to A7 |
| LG1 to LG5 | Language core components | B1 to B5 |
| LB1 to LB7 | Libraries and tools | C1 to C7 |
| EV1 to EV5 | Environment components | D1 to D5 |
| PS1 to PS3 | Persistence components | E1 to E3 |
| BM1 to BM5 | Boundary mechanisms | M1 to M5 |
| BS1 to BS4 | Boundary option sets | S1 to S4 |
| RT1 to RT4 | Implementation routes | I1 to I4 |
| LX-1 to LX-9 | Evidence spikes | X-1 to X-8; LX-9 is new |
| LP0 to LP6 | Phases | L0 to L6 |
| LQ1 to LQ21 | Owner questions | Q1 to Q18; LQ19 to LQ21 are new |
| Defaults 1 to 20 | Integrator defaults | Defaults 1 to 13 |

## 19. What round 3 corrected

Round 3 of the round table found errors in the text the owner confirmed. None changes this page's structure or any owner
question. Each is corrected in place, and this section keeps the change visible. The cards and the proofs are in
[DT-L1, the outcome of round 3](../../../progress/1.0/design-table/topics/language-ecosystem.md#outcome-of-round-3).

| Where | First text | Now | Card |
| --- | --- | --- | --- |
| INV-5, 8.1 | Sampling never upgrades a positive claim | Never a universal claim: a checked witness found by sampling proves an existence statement | N2, C.12 |
| KN1, 4.1, LG2 | Levels pose, visible pose class, labels, position, read as a chain with lower and higher | A partial order with sticker class, region class, coset, tree data, heights and shape; coarser and finer | N1, B.4; R3-M2 |
| LB4, 8.5 | Chart properties decided on exact vertices | Decided by an exact region test or a theorem; a vertex test can only refute a region property | R3-M2 |
| 8.1, section 13 | A stored claim's scope is an immutable input digest | Every scope field is immutable; a digest is at least as fine as the subject level | R3-M1 |
| 8.1 | Independent arguments combine by the strongest | Every undominated argument stays; a statement's summary is a set of alternatives | N2, C.6 |
| 8.1 | Five fields | Seven more facts in a stored record | R3-M1 |
| 8.1 | Refutation and negatives | Counterexamples need checked membership; mixed statements added; a negative can rest on a theorem; a completed job is not a completed search | N2, C.12 |
| Section 11 | Rice's theorem as the reason no checker exists | The verdict is not a function of (word, state); Rice covers programs without limits | R3-M3 |
| Section 11 | Only an enumerated list with flow control stops a composition | A list that leaves a part out stops the shipped composition; what a solver writes, only flow control stops | R3-M3 |
| Section 11 | One bit per run can leak | A stop can select among up to 36,000 previews, about 15.1 bits, and runs add up | R3-M3 |
| Section 11 | The boundary is a property of information flow | The checkable part is a flow property; "themselves" and "understanding" are not | R3-M3 |
| BM2 | A preview's outcome: end state, N, protection verdict | Also a refusal and its error category, and a read of the budget used; a missing field reads unknown | R3-M3 |
| LX-3 | Six must-refuse cases | Ten | R3-M3 |
| Default 10, LP2 | The key's property stated without conditions | Four hypotheses | R3-M2 |
| LP2 | Label-lift property test as a lead | A conformance test; the lemma is proved at status (theorem, producer) until a model review reads it | N1 |

### Corrected after round 3

A check of the text against its sources and the mathematician's round 4 made these corrections. None changes this page's
structure, and LQ16 gains one sentence. The cards are in
[DT-L1, round 4](../../../progress/1.0/design-table/topics/language-ecosystem.md#round-4).

| Where | First text | Now | Card |
| --- | --- | --- | --- |
| 1.2 | Each invariant restates a binding source | INV-12 is a proposal built on the owner's principle; no owner text names a principle record | R4-M19 |
| INV-3 | Every commit is one action on a fresh preview | A live twist has no preview: an immediate complete operation under the active protection, taken by one explicit solver action | Source check |
| INV-12 | Titled "Understanding is a deliverable"; a record of four parts; every result traces to primitives | Titled "The means of understanding are a deliverable"; the parts a person writes are listed; a runtime result traces to primitives and evidence, and an imported one stops at its stored bytes and says its producer is absent | R4-M10, R4-M13, R4-M17 |
| 8.1 | 156 of the 417 JSON files; the label families "map directly" | The bases are named (417 `.json` files; 497 records with the 80 `.json.gz` files, 232 without a top-level `format`), and every family goes through the (format, field, value) table | Source check |
| 10.3, stage 15 row | One certificate per admissible word; `pstar2-*` records | The stored `macro-certificate/1` accepts only descending macros ending in a jumble; the two formats are spelled out as `pstar2-stop/1` and `pstar2/2` | Source check |
| Section 11, the principle | "outputting a solution" | "outputting a solution by itself", as the owner wrote it | R4-M55 |
| BM4 | Which assistance classes were used | Which assistance classes the system recorded: a retyped word looks like the solver's own | R4-M11 |
| BS1 to BS4, integrator's leaning | "Authorship"; "the program ships no solver"; "the shipped research tier" | Author class, which records how bytes arrived; "the solving profile ships no solver"; a research tier if LQ16 ships one | R4-M11, R4-M15 |
| LB6 | The instance; generated parts | The instance's steps are checked from named givens, and the parts a person writes are named | R4-M1, R4-M17 |
| EV5 | Six learning steps | Seven: step 1 includes reading results, and a step for reading a principle record comes before writing tools | R4-M45, R4-M47 |
| LQ16 | Nothing on the theory book | The cut also settles whether the theory book covers the language's libraries | R4-M49 |
