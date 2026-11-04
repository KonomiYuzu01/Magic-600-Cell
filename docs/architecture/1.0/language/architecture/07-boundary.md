# Chapter 07: the human-solve boundary

Status: **draft for the owner. Not accepted.** Conventions are in [the overview](00-overview.md), section 3.

This chapter says how the language and its runtime carry the human-solve boundary. It states what binds, what software
can and cannot establish, the mechanisms that exist under every option, and the option sets the owner and the integrator
start from when they set the exact boundary together. It sets no policy. Every policy choice below is an option, a
proposal the owner can overrule, or an open item. Until the joint setting, built-in tools keep the 2 October boundary
and the 17 October exception.

Sources: [owner, 2 October 2026](../../../../wiki/decisions/owner-decisions-2026-10-02.md),
[owner, 17 October 2026](../../../../wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md),
[owner, 3 November 2026](../../../../wiki/decisions/owner-decisions-2026-11-03.md),
[DT-J1](../../../../progress/1.0/design-table/topics/jumble-solver-aids.md),
[jumbling observation](../../../../progress/1.0/jumbling-observation.md),
[DT-L1](../../../../progress/1.0/design-table/topics/language-ecosystem.md) (M5, R2-E3, R3-M3, R3-M4),
[the partition](../ecosystem-partition.md), sections 8.2 and 11, and chapters 01, 02, 03, 05, 08, 09 and 11.

Evidence: nothing in this chapter has run. The mathematics of section 2 is the mathematician's round 3 record, read by
the integrator and not yet by a reviewer. No flow check, label cost or refusal count exists (LX-1 and LX-3 have not run).

## 1. What binds

- **BND.1 [B]** The 2 October boundary. Allowed: tools outside the puzzle itself, such as entering and editing macros,
  macro analysis, templates and reuse of the solver's own macros, tracking, protection and view aids. A macro is entered
  by the solver, and the tool outputs its analysis. Setup search is allowed: for a target the solver names, the program
  may propose setups that keep the buffers fixed, for example from the certified guarded setup tables; the solver
  chooses the macro, may edit the setup and executes. Not allowed: the program choosing or outputting a solving macro by
  itself, executing without the solver's explicit action, or solving automatically. The result stays a human solve with
  appropriate tools.
- **BND.2 [B]** The 17 October exception. In the three-twist repair of the jumbled puzzle's descent, for a cap c the
  solver names, the program may list the pairs (x, d) whose repair lowers N, each with its y and z. The order is neutral
  (x in menu order, then d) and marks no pair as best. An empty list says that no pair at that cap lowers N. The solver
  picks one pair and makes its twists. The program does not choose the cap and makes no twist itself. Elsewhere the
  2 October boundary is unchanged: for the face match, the cover step and 600-cell-Full, no view lists helpful twists,
  ranks caps or marks a next move.
- **BND.3 [B]** The 3 November principle. If a solver develops a tool themselves, through the built-in base tools and
  while understanding its principle, and the tool assists solving, this does not count as the program outputting a
  solution by itself. The owner and the integrator set the exact boundary together. Until then, built-in tools keep the
  2 October boundary and the 17 October exception.
- **BND.4 [B]** DT-J1 binds as settled. Rule 6: a check answers about a twist or word the solver proposes, and showing
  N_c, the floor and blocked caps is allowed because these are facts about the configuration. A fact a built-in tool
  shows is static: it is defined at one revision, with no quantifier, minimum or argmax over twists or words, and it is
  shown per cap or for what the solver names, never as the result of the method's own selection (largest, top, first).
  DT-J1's filter keeps out a default sort of the cap map, a target cap or unlock letters the program picks, a cap-map
  mark on caps that cover some cap and that no cap covers, the combination of corrections ν calls for and any view that
  lights, filters or reorders the corrections by the current ν, an existence answer, Jumble off at ν ≠ 0, pose equality
  as the end test, a "next cap" key that skips caps with an empty list, a job started for a cap the solver has not
  named, and hot-cold signals. A headless test asserts that no view-model field holds a value for a twist the solver did
  not release (DT-J1 fix-first 6).
- **BND.5 [B]** Execution belongs to the solver (INV-3, charter S4, R-18, KN5.14). One deliberate action, a key press or
  a click, commits one previewed operation or one live twist (ONE.46). A held key never repeats a commit. Nothing commits
  in the background. Program views and the camera need no such action, because they change no puzzle state.
- **BND.6 [D]** This chapter owns `Profile`, `AuthorClass`, `ProvenanceBlock`, `BoundaryClass`, the declassifiers, the
  policy table and the option sets (overview, section 2). Chapter 02 owns the checker's rules, chapter 09 the sandbox and
  the fixed rows of the grant table, chapter 05 the stored form of the provenance field and chapter 11 what each route
  must build.

## 2. What software can and cannot establish

These are the facts every option faces. They come from DT-L1 M5 and R2-E3 as R3-M3 corrected them.

- **BND.7 [D]** A general-purpose language with read access to the state can express any solving procedure, and a solver
  can type a word computed elsewhere.
- **BND.8 [D]** Whether the program chose a word is not a function of (word, state). The identical pair can be the
  solver's named macro or the output of a search. So no test on the word and the state can decide it, however much
  computing it may use. This is the decisive fact.
- **BND.9 [D]** Budgets do not change that. For programs without limits, Rice's theorem rules out a checker that decides
  whether every program's output is a solving step. A budgeted program always stops, so one run is decided by running
  it, and only a check over all states stays out of reach. Budgets make runs finite. They do not make "searches" a
  property of what a program computes, and a budget is not a boundary mechanism.
- **BND.10 [D]** A static fact can answer a question over twists. In_c and Out_c are kept by a twist of c and A_c ≤
  min(In_c, Out_c), so N_c ≥ |In_c − Out_c|, the floor: when N_c equals its floor, no twist of cap c alone lowers N_c,
  and nothing was tried. A piece's domain at height 2 fixes the twists at each own holder that lower that piece.
