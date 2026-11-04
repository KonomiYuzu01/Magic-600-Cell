# DT-L1: The language's ecosystem and architecture

Status: **outcome written; owner questions pending.** Opened on 3 November 2026 by the main session under the
[round protocol](../README.md), after the owner's
[decision of 3 November 2026](../../../../wiki/decisions/owner-decisions-2026-11-03.md) that the engineering line
develops the project's own language first. Participants: the mathematician and the engineer advisor only; the front-end
designer and the interaction artist wait until the language's development is complete. Two rounds were run, then the
outcome.

## Question

How is the ecosystem of the project's own language (working name C600 Script) divided into components with clear
boundaries, so that it covers everything that complete restoration (600-cell-Full and the jumbling puzzle) and the open
jumbling research need, and which questions does the owner decide before the full architecture is written?

This round reviews step 1 of the owner's kickoff: the
[ecosystem partition and owner questions](../../../../architecture/1.0/language/ecosystem-partition.md). The full
architecture document and the ADR drafts come after the owner confirms the direction.

## Owner input (kickoff of 3 November 2026, summarised in English)

- This stage settles the complete ecosystem and architecture only; no implementation code.
- The architecture covers every capability that complete restoration and the open jumbling research use. The objects
  that the handoff documents name are a small part and are not the boundary.
- It answers at least: the whole ecosystem (core, types and effects, standard library, built-in tools written in the
  language where possible, user modules, packages and versions, the editor with completion, diagnostics, debugging and
  the link between source and graphical UI, macros and key bindings, recording, documentation and the learning path);
  the execution kernel (interfaces to the engine, session store, command layer, view model and renderer; preview and
  commit, undo, cancellation, asynchronous tasks, resource budgets; one authoritative state and no second puzzle
  state); mathematical correctness (exact decisions apart from floating-point display; the identities of piece,
  sticker, slot, position and pose; evidence levels proved, conditional, finite instance, not found within budget and
  incomplete; certificates and their independent checking); research capabilities (linked representations such as
  geometry, trees, relations, cell complexes and state graphs; custom maps and charts; restricted search and
  strategies; conditional simplified models; experiment branches and reproducible records; the proof techniques and
  data formats of restoration stages 14 to 19); the boundary mechanism for tools a solver develops (option sets with
  trade-offs, set together with the owner; until then the built-in tools keep the 2 October boundary); persistence and
  compatibility (source, modules, macros, recipes, bindings and evidence, their versions and migrations, and the
  relation to the 1.0 migration format freeze); the implementation route (own interpreter or an embedded runtime, and
  the engine language, as options with the evidence each needs); the phased route with products, acceptance and
  dependencies; and a list of owner questions, two or three lines each with a recommendation and alternatives.
- The architecture is independent of the renderer, which the owner closes and freezes in parallel.
- Process: the partition and the question list first; a Codex Astra plan check; the full proposal reviewed by Astra in
  at least two parallel shards; ADR drafts and the architecture document in a pull request; the owner accepts ADRs and
  freezes the architecture. The earlier schedule is set aside and no gate is relaxed. Subscription quota only: no API
  team and no new costs.
- The round table also helps the owner understand: at each stage the mathematician and the engineer advisor explain the
  key concepts, the mathematics and the trade-offs in plain language, and the owner confirms understanding before
  anything is settled. The mathematician watches over the parts of the ecosystem that let users understand principles
  (documentation, explanations, provenance, the learning path), because the 3 November principle presumes that
  understanding.

## Inputs and handoffs

- Binding: `docs/progress/1.0/charter-2.0-draft.md` (sections 2, 4 and 5), `docs/progress/1.0/command-table.md` (H-08),
  `docs/progress/1.0/solving-workflow.md`, `research/jumbling/state-contract.md`, `research/jumbling/render-contract.md`,
  the owner decisions of 2 October (human-solve boundary), 17 October (repair list), 27 October (handoff) and
  3 November 2026, and `docs/progress/1.0/requirements-from-screening.md`.
- Settled design-table answers that stay: DT-M1 (command and keyboard model) and DT-J1 (jumble solver aids), both of
  22 October 2026.
- Research data: `docs/progress/1.0/jumbling-handoff/research-data.md`, `research/jumbling/restoration/README.md` and
  stages 14 to 19.
- Reference implementations: `research/jumbling/sim/` (J1) and `research/jumbling/differential/` (EN2).
- Open technical choices: `docs/wiki/questions/engine-language-rust.md`, `docs/wiki/decisions/renderer-candidates.md`.
- References only: three private handoff documents of 2 and 3 November 2026 (not in the repository).

## Out of scope

- Implementation code, a frozen syntax, UI layouts and looks, DT-J2 to DT-J4, the renderer selection, the engine
  language decision itself, restarting the research, any paid API run.

## Round 1

### Engineer advisor

#### E1

- id: E1
- on: I-1, I-2, section 9 (where the runtime runs), 8.3, Q9
- kind: proposal
- claim: The runtime can stay a pure client of the command layer and still meet E2 and R-14 only if it stays off the turn's critical path. That means two things: a key bound to a kernel command row is dispatched without waiting for the interpreter or its jobs, and the renderer adopts each engine revision bundle without waiting for program output, which carries its own base revision and shows the D2 stale state until it is recomputed. Of the three placements, the shell process carries the most risk, so X-2's fault-injection run should include it and it stays out unless it passes; X-2 also decides between an engine-process isolate and a process of its own.
- evidence:
  - E2 is measured end to end. It is "the B4-12 M1 interval" (docs/progress/1.0/charter-2.0-draft.md:89), which ends at "the first correct full frame after adoption" (docs/wiki/decisions/b4-12-measurement-decisions.md:36).
  - The path inside that interval is: key, command layer, engine apply, the store's durable transaction (R-06; the renderer must not draw an uncommitted state, docs/progress/1.0/command-table.md:26), bundle adoption, frame.
  - Only the frame has a 1.0 measurement. S-B W3 re-acceptance gave a pooled 788.16 fps, p99 1.493 ms and max 3.855 ms (performance evidence, owner's RTX 4070 Laptop GPU, bare probe, build d41020f6…; docs/wiki/evidence/s-b-probe-results.md:52-56). The 0.4 "26 ms turn" (docs/progress/0.4.1/screening-findings.md:58) is a 0.4 headless timing, not evidence.
  - Keys: B4 makes the IR "shared by code, GUI and keys" (partition line 122). A key map is defined as (key, context) mapped to a command ID with absolute arguments (docs/progress/1.0/design-table/topics/command-keyboard-model.md:917), so a kernel row needs no interpreter step.
  - Adoption already works one revision at a time:
    - one revision-bound bundle, never a mixed frame (docs/architecture/1.0/10_V1_ARCHITECTURE.md:219);
    - results declare which revisions they depend on (:221);
    - one adoption per revision (research/jumbling/render-contract.md:14);
    - the D2 stale state (charter:106) and R-02 (docs/progress/1.0/requirements-from-screening.md:12).
  - Program views that recompute per revision should skip to the newest revision (latest-wins, as ENG-0J does for release checks at docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md:133). A burst of typed twists then never queues view work.
  - R-14 (requirements-from-screening.md:31) then holds by construction, because `invoke` returns a receipt at once (partition line 111).
  - Reads should also go through the command layer: journal travel goes through query rows, so the engine stays the only process that opens the session files. Partition line 252 currently has the runtime read the journal directly.
  - Why the shell process is the riskiest placement:
    - User programs would share the crash and memory domain of the window and renderer, whose frame pacing the gate measures. V03.2 made the same argument against embedding CPython in the shell (10_V1_ARCHITECTURE.md:175-176).
    - If the runtime were written in the shell's language, its host would change with the renderer selection (docs/wiki/questions/engine-language-rust.md:23).
    - The checker may still link into the shell as a library for editor latency (X-4). Only program execution leaves the shell.
  - The rule that decides X-2 is in E4.
- needs_owner: true. Placement is part of the owner's Q9, and this card adds a pass condition for one option.

#### E2

- id: E2
- on: I-1, I-3, A5, 8.3 (derived states and the commit path)
- kind: objection
- claim: Derived states are not a hidden second state only if three rules are added:
  - they carry no session revision, so a drawing can show them only through a display plan that names their derivation, never as the adopted state;
  - the only state a commit adopts is the engine's own preview object, never a state or digest a program supplies;
  - they live in a kernel value store, by digest, charged to the program's memory budget.

  A5 must also say which preview a commit names. Bind the user-action token to the ID and word digest of the preview on screen, because if a commit takes "the pending preview", none of A5's four rechecks catches a program that replaces it at the same revision.
- evidence:
  - Renderer rules: drawing a state that is not committed is forbidden (command-table.md:26); Global and Local are linked to one context and revision (charter:112, D8); a drawing adopts one revision at a time (render-contract.md:14). A derived state therefore needs its own identity in display plans (derivation digest and base revision). How it looks is a question for the design line later.
  - The commit does adopt a preview's end state: "the commit takes the end state from that preview when the revision and the word digest are unchanged; nothing is reused from an earlier revision" (ENG-0J:138). So I-1's "never committed" (partition line 42) should name the engine's preview as its one exception.
  - Size (source, arithmetic):
    - one 600-cell-Full state is 1,039,200 bytes of int32 labels (`slot_piece` in assets/model.npz is `<i4` with shape 259,800, read from its header; core.py:18 hashes `<i4` labels);
    - in 0.4's PuzzleState, with its per-piece arrays (core.py:257-268), it is about 3.2 MB;
    - a search that keeps 1,000 states therefore needs about 1 to 3 GB, and the memory budget must count them.
  - Counterexample sketch (preview swap):
    - 0.4 keeps one pending preview with its own token, replaced by each new preview (session.py:159). Its commit checks that token, then head, rev and pre-state (session.py:163-164).
    - The partition names only a user-action token (lines 111 and 253) and does not say which preview a commit names.
    - A program prepares preview A at revision r, and the screen shows A.
    - After a parameter change, the program prepares B, still at r.
    - The solver's commit chord lands before B is drawn, and B commits. Revision, word digest (B's own), state hash and protection all pass.
  - Fix: the shell mints the token from a non-repeating key-down (R-18) and binds it to the preview ID and word digest it adopted. The engine refuses a mismatch as stale, with a visible explanation (10_V1_ARCHITECTURE.md:221).
  - Test: a headless test swaps the pending preview between adoption and commit; the commit must be refused.
- needs_owner: false. The rules implement I-1 and I-3 as written. Whether a program may replace a shown preview at all is a later design-line question.

#### E3

- id: E3
- on: A7, 8.4, B3, B4; A5 (availability); DT-M1 Q4 and Q5
- kind: objection
- claim: Four parts of A5, A7 and B3 do not yet fit 8.4, "A never calls B", or the settled DT-M1 answers:
  - a time budget gives one reproducibility key different results;
  - "a commit cancels jobs that read the replaced revision" ends a research run at the solver's next twist;
  - the engine cannot compute the availability of a language-defined command without running language code;
  - a view that writes the workspace on every revision makes undo stale under Q4 (a).

  Proposed fixes:
  - deterministic budgets (steps, memory in logical units, and the cost each kernel call declares before admission), with wall-clock time only a safety stop whose result is incomplete and not reproducible;
  - jobs with a pinned input, which survive commits but are never shown against the current revision;
  - availability as the checker's static verdict plus the engine's availability of the kernel rows the command may invoke;
  - workspace writes only inside a command the solver invokes, as one undoable action.
- evidence:
  - Budgets:
    - The key includes budgets, and "the same key gives the same exact result, bit for bit" (partition lines 231-233). A7 budgets "steps, memory, time and output size" (line 113).
    - Counterexample: one key with a time budget, run once idle and once under load, is cut off at different steps.
    - Memory should be counted in logical units, because allocators differ between Windows and the Linux headless runs (I-9).
    - A declared cost already exists: the exact expanded length and cost are shown before any run (lines 558-559; ENG-0J:29).
  - Cancellation:
    - The owner chose Q5 (a): a commit "cancels the running jobs that read the revision it replaces …; file jobs finish" (command-keyboard-model.md:944, :978).
    - The filter forbids showing a per-revision result against a newer state (:799).
    - Long jobs exist: the research repair list takes 3 to 5 minutes per cap (docs/progress/1.0/jumbling-observation.md:178, research timing); 0.4 answered a lookup only after 64.8 s behind a 2.9M-primitive preview (screening-findings.md:65, 0.4 headless timing); words of 672,417 twists are on record (ENG-0J:84).
  - Availability:
    - "A never calls B" (partition line 96).
    - The engine computes the availability table (line 111), and language-defined commands enter that table (line 166).
    - The command layer asks the engine for availability (command-table.md:23).
    - The palette reads the table once per revision, with no engine call per keystroke (command-keyboard-model.md:840).
  - Undo:
    - Q4 (a) keeps one order of actions across the journal and the workspace, and refuses an undo as stale when another action landed first (command-keyboard-model.md:943, :977; XM3 at :926).
    - B3 grants a "change the workspace" effect (line 121), and views are evaluated per revision (line 122), so a view with that effect would make every undo stale.
    - The effect types can rule this out: views get no workspace-write effect.
  - A demonstrated failure that is still open at this head: `session.undo` is gated `not_busy` (docs/progress/1.0/build_command_table.py:36), and every commit row must carry `not_busy` (:155-156). This is DT-M1 fix-first item 3 (command-keyboard-model.md:810).
    - With programs running as jobs, undo would be refused whenever any program runs.
    - EN4 must land the fix before the runtime binds to H-08.
  - Experiment: XM4 (command-keyboard-model.md:927), with a long-running program job added.
- needs_owner: false. The fixes apply the owner's Q4 (a) and Q5 (a) answers and the DT-M1 filter; no owner option changes.

#### E4

- id: E4
- on: section 12 (I1 to I4, X-1 to X-8), Q8, Q9, Q18
- kind: objection
- claim: As written, the spikes cannot decide Q8 and Q9. X-1 holds an interpreter timing against E2, an end-to-end interval that a function timer may not stand in for, and X-6 is planned as a cloud spike whose speed comparison is not performance evidence. Give each spike a decision rule and a kill criterion on the protocol's card, and take the decisive timings on the owner's machine. Run X-5 with X-6 as inputs to Q8, and start X-3, X-7 and X-8 without waiting for Q8, since only X-1, X-2 and X-4 depend on the interpreter's host language.
- evidence:
  - E2 is the B4-12 M1 interval (charter:89), which ends at the first correct full frame (b4-12-measurement-decisions.md:36). The measurement contract says "do not … substitute renderer FPS or a short function timer" (10_V1_ARCHITECTURE.md:802). A turn needs p95 ≤ 100 ms over at least 100 formal observations on one frozen candidate (:798).
  - X-6 as recorded runs "in a cloud session" and compares speed "on the same headless machine" (engine-language-rust.md:42). The partition's own rule (line 434) and AGENTS.md "Evidence and publishing" do not count that as performance evidence. Its oracle equality is valid source evidence.
  - The protocol's experiment card has these fields: question, decision it feeds, kill criteria, evidence class (docs/progress/1.0/stage-2-experiment-protocol.md:17-28).
  - Proposed decision rules (a sketch; nothing has been measured):
    - X-1: a headless lead gives the cost per IR step and per kernel call for the three L3 tools. The decisive run comes in L3: E2 at p95 ≤ 100 ms over at least 100 observations on the owner's machine, with those tools' views active.
    - X-2 setup: compare the round trip of one kernel call in the same process against a separate process, at X-1's call mix.
    - X-2 faults: inject memory exhaustion, deep recursion, an endless loop and a killed runtime. A placement passes when the journal and state hash are unchanged, the shell stays responsive, and a commit completes within E2 during the fault.
    - X-2 startup: startup stays within E3's proposed 3 s (charter:90).
    - X-2 transfer is small: 1,039,200 bytes of labels and 708,480 bytes of pose ids per revision (177,120 × int32, render-contract.md:54). 0.4 already moves the label array over loopback (10_V1_ARCHITECTURE.md:172-173).
    - X-3: one reproducibility key gives identical result bytes over repeated runs, on Windows and on headless Linux. Planted clock, file and network reads are refused.
    - X-5: exact kernels at the recorded sizes: pose entries beyond 2^64 (partition line 109; ENG-0J:151), words of 672,417 letters (ENG-0J:84) and the 10^7 admission bound (partition line 332). A licence list goes to the owner.
  - The route options are complete:
    - I1 to I3 cover an own or borrowed front end over an own or borrowed executor. The one missing combination, a borrowed syntax over an own interpreter, is I1 with Q13. I4 is the host-language library.
    - In I1 to I3 the typed IR is the contract (line 122), so the executor below it can change later without changing what saved work means.
    - The front end is the hard choice to reverse.
