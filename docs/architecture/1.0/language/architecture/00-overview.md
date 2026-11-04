# Language architecture: overview and conventions

Status: **draft for the owner. Not accepted.** Nothing in this document is an ADR, a frozen architecture or a chosen
syntax. It is the full text that the [ecosystem partition](../ecosystem-partition.md) promises in its section 17. The
owner confirmed the partition's direction on 3 November 2026. That answered the kickoff's first step only: owner
questions LQ1 to LQ21 stay open, and every chapter keeps them open. Accepting ADRs and freezing the architecture are the
owner's decisions.

"The language" means the project's own language (working name C600 Script, LQ17). "The partition" means the page linked
above. Its IDs are used unchanged: INV-n, KNn, LGn, LBn, EVn, PSn, BMn, BSn, RTn, LX-n, LPn, LQn and its defaults 1 to 20.

## 1. What this document adds

The partition says what the parts are and why. This document says what each part must do, in terms that a test can
check:

- interface contracts between the parts;
- the exact rules of the evidence model;
- typed errors with stable codes;
- conformance tests, each with its evidence class;
- for every open choice, what stays the same under each option and what changes.

It describes no implementation. It presets no syntax, no implementation route and no engine language.

## 2. Chapters

| Chapter | Covers | Partition |
| --- | --- | --- |
| 00 Overview (this page) | Conventions, the system in one view, two walk-throughs, trust, coverage | sections 1, 2 |
| [01 Kernel](01-kernel.md) | KN1 to KN7: the interfaces the language needs from the engine, the session store and the command layer | section 3 |
| [02 Language core](02-language-core.md) | LG1 to LG5: syntax and editions, types, effects, the typed IR, modules | section 4 |
| [03 Libraries and built-in tools](03-libraries.md) | LB1 to LB7, and the research formats they must carry | sections 5, 10.2, 10.3 |
| [04 Environment](04-environment.md) | EV1 to EV5: editor, debugger, source and UI linkage, bindings and recording, docs and learning | section 6 |
| [05 Persistence](05-persistence.md) | PS1 to PS3, the data core, editions, the 0.4 import, the migration format freeze | sections 7, 13 |
| [06 Evidence and certificates](06-evidence.md) | The evidence model, its order and summaries, the label mapping, certificates | section 8.1 |
| [07 The human-solve boundary](07-boundary.md) | The mechanisms, flow control, profiles, the option sets for the joint setting | sections 8.2, 11 |
| [08 One state](08-one-state.md) | Snapshots, derived states, previews and commits, branches | section 8.3 |
| [09 Determinism, budgets and sandbox](09-determinism.md) | The reproducibility key, budgets, capabilities, cross-platform identity | section 8.4 |
| [10 Renderer and platform independence](10-renderer-platform.md) | Display plans, user charts, picking, platform rules | section 8.5 |
| [11 Implementation routes and spikes](11-route-and-spikes.md) | RT1 to RT4, the engine language, LX-1 to LX-9 with their decision rules | section 12 |
| [12 Phases and conformance](12-phases-and-conformance.md) | LP0 to LP6, the conformance suite, dependencies on the other lines | section 14 |
| [13 Error registry](13-errors.md) | Every category and code | default 5 |

The twelve ADR drafts are in [`../adr/`](../adr/). Each records one decision for the owner to accept and points to the
chapters that hold its detail.

Reading paths:

- **To see the whole:** this page, then chapter 08, then chapter 07.
- **To decide LQ1 to LQ21:** partition section 15, then the "Open items" section at the end of each chapter.
- **To build the kernel API:** chapters 01, 08, 09 and 13.
- **To write the library:** chapters 02, 03 and 06.

## 3. Conventions

### 3.1 Clauses and their status

Normative statements are numbered clauses: `KN5.7` is clause 7 of component KN5. Pillars and routes use `EVD` (evidence),
`BND` (boundary), `ONE` (one state), `DET` (determinism), `RPI` (renderer and platform independence) and `RTE` (routes).

Each clause starts with one status tag. The tag says what would have to change for the clause to change.

| Tag | Meaning | What changes it |
| --- | --- | --- |
| **[B]** | Binding: it restates a source in force, named in the clause | That source, through its own process |
| **[D]** | Integrator default: a partition default or a mechanism the partition adopted under every owner option, or a rule that follows from them | The owner overrules it |
| **[P LQn]** | Proposal: it holds if the owner accepts the recommendation of LQn | The owner's answer to LQn |
| **[O]** | Open: not decided here; the clause says what decides it | The named decision |

"Must" and "never" state requirements. "May" states a permission. A clause tagged [P LQn] names what changes under the
alternatives of LQn, where the change is not obvious.

A clause that rests on one of the round table's open obligations says so (section 8).

### 3.2 Signatures

Interfaces use a neutral notation. It fixes meaning, not syntax:

```text
operation name(parameter: Type, optional?: Type) -> Result
  effect:  the effect it needs (LG3)
  lane:    the admission lane, for a command (KN7)
  cost:    how its declared cost is computed (KN7)
  errors:  the error codes it can return (chapter 13)
```