- **BND.11 [D]** A composition can answer one too. Any composition-closed class that contains the ν read, the correction
  rows and reduction over F2 and F5 contains the function that returns the correction combination, which DT-J1's filter
  put out. The research's `solve(b2, b5)` is that function. "Searches" is intensional: a lookup table computes the same
  function. So no class of "non-searching" functions defines a safe tier. A list that leaves a part out stops the shipped
  composition. What a solver writes from base parts, only flow control stops.
- **BND.12 [D]** The boundary concerns who chose a word, not what the word does. The part of that question software can
  check is a flow property: which information reached a preview. "Themselves" and "understanding" are not checkable,
  because two identical flows can differ in whether the person understood.
- **BND.13 [D]** Software can control what it ships, how execution happens, which capabilities a profile grants, which
  flows inside the sandbox reach a preview, and how the records the system writes are labelled. It cannot establish
  authorship or understanding, and nothing it does binds a person who retypes a word.

## 3. Terms and types

### Sessions and profiles

- **BND.14 [D]** A *solving session* is a session whose journal can stand as a solve record. Every ordinary session is
  one. The partition uses the term without defining it; this definition is the integrator's, and the owner may change it.
- **BND.15 [O]** Whether the charter's clearly labelled practice copy of the full model counts as a solving session is
  open. If it does, one policy covers it. If it does not, it can be the place where a solver tries their own tool's
  previews before the joint setting has admitted them in a solve. Until the owner answers, the practice copy is treated
  as a solving session, which is the reading that widens nothing. LQ1, LQ2 and LQ4 touch this.

  ```text
  Profile = solving | tool_authoring | research
  ```

| Profile | A run works on | Purpose |
| --- | --- | --- |
| `solving` | The session the solver is working in, through snapshots at a revision | Assistance during a solve |
| `tool_authoring` | Fixtures and pinned inputs | Writing and testing a tool |
| `research` | Pinned inputs and derived states | Search, experiments and the research library |

- **BND.16 [P LQ1]** Profiles differ only in the capabilities they grant and the label they attach. Types, exact
  semantics and the run-record format are the same in all three. Under LQ1's alternatives, a separate research language
  or a product-only language removes the `research` row from the product (DET.39).
- **BND.17 [D]** A profile belongs to a run. The host sets it when it admits the run, from the place the run starts:
  a command in a solving session, the tool editor, or a research workspace. A program cannot set, raise or change it. The
  effective profile and the policy identity are complete inputs of the run (DET.4, DET.40). The kernel knows no profile:
  it exposes rows, and the table of which profile may call which row sits outside it (KN2.6).

### Author classes

  ```text
  AuthorClass = built_in | own | forked_built_in | imported
  ```

- **BND.18 [D]** The host assigns the class from how the module's bytes arrived.

| Class | Assigned when | What the host keeps |
| --- | --- | --- |
| `built_in` | The module's content digest is in this build's shipped manifest | The manifest digest |
| `forked_built_in` | The host's fork operation created the module from a built-in (LG5.8, LB7.3) | The origin digest and the fork record |
| `own` | The module was created in the local workspace, not by a fork or an import | The workspace record |
| `imported` | The module entered through a file or an import path (PS3, LB7.4) | The origin digest and the import record |

- **BND.19 [D]** A new version of a module keeps the class of its parent. Format conversion, renaming and saving never
  change a class (EV4.16, PS1.32). Whether a substantial edit of a fork or an imported module ever makes it `own` is the
  joint setting's question (BND.74, question 2).
- **BND.20 [D]** The class records where bytes came from. It proves neither authorship nor understanding (BND.13).
  Text copied by hand into a new module reads `own`, and software cannot tell.
- **BND.21 [D]** A missing or unreadable provenance field reads `unknown`, never `own` (R3-M3). A value that reads
  `unknown` keeps its bytes (PS1.33), and no policy row admits it unless the row says so.

### The provenance block

  ```text
  ProvenanceBlock = {
    block:        block version; form that produced it; checker or capability-contract pins,
    author:       AuthorClass | unknown,
    producer:     program(module: Digest<source>, entry) | manual | import(origin: Digest<bytes>),
    profile:      Profile,
    depends:      Labels | read_state(yes | no) | not_recorded,
    parts:        List<Digest<provenance>>,
    pick:         PickRecord?,
    declassified: List<DeclassifierUse>,
    legacy:       original labels and source digest?,
  }
  ```

  The wire form is chapter 05's (PS1.32 to PS1.34). `depends` holds `Labels` under flow labels and `read_state` under
  capability separation; `not_recorded` is the honest value for a legacy record.

- **BND.22 [D]** The host fills the block. `propose` takes it as a parameter (KN5.9), but the caller of `propose` is the
  runtime's adapter, which builds the block from the run's context. A program has no operation that sets any field.
  The engine stores the block as given and checks its form only, so the block is a claim by the host. Its strength is
  the strength of the checker and the interpreter (overview, section 7).
- **BND.23 [D]** A word the solver types or picks has `producer: manual` and `author: own`. A word entered by a program
  has that program's block. A 0.4 macro enters with `author: own`, `producer: import(...)`, `depends: not_recorded` and
  its captured source digest in `legacy`, and it gains no exception by that (PS1.33). The field reads how the word
  entered, not who thought of it (BND.20).
- **BND.24 [D]** A word built from parts cites the parts' blocks and carries the join of what they record. Copying,
  saving, exporting and re-importing keep the block (PS1.32). A value chosen through a state-derived branch does not lose
  its label because its letters came from an own macro.

### Labels and sinks

- **BND.25 [O]** Under flow labels, `Labels` is a set of atoms and the join of two sets is their union. A *transparent
  entry* is a shipped function on the policy's enumerated list whose output carries only the join of its inputs: exact
  application and composition of named words is the core of that list. Any other function adds `computed` to a word or
  target it returns. A declassifier removes named atoms from a named output. The set below is a working set for the LX-3
  corpus. LQ21 decides the real one, with LX-1's costs and LX-3's refusal counts.