- needs_owner: true. Q18 (whether spikes run in this stage) is the owner's, and these rules would be part of that permission.

#### E5

- id: E5
- on: Q7, X-7, 8.5, I-8 (user charts and the renderer); Q6 in passing
- kind: objection
- claim: The recommended Q7 option, evaluating charts once per revision, cannot serve the gate's workload. A chart in the projection chain gets new input for every vertex on every frame while the camera rotates in four dimensions or a turn animates. The unit is therefore the vertex (18,288,000 per frame), not the slot that X-7 counts, and per-revision CPU evaluation fits only per-object layouts. Define the chart subset now so that it compiles to portable HLSL (pure, total, float32, bounded loops, no recursion, allocation or big integers), keep the CPU interpreter as its reference evaluator, and put a generated projection stage, with its cost measured, into the renderer feature list before the renderer freeze.
- evidence:
  - Order: the 4D camera rotation comes first, then the d → d − 1 projection (render-contract.md:90-94). The gate's workload includes the most complex turn animation plus camera rotation (docs/wiki/decisions/renderer-candidates.md:40-41).
  - Counts (source and arithmetic):
    - assets/mesh.json gives 30,480 base vertices per cell, 600 cells and 6,096,000 triangles.
    - The probe draws them as one instanced draw of 30,480 vertices × 600 cells, projected in the vertex shader (s-b-probe-results.md:35; docs/wiki/evidence/w-j-probe-results.md:27).
    - That is 18,288,000 vertex evaluations per frame, about 549 million per second at 30 fps.
    - Each generator moves 4,596 or 4,600 of the 259,800 slots (offsets in assets/primitives.npz, counted read-only with the standard library).
    - At the mesh's average of 70.4 vertices per sticker, a turn re-evaluates about 324,000 vertices per frame even with a still camera.
  - The HLSL path is not tied to one candidate: the S-A2 native library compiles the S-B probe source, and the Qt app loads the same library (docs/wiki/evidence/level-2-framework-results.md:31). Shaders are written once in portable HLSL (charter:18).
  - A shader loop that never ends would trip the driver's timeout (Windows TDR), which R-17 treats as device loss (requirements-from-screening.md:34). That is why the subset must be total and its loops bounded.
  - Measured, bare W3 scene only (performance evidence, owner's RTX 4070 Laptop GPU, 10 October re-acceptance):
    - S-B: 788.16 fps, p99 1.493 ms, 81.7 MB peak VRAM (s-b-probe-results.md:54-55);
    - Godot: 670.69 fps, p99 1.907 ms;
    - Qt: 60.00 fps, p99 17.440 ms, capped at the display refresh, so its headroom is unknown (level-2-framework-results.md:37-38, :71).
  - H-06 and W5 are unmeasured and are never estimated (docs/progress/1.0/renderer-experiment-plan.md:102), so no cost is claimed for a user chart.
  - Q6's research views would add viewports outside W5's layout, and each needs its own gate run.
  - Timing: the owner freezes the renderer soon (docs/wiki/decisions/owner-decisions-2026-11-03.md:44-45), and its feature list freezes with the architecture (renderer-experiment-plan.md:169). "HLSL after the renderer freeze" (partition lines 507-509) would therefore reopen the freeze.
  - X-7 becomes decisive when it runs as follows:
    - PresentMon at W3 on the owner's machine, three cold runs per candidate;
    - a chart stage generated from the subset, compared with the built-in perspective stage;
    - a headless check that GPU and CPU evaluations agree within the declared display precision.
- needs_owner: true. Q7 is the owner's choice between (a) built-in parameterised charts only in 1.0, each measured in H-06, and (b) the HLSL-compilable subset, defined now and enabled when X-7 passes.

#### E6

- id: E6
- on: section 13 and Q10; section 14 (L1, L3, L6)
- kind: proposal
- claim: Section 14 puts the migration-format-freeze and architecture-freeze gates in L6, after the product is built, whereas the signed charter makes them the exit of stage 2.5, before building. Hold them at L1's exit instead, with a data core that covers every target of the path B import; section 13's list omits filters, preferences, checkpoints with their replay paths, and 0.4 key maps. Also fold the runtime's command-layer needs (E2, E3) into EN4's final H-08, add an end-to-end E2 measurement to L3's acceptance, and hand the display-plan primitives of 8.5 to the renderer line before its freeze.
- evidence:
  - Gates:
    - The 2.5 exit is "architecture freeze and migration format freeze gate rulings" (charter:149).
    - The freeze covers the command table, the layer interfaces and the renderer feature list (stage-2-experiment-protocol.md:180), and implementation starts after it (:115-117).
    - L6's acceptance holds both gates (partition line 480).
    - No gate is relaxed (owner-decisions-2026-11-03.md:72-73).
    - The design-system ADR follows the design line. The route ADR can follow the spikes, since it is not a layer interface.
  - Data core:
    - Section 13 names "recipes, macro sets, protected orbits and filters" (partition lines 455-456) but freezes "words and recipes, macro sets, protection sets, bindings, and the evidence envelope" (lines 457-458).
    - E3 imports preferences (line 175).
    - The migration package carries `session/checkpoint-paths.json`, with each event's recipe and pre and post hashes (docs/progress/1.0/migration-exporter-proposal.md:37), which the 1.0 engine replays (:39).
    - 0.4 key maps migrate "by the word each key typed, commands through E1's alias table" (command-keyboard-model.md:893).
    - "The importer may start on C3" (migration-exporter-proposal.md:3).
  - 0.4 semantics to keep:
    - recipes of word and star steps, 1 to 256 steps, at most 3,000,000 primitives (partition line 294);
    - the ceiling is a total that 100,000-letter chunks cannot raise (10_V1_ARCHITECTURE.md:359-363);
    - test: every recipe in the oracle fixtures gives the same replay hash after conversion.
  - Dependencies:
    - L1 waits on EN3, EN4 and Q8 (partition line 475).
    - At this head neither EN3 nor EN4 has delivered: `docs/progress/1.0/jumbling-interfaces.md` does not exist, and `docs/progress/1.0/command-table.json` is unchanged since its first version (commit d8ab87cc).
    - L3's acceptance has no latency item (line 477), and performance gates come only in L6 (line 480), so an E2 failure would appear only after everything is built.
  - Renderer: H-01, H-06 and H-09 are still open (stage-2-experiment-protocol.md:150, :160, :163), and the display plans of 8.5 (partition lines 240-245) are not yet a handoff row.
- needs_owner: true. The order of the two Astra gates is the owner's to set, and a new handoff row changes the cross-track record.

### Engineer advisor: plain-language explainers for the owner

The advisor returned these as B1 to B5; they are numbered P-E1 to P-E5 here so that they do not clash with the
partition's layer B components.

#### P-E1. One real state

The engine, the program's mathematical core, keeps the only real copy of your puzzle, as a notary keeps the original of a contract. Your programs and the screen get read-only copies stamped with a revision number, a counter that rises with every accepted change. A program may compute a "what-if" copy, called a derived state, to show or compare. That copy is never filed back as the original: to make it real, you play its moves through the normal path, where the engine checks them again. So two copies can never drift apart. Your decision (Q4): may one session keep side branches (alternative histories) you can view and compare, or do experiments live in separate sessions?

#### P-E2. Preview and commit

A preview is a dry run: the program's mathematical core works out exactly what a move or macro would do (every sticker it moves, its effect on protection) and changes nothing. A commit makes it real; it takes one deliberate key press, and a held key never repeats it. A program may prepare a preview but never commit one. The commit must also name the very preview shown on screen. Otherwise a program could swap in another preview in the split second between drawing and pressing, and you would commit something you never saw. Undo is always available, and a commit cancels background checks still running for the old state. Nothing here needs your decision.

#### P-E3. Running programs

Your program text is first translated into an IR (intermediate representation): a precise, simplified form, like a recipe rewritten as numbered steps with exact quantities. An interpreter carries out those steps one by one, checking each against the rules. Heavy work, such as moving all 259,800 stickers, goes to the fast mathematical core instead. An own interpreter is one we write ourselves: full control over exact numbers, safety rules and error messages, but more work, because the editing tools must be built too. An embedded runtime borrows a mature interpreter from another language: a faster start, but its habits, such as approximate numbers by default, leak in. Your decision (Q9), after measurements: control or a faster start.

#### P-E4. Containing programs

A sandbox is a fenced area: a program inside can read the puzzle and ask for changes, but cannot touch files or the clock unless you grant it, and never the network. A budget limits how much work a program may do; I propose counting it in steps rather than seconds, so the same program on the same input stops at the same place. Long work runs as a job, a stoppable background task, so the screen never freezes. Placement matters: programs inside the mathematical core's own process (a running program with private memory) reach the data most directly; in a separate process, their crash cannot take the core down. Your decision (Q9), after a deliberate crash test.

#### P-E5. Drawing and charts

The renderer draws pictures on the graphics card, a chip built for many small calculations at once. It only draws what the mathematical core has decided and never judges whether a move is legal, so a drawing bug can show a wrong picture but never corrupt your puzzle. A chart is a custom way of flattening the four-dimensional puzzle onto the screen. While the view turns, it must be recomputed for about 18 million points in every frame (one still picture of the motion), 30 times a second. That is graphics-card work, so user charts need a restricted form that translates into graphics-card code. Your decision (Q7): built-in charts only, or user charts in that form.

### Mathematician

The mathematician's round 1 finished after the engineer advisor's cards were in this record, so M1 to M4 also answer
E2, E3, E5 and E6.

Abbreviations used in the evidence (all paths are relative to the repository root):
- partition = `docs/architecture/1.0/language/ecosystem-partition.md`
- contract = `research/jumbling/state-contract.md`
- DT-J1 = `docs/progress/1.0/design-table/topics/jumble-solver-aids.md`
- tex = `research/Full_600cell_Puzzle_Theory.tex`
- trees = `research/jumbling/theory/restoration.md`
- stage N = the README under `research/jumbling/restoration/N-…/`
- charter = `docs/progress/1.0/charter-2.0-draft.md`
- v1arch = `docs/architecture/1.0/10_V1_ARCHITECTURE.md`
- repair = `docs/wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md`
- observation = `docs/progress/1.0/jumbling-observation.md`
- research-data = `docs/progress/1.0/jumbling-handoff/research-data.md`
- glossary = `docs/progress/1.0/glossary/glossary.md`
- topic = `docs/progress/1.0/design-table/topics/language-ecosystem.md`

**Answers to the engineer:**
- M1 answers E3, M2 answers E6, M3 answers E2 and M4 answers E5. Each is support with a condition.
- E1 and E4 raise nothing under the mathematical lens. I found no mathematical error in P-E1 to P-E5.
- In these cards, "bar M3" means the charter's bar and "mechanism M2" means the partition's mechanism (see M6).

#### M1

- id: M1
- on: 8.1 Claim axis, Q3, A7, default 6; answers E3
- kind: objection
- claim: The Claim axis mixes three independent facts: a statement's logical status, the set it is about, and how the job that produced it ended. So its values overlap: a search stopped by its budget is both "not found within budget" and "incomplete (cut off)", and stage 14's DIS-5 is both a "finite instance" and a proved negative for its family. Proposal:
  - Claim takes four values: proved, conditional (named premises), refuted or open.
  - Finite instances, declared search sets and admitted domains move to Scope.
  - Job outcomes stay in A7. A job that did not complete yields no negative claim; a witness it found still counts once checked.
  - The owner's five words become display labels computed from these fields.
- evidence:
  - The seven Claim values are at partition:185. The A7 outcomes are at :113, the composition rules at :193 and :195, and default 6 at :554.
  - These are scope, not status. DIS-5 covers "all 7,200 two-letter words" and is "a negative for that family only" (stage 14:328-333, :389). Stage 19 proves its bounds on D[k] under a reachability promise. `OUTSIDE_CERTIFIED_DOMAIN(k)` refutes membership in D[k], "not unreachable or unsolvable" (stage 19:143-152).
  - Negatives: a closed finite set "is a complete negative certificate; a search that stops on a limit is not" (stage 14:262-264). So a completed exhaustive search gives a proved negative, scoped to its declared set. A cut-off search reads "open" and keeps its search record (partition:155; DT-J1:944). The research's "stuck" covers only D19's case set (research-data:154-156).
  - Refutation is asymmetric:
    - One exactly checked counterexample refutes a universal statement, however it was found. Card 2 was "answered by a counterexample" (contract:185), so the rule at :195 ("sampling never upgrades") must not block such a refutation.
    - An existence statement is refuted only by a proof or a complete certificate.
    - "Refuted (a counterexample is held)" covers only the first case, and Basis "assumption" (:186) duplicates "conditional".
  - E3, support with a further reason: a scoped negative names the set it covered. Under a wall-clock budget that set depends on machine load, so only deterministic budgets give a reproducible scope (partition:231-233). A run stopped by time reads "open". A result with a pinned input is a claim about its input revision. It moves to another revision only through an agreement test (partition:108).
- needs_owner: true. Q3 is the owner's question, and the five levels are the owner's own words (topic:29-30); this proposal keeps them as display labels.

#### M2

- id: M2
- on: 8.1 composition rules and label mapping, I-5, I-7, Q10; answers E6
- kind: objection
- claim: The rule "a conclusion is never stronger than its weakest input" is undefined, because Basis and Check are only partially ordered, and it covers only one argument. The label mapping also loses facts: 0.4 state D is a permission, the research's "measured" is exact, and Scope lacks the menu order and the context of a check. Proposal:
  - Store evidence as an AND/OR provenance graph, with a stated partial order on (Basis, Check) pairs.
  - One argument is summarised by its set of weakest leaves. Independent arguments for one statement combine by the strongest.
  - A premise is discharged only by a claim with the identical statement and scope.
  - State D stays a permission, outside the axes.
  - Scope gains the menu-order digest, the contract revision and the context of a check (revision, protection set, goal, role).
- evidence:
  - No order is stated. The partition gives no order between, for example, "theorem with model review" and "certificate with independent checker". The label checkpoint rests on both: a theorem whose constructions had no external review, plus 46 certificates passed by an independent checker (contract:174, :191).
  - One claim can have two supports: W3 and W5 are certificates and are also replayed (stage 14:286-288). C1 already has AND/OR graphs (partition:151).
  - Discharge: "a lattice table without a reachability premise does not inherit the sticker theorem" (stage 19:166-168). An unreachable lattice configuration can have ν = 0 (`research/jumbling/restoration/README.md`:57-58; contract:175).
  - State D is a "valid preview/commit permission" (v1arch:309). Mapping it onto evidence (partition:206) breaks I-5 (:52-54) and bar M3 (charter:99). State C holds only for one state, orbit, role, protection set and goal (v1arch:308), and Scope (:188) cannot record that context.
  - "Measured" means "exact results on finite inputs" (research-data:13-14). Basis groups "sample or measurement" (:186), so mapping by name downgrades it. "Reported" means a program that is not committed (stage 15:12). That is a replayability fact and must stay visible.
  - Menu order: I-7 and the reproducibility key carry the menu-order digest (:60, :231), but Scope does not (:188). The reversed I_a record keeps identity fddc8532…, while 0 of 60 indices keep their rotation (DT-J1:954). So evidence on a (grip, index) word without the order digest can attach to a different word.
  - Check's "reused, fresh exact audit, actual replay" (:187) is not a strength order. Reuse through complete endpoint transport is exact (Lemma 14, tex:1431-1444, :1460-1464).
  - E6, support with a condition: the evidence envelope's identity fields freeze with the data core (partition:457-458). Moving that freeze to L1's exit therefore needs Q3 and these rules settled first, or it freezes a lossy mapping. Imported 0.4 evidence enters as recorded (v1arch:271, :317-322).
- needs_owner: false. These rules make 8.1 well defined under every Q3 option; the order of the freezes stays E6's owner question.

#### M3

- id: M3
- on: I-1, I-6, I-7, A1, B2, 4.1, 10.2 (row at partition:301); answers E2
- kind: proposal
- claim: Model identity should be a tower of exact maps with named kernels:
  - pose g_A;
  - visible pose class g_A·Stab_A;
  - labels (defined on the lattice only);
  - position.

  Every predicate, goal, protection check and derived model is typed by the level it reads, and a claim moves along a map only when the map preserves its subject (Lemma 13). Reachability then becomes a claim about a state in a named puzzle, using 8.1's values (witnessed, by theorem without a witness, promised, refuted, open), instead of I-1's three-valued status.
- evidence:
  - The maps have kernels. Generator 1 rotates centre piece 0 in place with equal labels (contract:148). "Fixed pointwise by every legal move" (tex:780-781) and "never move" (glossary:122) hold for slots and labels. They do not hold for poses, nor in the jumbling puzzle (contract:54).
  - Labels are defined only on the lattice and determine the pose up to Stab_A there (contract:26, :178).
  - The visible level:
    - No later twist separates poses in one class g_A·Stab_A (the label-lift lemma, contract:178-179). Its proof uses only posed regions and left multiplication, so it should extend off the lattice. I have not checked that in full.
    - Heights, branches, second vertices and domains are functions of γK⁺ (trees:20, :39-40). Sketch: γ[u, w] = [lu, rw], and the units u and w fix v₀. So these quantities live on the visible level.
  - Protection is the exception. It stays pose-based inside Jumble and is 600-cell-Full's after the crossing (contract:148, :185).
    - Counterexample: protect centre piece 0 in Jumble. Retained generator 1 fails the pose check there, but passes 600-cell-Full's label check after the crossing, on identical labels (also DT-J1:840).
    - So a protection verdict carries its level and never crosses over.
    - A pose-level refusal whose cause no sticker shows needs a mark when the design line resumes. The same filter put out unmarked pose equality (DT-J1:946).
  - Reachability:
    - A label checkpoint is reachable by theorem with no word: "No such word is produced, stored or searched for" (contract:174). None of the three values at partition:41 and :131 can say this.
    - One of the 36 corrections, applied from solved, gives a lattice configuration that is reachable in the jumbling puzzle with a witness. Yet no retained word reaches its labels (ν ≠ 0; contract:184, DT-J1:887). So reachability depends on the puzzle.
  - Goals are separate (partition:136, :557; stage 19:15-16). Typed as predicates, they are ordered: exact pose ⇒ sticker test ⇒ lattice (height 0) ⇒ low height. Also, retained checkpoint ⇒ label checkpoint (contract:176). A frontier exit meets none of them (stage 19:72).
  - Derived models:
    - Lemma 13 needs cost-preserving automorphisms of the actual state graph that keep the goal, keep protection and keep the data needed to lift a path (tex:1370-1374).
    - Its scope tests show permutations with equal target-orbit restrictions but different full actions, and permutations with disjoint visible supports that do not commute (tex:1528-1551).
    - Γ(Y) keeps admissibility and complete legal-word languages (stage 19:46-48), but changes which piece sits where. Only legality claims transfer.
  - E2, support with one refinement: binding the token to the preview ID and the word digest is sound only if the digest fixes the exact letters. For (grip, index) words, the digest must cover the menu-order digest (DT-J1:954).
- needs_owner: false. The tower implements A3, A5 and O8 (1) (DT-J1:1021); moving protection inside Jumble to the visible level would be a contract change for EN1's Astra check and then the owner.

#### M4

- id: M4
- on: A3, C1, I-4, default 10 (R-11), 10.2, 10.3; answers E5
- kind: objection
- claim: The kernel and base library lack three exact structures that restoration and the research make decisions with:
  - linear algebra over F2 and F5 (ν, the 36 corrections, O6 (b)'s check);
  - the quaternion algebra over Q(√5), with the icosian order, its 2-adic valuation and its tree (heights, branches, domains);
  - jumble words as partial maps, whose one normal form must keep each word's domain, which free reduction does not.

  I-4's list of float sites also misses the A1 input map, and E5's float32 charts are admissible only if every property a chart declares is decided exactly.
- evidence:
  - F2 and F5:
    - C1 has linear algebra over Z, Q and Q(√5) only (partition:151).
    - ν has 35 sign bits, 8 frame bits and two values mod 5 (contract:171).
    - The correction library has rank 34 over F2 and 2 over F5 (`research/jumbling/restoration/README.md`:56-57), with exponents solved over Z5 (:84-85).
    - O6 (b) checks a drafted set of corrections as "ν plus their fixed rows" (DT-J1:887, :1019). That is an addition in C2^34 × C5^2.
    - Lemma 8's sign and frame sums use the same algebra (tex:1006).
  - Trees: T1 to T3 decide heights and the lattice through the quaternion algebra over Q(√5), the icosian ring, the 2-adic valuation v₂ and the 5-regular Bruhat–Tits tree (trees:16-28). Domains are cosets γK⁺ (:39). A3 has only Q(√5) and matrices (partition:109).
  - Partial words:
    - Counterexample: where grip c has a straddler, (c, a)·(c, a⁻¹) is refused (contract:46-50), but its free reduction, the empty word, is admissible. Merging the two letters into the identity letter (c, 1) keeps the domain; deleting it widens the domain.
    - Inverses are safe: a twist maps both half-spaces of its cut onto themselves (contract:52), so w⁻¹ is admissible from the end state of w.
    - Conjugates are not: u·w·u⁻¹ need not be admissible where u and w are. So R-11's operations (`docs/progress/1.0/requirements-from-screening.md`:21) are total on well-formed words but partial on states.
    - Stage 16's local commuting rule "is not a normal form" (bca and cab; `research/jumbling/restoration/16-six-jams/PROOFS.md`:60-63). The lexicographically least ordering of a trace (its interchangeable orderings) is a normal form, but it needs a global rule.
    - Minimality and dynamic targets keep a representative of every shortest descent (stage 16:57-59). They are search reductions, not word identities, so a reduced search's negative reaches the full family only through that lemma.
  - Bounds: B11's 6,775 digits (partition:317) are a routine exact integer. What can never be expanded is a word of that length, and straight-line programs cover that (:151).
  - The A1 input map chooses the exact rotation that minimises the distance to a float target built from an axis and an angle (contract:126-130). Cosine and sine can round differently in different runtimes, so a near-tie can be decided differently on Windows and Linux (I-9, I-10).
    - Fix sketch: record the request as a dyadic Cayley vector ω, each float read exactly. R(ω) and every squared Frobenius distance then lie in Q(√5), and the choice is exact.
    - Settling experiment: run the map on requests near the bisector of two candidates under both runtimes.
  - E5, support with a condition: GPU–CPU agreement within display precision shows that a shader matches its reference, not that the chart is true.
    - Stereographic projection from a 600-cell vertex sends the 20 cells at that vertex to unbounded regions, in both evaluations.
    - So a chart's declared domain, injectivity and section membership are decided exactly on exact vertices, as C4 says (partition:154).
    - The precision check compares against an exact or interval evaluation on that declared domain.
- needs_owner: false. Everything here fits existing components and the Q16 cut; Q7 stays E5's owner question.

#### M5

- id: M5
- on: section 11 (S2, mechanism M2, the interim reading, "the fact every option faces"), I-11, Q2
- kind: objection
- claim: S2's base tier is well defined only as an enumerated list of shipped functions with recorded definitions, not as the class of functions "that do not search". Two reasons: a static fact combined with a theorem returns what a search returns, and S2's wording is narrower than DT-J1's rule 6, because it allows selections over caps and targets. What software can enforce is:
  - shipped content;
  - the execution gate;
  - capabilities;
  - a flow rule the policy may switch on: in the solving profile, a preview's word and target may depend on the state or on computed results (by data flow or by control flow) only through declared exceptions. These are setup search for a named target, the repair pair the solver picks, and whichever author classes the policy admits.
- evidence:
  - Wording: S2 forbids searching, minimising and ranking over twists or words (partition:379). Rule 6 also forbids selections ("largest, top, first") and targets the program picks (DT-J1:303, :940-942). The interim reading adds "quantifies" (partition:393).
  - Counterexample 1, a selection over caps: "the cap with the largest N_c" uses static facts and one choice over caps. Naming caps by decreasing N_c put a cap with a lowering pair first or second at 41 of 63 repair steps (repair:65-67). The 17 October scope forbids ranking caps (repair:57-59).
  - Counterexample 2, a static fact that equals a search:
    - When N_c equals the floor F_c = |In_c − Out_c|, no twist of cap c alone can lower N_c (observation:24, :118). A formula with no quantifier answers a question over twists.
    - A piece's domain (w, y) at height 2 (trees:41) is static under rule 6 (DT-J1:303). By T5 (trees:36), the domain fixes the twists at each own holder that lower that piece: the elements h with h⁻¹v₀ = y.
    - These form a coset of order 12 in I_a,c, because I_a,c (order 60) acts on the five neighbours of w with stabiliser A4_c (order 12). Listing them needs no search.
  - Counterexample 3, stored searches: certified setup tables, seeds and the 36 corrections are stored search results (charter:51; Lemma 15, tex:1473-1482; DT-J1:1020). The tier must say whether shipped tables count as "static facts".
  - Flow: mechanism M2 records "the inputs it read" (partition:364), which misses control flow.
    - Counterexample: a program searches, then builds its result letter by letter from one-letter words in the solver's own macro base. The preview is a composition of the solver's stored words, but the search chose it.
    - So flow labels must follow control flow as well. Such a rule is conservative and will refuse some harmless tools.
  - Limits:
    - The boundary concerns who chose a word, not what the word does, so it is a property of information flow, not of outputs.
    - No checker decides, for every program of a general language, whether its output is a solving step (Rice's theorem). Budgets make each run finite, but they do not make "searches" a property of the function a program computes (counterexample 2).
    - A retyped word and what the solver understood cannot be observed; partition:352-355 says this correctly.
    - The interim rule at partition:395 can be enforced inside the program by flow labels, but not against a person.
- needs_owner: true. The owner and the integrator set the boundary (I-11); the edge rulings and switching on the flow rule are policy.

#### M6

- id: M6
- on: I-12, D5, default 8, Q14; bars M1, M4 and M6 (charter:97-102); the relationship strip
- kind: proposal
- claim: "While understanding its principle" can be checked only if each tool's principle exists apart from its code:
  - an exact specification: the function, its domain, and what an empty or negative answer means;
  - the lemma or certificate it rests on, with its evidence;
  - one small instance on the real geometry that a person can check by hand;
  - its decomposition into base parts.

  The language should give these a form: contracts as typed claims checked at run time, a provenance view down to primitives and stage-qualified lemmas, a step-through of the independent checker, and forks that inherit no evidence. The relationship strip's statements should be the same typed relations, so that code, strip and theory book share one grammar, with a glossary entry for every term.
- evidence:
  - The principle is owner decision 6 of 3 November (`docs/wiki/decisions/owner-decisions-2026-11-03.md`:46-48). I-12 asks only for an explanation and traceability (partition:73-75), and D5 for an explanation page per tool (:167).
  - Empty answers need a stated meaning. "An empty list says that no pair at that cap lowers N" (repair:51-52) means no three-twist repair of the stated shape, not that no step exists (DT-J1:944).
  - The theory's scope tests use six and seven abstract stickers that a reader can check by hand (tex:1528-1551). A tool's instance should use the real geometry.
  - Forks: an import grants no frame or action certificate, and a prefix inherits no approval or cost (v1arch:271, :291). The checker's mutation gate feeds it tampered records (partition:112; stage 18:60-63).
  - The strip:
    - The owner's design decision and the charter require its grammar to be defined and proved (`docs/wiki/decisions/owner-decisions-2026-10-02-design.md`:43-44; charter:58, :100).
    - The partition never mentions the strip or the encoding grammar.
    - The strip's relations (destination, current occupant, wanted piece, buffers, frame, residual) are typed by level like everything else. Off the lattice a piece occupies no slot (contract:26), so "current occupant" is partial inside Jumble.
  - The glossary has 30 terms.
    - "Position" appears in 12 lines and is never defined.
    - There is no entry for pose, sticker, word, recipe, certificate, menu, the evidence levels or any jumbling term.
    - "Twist" means "one of the 1,200 generators or its inverse" (glossary:88-90), but (c, g) in the jumbling contract (contract:12). So B2's cost unit "twists", listed beside "menu letters" and "retained generators" (partition:120), has no single meaning.
    - D5's script check (:167, :556) has nothing to check the language's terms against until they are entered.
  - Label collisions:
    - A5 is the command layer (partition:93, :111) and also the state contract's amendment (:301).
    - E2 is Editions (:174) and also the turn-latency bar (:421).
    - M1 to M5 are mechanisms (:359-372) and also charter bars (:52, :167).
    - X3 (:444) could mean spike X-3 (:425) or the research's open problem X3 (research-data:181).
    - This round's card IDs collide again.

    The form of a fix is notation preference (8B); the disambiguation itself is not.
- needs_owner: true. Whether understanding is only explained and labelled, or also checked as part of the boundary (S3; Q2, Q14), is the owner's call.

### Mathematician: plain-language explainers for the owner

#### P-M1. Exact numbers for decisions

Computers usually store numbers approximately ("floating point"), with tiny rounding errors: harmless for drawing, risky for deciding. In the solved puzzle, pieces touch the cuts (the flat boundaries along which caps turn) at distance zero, so a rounding error could make a legal twist look blocked. The jumbling contract's tests include a piece crossing a cut by five ten-billionths of the distance from the centre to a cell, which must count as blocked. So every decision uses exact numbers: whole numbers, fractions and numbers a + b√5, which the geometry needs. A fast approximate test may answer first only where a proved limit on its error shows it cannot be wrong. Trade-off: none for you; the charter binds this.

#### P-M2. Poses, stickers and what you can see

A pose is how a piece sits in 4D; a slot is a fixed place where one sticker sits. Some pieces, like cell centres, can turn in place: the pose changes, every sticker stays in its slot, so nothing shows. A proof in the jumbling contract, so far reviewed only by AI, shows no later twist can tell such poses apart. So a solve ends on the sticker test (all stickers home), as you chose on 22 October. I propose the language checks whether each value describes poses or stickers. One catch: protection inside Jumble reads poses, so it can object to a turn no sticker shows. Trade-off, later: mark such objections, or let protection read stickers, a contract change.

#### P-M3. How sure is a result?

Your five words (proved, conditional, finite instance, not found within budget, incomplete) mix three questions. How true is it: proved, true if named assumptions hold (conditional), disproved, or open? What is it about: one finite case, everything a stated search covered, or every state the puzzle can reach? How did the calculation end: finished, cancelled, or stopped at its budget, a preset limit on work? For example, a search stopped by its budget is both "not found within budget" and "incomplete". I propose storing the three answers separately and computing your five words from them for display. Trade-off: one simple scale is easier to read but can mislabel results; separate answers are exact but need a careful display.

#### P-M4. Certificates and independent checkers

A certificate is a record that lets anyone confirm a result without trusting how it was found. To show a twist is blocked, it gives two exact points of one piece, one on each side of the cap's boundary. To show what a macro does, it gives every twist and a digest, a short fingerprint, of each state. A checker is a separate small program that recomputes every claim from the certificate, twist by twist, sharing no code with the program that found it. It is tested with damaged certificates, which it must reject. Reviews by an AI model or a person are recorded as other kinds of check. Trade-off: extra work and storage, against trusting the finder.

#### P-M5. What the boundary can and cannot enforce

A read-only program can search: it tries twists on copies and reports the best. No checker can tell, for every program, whether its output is a solving move; a classic theorem (Rice's) rules that out. Even a plain formula can answer a search: when a cap's mismatch count equals its floor, a simple formula, no twist of that cap alone can lower the count, and nothing was tried. Software can enforce what ships, one key press for every change, which abilities each mode (solving, writing tools, research) grants, and where each previewed move came from, indirect choices included. A word typed from elsewhere, or your understanding, stays invisible. Trade-off: stricter rules forbid harmless tools; looser rules rely on honest labels.

## Round 2

Round 2 ran for the engineer advisor only. The mathematician's round 1 began after the engineer advisor's cards were in
this record, so M1 to M4 already answer E3, E6, E2 and E5; here the engineer advisor answers M1 to M6. The cards cite
the first draft's IDs and line numbers, as in round 1.

### Engineer advisor

Abbreviations follow the mathematician's list (partition, contract, DT-J1, tex, research-data, glossary, v1arch). In addition, ENG-0J = `docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md` and nucoords = `research/jumbling/restoration/03-retained-group-and-lattice-restorer/scripts/nucoords.py`. "Card E3" means my round 1 card and "charter E2" means the bar, so the collision M6 points out does not happen here.

#### R2-E1

- id: R2-E1
- on: M1, M2 (and their conditions on cards E3 and E6)
- kind: alternative
- claim: I support M1's split of the Claim axis and M2's provenance graph, and I accept both conditions. On card E3: a run stopped by the clock reads "open", and its outcome stays in A7. A result with a pinned input reaches a newer revision only by a re-run or an exact transport, so my "never shown" was too strict. On card E6: no lossy envelope may freeze. Instead of settling Q3 and M2's order before the freeze, I propose that the data core store facts and derive judgements. It freezes the leaf facts, the AND and OR edges, the scope with the level of every digest, and each source's label word for word. M2's order, the summaries and the owner's five words stay versioned functions over those facts.
- evidence:
  - **Cost** (source and arithmetic; nothing measured):
    - A graph node holds enumerations and digests, never a state. One 600-cell-Full state is 1,039,200 bytes (card E2).
    - A summary is a subset of at most 5 × 6 × 3 = 90 (Basis, Check, check mode) values (partition:186-187).
    - Summaries are recomputed when a claim changes, never per frame, and cached by digest with fail-closed reads (partition:236).
    - Changing a function is an edition with a recorded note, so no saved meaning changes silently (partition:174).
    - A test runs through every combination of stored fields and checks that each gets exactly one display word or a typed inconsistency.
  - **Experiment (none yet):** build envelopes for the stage 14 to 19 corpus. Report their nodes and bytes against the records' own size: 13,387,282 bytes of JSON and JSONL files, gzip included (source, counted read-only).
  - **The import is lossless only if it maps by format, not by word** (source, counted read-only):
    - The restoration archive's uncompressed records carry 60 distinct `format` values.
    - 146 of the 411 JSON files in stages 14 to 19 have no `format` key, 61 of 103 in stage 16 (as partition:330 says).
    - Status strings differ by stage: "STUCK" and "BUDGET" in `research/jumbling/restoration/13-tutorial-runner/runs/hd2p2/campaign.json`, "stuck" and "budget" in stages 11 to 15.
    - The same word can mean different things. "exhausted" next to "best" closes a search over a declared set (`research/jumbling/restoration/14-blocking-structure/results/s7304-retained-deep.json:13-18`). "stuck" covers only D19's case set (research-data:154-156).
    - Rule: a versioned table keyed by (format, field, value). A value the table does not map is imported as "recorded", with its text kept word for word.
    - Test: every (format, field, value) in the corpus is either mapped or listed as unmapped, and each envelope cites its source bytes by digest (partition:340).
  - **The 0.4 import:**
    - 0.4 persists only states A and B, and B is untrusted on load. C is never persisted, and D lives in memory only (v1arch:306-309).
    - So A and B come in as recorded claims until the 1.0 engine re-verifies them (v1arch:317-322).
    - D is the command layer's preview token (partition A5), outside the axes, as M2 says.
    - C keeps the v1arch:308 rule that it is never persisted. M2's check context therefore serves only in-memory claims and the display. The envelope persists only claims whose scope is an immutable input digest.
  - **Scope names the level of its digest** (see R2-E2). 600-cell-Full's `state_hash` covers int32 labels (`docs/progress/1.0/oracle/README.md:13-14`). J1's digest covers poses (`research/jumbling/sim/README.md:42`).
  - **Discharging a premise** by an identical statement (M2) needs a statement ID. That is the stage-qualified, commit-pinned name (partition:198) or the digest of a typed statement, and only equal IDs discharge. An equivalent statement under another ID is refused, which is the safe direction; a different statement is never accepted.
  - **Card E3 as changed:** a pinned-input result is re-run at the new revision, or transported where a lemma makes the transport exact (Lemma 14 for conjugates, tex:1428-1444).
- needs_owner: true. Q3 and the order of the two freezes (card E6) are the owner's; this card changes what Q3 must settle before the freeze.

#### R2-E2

- id: R2-E2
- on: M3, M4 (and their conditions on cards E2 and E5)
- kind: support
- claim: I support M3's tower and M4's three structures, placed by the partition's own rule (partition:145-147):
  - Maps and per-revision batches that decide something are kernel calls typed by puzzle and level, each with an oracle agreement test. These are the projections, protection, the end tests, ν and heights.
  - The small exact algebra is library code over A3: rows over F2 and F5, quaternions over Q(√5), the icosian order and its tree, and trace normal forms.

  I accept both conditions, and both change my wording. Card E2's token binds the digest of the literal letters together with the model identity, menu identity, menu-order digest and contract revision, never a normal form. Card E5's GPU–CPU check is only a shader test; C4 decides a chart's declared properties exactly.
- evidence:
  - **The levels differ by puzzle** (source):
    - 600-cell-Full holds labels only; positions are derived from them (core.py:255-266). The oracle hashes int32 labels (`docs/progress/1.0/oracle/README.md:13-14`).
    - J1 holds pose ids into a canonical pose table (`research/jumbling/sim/README.md:42`). Labels exist only on the lattice (contract:26, :146-147). The J1 harness compares pose digests (ENG-0J:31).
  - **Kernel API sketch:**
    - Snapshots, derived states, digests and predicates name their puzzle and level.
    - A projection is an explicit kernel call with a typed partial result ("off the lattice: k pieces"), never a sentinel value. ν is already specified this way: "undefined" off the lattice (ENG-0J:135).
    - There is no lift from a lower level to a higher one.
    - A derived state carries a reachability claim in a named puzzle (an envelope reference, see R2-E1) instead of I-1's three values.
  - **Agreement tests:**
    - Existing: poses by the J1 harness; labels by the 600-cell-Full oracle and J1's acceptance against rotperms, move_src and move_dst (contract:183); the crossing by ENG-0J item 7 (:49-55).
    - New: M3's centre-0 case as a regression test. It must be refused at pose level inside Jumble and pass at label level after the crossing (contract:148, :185).
    - New: the label-lift lemma as a property test on J1 fixtures. That is a lead, not a proof.
    - New: heights and domain ids. They are per-revision engine data (ENG-0J:132), so they become a kernel batch per distinct pose. It must equal both the library and `trees.py` on ENG-0J item 8's states (:57-58), and on the H6030 witness (left height 6,030; fixture, ENG-0J:31).
  - **Library placement:**
    - ν's correction combination is a row reduction over F2 and F5, on 45 coordinates and 36 corrections (contract:171; nucoords:170-196; source). That is library code, but R2-E3 covers its boundary problem.
    - B11's 6,775 digits are about 22,500 bits (arithmetic), an ordinary big integer.
  - **Words:**
    - M4's merge keeps the domain and the end state. For exact letters, admissibility of (c, g) does not depend on g (contract:30-50), and I_a has the identity at index 0 (ENG-0J:127).
    - The merge still deletes the state between the two letters, and Strict protection (partition:296) and peak height (partition:318) are evaluated on exactly those states.
    - Stage 16's R1 and J1's checkpoint derivation drop identity products (`research/jumbling/restoration/16-six-jams/PROOFS.md:55-56`; `research/jumbling/sim/README.md:62-64`). That is valid only on admitted words, so these rules stay search reductions inside C5.
    - So a normal form is only an equality key. Preview, protection, commit and the token all use the literal letters. 0.4's `normalize` already keeps literal moves (core.py:165-181).
    - Word operations stay total on well-formed words. Admissibility is a verdict per revision, from the whole-word replay (ENG-0J:138).
    - R-11's property test (`docs/progress/1.0/requirements-from-screening.md:21`) gains a jumbling case: the key's word is admissible at X exactly when the literal word is, with the same end digest.
  - **The card E2 condition:**
    - ia16 codes are 60·grip + item (research-data:193).
    - The reversed I_a record keeps identity fddc8532… while 0 of 60 indices stay in place (DT-J1:954). So the digest must cover the order digest.
    - The commit already requires an unchanged word digest (ENG-0J:138).
  - **The A1 input map (new evidence):**
    - J1 builds the float target with `math.cos`, `math.sin` and NumPy norms before exact fractions take over (`research/jumbling/sim/twists.py:118-129`, :168).
    - I ran an in-memory check in this local Windows 11 session (CPython 3.15.1, MSC build) on 13,600 requests: every 0.1° plus 10,000 random angles. At the half-angle `twists.py` computes, cos was not correctly rounded for 484 requests and sin for 436.
    - Examples: cos at 32.9° is 0.506 ulp off, and sin at 26.8° is 0.512 ulp off. So the target bits are this maths library's, and another library can return the neighbouring value.
    - Evidence class: synthetic, Windows CPython. Not DirectX or performance evidence.
    - M4's dyadic ω takes cos and sin out of the decision. The same fix should cover menu-mode drags (ENG-0J:130), whose 60 candidates can be compared exactly in Q(√5).
    - Settling experiment, in two steps. First run the same 13,600 requests on Linux CPython and the X-6 engine candidate, and compare the target bits. Then place requests within one ulp of exactly computed bisectors and compare the chosen rotations.
    - Replay already uses the recorded exact rotation (contract:130).
    - The same maths-library dependence reaches the display plans that headless tests compare (partition:244). So X-3 should compare those plans across platforms at their declared precision.
  - **The card E5 condition:**
    - The float camera rotation comes before the chart (`research/jumbling/render-contract.md:90-92`).
    - So C4 decides a chart's declared properties exactly only for the chart function, on exact vertices, at exact camera poses (the identity or a K⁺ element).
    - Between those poses, whether a point is in the chart's domain is a display decision that decides nothing (I-4). Picking uses instance IDs, not chart inverses (partition:555).
    - The shader test compares against exact or interval values and excludes a stated neighbourhood of singular sets.
    - PresentMon in X-7 stays the performance evidence.
- needs_owner: false. The placement and the tests implement contract A1, A3 and A5 as written. The A1 change is a contract amendment for EN1's Astra check, and Q7 stays card E5's owner question.

#### R2-E3

- id: R2-E3
- on: M5; M4 (F2 and F5 in C1); Q2, Q9
- kind: support
- claim: I support M5's enumerated base tier and its flow rule, with one consequence for the route. Without modifying a borrowed VM, labels that follow both values and branches can be built only where the project owns the checker and the executor's values: in I1, or in I3's own front end. That is why the owner should decide whether this mechanism exists before L2 builds the checker and the IR. A fallback that works in every route is capability separation in the solving profile: a program either reads state-derived data or prepares previews, never both, apart from the shipped exceptions.
- evidence:
  - **New counterexample, from M4's request:**
    - Generic elimination over F2 and F5, applied to the ν on screen (O6 (b)) and the shipped correction rows (O7 (a)), returns the correction combination that DT-J1's filter put out (DT-J1:943, :1019-1020).
    - The research already has this function: `solve(b2, b5)` returns "library id -> multiplicity mod 10" (nucoords:170-196).
    - So no class of "non-searching" functions can define the tier. Only an enumerated list, plus a flow rule or a capability rule, stops this composition.
  - **The routes** (source, partition):
    - B3 infers effects through calls, and B4 owns the interpreter (:121-122). So I1 can label every value and every branch: statically in the checker (no run-time cost, more refusals), or also dynamically (fewer refusals, a cost per step).
    - I3 does the static part in its own front end and compiles dynamic labels as explicit fields.
    - In I2, "types, effects and evidence become conventions or need a separate checker" (:409). I4 has "no static effect discipline for user code" (:411).
    - Neither I2 nor I4 sees a native branch on a plain number, so each needs a modified VM or the coarse rule.
  - **Timing:** the IR is the stable contract (:122), and a saved meaning never changes silently (:174). Flow checks added after L2 would refuse saved tools that used to run.
  - **Persistence:**
    - Without stored labels, a tool that reads the state can write a computed word into the workspace, and a tool that previews can read it back.
    - "Research-profile output never becomes a preview in a solving session" (:395) holds across sessions only if saved words keep their label.
    - So stored words, macros and workspace records need a provenance field in the data core before the migration format freeze (:441-445, :457-458; card E6). 0.4 macros come in labelled as the solver's own, imported.
  - **Declassifiers** are fixed kernel or library entries that the policy enables: setup search for a named target, the repair-pair pick, the solver's own pick (:366), and whatever author classes the policy admits. No program can declare one.
  - **Limits beyond M5's:**
    - Labels cover data and branches, not whether a run ends or emits a preview, so one bit per run can still leak.
    - M5's rule checks only the preview. A tool that shows a state-derived word for the solver to retype passes it. DT-J1's filter also covers what is shown (:943), so display sinks are a policy question too.
  - **Experiments (none yet):**
    - X-1: run the three L3 tools with labels off and on, and report the cost per step.
    - X-2: unchanged, except that a preview request carries its label set to the command layer.
    - X-3: must-refuse cases (caps chosen by largest N_c, M5's search-then-rebuild, the ν solve above, laundering a word through the workspace), and a count of false refusals over the C6 tools and the solving-workflow tools.
- needs_owner: true. Whether the mechanism exists narrows Q9's routes and is one of Q2's mechanisms; switching it on is policy (I-11).

#### R2-E4

- id: R2-E4
- on: M6 (with M2 and B2)
- kind: support
- claim: M6's principle record is cheap to check if it is data in the module manifest:
  - its hand-checkable instance runs as a headless fixture;
  - its decomposition is generated from the IR's call graph and kernel calls, not written by hand;
  - its lemma or certificate resolves to an evidence envelope (R2-E1);
  - the A6 checkers stay kernel code and emit a per-claim trace that the debugger steps through.

  The label collisions and the two meanings of "twist" must be fixed before the data core freezes, because these names get stored in error codes, budgets and evidence citations.
- evidence:
  - **Checker trace:**
    - A6 never imports the producer (partition:96-97, :112). A trace is the checker's own output, so stepping through it adds no import.
    - The stage 13 checker already binds its report to its replay (`research/jumbling/restoration/13-tutorial-runner/scripts/check_run.py:1073`).
    - `macro-certificate/1` holds one digest per letter. Fixture: `research/jumbling/restoration/15-lowering-machine/results/certificates/R01.json`, 3 letters and 3 digests. So a trace grows with its certificate.
    - Provenance down to primitives stays lazy: a 672,417-letter step (ENG-0J:84) is cited by word digest and positions and expanded only on demand.
  - **Run-time contracts:**
    - Pointwise contracts run in the interpreter and are charged to the deterministic step budget (card E3).
    - A contract over a whole state (259,800 labels or 177,120 pieces) runs once per revision or as a CI agreement test (partition:145-147, :546-547), never once per call inside a loop.
  - **Forks:** modules are addressed by content digest (partition:123), so a fork inherits no evidence. 0.4 already works this way ("original approval and cost are not inherited", v1arch:291).
  - **Relationship strip:**
    - Its data contract can join EN3's view-model fields now. Its look waits for the design line (owner decision 4 of 3 November 2026).
    - Each relation carries its level (M3), and "none: off the lattice" is a value distinct from "unknown" (I-5; contract:26).
    - Relations are computed per revision, never per frame (R-19, `docs/progress/1.0/requirements-from-screening.md:36`).
  - **Names:**
    - Budgets and cost units are part of the reproducibility key (partition:231-233), so B2's unit "twists" (partition:120) is written into every run record.
    - That unit needs one meaning before L2 (glossary:88-90 against contract:12). "Seconds" stays a display unit and is never a budget unit (card E3).
    - Stored identifiers get a source prefix: charter bar, partition component, contract amendment, spike or research item. A lint fails on any bare, ambiguous identifier in formats and architecture pages (M6's collisions).
- needs_owner: false. The form of the names is notation (8B), and whether understanding gates anything stays M6's owner question; this card only places the parts.

### Engineer advisor: corrections to P-M1 to P-M5

1. P-M2: Add: 600-cell-Full records only stickers. Poses exist only for the jumbling puzzle: Jumble off stores the exit poses, and Jumble on replays later moves from them (contract A5). So 600-cell-Full itself cannot see a turn in place.
2. P-M3: The calculation can also end with an error. Like a cancellation or a budget stop, an error never yields a negative answer; the job outcomes are completed, cancelled, cut off and error (partition, A7).
3. P-M4: "Sharing no code with the program that found it" is too strong: the checker shares no search or construction code with the finder, but both use the same exact simulator and model files, tested separately (`research/jumbling/restoration/13-tutorial-runner/scripts/check_run.py:38-43`).
4. P-M5: "One key press for every change" should read "one deliberate action (a key press or a click) for every commit" (charter S4). A held key never repeats a commit; program views and the camera need no such action.
5. P-M5: "Where each previewed move came from, indirect choices included" needs a language that tracks data and branches: an own interpreter can, a borrowed runtime only if modified (partition, I2). A retyped or file-carried word loses the record.

## Outcome

Settled by the main session on 3 November 2026, in the order of the [round protocol](../README.md): the filter, then
demonstrated failures, then the 8B order among admissible options, then experiments for unmeasured objections. Nothing
here is an owner decision until the owner answers the questions at the end.

- The cards cite line numbers of the first draft of the partition, whose sha256 is
  `6b8d920024bed94997b7f5048e5f149e4a71a14dcd4f9b7aa1913970e460a9a9`. The revised partition renames its IDs, and its
  section 18 maps the old IDs to the new ones.
- Round 2 ran for the engineer advisor only, as its note says. It brought new evidence: the rounding of the cosine and
  sine behind the state contract's A1 input map on Windows CPython, a composition of shipped functions that returns a
  correction combination, and the format counts of the restoration archive. A third round would therefore be allowed.
  The main session settles here instead, because the new evidence calls for the experiments below and one new owner
  question (LQ21), not for another exchange of positions. The mathematician has not answered R2-E1 to R2-E4 (see the
  open obligations).

### Out by the filter

- A base tier defined as the functions "that do not search" (first-draft S2; M5). It admits a selection over caps and a
  target the program picks, which DT-J1 rule 6 and the 17 October scope forbid. It is replaced by an enumerated list of
  shipped functions under rule 6's wording, each with a recorded definition and a boundary ruling (partition BS2 and the
  interim reading). R2-E3 adds a composition that no class of functions can exclude (fix-first item 17).
- Provenance that records only the inputs a preview read (first-draft mechanism M2; M5). A search can choose a word that
  it assembles from the solver's own macros, and the inputs read would not show that choice. It is replaced by mechanism
  BM2, which tells whether a preview may depend on the state, indirect choices included. Its form is owner question
  LQ21 (R2-E3): flow labels that follow data flow and control flow, or capability separation in the solving profile,
  which is coarser.
- Charts evaluated once per revision as the recommended user-chart option (first-draft Q7; E5). A projection chart gets
  new input for every vertex in every frame while the camera rotates or a turn animates, so an evaluation once per
  revision cannot serve the renderer gate's workload. It is replaced by the recommendation of partition LQ7.
- The 0.4 macro-certificate state D mapped onto evidence (first-draft 8.1; M2). State D is a permission, and the mapping
  breaks charter bar M3. State D stays outside the evidence fields.

### Fix first

These are demonstrated failures that affect the work or correctness: items 1 to 14 in the first draft, items 15 to 19
in fixes proposed in rounds 1 and 2. They need no owner answer; item 16 adds owner question LQ21.

1. **Preview swap** (E2, with M3's refinement). A program could replace the pending preview at the same revision just
   before the commit, and every recheck would pass. The commit's user-action token now binds the ID of the preview on
   screen and the digest of its literal letters, with the model identity, menu identity, menu-order digest and contract
   revision (R2-E2), and the engine adopts only its own preview object. A headless test swaps the preview and the commit
   must be refused (partition INV-1, INV-3, KN5, 8.3, LP2).
2. **Undo refused while a job runs** (E3). `session.undo` is still gated `not_busy` at this head, which is DT-M1
   fix-first item 3. With programs running as jobs, undo would be refused whenever a program runs. EN4 lands the fix
   before the runtime binds to H-08 (KN7, LP1).
3. **Views that write the workspace** (E3). Under DT-M1 Q4 (a), a view that writes the workspace on every revision makes
   every undo stale. Views get no workspace-write effect, and a workspace change happens only inside a command the
   solver invokes, as one undoable action (LG3, default 16).
4. **Time budgets** (E3, M1). A wall-clock budget gives one reproducibility key different results under different load,
   so a scoped negative has no reproducible scope. Budgets are deterministic, and wall-clock time is only a safety stop
   whose run reads open (INV-10, KN7, 8.4).
5. **Availability of a language-defined command** (E3). Computing it would have made the engine run language code. It
   now combines the checker's static verdict with the engine's availability of the kernel rows the command may invoke
   (KN5, default 17).
6. **Freeze gates after the build** (E6). The first draft held the architecture-freeze and migration-format-freeze gate
   rulings in its last phase, while the charter makes them the stage 2.5 exit, before implementation. They now sit at
   LP1's exit. Their order against the design-system ADR is an owner question (LQ19).
7. **The data core** (E6). It missed path B import targets. Filters, preferences, checkpoints with their replay paths and
   the 0.4 key maps are added (section 13).
8. **Jumbling word normal form** (M4). Free reduction can widen a word's domain: where grip c has a straddler,
   (c, a)·(c, a⁻¹) is refused, but its free reduction, the empty word, is admissible. The normal form merges letters and
   never deletes them (LG2, LB1, default 10).
9. **Missing exact structures** (M4). Linear algebra over F2 and F5, and the quaternion algebra over Q(√5) with the
   icosian order, its 2-adic valuation and its tree, are added. R2-E2 places them as library code over the kernel's
   exact arithmetic, while the per-revision batches that decide something (projections, protection, the end tests, ν,
   heights) are kernel calls with oracle agreement tests (KN2, KN3, LB1, LB3).
10. **Protection across levels** (M3). A protection verdict carries its level, pose-based inside Jumble and label-based
    in 600-cell-Full, and never crosses over (LB3).
11. **Evidence rules** (M1, M2). The overlapping claim values, the undefined "weakest input" rule and the lossy label
    mapping are replaced by the fields and rules of partition 8.1. Scope gains the menu-order digest, the contract
    revision and the context of a check; "measured" maps to an exact computation on finite inputs; "reported" stays
    visible.
12. **Direct journal reads** (E1). The runtime reads the store only through the command layer's query rows (INV-2,
    section 9).
13. **An unlisted float site** (M4, R2-E2). The state contract's A1 input map is listed as a float site. On Windows
    CPython the cosine and sine behind it are not correctly rounded for 484 and 436 of 13,600 requests (synthetic
    evidence), so another maths library can return the neighbouring value. Spike LX-9 settles in two steps whether the
    chosen rotation changes, menu-mode drags included. If it does, the fix amends the state contract, which needs EN1's
    Astra check and then the owner.
14. **ID collisions** (M6). Every ID on the partition now carries its own prefix (section 18). The form of the IDs is
    notation preference; the disambiguation is not.
15. **A normal form used for execution** (R2-E2). Item 8's merge keeps a word's domain and end state, but it deletes the
    states between the merged letters, and Strict protection and the peak height are evaluated on exactly those states.
    A normal form is now only an equality key, for both puzzles: preview, protection, commit and the token use the
    literal letters, as 0.4's `normalize` already does. R-11's property test gains a jumbling case (INV-3, 4.1,
    default 10).
16. **Flow labels that silently constrained routes** (R2-E3). The revision after round 1 put M5's flow labels into
    mechanism BM2 under every option, and the integrator's leaning relied on them. Labels that follow values and
    branches at run time need the project to own the executor's values: route RT1, RT3's compiled label fields, or a
    modified borrowed VM. Static labels need a checker over exactly the source that runs, which RT1 and RT3 have and
    RT2 or RT4 can add over an approved subset of the host language (narrowed by the plan check of 3 November). A fixed
    mechanism would have bound RT2 and RT4 to a modified VM or a separate checker without an owner decision. BM2 now has
    two forms, flow labels or capability separation in the solving profile, and the owner chooses inside LP1, before
    LP2 (LQ21).
17. **A composition that returns a correction combination** (R2-E3). Generic elimination over F2 and F5, which item 9
    adds, applied to the ν on screen and the shipped correction rows, returns the correction combination that DT-J1's
    filter put out; the research's `solve(b2, b5)` already does this. The partition records it among the facts every
    boundary option faces, keeps the solve in the research tier and adds it to LX-3's must-refuse cases (section 11,
    LB3, LX-3).
18. **Labels lost through storage** (R2-E3). A tool that reads the state could write a computed word into the workspace,
    and a tool that previews could read it back, so "research-profile output never becomes a preview in a solving
    session" would not hold across sessions. Stored words, macros and workspace records get a provenance field in the
    data core before the migration format freeze, and 0.4 macros enter labelled as the solver's own, imported (BM2,
    section 13).
19. **A preview's own outcome** (main session, on R2-E3's fallback). A preview's outcome (its end state, N, its
    protection verdict) is state-derived, so a program that prepares previews and reads their outcomes could search by
    preview. Under capability separation a previewing program does not read its previews' outcomes; under flow labels
    the outcomes carry the state label (BM2).

### Adopted

Mechanisms that hold under every owner option, adopted in the revised partition:
- the runtime stays off the turn's critical path (E1);
- derived states live in the kernel's value store and are shown only through display plans that name their derivation
  (E2);
- the identity tower, goals as predicates typed by level, and reachability claims (M3);
- the facts every boundary option faces, and the base tier as an enumerated list (M5);
- a principle record for every built-in tool, glossary entries before use, one grammar for code, the relationship strip
  and the theory book, and forks that inherit no evidence (M6); the strip's design waits for the design line;
- a decision rule, kill criteria and an evidence class for every spike, with decisive timings on the owner's machine
  (E4);
- a chart's declared properties decided exactly, and a precision check against an exact or interval evaluation (E5,
  M4);