- Type names such as `Snapshot` or `Preview` name concepts and are not final identifiers.
- `A | B` is a typed choice. `A?` is an optional value. `List<A>` is a list. `Digest<level>` is a SHA-256 digest of
  normalised bytes that names the level it covers (KN1.8).
- A signature never implies a language binding. Whether a call is a method, a function or a message is a matter for
  LP1's kernel interface specification.

### 3.3 Identifiers

Default 19 applies. Every identifier in this document carries a prefix of its own, and a reference to another document
names that document: "charter E2", "state contract A1", "DT-M1 Q5 (a)". Stored identifiers (error codes, budget names,
evidence citations) carry a source prefix too.

### 3.4 Conformance tests

A conformance test has the ID `CT-<component>-<number>`, for example `CT-KN5-03`, and states what it checks and its
evidence class. The classes are those of `AGENTS.md`, with the human tests this ecosystem adds:

| Class | Meaning |
| --- | --- |
| source | Source or fixture evidence, from a headless run |
| synthetic | Synthetic geometry or synthetic input |
| windows | Actual Windows or DirectX evidence from a native run |
| performance | A timing on the owner's machine under the measurement rules |
| human | A test with people, in a form the owner sets |

A headless run never supports a Windows, DirectX, input, long-session or performance claim. Chapter 12 collects the tests
by phase. No conformance test exists yet: every test named here is a requirement on LP1 and later phases.

### 3.5 Figures

Every number that a design choice rests on names its evidence class and its source. A figure that was not measured says
so. Performance figures count only from the owner's machine. Sizes counted from a file are source evidence.

### 3.6 Terms

Terms come from the [glossary](../../../../progress/1.0/glossary/glossary.md) (default 8). In particular:

- A **twist** is a turn of one grip by a non-identity element of the active menu (DT-M1 terms): 11 retained twists and 48
  jumble twists per grip under I_a. A **generator** stays one of the 1,200. Every count names its alphabet: keys, twists,
  generators or menu letters.
- **600-cell-Full** and **the jumbling puzzle** are two puzzles with separate model identities. "Jumble on" and "Jumble
  off" are the two modes of one session.
- A **word** is a chronological list of letters. A **literal word** is the list as written, before any normal form.

Chapter 04 lists the entries the glossary gains (EV5).

## 4. The system in one view

```text
                 solver
                   │ keys, pointer                       ▲ frames
                   ▼                                     │
  ┌──────────────── Shell ──────────────┐       ┌── Renderer ──┐
  │ routing · bindings · editor host    │       │ draws plans  │
  │ mints the user-action token (INV-3) │       └──────▲───────┘
  └───┬───────────────────────────▲─────┘              │ adapter
      │ invoke(command, args)     │ view state         │
      ▼                           │                    │
  ┌── Command layer (KN5) ──┐  ┌── View model ─────────┴──────┐
  │ gates · previews ·      │  │ one mode · no puzzle copy    │
  │ receipts · availability │  └───────▲──────────▲───────────┘
  └───┬───────────────▲─────┘          │ revision │ view nodes,
      │               │ queries,       │ bundles  │ display plans
      │               │ proposals      │          │
      │        ┌── Runtime (LG, running LB) ──┐   │
      │        │ interpreter · budgets ·      ├───┘
      │        │ capabilities · user modules  │
      │        └──────────────────────────────┘
      ▼
  ┌── Engine (KN1, KN2, KN3, KN6, KN7) ──┐   ┌── Session store (KN4) ──┐
  │ the one authoritative state          ├──►│ journal · revisions ·    │
  │ exact kernels · value store          │   │ workspace · receipts     │
  └──────────────────────────────────────┘   └──────────────────────────┘
```

The runtime sits beside the path from key to frame, never on it. A key bound to a kernel command row reaches the command
layer without passing through the runtime, and the renderer adopts each engine revision without waiting for a program
(chapter 08).

Where the runtime runs, in the engine process, in a process of its own or in the shell process, is open (LQ9, LX-2). The
picture shows its interfaces, not its process.

## 5. Walk-through: a solver's own tool prepares a word

This walk-through follows one action through the parts. It is informative. The chapters hold the rules.

1. The solver presses a key bound to a language-defined command. The shell finds the binding in its table and invokes
   the command through the command layer (KN5, EV4).
2. The command layer admits the command in the program lane and returns a receipt at once. The runtime starts the
   command's program as a job with its budget (KN7).
3. The program opens a snapshot of the current revision. It reads through batch queries, and every value it receives
   names the revision and the level it came from (KN2, chapter 08).
4. The program builds a literal word and sends a proposal: the word, the revision it read and its provenance block
   (BM2). What the provenance block holds depends on the form of flow control (LQ21).
5. The engine replays the literal word at the current revision, prefix by prefix, and checks protection. It builds its
   own immutable preview object, with an ID, a witness and the digest of the literal letters (R-10, KN5).