| Atom | Attached to |
| --- | --- |
| `state` | Anything read from a snapshot or derived from one: kernel results, derived states, a preview's outcome, a refusal and its error category (BND.47), and the budget used after such a read |
| `computed` | A word or target returned by a function that is not a transparent entry |
| `research` | Any value produced by a run in the research profile, and anything stored from it |
| `unknown` | A value whose stored provenance reads `unknown` (BND.21) |

  ```text
  Sink = preview_word | preview_target | draft | display | workspace | file | record
  ```

| Sink | What reaches it | Where the host looks |
| --- | --- | --- |
| `preview_word` | The literal letters of a proposal | `propose` |
| `preview_target` | The target a proposal names | `propose` |
| `draft` | A word written to a draft (ONE.36) | The workspace write |
| `display` | Display plans and panel content | `publish_view` (RPI.10) |
| `workspace` | Workspace records other than drafts | The workspace write |
| `file` | Writes to a granted file | The file grant |
| `record` | Run records and evidence | The record write |

- **BND.26 [D]** Every sink stores the block. Whether a sink can refuse is policy. The flow rule (BND.33) covers the two
  preview sinks. `draft` is the same question as a preview, because a draft is a word waiting for a preview. Whether
  `display` refuses is LQ2 (BND.74, question 4). `workspace`, `file` and `record` keep labels and refuse nothing on labels
  alone, so a laundered word stays labelled and the check happens where the word is used.

### Boundary classes and the policy table

  ```text
  BoundaryClass = tracking | analysis | checking | protection | view_aid
                | setup_search_named_target | repair_list
  ```

- **BND.27 [D]** The class is a field of a built-in tool's module manifest (LG5.2, PS1.15). It is data for the policy. It
  grants nothing by itself and is reviewed when the tool ships (BM5). A class declaration and a `PrincipleRecord` are
  separate obligations (LB6.3). A solver's own tool has no class unless the joint setting gives it one.

  ```text
  PolicyTable = {
    identity:     Digest<policy>,
    form:         flow_labels(static | run_time) | capability_separation,
    flow_rule:    on | off,
    stop_rule:    the fix of BND.51 the policy uses,
    capabilities: rows of (profile, author class or boundary class, capability, row set, verdict),
    sinks:        rows of (profile, sink, labels admitted),
    transparent:  the enumerated transparent entries,
    declassifiers: the entries the policy enables,
    admitted:     the author classes the policy admits,
  }
  ```

- **BND.28 [D]** The policy table is versioned data outside the kernel, shipped with the build and read by the host. No
  program can change it. Its identity enters every run record (DET.4, DET.40). The person who owns a local install can
  edit their copy. A record then names a different identity, and that is all it can do (BND.82).

## 4. The five mechanisms

These exist under every option, so the policy can be set later without a redesign (INV-11).

**BM1 Execution gate.**

- **BND.29 [B]** A commit needs an explicit user action on the shown preview. The shell mints the token from a
  non-repeating key-down or click, bound to the preview's ID and the digest of its literal letters, with the model
  identity, menu identity, menu-order digest and contract revision (KN5.14). The runtime has no commit row (ONE.3), no
  commit effect (LG3.4) and never sees a token (ONE.32).
- **BND.30 [D]** A preview is prepared only inside a command the solver invokes, and a view has no preview effect (LG3.6,
  ONE.34). The kernel row that previews a word the solver composed is not a program, so a key bound to it passes through
  no flow check. The word is the solver's.

**BM2 Provenance and flow control.**

- **BND.31 [D]** Every preview names the program, its module digest, its author class and what it may depend on, as far
  as the form of flow control records it. A commit's journal row keeps that block (KN4.4). An explicit pick is recorded
  as a pick.
- **BND.32 [D]** Stored words, macros and workspace records carry the provenance field, so a word laundered through the
  workspace or a saved file keeps its label (PS1.32). A format conversion, a recording and a turn into source keep it too
  (EV4.16).
- **BND.33 [O]** The *flow rule* is a policy switch. When it is on, in the solving profile, a preview's word and target
  may depend on the state or on computed results only through the declassifiers. Whether it is on, and which author
  classes it admits, is LQ2 (BND.74, question 3).

**BM3 Profiles.**

- **BND.34 [D]** The three profiles of section 3 grant capabilities by the table of section 8 and attach their label.
  Research on derived states never changes the session state (ONE.2).

**BM4 Labelled records.**

  ```text
  AssistanceSummary = {
    session:  session and epoch,
    journal:  Digest<journal range>,
    runs:     Digest<run-record set>,
    policy:   Digest<policy>,
    commits:  counts by author class, profile and declassifier use,
    tools:    the boundary classes and author classes of the runs recorded for the session,
  }
  ```

- **BND.35 [D]** A solve record states which assistance classes the system recorded. The host computes
  `AssistanceSummary` from the journal's provenance blocks and the run records, and the summary names its inputs: `runs`
  is the digest of the sorted digests of the run records it read. No program writes it, and recomputing it from the same
  journal range, run records and policy gives the same bytes. A view tool leaves no journal row, so its class comes from
  the run records: a later view-only run changes `runs`, and it can change `tools`, while the journal stands still. A
  word the solver retyped appears as the solver's word (BND.13).
- **BND.36 [O]** Whether labels are shown during a solve or only in records is LQ2 (BND.74, question 6). The summary
  exists either way.

**BM5 Boundary classes.**

- **BND.37 [D]** Each built-in tool declares its class under the 2 October boundary. The review checks the tool's exact
  entry definition and its row set against the class (CT-LB6-03). The classes are those of BND.27.

## 5. Flow control: the two forms

Flow control is what stops a program from reaching a preview with a word it chose from the state. It takes one of two
forms, and LQ21 chooses. Neither is a decision of this chapter. Under LQ21's recommendation the choice is made with LQ9
inside LP1, on LX-1's label cost and LX-3's refusal counts, before LP2.

- **BND.38 [O]** *Flow labels* (LG3.12) follow data flow and control flow, including choices between words and targets,
  module calls and stored values. Static labels run in the checker. Run-time labels also track executor values and branch
  context. Approval covers exactly the source and dependency bytes executed.