- the import of research records by a versioned table keyed by (format, field, value), an unmapped value imported as
  recorded, word for word, and a test that every (format, field, value) of the corpus is mapped or listed (R2-E1);
- premises discharged only by an identical statement ID: a stage-qualified, commit-pinned name or the digest of a typed
  statement (R2-E1);
- a result with a pinned input reaches a newer revision only by a re-run or by an exact transport that a lemma
  justifies, which replaces E3's "never shown" (R2-E1);
- the 0.4 import of the macro-certificate states: A and B as recorded claims until the 1.0 engine re-verifies them, C
  never persisted, D as the preview token outside the evidence fields; the stored envelope keeps only claims whose scope
  is an immutable input digest, and a scope names the level its digest covers (R2-E1);
- kernel calls typed by puzzle and level for what decides, with typed partial results and no lift from a lower level to
  a higher one; the small exact algebra as library code over exact arithmetic (R2-E2);
- the token's binding to the digest of the literal letters with the model identity, menu identity, menu-order digest and
  contract revision (R2-E2);
- new tests: the centre-0 regression case, the label-lift lemma as a property test (a lead, not a proof), and heights
  and domain IDs against `trees.py` on ENG-0J item 8's states and the H6030 witness (R2-E2);
- chart properties decided exactly only at exact camera poses, domain membership between poses as a display decision,
  picking by instance IDs, and a shader test that excludes a neighbourhood of singular sets (R2-E2);
