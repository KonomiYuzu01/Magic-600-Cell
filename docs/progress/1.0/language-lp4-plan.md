# Language LP4 plan: the authoring environment, functional part

Status: plan of 22 November 2026, written after LP3's exit
([C10 and LP2 exit](../../wiki/decisions/owner-decisions-2026-11-22-c10-lp2-exit.md)) and before any LP4 code. It
records the integrator's selections, and the owner may overrule any of them. It is no ADR and freezes nothing. It
covers LP4's functional work, which PHS.41 lets start after LP3; the editor's look is not in it (PHS.26, PHS.28).

Inputs: partition LP4 ([ecosystem partition](../../architecture/1.0/language/ecosystem-partition.md), section 14);
[chapter 12](../../architecture/1.0/language/architecture/12-phases-and-conformance.md), section 6 (PHS.26 to
PHS.29), sections 9 and 10 (PHS.38, PHS.41) and section 15.5; [chapter 04](../../architecture/1.0/language/architecture/04-environment.md),
EV1 to EV5; the owning tables of CT-LG4-07 (chapter 02), CT-LB6-01 and CT-LB6-04 (chapter 03), CT-PS1-15, CT-PS1-17,
CT-PS1-24 and CT-PS1-25 (chapter 05), CT-BND-03 (chapter 07) and CT-ERR-18 (chapter 13); H-08's stored
[command table](command-table.json); the owner decisions of 22 November 2026
([design and art path](../../wiki/decisions/owner-decisions-2026-11-22-design-art-path.md),
[ADR-D01 overrules](../../wiki/decisions/owner-decisions-2026-11-22-adr-d01-overrules.md),
[LX bars](../../wiki/decisions/owner-decisions-2026-11-22-lx-bars-freeze.md)); the [LP3 plan](language-lp3-plan.md).

Evidence kind for everything this plan produces without the owner's machine: source and fixture, and synthetic where a
test's class says so. Nothing here is Windows interaction, DirectX, input, long-session, performance or human evidence.
CT-EV4-05 (windows), the editor latency bars (performance) and the three human tests need the owner's machine or the
owner's protocol (PHS.27, PHS.64).

## 1. What LP4 delivers

PHS.26's four products, each with the clauses it implements and the section 15.5 tests it makes runnable. The 51
tests are 42 source, 5 synthetic, 1 windows and 3 human.

| Product | EV clauses | Tests (class) |
| --- | --- | --- |
| Editor services: recovery, completion, diagnostics, navigation, version separation | EV1.1 to EV1.12 (EV1.9's latency is LX-4's; EV1.13 is open) | CT-EV1-01, CT-EV1-02, CT-EV1-03, CT-EV1-04, CT-EV1-07, CT-EV1-08 (source); CT-EV1-05 (synthetic) |
| Debugger and replay: read-only stepping, replay, checker traces, cost profile | EV2.1 to EV2.12 (EV2.9 is open) | CT-EV2-01 to CT-EV2-04 and CT-EV2-06 to CT-EV2-11 (source); CT-EV2-05 (synthetic); CT-LG4-07 (source) |
| Linkage and authoring actions: declared-part round trip; overlay rows and the palette; bindings and rebind; macros; semantic recording, playback and conversion | EV3.5 to EV3.7; EV4.1 to EV4.23 | CT-EV3-05; CT-EV4-01, CT-EV4-07 to CT-EV4-13, CT-EV4-15 to CT-EV4-20; CT-PS1-15, CT-PS1-17, CT-PS1-24; CT-BND-03; CT-ERR-18 (source). CT-EV4-06, CT-EV4-14, CT-PS1-25 (synthetic). CT-EV4-05 (windows) |
| First edition of docs and learning | EV5.1 to EV5.18 | CT-EV5-01, CT-EV5-02, CT-EV5-04, CT-EV5-06, CT-LB6-01 (source); CT-EV5-03, CT-EV5-05, CT-LB6-04 (human) |

PHS.27's two acceptance items, beside the 51 tests run whole (PHS.62, PHS.63):