- **BND.39 [O]** *Capability separation* (LG3.13) splits programs in the solving profile. A program either reads
  state-derived data or prepares previews, never both, apart from the declassifiers, and a program that prepares previews
  does not read their outcomes.

| Question | Static labels | Static and run-time labels | Capability separation |
| --- | --- | --- | --- |
| What is checked | Every possible data and control dependence, over the exact source that runs | The same, narrowed to the path taken | Which capabilities a program holds |
| Cost per step | None | A cost per step, unmeasured (LX-1) | None |
| Harmless tools refused | The most among label forms, uncounted (LX-3) | Fewer, uncounted (LX-3) | Every tool that both reads state and previews |
| What a record can say | The labels of each previewed word and target, indirect choices included | The same, closer to the actual run | Whether the program read the state |
| A preview's outcome | Carries `state` | Carries `state` | Not readable by a previewing program |
| Routes (RTE.24) | RT1, RT3; RT2 and RT4 add a checker over an approved subset | RT1, RT3's compiled fields, or a modified borrowed VM | Every route |
| The stop channel (section 6) | Stays open until a fix is chosen | Stays open until a fix is chosen | Does not arise |

- **BND.40 [D]** Either form is conservative and refuses some harmless tools. Either must be fixed before LP2, because
  checks added after the checker and IR exist would refuse tools that had been allowed to run and saved (RTE.26).
- **BND.41 [D]** Adopting either form does not switch the flow rule on. The form is how flows are seen. BND.33 is whether
  they are refused. With the flow rule off, the form still feeds records.
- **BND.42 [D]** Preview outcomes. ONE.32 gives a program a preview's public fields, including its effect and protection
  verdict. In the solving profile this chapter narrows that clause:
  - under flow labels, those fields carry `state`, so a program that reads one and then previews has a state-dependent
    preview;
  - under capability separation, a previewing program does not receive `effect` or `protection`. It receives only the
    preview's ID and base.

  In the other profiles ONE.32 stands.
- **BND.43 [D]** A refusal is a preview outcome (R3-M3). Under flow labels, the receipt's error carries `state`. Under
  capability separation, a previewing program in the solving profile receives one fixed acknowledgement whatever the
  engine answered. The solver still sees the full message with its reason and remedy (charter E6, ERR.13), because the
  shell reads the receipt and the program does not.
- **BND.44 [D]** Reads of use. A program can read the granted limits and the amount used (DET.27). Once a run has read
  state, what it has used can depend on the state, so a read of the amount used carries `state` under flow labels. Under
  capability separation a previewing program has read no state, and declared kernel costs depend only on inputs (KN7.8),
  so the channel does not arise.
- **BND.45 [D]** Neither form is evidence that a mathematical claim is true (LG3.15, INV-5). A label says where a value
  came from. Whether the value is right is chapter 06's question.

## 6. Channels a flow check does not close

R3-M3 showed that DT-L1's "one bit per run" is false. This section gives what replaces it.

- **BND.46 [D]** A flow check covers data and branches. It does not cover whether a run ends or emits a preview. In the
  sketch below every previewed word is a constant and passes a label check. The pending preview is replaced by the next
  one (KN5.11), so the preview left pending when the run stops is the program's choice among up to 36,000 letters: about
  15.1 bits (log₂ 36,000), and `state_test` may hold a whole search.

  ```text
  for j in public_list:
      preview(letter[j])       # constant word, passes a label check
      if state_test(j): stop   # state-dependent stop, invisible to the rule
  ```

  One bit holds only for one preview, as the last action, on a path with no state-dependent stop. KN5.12's answer on
  replacing a shown preview changes the size of this channel, not its existence.

- **BND.47 [D]** The channels and what closes each:

| Channel | What carries the information | Under flow labels | Under capability separation |
| --- | --- | --- | --- |
| Stop | Which preview is pending when the run stops | Open until a fix of BND.51 is chosen | Does not arise |
| Emission | Whether a run emits a preview at all | Open until a fix of BND.51 is chosen | Does not arise |
| Refusal | A refused proposal and its error category | `state` label (BND.43) | Not readable (BND.43) |
| Budget | The amount used | `state` label (BND.44) | Does not arise (BND.44) |
| Accumulation | Repeated runs with a public index add up the bits of one run | Closed with stop and emission | Does not arise |
| Storage | A word carried through the workspace or a file | Provenance field | Provenance field |
| Display | A state-derived word shown for the solver to retype | Only if `display` is checked (LQ2) | Only if `display` is checked (LQ2) |
| Retyping | A person types what they saw | Not closable | Not closable |
| Outside the sandbox | A program the runtime does not run | Not closable | Not closable |

- **BND.48 [D]** Display deserves its own line. A tool that shows a state-derived word passes every preview check, and
  DT-J1's filter also covers what is shown. So a preview-only rule leaves the same output one retyping step away.
- **BND.49 [D]** Labels hold only against sandboxed programs. A program outside the runtime that reads an exported
  journal and prints a word is the same as retyping.
- **BND.50 [D]** Clock. A granted clock is an input that makes a run non-reproducible (DET.40). It gives a running program
  no new reach beyond its other reads. Nothing here adds a rule for it.
- **BND.51 [O]** Three fixes for the stop and emission channels. None is chosen. The first two apply to flow labels. The
  third is the other form of LQ21.

| Fix | What it requires | Trade-off |
| --- | --- | --- |
| One preview per run | A run emits at most one preview, as its last action, on a control path with no labelled condition. The checker must prove this. | Simple to explain. Refuses a tool that decides between previewing and stopping from the state. Saves nothing for a tool that needs several steps. |
| Stop-sensitive labels | A stop, a loop exit or the end of a run under a labelled condition raises the label of whatever preview is pending. | Keeps more tools. Needs run-time tracking or a stricter static rule, and more of the checker must be trusted. Refusal counts are unknown (LX-3). |
| Capability separation | A previewing program reads no state. | Closes the channel with no analysis. Coarse: a solver's own tool cannot both read the state and preview, and a record says only whether a program read the state. |