- LX-9 in two steps with menu-mode drags, and LX-3 comparing display plans across platforms (R2-E2);
- declassifiers as fixed kernel or library entries that no program can declare; the one-bit leak per run and display
  sinks as recorded limits and policy questions; LX-1 with labels off and on (static and run-time labels apart since
  the plan check), LX-2's label set, and LX-3's must-refuse cases with a false-refusal count (R2-E3);
- the principle record as data in the module manifest, with a headless fixture, a generated decomposition, an envelope
  reference and a per-claim checker trace; pointwise contracts charged to the step budget and whole-state contracts run
  once per revision or in CI; one meaning of "twist" and source-prefixed stored identifiers, with a lint, before the
  data core freezes; seconds as a display unit only; the strip's relations with their level, per revision, with "none:
  off the lattice" apart from "unknown" (R2-E4).

### Open obligations for the full text

- The label-lift lemma off the lattice (M3). Its proof should extend off the lattice, but the mathematician has not
  checked that in full. The full text does not rely on the visible pose class off the lattice until that check is done.
- The mathematician's answer to R2-E1 to R2-E4. The full text does not rely on what the revised partition adopted from
  those cards until the mathematician has checked it.

### Experiments

- The spikes LX-1 to LX-9 with the decision rules of partition section 12, if the owner allows spikes in this stage
  (LQ18).