| Acceptance item | Without the owner | Waits for |
| --- | --- | --- |
| Editor latency bars | The editor services, the shell's editor host and LX-4's module set on them (LP4-1, LP4-3) | The owner's adoption of the bars for LP4 (section 8, item 1) and LX-4's protocol run on LP4's frozen candidate on the owner's machine (PHS.27, PHS.64) |
| Non-author tests | The tasks and fixtures (LP4-10, LP4-11) | The owner's form and success criteria (EV5.12, PHS.27), run at minimal scale (owner, 16 November 2026) |

The functional exit needs every acceptance item's evidence on one candidate, the owner's understanding confirmation and
section 9's LP4 answer state (PHS.29). While the editor's look is open, LP4 is never reported as complete (PHS.28).

## 2. What may start now and what waits

| Work | State | Waits for |
| --- | --- | --- |
| EV1 services, activation, overlay rows and the palette, the shell's write-check-run flow | May start | Nothing: LP3 exited, H-08 is frozen at 198 commands, the shell exists (C9) |
| The flow's preview of a solver's own word | May start as data: the run shows the program's view output and proposed word | How author classes map to the solver's own programs (ADR-D01.8, settled with ADR-D01's text at A7); until then BND.84 refuses a program-produced word at preview in a solving session |
| EV2 stepping, inspection, replay, profile, journal travel | May start | CT-EV2-07 needs one checker that writes KN6.4's per-claim trace; CT-EV2-10 needs EV2.9's contract (section 8, item 3) |
| EV3.7 round trip, EV4 bindings, macros, recording | May start after the pieces they build on (section 4) | CT-EV4-05 runs on the owner's machine; the undo tests wait for the reading of contradiction X1 (section 7) |
| EV5 docs, principle records, glossary check, learning fixtures | May start once a session is free; last in the order of value | EV5.14's first-use order (section 8, item 5); the human tests wait for the owner's form |
| The editor's look | Waits | Freeze round 3, after stage 2 (owner, 22 November 2026, item 1), conditioned by the accepted design-system ADR (PHS.28) |

## 3. Integrator selections (the owner may overrule them)

1. **Crates, existing first.** The work goes into these crates of `rust/`, with no external crate:
   - `c600-syntax` (LP2's parser): additive only. Comment and trivia retention, which M7 deferred to the authoring
     services, and the recovered structure that completion and navigation read (EV1.2).
   - `c600-module`: `activate_module` (EV1.11). It already holds closures, lockfiles and the approval bound to the
     closure digest, which EV1.11's check reference must cover.
   - `c600-tools` and `c600-runtime`: a run binds wholly to one active digest and stays on it after activation
     (EV1.7, EV1.11).
   - `c600-command` (critical, LP3 selection 6): overlay rows (KN5.3, EV4.1), `command.find`'s palette data (EV4.2),
     the keymap and macro rows (EV4.7 to EV4.9) and the `recording.*` rows with capture at the layer (EV4.10, EV4.11,
     EV4.22).
   - `c600-session`: PS1.30's recorded steps and PS1.44's `PlaybackState` as durable workspace records, in the frozen
     data core's forms.
   - `c600-input`: bindings to absolute arguments, layouts, rebind (EV4.3, EV4.7, EV4.8).
   - `c600-run`: an additive stepping interface on RT1's evaluator at LG4/DET.20 positions (EV2.12).
   - `c600-durable`: the crash cases of CT-EV4-14 and CT-PS1-25.

   New crates:
   - `c600-editor`: EV1's pure services and EV3.7's `edit_declared_part`. It depends on `c600-syntax`, `c600-check`,
     `c600-ir` and `c600-digest` only, so a service runs no user program (EV1.9).
   - `c600-debug`: EV2. It depends on `c600-run`, `c600-ir`, `c600-syntax` and `c600-digest`. It reads journal
     rows through a read-only trait that the host implements over the command layer's query rows, so it has no path
     to a write (EV2.1, EV2.8).
   - `c600-docs`: EV5.2 to EV5.5's reference and principle-record generation from checked metadata and the IR's
     call graph. It depends on `c600-syntax`, `c600-check`, `c600-ir`, `c600-module` and `c600-digest`.

   Outside the language workspace:
   - `renderer/bridge/editor`: a new library `c600-shell-editor`, beside `lx4`, over `c600-editor` and
     `c600-module`.
   - `renderer/shell/project/Editor/`: the shell's greybox editor panel on Godot's own text control, with no
     appearance work.
   - `renderer/bridge/lx4`: re-pointed onto `c600-editor`, so that LX-4 times the product's services.

   New crate edges stay inside the layer interfaces' boundaries. `c600-command` and `c600-input` keep no path to the
   interpreter (ONE.48). An edge or a format change that would cross a frozen layer boundary, or change PS1.30's or
   PS1.44's frozen forms, is not taken: it goes to the integrator as a PHS.52 change. A new product dependency needs
   the owner's approval; none is planned.
2. **Runner and records (PHS.65).** As in LP3:
   - `cargo test --locked --offline` in `rust/` through `tests/test_language_rust.py`;
   - the bridge crates' tests in `renderer/bridge`;
   - `tests/test_renderer_shell.py` for the shell's fixed points;
   - Python checks for the glossary listed in `tools/ci/run_headless.py`.

   A test that implements part of a conformance test names the ID and the part. The ID counts only when its whole
   definition runs (PHS.62, PHS.63). The evidence record is the CI run of the merged commit.
3. **Reviews.** Each change to `c600-command` or `c600-session` gets one read by a fresh Claude subagent on a smaller
   model before commit, given the diff and the piece's acceptance check (`AGENTS.md`, "Codex minimum"). A session
   store change also runs the mechanics and persistence checks that `AGENTS.md` lists. No Codex call.
4. **The human-solve boundary.**
   - Editor services suggest and check; they run no program, and a suggestion grants nothing (EV1.3, EV1.9).
   - The debugger has no write path (EV2.1).
   - Unattended playback plays view and workspace steps only and stops at every commit-lane row (EV4.12, EV4.18).
     A recording never commits and never replaces a preview (owner, 22 November 2026; ADR-D01.9).
   - No piece chooses, ranks or outputs a solving macro. Docs and learning fixtures read recorded instances only.
5. **Greybox only.** The shell's editor panel, palette and recording controls are greybox placeholders for stage 2's
   framework (freeze round 2). Final messages, layout and look wait for round 3 (EV1.12).
6. **One line, one pull request.** The LP4 line's work goes into one pull request, kept apart as commits (owner,
   22 November 2026). Parallel sessions branch from the line's branch, and the line's integrator merges them there
   (section 8, item 6, asks whether phase I lands earlier).

## 4. Pieces in order

Phase I gives stage 2's `.6cs` flow (write, check, run a macro in `renderer/shell`). Phases II to IV follow in the
order of value; pieces in different files may run at the same time (section 5).

| Piece | Phase | Content | Crates and files | Depends on | Tests whole at its end |
| --- | --- | --- | --- | --- | --- |
| LP4-1 | I | EV1's services: parse with recovery and trivia, complete, diagnose, hover, definition, rename; `SourceRange` conversion (EV1.10); typed refusal data (EV1.12) | `c600-syntax` (additive), `c600-editor` | Nothing | CT-EV1-01, CT-EV1-02, CT-EV1-03, CT-EV1-05, CT-EV1-07 |
| LP4-2 | I | Version separation and activation: a running tool stays on its checked digest; `activate_module` with its atomic compare | `c600-module`, `c600-tools`, `c600-runtime` | Nothing | CT-EV1-04, CT-EV1-08 |
| LP4-3 | I | The shell's write-check-run flow: the editor host library, the greybox panel, check, explicit activation (EV1.13 stays open), run as a program job through the host, preview and the solver's commit through the key route; LX-4's module on `c600-editor` | `renderer/bridge/editor`, `renderer/bridge/host`, `renderer/bridge/lx4`, `renderer/shell/project/Editor/` | LP4-1, LP4-2 | None alone; its headless flow test is the stage 2 flow's evidence (source) |
| LP4-4 | I | Overlay rows (EV4.1) and the palette's data from KN5.6's table (EV4.2), so a checked user command appears and is invoked by key or palette | `c600-command` | Nothing | CT-EV4-01 |
| LP4-5 | II | Stepping and inspection: the evaluator's stepping interface, `debug_step` and `inspect_value`, suspended kernel calls, word ranges (EV2.1, EV2.2, EV2.11, EV2.12) | `c600-run` (additive), `c600-debug` | LP4-1 for source maps | CT-EV2-01, CT-EV2-02, CT-EV2-06, CT-EV2-11, CT-LG4-07 |
| LP4-6 | II | Replay, mismatch, cost profile, journal travel, checker stepping, pinned research runs (EV2.3 to EV2.10) | `c600-debug`, `c600-tools` (host side of the read trait) | LP4-5 | CT-EV2-03, CT-EV2-04, CT-EV2-05, CT-EV2-08, CT-EV2-09; CT-EV2-07 if a KN6.4 trace producer exists; CT-EV2-10 after EV2.9's contract |
| LP4-7 | III | Declared-part round trip (EV3.7) on the built-in tools' declared parameters and layouts | `c600-editor`, the tools' `.6cs` sources | LP4-1 | CT-EV3-05 |
| LP4-8 | III | Bindings, layouts and rebind (EV4.3, EV4.4, EV4.7, EV4.8); macros with identity, alphabet and exact length, word and trace keys (EV4.9) | `c600-input`, `c600-command` | LP4-4 (same crate) | CT-EV4-06, CT-EV4-07; CT-EV4-05 on the owner's machine |
| LP4-9 | III | Semantic recording, playback and conversion (EV4.10 to EV4.23): capture at the layer, PS1.30's records, PS1.44's playback state and recovery, the eligibility table, the action map, conversion to draft and source | `c600-command`, `c600-session`, `c600-durable` | LP4-8 (same crate) | CT-EV4-08 to CT-EV4-20, CT-PS1-15, CT-PS1-17, CT-PS1-24, CT-PS1-25, CT-BND-03, CT-ERR-18 |
| LP4-10 | IV | Reference docs and principle records (EV5.2 to EV5.5, EV5.13) for the three built-in tools and the library | `c600-docs`, the tools' and library's record data | LP4-1 | CT-EV5-01, CT-EV5-06, CT-LB6-01 |
| LP4-11 | IV | The glossary check (EV5.10, EV5.15, EV5.18), the integrated entries with diagrams, the learning path's fixture tasks (EV5.6) and soft disclosure (EV5.8), under the mathematician's review | `docs/progress/1.0/glossary/`, learning fixtures, `c600-command`'s disclosure data | EV5.14's first-use order; LP4-9 for `c600-command` | CT-EV5-02, CT-EV5-04 (BS3's branch waits for LQ2) |

Unattended playback covers view and workspace steps only, never a commit (EV4.12, EV4.17). CT-EV5-03, CT-EV5-05 and
CT-LB6-04 run under the owner's form once LP4-10 and LP4-11 exist.

### Acceptance per piece

Every piece also meets these:
- `cargo test --locked --offline` and `cargo fmt --check` per member pass (`cargo fmt --all` hits the Windows
  command-line limit in worktrees);
- each listed test's part exists and names its part;
- no file under `docs/architecture/1.0/language/` changes, and the eight language checkers and
  `tests/test_language_scans.py` pass;
- what does not finish is listed in `rust/README.md` as not built.

Piece by piece:

- **LP4-1.**
  - Recovery: an incomplete declaration gives recovered spans and completion context and no runnable module.
  - Diagnostics: every EV1.4 fixture kind gives its own code and range.
  - Hover, definition and rename resolve the pinned graph; rename refuses changed bytes with `EV1:stale.editor_source`.
  - `SourceRange`: EV1.10's boundary triples and its two malformed UTF-8 inputs pass.
  - No service links `c600-run` (a dependency check in the crate's tests).
- **LP4-2.**
  - Activation is the only switch, and every refusal leaves A active.
  - A suspended run on A finishes on A after B activates.
  - An invocation racing activation binds wholly to one side.
- **LP4-3.**
  - A headless test drives the editor library and the host together: type a `.6cs` macro, see diagnostics, fix it,
    activate, run as a job, preview, commit by the key route's token, undo.
  - Every step goes through the command layer, and the runtime gets no token.
  - `tests/test_renderer_shell.py` covers the new panel's fixed points.
  - LX-4's module set still checks clean on the product services.
  - The on-screen flow joins the owner's next sitting.
- **LP4-4.**
  - A language row registers only as `view`, `workspace`, `query` or `file` with a grant.
  - Availability uses the stored table and calls no program.
  - An unavailable entry keeps its place with its reason.
  - Gets the fresh read of section 3.
- **LP4-5 and LP4-6.**
  - Stepping changes no journal, workspace or session digest.
  - Replay matches bytes, outcome, coverage and charges.
  - A planted difference reports `EV2:evidence.replay_mismatch`.
  - A safety-stopped record stays incomplete.
  - Duplicate step requests across a suspended call do not redispatch or recharge it.
- **LP4-7.** A declared parameter, layout or default edit gives source edits that recheck; an undeclared field gives
  `EV3:input.round_trip_undeclared`.
- **LP4-8.**
  - Duplicate and prefix conflicts fail per routing class and under each declared layout.
  - Only rebind changes the applied map.
  - Both puzzles share one macro base with literal order and exact length kept.
  - Gets the fresh read.
- **LP4-9.**
  - CT-EV4-12's table is enumerated whole, and only eligible view, workspace and no-job query rows play unattended.
  - No token or preview ID is stored.
  - The crash points of CT-EV4-14 recover by outcome lookup first, advancing at most once.
  - Conversion writes nothing unless every step passes.
  - Gets the fresh read, and the session store's checks run.
- **LP4-10.**
  - Each built-in tool has every `PrincipleRecord` part, with its source and evidence digests and a runnable
    real-geometry fixture.
  - A changed decomposition marks the hand parts stale.
  - Explanation words come from chapter 06's functions.
- **LP4-11.**
  - The glossary check refuses an unknown term, an ambiguous citation and a definition above its level.
  - Every integrated entry has its diagram evidence (EV5.15), made with the local Mermaid CLI.
  - The learning path's prerequisites form a DAG.
  - Soft disclosure never locks a command.

Stop condition for each piece: its acceptance passes and its commits are on the line's branch. It stops earlier, and
reports to the line's integrator, when it needs a frozen-file change (PHS.52), a product dependency, an owner answer,
or a change to another session's files.

## 5. Parallel sessions

Two writers never touch the same files. `c600-command` has one owner at a time, so LP4-4, LP4-8, LP4-9 and LP4-11's
disclosure data run in that order in one session. The workspace `Cargo.toml`, `Cargo.lock` and `rust/README.md` are
shared. Each session adds only its own member line and README section, and the line's integrator merges them.

| Wave | Session A | Session B | Session C |
| --- | --- | --- | --- |
| 1 | LP4-1 (`c600-syntax`, `c600-editor`) | LP4-2 (`c600-module`, `c600-tools`, `c600-runtime`) | LP4-4 (`c600-command`) |
| 2 | LP4-3 (`renderer/`) after LP4-1 and LP4-2, then LP4-7 (`c600-editor`) | LP4-5, then LP4-6 (`c600-run`, `c600-debug`) | LP4-8 (`c600-input`, `c600-command`) |
| 3 | LP4-10 (`c600-docs`) | LP4-11's glossary and fixtures (`docs/progress/1.0/glossary/`) | LP4-9 (`c600-command`, `c600-session`, `c600-durable`), then LP4-11's disclosure data |

## 6. Estimates

Agent days are working days of one session. They are rough, and they do not bind (PHS.1).

| Piece | Agent days |
| --- | --- |
| LP4-1 | 2 |
| LP4-2 | 1 |
| LP4-3 | 2 |
| LP4-4 | 1.5 |
| LP4-5 | 2 |
| LP4-6 | 2 |
| LP4-7 | 0.5 |
| LP4-8 | 1.5 |
| LP4-9 | 3.5 |
| LP4-10 | 2 |
| LP4-11 | 1.5 |
| Integration, fresh reads, README and the exit record | 1.5 |
| Total | about 21, or about 8 to 10 working days with three sessions; phase I alone about 4 |

Owner hours, apart from agent days:
- one milestone card: about 0.25 hours;
- the non-author form: about 0.5 hours;
- the owner-machine part, attended, inside the S2 sitting: about 0.5 hours. It covers CT-EV4-05, LX-4 on LP4's
  candidate and the greybox flow.
- the non-author tests at minimal scale: about 1 to 2 hours;
- understanding confirmation and the exit sign-off: about 0.5 hours.

That is about 3 to 4 owner hours in all.

## 7. What stays open

- **The editor's look:** freeze round 3, after stage 2, under the design-system ADR (PHS.28). EV1.13's trigger
  (automatic or explicit activation) stays the owner's and the design line's; LP4-3 builds the service and an
  explicit greybox action only.
- **The human acceptance under PHS.27:** the owner's form and criteria (EV5.12), and CT-EV5-03, CT-EV5-05 and
  CT-LB6-04. The mathematician checks the content. No pass rate is invented.
- **The LX-4 latency bars for LP4.**
  - LX-4's bars became 100 ms, and a sample without a confirmed frame is counted apart (owner, 22 November 2026).
  - PHS.27 still leaves the adoption of LP4's editor bars to the owner.
  - LP4 changes the services that LX-4 times, so a run on LP4's frozen candidate is owed (PHS.27).
  - The confirmation gap is product work after the freeze.
- **EV2.9's replay of effects, EV5.9's BS3 gate (LQ2's joint setting) and EV5.14's first-use order.**
- **ADR-D01.8's author classes for the flow's solving preview:** settled with ADR-D01's text at A7.
- **Contradictions for the integrator.** Frozen text is not edited. The integrator files each one as a wiki question
  page (`docs/wiki/SCHEMA.md`):
  - **X1.** EV4.15 keeps one durable undo order for puzzle and workspace actions. EV4.18, EV4.20 and EV4.23 target
    `session.undo` and `session.redo` in it, and CT-EV3-07, CT-EV4-15, CT-EV4-17, CT-EV4-18 and CT-EV4-20 test it.
    ADR-D01.14 and .15 give two histories with their own keys and multi-step rollback. They overrule KN5.19 and
    KN5.20 ([record](../../wiki/decisions/owner-decisions-2026-11-22-adr-d01-overrules.md)), but that record lists
    only EV4.16 in chapter 04. Until the reading is recorded, LP4-9 builds on the command layer's undo entries as they
    stand on main and names which reading each test uses.
  - **X2.** PHS.28, PHS.41, PHS.44, section 19's design-system row and CT-PHS-03 hold the design line until LP2 to LP5
    are done apart from LP4's editor look. The owner opened K2 beside LP4 and LP5 on 22 November 2026 (item 4), with
    the ADR-D01 line already running, so CT-PHS-03 fails as written. No overrule record names these clauses.

## 8. Owner items for the integrator's milestone card

Recommended option first. The integrator asks them; this plan does not.

1. **LP4's editor bars.** Recommended: adopt LX-4's bars of 22 November 2026 as PHS.27's editor bars, and run LX-4's
   protocol on LP4's frozen candidate in the next owner-machine batch. Alternative: set separate editor bars.
2. **The non-author form.** Recommended: the owner sets it when LP4-10 and LP4-11 exist, on a one-page draft from
   the integrator at minimal scale (fixture and transfer tasks from EV5.6's steps, with correctness, transfer, help
   and time recorded apart). Alternative: set it now.
3. **EV2.9's replay of effects.** Recommended: inspect recorded effects without re-executing them, recorded as a
   PHS.52 revision of chapters 05 and 09. Alternative: isolated replay sinks with captured, digest-pinned inputs.
4. **EV1.13 in stage 2's greybox.** Recommended: explicit activation by a user action; round 3 may change it.
   Alternative: automatic activation after a clean check.
5. **EV5.14's first-use order.** Recommended: the manual. Alternative: the shelved tutorial.
6. **Pull request shape.** Recommended: phase I, the `.6cs` flow, lands as its own pull request before the greybox
   build of the five flows, and the rest as the line's one pull request. Alternative: one pull request for all of
   LP4's functional work.
7. **Contradictions X1 and X2.** Recommended reading: ADR-D01.14 and .15 extend to EV4.15 and the tests built on it,
   and the 22 November item 4 overrules chapter 12's design-line hold. Both are recorded as overrules. Alternative:
   amend the clauses under PHS.52.