- **BND.52 [D]** LX-3's must-refuse corpus covers these channels (RTE.45): a stop-selected preview, refusal-driven
  selection, a one-item pick and accumulation across runs, added to DET.50's six cases.

## 7. Declassifiers and picks

- **BND.53 [D]** A declassifier is a fixed kernel or library entry that the policy enables. It clears named atoms from a
  named output, under a stated condition, and records that it did. No program and no library can declare one (LB1.6,
  LG3.11). A declassifier does not widen BND.1 to BND.5.

  ```text
  Declassifier = {
    id:       one of the fixed entries below,
    entry:    the kernel or library entry, by digest,
    clears:   atoms,
    applies:  the output it clears them from,
    needs:    the condition that must hold,
    records:  what it writes into `declassified`,
  }
  ```

| Entry | Source | Output | Condition | Records |
| --- | --- | --- | --- | --- |
| Setup search for a named target | BND.1 | The proposed setups, and the word the solver takes from them | The solver named the target | The target, the entry's digest, the table digest |
| Repair pair | BND.2 | The pair's word | The solver named the cap and picked the row | The cap, the row, the menu-order digest |
| The solver's own pick | BND.1 ("the solver chooses") | The solver's choice among the options | See BND.55 | A `PickRecord` |
| Admitted author class | BND.3 | A word from a class the policy admits | The class is in `admitted` | The class and the policy identity |

- **BND.54 [D]** In the interim, `admitted` is empty and the first three entries serve built-in tools only (BND.69).

  ```text
  PickRecord = {
    list:     the entry or program that built the list, by digest,
    fixed:    yes when the list is a fixed entry or a fixed object set,
    options:  Digest<option list>,
    chosen:   the option, and the target or content it fixed,
  }
  ```

- **BND.55 [O]** The pick rule. R3-M3 found that a pick that clears everything launders a one-item list: a program builds
  a list with one word, and one key press turns its choice into the solver's. The rule proposed for the joint setting is
  that a pick declassifies the solver's *choice* and not the *option set*. The picked word keeps the join of the list's
  labels, unless the list is a fixed entry (setup search, repair list) or a fixed object set such as any cap on the map.
  The alternative, a pick clearing everything, needs a separate way to close the one-item case. This changes nothing in
  the interim, because no solver tool previews. It is a rule the owner may change.

## 8. Profiles and capabilities

This table fills the cells DET.37 leaves to this chapter. It is what the interim reading gives. It stays within the
partition's interim reading and adds to it only where marked. Network and process are never granted, and the rows for
files, clock and seeded randomness stay as DET.37 states them in every profile.

| Capability | Solving: built-in tool | Solving: own, forked or imported tool | Tool authoring | Research |
| --- | --- | --- | --- | --- |
| Snapshot read | The read rows its reviewed class needs | Open (BND.59) | Every read row, on a fixture or a pinned input | Every read row, on a pinned input |
| Derived state | Only for a word or target the solver named, and inside the two exceptions | Open (BND.59) | Yes, on fixtures and pinned inputs | Yes, as immutable kernel values |
| View change | Display plans within the tool's class; facts follow rule 6 | Open (BND.59) | Inside D8 until RPI.44 is answered | Under LQ6 (RPI.43); the output keeps its label (RPI.45) |
| Workspace change | Inside a command the solver invokes; a word only if the solver named it or a declassifier covers it | Inside a command the solver invokes; no tool word to a draft (BND.58) | Records of the authoring workspace, labelled | Research records, labelled; no draft of a solving session |
| Prepare a preview | A word the solver named, and the two exceptions | No, until the joint setting | Capture only (BND.60) | Capture only, or a research session under LQ4; never a solving session |

- **BND.56 [P LQ2]** The table above holds under LQ2's recommendation, which keeps the interim reading until the joint
  setting. Each own-tool cell is what the joint setting replaces.
- **BND.57 [D]** A built-in tool's rows are listed by its class. `derive` is the row where the policy acts, because a search
  over twists is built from it (ONE.23). A built-in tool in a solving session calls `derive` only for what the solver
  named, or inside the two exceptions.
- **BND.58 [P LQ2]** A draft is a word waiting for a preview. So while no solver-tool solving preview is enabled, a
  solver's own tool does not write a word to a draft of a solving session either. Without this clause the draft would
  bypass the interim reading. ONE.36 left the question here.
- **BND.59 [O]** The interim reading bars own-tool previews. It says nothing about what an own tool may read or show in a
  solving session. A loop over snapshot reads can select the largest N_c and show it. This gap is real, and this chapter
  does not fill it by inference, in either direction. Nothing ships to a solving session before LP6, and the joint
  setting comes first (PHS.7, PHS.38), so the gap costs nothing yet. What the interim reading does say stands: it
  authorises no shipped function that selects, and it enables no word shown for the solver to retype from a shipped tool.
  The joint setting answers this under question 4. The owner can also answer sooner by changing the interim reading.
- **BND.60 [P LQ2]** In the tool-authoring and research profiles, `propose` goes to a capture. The word and its labels
  are recorded in the run's output for a test to read, and no engine preview exists, so no token and no commit can follow.
  This lets a solver test a tool that previews without touching a session. It is new here, and the owner may overrule it.
- **BND.61 [D]** Research output never becomes a preview in a solving session. A proposal whose block, or any part's
  block, has `profile: research` is refused with `LG3:capability.not_granted`, whether it arrives directly, from the
  workspace or from a file. The bytes stay readable. A person can retype them, and that leaves no record (BND.13).
- **BND.62 [D]** This chapter adds no capability-dependent budget ceiling (DET.28). A budget does not make a program
  non-searching (BND.9), so a ceiling would be a resource rule and not a boundary. If the joint setting wants one, it is a
  row of its own.

## 9. What ships: the interim policy table

- **BND.63 [P LQ2]** Under the interim reading, the policy table starts at the 2 October boundary for everything that
  ships, library functions included. Each class may do the following in a solving session. Chapter 03's candidates are
  listed by their LB6.5 names. A listing admits nothing: each tool still needs its review, and none has a
  `PrincipleRecord` yet.