- DT-M1's XM4, with a long-running program job added (E3).
- R2-E1's envelope experiment, under the same condition: envelopes for the stage 14 to 19 corpus (13,387,282 bytes of
  records), with their nodes and bytes against the records' own size. It informs LQ10.

### Questions for the owner

Each question is in partition section 15, with a recommendation and the alternatives.

| Question | From | What the owner decides |
| --- | --- | --- |
| LQ2 | M5, M6, R2-E3 | Which boundary option set to start the joint setting from, the edge rulings, whether the flow rule is on, and which display sinks it checks |
| LQ3 | M1, M2 | The evidence model: separate fields with the five words as display labels, or one claim axis |
| LQ4 | E3, R2-E1 | Experiment branches, and whether a research job pinned by digest finishes after a commit; its result reaches a newer revision only by a re-run or an exact transport. E3 marked its fixes as needing no owner answer; the main session sends this one to the owner because it extends DT-M1 Q5 (a)'s exception for file jobs. |
| LQ6 | E5 | Whether research views may open outside Global and Local in the research profile |
| LQ7 | E5 | User charts: built-in parameterised charts only, or a subset that compiles to portable HLSL |
| LQ9 | E1, R2-E3 | The route, and where the runtime runs; the shell process only if it passes LX-2's fault run; decided together with LQ21 |
| LQ10 | E6, M2, R2-E1 | Which data the migration format freeze fixes, the provenance field included; whether the evidence envelope stores facts and freezes before LQ3 is answered (R2-E1), or stores judgements and freezes after it (M2) |
| LQ14 | M6 | Whether understanding is explained and labelled only, or also checked |
| LQ18 | E4 | Whether the spikes run in this stage, under the stated decision rules |
| LQ19 | E6 | Where the two freeze gates sit, and whether the charter is amended so that LP2 to LP5 may start before the design-system ADR |
| LQ20 | E6 | A handoff row that gives the renderer line the display-plan primitives before its freeze |
| LQ21 | R2-E3 | The form of flow control in the solving profile: flow labels (static labels need a checker over the source that runs, which RT2 and RT4 add over an approved subset; run-time labels need RT1, RT3 or a modified VM) or capability separation (every route, coarser); decided inside LP1, before LP2 |