6. The view model adopts the preview, and the renderer draws it through the adapter. The solver sees the effect, the
   cost and the provenance label.
7. The solver presses the commit chord. The shell mints a user-action token from that non-repeating key-down. It binds the
   token to the shown preview's ID and literal-letter digest, with the model identity, menu identity, menu-order digest
   and contract revision (INV-3).
8. The engine rechecks the revision, the digest, the full-state hash and protection, and adopts the end state of its own
   preview object. One durable transaction writes the journal event and the receipt (R-06).
9. A program view reads the new revision bundle and recomputes, skipping to the newest revision. Until it finishes, the
   view shows its designed stale state and names the base revision of its last output (charter D2). The old output is
   not drawn against the new state (chapter 08, ONE.11 and ONE.12).

A program never touches steps 7 and 8. It holds no token, and no operation available to it returns one.

## 6. Walk-through: a research run

1. The solver starts a run in the research profile, with a seed and a deterministic budget (BM3, chapter 09).
2. The program derives states from a pinned input with pure kernel calls. Each derived state is an immutable value in
   the kernel's value store, with its derivation and a reachability claim (KN2, INV-1).
3. It stops when it finishes or when a budget ends. The job's outcome is typed. A run that a budget cut off yields no
   negative claim, and a witness it found counts once it is checked (KN7, chapter 06).
4. An independent checker recomputes the result claim by claim and emits a trace (KN6).
5. The run record stores the reproducibility key, the stop reason and the evidence envelope (PS1). The same key gives the
   same result bytes on Windows and on Linux (chapter 09).
6. The session state did not change. Under the interim reading, research output never becomes a preview in a solving
   session (partition section 11).

## 7. What each part is trusted for

The design keeps the part that must be right for the puzzle to stay correct small, and keeps user programs out of it.

| Part | If it is wrong, what can go wrong | What cannot go wrong |
| --- | --- | --- |
| Exact engines, session store, command layer | The puzzle state, the journal or a gate | Guarded by the differential oracles and the repository's critical-path rules |
| Shell's token minting | A commit the solver did not intend | A commit outside the gates: the engine still rechecks everything |
| Independent checkers | A false certificate pass, which is why each has a mutation gate | The session state: a checker changes nothing |
| Language checker and interpreter | A wrong answer, a wrong label, a refused harmless tool, a flow-control hole | The session state: the runtime has no write path, and every proposal is replayed by the engine (INV-2) |
| Libraries and built-in tools | A wrong answer or a wrong display | The session state; evidence: a claim rests on its own certificate and check, not on the library's word |
| User modules | Anything a program can say | The session state, the journal, other sessions, files not granted |
| Renderer | A wrong picture | Mechanical state (INV-8) |

Two consequences follow:

- A bug in the language can make a tool lie. It cannot corrupt a session, commit a word the solver did not see, or turn a
  claim into a proved one.
- The boundary mechanisms that rest on the checker or the interpreter (flow control, BM2) are only as strong as those
  parts. Their limits are stated in chapter 07.

## 8. What the full text does not rely on yet

DT-L1 records two open obligations. Until the mathematician has answered them and the main session has settled the
answers:

- No clause relies on the visible pose class off the lattice.
- No clause relies, without saying so, on what the revised partition adopted from the engineer advisor's cards R2-E1 to
  R2-E4.

## 9. Coverage of the kickoff

The kickoff lists what the architecture must answer at minimum. This table says where each answer is.

| Kickoff item | Where |
| --- | --- |
| Whole ecosystem: core, types and effects, standard library, built-in tools, user modules, packages and versions, editor, macros and bindings, recording, docs and learning path | Chapters 02, 03, 04 |
| Execution kernel: interfaces to engine, session store, command layer, view model, renderer | Chapters 01, 08, 10 |
| Preview and commit, undo, cancellation, async work, resource budgets | Chapters 01, 08, 09 |
| One authoritative state, no second puzzle state | Chapter 08 |
| Exact decisions apart from float display | Chapter 01 (KN3), chapter 02 (LG2), chapter 10 |
| Object identity: piece, sticker, slot, position, pose | Chapter 01 (KN1) |
| Evidence levels, certificates and independent checking | Chapter 06, chapter 01 (KN6) |
| Linked representations, custom maps and charts | Chapter 03 (LB4), chapter 04 (EV3), chapter 10 |
| Restricted search and strategies, conditional simplified models, experiment branches, reproducible records | Chapter 03 (LB5), chapter 05, chapter 09 |
| Stage 14 to 19 techniques and formats | Chapter 03, chapter 05 |
| The boundary mechanism and option sets, not finalised | Chapter 07 |
| Persistence and compatibility, relation to the migration format freeze | Chapter 05 |
| Implementation route and engine language: options and evidence needs | Chapter 11 |
| Phased route with products, acceptance and dependencies | Chapter 12 |
| Owner questions | Partition section 15; "Open items" in each chapter |