| Class | May, in a solving session | Rests on | Candidate tools (LB6.5) |
| --- | --- | --- | --- |
| `tracking` | Show what is recorded and what the state shows, per object the solver names or over a fixed list, in a fixed order | BND.1, BND.4 | Orbit residual and work-sheet inspector; piece, buffer and frame inspector; solver's macro and work-sheet reuse; jumbling facts and named straddlers |
| `analysis` | Output its analysis of a macro or word the solver entered or named | BND.1 | Word/recipe composer and full-effect inspector; named-macro comparison and residual inspection; named-cap covers query; ν, drafted-set check and static correction catalogue |
| `checking` | Answer about a twist or word the solver proposes | BND.4 | Named twist/word checker and kept checks; checkpoint and sticker-test inspector |
| `protection` | Hold and verify the protection the solver set; a verdict carries its level (KN1.11) | BND.1 | Protection and block-condition checker |
| `view_aid` | Change what is shown and how, for what the solver names; choose no target | BND.1 | Representation, work-set and filter tools |
| `setup_search_named_target` | Propose setups that keep the buffers fixed, for a target the solver names | BND.1 | Named-target guarded setup tool |
| `repair_list` | List the pairs at a cap the solver names, in neutral order | BND.2 | Named-cap repair list |

- **BND.64 [P LQ2]** In a solving session no shipped function searches, minimises, ranks or selects (largest, top, first)
  over twists, words or caps, none picks a target, and no shipped job starts for a cap the solver has not named. Every
  chapter 03 entry marked **research tier candidate** stays out of the solving profile. The marker is what the host's
  shipped-manifest scan reads (CT-BND-16).
- **BND.65 [P LQ2]** The items chapter 03 left to this chapter stand as follows under the interim reading:
  - the correction-combination solve (LB3.1) is not admitted;
  - the workflow's historical candidate and next-orbit suggestions (LB2.5) are not admitted;
  - H-08's `view.recommendations`, `target.next.activate`, `object.actions`, `structure.find` and any macro-comparison
    mode that selects a solving target or word (LB6.7) are not admitted. A static version that selects nothing needs its
    own content definition first;
  - a research runner, a correction solver and WC-PLAN stay research tier candidates.

  Whether any of them ships at all, in any profile, is LQ2 (question 7) and LQ16.
- **BND.66 [O]** Edges for the mathematician and the joint setting. These are facts rule 6 allows and that also answer a
  question about twists:
  - N_c at its floor (BND.10). Rule 6 names N_c and the floor as allowed, so they stay shown.
  - A piece's domain and height, shown for a piece the solver names. They stay shown as DT-J1 settled them.
  - Shipped tables: the certified setup tables, the seeds and the 36 corrections. The setup tables serve setup search for
    a named target. The corrections stay a static catalogue, with no view that lights, filters or reorders them by ν.

  This chapter rules on none of these for a solver's own tool. For a built-in tool, DT-J1 stands. The joint setting's
  question 5 asks whether such facts count as static facts when a tool, and not a person, reads them.

## 10. Option sets for the joint setting

Each set keeps BM1 to BM5, the built-in tools' 2 October boundary and the 17 October exception. Each is an option, and
the order implies nothing.

| Set | What a solver's own tool may do in a solving session | How "themselves", "base tools" and "understanding" are read | For | Against |
| --- | --- | --- | --- | --- |
| BS1 Provenance | Prepare a preview of any word it computes; the solver commits each one; BM2 records what the preview may depend on (under capability separation, only whether the program read the state) | Author class (BND.18): created or forked locally. It records how the bytes arrived, not who thought of them. | Simple; most freedom; honest labels | Base tools and understanding are not checked; a copied solver counts after a trivial fork; the line rests on labels |
| BS2 Base tier | As BS1, but in the solving profile the tool may call only the base tier: an enumerated list of shipped functions, each with a recorded definition and a boundary ruling. The list follows rule 6, plus exact application and composition of named words. It ships no function that searches, minimises, ranks or selects over twists, words or caps. The solver may build such a function from base parts. A research tier (ready-made search, strategies), if LQ16 ships one, runs in the research profile on derived states. | Author class as in BS1, plus a capability boundary enforced with either form of flow control; building from base parts is the solver's own development | Matches the principle's wording: the solving profile ships no solver, and the solver builds one from base parts; research stays free | A determined user can rebuild any search; understanding is not checked; every listed function needs the mathematician's ruling, shipped tables and theorem-backed facts included; listed functions compose into forbidden outputs (BND.11), so flow control must also cover what a tool shows, or the list must leave such parts out |
| BS3 Understanding | As BS2. In addition, the capability to prepare solving previews opens per principle, after the solver passes the competence check on that principle's record | Author class as in BS1, base tier and competence checks | Closest to "while understanding its principle"; ties into the learning path | Gating conflicts with soft disclosure unless the checks only label; checks can be gamed; a check shows competence on a task, not understanding (BND.72); writing the checks is substantial work |
| BS4 Named targets | A solver's own tool may prepare previews only for a target the solver names, which extends the 2 October setup-search allowance to own tools; flow control checks that the target came from the solver. Anything else runs in the research profile. | Author class as in BS1; naming the target is the solver's own act | Smallest step from 2 October; easy to explain; enforceable with either form: flow labels check that the target came from the solver, and under capability separation the tool reads no state and reaches it only through the shipped setup search | Restricts assistance more than the principle requires; "named target" needs a definition per tool kind |

- **BND.67 [D]** What each set writes into the policy table:

| Field | BS1 | BS2 | BS3 | BS4 |
| --- | --- | --- | --- | --- |
| `flow_rule` | Off, or on with `own` admitted | Available; on means the rule of BND.33 | As BS2 | On |
| `admitted` | `own`; `forked_built_in` and `imported` by question 2 | By questions 2 and 3 | By questions 2 and 3, per principle passed | None; the target is the declassified part |
| Own tool's read rows | Every read row | The base tier | The base tier | None under separation; read rows under labels, with the target checked |
| `display` sink | By question 4 | By question 4; needed if the list keeps parts that compose into forbidden output | As BS2 | By question 4 |
| Further work | None | The enumerated list and its rulings | The list, its rulings and a competence check per principle | A definition of "named target" per tool kind |