### Owner's answers

- 3 November 2026: the owner confirmed the direction of the partition and asked for the full text. This answers the
  kickoff's first step only. It does not answer LQ1 to LQ21: each stays open with its recommendation and alternatives,
  and the full text keeps them open.
- The answer sheet on the owner's private page held no saved answers when the full text began.

### Records

- The partition is revised as listed above.
- The Astra plan check of 3 November found two major and two minor issues, and the partition is revised for all four:
  an acyclic order of decisions and spikes, with LQ8, LQ9 and LQ21 decided inside LP1 after their spikes; static flow
  labels over an unchanged runtime with a separate checker, which narrows item 16 and the engineer advisor's correction
  5; LQ19 as a charter amendment; and the existing 600-cell-Full oracle in `tools/oracle/`.
- The engineer advisor's corrections to P-M1 to P-M5 are applied when the explainers are given to the owner.
- The handoff tables in `docs/progress/1.0/stage-2-experiment-protocol.md` and
  `docs/progress/1.0/renderer-experiment-plan.md` change only if the owner accepts LQ20.

## Round 3

The owner confirmed the partition's direction on 3 November 2026. Round 3 gives the full text what the partition left to it:
the label table of default 20, the error registry of default 5, the figures for the interface contracts and the bars for
the spikes (engineer advisor), and the answers to the two open obligations above (mathematician). Each record is appended
unchanged, with its headings lowered two levels. The table the engineer advisor names under
`work/reviews/language-round3/` entered the repository at
[`docs/architecture/1.0/language/evidence-import/`](../../../../architecture/1.0/language/evidence-import/README.md).

### Engineer advisor

Source state: Git HEAD `9e060af083f2562f4cb6d604d392eb08445416d2`. Nothing here is a performance result: I took no timing, and every bar and rule in item 4 is an unmeasured proposal. Figures use the evidence classes of overview section 3.4. In paths, `14/` to `19/` stand for the stage folders under `research/jumbling/restoration/`. I do not choose any option under LQ1 to LQ21.

#### 1. Label table (default 20)

**Files**
- Full table: `work/reviews/language-round3/corpus-labels.json`
- Human summary: `work/reviews/language-round3/corpus-labels.md`
- Script: `work/reviews/language-round3/extract_labels.py`, with its inputs `rules.txt`, `refs.txt`, `ext.txt` and `rule.txt` in the same folder. Run it from the repository root.
- It is read-only on the corpus. Reruns gave the same SHA-256, `d6156092cb3cdc01fba013cfbfa37d2ce78bc8a8d766ca473a17faa80337bf13`, the last with only these files in the folder.
- Corpus manifest SHA-256: `e2b388befa1671be67219b23c4a565c003c8ae960bd0aa69e07ddb030a669702`.

**What was read**
- The scan reads Git blobs, so both files of a case pair are read.
- It covered 497 records: 417 `.json` (8,082,139 bytes) and 80 `.json.gz` (5,330,630 bytes compressed).
- There are no JSONL files. There were no parse failures, and no file was unreadable.
- The 53 `.ia16.gz` files are binary words with no fields.

**The six modified files**
- The six `16/results/runs/PX-*.json` working-tree files differ from HEAD. They are a case collision on Windows: each working file holds the bytes of its lowercase `px-*` twin, as the case-pair table in `corpus-labels.md` shows.
- I read them only as blobs and did not touch them.
- At HEAD the six `PX-*` blobs are `macro-certificate/1` records, 25,487 bytes in all.

**The rule for "enumerated"** (full text in `corpus-labels.md`)
- **Class S, enumerated string.** The last key is in a label lexicon (status, verdict, result, outcome, reason, stop, label, kind and the like), or the value starts with PASS, FAIL, UNKNOWN or INVALID, or holds `not_claimed`. In addition the value has at most 80 characters, is no digest, number, word code or path, and its (format, field) has at most 32 distinct values.
- **Class B, verdict boolean.** The last key names a check, a job's end, a search's coverage, a letter's outcome or a claim.
- **Not triples.** Prose fields are counted and kept word for word. Payload booleans are counted by last key. Numbers are skipped.
- **A record without `format`.** It gets the synthetic format `path:<stage>/<path, digit runs as #>`.
- **Field.** The key path, with `[]` for list items and `<k>` for keys that are numbers, digests or state names.

**Totals**

| Measure | Value |
| --- | --- |
| Formats | 160 (54 declared, 106 synthetic) |
| Triples | 387 (190 strings, 197 booleans) |
| Mapped | 298 |
| Unmapped, recorded with a stated reason | 89 |
| Without a rule | 0 |
| Occurrences | 22,513 |
| Extra rows | 24 (16 external, 8 values that code can emit but the corpus does not hold) |

- The 16 external rows are the 0.4 states A to D, the four H-08 states and the eight research labels.
- R2-E1's 60 formats covered the whole restoration archive, so they do not compare with 54.

The 89 recorded rows:

| Reason | Rows |
| --- | --- |
| Titles, modes, test expectations, predictions, run parameters: not verdicts | 43 |
| Predicates of an instance | 17 |
| Method or archive-integrity facts | 11 |
| Phase not run | 6 |
| Limits of a claim | 3 |
| The runner's own ladder | 3 |
| Review findings | 3 |
| One word holding two claims | 3 |

One row rarely states a whole claim. Of the 318 mapped rows (corpus plus external), 126 set a claim and 97 set a job outcome, so an envelope is built from several rows of one record.

**The ten most ambiguous cases** (rows, occurrences, formats, distinct mappings; substring counts)

| # | Word | Meanings found | Evidence |
| --- | --- | --- | --- |
| 1 | `budget` (3, 305, 2, 1) | A row of the T3P ledger F−D inside an exhausted family: a proved, scoped negative that names no resource. A mapper by word reads "not found within budget". | `14/scripts/t3p.py:557-638`; `15/README.md:127-129` |
| 2 | `exhausted` (18, 323, 8, 3) | `groups[].status`: a beam search ran to its depth, so not exhaustive. `families.[TMEL].exhausted=true`: the declared family was searched. `families.R.exhausted=false`: only the flag `depth<=2`. | `14/scripts/retained_search.py:107-148`; `15/scripts/pstar.py:285` |
| 3 | `stopped` (14, 102, 7, 2) | In `pstar-run/1`, explicitly not a negative certificate for family R. In `pstar2/2`, a negative for five declared families. Final stops of P** and P***: a negative for the declared set, with minimal X3 through its lemma. In `segments[]`, a transition. | `15/README.md:212-214`, `:236-242`; `16/scripts/pxrun.py:92-100`; `17/scripts/pxrr.py:98-120` |
| 4 | `unknown` (5, 6, 5, 3) | A mid-run snapshot left in the repository. The first write of a run stopped by hand. `reachability: UNKNOWN`, a claim about a state. Outside the corpus, H-08's `unknown` and the category `unknown_outcome`. | `15/scripts/pstar.py:925`; `17/results/runs/pxrr-7304.json`; `17/README.md:151`; `19/README.md:90-92` |
| 5 | `complete` and relatives (33, 68, 29, 11) | A job's end even when the content is negative (status complete with `uniform_*_guard_pass=false`). A finished enumeration. A run's terminal flag. A predicate (`complete_home_vertex_set_map`). A candidate's name. | `16/scripts/routing.py:89-133`; rows in `corpus-labels.json` |
| 6 | `pass`, `passed` (75, 367, 58, 12) | Producer audit and model review share the word. Some words hold a condition, such as `PASS_CONDITIONAL_SYMBOLIC_THEOREM`. `macro-check/1 passed` comes from the certificate's own program. | `15/README.md:414-416`; `19/README.md:32-34` |
| 7 | `verified` (24, 817, 13, 6) | `findings[].verification_status: verified` means the finding is real, so it speaks against the claim. Elsewhere it is a certificate check, or H-08's engine-checked state. | `docs/progress/1.0/command-table.md:21` |
| 8 | `solved` (8, 8, 6, 5) | Sticker sense, exact identity in `*_exact_solved` keys, and the limit `all_lattice_not_claimed_sticker_solved_or_exact_identity`. | `docs/progress/1.0/jumbling-handoff/research-data.md:33` |
| 9 | `blocked` (3, 496, 3, 2) | One word about three subjects: a word's case (468), a grip's certificate kind (24), a leaf's return (4). | rows in `corpus-labels.json` |
| 10 | `not_` keys (16, 20, 11, 5) | In `*_not_replayed`, `*_not_reexecuted`, `*_not_recertified` and `not_restored`, true means not done, beside `replayed=true`, where true means done. | `19/README.md:93-94` |

**Inferences to review** (for the mathematician and the main session)
- Rule V14 (35 rows) maps historical `*-independent*` reviewer records to model review with a fresh audit, and keeps KN6's "independent checker" for a 1.0 checker. This is inferred from names and `19/README.md:32-34`.
- Rule P13 (38 rows, 2,396 occurrences) sets check mode from a key's name alone.
- Rule K1 (27 rows, 14,786 occurrences) leaves the check to the enclosing record.
- Rule H1 maps reachability rows to open on one finite instance, as a claim of their own.

#### 2. Error registry (default 5)

Chapters 01 and 08 already spell 14 codes in `category.slug` form, and chapter 08 says chapter 13 owns the final spelling. I keep those spellings and add the prefix, apart from the changes below. No implementation is proposed.

**Code form**

```text
<source>:<category>.<slug>       for example KN5:stale.revision_changed
```

- `source` is the component that raises the code, or `EVD` for the evidence pillar. This is one of default 19's prefix kinds. Charter bars, contract amendments and research items go in the registry's rule column, because one code can rest on several rules.
- `category` is one of the sixteen below and equals the envelope's `category`. A lint checks this.
- `slug` is lower-case snake case, at most 40 characters. It names the fact, never the remedy, and never a twist or a correction (DT-J1 A5).
- The short form `category.slug` stays legal only in an `errors:` line of a component's section. The lint expands it there. `category.*` means every registered code of that category for that component.
- Records and the registry store the full form.
- A code keeps its meaning for ever and is never reused. A retired row stays, with its edition. `summary` may change and may be translated. Tests assert `code`, never text (V06, `10_V1_ARCHITECTURE.md:533-535`).

**Envelope additions to V06**
- `form` takes one of four values:
  - `diagnostic`: on a source span; nothing ran.
  - `refusal`: returned to the caller; no state change.
  - `stop`: the reason inside a job outcome.
  - `freeze`: a refusal that also freezes execution until recovery.
- `span` is added for diagnostics.
- `retry` gains one value, `after_user_request`: the same request may be sent again by a fresh user action, never automatically. `busy` and `cancelled` fit none of V06's four values (`10_V1_ARCHITECTURE.md:513`, `:521`).

**The sixteen categories**

| Category | Meaning | Recoverable, default retry | Display rule | Raising layer |
| --- | --- | --- | --- | --- |
| `input` | The request or file is wrong in a way the caller can fix. Nothing ran. | yes, `after_user_change` | Name the field or fact. Keep the draft. | Engine, session store, command layer, persistence |
| `unverified_reference` | A reference does not determine one correspondence. Not a mathematical impossibility. | yes, `after_user_change` | Ask for the disambiguating frame. Never say impossible. | Engine |
| `protection` | A protected set would move. A hard constraint, never a low score. | yes, `after_user_change` | Name the set and its level, and the two kinds of action. | Engine |
| `stale` | The request rests on a revision, preview, token, handle or job result that is no longer current. | yes, `after_refresh` | Explicit action: say what changed and keep inputs. Hover and picking: discard quietly (V03.3). | Command layer, engine |
| `cancelled` | Stopped on request, or because a commit or undo replaced the revision. State unchanged. | yes, `after_user_request` | No alarm. Say what was covered. Never "empty". | Engine (KN7) |
| `resource_limit` | A limit the job did not declare: a product ceiling or a machine limit. | yes, `after_user_change` | Name the limit and the actual value (V05.2). A machine stop reads incomplete and not reproducible. | Engine, session store, persistence |
| `consistency` | The kernel's own invariants disagree. | no, `never` | Stop and report. Draft kept. Hashes go only in `diagnostic`. | Kernel layers only |
| `unknown_outcome` | The fate of a durable command cannot be established. | no, `never` | Read-only recovery. Say "not known", never "failed". Draft kept. | Command layer, session store |
| `auth` | The authenticity of a connection or token is not established. | yes, `after_reconnect` | Say the link to the engine must be renewed. | Command layer, platform layer |
| `busy` | Another job or commit holds the lane. | yes, `after_user_request` | Show what runs. Offer wait or cancel. Never replay blindly. | Command layer, engine |
| `syntax` | Source cannot be parsed, or its edition header is missing or unknown. | yes, `after_user_change` | At the span. The last valid version keeps running (EV1). | Language core (LG1) |
| `type` | A value has the wrong puzzle, level, exactness, cost unit or mode, or a partial result or verdict is ignored. | yes, `after_user_change` | At the span, naming both types with their levels. | LG2 |
| `effect` | A program performs or inherits an effect its context forbids. | yes, `after_user_change` | At the span, with the call path that brought it. | LG3 |
| `capability` | A grant is not held at load or at run time. | yes, `after_user_change` | Name the capability and the profile. Granting is the solver's act. | LG3 host, command layer |
| `budget` | A limit the job declared. The same key stops at the same step. | yes, `after_user_change` | Name the budget, the amount used and what was covered. Never empty, never a negative. | Engine (KN7), interpreter (LG4) |
| `evidence` | A certificate fails, a premise is unmet, a scope or digest differs, or stored fields are inconsistent. | yes, `after_user_change` | The claim reads as it would without that support. A failed check is never shown as refuted. | KN6, evidence pillar |

**Rules for every category**
- A message says what happened, why, and what the user can do (charter E6).
- `summary` never needs internals (V06).
- A message never names which correction or twist to use (DT-J1 A5).
- Evidence, score and permission stay separate (charter M3).
- The draft survives every category (`10_V1_ARCHITECTURE.md:537-540`).

**Boundaries that must stay exact**
- `budget` and `resource_limit`: a code is `budget` only if the job declared the limit and the same reproducibility key stops at the same place. Product ceilings are deterministic, but no job declares them, so they stay `resource_limit`.
- `capability` and `auth`: a grant to a program, against the authenticity of a connection or token.
- `effect` and `capability`: found statically in source, against a grant checked at load or run time.
- `stale` and `consistency`: the revision moved, against equal revisions with unequal digests.
- `consistency` is reserved to the kernel layers, because it freezes execution. A fault in the runtime ends the job, not the session (KN7.13).

**Not errors.** These stay in the registry so nobody adds them as errors.
- Off the lattice: `Partial<T>`, "off the lattice: k pieces" (KN0.4, KN1.9). It has no code. Ignoring it is `LG2:type.partial_ignored`.
- `unknown(reason)` (KN0.5).
- Admissible, blocked, invalid, uncertain, and the value `Refused` of `derive` (KN2.8, KN2.11).
- Empty, not found within budget, not run (KN7.11).
- "Applied, display out of date" (KN5.22, R-02).
- Facts shown and not refused, such as a false step flag (ENG-0J item 3).

**First code list.** Items of the brief's minimum list are in bold. Spellings marked c01 or c08 come from chapters 01 and 08.

