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

Both are cleared: see [the outcome of round 3](#outcome-of-round-3).

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

### Mathematician

Both open obligations are answered.

- **Label-lift (N1):** the lemma holds off the lattice, with a full proof.
- **R2-E1 to R2-E4:** most adoptions stand. These do not stand as written:
  - "one bit per run" is false;
  - "combine by the strongest" is undefined on incomparable arguments;
  - LB4's vertex rule cannot establish a region property;
  - the scope rule "immutable input digest" would exclude every theorem;
  - "height zero" means the lattice only on Γ(Λ);
  - KN1.7 (chapter 01:118) names the wrong kernel for the map from labels to position.

Default 10's key property is true only under four hypotheses, proved in R3-M2.

**Source and abbreviations.** Source state: Git HEAD `ea73aaef`; I did not read the six modified `PX-*.json` files.
- partition = `docs/architecture/1.0/language/ecosystem-partition.md`
- contract = `research/jumbling/state-contract.md`
- tex = `research/Full_600cell_Puzzle_Theory.tex`
- restoration = `research/jumbling/theory/restoration.md` (earlier cards call it "trees")
- stage N = the README of that stage under `research/jumbling/restoration/`
- state.py = `research/jumbling/sim/state.py`
- chapter NN = the chapter of that number under `docs/architecture/1.0/language/architecture/`

**Evidence.** Every computation here is synthetic exact geometry: headless, fresh data from `assets/`, no session. None of it is Windows, DirectX, input or performance evidence.

**Scope of this record.** I choose no option under LQ1 to LQ21. "(preference)" marks notation that can change without touching a proof. N1 and N2 fall inside bar M4's reduced scope, because the identity levels and the display words rely on them. The engineer's round 3 record was in the file when I finished, so R3-M1 also answers R3-E1, R3-E2, R3-E3 and its "Inferences to review".

#### A. Cards

##### R3-M1

- id: R3-M1
- on: R2-E1; Fix-first item 11; the Adopted rows on the import table, discharge, pinned inputs, the 0.4 import and scope; partition 8.1, section 13, PS3; also R3-E1, R3-E2, R3-E3 and the engineer's "Inferences to review"
- kind: support
- claim: Facts stored and judgements derived is sound: every judgement M1 and M2 need is a function of the stored facts and four versioned tables, and N2 gives that function, but only once the facts gain seven fields. The import table, identical-ID discharge, the pinned-input rule and the 0.4 mapping stand; the scope rule and "refutation is asymmetric" need the corrections below.
- evidence:
  - **Derivability.** M1's Claim values, M2's summaries and the display words are all computed in N2 (C.4, C.8).
    - The only inputs that are not record facts are four versioned tables: the order, the invalid pairs, the threshold and the import table.
    - So a word is a function of the facts and an edition. N2's proofs hold for any finite poset, so changing a table is an edition and breaks no proof (partition:659-660).
  - **Seven fields the facts must gain:**
    1. Statement form (finite, universal, existence, mixed), with every parameter bound in the ID. Refutation depends on the form (C.12, correction 4).
    2. The negation pair. One record, `535:1,363:5,535:1,597:14`, gives "finite instance" for "a descent exists here" and "refuted" for "no descent exists here" (stage 15:223).
    3. The subject level (counterexample under Scope below).
    4. Scope-membership evidence. A failure at stage 19's synthetic state says nothing about reachable states, because that state's reachability is unknown (stage 19:88-92).
    5. Completeness or closure of a search, apart from the job outcome. A completed beam job can be empty while 228 descending words exist (stage 15:212-223).
    6. The premise class: reachability promise, open hypothesis or model assumption.
    7. The tool digest of each check and the declared common base, which "independent" reads (C.6). `macro.py` holds both `cert` and `check --replay` (stage 15:415).
  - **Stand as adopted:** the import table; identical-ID discharge; the pinned-input rule (N1 and Lemma 14, tex:1431-1444, supply transports); the 0.4 mapping. Versioned functions are mathematically sound; the choice stays LQ10.
  - **Discharge.** It is sound if IDs are injective on meanings, so an ID must bind every parameter, the puzzle and the level. An ID that omits k would let a proof for D[7] discharge a premise about D[9].
    - A universal reaches an instance only through an AND node with the instance's membership check.
    - Equivalent statements under different IDs stay refused until a bridge lemma with its own ID links them. That is the safe direction.
  - **Refutation** stands with C.12 corrections 3, 4 and 5.
  - **Scope, two changes and a gap:**
    - "Immutable input digest" would exclude every theorem and every bound on D[k], which have no input digest (partition:244, :665; PS1.35 at chapter 05:454). Rule: every scope field is immutable, and no field is revision-bound context.
    - "A scope names the level its digest covers" stands, with a rule: the digest must be at least as fine as the statement's subject level, and a coarser one is a typed inconsistency. Counterexample: "Z is reachable in J(A4₀)" with a label digest. Solved and solved-with-centre-0-turned share their labels, and the second is unreachable (B.5).
    - The list "(labels or poses)" has a gap off the lattice. Labels do not exist there, so pose is the only stored level until a sticker-class digest exists (B.7).
  - **Summary size.** R2-E1's 90 becomes 30: check mode is a fact and leaves the order (C.2).
  - **Import limit.** R3-E1 withdraws the "exhausted" example, and that shows the limit: one (format, field, value) row rarely states a claim. A value whose meaning depends on a sibling field stays recorded until a whole-record rule maps it, which is the safe direction because a recorded claim reads open. Stage 15's `stopped` is a negative for T, M, E and L and not for R (stage 15:212-214). The count is 156 of 417 files with no top-level `format` key and 146 with none at any depth (partition:279-280).
  - **Engineer's round 3 record:**
    - R3-E1, support. "Not run" and "not stated" are import values, not KN7 outcomes. "Not run" reads row 7. "Not stated" makes a negative leaf inert and is not an inconsistency (C.9).
    - R3-E2, support. A leaf's identity needs exactly this blob digest.
    - R3-E3, support. A mapped word without its scope text overstates the claim, and a scope that cannot be typed keeps the row recorded. One term per concept: R3-E3's "polarity" is the sense of a boolean key; my field 2 is the negation pair, so I do not call it polarity.
    - Rule V14 stands. KN6 is defined by properties the historical reviewer records do not show, and the two checks are incomparable, so the mapping loses no strength. Limit: it applies to reviewer records, not to program checkers such as `replay_word.py`.
    - Rule P13 is safe because no word reads check mode. Condition: a mode read from a key name never creates a transport edge.
    - Rule K1 is safe when the enclosing record's check covers the row. Otherwise the check reads none.
    - Rule H1 is correct. An open reach(Z) row adds no OR child, so it can discharge nothing (C.10).
    - The `budget` row of ambiguity table case 1 shows why row 5 reads the search record's stop reason and never the word.
- needs_owner: yes. These facts bear on LQ3 and LQ10. I choose no option.

##### R3-M2

- id: R3-M2
- on: R2-E2; Fix-first items 13 and 15; the Adopted rows on kernel calls, the token, tests, charts and LX-9; default 10; LB4
- kind: objection
- claim: The placement, the token, the equality key, items 13 and 15, and LX-9 stand, and default 10's property is true under four hypotheses, proved below, and false without them. LB4's "on exact vertices" must change, which also corrects my M4.
- evidence:
  - **Stand as adopted:** kernel calls typed by puzzle and level; typed partial results; small exact algebra as library code; the token with the menu-order digest; literal letters for preview, protection, commit and token (peak height and Strict protection read the states between letters); LX-9; picking by instance IDs; the shader test.
  - **Default 10, proposition.** Let w be a word and key(w) the word that replaces every maximal run of adjacent letters at one grip c by the single letter (c, g_m⋯g₁).
    - Under the four hypotheses below, w is admissible at X exactly when key(w) is, with the same end state and so the same end digest.
    - Proof. g fixes n_c, so it maps both closed half-spaces of c onto themselves. After an admissible (c, g₁), c has no straddler and the same inside set. So a run at c is admissible exactly when c has no straddler at its start, and it ends where (c, g_m⋯g₁) ends. Induction over runs gives the claim.
  - **The four hypotheses:**
    1. Adjacent same-grip runs only.
    2. Identity products are kept. (2, id) at a blocked grip measures `blocked`, while the empty word is admissible. Stage 16's R1 and `_journal_witness` drop identities, which is valid only on words already admitted (stage 16 PROOFS.md:53-69; state.py:489-510). LB1.9 and CT-LB1-02 already agree.
    3. The menu is closed under products. A4₀, S4₀ and I_a are; the contract allows menus that are not groups (contract:151-154). With a 16-item menu, g a half-turn fixing n_0 and n_13 (`research/jumbling/sim/accept.py:964-975` style), (0,g), (0,h) are both admissible and the merged letter is `invalid`. For such a menu the key holds group elements and is not run through the menu test.
    4. Exact classification, not budgeted `uncertain` (B.6 (d)).
  - **The key is sound, not complete.** Equal keys give equal partial maps; unequal keys say nothing, because letters at grips that do not interact commute (stage 16 PROOFS.md:58-63). A trace key needs the global lexicographic rule and its own property test (LG2.18 already says so).
  - **Consequences of N1:**
    - Partition:152's "lower" and "higher" must read coarser and finer on a branching order (chapter 01 KN1.7 and KN1.10 already do).
    - Heights can batch per coset.
    - A lattice test by heights holds only on Γ(Λ). M3:371's "lattice (height 0)" and chapter 02:130's "equivalently height zero" need that scope: Cayley ω = (2,0,0) at cap 0 is not in K⁺, has heights (0,0), and has cosine −3/5 (B.6 (e)). T3's proof needs entries in Z[φ][½] (restoration:25-28).
    - In free mode, domain IDs go by exact coset tests, not tree positions, because T6 uses T3. Add a free-mode case to the height tests.
    - The label-lift property test (CT-KN2-06) becomes a conformance test with S, R and σ ≠ id cases, no longer a lead for the lemma.
    - KN1.7's map table (chapter 01:118): the kernel of labels to position is the frame group R_A/S_A, not "sticker orientation within a slot". A3's orientation is pose-level and labels do not read it (B.6 (d)).
  - **LB4, objection.** A per-vertex test can refute a region property; it cannot establish one.
    - Domain: take a stereographic chart with its pole on the axis of cell c. The centre piece's 8 vertices are at least 0.2296 from the axis, and its centroid is on it (computed distance 1.9e-16). So the vertex test passes while the chart is undefined at an interior point.
    - Injectivity: a linear map to 3D can be injective on the vertices and is never injective on a 4D region.
    - Section membership does reduce to vertices, by convexity.
    - Fix for LB4, LB4.2, RPI.30 and CT-LB4-01: decide domain by an exact region test (KN3's polytope operations), injectivity by a theorem or an exact region test, and use vertices only where a theorem reduces the property. Include the posed vertices. The restriction to exact camera poses stands.
    - My M4:409-410 used a pole at a 600-cell vertex, where the singular point is itself a piece vertex. That example hid the gap.
- needs_owner: no

##### R3-M3

- id: R3-M3
- on: R2-E3; Fix-first items 16 to 19; partition section 11 facts; BM2; LX-3
- kind: objection
- claim: R2-E3's composition is right, and no composition-closed class excludes it; but "one bit per run" is false, and Rice's theorem does not carry the argument once budgets exist. Items 16 to 19 stand with the additions below.
- evidence:
  - **Audit of the facts (partition:479-495):**
    - F1 (:479-480): true.
    - F2 (:481-482): right conclusion, wrong reason once budgets exist. A budgeted program is total, so Rice's theorem no longer applies as stated: one run is decided by running it, and only a check over all states stays out of reach. The decisive fact is that the verdict is not a function of (word, state), because the identical pair can be the solver's named macro or a search's output. This also corrects M5:437.
    - F3 (:483-484): true. In_c and Out_c are kept by a twist of c and A_c ≤ min(In_c, Out_c), so N_c ≥ F_c. "Lower it" means that piece, at each own holder (M5:429-430).
    - F4 (:485-488): the first half is true (45 coordinates, 36 corrections, `Span.solve` over F2 and F5, nucoords.py:170-196). Precise form: any composition-closed class containing the ν read, the rows and F2/F5 reduction contains the forbidden function, and "searches" is intensional, since a lookup table computes the same function. "Only an enumerated list with flow control" is overstated: a list that leaves a part out also stops the shipped composition (BS2, partition:545). What a solver writes from base parts, only flow control stops.
    - F5 (:489-491): "one bit per run" is false (below). The display-sink sentence is true.
    - F6 (:492): overstated. The checkable part is a flow property. "Themselves" and "understanding" are not, because two identical flows can differ in whether the person understood.
    - F7 (:493-495): true, with limits: flows inside the sandbox and records the system writes.
  - **Counterexample to one bit:**

    ```text
    for j in public_list:
        preview(letter[j])       # constant word, passes a label check
        if state_test(j): stop   # state-dependent stop, invisible to the rule
    ```

    The pending preview (a later one replaces it, `KN5:stale.preview_replaced`) is the program's choice among up to 36,000 letters, log₂ 36,000 ≈ 15.1 bits, and `state_test` may hold a whole search. One bit holds only for one preview as the last action on a path with no state-dependent stop. LG3.15 already says data and branch checks do not close these channels; the partition's F5 and the Adopted row's "one-bit leak" need the correction.
  - **Three fixes, no option chosen:** one preview per run, as the last action on a low control path; stop-sensitive labels; capability separation, where a previewing program reads no state, so this channel does not arise, at the coarseness BM2:525-526 states.
  - **Missing facts:**
    - The verdict is not a function of (word, state).
    - Repeated runs with a public index accumulate bits.
    - Step and budget profiles are channels: a program that reads its remaining budget reads a state-derived value.
    - A refusal and its error category are preview outcomes; add them to item 19 and BM2:514.
    - A pick declassifier launders a one-item list, so it should declassify the choice, not the option set, unless the list is a fixed entry such as the repair list.
    - Labels hold only against sandboxed programs, and a missing provenance field reads unknown, never own.
  - **LX-3 must-refuse additions:** stop-selected preview, refusal-driven selection, one-item pick, accumulation across runs.
- needs_owner: yes. These facts bear on LQ2 and LQ21. I choose no option.

##### R3-M4

- id: R3-M4
- on: R2-E4; M6; INV-12; EV5, LB6, EV3; the Adopted rows on the principle record and the strip
- kind: support
- claim: The principle record as data, the fixture, the envelope reference and the checker trace stand, but a decomposition generated from the call graph gives structure, not reasons, so it is necessary and not sufficient for a person to understand a principle. Seven things must be added by hand.
- evidence:
  - **Counterexamples.** The floor's call graph is two set sizes and a subtraction; the reason is the invariance of In_c and Out_c and A_c ≤ min (`docs/progress/1.0/jumbling-observation.md:118`). `Span.solve`'s graph is generic elimination; the reason is that ν is a homomorphism, the corrections span, and G is the kernel (card 1). Neither reason appears in any call graph.
  - **Add by hand to each principle record:**
    1. A map from decomposition nodes to lemma steps.
    2. The invariant or reason, by statement ID.
    3. A dropped-hypothesis failure example, in the style of tex:1528-1551.
    4. What an empty or negative answer does not mean. INV-12:111 keeps "means"; M6:456's repair list is the example.
    5. A written hand derivation of the instance with its step count. A fixture proves numbers, not that a person can check them by hand (INV-12:113).
    6. Prerequisites: glossary terms and earlier records, which bar M6 needs.
    7. Specifications for opaque kernel leaves.
  - **Generate** the decomposition from the typed IR before lowering, and bind the record to the module digest. Higher-order calls over-approximate. A fork inherits no evidence, and its statement counts as unchanged only under an identical ID.
  - **EV5.** It can supply the means of understanding, but it cannot certify understanding (R3-M3, F6). Whether a check gates anything stays LQ14.
  - **Stand as adopted:** pointwise and whole-state contracts with their charging, one meaning of "twist", source-prefixed IDs (their form is preference), seconds as a display unit only.
  - **Strip:**
    - The relation "frame" is R_A/S_A, at label level.
    - "Frame" currently names four things: reference stickers (glossary:244-252), the nucoords frame group, restoration:173-174's C₃ component that no sticker shows, and A3's F_f. The disambiguation is correctness; the word chosen is preference.
    - "Current occupant" is partial.
    - "None: off the lattice" stays distinct from "unknown".
- needs_owner: yes. LQ14 stays the owner's.

#### B. Note N1: the label-lift lemma off the lattice

**Answer.** The lemma holds off the lattice, for every configuration, with a full proof (Theorem 1). The lattice is needed for one step only, deriving s_A from equal labels (Corollary 3). What does not extend is anything that reads a pose literally, above all reachability.

##### B.1 Objects

- A configuration X assigns to each piece A a pose X[A] ∈ SO(4). Twists multiply on the left. A's posed region is X[A](Ā₀).
- w(X) is the end state of a word w admissible from X. m_A(w; X) is A's left multiplier: w(X)[A] = m_A(w; X)·X[A]. (Preference: the contract's h_A(x) is the signed offset, so I write m_A; stage 19 writes pi for the piece permutation, which collides with A3's π, so I write σ.)
- R_A = {s ∈ SO(4): s(Ā₀) = Ā₀}.
- S_A = {s ∈ R_A: s maps each sticker region of A onto itself}. Equivalently, s fixes n_f for every f ∈ Host(A), because a sticker region is a 3-dimensional patch that spans the facet hyperplane of f.
- R_A ⊂ K⁺ (computed, B.3), so R_A permutes A's sticker regions. S_A is the kernel of R_A → Sym(Host(A)), so S_A is normal in R_A, and R_A/S_A is the frame group.
- KN1's Stab_A is S_A, as chapter 01:110 defines it. It must not be read as R_A: with R_A the class is not visible (39,530 labels differ, B.6 (d)).
- Classes are those of contract section 3 in real arithmetic (contract:30-36), not budgeted certificates.
- I write sticker class for KN1's visible pose class g_A·S_A. "Visible" cannot mean colours: 177,000 of the 177,120 pieces share their host set with another piece, so it must mean labelled. The name is preference; the definition is not.

**The gauge group.** 𝒢 is the set of pairs γ = (σ, t), with σ a permutation of the pieces and t_A ∈ SO(4) satisfying t_A(Ā₀) = (σA)‾₀.
- It acts on the right: (X·γ)[A] = X[σA]·t_A.
- The product is (σ,t)(σ′,t′) = (σσ′, A ↦ t_{σ′A}·t′_A).
- Subgroups: 𝒢_S (σ = id, t_A ∈ S_A) ≤ 𝒢_R (σ = id, t_A ∈ R_A) ≤ 𝒢.

##### B.2 Theorem and corollaries

**Theorem 1 (gauge transfer).** No lattice hypothesis; any exact alphabet. Let X be any configuration, γ = (σ, t) ∈ 𝒢, Z = X·γ, and (c, g) any exact twist.
- (a) A's posed region in Z equals σA's posed region in X.
- (b) A's class at c in Z equals σA's class at c in X. So the outcomes agree, and I_c(Z) = σ⁻¹·I_c(X).
- (c) If the twist is admissible, (c,g)Z = ((c,g)X)·γ with the same γ.
- (d) For every word w: w is admissible from X exactly when it is admissible from Z; w(Z) = w(X)·γ; and m_A(w; Z) = m_{σA}(w; X).

*Proof.*
- (a) Z[A](Ā₀) = X[σA](t_A(Ā₀)) = X[σA]((σA)‾₀).
- (b) Classes read only the posed region (contract:30-36, :179). The menu test reads only the letter.
- (c) If σA ∈ I_c(X), then ((c,g)Z)[A] = g·X[σA]·t_A = ((c,g)X)[σA]·t_A. If not, both sides equal X[σA]·t_A, since an admissible twist leaves the other pieces in place (contract:51). Left and right multiplication commute.
- (d) Induction on the prefixes. ∎

**Corollary 2.** For γ ∈ 𝒢_S the labelled posed sticker regions of X and Z agree at every later prefix, because X′[A]·t_A maps each sticker region of A where X′[A] does.

**Corollary 3 (kernel of π).** On lattice configurations, π(X) = π(Y) exactly when X[A] ∈ Y[A]·S_A for every piece A.
- Proof: equal labels put each sticker (A, f) in slot(k(A), k(f)) for both poses (contract:146), so s = Y[A]⁻¹X[A] fixes A's chamber and every f ∈ Host(A). Every piece has a sticker (B.3), so this reaches every piece. The converse is direct.
- "Labels" here exclude A3's orientation, which `lattice_stickers` returns separately and which differs at equal labels (B.6 (d)).

**Corollary 4 (goals; Lemma 13, tex:1370-1374).** If a goal set 𝒯 is invariant under 𝒢′ ≤ 𝒢, then for γ ∈ 𝒢′ the words from X that end in 𝒯 are exactly those from X·γ, so the distance in letters to 𝒯 is constant on 𝒢′-orbits.
- The sticker test is invariant under 𝒢_S. Lattice placement and low height are invariant under 𝒢.
- Solvability to the sticker test is therefore a sticker-class property. Menus are inverse-closed (contract:153), so a reachable state is solvable, and so is every state in its sticker class, although such a state may be unreachable.

**Relation to the record.** Contract A5's lemma is Theorem 1 at σ = id on the lattice. Proposition S (restoration:82-86) is its shape part. Stage 19's transfer is the same step off the lattice: "Z_p(w) = Y_pi(p)(w) t_p", specialising a previously reviewed lemma, with reachability not needed (`records/REGIONAL_PROFILE_DESCENT_THEOREM.md:16`, :61-67). New here are the statement for all of 𝒢 at once, the typing by levels and Lemma R. Stage 19's Γ(Y) is Y·𝒢.

##### B.3 Structure (computed, exhaustive over the 36 orbits)

| Retained orbit | Size | Stickers | \|R_A\| | \|S_A\| | Vertices |
| --- | --- | --- | --- | --- | --- |
| −1 (centres) | 600 | 1 | 12 | 12 | 8 |
| 0 | 1,200 | 2 | 6 | 3 | 12 |
| 1 | 3,600 | 1 | 2 | 2 | 10 |
| 3 | 3,600 | 2 | 2 | 1 | 10 |
| 6 | 720 | 5 | 10 | 1 | 24 |
| 2, 15, 29 | 2,400 | 1 | 3 | 3 | 9 |
| 33 | 2,400 | 1 | 3 | 3 | 5 |
| 8, 11, 22, 25 | 3,600 | 2 | 2 | 1 | 7 |
| 32 | 3,600 | 2 | 2 | 1 | 6 |
| 17, 28 | 1,440 | 5 | 5 | 1 | 13 |
| 34 | 120 | 20 | 60 | 1 | 14 |
| 19 others | 7,200 | 1 or 2 | 1 | 1 | 5 to 7 |

- Every piece has a sticker (histogram: 1: 107,400; 2: 66,000; 5: 3,600; 20: 120).
- In all 36 orbits the SO(4) stabiliser equals R_A, and R_A ⊂ K⁺.
- R_A/S_A has order 2 for orbits 0, 3, 8, 11, 22, 25, 32; 5 for 17, 28; 10 for 6; 60 for 34; 1 otherwise. This equals nucoords.py's HG (line 9).
- 15,000 pieces have S_A ≠ 1, so the lattice fibre of π has 12^600 · 3^1200 · 2^3600 · 3^9600 pose tables.

**Lemma R (rigidity, computed).** If t ∈ SO(4) and t(Ā₀) = B̄₀, then t ∈ K⁺ and B lies in A's K⁺ orbit.
- Evidence: only four orbit pairs share a vertex Gram multiset (retained 4/5, 12/13, 18/19, 26/27). Each has one matching map, with determinant −1, so they are mirror images.
- Consequences:
  - A posed region is a retained chamber closure exactly when the pose lies in K⁺, which is the converse of contract:26.
  - Each posed region fixes its coset gK⁺, so domains and N are functions of the shape alone. This sharpens restoration:86.
  - 𝒢 ≅ ∏ over the 36 orbits of R_O ≀ Sym(O).
- Theorem 1 and Corollaries 2 to 4 do not use Lemma R. The 𝒢-orbit of X is the set of configurations with the same occupied regions, by the no-collision rule alone (contract:52).

##### B.4 Levels

A function has a level when it is constant on that level's classes. Piece-indexed functions transform by σ at shape level. The levels form a partial order, not a chain: pose → sticker class → region class → {coset → tree data → heights; shape}.

| Level | Object | Equal when | Forgets | Defined |
| --- | --- | --- | --- | --- |
| Pose | g_A | X = Y | nothing | always |
| Sticker class | g_A·S_A | Y = X·γ, γ ∈ 𝒢_S | turns in place | always |
| Labels | the slots of A's stickers | as the sticker class (Cor. 3) | | lattice only |
| Region class | g_A·R_A, that is the posed region | Y = X·γ, γ ∈ 𝒢_R | frames R_A/S_A | always |
| Position | the chamber | as the region class | | lattice only |
| Coset | g_A·K⁺ | quotient, not a gauge | place within the domain | always |
| Tree data, heights | functions of the coset | quotient, not a gauge | | always; they determine the coset and the lattice test only on Γ(Λ) |
| Shape | the set of posed regions | Y = X·γ, γ ∈ 𝒢 | which piece fills which region | always |

Only 𝒢_S, 𝒢_R and 𝒢 are gauges. Equal cosets or equal heights give no transfer by Theorem 1, because two such configurations can have different shapes.

| Reads at this level | Functions |
| --- | --- |
| Pose only | The pose table; the pose digest (state.py:473-486); `moved_count` (state.py:162); the orientation index; the journal; `v`; reachability; label and retained checkpoints through their reachability clause; budgeted `uncertain`; pose groups |
| Sticker class | Labelled posed stickers; on the lattice, labels, ν, the sticker test and the label digest |
| Region class | Each piece's posed region, lattice flag, domain, tree positions and heights; inside sets and straddlers as sets of pieces; left multipliers |
| Shape | Occupied regions; admissibility of every word; N, N_c, In_c, Out_c, F_c; Φ, q_t, H; the off count; multisets of heights by type; anything defined only from these, such as the repair list |

##### B.5 Protection and reachability

**Protection (refines M3:364-367).** The pose-level verdict m_A(w) = 1 and the label-level verdict X[A]⁻¹·m_A(w)·X[A] ∈ S_A are both invariant under 𝒢_R; the second uses that S_A is normal in R_A. So M3's "exception" means two different predicates, not non-invariance. M3's rule stands: a verdict carries its level and never crosses over. A comparison with a stored literal pose is pose-level only.

**Reachability does not transfer.** Let Y be solved and let X be Y with only centre 0 turned by a third-turn of A4₀, so X and Y are in one sticker class.
- v(X) is nonzero at the centres and zero at O02.
- The 1,200 generators give v(G) = {0, V, 2V}, where V is nonzero at O02 (restoration:173-174; `research/jumbling/theory/results/lens-invariants.json`).
- So v(X) ∉ v(G), and X is unreachable in J(A4₀), yet X passes the sticker test.
- Reachability of such a state in J(I_a) is open.

##### B.6 Computations

Command form, from the repository root: `tools/.venv/engine/Scripts/python.exe -B <script>`, with `research/jumbling`, `research/jumbling/theory` and `research/jumbling/fixtures` on `sys.path`. The script is not in the repository; its core is:

```python
import numpy as np, sim, wj, rim          # sim: research/jumbling; rim: theory; wj: fixtures
c = sim.get_context()                     # reads assets/ only; no session
d, k, g = c.data, c.kplus, c.regions
P = k.perms.astype(np.int64)
for o, p in enumerate(g.reps):            # one representative per K+ orbit
    R = np.flatnonzero(g.piece_of[o] == p)    # R_A as indices into K+
    h = np.array(d.hosts(p))
    S = R[(P[R][:, h] == h).all(1)]           # S_A: also fixes every host facet
M = wj.make_menu(c, 'I_a')
# right factor of piece A: s_A = t s t^-1 with t = g.transport[A], s random in S or R of its orbit
# X[A] = Y[A] s_A; per step compare at random grips the exact class of every piece
# (sim.kernel: Evaluator(pullback_form(pose, d.N[e])).classify(g.vform(A))), N (rim.Domains(st).total_misalignment())
# and, after one twist applied to both, status, moved count and X[A] == Y[A] s_A exactly.
```

SO(4) stabilisers come from backtracking over the Gram matrix of the exact region vertices read as floats, tolerance 1e-9. A true symmetry cannot be missed, and everything found was an exact element of K⁺.

Walks are samples: they test that the simulator matches the theorem and prove nothing themselves.

- **(c) Right-factor walks.** I_a, seed 7, five jumbles then four twists, six random grips per step, every piece given a random S or R factor (factors on off-lattice pieces capped at 1,500; nontrivial factors 8,853 for S, 21,945 for R).
  - 0 class differences in both walks.
  - N 7830, 7830, 9190, 10550 on both sides; domains 6, 6, 7, 7 with the same partition.
  - Same outcomes; pieces moved by each twist 3097, 2808, 2702, 3097.
  - Off-lattice counts 15,485, 18,293, 20,995, 20,995.
  - X[A] = Y[A]·s_A exact at each step; digests differ.
- **(d) Boundary cases:**
  - Solved with S factors: labels equal; the orientation index differs at 10,311 slots.
  - Solved with R factors: 39,530 labels differ, `moved_count` 23,595, frame agreement true.
  - Lens word (0,12)(24,a)(0,12⁻¹), 2,808 pieces moved, from the S variant: admissible, ends on the lattice, labels equal, pose digests differ.
  - Six-jumble state (seed 5) with S factors: all 600 grips have the same status (142 blocked, 458 admissible).
  - Budget: at grip 0 with budget 240, Y is admissible and X is `uncertain`, because X needs 2,760 exact evaluations.
  - The identity letter at blocked grip 2 reads `blocked`.
  - Merge: (c,g₁), (c,g₂) move 3,097 pieces each, the merged letter is admissible, moves 3,097, and gives the same digest.
- **(e) σ ≠ id and free mode:**
  - 3,600 pieces swapped in pairs, 50 transpositions per K⁺ orbit, t_p ∈ K⁺, Y off the lattice. Over five steps of 10 grips: 0 differences between class(Z, e) and class(Y, e)[σ]; N equal (7624, 9190, 10756, 10962, 12344); Z[p] = Y[σp]·t_p exact; digests differ.
  - Tree data equal for 40 of 40 sampled off-lattice pieces under R factors.
  - Free-mode Cayley rotations at cap 0:

  | ω | In K⁺ | Angle | Heights | Note |
  | --- | --- | --- | --- | --- |
  | (2,0,0) | no | 126.87° | (0,0) | cos −3/5, denominator 5 |
  | (1,1,0) | no | 109.47° | (0,0) | cos −1/3, denominator 3 |
  | (1,0,0) | no | 90° | (1,1) | |
  | (3,0,0) | no | 143.13° | (1,1) | |
  | (1,1,1) | yes | 120° | (0,0) | |

  So "heights 0 exactly when on the lattice" is false outside Γ(Λ): an odd denominator takes the pose off the lattice at an odd prime, and the tree at 2 does not see that.

##### B.7 What the full text may and may not rely on

**May rely on:**
- Theorem 1 and Corollaries 2 to 4, on and off the lattice, for exact classes. This lifts overview section 8's first restriction and chapter 01:116's "relies on nothing".
- The named classes g_A·S_A and g_A·R_A.
- The table of B.3 and Lemma R, as computed results.
- The level table as a partial order.
- Exact transport: a level-ℓ claim moves along that level's subgroup with no re-run, and the reused leaf is AND(source leaf, Theorem 1, the checked relation Z = X·γ).
- Correction to M3:375: "only legality claims transfer" is too strict. Legality and shape-level claims (q, n, Φ, H) transfer.
- Status under N2's own grades: (theorem, producer) until a model review reads this note. The transfer step itself is the reviewed stage 19 lemma. No person has reviewed either (owner, 3 November: none required). The property test stays as an engine conformance test.

**May not rely on:**
- Labels or π off the lattice; say labelled posed stickers.
- Stab_A = R_A as "visible".
- Transfer of reachability, or of label and retained checkpoints.
- Equality of digest, journal, `moved_count` or orientation.
- Budgeted `uncertain`.
- "Different class means a later twist separates." This is false: Z = Y·γ with γ ∉ 𝒢_S has the same word language.
- Shape as the coarsest congruence, which is open and not needed.
- Tree tests outside Γ(Λ).
- A class as equality: contract:25 and A5:180, :182 and :183 stand, with no symmetry quotient taken.
- A gauge at the coset, tree or height rows.

**Off-lattice digests** need a level. An optional sticker-class digest is the least exact key in g_A·S_A, at most 12 products per piece; until one exists, pose is the only stored level.

**Open:**
- Whether the same word language implies the same shape.
- Reachability of sticker classes in J(I_a).
- An exact certificate for Lemma R.
- A bridge lemma with its own ID, if the text wants to weaken reach(Z) to "some member of Z's sticker class is reachable" for sticker-level goals.

#### C. Note N2: the order on evidence

##### C.1 The order

Grades are P ⊂ Basis × Check, at most 30 values, with the product order: (b,k) ≤ (b′,k′) exactly when b ≤ b′ and k ≤ k′.
- Basis: sample lies below theorem, certificate, exhaustive and exact, which are pairwise incomparable.
- Check: none < producer < independent checker, model review, human review and formal proof, which are pairwise incomparable.

**Principle.** x ≤ y only when a trust policy that accepts x and rejects y would be incoherent. Everything else stays incomparable: a wrong comparability hides a leaf behind another, while a missing one only shows one leaf more.

**Reasons:**
- A later check runs on a result the producer accepted, so what passes it has passed the producer, and "none" detects nothing.
- A sample does not entail its statement even when everything recorded is right.
- The product is right and an ordinal sum is not: (sample, independent checker) against (theorem, none) can be coherently accepted and rejected either way.

All proofs below hold for any finite poset, so the table is versioned. Invalid pairs are a versioned table and read as typed inconsistencies.

##### C.2 Check mode stays out

The partition's "modes are facts" stands. A reused leaf is AND(source leaf, transport lemma leaf). A fresh audit and an actual replay differ in what they cover, so they are different statements or a recorded fact. So R2-E1's 90 becomes 30.

##### C.3 Scenarios

A scenario is ω = (D, T): D is a down-set of distrusted grades, T a set of assumed premises. Ω is ordered by less distrust and more assumptions. val is the least solution of:
- val(leaf q) = {ω: q ∉ D}
- val(premise p) = {ω: p ∈ T} ∪ val(C), with C the claim with the identical key, if one exists
- AND = ∩ (the empty AND is Ω)
- OR = ∪ (the empty OR is ∅)

Each statement key has one OR node. The system is monotone on a finite lattice, so the least solution exists (Knaster-Tarski) and is reached by iteration from ∅ (Kleene). Every val is an up-set of Ω.

##### C.4 Summaries

An alternative is a = (L, Π), with L a set of grades and Π a set of premises. Sat(a) = {(D,T): L ∩ D = ∅, Π ⊆ T}. Every val is a finite union of such sets. a dominates b when Sat(b) ⊆ Sat(a).

**Theorem A (the weakest governs).** Sat(L, Π) = Sat(min L, Π).
*Proof.* If y ∈ L ∩ D, a minimal x ≤ y in L lies in D, because D is a down-set. ∎

**Theorem B.** Sat(L₂,Π₂) ⊆ Sat(L₁,Π₁) exactly when Π₁ ⊆ Π₂ and L₁ ⊆ ↑L₂.
*Proof.* (⇐) Let (D,T) ∈ Sat(L₂,Π₂). Then Π₁ ⊆ T. If x ∈ L₁ ∩ D, some y ∈ L₂ with y ≤ x lies in D, a contradiction. (⇒) Take ω = (P∖↑L₂, Π₂), which lies in Sat(L₂,Π₂). ∎

**Theorem C (canonical summary).** Each val(v) has exactly one set 𝒮(v) of alternatives whose L are antichains and which are pairwise non-dominating.
- AND: take pairwise (min(L₁∪L₂), Π₁∪Π₂), then reduce.
- OR: take the union, then reduce.

*Proof.* Existence: use Theorem A and delete dominated alternatives. Uniqueness: let 𝒜 and ℬ be two such sets. For a ∈ 𝒜, ω_a = (P∖↑L_a, Π_a) lies in some Sat(b), which by Theorem B gives Sat(a) ⊆ Sat(b). Symmetrically Sat(b) ⊆ Sat(a′). Non-domination forces a = a′, so Sat(a) = Sat(b), and antichains then give a = b. ∎

So:
- The summary does not depend on evaluation order, because ∩ and ∪ are associative, commutative and idempotent.
- It is defined on a DAG, because sharing is harmless.
- It is monotone: raising a grade or adding an OR child enlarges val.

"The set of weakest leaves" is right at an AND node. At an OR node the summary is a set of alternatives, not one set of leaves.

##### C.5 Cycles and unresolved premises

The least solution is the union of well-founded derivations, so a cycle gives its members only what they have from outside.
- A pure child cycle has val ∅ and reads open with a diagnostic.
- A circular discharge stays conditional on its premises and is never proved.
- An unresolved premise is a free atom. With the OR form, an open claim C leaves S conditional on p; it does not drag S to open.

##### C.6 Independent arguments

Replace "combine by the strongest" with "keep every undominated argument". Nothing adds up, and if one argument dominates, it alone stays. When two arguments are incomparable both stay: two samples stay below every threshold, and no number of them makes a proof.

"Independent" is a displayed fact: two arguments share no leaf identity and no tool digest apart from the declared common base (kernels, model files). It is not needed for soundness. A leaf present in every alternative stays a single point of failure, since val ⊆ val(leaf). The graph keeps leaf identities, grades are a projection of them, and common failure points are shown.

##### C.7 Threshold and standing

The threshold is a versioned up-set 𝒰 ⊆ 𝒰₀ = {(b,k): b ≠ sample, k ≠ none}. 𝒰₀ is the weakest threshold that keeps 8.1; raising it belongs to LQ3, and I do not choose.

- ω₀ = (P∖𝒰, ∅), and stands₀(v) means ω₀ ∈ val(v).
- Ref = {p: stands₀(neg p)}.
- ω₁ = (P∖𝒰, Prem∖Ref), and stands₁(v) means ω₁ ∈ val(v).

stands₀ does not read Ref, so there is no circularity through negation. Both are one pass of unit propagation, and stands₀ implies stands₁. The exact alternative form can be exponential and is computed lazily for display.

A failed check removes its leaf from val and refutes nothing; the claim reads as it would without that support, as the error registry's `evidence` row says.

##### C.8 The word function

A decision list, first match wins, so it is total and single-valued.

| Row | Condition | Word | Claim field |
| --- | --- | --- | --- |
| 0 | Any inconsistency I1 to I6 in Reach(S) | typed inconsistency | none |
| 1 | stands₀(neg S) | refuted | refuted |
| 2 | stands₀(S), covered set one finite instance or a declared search set | finite instance | proved |
| 3 | stands₀(S), covered set an admitted domain or every reachable state | proved, with a mandatory scope phrase ("proved on D[k]") | proved |
| 4 | stands₁(S) | conditional, premises named | conditional |
| 5 | A search record for S or neg S with complete = false and stop reason a deterministic budget or a declared limit | not found within budget | open |
| 6 | A job for S or neg S ended cancelled, error, wall-clock or resource stop, or cut off on a non-search job | incomplete | open |
| 7 | Otherwise | open (recorded-only claims keep their source label) | open |

- Default 6's three results are distinct: "empty" is a completed exhaustive negative (row 2), "not found" is row 5, and "not run" is no record or R3-E1's import value (row 7).
- A search record's target is the existence statement, shared by the pair {E, neg E}.
- 4.1's reachability values map as: witnessed to finite instance; by theorem without a witness to finite instance with basis theorem shown; promised to conditional; refuted to refuted; open to open.
- A product ceiling is not a declared limit, so it reads row 6, matching the registry's `budget` and `resource_limit` boundary.

R2-E1's combination test is finite at node level: three booleans with stands₀ ⇒ stands₁ (6 cases), covered-set kind (4), search record (none, complete, budget or limit: 3), job outcome (none, the four of KN7, not run, not stated: 7). That is 504 cases, plus property tests of the passes on small graphs.

##### C.9 Inconsistencies

These read `EVD:evidence.inconsistent_fields`, and I5 reads `EVD:evidence.negative_without_completion`.
- I1: a value outside its versioned enumeration, or a pair the table lists invalid.
- I2: a discharge edge between different keys, a negation pair that is not mutual, or a leaf whose identity fields differ from its statement's scope.
- I3: a scope digest coarser than the statement's subject level.
- I4: stands₀(S) and stands₀(neg S).
- I5: a negative or exhaustive leaf whose search record says complete = false, or complete = true with job outcome cancelled, cut off, error or not run.
- I6: a refuting leaf on a universal statement with no scope-membership fact.

Job outcome "not stated" (R3-E1) is no contradiction. It makes a negative leaf inert, which blocks the negative claim, and the word stays row 7 with the source label. A witness from such a job counts once checked.

##### C.10 Non-upgrade

Key = statement ID plus scope. Arguments are immutable, so a new record enters as a new node or a new OR child. Reach(S) is the smallest node set containing S and neg S, closed under children, discharge edges and negation pairs of premises.

**Theorem D.**
- (a) Locality. word(S) reads only facts on Reach(S) and the versioned tables, so a record that adds no edge into Reach(S) changes no word.
- (b) For any 𝒰 ⊆ 𝒰₀, a sample leaf or a job outcome never moves S from rows 5 to 7 into rows 1 to 4. *Proof.* A sample leaf's grade is outside 𝒰, so it is false at ω₀ and ω₁. As a new OR child it changes neither pass, nor Ref. A job outcome is not a leaf. Rows 1 to 4 read only the passes. ∎
- (c1) Scoped negative. Its covered set is a declared search set, and scope is part of the key and immutable, so its word is at most "finite instance". The full-family statement has a different key, and the only edge to it is a bridge lemma with its own ID (stage 16 PROOFS.md:53-79; LB5). An upgrade happens only by a proof.
- (c2) Reachability promise. T(Z) = AND(theorem on D[k], admission(Z), reach(Z)). val(reach(Z)) at T = ∅ is nonempty only through a claim with the identical key: a journal from solved, a checked witness or a theorem. So T(Z) stays conditional until that claim exists.
- (c3) `OUTSIDE_CERTIFIED_DOMAIN(k)` is a leaf for ¬Adm_k(Z), whose word is finite instance, with no edge to reachability or solvability (stage 19:152), so their words do not change. T(Z) falls to open, not refuted. A different k is a different key.

##### C.11 Worked examples

**E1, stage 15 macro certificate R01.** S₁ is "the word `104:8,46:4,104:15` from `b47d8474…` ends at `1db0dd54…` with Φ 1376, 1376, 1376, 1350 and excursion 0" (stage 15:104); finite instance, pose level. S₁ is the OR of:
- a₁ = {(certificate, producer) `results/certificates/R01.json`, (theorem, model review) Theorem MC}
- a₂ = {(exact, producer)} `macro.py check --replay`, `R01-check.json`: same script file as `cert`, so it counts as an independent checker only if the tool-digest fact says so
- a₃ = {(exact, independent checker)} `replay_word.py` (under rule V14 the import may record it differently; the word below does not change)

a₃ dominates a₂ by Theorem B. a₁ and a₃ are incomparable, so "strongest" is undefined and both stay. Word: finite instance. Under a raised threshold that requires a later check, a₁ falls and a₃ carries the same word. All three share the exact geometry and model files, the declared common base.

**E2, stage 19.**
- Fixed-L implication: (theorem, model review), `records/type-height-weight-proof.md` with its review. Word: proved.
- The bound "cost ≤ L·R_L(Z₀)" is AND(implication, premise Sel). Word: conditional. Sel is "every admitted nonterminal state has a legal selected macro …", a mixed statement (stage 19:100-102); "Neither theorem proves that the needed macro exists at every reachable state" (:112).
- A macro found at one state discharges nothing, and a failed budgeted search at one state does not refute Sel.
- U(h,k) on D[k]: (theorem, model review), `records/continuous-framework-independent-final-review.md:5`. Word: "proved on D[k]", with the reachability promise inside the statement.
- Applied to a concrete Z it reads conditional on reach(Z), and the synthetic state stays conditional (stage 19:88-92).
- For a rejected Z, ¬Adm_k(Z) reads finite instance and T(Z) reads open.

**E3, a cut-off search, the 7302 stop at Φ 1094.** E₃ is "some family-R word with at most three retained letters lowers Φ here", and S₄ is neg E₃.
- Declared run: job completed, beam 64 truncated at three letters, complete = false (stage 15:212-214). Word for both: not found within budget.
- The two-letter subset was exhaustive. It has its own key and reads finite instance.
- The unbeamed search found `535:1,363:5,535:1,597:14` (Φ 688; 228 descending words; 545 CPU s; stage 15:223). E₃ reads finite instance, and S₄ reads refuted.
- The same search at the 7303 stop found none, 0 words: a scoped negative, finite instance, for three letters of family R only.
- The runs at 7305 and 7306 stopped by the integrator at 922 and 699 CPU s are cancelled jobs: incomplete, and their continuation has its own record (stage 15:198-201).
- The two-level stop at Φ 498 (`661010e5…`) is a negative for the five declared families only, and "no word descends there" stays open (stage 15:239-241, :395-404).

##### C.12 Corrections to 8.1

1. Line 254: keep every undominated argument (C.6).
2. Line 267 and INV-5:78: "never upgrades a universal claim". Counterexample: a checked witness found by sampling proves an existence statement, as line 266 agrees; its leaf's basis is exact or certificate, not sample.
3. Line 263: a negative can also be proved by a theorem (the floor lemma; the all-word exclusions at `records/CONTINUOUS_FRAMEWORK_CANDIDATE.md:54`).
4. Line 261: a counterexample needs checked scope membership and identical identity fields (stage 19:88-92). A mixed statement is refuted only by a witness x plus a complete negative at x.
5. A conditional refutation is a conditional neg S. S itself reads open, and no extra Claim value is needed.
6. Line 266: job completed is not search complete, so store completeness or closure separately (the beam).
7. Line 253: "weakest leaves" holds per argument; a statement's summary is a set of alternatives.
8. Line 244, section 13 line 665 and PS1.35: every scope field immutable, no field revision-bound context.

Line 264's "reads open" stands as the Claim value; the display word is "not found within budget".

#### D. Explainers

**P-M6. How supports are compared, and why the weakest governs.** Most results rest on several supports at once. Each support carries two stamps: what it is based on (a proof, a certificate, a complete enumeration, an exact calculation or a sample) and who confirmed it (nobody, the program that made it, a separate checker, an AI review, a person, or a formal proof). Only a few stamps can be compared: a support nobody confirmed is below the same support confirmed by its producer, that is below any later check, and a sample is below anything that actually implies the result. Most pairs cannot be compared, because a certificate passed by a checker and a proof read by an AI model fail in different ways, so the system keeps both and never forces them onto one scale. When a result needs two supports together, it is only as safe as the weaker one, because whoever distrusts the stronger must also distrust the weaker; so only the weakest supports need showing. When a result has two separate arguments, it stands if either stands, so both stay listed unless one is plainly better, and they never add up: two samples are still no proof. You set the bar a support must clear before a word such as "proved" appears. Trade-off: a single scale would read faster, but it would claim comparisons nobody can justify.

**P-M7. What the label-lift result means for a solver.** Two puzzle states can differ only by pieces turned in place: every sticker sits where it sat, but the exact pose of some pieces differs. It is now proved that such states behave identically for ever, in the middle of a jumble as well as when every piece is in a retained place: the same twists are legal, the same pieces move, and the stickers stay the same after every twist. So a hidden turn can never block you later, never make a solve longer and never make the puzzle unsolvable by the sticker test. Tools may therefore reuse a check between such states without running it again. Three things do not carry over: the exact record of poses (the digest and the journal), protection that reads poses, and the question whether that exact state can be reached from solved. There is also a warning in the other direction: two states with different stickers can have exactly the same legal twists, so equal legality never means the same puzzle. The proof is short and uses only the rule that a twist turns whatever lies wholly inside a cap; the same step was already reviewed by AI in stage 19, no person has reviewed it, and the new typing by levels still needs a second reader.

**P-M8. Levels of identity (ADR 4).** The language must know what a value talks about, so each value names its level. The finest level is the pose: exactly how a piece sits in 4D. Next comes the sticker class: what the labelled stickers show, which ignores turns in place. Then comes the region class, only the space a piece fills, which also ignores how its stickers are swapped within that space, and finally the shape, the set of filled spaces, without saying which piece fills which. Labels and slot positions are the sticker class and the region class read when every piece is in a retained place; otherwise they do not exist, so the program says "none: off the lattice", which is different from "unknown". The levels branch rather than form one ladder: a piece's height (how far its pose is from a retained one) keeps the piece's name but forgets its place, while the shape keeps places but forgets names. A claim may be carried to another state only along a level it does not depend on: "this twist is legal" carries across the whole shape level, "the stickers are home" across turns in place only, and "this state is reachable" across nothing. Trade-off: every value and claim carries a level, which is more to read and write, and in return a result cannot silently be applied where it is false.

**P-M9. Scope, premises and how a premise is cleared (ADR 5).** Every claim says what it is about (its scope) and what it assumes (its premises). A premise is an open debt: the result reads "conditional" and names it. It is cleared only by a claim with exactly the same identity, meaning the same statement, the same puzzle, the same menu and the same state. If two statements merely mean the same thing, the system refuses the link until someone adds a small lemma connecting them; that can leave a true result unlinked, but it never links a false one. The main example is the reachability promise: the restoration theorems of stage 19 apply only to states that can be reached from solved, so a bound for a given state stays conditional until a journal or a checked word shows that state is reachable. Likewise "no macro in this family" is a result about that family only, and "outside the certified domain" says only that the finite entrance test failed, not that the state is unreachable or unsolvable. A theorem about a whole set reaches one state only through an explicit step that checks the state belongs to the set. Trade-off: more "conditional" and "open" on screen, against no result that quietly claims more than was shown.

**P-M10. How the display word is computed, and why refutation is one-sided.** The word on screen comes from a fixed list of questions, and the first yes decides, so every result gets exactly one word. An inconsistent record shows an error instead of a word. If the opposite statement is established, the word is "refuted". If the statement is established at your bar, the word is "finite instance" for one case or a stated search, and "proved" for a whole domain, whose name is always shown. If it would stand once its named assumptions hold, the word is "conditional". Otherwise it is open, and the word says why: "not found within budget" for a search stopped by its preset limit, "incomplete" for a job cancelled, failed or stopped by the clock, or plain "open". Refutation is one-sided because one checked counterexample inside the claim's scope ends "always", while failing to find something ends nothing: in stage 15 a finished search with a narrowed beam found no macro, and the full search then found 228. Trade-off: you see more "open" and "not found" than a looser rule would show, and none of them is a disguised "no".

**Corrected P-M1 to P-M5.** The engineer advisor's corrections 1 to 5 are applied, plus two updates from this round. P-M1 had no listed correction; I added one sentence for INV-4's third float site. P-M2 and P-M5 also carry updates from N1 and R3-M3.

**P-M1. Exact numbers for decisions.** Computers usually store numbers approximately ("floating point"), with tiny rounding errors: harmless for drawing, risky for deciding. In the solved puzzle, pieces touch the cuts (the flat boundaries along which caps turn) at distance zero, so a rounding error could make a legal twist look blocked. The jumbling contract's tests include a piece crossing a cut by five ten-billionths of the distance from the centre to a cell, which must count as blocked. So every decision uses exact numbers: whole numbers, fractions and numbers a + b√5, which the geometry needs. A fast approximate test may answer first only where a proved limit on its error shows it cannot be wrong. One place still starts from approximate numbers: a rotation requested by angle is turned into an exact rotation, the exact result is recorded and replay uses it, and a spike (LX-9) checks that this choice is the same on every computer. Trade-off: none for you; the charter binds this.

**P-M2. Poses, stickers and what you can see.** A pose is how a piece sits in 4D; a slot is a fixed place where one sticker sits. Some pieces, like cell centres, can turn in place: the pose changes, every sticker stays in its slot, so nothing shows. 600-cell-Full records only stickers, so it cannot see such a turn at all; poses exist only in the jumbling puzzle, where Jumble off stores the exit poses and Jumble on replays later moves from them. A proof in the jumbling contract, so far reviewed only by AI, shows no later twist can tell such poses apart; that proof covered states with every piece in a retained place, and note N1 of this round extends it to every state, mid-jumble included (the extension has no second reader yet). So a solve ends on the sticker test (all stickers home), as you chose on 22 October. I propose the language checks whether each value describes poses or stickers. One catch: protection inside Jumble reads poses, so it can object to a turn no sticker shows. Trade-off, later: mark such objections, or let protection read stickers, a contract change.

**P-M3. How sure is a result?** Your five words (proved, conditional, finite instance, not found within budget, incomplete) mix three questions. How true is it: proved, true if named assumptions hold (conditional), disproved, or open? What is it about: one finite case, everything a stated search covered, or every state the puzzle can reach? How did the calculation end: finished, cancelled, stopped at its budget (a preset limit on work), or with an error? A cancellation, a budget stop or an error never yields a negative answer. On one scale a search stopped by its budget would be both "not found within budget" and "incomplete". I propose storing the three answers separately and computing exactly one of your five words, or refuted, or open, from them for display. Trade-off: one simple scale is easier to read but can mislabel results; separate answers are exact but need a careful display.

**P-M4. Certificates and independent checkers.** A certificate is a record that lets anyone confirm a result without trusting how it was found. To show a twist is blocked, it gives two exact points of one piece, one on each side of the cap's boundary. To show what a macro does, it gives every twist and a digest, a short fingerprint, of each state. A checker is a separate small program that recomputes every claim from the certificate, twist by twist; it shares no search or construction code with the program that found the result, but both use the same exact simulator and model files, which are tested separately. It is tested with damaged certificates, which it must reject. Reviews by an AI model or a person are recorded as other kinds of check. Trade-off: extra work and storage, against trusting the finder.

**P-M5. What the boundary can and cannot enforce.** A read-only program can search: it tries twists on copies and reports the best. For a language without limits, no checker can tell, for every program, whether its output is a solving move; a classic theorem (Rice's) rules that out. Budgets make each run finite, so one run can be decided by running it, but checking a program on every state stays out of reach, and the deeper limit remains: the same word at the same state can be your own choice or a program's, so no test on the word and the state can tell them apart. Even a plain formula can answer a search: when a cap's mismatch count equals its floor, a simple formula, no twist of that cap alone can lower the count, and nothing was tried. Software can enforce what ships, one deliberate action (a key press or a click) for every commit (a held key never repeats a commit; program views and the camera need no such action), and which abilities each mode (solving, writing tools, research) grants. It can also record where each previewed move came from, indirect choices included, but only in a language that tracks data and branches: static labels need a checker over exactly the code that runs, which an own language has and a borrowed runtime can add over an approved subset, and labels at run time need an own interpreter, an own compiled front end or a modified runtime. Even then a program can signal by when it stops, which needs a stricter rule; a word retyped or carried in a file from elsewhere loses the record, and your understanding stays invisible. Trade-off: stricter rules forbid harmless tools; looser rules rely on honest labels.

### Outcome of round 3

Settled by the main session on 4 November 2026, in the order of the [round protocol](../README.md): the filter, then
demonstrated failures, then the 8B order among admissible options, then experiments for unmeasured objections. Nothing
here is an owner decision until the owner answers the questions at the end. In this section N1, N2 and the section
letters are those of the mathematician's record above.

#### The two open obligations

Both are cleared.

- **The label-lift lemma off the lattice (M3).** N1 proves it for every configuration and any exact alphabet (Theorem 1,
  Corollaries 2 to 4). The lattice is needed for one step only: deriving a turn in place from equal labels. The full
  text relies on what B.7 lists and on nothing B.7 rules out. Under N2's own grades the note's status is (theorem,
  producer) until a model review reads it. The transfer step itself is the reviewed stage 19 lemma. No person has
  reviewed either. CT-KN2-06 stays as an engine conformance test.
- **The mathematician's answer to R2-E1 to R2-E4.** R3-M1 to R3-M4 give it. What the revised partition adopted from
  those cards stands, except for the corrections below.

#### Out by the filter

- A pick that clears the labels of the list it was made from. A program could build a one-item list and turn one key
  press into its own choice, which the 2 October boundary forbids. The record's fix enters chapter 07 as a rule for the
  joint setting: a pick declassifies the solver's choice, and the option set keeps its labels unless the list is a fixed
  entry such as the repair list.
- A reachability claim moved along a gauge, and a label or retained checkpoint moved along one. Neither transfers
  (B.5).

#### Demonstrated failures, adopted

| Failure or gap | Card | Applied in |
| --- | --- | --- |
| "One bit per run" is false: a state-dependent stop selects among up to 36,000 pending previews, about 15.1 bits | R3-M3 | Partition section 11, BM2, chapter 07, chapter 11's LX-3 corpus |
| Rice's theorem does not carry the argument once budgets exist; the verdict is not a function of (word, state) | R3-M3 | Partition section 11, chapter 07 |
| A refusal and its error category are preview outcomes; a read of the remaining budget is a state-derived read; a missing provenance field reads unknown, never own | R3-M3 | BM2, chapter 07, chapter 09 |
| "Combine by the strongest" is undefined on incomparable arguments; keep every undominated argument | N2, C.6 and C.12 | Partition 8.1, chapter 06 |
| Facts lack seven fields: statement form, negation pair, subject level, scope-membership evidence, search completeness, premise class, tool digest of each check | R3-M1 | Partition 8.1, chapter 05's envelope, chapter 06 |
| "Immutable input digest" would exclude every theorem; every scope field is immutable and none is revision-bound context | R3-M1, C.12 | Partition 8.1 and section 13, PS1.35, chapter 06 |
| The eight corrections of C.12, among them a checked witness found by sampling proves an existence statement, and a job completed is not a search complete | N2, C.12 | Partition 8.1, INV-5, chapter 06 |
| Default 10's key property needs four hypotheses | R3-M2 | Partition default 10, LG2.13, LG2.14 |
| A per-vertex test can refute a region property and cannot establish one | R3-M2 | Partition LB4, LB4.2, RPI.30, CT-LB4-01 |
| "Height zero" means the lattice only on Γ(Λ) | R3-M2 | Chapters 01 and 02, with a free-mode height test |
| The kernel of labels to position is the frame group R_A/S_A | R3-M2 | KN1.7 |
| The levels form a partial order with region class, coset, tree data, heights and shape | N1, B.4 | Partition KN table, KN1.7 to KN1.11 |
| Two protection predicates, each with its level | N1, B.5 | KN1.11 |
| A generated decomposition gives structure, not reasons; seven parts are added by hand | R3-M4 | `PrincipleRecord` (LB6.1, LB6.2), EV5.3, EV5.4 |
| "Frame" names four things, and "current occupant" is partial | R3-M4 | EV3.8, the strip's clauses |
| The corpus is read from Git blobs, never from a checkout; the counts at partition lines 279 and 662 | R3-E2 | Partition, default 13 |
| The largest word figure | R3-E4 | Chapter 01, the partition |

#### Order among admissible options

No option under LQ1 to LQ21 is chosen. Round 3 changes what the owner weighs under three of them:

- LQ2 and LQ21: the three fixes for the stop channel (one preview per run as the last action on a low control path,
  stop-sensitive labels, capability separation) enter chapter 07 as options. None is preferred.
- LQ3: the threshold is a versioned up-set. Chapter 06 states the weakest one that keeps partition 8.1. Raising it is
  the owner's.
- LQ10: the envelope stores the seven facts. Which word encoding the data core uses stays open (R3-E4).

#### Experiments

- LX-3's must-refuse corpus gains four cases: a stop-selected preview, refusal-driven selection, a one-item pick, and
  accumulation across runs.
- R2-E1's combination test becomes the 504 cases of C.8, plus property tests of the two passes on small graphs.
- CT-KN2-06 becomes a conformance test with S, R and σ ≠ id cases.

Each runs only under LQ18 or inside its phase. None has run.

#### Questions for the owner

The items marked `needs_owner` go to the owner once, with the topic. None of them adds a question to partition section
15.

| Card | Question | What the owner weighs |
| --- | --- | --- |
| R3-E1 | LQ3, LQ10 | "Not run" and "not stated" as import values of job outcome |
| R3-E4 | LQ10 | The word encoding of the data core: one journal row citing literal letters by digest, stored as `ia16` chunks |
| The engineer advisor's LX-8 bars | LQ13 | The bars for the syntax comparison |
| R3-M1 | LQ3, LQ10 | The seven facts and the four versioned tables |
| R3-M3 | LQ2, LQ21 | The corrected facts and the three fixes for the stop channel |
| R3-M4 | LQ14 | Whether a check gates anything, given that EV5 can supply the means of understanding and cannot certify it |

#### Records

- The record of the mathematician's computations is synthetic exact geometry: headless, with fresh data from `assets/`.
  None of it is Windows, DirectX, input or performance evidence.
- The explainers P-E6 to P-E9, P-M6 to P-M10 and the corrected P-M1 to P-M5 go to the owner with the full text.
- The partition is corrected as listed above, and its section 19 records the corrections.

## Round 4

The kickoff has the mathematician oversee what lets a user understand a principle: the documents, the explanations, the
source tracing and the learning path, because the owner's 3 November principle presumes that understanding. Round 4 is
that check of the full text, at `683c863a`. The record and its follow-up are appended with their headings lowered two
levels, and two lines are changed: each opened a list of files with a private path, and those paths are now relative to
the repository root.

### Mathematician: the understanding parts

### R4: the mathematician's check of the understanding parts

**Verdict.** The core is sound and should stay: LB6.2, EV5.3, EV5.4, BND.12, BND.13, BND.72 and Theorem D state the hard facts correctly. A decomposition gives structure and not reasons, nothing certifies understanding, and a recorded sample never upgrades a claim. The text around that core does not match it yet, so I would not accept it as it stands. Four sentences promise what software cannot deliver. The `PrincipleRecord` has the right shape, but a real tool cannot fill it until "by hand" is defined and its fields are typed. An explanation can show an unreviewed result as "proved" today, and two gaps let a premise-carrying or finite result pass the same way. Eight glossary findings state something false, including Gauge and Frame; Standing is right in substance. The learning path is six step names, so its order cannot be verified, and where it can be tested it teaches evidence too late. Several errors began in my own cards M6, R3-M4 and N2. No fix needs an option under LQ1 to LQ21; where a finding reaches one, I stop there. Totals are 14 blockers, 29 major, 11 minor and 1 nit, and most blockers are one-line wording fixes.

**Reading notes**
- Line numbers are at HEAD `683c863a`. The source moved from `f7bb5d3f` during this check. In the cited files, chapters 03 and 04 changed only by reflowing and one added sentence (ch03:45-46), which affects no finding.
- I wrote no file and ran no engine computation. Numbers come from the cited records, plus my arithmetic on them.
- "My inference" marks anything I did not read directly in a source.

**Short names.** All paths are relative to the repository root.
- partition: `docs\architecture\1.0\language\ecosystem-partition.md`
- ch00, ch01, ch03, ch04, ch06, ch07, ch10: `docs\architecture\1.0\language\architecture\00-overview.md`, `01-kernel.md`, `03-libraries.md`, `04-environment.md`, `06-evidence.md`, `07-boundary.md`, `10-renderer-platform.md`
- DT-L1: `docs\progress\1.0\design-table\topics\language-ecosystem.md`
- terms.json, build.py: `docs\progress\1.0\glossary\terms.json`, `docs\progress\1.0\glossary\build.py`
- owner page: `docs\wiki\decisions\owner-decisions-2026-11-03.md`
- contract: `research\jumbling\state-contract.md`
- restoration: `research\jumbling\theory\restoration.md`
- card 6, card 7: `research\jumbling\theory\cards\card-6-blocking-structure.md`, `card-7-lowering-machine.md`
- PROOFS, t3p.py: `research\jumbling\restoration\14-blocking-structure\PROOFS.md`, `scripts\t3p.py` in the same folder
- stage 14, 15, 18 and 19 README: `README.md` in `research\jumbling\restoration\14-blocking-structure\`, `15-lowering-machine\`, `18-tail-completion\`, `19-height-depth-framework\`
- manual 02, manual 03: `research\jumbling\manual\02-descent.md`, `03-tail.md`
- tutorial: `research\jumbling\tutorial\README.md`, `02-bulk.md`
- open problems: `research\jumbling\theory\open-problems.md`
- tex: `research\Full_600cell_Puzzle_Theory.tex`

#### Question 1: does the `PrincipleRecord` work?

**Answer.** Nearly. The seven hand-written parts cover what a learner needs, and no part is mathematically meaningless. Three things stop it today:
- It never says which part is the principle, and `reason` can be a bare ID.
- `by hand`, `step map`, `justification` and `specification` are ill-defined as written.
- Several required parts do not exist for some real tools, so admission would refuse an honest record.

**Test: `t3p.py check` from stage 14.** My mapping: its nearest home is LB5's theorem-schema instance check (ch03:253).

| Part | How it fills | Fit |
| --- | --- | --- |
| specification | check(Z, certificate) on poses in Γ(Λ), since heights need the tree. Answer: passed, or the failed hypotheses of the first failing round (t3p.py:340-343, 398-406) | No empty answer exists; output type missing (M4, M5) |
| justification | Theorem T3P with Lemma A (PROOFS:72-96). Separately the code replays the word (t3p.py:352-382), and that replay needs no theorem | Two arguments, not one reference (M3) |
| instance | 7304 stop `ac127dc3…`, x = 522, h = 14, helpers (344, 9) and (451, 55): F = 22, D = 70, Φ falls from 1594 to 1498 (card 6, C7) | Fits |
| decomposition | Label table, inside sets, tree arithmetic, digraph test, sums, replay | Fits |
| step map | P0 to P4 map to nodes. Lemma A's induction has no node, and the label table has no proof step | A relation, not a map (M2) |
| reason | P0 and P1 let each helper act alone; labels are functions of the pose, so the counts at x add; Lemma Φ gives 2(F − D) at the last letter | Fits, if prose is allowed (M2) |
| failure | Overlapping inside sets at 229 and 336 break P0 (stage 14 README:407-408). F ≥ D gives no descent. Dropping P2 gives T3PE, a weaker theorem and not a break (stage 15 README:66-68) | "What breaks" is too narrow (M5) |
| not meant | A fail concerns this certificate at this Z only; "15 of the 131 descend" outside the family (card 7:19-20) | Fits |
| derivation | 1594 + 0 + 2(22 − 70) = 1498. The 0 is my arithmetic from these figures. 22 and 70 come from a 177,120 × 600 label table | Not hand-checkable as written (M1) |
| prerequisites | Branch, clean, foreign load, lowering count, tame | No correct glossary rows (M52) |
| leaf specs | Exact evaluator, tree arithmetic | Fits (see M9) |

**Contrast cases**
- The floor, LB6.2's own example, passes once `givens` exists. Cap 30 has In 840, Out 737, A 0 (manual 02:114-118). A person can check 840 + 737 = 1,577, 840 − 737 = 103 and 30,286 − 1,474 = 28,812. Nobody can check 840 itself.
- WC-PLAN (stage 18) has steps of up to 871,582 letters, and "a person cannot" play them (manual 03:62-63). It is a research tier candidate. If it shipped, INV-12 would apply and admission would refuse it. I stop at LQ16.
- D[k] (stage 19) has no producer and no implementation, so nothing exists to generate a decomposition from.

**R4-M1 (blocker). "By hand" is undefined and, read literally, undeliverable.** LB6.1, ch03:378-381; `derivation`, ch03:400; partition:114; CT-LB6-04, ch03:523.
No person can verify 840 cut cells or a count from that label table, and ch03:380-381 rightly excludes a toy geometry.
Replacement:
- Split `derivation` into `givens` (facts read from named leaf calls or certificates) and `steps` (what a person verifies from the givens, with the step count).
- LB6.1: "one small instance on the real geometry whose steps a person can check by hand from named givens".
- Add: "A digest is never a step."

Any ceiling on the step count is the owner's; I propose no number.

**R4-M2 (major). `step map` and `reason` are not well defined.** ch03:396-397.
- A map from nodes to lemma steps can be neither total nor onto. The floor's proof has three steps (N_c = In_c + Out_c − 2A_c; A_c ≤ min; a twist keeps In_c and Out_c), and the third has no node.
- Statement IDs name statements (ch06:78), so nothing addresses a step inside a proof.
- "By statement ID" permits a bare ID, and an ID is not a reason.

Replacement:
- "`step map`: a relation between nodes and (statement ID, step label) that reports the unmapped items on both sides."
- "`reason`: the statement ID and a short proof idea in glossary terms at or below the record's level."
- Add to LB6.2: "The principle of a tool is the statement named in `reason`, with its hypotheses." This is a proposal, and its wording is preference.

**R4-M3 (major). `justification` assumes one lemma or certificate per tool.** ch03:392.
Theorem MC certifies a word event by event (stage 15 README:61-65), so its evidence is per output. U2's "take the most misaligned first" (restoration:103) has no lemma. A tracking tool is true by definition. `t3p.py check` has two arguments.
Replacement: "`justification`: a list of arguments, each a lemma, a per-output certificate kind, a replay or "by definition", with an evidence reference where one exists."

**R4-M4 (major). `specification` lacks what a reader needs to use the answer.** ch03:391.
It has no output type, puzzle, menu, level (KN1.15), parameters, tie rule or budget dependence. "Function" also fails for P*, a procedure with a key, E₀ = 20 and beam 64 (stage 15 README:168-189).
Replacement: "`specification`: function or declared procedure; inputs, parameters and budget; output type; puzzle, menu and level; domain; tie rule; empty-answer meaning; negative-answer scope."

**R4-M5 (major). Required parts may not exist, yet admission refuses incompleteness.** ch03:398, ch03:410.
A check has no empty answer. A definition has no hypothesis. Dropping a hypothesis can give a weaker theorem or an open question.
Replacement: "Every part admits a typed "none, because ...". `failure` is given per hypothesis, as a concrete counterexample, a weaker conclusion or "necessity open"."

**R4-M6 (major). No rule ties a tool's domain to its lemma's scope.** ch03:391-392.
T3 is stated on Γ(Λ) (restoration:25). In free mode ω = (2,0,0) at cap 0 has heights (0,0) and is not in K⁺ (DT-L1:1653, :1659). A height-based lattice test claiming "every pose" and citing T3 would be wrong there, and the record format would accept it.
Replacement: "The domain lies inside the cited statement's scope, or the record names the part outside it and what the answer means there."

**R4-M7 (major). Binding to the module digest alone is too weak.** ch03:384.
A dependency or kernel change leaves that digest unchanged while the decomposition changes, and a regenerated graph can strand the hand-written step map.
Replacement: "The record is bound to the module digest, its lockfile closure, the model identity and its statement IDs. A changed decomposition marks the hand-written parts stale."

**R4-M8 (minor). The floor example cites no pinned statement.** ch03:385-387; EVD.43, ch06:332; DT-L1:1479.
The proof exists: tutorial `02-bulk.md`:94-99 ("proved (model review only)"), manual 02:34-36 and open problems:125, restated in BND.10. It has no stage-qualified name, which EVD.2 requires for basis theorem.
Replacement: cite open problems:125 in LB6.2 and EVD.43, and name the statement before it serves as evidence.

**R4-M9 (minor). Three small rules are missing.** ch03:401-402, ch03:408.
Replacement:
- "Prerequisite records form a DAG, and each prerequisite term sits at or below the record's level."
- "A leaf specification names the oracle test that covers it."
- "An over-approximated call edge is marked "may call"."

#### Question 2: does any clause promise too much?

**Answer.** Yes. No clause says outright that software certifies understanding, but INV-12's title and EV5.1's "alone" imply it, and "authored" and `author: own` imply authorship. No clause says a decomposition supplies reasons, but INV-12, the glossary row and CT-EV5-05 leave the reasons out.

**R4-M10 (blocker). "Understanding is a deliverable."** partition:110, :117; EV5.1, ch04:418-420; origin my M6, DT-L1:447.
This promises a state of a person as a product. "Alone" implies that a combination could establish it, which contradicts EV5.3 (ch04:435-436) and BND.72. Two identical records can differ in what the person understood (BND.12), so no combination helps. "This understanding" also slides from built-in tools to the solver's own tool.
Replacement:
- INV-12 title: "The means of understanding are a deliverable."
- EV5.1: "Nothing the software records, alone or together, establishes that a person understands a principle, and none of it decides the joint boundary."
- partition:117: "The 3 November principle presumes that a solver understands their tool's principle. The program can supply only the means."

**R4-M11 (blocker). BND.82 fails in both halves.** ch07:596-597; also ch07:545, partition:563, partition:588, ch07:511.
- A record cannot describe "what a solve used": a retyped search result appears as the solver's word (BND.13, BND.35, BND.76).
- "Ships no solver" presets LQ16. Partition:563 says "The shipped research tier (ready-made search, strategies)" in the same row as "the program ships no solver". ch03:251 lists P* and WC-PLAN as candidates, and BND.74 question 7 is open.
- The same overreach sits in BM4 (partition:552), BND.35 (ch07:275) and Assistance summary (ch04:662).

Replacement:
- BND.82: "The boundary describes what the system recorded about a solve, and says what it cannot record. Under the interim reading, nothing shipped in the solving profile chooses a solving word by itself."
- BM4 and BND.35: "which assistance classes the system recorded".
- Assistance summary: "The per-solve account of the assistance the system recorded."
- BND.70 and the BS2 cells: "the solving profile ships no solver".

Whether a research tier ships is LQ16, and I stop there.

**R4-M12 (blocker). A bug can make a claim read as proved.** ch00:226-227.
EVD.54 says the word function is library code and "a bug in them can give a wrong word" (ch06:453-454), and a user module can say anything (ch00:221).
Replacement: "A bug in the language can make a tool lie, and a bug in the word function can show a wrong word. It cannot corrupt a session, commit a word the solver did not see, or change a stored claim."
Proposal: "An evidence word reaches a display only through `word()` on a key; any other program text is shown as the program's own." I found no such clause, by grep only.

**R4-M13 (blocker). "Every result can be traced to the primitives."** partition:116; compare ch03:379-380.
Stage 19's archive holds no executable producers (stage 19 README:188-192; partition:457), so an imported result cannot reach primitives. "Reported" results have no committed program (ch06:400). EV2.9 and EV2.12 are open.
Replacement: "Every result the runtime produces can be traced to primitives and evidence. An imported result traces to its stored bytes and source label, and the trace says that its producer is absent."

**R4-M14 (major). Names imply a measurement.** "Understanding labels" (ch03:454), "understanding gate" (ch04:471, ch04:507-508), "A checkable principle" (ch03:564), "principle check" (ch03:412).
None is defined, and each reads as a test of understanding.
Replacement: "competence results", "competence gate", "A principle record stays required", "no inherited pass, cost approval or record".

**R4-M15 (major). "Authorship" contradicts BND.18 and BND.20.** partition:562-565; ch07:510-513; BND.23, ch07:162; ch03:469.
BND.20's own counterexample applies: text copied by hand reads `own`.
Replacement: "Author class (BND.18): created or forked locally. It records how the bytes arrived, not who thought of them." ch03:469: "chapter 07's author-class mechanism". The field name `author` is preference.

**R4-M16 (major). The solver's own tool has no record and no defined principle.** EV5.9, ch04:469-473; partition:564; ch07:512.
Owner decision 6 concerns the solver's tool, but LB6 covers built-ins only and "that principle" has no map from a tool. My inference: code cannot supply that map, because a closed formula for the floor and a loop over 60 items have different call graphs and one principle.
Replacement, added to EV5.9: "No source defines which principle a solver's tool embodies. Any such map belongs to the joint setting." I stop at LQ2.

**R4-M17 (major). The hand-written parts exist only in LB6.2.** partition:111-115, :214; ch03:377-381; ch04:642; CT-EV5-01, ch04:788; CT-EV5-05, ch04:792; partition section 19.
A record with no `reason` satisfies INV-12 and CT-EV5-01, and only CT-LB6-01 catches it. CT-EV5-05 tests "to its primitives", which is structure.
Replacement:
- Row: "A tool's specification, justification, worked instance, generated decomposition and the parts a person writes: step map, reason, failure, what an answer does not mean, derivation, prerequisites and leaf specifications."
- CT-EV5-05: "to its primitives and its reasons".
- Add the hand parts to INV-12 and to section 19.

**R4-M18 (major). "A fixture proves numbers."** ch03:408-409; origin my DT-L1:1485.
This conflicts with EV5.4's "source evidence" (ch04:442-444). Nothing says where expected claims come from, so a fixture can test a module against its own output.
Replacement: "A fixture pass shows that the module reproduced the expected claims at that input. The expected claims come from the written derivation, never from the module's own output."

**R4-M19 (major). The [B] tags overstate the record's status.** ch03:377; ch04:417 (the understanding sentence); ch04:429; partition:40.
[B] means restating "a source in force" (ch00:66). INV-12 sits on a page marked "Not accepted" (partition:3-4), and no owner text names a principle record (DT-L1:46-50; owner page:46-48). The chain is my M6, default 18 and R3-M4.
Replacement: tag these [D], and write at partition:40 that INV-12 is a proposal built on the owner's principle. This corrects a status and does not object to the record.

**R4-M20 (minor). BND.72 and BND.73 say slightly too much.** ch07:558-560, :562-563.
No act of "a promise the solver makes" exists. A check cannot show who completed a task, or what help came from outside the program. Partition:772-773 already foresees BS3 gating.
Replacement: "A competence check records that a task was completed in this install, with the help used inside the program. Neither shows what anyone understood, so "understanding" in BND.3 stays outside what the software can establish; how the boundary treats that is the joint setting's question."

#### Question 3: are the glossary additions true?

**Answer.** Mostly, but eight findings state something false. Gauge and Frame fail. Standing is right in substance. Exact transport is the only duplicate name in the table; a one-line check of all 199 rows confirmed that.

**R4-M21 (blocker). Exact transport appears twice.** ch04:546, :637.
build.py:60-62 fails on a duplicate, and row 546 is too narrow for KN7.5 and EVD.18.
Replacement, keeping one row: "A lemma-justified move of a claim to another input that preserves its subject, with no re-run. Along a gauge, the lemma is Theorem 1 of note N1 and the checked relation is Z = X·γ."

**R4-M22 (blocker). Frame group describes R_A, and it duplicates an existing entry.** ch04:543.
- "The turns that keep its region" are the elements of R_A, not of the quotient. A centre has 12 such turns and a trivial frame group. Orbit 0 has 6 turns and a group of order 2 (DT-L1 B.3, :1546-1563).
- Orientation group (terms.json:246, L5) is the same concept: the census equals `HG` on all 35 orbits (DT-L1:1563), and tex Lemma 7 calls it H_o. Two names break M1, and L6 sits above where the solver meets it.

Replacement, one entry at L5: "The group of permutations of a piece's sticker regions that turns keeping its region can make: R_A/S_A; trivial, C2, C5, D5 or A5 by orbit." Which name stays is preference. The old gloss "ways ... turned in place" goes too.

**R4-M23 (blocker). Frame row and EV3.8.** ch04:533, ch04:268-271; origin my DT-L1:1492-1493.
- "The other three uses" is a false count. Frame also names the frame domain (ch04:577; card 6:14), "frame part" (ch04:578), frame bits (contract:171), frame sums (tex Lemma 8), "piece frames" (restoration:173) and "D's frame" (restoration:53).
- "Take their own names" fails, because "the C₃ frame component no sticker shows" is a description. It also omits O01's "C₂ component of the same kind" (restoration:174).
- That component lies in the abelianised K⁺ stabiliser (restoration:173), so it is not a frame-group value.
- KN1.7 defines no reference stickers; the source is tex Lemma 7. The old gloss's "fixes how a piece or block is oriented" is lost.
- A3's F_f is per facet and Lemma 7's frame is per position (my inference), so "transported frame" may not be one thing.
- "The relation "frame" is the frame group" is my type error. A role takes one element, which depends on chosen reference frames, and changing them conjugates it (tex:997-998).

Replacement:
- Row: "Unqualified, an ordered choice of reference stickers that fixes how a piece or block is read. Every other use carries a qualifier."
- EV3.8: "The role "frame" takes a value in the frame group under chosen reference frames, at label level, on the lattice; changing the frames conjugates the value." The words chosen are preference.

**R4-M24 (blocker). Gauge drops its defining condition and reads as "same state".** ch04:544.
- The row omits that t_A carries A's reference region onto σA's (ch01:115-116; used at DT-L1:1527). Without it the admissibility clause is false: σ = id with a small rotation t_A moves A's region.
- "Renames" misleads. A piece's identity is canonical (INV-6). For σ ≠ id, A takes σA's place and digests differ (DT-L1 B.6 (e)). For σ = id, solved with one centre turned is a gauge image of solved, passes the sticker test and is unreachable (KN1.17).
- On the charitable reading "representatives of the region class" the second clause is true, but the row gives a reader no way to know that.

Replacement: "A pair (σ, t): σ permutes the pieces, and each t_A is a rotation carrying A's reference region onto that of σA. Piece A takes the place σA had, so the same regions stay occupied. By Theorem 1 of note N1 (status: theorem, producer) every word stays admissible and the two runs stay related by the same gauge. A gauge is not equality and does not carry reachability."
The word "gauge" is preference.

**R4-M25 (minor). Standing is right, but three words need care.** ch04:641, against EVD.30.
"Threshold" has no row, "carries" is undefined, and "unrefuted" means "whose negation does not stand in the first pass".
Replacement: "Whether some argument for a statement survives when every grade outside the threshold is distrusted: first with no premise assumed, then with every premise assumed except those whose negation stood in the first pass. It describes the records under one edition; it does not say the statement is true."
That the plain word suggests approval is preference.

**R4-M26 (blocker). Solving session contradicts its source.** ch04:658, against BND.14 and BND.17.
A profile belongs to a run, not to a session.
Replacement: "A session whose journal can stand as a solve record; every ordinary session is one. A profile belongs to a run."

**R4-M27 (blocker). Label checkpoint is not label-level.** ch04:586, :585.
KN1.15 (ch01:188) and contract:174-175 make it a pose-level fact through its reachability clause, and the row is circular. Counterexample: solved with one centre turned has solved labels and is unreachable (KN1.17).
Replacement:
- Label checkpoint: "A reachable lattice configuration with ν = 0 (state contract A5). Its reachability clause reads the pose and the journal, so labels alone cannot establish it."
- Retained checkpoint: "A lattice configuration with a witnessed legal word in the 1,200 retained generators from solved, or certified equal to such a state. Every retained checkpoint is a label checkpoint." (contract:76-80, :176)

**R4-M28 (blocker). Five evidence rows disagree with EVD.32.**
- Not found within budget (ch04:625) implies a scoped negative exists. The beam reads this word while 228 descending words exist (ch06:442-444). Replacement: "A search stopped at its budget or declared limit without covering its set, and found no witness. It gives no negative at all, even inside that set."
- Finite instance (ch04:624): "without a universal claim" is false for an exhaustive negative. Replacement: "The word for a statement that stands on one input or on a declared search set. Nothing outside that set is claimed."
- Conditional (ch04:623): a premise whose negation stands gives open, not conditional (ch06:240-241). Replacement: "A statement that stands once every premise not refuted is assumed; the word names the premises."
- Incomplete (ch04:626): a budget stop is also not a completed search, yet it reads row 5. Replacement: "The job was cancelled, failed, or hit a resource stop, so it gives no negative. A search stopped by its own declared budget reads "not found within budget"."
- Basis (ch04:610) omits exhaustive computation (ch06:41). Replacement: "The theorem, certificate, exhaustive computation, exact computation or sample on which a claim rests."

**R4-M29 (blocker). Rim summary is not per named cap.** ch04:574.
KN2.4 (ch01:236), ENG-0J and `jumbling-observation.md`:128 give it for every cap.
Replacement: "Per revision, for every cap: N_c, In_c, Out_c, A_c, blocked, and whether it is at its floor. It selects no twist and no cap."

**R4-M30 (major). Reachability claim calls a promise evidence.** ch04:597.
Promised maps to conditional (ch06:266-267), and open is no evidence at all.
Replacement: "What is on record about whether a state can be reached in a named puzzle: witnessed, by theorem without a witness, promised, refuted or open."

**R4-M31 (major). Orientation is circular and merges two quantities.** ch04:532.
Label-level orientation (core.py:268) is not A3's per-sticker element, which differs at equal labels (DT-L1:1536, :1639; ch01:133).
Replacement: "How a piece sits in its position as its labels show it; a piece can be at Home and wrongly oriented. A3's pose-level orientation is a separate quantity that labels do not read, and takes its own name."

**R4-M32 (major). ν is called an invariant without saying of what.** ch04:582.
Jumble words change it (contract:185).
Replacement: "The 45 coordinates read from the labels of a lattice configuration: 35 sign bits, 8 frame bits and two values modulo 5. Retained twists do not change them, and jumble words can. Undefined off the lattice."

**R4-M33 (major). Homonyms break EV5.10 (ch04:476).** One reference cannot resolve to one concept for these. Telling them apart is correctness; the words are preference.
- Domain (ch04:613) against alignment domain.
- Projection (ch04:707) against π and KN1.7's maps.
- Key (ch04:608) against keyboard key.
- Checkpoint (ch04:675) against the two checkpoint predicates.
- Branch (ch04:673) against tree branch.
- Edition (ch04:650) against EVD.14's edition.
- Rank (ch04:682) against census rank.
- F_c (floor) against F_x (foreign load).
- Clean grip against clean step.
- Block, in three senses; provenance, in three senses; macro, against stage 15's macro.

**R4-M34 (minor). Circular or empty rows.**
- Lattice (ch04:566): "Configurations where every piece's pose lies in K⁺, so each piece fills a chamber and its stickers sit on slots." (contract:26)
- Invalid (ch04:564): "The outcome of a request that is not an exact element of the active menu; the state is not tested." (`research\jumbling\sim\state.py`:348-350; the contract's outcome list has no such outcome)
- Cap (ch04:521) uses grip as a place (KN1.5:88), while Grip says "the choice" (terms.json:80). Grip needs a listed scope change.
- Tree, Section, Action, Embedding, Computed and Verified repeat their own word. Their owning chapters should supply a definition.

**R4-M35 (minor). Imprecise rows.**
- Generator (ch04:524): "One of the 1,200 generators of 600-cell-Full; a negative number is its inverse. Every count names its alphabet."
- Height (ch04:571): add "On Γ(Λ), left and right height 0 together mean the lattice; in free mode they do not."
- Lowering supply (ch04:569): sources say "lowering count" (stage 15 README:19; ch03:146). "Lowering count D_x(h): the number of inside pieces of height at least 4 and branch CLS(x) that item h lowers at grip x."
- Alternative (ch04:640): "One argument's summary: its weakest grades and its premises. A statement's summary keeps the undominated ones." (ch06:188-189)
- Reported (ch04:636): "Computed by a program that is not committed, so the repository cannot replay it." (ch06:400)
- ν-correction (ch04:583): "One of the 36 named constructions in the static catalogue, with a fixed effect on ν."

**R4-M36 (minor). Existing entries need changes the table does not list.**
- Occupant is partial off the lattice.
- Stage order's UI "next-orbit suggestions" (terms.json:129) and Target's "candidate list with reasons" (terms.json:169) conflict with BND.65 and LB6.7.

#### Question 4: is the trace honest about standing?

**Answer.** It is honest about recorded samples and named premises. Theorem D is right for what it states. An explanation can present an unreviewed result as proved today. Two gaps beside Theorem D let a premise-carrying or finite result read as proved.

**R4-M37 (blocker). Unreviewed reads as proved.** Proved, ch04:622; EV5.13, ch04:492-496; EVD.32 row 3; EVD.39, ch06:286-287.
(theorem, producer) lies in 𝒰₀, so it stands and reads "proved", with the evidence shown "on request". The row says "established". Note N1 is itself at (theorem, producer) (ch01:181-182), and no research proof has had a human review (owner page:57-58). Raising the threshold is LQ3, and I stop there. This finding asks for neither that nor human review.
Replacement, independent of LQ3:
- Add to EV5.13: "An explanation shows the check beside the word, and says so where no human review or formal proof is recorded."
- Row: "The word shown when a statement stands at the threshold with no premise assumed, over an admitted domain or every reachable state. It always carries its scope phrase. It says the recorded argument passes the threshold, not that a person has reviewed it."

**R4-M38 (major). A premise inside a statement disappears from the phrase.** E2, ch06:430; EVD.36, ch06:270-272.
Counterexample (my inference; stage 19's records not read):
- A lattice table with two pieces of O0 swapped has b0 = 1, which never occurs on a reachable configuration under I_a (contract:173).
- Menus are inverse-closed, so no word solves it (DT-L1:1540).
- It passes the k = 0 lattice test (stage 19 README:143-145), and such a table "does not inherit the sticker theorem" (:167-168).

A reader of "proved on D[k]" concludes the bound holds for it. Theorem D (c2) treats a concrete Z correctly, so only the phrase misleads.
Replacement: "The scope phrase names any premise class held inside the statement: "proved on D[k], for reachable inputs"."

**R4-M39 (major). A finite computation can read as proved.** ch06:43, :171-172, :297, :312.
Theorem D (b) covers only leaves recorded as sample, while "sample" is relative to a statement. I2 reads identity fields, and `covers` is not one of them. EVD.21's table reads (basis, check) pairs, which cannot see the statement.
Counterexample: the I_a fixture at t = 150 has heights at most 4 (restoration:45). Attached as (exact computation, producer) to "every reachable state has height at most 4", it reads row 3. Height 8 occurs, by the same line.
Replacement: new I7, "A leaf of basis exact computation, exhaustive computation or certificate under a universal or mixed statement, where the leaf's own covered set is smaller than the statement's `covers`." Leaves then need a covered-set fact. This corrects my N2.

**R4-M40 (major). The lemma's word is not the answer's word.** EV5.3, ch04:429-431; EVD.17.
An answer rests on the lemma, on the input's membership and on the code computing the function. T3P reading "proved" says nothing about a certificate that its own program re-evaluated (corpus rule C3, `docs\architecture\1.0\language\evidence-import\refs.txt`:39).
Replacement, added to EV5.3: "The word of a cited statement is the word of that statement only and never reads as the word of the shown answer."

**R4-M41 (major). Row 2 may drop its scope, and the claim field says "proved".** ch06:270-272, :257, :281, :445.
"Finite instance" hides "three letters of family R only". A view printing `claim` prints "proved" for a finite instance.
Replacement: "Every row prints its scope in an explanation, row 2 included. The `claim` field is never a display word."

**R4-M42 (minor). Two kinds of edge look like one.** EV2.5, ch04:163-167; Provenance graph, ch04:619.
"This value came from that call" is not support.
Replacement: show computation edges and evidence edges separately, count only evidence edges, and show the edition with the word (EVD.14).

**R4-M43 (minor, unverified). Statement references.** EV3.8 gives relations a statement reference. I could not establish that every other shown fact has one, because I did not read chapter 10 in full.
Proposal: a shown fact without a statement reference carries no evidence word.

#### Question 5: does the learning path teach ideas in time?

**Answer.** This cannot be shown yet. Where it can be tested, reading evidence and reading a principle come too late or not at all. The level findings below are my inference from the rows.

**R4-M44 (major). The steps have no content.** EV5.6, ch04:451-455; partition:225.
The clause says each step names its concepts, routes and fixture task, and none is given.
Replacement: add a table (step, prerequisite concepts, command routes, fixture task), marked as owed until written.

**R4-M45 (major). Reading results comes last.**
Step 1 needs evidence words, empty against not found against not run, stale, unknown against "none: off the lattice", and the scope of a negative. Evidence appears at step 6.
Replacement: step 1 becomes "use tools and read their results". Step 6 stays "maintain modules and evidence".

**R4-M46 (major). Used before taught.**
- Step 2: round trip is L6. An edit makes a new digest, and "a fork inherits no evidence", so fork and scope come first.
- Step 3: Layout is L6.
- Step 4: Preview (L2) reads full effect, while Complete effect is L4.

**R4-M47 (major). No step teaches a principle.**
Nothing teaches reading a record or stating one's own tool's principle, although the owner's principle turns on this.
Replacement: before "write tools", add "read a principle record, then state your tool's specification, reason, a dropped hypothesis and a worked instance".

**R4-M48 (major). Concepts missing for building a tool from base parts.**
- Language: exact against approximate; level typing; snapshot and revision; partial results and verdicts; budgets and jobs; effects and capabilities; profiles; derived state and reachability; fork semantics; the boundary itself (rule 6, the flow rule, why a preview is refused).
- Logic: universal, existence, mixed, negation, premise, witness.
- Mathematics: conjugate, commutator, stabiliser, coset, quotient, homomorphism, invariant, F2 and F5, Q(√5).

**R4-M49 (major, owner scope). The mathematics may have no text.** EV5.11, ch04:482-483.
"Its 1.0 scope is what the interface shows", so LB1's mathematics may go unwritten. Whether the language's library counts as shown is the owner's call, and I stop there.

**R4-M50 (major). The levels do not cover the jumbling or language terms.**
L1 to L6 are retained-workflow levels, and `solving-workflow.md` has no jumbling content. 89 of 199 rows sit at L6, so bar M6 cannot be checked. The manual and tutorial chapters give an order (my inference), but EV5 cites neither, and the tutorial is "frozen and shelved, unaccepted" (tutorial README:70). Using them is the owner's call.
Replacement, added to EV5.14: "Jumbling and language terms need a first-use order before their levels can be checked."

**R4-M51 (minor). CT-EV5-02 tests flow goals only.** ch04:789.
Replacement: add "each step's concepts are taught earlier, and prerequisites form a DAG."

#### Question 6: bar M1 and the term levels

**Answer.** Yes to both: learner-facing terms lack entries, and terms sit above their level.

**R4-M52 (major). Learner-facing terms with no entry.**
- A grep of terms.json and the table found none of: principle, decomposition, fixture, base tool, base part, fork, lemma, theorem, hypothesis, invariant, counterexample, witness, threshold, stale, chamber, digest, level, floor, foreign load, clean grip, tame, seam, cut cell.
- From reading, also none of: hand derivation, step map, leaf specification, statement form, negation pair, typed inconsistency (row 0's word), evidence graph, base revision, research tier, flow rule, free mode, solve record, u, covers.

**R4-M53 (major). Terms above their level.** Each left term's definition needs the right term.
- Height counts (L1) against Height (L6).
- N (L1) against domain and cut cell.
- Menu identity and Menu-order digest (L1) against digest (L2).
- Canonical ID and Relation (L1) against the identity levels (L6).
- Principle record (L2) against Evidence envelope, Typed IR and Statement ID (L6).
- Solving session (L2) against Profile (L6).
- Assistance summary and Proposal (L2) against Provenance block (L6). Whether labels show at preview is BND.74 question 6, and I stop there.
- Typed partial result, Availability and Seat ("right-coset") at L1.
- Relationship strip at L1 ("consumer" is implementation language). Replacement: "A display of typed relations about what is in view, in the grammar shared by code and the theory book."

**R4-M54 (minor). EV5.10's check does not exist.** ch04:475-479; build.py:60-62 checks duplicates only.
Proposal: a definition may use only terms at its own level or lower, and rows that never reach the UI carry a "not learner-facing" flag.

**R4-M55 (nit).**
- partition:482-484 drops the owner's "by itself" (owner page:47-48).
- "Base" has several senses; what "built-in base tools" means stops at LQ2.
- EV5.7's "independent completion" is the owner's wording, and the software sees only help used inside the program.
- EVD.18 cites KN1.13 for gauge transport (ch06:150), and KN1.16 is the clause.

#### Checked and found right

- **Understanding core:** all seven R3-M4 parts are in LB6.2. Also right: "structure, not reasons", "cannot certify", EV5.3's repair-list sentence, EV5.5 and LB6.3 on forks. EV5.7 matches the owner's text, and EV5.8 and EV5.12 invent no threshold.
- **Tracing:** EV2.2, EV2.4, EV2.6, EV3.3, EV3.4 and EV3.9, plus EV2.5's "adds no evidence edge". Theorem D (a), (b) for recorded samples, and (c1) to (c3). EVD.30, EVD.33, EVD.34, EVD.42 to 45. Examples E1 to E3 read correctly under the word function.
- **Boundary:** BND.3 matches the owner's wording. BND.7 to 9, BND.11 to 13, BND.20 and BND.79 are right. BND.81 and ch10:348-350 make no promise.
- **Kernel:** KN1.7 and KN1.14 to 1.17 match N1. I re-derived the gauge product law and re-read Theorem 1's proof steps. ν's 35 + 8 + 2 coordinates match contract:171, and the census frame groups match Lemma 8's abelianisations at restoration:180.
- **Rows true against their source** (level and homonym issues are listed above):
  - Puzzle and model: Label, Piece, both Twist rows, Retained twist, Jumble twist, Orbit, Net and Strict, Complete effect, Position, Pose, Sticker, Model identity, Canonical ID, Identity block, Identity tower, Sticker class, Region class, Shape, Level-qualified digest.
  - Menus and words: Menu, Menu letter, Menu identity, Menu-order digest, Seat, Jumble on and off, Word, Literal word, Equality key, Recipe, Compressed word, Macro, Macro base, Cost alphabet.
  - Jumbling: Admissible, Blocked, Uncertain, Straddler set, N, Depth Φ, Domain ID, Height counts, Lifted, Jumbling block, Jumbling frame domain, Clean-step unit, Pose unit, Clean step, Excursion, Repair list, Sticker test.
  - State and results: Revision reference, State reference, Snapshot, Derived state, Derived model, Difference, Typed partial result, Verdict, None: off the lattice, the three reachability values.
  - Commands: Proposal, Preview, User-action token, Receipt, Availability.
  - Evidence: Claim, Statement ID, Key, Scope, Check, Premise, Goal, Certificate, Check report, Evidence, Evidence envelope, Per-claim trace, Evidence words, Refuted, Open, Recorded, Unknown, Measured, Lead, Check mode, Grade.
  - Boundary: Author class, Provenance block, Boundary class, Declassifier, Flow label, Capability separation, Sink, Transparent entry, Policy table, Pick record.
  - Jobs and session: Job, Job result, Budget, Lane, Latest wins, Kernel row, Language overlay, Journal, Workspace record, Filter, Preference.
  - Environment: the EV1 to EV4 rows, Competence check, Soft disclosure.

**Not checked, or only partly:**
- Rows owned by chapters 02, 05, 09 and 10 were read for internal consistency only: Library entry, Effect, Capability declaration, Typed IR, Module, Module manifest, Lockfile, Edition, Reproducibility key, Run record, Search space, Search record, Strategy, Rank, the view and display rows, and the representation rows.
- Verified and Unchecked were checked only against the mapping at ch06:401, because I did not read H-08.
- For stages 16 and 17 I read headings only. For stage 19 I read the README only, and manual 02 and the tutorial only at the cited lines. I read no modified `PX-*.json` file.
- I did not re-derive BND.10's sentence on height-2 domains.
- I ran no tests and no geometry.

#### For the owner: what "understanding the principle" can mean here

Software cannot measure understanding. The program can give you everything needed to understand a tool: what it computes, why that is true, where it stops being true, a worked example you can check yourself, and a trail back to its sources. It can record that you finished a practice task and what help you used inside the program. It cannot see what is in your head, who first thought of a word, or what you did outside the program: a retyped word looks like your own. So inside this software "understanding the principle" can only mean two things: the means are there, and the records are honest about what they show. Whether that is enough for a solver's tool to count as their own is for you and the integrator to decide. And "proved" here means passed the recorded checks; no person has yet reviewed the research proofs.

### Mathematician: follow-up

The record left seven items for drafting: R4-M52, R4-M53, R4-M33, R4-M44 to R4-M48, R4-M39, R4-M38 and R4-M43. The
mathematician drafted them on request and wrote no file. Line numbers in this reply are at `01263d68`.

All seven drafts follow. I wrote nothing and changed nothing.

HEAD moved after your message, from d97240d8 to 01263d68, and the language documents are clean in the working tree. Everything below is rebased onto 01263d68. Where lines moved I cite clause IDs. `chNN` means the chapter under `docs/architecture/1.0/language/architecture/`.

#### 1. R4-M52

| Term | Level | Definition | Source |
| --- | --- | --- | --- |
| Principle | L2 | The statement named as a tool's reason in its principle record, with that statement's hypotheses: why an answer means what the specification says. | LB6.2, ch03:413-414 |
| Decomposition | L2 | The generated list of the parts a tool calls, down to base parts. It shows how the tool is built, not why its answer is right. | ch03:398; EV5.4 |
| Fixture | L2 | A fixed input with its expected results, kept apart from the solver's session. A pass shows the tool reproduced those results at that input, nothing wider. | ch03:397; LB6.2; EV5.7 |
| Base tool | no row yet | Owner's phrase. What counts as one is the joint setting's question, so I give no definition (LQ2). | owner page:46; BND.3; ch07:509 |
| Base part | L2 | A shipped function at which a tool's decomposition stops. | LB6.1; ch03:398 (inference: library entries and kernel calls) |
| Fork | L2 | Your own copy of a built-in tool. It keeps a link to its origin and inherits none of the origin's evidence. | LB6.3; EV5.5 |
| Lemma | L2 | A statement with a written proof, used as a step towards other statements; cited by stage and source commit. | EVD.2 |
| Theorem | L2 | A statement with a written proof, cited by stage and source commit. As a basis it says a proof is cited; who checked it is separate. | EVD.1, EVD.2 |
| Hypothesis | L2 | A condition a statement assumes; the statement says nothing about cases where it fails. | ch03:402; ch06:118 |
| Invariant | L2 | A quantity or property that a named set of twists leaves unchanged. | LB6.2 (invariance of In_c and Out_c) |
| Counterexample | L2 | A checked case that lies in a statement's scope, meets its hypotheses and breaks its conclusion. | EVD.42; ch03:402 |
| Witness | L2 | A concrete object that proves something exists; for reachability, a finite legal word from solved. | KN2.13; EVD.44 |
| Threshold | L2 | The versioned bar a recorded argument must clear before a word such as proved or conditional appears. | EVD.29 (raising it is LQ3) |
| Stale | L2 | Output read at a revision whose dependencies no longer match the current one. It stays marked with that revision and is never passed off as current. | EV3.3; RPI.12, RPI.13 |
| Chamber | L1 | One of the 177,120 fixed places a piece can fill. A position names one. | state contract:26; card 6:13; tex:753-756 |
| Digest | L2 | A short value computed from exact bytes, used to name them. Digests compare only at the same level. | KN1.8 |
| Identity level | L2 | How finely a value describes a state: from each piece's exact pose down to what the stickers show and coarser. Every stored value names its level. | KN1.7, KN1.8 |
| Floor | L1 | A lower bound on a cap's misalignment: the size of the difference between In_c and Out_c. Twists of that cap alone keep both, so they cannot go below it. | manual 02:34-36; open problems:125; BND.10 |
| Foreign load | L6 | The number of pieces in grip x's cap of height at least 2 and branch other than the grip's class. | card 6:20; ch03:146 |
| Clean grip | L6 | A grip with no straddlers, so all its twists are admissible. The opposite of blocked. | card 6:17 |
| Tame | L6 | Of a word: no letter raises Φ. Stronger than every prefix staying at or below the start. | card 6:22 |
| Seam | L1 | A misaligned cut cell. An off-grip seam lies on a rotated cut that is no cap's cut, so no single twist closes it. | restoration:58; observation:25, :124 |
| Cut cell | L1 | A pair of neighbouring chambers on either side of a cap's cut, sharing one face on it. | restoration:50; observation:110 |
| Derivation (my "hand derivation") | L2 | The written working of a worked instance: the givens read from named parts, and the steps a person checks from them, with the step count. | ch03:404-405 |
| Step map | L2 | A stated relation between a decomposition's parts and the steps of a proof, reporting what is unmapped on both sides. | ch03:400 |
| Leaf specification | L2 | A written statement of what an opaque base part computes, naming the test that covers it. | ch03:407 |
| Statement form | L2 | Whether a statement concerns one finite case, every case, some case, or a mix such as every case having some answer. | EVD.9; ch06:72; DT-L1:1376 |
| Negation pair | L2 | A statement and its exact opposite, recorded together, so that one record can settle both. | EVD.9 |
| Typed inconsistency | L2 | The word shown when the records behind a statement contradict each other or break a record rule. It states no claim. | EVD.32 row 0; EVD.40 |
| Evidence graph | L2 | The record of how supports combine: an argument needs all its parts, and any one argument that clears the threshold carries the statement. | EVD.15, EVD.39 |
| Base revision | L2 | The revision a result or view read. Once it is replaced, the output is stale until recomputed. | EV3.3; RPI.5 |
| Research tier | L6, not learner-facing unless LQ16 ships it | The entries that search, minimise, rank or select over twists, words or caps. They stay out of the solving profile; whether they ship is open. | LB1.5; BS2 |
| Flow rule | L6, not learner-facing unless LQ2 turns it on | A policy switch: when on, in the solving profile, a preview's word and target may depend on the state or computed results only through declassifiers. | BND.33 |
| Free mode | L6, not learner-facing | The simulator's infinite field-valued alphabet in place of a finite menu. On Γ(Λ) left and right height 0 together mean the lattice; in free mode they do not. | state contract:158; ch02:130 |
| Solve record | L2 | The journal of a solve, read as its record, with the assistance the system recorded. | BND.14, BND.35 (my reading) |
| u | L1 | The number of pieces that straddle a cap's cut and so block it. | engineering.md:60; ch01:238; ch03:142 |
| Covered set | L2; the field name `covers` is not learner-facing | What a claim is about: one instance, a declared search set, an admitted domain or every reachable state. | ch06:81-82; EVD.32. Not the kernel row `covers` (ch01:239) |

The level rule forces seven more rows:

| Term | Level | Definition | Source |
| --- | --- | --- | --- |
| Revision | L1 | One numbered moment of a session; each change makes a new revision. | KN0.2 |
| In_c, Out_c, A_c | L1 | At cap c's cut: the cut cells inside pieces cover, those outside pieces cover, and those covered on both sides by one alignment. | manual 02:30-32 |
| N_c | L1 | A cap's misalignment: In_c + Out_c − 2·A_c. | manual 02:34 |
| Editable parameter | L2 | A value a tool's source declares editable. Changing it makes a new version of the tool with no inherited evidence. | EV3.7; LB6.3 |
| Worked instance | L2 | A small instance on the real geometry whose steps a person can check by hand from named givens. | LB6.1 |
| Admitted domain | L2 | A named set of inputs with a test for membership, such as D[k]. | ch06:82; stage 19 README:143-145 |
| Statement | L2 | A precise sentence about named objects that records can support or refute. | ch06:70-76 |

Foreign load, Clean grip and Tame are learner-facing to tool writers only. The L1 word for a grip is blocked or not.

**Not grounded, or only partly:**
- **Principle:** the integrator's working definition at ch03:413-414; no owner text.
- **Base tool:** I stop at LQ2.
- **Base part:** meaning inferred.
- **Solve record:** used, never defined.
- **Fixture:** generalised from uses.
- **Revision, Statement:** my wording.
- **Lemma, Theorem, Hypothesis, Invariant, Counterexample:** standard mathematics the sources use without defining.

#### 2. R4-M53

These are proposals, because final levels follow the owner's learning path (EV5.14).

| Term | Level | Text or note |
| --- | --- | --- |
| Height counts | stays L1 | Forces Height to L1 |
| Height | L6 to L1 | "How far a piece's pose lies from the lattice, on the stated side and menu. A jumble twist changes it by at most 2; a retained twist never does." (card 6:14, :26). The Γ(Λ) sentence moves to Free mode |
| N | stays L1 | "The total misalignment: how many times a cut cell, on any rotated cut, is covered on one side and not by the same alignment on the other." Forces Cut cell to L1. Alignment domain stays L6 (my choice) |
| Menu identity, Menu-order digest | L1 to L2 | Beside Identity block and Level-qualified digest |
| Menu letter | stays L1, forced by that move | "A grip and a menu-item number, meaningful only under its named menu and that menu's order." |
| Canonical ID | stays L1 | "An object's fixed identifier in its model, independent of how it is drawn." |
| Relation | stays L1 | "A stated connection between named objects in one state, such as which piece occupies a position." |
| Principle record | stays L2 | Text at HEAD is right. Forces Decomposition, Worked instance, Step map, Derivation and Leaf specification to L2 |
| Solving session | stays L2 | Drop "A profile belongs to a run." (Profile is L6). Forces Solve record to L2 |
| Assistance summary | stays L2 | Text at HEAD is right |
| Proposal | stays L2 | "A word, with a record of where it came from, submitted for a preview at a base revision." |
| Typed partial result | stays L1 | "A result that says which part it covers, which part is missing, or why it is undefined." |
| Availability | stays L1 | "Whether a command can run now, and the reason when it cannot." |
| Seat | stays L1 | "One of the four sets of twelve jumble twists at a cap; hold a seat, then press a twist key." (DT-M1:856-858; only if Q7 picks seats) |
| Relationship strip | stays L1 | "The strip that shows stated relations, in the grammar shared by the code and the theory book, inside Global and Local." |
| Preview | stays L2 | "What the engine says will happen if you commit: the exact word and what it does, fixed at one base revision." Complete effect stays L4 |
| Frame group | L5, settled at HEAD | Move "R_A/S_A" to the source column, because its parts are defined in L6 rows |
| Standing | L6 to L2, forced by Proved, Conditional, Finite instance | "Whether a statement's recorded arguments clear the threshold: first with no premise assumed, then with every premise assumed except those refuted in the first pass. It describes records, not truth." |
| Threshold | enters at L2 | Same force |
| Domain | stays L6 | Proved uses "admitted domain", so only the new Admitted domain row enters at L2 |
| Reachable by theorem, Promised reachability | L6 to L2 | Forced by the Reachability claim row at HEAD; their texts are fine |
| Layout | L6 to L2, forced by step 3 | "A saved arrangement of what a view shows; a program's layout can change only where its source declares it editable." |
| Round trip | stays L6 | The learner word at step 2 is Editable parameter |
| Grade, Key | stay L6 | |
| View | stays L1 | "A presentation of information at one revision." D8 and LQ6 go to the source column |
| Competence check | stays L1 | "An isolated task recording correctness, mistakes, help and transfer." LQ14 goes to the source column |

These are new, from a full pass over section 6 at 01263d68. They were not in R4-M53:

| Term | Level | Problem | Text |
| --- | --- | --- | --- |
| Lattice | L1 | K⁺ is undefined; this was my R4 wording | "Configurations where every piece's pose is a rotation symmetry of the 600-cell, so each piece fills a chamber and its stickers sit on slots." K⁺ goes to the source column (state contract:26; card 6:13) |
| Twist, 600-cell-Full | L1 | Uses Word (L2) | "... which can be a sequence of generators." |
| Generator | L1 | Uses alphabet (L2) | Last sentence: "Every count says what it counts." |
| Lifted | L1 | Uses Preview (L2) | "The lattice pieces a proposed operation takes off the lattice, distinct from its full moving set." |
| Unknown | L1 | Uses Evidence (L2) | "Not enough is known for the requested conclusion; never false or a numeric zero." |
| Overlay | L1 | Uses Display plan (L6) | "Declarative information attached to a view." |
| Evidence | L2 | Uses Author class (L6) | "The support for a scoped claim, separate from who wrote the tool, score and permission." |

#### 3. R4-M33

The word chosen is preference (owner 8B). Three things are correctness:
- One reference resolves to one concept (EV5.10).
- Each sense has its own row, so a script can tell them apart.
- A bare word is never used where two senses can occur.

| Word | Qualified names |
| --- | --- |
| Domain | **alignment domain**: a coset γK⁺, pieces whose poses fit one rotated copy of the lattice (card 6:14). **declared domain**: the inputs on which an operation or claim is defined. **admitted domain**: a declared domain with a membership test, such as D[k] (ch06:82). **chart domain**: where a chart's map is defined (ch10:260). Jumbling frame domain stays: an alignment domain at height 2 |
| Projection | **display projection**: a chart stage that draws the 4D geometry (RPI.31). **level projection**: the exact, possibly partial map to a coarser identity level (`project`, KN1.9); the label projection π is one |
| Key | **key**: on the keyboard. **statement key**: statement ID with scope (EVD.7). **equality key**: a word's normal form. **reproducibility key**: chapter 09. **source key**: the DisplayInstance field (ch10:187) |
| Checkpoint | **saved checkpoint**: a stored session point (`session.checkpoint.save`, KN4.1). **retained checkpoint** and **label checkpoint**: predicates on a state (A5) |
| Branch | **journal branch**: an engine-owned journal line. **tree branch**: the class b(A) of a piece's first tree step (card 6:14). Stage 19's "complete branch" stays research prose |
| Edition | **language edition**: a source or semantic version. **evidence edition**: the four versioned tables (EVD.14) |
| Rank | **descent rank**: a declared quantity with decrease obligations (ch03:330). **matrix rank**: linear algebra (ch03:68). **census rank**: the orbit field `rank` (census.json:10; I did not establish what it counts) |
| F symbols | F_x stays foreign load (card 6:20; card 7:82 writes F_c for it). The floor takes the word floor or another symbol (BND.10 and KN1.15 write F_c). Which symbol is preference; the collision is correctness. F_f, F2 and F5 keep their own rows |
| Clean | **clean grip**: S_d empty (card 6:17). **clean step**: a legal word landing a unit. Bare "clean" is never used. "Unblocked grip" is an option (preference) |
| Block | **achievement block**: solver-declared (terms.json:206). **jumbling block**: pieces no cut separates. Identity block and Provenance block keep their names, as bundles of fields. Bare "block" means achievement block only. Blocked is unrelated to all of these |
| Provenance | **assistance provenance**: the origin of a word (BM2, PS1.32). **result trace**: how a result traces to its inputs and support (EV2.5). **attribution**: author, reproducer, scope setter, review class (EVD.6; PS1.35 already says Attribution) |
| Macro | **macro**: a reusable named word. **lowering macro**: a word ending in a jumble that lowers Φ (stage 15 README:25). **macro certificate**: Theorem MC's event-by-event certificate of a word (stage 15 README:61) |

Five more collisions turned up under the same rule:
- **covers:** covered set (ch06:81) against helper cover (ch01:239, card 7:50).
- **level:** identity level, learning level, grip level lev(c) (card 7:47).
- **witness:** witness word against witness piece (ch10:148).
- **chamber:** cut chamber (tex:753-756) against flag chamber (tex:588-604).
- **base part:** decomposition leaf (LB6.1) against base-tier function (BS2).

#### 4. R4-M44 to R4-M48

I found no order error, so no step moves or splits. All 22 cited H-08 IDs exist in `command-table.json` at HEAD, but those rows are draft (`pending-2.2`). No fixture task exists yet, so every fixture is owed; candidates are mine.

| Step | Prerequisite concepts | Command routes | Fixture |
| --- | --- | --- | --- |
| 1 Use tools and read results | Command, Availability, Revision, Base revision, Stale; Claim, Scope, Covered set, Basis, Check, evidence words, Standing, Threshold, Premise; every, some and negation (Statement form, Negation pair), Witness; empty, not found, not run (EVD.33); Unknown against None: off the lattice; Typed partial result, Verdict; Job, Budget, Job result; Reachability claim; Evidence graph on request; record parts `specification` and `not meant` | H-08 `command.find`, `object.inspect`, `view.findings`, `view.residuals`, `operation.check`, `job.stop`. `invoke` on an overlay row (ch01:371), `word` (ch06:277), `follow_pick` (ch04:245). Explanation and provenance view: owed | Owed. Candidate: read E3's 7302 and 7303 records (ch06:445-448) |
| 2 Change a parameter | Editable parameter, new version, Fork, "a fork inherits no evidence", Diagnostic, Typed error | `edit_declared_part` (ch04:249), `diagnose` (ch04:45). Workspace command that persists it, host fork operation: owed | Owed |
| 3 Save layouts and bindings | Layout, Workspace record, Binding with absolute arguments, Routing class, Preference, workspace undo | H-08 `keymap.edit`, `file.keymap.export`, `file.keymap.check`, `file.keymap.apply`, `grip.capture`. `edit_declared_part`. Save layout: owed | Owed |
| 4 Compose recipes and macros | Word, Literal word, Equality key, Cost alphabet, Setup, Star, conjugate and commutator (my inference), Recipe, Macro, Macro base, Proposal, Preview, User-action token, Protection, Net and Strict, Complete effect, assistance provenance, Recording, fresh preview on reuse (LB7.2) | H-08 `macro.base.open`, `macro.select`, `macro.add`, `macro.save`, `macro.variant.create`, `macro.compare`, `operation.new`, `operation.phase.edit`, `operation.reuse`, `operation.check`, `operation.preview`, `operation.execute`, `worksheet.save`, `worksheet.load`. `propose` (ch01:409), `word_key` (ch02:167). Recording to macro: owed | Owed |
| 5 Read a record, state your principle | Principle, Principle record, Hypothesis, Lemma, Theorem as a proof, Invariant, Counterexample, mixed statements, Decomposition, Base part, Step map, Leaf specification, Worked instance, Derivation, Fixture, a domain inside its lemma's scope | `hover`, `definition` (ch04:69, :72). Reading a record: owed. Stating your own: owed; no data route exists, and EV5.9 leaves the map to the joint setting | Owed. Candidate: the floor instance in LB6.2 |
| 6 Write tools | Module, Module manifest, Lockfile, language edition; exact against approximate; Identity level typing; Snapshot, State reference, Derived state; partial results and verdicts in code; budgets, jobs, Cost profile; Effect, Capability declaration, Profile, Author class, Provenance block; view nodes and overlays; boundary: KN2.5, Flow rule, Declassifier, why a preview is refused; stabiliser, coset, quotient, homomorphism, F2, F5, Q(√5), whose text depends on EV5.17 | `parse`, `complete`, `diagnose`, `hover`, `definition`, `rename` (ch04:39-75), `debug_step`, `inspect_value` (ch04:125, :129), `open_snapshot`, `project`, `derive`, `propose` (ch01:213, :149, :278, :409), `publish_view` (ch10:107). Create a module: owed | Owed |
| 7 Maintain modules and evidence | Digest, Reproducibility key, Run record, Deterministic replay, Certificate, Check report, Per-claim trace, Grade, Alternative, Check mode, Statement ID, Key, Evidence envelope, Exact transport, evidence edition | `replay_run` (ch04:149), `reproduce` (ch09:96), `check` (ch01:501), `word` (ch06:277), `import_archive`, `export_check_bundle`, `migrate_record` (ch05:798, :804, :612) | Owed |

**Placements that may not be obvious:**
- `specification` and `not meant` are read at step 1, because empty against not found cannot be read without them. The whole record is step 5.
- Basis and Check sit at step 1, because EV5.13 shows the check beside the word.
- Partial results, verdicts, budgets and jobs are read at 1 and written at 6.
- Mixed statements and Invariant sit at 5.

**One text fix:** EV5.6 (ch04:457) still lists six steps. Replace the list with: "use tools and read their results; change a parameter; save layouts and bindings; compose recipes and macros; read a principle record and state your tool's principle; write tools; maintain modules and evidence."

#### 5. R4-M39

**Text**

ch06:72 becomes:

```text
form:      finite | universal | existence | mixed(outer: forall | exists),
```

Add to EVD.9: "A statement whose `covers` is not one_instance is never of form finite. A mixed statement names its outer quantifier, and its negation has the other one."

Add to the block at ch06:108-119:

```text
Leaf    = { grade, identity, covered: Covered? }
Covered = { covers: a value of Scope.covers, input: Digest<level>? }
```

New clause after EVD.13, next free number:

"**EVD.59 [D]** `covered` is computed from the leaf's own record, never copied from the statement. Identical means equal field by field, and digests at different levels are never identical (KN1.8)."

| Basis | `covered` |
| --- | --- |
| Exhaustive computation | `SearchRecord.declared` as search_set(family, alphabet, bounds), with the job's input digest |
| Exact computation | The job's input: one_instance with its digest, or search_set for a declared list |
| Certificate | The certificate's stated inputs (KN6.6, EVD.46); each checker kind states them |
| Theorem, sample | Not read |

New row after I6:

| Case | Inconsistency |
| --- | --- |
| I7 | (a) A leaf of basis certificate, exhaustive computation or exact computation in an argument of a universal statement, or of a mixed statement with outer quantifier universal, whose `covered` is not identical to the statement's `covers` and `input`. In a reused leaf (EVD.4) the source leaf is compared after the transport lemma's checked relation. (b) A leaf of basis exhaustive computation or exact computation whose `covered` is neither one_instance nor search_set. (c) A statement of form finite whose `covers` is not one_instance. |

After the table: "A leaf is in an argument of S when it is the direct child of S's OR node, or a child of an AND node under it. A child reached through a premise or another key belongs to that key's statement."

Add to EVD.41: "Two further absences are no contradiction. A leaf of basis certificate, exhaustive computation or exact computation with no `covered` fact, in an argument I7 (a) reads, is inert. A statement with no `form` fact counts as universal for I7 (a), and a leaf that I7 (a) would flag there is inert instead. An inert leaf counts at no threshold, and the word reads as it would without it."

Add to EVD.17: "A finite computation that serves a proof of a universal statement has its own statement and key. It enters the universal's argument as a premise discharged by that key, beside a theorem leaf saying why it suffices. A bridge lemma is such a theorem."

Add to EVD.18: "A leaf that would meet I7 is refused at entry with `EVD:evidence.scope_mismatch`. I7 is what a stored or imported record reads, because arguments are immutable (EVD.16)."

**Where the fact lives**
- **Record and field:** `Leaf.covered`, carried in the `EvidenceEnvelope` facts payload (PS1.36, ch05:484).
- **PS1.35 table, new row:** `| Covered set of a leaf | `covered` of chapter 06, computed from the leaf's own record (EVD.59) |`
- **EVD.13, new row:** `| Covered set of a leaf | A finite computation can otherwise stand for a wider statement (I7) |`
- **LQ10:** under the judgement alternative I7 needs the same fact in whatever stores the justification. I stop there.

**Theorem D**
- (a) and (b) are unchanged. I7 reads only `covered`, `covers`, `input` and `form` on Reach(S).
- New, after (b):

  "(b2) Narrower support. Let S be universal, mixed with outer quantifier universal, or without a `form` fact, and let 𝒰 ⊆ 𝒰₀. A leaf of basis certificate, exhaustive computation or exact computation in an argument of S, whose `covered` is absent or, as I7 compares it, not identical to S's `covers` and `input`, never moves S from rows 5 to 7 into rows 1 to 4. The same holds for refuting an existence statement, whose negation is universal. *Proof.* If S has a form and `covered` is present, I7 holds, and row 0 comes first in the decision list. Otherwise the leaf is inert (EVD.41), so it is false at ω₀ and ω₁ as a sample leaf is, and it changes neither pass, nor Ref. Rows 1 to 4 read only the passes. ∎ (b2) concerns recorded fields: a false `covered` is a false record, which no word can detect, and the check field is the defence."
- (c1), last two sentences become: "The full-family statement has a different key. I7 keeps a leaf of the narrower search out of its arguments, so the only edge to it is a bridge lemma with its own ID (stage 16 `PROOFS.md`, LB5). An upgrade happens only by a proof."
- (c2) and (c3) are unchanged.

**Other edits**
- Row 0, ch06:255: "I1 to I7".
- EVD.40, ch06:292: "I1 to I4, I6 and I7 read".
- ch06:474: "I1 to I4, I6 or I7".
- CT-EVD-09, ch06:496: "I1 to I7".
- New CT-EVD-22: a narrower leaf reads row 0 or is refused at entry, an absent `covered` is inert, and a mixed(exists) refutation passes.
- The fact count changes at EVD.13, EVD.55, partition:264 and ADR-L05:29; see item 6 for the number.

**Why these limits**
- Theorem leaves are excluded because a wider theorem reaches a narrower statement through EVD.17, and comparing would flag every instantiation.
- Sample leaves are excluded because they sit below every threshold.
- Existence is excluded because one witness entails it (EVD.44).
- Without `outer`, EVD.42's refutation of a mixed statement and CT-EVD-11 break, because neg S rests on a witness plus a complete negative at x.
- The test is "identical", not "contained", because inclusion of search sets and domains is not decidable in general (EVD.8's safe direction).
- (b) spares certificates, because a closure-set certificate may cover more (EVD.43).
- Counterexample to today's text (mine): the two-letter exhaustive leaf attached to the three-letter negative reads row 2.

**Checks: all hold**
- **E1:** S₁ is finite, so I7 is not read; finite instance stands.
- **E2:** implication and U(h, k) have theorem leaves, not read. T(Z) and ¬Adm_k(Z) are finite and one_instance, so (c) passes. A macro at one state, offered for Sel, is flagged or refused, which enforces ch06:437.
- **E3:**
  - Beam: no leaf, row 5.
  - Two-letter set: `covered` equals its `covers`, row 2.
  - Unbeamed search: witness under an existence statement, not read, S₄ refuted.
  - 7303: `covered` equals `covers`, row 2.
  - 7305 and 7306: no leaf, row 6.
  - Φ 498: identical for the five families, and "no word descends there" stays open.
- **EVD.21:** unchanged and still empty. (exact computation, producer) is valid in E1's a₂ and wrong only relative to a statement, so a pair table cannot hold it.
- **EVD.32:** conditions of rows 1 to 7 are unchanged and row 0's range extends. The list stays total and single-valued. CT-EVD-01's 504 count is unchanged, because I7 reads leaf facts, not node-level cases.
- **EVD.42 to EVD.44, EVD.4, EVD.18:** hold. A witness refuting a universal sits under neg S, which is an existence statement, so I7 is not read.

**Import**
- Counted at HEAD from the table's `code` strings, which agree with its `mapping` lines:

  | Basis | Rows | Scope in those rows |
  | --- | --- | --- |
  | Exhaustive computation | 56 | declared search set in all 56 |
  | Exact computation | 64 | one finite instance in 46, declared search set in 18 |
  | Certificate | 4 | one finite instance in 3, no scope in 1 |
  | Theorem | 10 | no scope |

- No row sets admitted domain, every reachable state or sample. So I7 (b) has no trigger on a labelled row, and EVD.59's table fits every labelled pair.
- I7 (a) cannot be tested on the table, which has no `form` and no statement's `covers`.
- I did not read `rule.txt` in full, so a whole-record rule may add to these counts.
- Inference: on import `covers` and `covered` are typed from the same record, so (a) bites only when a record is attached to a statement built elsewhere. No importer exists to confirm this.
- Existing gap, not created by I7: `form` has no producer in the label table, though EVD.42 and I6 read it too. CT-EVD-12 therefore needs the forms of E1 to E3 from a whole-record rule or from the test's own statements.

#### 6. R4-M38

Keep EVD.36 as it stands at HEAD and append:

"The phrase names the covered set and every *guard* of the statement. A guard is a hypothesis that meets three conditions: the statement's own text holds it, so it is no node of the graph; the membership test of `covers` does not decide it; and applying the statement to an instance must discharge it as a premise (EVD.17). A guard reads by its premise class: a reachability promise as "for reachable inputs", an open hypothesis as "given" and its name, a model assumption as "under" and its name. A hypothesis the membership test decides is not a guard, because the set's name already says it. A premise that is a node of the graph is not a guard, because row 4 names it. Where the covered set is an admitted domain or every reachable state, a statement whose guards cannot be typed keeps the row recorded."

**Supporting edits**
- Add `guards: List<{ class, name }>,` to Statement.
- EVD.8: "It binds every parameter, every guard, the puzzle and the level."
- `scope_phrase` is computed from `covers` and `guards`.
- EVD.13 gains `| Guards of a statement | "Proved on D[k]" would otherwise hide a reachability promise (EVD.36) |`.
- With item 5 that makes nine facts: "the seven R3-M1 named and two that round 4 added", and "nine" in EVD.55, partition:264 and ADR-L05:29.
- Add a guard case to CT-EVD-15.
- No label row sets those two scopes, so the import is unaffected today.

**Needs it:** U(h, k) on D[k].
- The admission test takes a promised-reachable input (stage 19 README:143-145) and does not decide reachability (:152). T(Z)'s reach(Z) in Theorem D (c2) is exactly this guard.
- The phrase is "proved on D[k], for reachable inputs". Use it at ch06:258 and ch06:433.
- A concrete case, my inference because I did not read the stage 19 records: a lattice table with two O0 pieces swapped.

**Does not need it:** the 7303 negative (ch06:448). Input, family R and three letters are all in `covers`, and it speaks of that input as given. Its phrase stays "finite instance, three letters of family R only".

**Consequence:** the fixed-L implication holds Sel in its text (stage 19 README:100-102), so ch06:431 reads "proved, given Sel". The bound's premise node stays row 4.

#### 7. R4-M43

**Settled: no.** I read chapter 10 in full at d97240d8, and the 01263d68 changes do not touch this question.
- `ViewNode`, `DisplayPlan`, `DisplayInstance` and `SourceMap` have no statement field (ch10:45-54, :63-76, :179-189, :198-203).
- Links exist only for relation overlays (:149), certified overlays (:148), derived-state reachability (:151), chart properties (:264) and the strip's contract (EV3.8).
- `content` can hold any typed data, including the text "proved", and chapter 10 never mentions `word()`.

Add to `ViewNode`:

```text
claims:       List<Key>                   # chapter 06; empty when the node states no claim
```

New clause, placed after RPI.5:

"**RPI.65 [D]** Content shown in a view is one of three kinds. *State data* is what a kernel row returned for the subject at `base`: labels, poses, exact sets, counts, partial results and verdicts, with any H-08 evidence state as the engine gave it. Its support is that revision and that row, so it needs no statement reference. A *claim* is a relation (EV3.8), a declared property, or anything shown with a word of EVD.32. It carries its `Key` in `claims`. *Program content* is everything else a program publishes. It is shown as the output of its producing module and carries no word of EVD.32. A word of EVD.32 reaches a display only through `word()` on a key, with the phrase of EVD.36, and a view asks for it only on a key that has a record: with none, row 7 would call a kernel read open. A node of kind relation with empty `claims` is refused with `EV3:input.display_plan_invalid`. Text a program writes stays the program's text, whatever it says: the host does not read it for evidence. How the three kinds look is not set here. Under LQ3's alternatives the function changes, not this rule."

**Supporting edits**
- ch10:462 meaning gains "or a relation node with no statement reference".
- `claims` is where EV3.8's typed statement reference travels.
- Add a CT-RPI case.

#### Corrections to my R4 wording

1. R4-M26: "A profile belongs to a run." is above L2 and must leave the Solving session row (at HEAD).
2. R4-M37, R4-M28: Proved, Conditional and Finite instance use stands, threshold and admitted domain at L2; item 2's moves resolve this.
3. R4-M25: the Standing text uses grade and edition; item 2 rewords it for L2.
4. R4-M35: the Γ(Λ) sentence cannot sit in an L1 Height row; it moves to Free mode.
5. R4-M12: limited to EVD.32's words, because H-08's four states come straight from the engine (the command-layer row of ch06's vocabulary table).
6. R4-M39: "smaller than" becomes "not identical", and "universal or mixed" splits by outer quantifier.
7. My Lattice row used K⁺ at L1 (at HEAD); item 2 fixes it.
8. My first Floor draft said "the least twists can reach", which is false: it is a bound that need not be attained. The row above says so.

**Found in passing:** ch06:61 cites EVD.12 for completeness of a search; the fact table is EVD.13.

**Owner:** none of the seven drafts needs an owner answer as written. They stop at LQ2 (Base tool, the principle map of a solver's tool), Q7 (Seat), LQ10, LQ14, LQ16 and EV5.17. The level moves stay proposals under EV5.14.

Files, all under `docs\`:
- `architecture\1.0\language\architecture\03-libraries.md`
- `architecture\1.0\language\architecture\04-environment.md`
- `architecture\1.0\language\architecture\05-persistence.md`
- `architecture\1.0\language\architecture\06-evidence.md`
- `architecture\1.0\language\architecture\10-renderer-platform.md`
- `architecture\1.0\language\ecosystem-partition.md`
- `architecture\1.0\language\adr\ADR-L05-evidence.md`
- `architecture\1.0\language\evidence-import\corpus-labels.json`
- `progress\1.0\command-table.json`

### Outcome of round 4

Settled by the main session on 4 November 2026. Nothing here is an owner decision. The verdict stands as the record gives
it: the core is sound (LB6.2, EV5.3, EV5.4, BND.12, BND.13, BND.72 and Theorem D) and the text around it needed
correcting. The totals are 14 blockers, 29 major, 11 minor and 1 nit. No correction chooses an option under LQ1 to LQ21.

#### Dispositions

Every card is adopted, except where the table says otherwise. Partition section 19 records what changed on that page.

| Card | Severity | Disposition | Applied in |
| --- | --- | --- | --- |
| R4-M1 | blocker | Adopted | LB6.1, LB6.2 (`givens` and `steps`), CT-LB6-04, partition LB6 row. Any ceiling on the step count stays with the owner |
| R4-M2 | major | Adopted. The sentence naming the principle stays a working definition | LB6.2 |
| R4-M3 to R4-M7 | major | Adopted | LB6.2 |
| R4-M8, R4-M9 | minor | Adopted | LB6.2, EVD.43 |
| R4-M10 | blocker | Adopted | INV-12, EV5.1, EV5.16 |
| R4-M11 | blocker | Adopted | BND.82, BM4, BND.35, BND.70, BS1 to BS4, the Assistance summary row |
| R4-M12 | blocker | Adopted. The clause the card proposed is RPI.65, limited to EVD.32's words, because H-08's four states come straight from the engine | Overview, RPI.65 |
| R4-M13 | blocker | Adopted | INV-12, LB6.1 |
| R4-M14 | major | Adopted | Chapters 03 and 04 |
| R4-M15 | major | Adopted | BS1 to BS4, BND.23, LB7.3 |
| R4-M16 | major | Adopted. Any map from a tool to a principle stays with LQ2 | EV5.9 |
| R4-M17 | major | Adopted | INV-12, the LB6 and EV5 rows, the Principle record row, CT-EV5-01, CT-EV5-05 |
| R4-M18 | major | Adopted | LB6.2 |
| R4-M19 | major | Adopted | LB6.1, EV5.3, EV5.16, partition 1.2 |
| R4-M20 | minor | Adopted | BND.72, BND.73 |
| R4-M21 to R4-M27 | blocker | Adopted | Chapter 04's glossary rows, EV3.8 |
| R4-M28 | blocker | Adopted, with one row reworded (below) | Chapter 04's glossary rows |
| R4-M29 | blocker | Adopted | Rim summary row |
| R4-M30 to R4-M32 | major | Adopted | Chapter 04's glossary rows |
| R4-M33 | major | Adopted through the follow-up. The qualified names are proposals, and the word chosen is preference | EV5.18, the renamed glossary rows, KN1.15, BND.10 |
| R4-M34 | minor | Adopted for Lattice, Invalid and Grip. Tree, Section, Action, Embedding, Computed and Verified are deferred: their definitions come from the chapters that own them, and nothing here invents them | Chapter 04's glossary rows |
| R4-M35, R4-M36 | minor | Adopted | Chapter 04's glossary rows |
| R4-M37 | blocker | Adopted. Raising the threshold stays with LQ3 | EV5.13, the Proved row |
| R4-M38 | major | Adopted through the follow-up: a statement names its guards | EVD.8, EVD.13, EVD.36, section 9 example E2, PS1.35, CT-EVD-15 |
| R4-M39 | major | Adopted through the follow-up: a leaf's `covered`, the inconsistency I7 and Theorem D (b2) | EVD.9, EVD.13, EVD.17, EVD.18, EVD.32, EVD.40 to EVD.42, EVD.59, Theorem D, PS1.35, CT-EVD-22 |
| R4-M40 | major | Adopted | EV5.3 |
| R4-M41 | major | Adopted | EVD.36 |
| R4-M42 | minor | Adopted | EV2.5 |
| R4-M43 | minor | Adopted through the follow-up: `claims` on `ViewNode` and RPI.65 | RPI.5, RPI.65, EV3.8, CT-RPI-27, chapter 13's `display_plan_invalid` row |
| R4-M44 to R4-M48 | major | Adopted through the follow-up. No step moves or splits, and a table gives each step's concepts, routes and fixture | EV5.6, CT-EV5-02, partition EV5 row |
| R4-M49 | major | Adopted as an open item for the owner | EV5.17, LQ16 |
| R4-M50 | major | Adopted | EV5.14 |
| R4-M51 | minor | Adopted | CT-EV5-02 |
| R4-M52, R4-M53 | major | Adopted through the follow-up, as proposals under EV5.14 | Chapter 04's glossary rows, EV5.14 |
| R4-M54 | minor | Adopted as a proposal for LP4 | EV5.10 |
| R4-M55 | nit | Adopted where the text was wrong. "Base" stops at LQ2, and EV5.7 keeps the owner's "independent completion", with BND.72 saying what the software sees | Partition section 11, EVD.18 |

The "Not found within budget" row differs from R4-M28. The card's "It gives no negative at all, even inside that set."
would contradict DET.30, which lets a part examined completely carry a narrower claim of its own. The row now reads "It
gives no negative over that set; only a part it examined completely can carry a narrower claim of its own."

#### What the main session changed in the follow-up

The mathematics of the seven drafts was checked against their sources and holds. The mathematician's eight corrections to
its own round 4 wording are listed in its reply. The citation at EVD.5 now reads EVD.13, as it found. The learning path's
H-08 citations number 24 distinct IDs by a separate count, not the 22 the reply gives, and all of them exist. The label
table's counts in the reply's import section were confirmed by a separate count of `corpus-labels.json`. These changes
are the main session's own:

| Change | Why | Applied in |
| --- | --- | --- |
| A guards row beside the covered-set row | The envelope has to hold every fact EVD.13 names | PS1.35 |
| The mixed-statement sentences split by outer quantifier | Row 1 reads the negation. A statement with an outer universal is refuted through a mixed negation, which rests on a witness plus a complete negative; a statement with an outer existential has a negation that I7 (a) reads, so it needs a proof or a complete certificate | EVD.42, CT-EVD-11, partition 8.1 |
| The floor is written as a word | F_c meant the floor in BND.10 and KN1.15 and foreign load in card 7. DT-L1 and the jumbling observation keep their F_c as records | KN1.15, BND.10, the F symbols row of EV5.18 |
| Height's row names its menu | Card 6 states the height facts for menu I_a | The Height row |
| "Hand derivation" for the reply's "Derivation" | KN2.11 and RPI.6 use "derivation" for a derived state | The Hand derivation and Principle record rows, the Derivation row of EV5.18 |
| "Keep their own meanings" for the reply's "keep their own rows" | `terms.json` has no row for F_f, F2 or F5 | The F symbols row of EV5.18 |
| An Alignment domain row at L6 | The reply names the sense and keeps it at L6 without drafting a row | Chapter 04's glossary rows |
| The covered-set rule in the record of the decision | So that accepting ADR-L05 covers EVD.59 and I7 | ADR-L05.7 and its overrule list, with the guards of EVD.36 |

#### The source check

Beside round 4, the text was read against a list of problems another session raised while the chapters were being
written. Most of the list was already answered by the text or is an open item with an owner. Five points were not:

| Finding | Applied in |
| --- | --- |
| A live twist has no preview, while INV-3 spoke only of a fresh one. The reading of charter S4 goes to the owner | INV-3, ONE.78, BND.5, ADR-L01.8 |
| Three chapters read a derived state's base revision differently. A value's lineage and a plan's base are now separate | ONE.21, EV3.4, EV3.12 (now [D]), RPI.7, ADR-L11.3 |
| KN2.9 and KN7.3 did not state R-08's distinction: a read result may repeat when a digest returns, a permission never | KN2.9, KN7.3 |
| The stage 15 row promised one certificate per admissible word, and `pstar2-*` is no format name | Partition 10.3 |
| Partition 8.1 mixed the bases of two counts and said that label families map directly | Partition 8.1 |

#### Questions for the owner

These go to the owner once, with the topic. None adds a question to partition section 15.

| Card | Question | What the owner weighs |
| --- | --- | --- |
| R4-M1 | Boundary, with LQ14 | Whether a hand-checkable instance has a ceiling on its step count |
| R4-M16, R4-M55 | LQ2 | Which principle a solver's tool embodies, and what "built-in base tools" covers |
| R4-M37 | LQ3 | Whether "proved" needs a higher threshold than a recorded argument at (theorem, producer) |
| R4-M20 | LQ2, LQ14 | How the boundary treats an "understanding" the software cannot see |
| R4-M49 | LQ16 | Whether the language's libraries count as shown, so that the theory book covers them |
| R4-M50 | Owner's call | Whether the shelved tutorial's chapter order may inform the first-use order of the jumbling terms |
| Record, question 1 | LQ16 | Whether a research tier with WC-PLAN ships. A principle record could not admit it as it stands |

#### Records

- The record's numbers come from the cited records and the mathematician's arithmetic on them. It ran no engine
  computation, no test and no geometry.
- The mathematician's explainer "For the owner" goes to the owner with the full text.
- The corrections are listed in partition section 19, "Corrected after round 3".

## Astra review of the full text

The kickoff asks for a full Astra review of the whole proposal in at least two parallel shards. It ran at `7927cb30`,
read-only and at the fast tier, as three concurrent shards. Shard A took chapters 01, 02, 08, 09 and 13 with ADR-L01, L02,
L03, L04, L07 and L08. Shard B took chapters 05, 06 and 07, the evidence import table, and ADR-L05, L06, L09 and L10. Shard
C took chapters 03, 04, 10, 11 and 12, ADR-L11 and L12, the ADR index and the kickoff's coverage table. Their records are
`20261104T045920Z-f29d6171`, `20261104T045921Z-6618a6a2` and `20261104T045922Z-85b33949` under `work/reviews/`, which Git
ignores.

The result was five major and seven minor findings, and no blocker. Each was read against the text it cites. All twelve
stand, and all twelve are adopted, the minor ones too, because each fix is small and local. Shard C's verdict was a pass,
and shard B reproduced the corpus counts from Git blobs.

| Finding | Severity | What was wrong | Applied in |
| --- | --- | --- | --- |
| C600-A-01 | major | Availability changes with no new revision: a preview reaching `presented`, a lane's admission or release. A palette reading once per revision would show a stale table | KN5.25 (new), KN5.6, EV4.2, chapter 08's lifecycle table, CT-KN5-12, partition KN5, ADR-L02 |
| C600-A-02 | major | Shape as a set fails on raw tables that collide, which KN2.14 admits: (R, R, QR) and (R, QR, QR) give one set, different off counts and no gauge between them | KN1.7, KN2.14, shape's glossary row, CT-KN1-06, ADR-L04 |
| C600-A-03 | major | ONE.27 and ADR-L01.4 said a wordless import can be compared and displayed "and nothing more", while ONE.21 and KN2.11 let `derive` start from it | ONE.27, ADR-L01.4, CT-ONE-28 |
| B-01 | major | Theorem D (b) said a job outcome never moves a statement out of rows 5 to 7. EVD.41 makes a leaf inert under "not stated", so a stated outcome can release a leaf the graph already holds: row 7 to row 2 | Theorem D (b) narrowed, (b1) new, EVD.5, EVD.41, CT-EVD-05, ADR-L05 |
| B-02 | major | `AssistanceSummary` reads run records, while BND.35 promised equal bytes from the journal alone. A view-only run leaves no journal row | The summary's `runs` field, BND.35, CT-BND-21, ADR-L09.4 |
| C600-A-04 | minor | A local wildcard for another component's category expanded to nothing, so ERR.4's expansion rule failed on it | ERR.4, CT-ERR-04, and the `errors:` lines of chapters 01, 02, 03 and 09 |
| B-03 | minor | ADR-L05.8 forbade all code sharing between a checker and its producer. KN6.2 forbids only search and construction code | ADR-L05.8 |
| B-04 | minor | Rule R6 mapped `pxrr-7304.json` to cancelled, while EVD.47 calls it "not stated" and only the README says it was stopped | Rule R6, the generator's legend, the table and its digest, EVD.47, CT-EVD-23 |
| B-05 | minor | CT-EVD-04 said neither pass reads the other's output, but stands₁ reads Ref, which comes from stands₀ | CT-EVD-04 |
| B-06 | minor | The table's check mode "stated: not replayed" was not among EVD.4's modes | EVD.4, EVD.47, the import README, CT-EVD-23 |
| C-001 | minor | The map property row said one pass implies no other property, which is false: an isometry or an embedding is injective | The row in chapter 03, section 4 |
| C-002 | minor | The term "covers" merged the cap cover "d covers c" with card 7's helper cover | The confusable-terms row of chapter 04 |

Three of these change something a round had settled, and the owner sees them with the integrator additions:

- **Note N1.** Shape is a multiset. For a collision-free configuration that is the set note N1 writes, so Theorem 1 and every
  use on a reachable state stand. The note's wording is corrected in chapter 01 and stays as written in round 3 above.
- **Note N2.** Theorem D (b) as written in round 3 is refuted, and (b1) replaces its claim about job outcomes. The review
  gives no verdict on the proofs as a whole, so N1 and N2 stay at (theorem, producer), and no person has reviewed either.
- **The label table.** Rule R6's rows change value, so the table's digest changes. The digest in round 3's record stays as
  history, and the import README carries the current one. The corpus manifest and every total are unchanged.

New requirements: CT-KN1-06, CT-KN5-12, CT-ONE-28 and CT-EVD-23, which makes 373 in all. No test exists yet.

### Scoped verification

One round ran on `3e3bf96e`, read-only, with Astra at the fast tier (`20261104T053645Z-2d8c9814` under `work/reviews/`). It
checked two things only: whether each of the five major findings is fixed, and whether the fix delta from `7927cb30` adds
a new blocker or major.

The verdict is a pass, with no finding. C600-A-01, C600-A-02, C600-A-03, B-01 and B-02 are each fixed, and the delta adds
no blocker or major. So the full text has no open blocker or major, which is the review half of LP0's acceptance.

This is document evidence only. It reads the text, runs nothing, and proves no conformance test passes: none exists yet.
The other half of LP0's acceptance, the owner's acceptance of the ADR drafts, is open.