- **BND.68 [D]** Worked comparison. Five tools, read with the flow rule as each set uses it. Nothing here is a ruling. A
  cell says what the option as written gives.

| Tool | BS1 | BS2 | BS3 | BS4 |
| --- | --- | --- | --- | --- |
| 1. Conjugates a macro the solver named by a setup the solver named, and previews the word | Allowed | Allowed: exact composition of named words | Allowed once the principle's check is passed | Allowed if the word counts as a named target; needs BS4's definition |
| 2. Reads N_c at every cap and shows them sorted | Allowed; a built-in could not do this | Allowed if the solver built the sort from base parts, and if the `display` sink admits it | As BS2 | Research profile only |
| 3. Searches over twists on derived states and previews the best word | Allowed; the record says so | Allowed with `own` admitted, refused with the flow rule on and `own` not admitted | As BS2, once the principle's check is passed | Research profile only |
| 4. The same tool, copied from another person | Counts as own after a trivial fork, or per question 2 | By question 2 | By question 2, and the check still applies | Research profile only |
| 5. Shows the search result for the solver to retype | Allowed | Passes unless `display` is checked | Passes unless `display` is checked | Research profile only |

  Whatever the option, a person who retypes the word of tool 5 leaves a record that says the solver typed it.

## 11. Interim reading and the integrator's leaning

- **BND.69 [P LQ2]** The interim reading is the integrator's, and the owner may change it. The policy table starts at the
  2 October boundary for everything that ships. BND.64 and rule 6 apply. The two exceptions are setup search for a target
  the solver names and the repair list at a named cap. Research-profile output never becomes a preview in a solving
  session. No solver-tool solving preview is enabled until the boundary is set.
- **BND.70 [D]** The integrator's leaning, for discussion only: build BM1 to BM5 in any case, with the form LQ21 chooses,
  and start the joint discussion from BS2 with the flow rule available. The solving profile then ships no solver, the
  solver may build one from base parts, and the line rests on capabilities and flow control rather than on guessing what
  a program computes. It does not gate learning. This is not a decision, and it presets neither LQ21 nor the route.
  Round 3's corrections weaken one reason for it: labels alone leave the stop channel open, so BS2 with flow labels also
  needs a fix of BND.51.
- **BND.71 [P LQ2]** The joint setting. The round table explains BS1 to BS4 and the facts of section 2 to the owner in
  plain language. The owner and the integrator go through the questions of section 12. The result is a versioned policy
  table and an owner decision record. It comes before LP6 (PHS.38), and LQ21 comes earlier, before LP2. LX-3's counts
  inform it, and no count decides it.

## 12. Understanding and the questions for the joint setting

- **BND.72 [D]** EV5 can supply the means of understanding. It cannot certify understanding (R3-M4). A principle record
  gives the specification, the reason, the failure of a dropped hypothesis and the derivation of an instance. A
  competence check records that a task was completed in this install, with the help used inside the program. Neither
  shows what anyone understood, so "understanding" in BND.3 stays outside what the software can establish; how the
  boundary treats that is the joint setting's question. This holds under every option.
- **BND.73 [P LQ14]** Under LQ14's recommendation, competence checks guide and do not lock (EV5.8). If the owner selects
  BS3, the check gates the capability to prepare solving previews per principle (EV5.9), as LQ14 foresees; that needs the
  owner's word. No learning result grants a capability until then.
- **BND.74 [O]** The questions for the joint setting, from the partition, then those round 3 added:
  1. Does "assists solving" exclude a tool whose single preview completes the puzzle?
  2. Is a tool written by another person treated as built-in, as own, or as a class of its own? When, if ever, does an
     edited fork or import become own?
  3. Is the flow rule switched on, and which author classes does it admit?
  4. Which display sinks does flow control check: previews only, or also what a tool shows for the solver to retype?
     This also fills BND.59.
  5. Do shipped tables and theorem-backed static facts count as static facts?
  6. Are labels shown during a solve, or only in records?
  7. Is the research profile part of the shipped product?
  8. Which fix of BND.51 does the policy use, if the form is flow labels?
  9. Does the pick rule of BND.55 stand?
  10. Does the practice copy count as a solving session (BND.15)?

## 13. Limits

- **BND.75 [D]** The boundary mechanisms are only as strong as the checker and the interpreter (overview, section 7). A
  bug in either can give a wrong label or a flow-control hole. It cannot commit a word the solver did not see, because
  BM1 does not rest on them.
- **BND.76 [D]** Labels cover flows inside the sandbox and records the system writes. A retyped word, a word carried by
  hand and a program outside the runtime leave no label (BND.13, BND.49).
- **BND.77 [D]** No form closes everything. Flow labels without a fix of BND.51 leave stop and emission channels. A checked
  `display` sink cannot stop a person reading a fact and reasoning from it, and that is what the boundary wants.
- **BND.78 [D]** A determined user can rebuild any search from base parts. Under BS2 that is the principle's own case, and
  the label says only that a solver's tool did it.
- **BND.79 [D]** Author class records how bytes arrived (BND.20), and a competence check records a task (BND.72). Neither is
  proof of authorship or understanding.
- **BND.80 [D]** A conservative check refuses some harmless tools. How many is unknown until LX-3 reports its false-refusal
  count, and no threshold is set.
- **BND.81 [D]** Boundary mechanisms carry no evidence. A label, a class or a passed flow check is not a claim about the
  mathematics (INV-5), and a claim's strength never grants a capability.
- **BND.82 [D]** The owner of a local install can change the policy table or the program. A record can name the policy
  identity it ran under. Nothing can stop a person from changing their own copy, and this project does not try. The
  boundary describes what the system recorded about a solve, and says what it cannot record. Under the interim reading,
  nothing shipped in the solving profile chooses a solving word by itself.