| Code | Form | Meaning | Rule source |
| --- | --- | --- | --- |
| `KN1:input.unknown_model` | refusal | Identity not in the registry | c01 |
| **`KN1:input.identity_mismatch`** | refusal | Model identity mismatch: model, menu, menu order or contract revision differs from what the value is read against; `context.field` says which. Never an empty list or "no solution". | c01, KN1.4, ENG-0J item 3 |
| **`KN1:input.level_not_held`** | refusal | Wrong level at run time | c01 |
| `KN1:consistency.identity_disagrees` | freeze | Two parts of one running session name different identities | INV-7 |
| `KN2:stale.revision_gone` | refusal | The store no longer holds the revision | c01, c08 |
| `KN2:stale.epoch_changed` | refusal | A handle or preview of an earlier engine epoch | c08 |
| `KN2:input.rotation_not_representable` | refusal | No exact form; never approximated | ENG-0J item 3 |
| `KN2:input.twist_off_menu` | refusal | Outside the signed menu | ENG-0J item 3 |
| `KN2:input.menu_invalid` | refusal | Not inverse-closed or not A4-invariant | ENG-0J item 3 |
| `KN2:input.word_blocked_prefix` | refusal | A stored word has a blocked prefix at this revision; carries the prefix length | ENG-0J item 3 |
| `KN2:input.recipe_shape` | refusal | Outside 1 to 256 steps, or not word and star steps | partition section 13 |
| `KN2:unverified_reference.frame_ambiguous` | refusal | The references do not fix one correspondence | V06 |
| `KN2:protection.orbit_would_move` | refusal | Label level | V06, KN1.11 |
| `KN2:protection.piece_would_move` | refusal | Pose level; never crosses levels | KN1.11 |
| `KN2:resource_limit.primitive_ceiling` | refusal | Above 3,000,000 primitives | V05.1 |
| `KN2:resource_limit.word_length_bound` | refusal | Beyond X3's bound, with the length shown | ENG-0J X3 |
| `KN2:consistency.fixed_nu_nonzero` | freeze | Nonzero in one of the nine fixed ν coordinates of I_a | ENG-0J item 3 |
| `KN4:input.unknown_session` | refusal | No such session | c01 |
| `KN4:input.checkpoint_unwitnessed` | refusal | Neither a witness nor a label certificate | ENG-0J item 3 |
| `KN4:resource_limit.disk_full` | refusal | A durable write could not be made; nothing half-written | R-06 |
| **`KN5:stale.revision_changed`** | refusal | Stale preview: its base differs from the current revision | c08 |
| **`KN5:stale.preview_replaced`** | refusal | Token mismatch: the token names a preview that is no longer pending | c08 |
| **`KN5:stale.token_mismatch`** | refusal | Token mismatch: a genuine token whose row, arguments, digest or identity differ | INV-3, CT-ONE-03 |
| `KN5:stale.token_used` | refusal | A single-use token seen twice | KN5.15 |
| `KN5:stale.no_fresh_preview` | refusal | The commit key was pressed with no fresh preview | c08 |
| `KN5:stale.undo_superseded` | refusal | Another action landed first | KN5.19 |
| `KN5:auth.token_invalid` | refusal | The authenticator fails | c08, narrowed |
| `KN5:auth.session_credential` | refusal | The local engine credential is invalid | V06 |
| `KN5:busy.commit_in_flight` | refusal | Another commit is in flight | c01, c08 |
| `KN5:input.command_id_reused` | refusal | Same command ID, other content | KN5.21 |
| `KN5:input.overlay_row_kind` | refusal | An overlay row of a forbidden kind | KN5.3 |
| `KN5:input.jumble_off_pieces_off_lattice` | refusal | Jumble off at N > 0 | ENG-0J item 7 |
| `KN5:input.jumble_off_nu_nonzero` | refusal | Jumble off at ν ≠ 0 | ENG-0J item 7, DT-J1 A5 |
| `KN5:input.jumble_off_not_shown_reachable` | refusal | No journal reaches the configuration | ENG-0J item 7, A5 |
| `KN5:input.jumble_on_labels_only` | refusal | Labels alone | ENG-0J item 7 |
| `KN5:input.retained_tool_needs_jumble_off` | refusal | A retained tool while Jumble is on | ENG-0J item 7 |
| `KN5:input.menu_word_needs_jumble_on` | refusal | A jumble letter while Jumble is off | ENG-0J item 7 |
| `KN5:input.menu_change_in_episode` | refusal | A menu change inside an episode | ENG-0J section 4 |
| `KN5:capability.commit_row_refused` | refusal | A runtime call names a commit row | c08 |
| `KN5:consistency.state_hash_mismatch` | freeze | Equal revisions, unequal digests | c08 |
| **`KN5:unknown_outcome.commit_fate`** | freeze | Unknown outcome: no receipt, broken continuity or an incomplete recovery record | KN5.23, V03.5 |
| `KN5:unknown_outcome.read_only_recovery` | refusal | A request during read-only recovery | c08 |
| `KN6:evidence.certificate_rejected` | refusal | A claim fails its check for the stated inputs | KN6.1 |
| `KN6:evidence.no_checker` | refusal | No checker for this kind | KN6.1 |
| `KN6:evidence.start_table_mismatch` | refusal | Wrong start table | KN6.5 |
| **`KN7:budget.steps`**, `.kernel_cost`, `.memory`, `.output` | stop | Budget exhausted: job outcome cut off, reads open | KN7.7, KN7.10 |
| `KN7:budget.not_declared` | refusal | A job without an explicit budget | KN7.7 |
| `KN7:budget.over_ceiling` | refusal | A budget above the profile's ceiling | BM3 |
| `KN7:resource_limit.safety_stop` | stop | Wall-clock stop: incomplete and not reproducible | KN7.10, INV-10 |
| `KN7:resource_limit.runtime_lost` | stop | The runtime died; job outcome error; session untouched | KN7.13 |
| `KN7:cancelled.by_user` | stop | Cancelled by the solver | V06 |
| `KN7:cancelled.revision_replaced` | stop | A commit or undo replaced the revision | KN5.18 |
| `KN7:busy.lane_full` | refusal | The lane admits no more jobs | KN7.2 |
| `KN7:stale.job_input_digest` | refusal | A job result is read against another digest | KN7.3, ENG-0J item 3 |
| `LG1:syntax.unexpected_token` | diagnostic | Parse error with recovery | LG1 |
| `LG1:syntax.edition_missing` | diagnostic | No edition header | LG1 |
| **`LG1:syntax.edition_unsupported`** | diagnostic | Edition unsupported, in source | LG1, PS2 |
| `LG2:type.mismatch` | diagnostic | A general type error | LG2 |
| **`LG2:type.level_mismatch`** | diagnostic | Wrong level, found statically | LG2 discipline 1 |
| `LG2:type.puzzle_mismatch` | diagnostic | A 600-cell-Full piece used as a jumbling piece | LG2 discipline 1 |
| `LG2:type.exact_required` | diagnostic | An approximate value in an exact position | LG2 discipline 2 |
| `LG2:type.cost_unit_mix` | diagnostic | Letters, generators or actions mixed | LG2 discipline 3 |
| `LG2:type.reachability_missing` | diagnostic | A state without a reachability claim | LG2 discipline 4 |
| `LG2:type.mode_mismatch` | diagnostic | A tool used in the wrong Jumble mode | LG2 discipline 5 |
| `LG2:type.verdict_as_boolean` | diagnostic | A verdict read as a boolean | LG2 discipline 6 |
| `LG2:type.partial_ignored` | diagnostic | A `Partial<T>` used without its off-the-lattice case | KN0.4 |
| `LG3:effect.undeclared` | diagnostic | An effect outside the declared set | LG3 |
| `LG3:effect.view_writes_workspace` | diagnostic | A view with a workspace write | LG3, default 16 |
| **`LG3:effect.flow_refused`** or **`LG3:capability.flow_refused`** | by LQ21 | Flow refused. Both spellings are reserved; LQ21 retires one before first use. | BM2, LQ21 |
| **`LG3:capability.not_granted`** | refusal | Capability not granted | LG3 |
| `LG3:capability.ambient_read` | refusal | A clock, file or network read without a grant | INV-10, LX-3 |
| `LG4:input.ir_rejected` | refusal | IR fails validation at load | LG4 |
| `LG5:input.module_digest_mismatch` | refusal | Module bytes differ from the lockfile | LG5 |
| `LG5:input.model_not_supported` | refusal | The manifest does not list the model | LG5 |
| `PS1:input.digest_mismatch` | refusal | Stored bytes differ from their digest | PS1 |
| `PS1:resource_limit.record_too_large` | refusal | Above the shared limits table | R-01 |
| **`PS2:input.edition_unsupported`** | refusal | Edition unsupported, in a stored record | PS2 |
| `PS3:input.export_off_lattice` | refusal | An export off the lattice | ENG-0J item 3 |
| `EVD:evidence.premise_unmet` | refusal | A cited premise is not held | 8.1 |
| `EVD:evidence.scope_mismatch` | refusal | A claim is read outside its scope digest | 8.1 |
| `EVD:evidence.inconsistent_fields` | refusal | Stored fields give no display word | section 13 |
| `EVD:evidence.negative_without_completion` | refusal | A negative from a job that did not complete | KN7.4 |

**Corrections for the drafted chapters**
1. `auth.token_invalid` (chapter 08, step 3 and section 15) joins two cases, with the continuation "press the commit key again". V06 says `auth` needs an authenticated reconnect, and pressing again cannot repair a bad authenticator. I propose the split above: `stale.token_used` and a narrowed `auth.token_invalid`.
2. KN5.16 says a mismatch is refused as stale, and chapter 08 step 7 says `consistency`. I support chapter 08. KN5.16 should be narrowed to a moved revision or digest binding.
3. KN7.7 puts `safety_stop` inside `Budget`, and `JobResult` writes `cut_off(budget: BudgetName)`. A `budget.*` code promises a reproducible stop, and a wall-clock stop cannot. The outcome should read `cut_off(by: Code)`, with `KN7:resource_limit.safety_stop` for that case.

#### 3. Figures for the interface contracts

Every figure here is a size or a count. No figure is a timing, and none was measured on the owner's machine. "Shown" means current code or a committed record demonstrates it. "Arithmetic" means it is computed from shown figures.

**600-cell-Full**

| Figure | Value | Class | Basis |
| --- | --- | --- | --- |
| Slots, pieces | 259,800; 177,120 | source | Shown: `core.Model` on `assets/` |
| Labels per revision | 1,039,200 bytes, int32 | source | Shown: `core.py:18`; the 0.4 transport sends them as `<u4` |
| `PuzzleState` working arrays | about 3.2 MB | source | Arithmetic on `core.py:257-268` |
| Generators | 600 move 4,596 slots, 600 move 4,600 | source | Shown: counted from `primitives.npz` |
| Delta per generator | 2 bytes as a letter; 36,768 or 36,800 bytes as (slot, label) pairs | source | Arithmetic |
| Pairs stop paying | above 129,900 changed slots | source | Arithmetic |
| Stored snapshot, zlib level 3 | 623,022 bytes solved; 624,979, 644,567, 706,909, 747,783 after 1, 10, 100, 1,000 random generators | synthetic | Shown: fresh model, seed 600, no session |
| Recipe ceilings | 1 to 256 steps; at most 3,000,000 primitives; chunks of 100,000 | source | Shown: `10_V1_ARCHITECTURE.md:359-363` |
| Largest possible recipe | 6,000,000 bytes int16; 12,000,000 int32; up to 18,000,000 JSON characters | source | Arithmetic |
| Largest recipe on record | None committed: oracle cases are generated, and I opened no user session | none | Not available |
| 0.4 journal row | two 64-character hashes, recipe, certificate and ten small fields; about 250 bytes for a one-generator turn with an empty certificate | source | Arithmetic from `session.py:31` |

**Jumbling puzzle**

| Figure | Value | Class | Basis |
| --- | --- | --- | --- |
| Pose IDs per revision | 708,480 bytes, int32 | source | Shown: `research/jumbling/sim/state.py:103` |
| Same in `u32le-base64` | 944,640 characters plus SHA-256 | source | Arithmetic on `research/jumbling/differential/candidate/store.py:24-28` |
| Pose-table entries P | largest 2,049 (stage 18 lattice-d1); others 317 to 1,827; fixtures 1,248 to 1,591 | source | Shown: 12 records, 3 fixtures |
| `poses.f32`, `pose_lattice.i32` at P = 2,049 | 131,136; 8,196 bytes | source | Arithmetic |
| Render arrays at P = 2,049 | 847,812 bytes | source | Arithmetic |
| One mask of 177,120 bits | 22,140 bytes | source | Arithmetic |
| Render arrays plus one mask | 869,952 bytes | source | Arithmetic |
| Exact pose as JSON | mean 146, largest 169 characters; 298,568 for P = 2,049 | source | Shown |
| Largest integer in those tables | 3 decimal digits; fixture heights at most 32 | source | Shown: nothing on record reaches 2^64 |
| Grip sets | 75 bytes each | source | Arithmetic |
| Rim summary | 600 rows; about 14,400 bytes at six 4-byte fields | source | Arithmetic; width assumed |
| End survey with certificates | 205,233 to 209,045 JSON characters; 563 to 578 blocked grips | source | Shown: `research/jumbling/fixtures/wj-*.json` |
| J1 journal row | mean 274 to 276, largest 290 JSON characters over 2,366 rows | source | Shown: the same fixtures |
| Pieces moved per twist | 1,637 to 4,193; medians 2,566 to 2,749 | source | Shown: the same fixtures |
| Delta per twist | up to 33,544 bytes of (piece, pose ID) pairs, plus the remap of P × 4 bytes, plus new poses | source | Arithmetic |
| Menu record | 10,750 characters (I_a, I_b); 4,362 (S4₀) | source | Shown |
| Letters | K⁺ order 7,200; 36,000 letter codes, which fit 16 bits | source | Shown: `research/jumbling/tutorial/07-records.md:311-319` |
| ν and corrections | 45 coordinates; 36 corrections of 5 to 5,629 twists | source | Shown: ENG-0J, KN3.2 |
| Labels | 1,039,200 bytes, on the lattice only | source | Arithmetic |

**Words on record**

| Figure | Letters | Raw `ia16` bytes | Class | Basis |
| --- | --- | --- | --- | --- |
| Longest literal word stored flat: `18/results/lattice-d1/run/words/00014.ia16.gz`, step 14, rule `lattice/orbit-10` | 2,352,984 | 4,705,968 (118,208 gzipped) | source | Shown: `run.json` and the blob |
| That phase, 53 files | 13,231,266 | 26,462,532 (1,965,617 gzipped) | source | Shown |
| Longest E2 literal step | 672,417 | 1,344,834 | source | Length shown; bytes arithmetic |
| Stage 18 admission bound, per word | 10⁷ | 20,000,000 | source | `18/README.md:169`; bytes arithmetic |
| Lattice word, checked by composition, not played | 166,788,405 | 333,576,810 | source | `docs/progress/1.0/jumbling-handoff/research-data.md:125`; bytes arithmetic |
| Largest retained cascade | 224,683,850 | 449,367,700 | source | `research/jumbling/restoration/README.md:40`; bytes arithmetic |
| The 2,352,984-letter word as J1 rows | n/a | about 649 million characters at 276 per row | source | Arithmetic |
| `macro-certificate/1` | 66 records, 226,451 bytes, 1,057 to 10,652 each | n/a | source | Shown |

**Shared**

| Figure | Value | Class | Basis |
| --- | --- | --- | --- |
| Digest | 32 bytes, 64 hex characters | source | Shown |
| Identity block, revision reference | about 110 and 200 bytes in binary | source | Arithmetic; widths assumed |
| Command rows | 86; IDs 1,340 characters (mean 15.6, longest 30); static table 21,348 JSON characters | source | Shown: `docs/progress/1.0/command-table.json` |
| Availability per revision | about 1.4 KB when all are available; about 15 KB if all 86 carry a compact error of about 150 bytes | source | Arithmetic; error size assumed |
| Language overlay rows | Not known yet | none | Depends on LP2 onward |
| Value-store entry | 1,039,200 bytes; up to 1,007,048 at P = 2,049; plus a 32-byte key | source | Arithmetic |
| Reference-map cache entry | about 1.75 MB plus words | source | `10_V1_ARCHITECTURE.md:471-472`, proposal |