## 14. Errors

- **BND.83 [D]** This chapter adds no codes. It uses these rows of chapter 13:

| Code | Used for |
| --- | --- |
| `LG3:capability.not_granted` | A capability the profile, class or policy does not grant, and research output proposed in a solving session (BND.61) |
| `LG3:capability.grant_mismatch` | A run reproduced under another profile or policy identity |
| `LG3:effect.flow_refused`, `LG3:capability.flow_refused` | The flow rule, one of the two after LQ21 (ERR.24); neither is emitted before that |
| `PS1:capability.provenance_incompatible` | A stored block whose form cannot justify a gated use (PS1.33) |
| `KN5:capability.commit_row_refused` | The runtime reaching for a commit row |
| `LG3:effect.undeclared` | A view reaching a preview or workspace effect through a callee (LG3.5, LG3.6) |

## 15. Conformance tests

No conformance test exists yet. Every test below runs under each form LQ21 still has open, and its case set follows the
policy table in force. Chapter 12 assigns them to phases.

| Test | Checks | Class |
| --- | --- | --- |
| CT-BND-01 | No program operation sets any field of a `ProvenanceBlock`; attempts through every API, helper and module call leave the host's block unchanged (BND.22) | source |
| CT-BND-02 | A missing or unreadable field reads `unknown`, never `own`, and a row that admits `own` does not admit it (BND.21) | source |
| CT-BND-03 | A word carried through a workspace record, a file, a macro, a recording and a conversion to source keeps its block (BND.32) | source |
| CT-BND-04 | A word built from parts carries the join of the parts, including a choice made through a labelled branch (BND.24) | source |
| CT-BND-05 | DET.50's first six must-refuse cases are refused with the expected code, no state change and no data returned by a planted read | source |
| CT-BND-06 | A stop-selected preview is handled as the fix in force states (BND.46, BND.51) | source |
| CT-BND-07 | Refusal-driven selection: a program that previews and reads the refusal gets a labelled value or the fixed acknowledgement (BND.43) | source |
| CT-BND-08 | A one-item pick keeps the list's labels under BND.55's rule; a pick from a fixed entry clears them | source |
| CT-BND-09 | Accumulation: repeated runs with a public index gain nothing under the form and fix in force (BND.47) | source |
| CT-BND-10 | A read of the amount used after a state read carries `state`; under separation a previewing program's counters do not vary with the state (BND.44) | source |
| CT-BND-11 | In the solving profile a preview's outcome is labelled or withheld as BND.42 states; in the other profiles ONE.32 holds | source |
| CT-BND-12 | A refused proposal under separation gives the program the fixed acknowledgement, and the shell still shows the full message | source |
| CT-BND-13 | Research output is refused as a solving preview directly, through the workspace and through a file (BND.61) | source |
| CT-BND-14 | In tool authoring and research, `propose` creates no engine preview and no token, and the session's digest is unchanged (BND.60) | source |
| CT-BND-15 | Every built-in tool declares a class, its row set lies within the class, and a review record names the entry digest (BND.37, CT-LB6-03) | source |
| CT-BND-16 | The shipped-manifest scan finds no entry marked research tier candidate in the solving profile (BND.64) | source |
| CT-BND-17 | The repair list runs only for a named cap, keeps neutral order, marks no row best, words an empty list as BND.2 states, and no key skips caps with an empty list | source |
| CT-BND-18 | Setup search runs only for a named target, its setups keep the buffers fixed, the solver can edit the setup, and nothing previews without the solver's action | source |
| CT-BND-19 | No view-model field holds a value for a twist the solver did not release (BND.4) | source |
| CT-BND-20 | A held key commits at most once and a program cannot obtain a token (BND.29, with CT-KN5 and CT-ONE) | windows |
| CT-BND-21 | `AssistanceSummary` recomputed from one journal range, one set of run records and one policy gives the same bytes; two sets that differ by one view-only run give summaries with different `runs`; no program can write it; imported 0.4 macros appear as imported (BND.23, BND.35) | source |
| CT-BND-22 | A switch of forms is a recorded migration; an older block keeps its form, and a gated use it cannot justify is refused with `PS1:capability.provenance_incompatible` | source |
| CT-BND-23 | Every run record names the policy identity, and reproducing under another one is refused with `LG3:capability.grant_mismatch` | source |
| CT-BND-24 | The flow code LQ21 retires is never emitted, and before LQ21 neither is (ERR.24, CT-ERR-15) | source |
| CT-BND-25 | The false-refusal report lists every refused program on the frozen allowed list, with its code, per form and variant, and sets no threshold (RTE.46) | source |

## 16. Open items

| Item | Clauses | What decides it |
| --- | --- | --- |
| LQ1 | BND.15, BND.16 | The owner; a separate research language removes the research profile from the product |
| LQ2 | BND.56 to BND.60, BND.63 to BND.66, BND.69 to BND.71, BND.74 | The joint setting of the owner and the integrator, before LP6 |
| LQ4 | BND.15; the research-session cell of section 8 | The owner, with branches |
| LQ6, RPI.44 | The view row of section 8 | The owner; D8 holds meanwhile |
| LQ14 | BND.73 | The owner; a gate needs BS3 |
| LQ16 | BND.65 | The owner, on what ships |
| LQ21 | BND.25, BND.38, BND.39, BND.51, BND.83 | The owner, with LQ9, inside LP1, before LP2, on LX-1 and LX-3 |
| KN5.12, LG3.7 | BND.46 | Whether a program may replace a shown preview |
| PS1.34 | Section 3, provenance block | The block's initial wire version, before any conditional migration-format freeze |

Additions in this chapter that the owner may overrule: BND.14 (the term "solving session"), BND.42 and BND.43 (what a
previewing program receives), BND.55 (the pick rule), BND.58 (no tool word to a draft), BND.60 (capture in the
non-solving profiles) and BND.62 (no capability-dependent ceiling).

The label atoms of BND.25 are a working set. The gap of BND.59 is the owner's to see: the interim reading does not say
what an own tool may show in a solving session.