**Consequences for the drafted clauses**
1. KN2.10, CT-KN2-07 and partition LX-5 are out of date (card R3-E4).
2. KN7.9 counts memory as "the length of a value's canonical encoding" but gives 708,480 bytes. The existing store form is 944,640 characters, and PS1's canonical JSON would be larger again. Proposed wording: "Memory is counted in logical units: the byte length of a value's arrays in the KN1 digest convention, plus, for a pose table, the length of each entry's canonical exact key. The text form in a file is never the unit."
3. A jumbling delta cannot be only the moved pairs. `_compact` reorders the pose table after a commit and remaps every pose ID (`research/jumbling/sim/state.py:385-397`), so unmoved pieces can change ID. A delta carries the remap, or the revision carries the full array. This agrees with KN2.9.
4. KN3.2 should read "pose entries can exceed 2^64". The basis is unbounded height under I_a (ENG-0J section 4), not a record, so the test needs a synthetic fixture.

#### 4. Bars for spikes

All of this is proposal. Running any spike needs the owner's allowance (LQ18).

##### LX-4 editor latency

**Module sizes** (source: the 33 research scripts run from 55 to 1,034 lines, median 164, 7,723 in all, about 47 bytes per line)

| Size | Lines | Why |
| --- | --- | --- |
| S | 200 | Near the median research script |
| M | 1,000 | The largest research script |
| L | 5,000 | Stress, five times the largest |
| W | 50 modules, about 20,000 lines | About 2.6 times all 33 scripts |

**Bars, p95 / p99 in milliseconds**

| Interval, from the key message | S | M | L |
| --- | --- | --- | --- |
| Typed echo | 50 / 67 | 50 / 67 | 50 / 67 |
| Syntax feedback | 67 / 100 | 67 / 100 | 100 / 150 |
| Completion list shown | 100 / 150 | 100 / 150 | 150 / 250 |
| Type and effect diagnostics, from the last key of a burst | 150 / 250 | 200 / 300 | 500 / 750 |
| Stale state shown on a view (charter D2) | 100 / 150 | 100 / 150 | 100 / 150 |
| Reference view re-run, updated view presented | 250 / 400 | 250 / 400 | 500 / 750 |

At W, an edit to an exported signature has dependents re-checked within 1,000 / 1,500 ms.

**Reasoning**
- The gate allows 33.3 ms frames, and a key can land just after a frame samples input, so two frames (66.7 ms) is the honest worst case at the 30 fps floor.
- Syntax feedback must land before the next key. A fast typist's gap is about 125 ms, which is a common estimate, not project evidence.
- Completion uses 100 ms, the project's existing meaning of "responds" (charter E2).
- Diagnostics are not on the keystroke path. The bars sit between the usual 0.1 s and 1 s human-factors limits, which are also not project evidence.
- p99 is about 1.5 times p95. Any debounce counts against the bar.
- The re-run bar applies only to a reference view: about 200 lines, one batch read of labels or pose IDs, and a per-orbit count. Another view has only the stale-state bar.

**Protocol on the owner's machine**
1. Freeze one candidate and its build identity. The module set is generated deterministically from the three LP3 tools and frozen by digest.
2. Use a fixed, seeded edit script with four edit kinds: inside a body, a change to an exported signature, a completion trigger, and a break in syntax.
3. Run with Jumble off and with Jumble on, at a fixture's end state, on fresh isolated data.
4. Record the D6 conditions exactly, with frame generation off, PresentMon pinned (D7), and a quiet machine.
5. Timed input is an owned-window message posted to the window's queue. Start the clock at the post. The end is the first presented frame that shows the result, matched by frame ID to PresentMon.
6. Unlike the M1 series, automatic rendering stays on, because the gate must hold in the same run.
7. One smoke run, then three series of 100 samples per metric, size and puzzle. No series stops early for being slow. A run that fails a correctness or state-hash check is invalid, kept and repeated whole.
8. In the same runs, capture PresentMon at W3 and script at least 100 turns per series for E2.
9. Report the median, p95, p99 and largest value for each series, with all three series shown.

That is 10,800 samples, about three hours of foreground time at one second per sample (arithmetic), so it needs an agreed window.

**Proposed decision rule.** "On the owner's machine, with one frozen candidate per route, the fixed module set and the fixed edit script: a route's editor path passes when every bar holds in each of three series at S, M and L on both puzzles, and the renderer gate (259,800 slots, average at least 30 fps, p99 at most 33.3 ms, peak VRAM at most about 7 GB) and charter E2 hold in the same runs. A bar missed at S or M fails the editor path until it is fixed. A bar missed only at L or W goes to the owner with the numbers. If editor work makes a run miss the gate or E2, the route fails at any size. No bar is relaxed without the owner."

##### Exact wording for the other rules

**LX-1.** Missing: a route-neutral meaning of "step", a sample plan and a unit of cost.

"For each route and flow-label mode, run the three LP3 tools on one frozen input set, listed by digest. Report per tool: steps, kernel calls by row, bytes crossing the kernel boundary, and time per step and per call as the median and p99 of three series of 100 runs. A step is one instruction of the route's executed form, and the report names that form. Step counts are never compared across routes. The result is a lead for LQ9 and LQ21. No LX-1 figure passes or fails a route, and none stands in for charter E2. The decisive check is LP3's end-to-end E2 measurement on the owner's machine."

**LX-2.** Missing: "responsive", "unchanged", the sample count during a fault, the meaning of startup, and the jumbling payload.

"On the owner's machine, on a frozen candidate, with fresh isolated data, inject each fault into the runtime with a session open: allocation to its limit, recursion to its limit, a loop with no kernel call, and a forced kill. A placement passes only if, for every fault: (1) the journal rows and state hash afterwards equal those of a control run of the same commands without the fault; (2) the job ends with a typed outcome and the session stays open; (3) during the fault window the renderer gate holds and the LX-4 echo bar holds; (4) at least 100 commits issued during fault windows meet charter E2, the fault being repeated until 100 are observed. A placement that cannot survive the forced kill fails. Transfer is measured at 1,039,200 bytes of labels and at 869,952 bytes for the pose bundle at P = 2,049 with one mask. Startup runs from process start to the first interactive frame, with an existing session and the runtime loaded, in each of three cold runs, within the 3 s that charter E3 proposes." E3 stays a proposal until the owner sets it.

**LX-3.** Missing: repeat count, the bytes compared, what counts as a refusal, and the denominator of false refusals.

"Cases and keys are frozen by digest before the run. Each key runs ten times on Windows and ten on headless Linux, and all twenty runs give one SHA-256 over the canonical result bytes: the result value and stop reason in canonical JSON, with timing, host and path fields excluded and listed on the experiment card. A display plan agrees when its non-numeric fields are equal and each numeric field is within its declared precision. A case counts as refused only if it ends with its expected registry code and no state change, and a planted read returned no data. A crash, a hang or another code is a failure of that form. A false refusal is a refusal of a program on the frozen list of built-in and solving-workflow tools that the interim boundary allows. For each form and variant the report gives the count and each refused program with its code. No threshold is set; the count goes to the owner with LQ21."

**LX-5.** Missing: current sizes, an equality rule and a source for the large entries.

"Sizes: integers at 2^64, 2^256 and about 22,500 bits; synthetic pose entries built from the J1 negative control, because no table on record exceeds three decimal digits; literal words of 672,417 and 2,352,984 letters and the 10⁷ bound; 13,231,266 letters as one stream; and the 166,788,405-letter length as exact length arithmetic. Every candidate's results equal Python's exact integers on every fixture, and a difference removes the candidate. Speed is reported from the owner's machine, as median and p99 per operation and size, with no speed threshold at this stage. A licence list goes to the owner, and a copyleft dependency needs the owner."

**LX-6.** Missing: the hot path, the oracle cases and the machine. The wiki question says "the same headless machine", while the partition counts speed only from the owner's machine.

"The port covers applying a permutation and composing macros. Cases O01 to O07 of `tools/oracle/` give the same state hash, net digest and expansion digest as the 0.4 reference through `tools/oracle/compare.py`. Any difference is a demonstrated failure and stops the spike until it is explained. Speed compares the same inputs against NumPy in the pinned engine environment on the owner's machine, by the B4-12 method. A cloud or headless timing is a lead only. There is no speed threshold: both results go to LQ8 with LX-5."

**LX-7.** Missing: the gate numbers, whether a pass is per run, and a contradiction. A GPU evaluation cannot be plain "headless"; it needs a named device.

"At W3 on the owner's machine, the generated chart stage and the built-in perspective stage each make three cold runs. A candidate passes when each of its three runs meets the whole selection gate: all 259,800 slots at full detail, the most complex animation, average at least 30 fps, p99 at most 33.3 ms, PresentMon at native resolution with no frame generation, and peak VRAM at most about 7 GB. The report shows the difference from the built-in stage. For exact vertices and camera poses fixed before the run, GPU and CPU evaluations agree within the precision the chart declares, outside its declared singular neighbourhoods. That comparison runs without a window, names the device it used, and is never a gate result."

**LX-8.** "No rule exists until the owner sets the protocol (LQ13). Agreement between authors or models does not replace it." `needs_owner: yes`.

**LX-9.** Missing: what "differ" compares, what happens when only target bits differ, and what no difference means.

"Step 1 compares the 64-bit patterns of the target coordinates for the 13,600 requests of R2-E2 on Windows CPython, Linux CPython and the LX-6 engine candidate. Step 2 places at least 1,000 requests within one ulp of exactly computed bisectors, including those step 1 found to differ, and compares the exact key of the chosen rotation. If any chosen rotation differs between any two runtimes, the request is recorded as a dyadic Cayley vector read exactly, and a drag compares its 60 candidates exactly in Q(√5). That amends the state contract: EN1's Astra check, then the owner. A difference in target bits alone is reported and feeds step 2. A run that finds no difference is a sample: it holds for the requests tested only, the report says so, and the amendment stays open to the owner."

#### 5. Cards and explainers

- id: R3-E1
- on: partition 8.1, PS3, default 20, LQ3, LQ10, my R2-E1
- kind: proposal
- claim: Mapping by (format, field, value) works on this corpus, but job outcome needs two more import values, "not run" and "not stated", and "not stated" must block a negative claim. I also withdraw my R2-E1 sentence that "exhausted" next to "best" closes a search over a declared set: that status is a beam search that ran to its depth.
- evidence:
  - 387 triples: 298 mapped, 89 recorded, none without a rule; seven exact words have more than one mapping (`corpus-labels.md`).
  - Default 6 keeps "not run" apart, while KN7's outcome has four values; six rows fall under rule U2.
  - `17/results/runs/pxrr-7304.json` reads status unknown, reason initialised; only `17/README.md:151` says it was stopped.
  - `14/scripts/retained_search.py:107-148` against `docs/progress/1.0/design-table/topics/language-ecosystem.md:528`.
- needs_owner: yes

- id: R3-E2
- on: partition lines 279 and 662, defaults 13 and 20; topic lines 523, 526 and 830
- kind: objection
- claim: The corpus must be identified by Git blobs and a digest manifest, never by a checkout. On Windows the six case pairs collapse, so a walk of the checkout never sees six `macro-certificate/1` records. That is why the record says 411 files and 13,387,282 bytes.
- evidence:
  - Demonstrated: 417 plain JSON files and 13,412,769 record bytes at HEAD; 417 − 6 = 411, and 13,412,769 − 25,487 = 13,387,282.
  - Correct wording for line 279: "156 of the 417 JSON files have no top-level `format` key, and 146 have none at any depth".
  - Line 662 should read "497 records, 13,412,769 bytes at Git HEAD".
  - Add to default 13: "The corpus is read from Git blobs through a manifest of path, blob digest and size; no importer or test walks a checkout."
- needs_owner: no

- id: R3-E3
- on: default 20, PS3
- kind: proposal
- claim: Default 20's test cannot run until "field", "value" and the enumeration rule are defined. It must also cover booleans with their polarity and keep prose scope text beside each mapped row. I propose the rule of `corpus-labels.md`.
- evidence:
  - 197 of 387 triples are booleans, and 16 rows have inverted polarity.
  - 25,134 boolean leaves exist, of which 10,388 over 175 keys are payload, so "every value" needs a stated boundary.
  - 436 prose leaves over 16 keys are kept, 393 of them `scope`; a mapped word without its scope text overstates the claim.
- needs_owner: no

- id: R3-E4
- on: partition LX-5, KN2, KN4, section 13; chapter 01 KN2.10 and CT-KN2-07; LQ10
- kind: objection
- claim: "The largest word on record has 672,417 twists" is wrong. The longest literal word stored flat has 2,352,984 letters, and composed words reach 224,683,850. A long word cannot be kept as J1 rows either: one word commit should be one journal row citing the literal letters by digest, stored as `ia16` chunks.
- evidence:
  - `18/results/lattice-d1/run/run.json`, step 14, with its blob of 4,705,968 bytes.
  - About 649 million characters as J1 rows, at 276 per row, against 4.7 MB (arithmetic).
  - `docs/architecture/1.0/language/architecture/01-kernel.md:208-209`, `:558`; partition line 607.
  - Proposed test: CT-KN2-07 streams 2,352,984 letters and also 13,231,266 as one stream.
- needs_owner: yes, for the word encoding of the data core (LQ10); the figure corrections need no one

**P-E6. Why the runtime stays off a turn's critical path.** When you press a twist key, the program has about a tenth of a second to respond, while it keeps drawing every sticker. A program you or someone else wrote can be slow, can have a bug, and can even run for ever, and nobody can know that beforehand. So the key goes straight to the engine, which makes the turn and writes it down, and no user program stands in that line. User programs work beside it: they read a frozen copy of the puzzle and show their results when they are ready. If one is slow, its panel is marked out of date, and your twist still happens on time. If one crashes, only that program stops, and the puzzle, its history and the picture carry on. This is a design promise until it is measured on your laptop, which is what spike LX-2 is for.

**P-E7. What a deterministic budget is, and why seconds cannot be one.** A budget says how much work a program may do before it is stopped. A deterministic budget counts things that come out the same every time: program steps, declared engine work, memory in fixed units and the size of the output. Run the same program on the same input with the same budget, and it stops at exactly the same point on any machine. Seconds do not behave that way, because the same run takes longer when the laptop is warm, on battery or busy with something else. A search stopped after ten seconds could find a result today and miss it tomorrow, and nobody could repeat it to check. "Not found within this budget" only means something if someone else can get the very same answer. Seconds are still shown for information, and a clock can still stop a run that has gone wrong. A run stopped that way is marked incomplete and not repeatable, and it can never support a claim that something does not exist.

**P-E8. What the kernel's interface gives a program, and what it never gives.** The kernel is the small exact core that knows the rules of the puzzle. A program can ask it questions: read a frozen copy of the state, work out what a word of moves would do, or try moves on a private copy. Every answer says which state it is about, and "I don't know" is a real answer that never turns into "no". A program can also prepare a proposal, which the engine replays by itself and shows you as a preview. A program never gets a writable state, never gets the key that commits a move, and cannot hand the engine a result to adopt. It gets no clock and no files unless they are granted, and no network at all. Only your own key press on the preview you can see commits anything, and the engine checks everything again at that moment. So a wrong program can show you something wrong, but it cannot damage your session.

**P-E9. Why a stored format must be frozen before the program that reads it.** A saved file outlives the program that wrote it, and your macros and checkpoints must still open in every later version. Code can be fixed any day, but a file already on your disk stays exactly as it was written. So the file's exact layout is agreed first, and the reader is written against that agreement, not the other way round. If the reader came first, its accidents, such as a field order or a lazy default, would quietly become the format, and every later version would have to copy them. Freezing also forces the hard question early: anything the format cannot record is lost for good when old data is imported. That is why the proposal under LQ10 freezes only a small core: words, recipes, macros, protection, filters, checkpoints, bindings and the evidence record. Other things, such as the syntax, can keep changing, because each change comes with a recorded migration. An old file then keeps the meaning it had on the day it was saved.
