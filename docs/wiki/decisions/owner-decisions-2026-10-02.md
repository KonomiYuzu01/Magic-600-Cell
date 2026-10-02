---
id: owner-decisions-2026-10-02
type: decision
status: verified
visibility: public
summary: Owner decisions of 2 October 2026 - working-day schedule, later macOS and Linux versions, Taste Lab on a private Artifact page with a learning model, the 1.0 product thesis, target users and human-solve boundary, and several themes on one design language.
related: [owner-decisions-2026-10-01, owner-decisions-2026-09-29, renderer-candidates]
supersedes: []
claims:
  - {id: working-day-schedule, evidence_kind: decision, checked_at: 2026-10-02}
  - {id: future-macos-linux, evidence_kind: decision, checked_at: 2026-10-02}
  - {id: windows-first-line, evidence_kind: source, path: docs/architecture/1.0/10_V1_ARCHITECTURE.md, line: 748, checked_at: 2026-10-02}
  - {id: taste-lab-artifact, evidence_kind: decision, checked_at: 2026-10-02}
  - {id: product-thesis-and-boundary, evidence_kind: decision, checked_at: 2026-10-02}
  - {id: themes-one-language, evidence_kind: decision, checked_at: 2026-10-02}
---

# Owner decisions, 2 October 2026

These decisions replace the calendar-day count and the no-extension rule of the [1 October](owner-decisions-2026-10-01.md) "Stage 2 within 20 days" decision, add a 1.0 architecture constraint, settle the Taste Lab plan, state the 1.0 product thesis and human-solve boundary, and replace the "themes are undecided" line of [29 September](owner-decisions-2026-09-29.md).

## Stage 2 days are working days, and exit days are targets

The owner has other commitments and asked that the schedule count working days and not be held too tight.

- **Day numbers count the owner's working days, not calendar days.** A day counts when the owner works on the project that day; a day the owner does not work does not count. The owner can correct the count at any time. Day 1 was 1 October 2026. The owner did not work on 2 October, so the next working day is day 2. Calendar dates in the stage 2 plans are no longer deadlines.
- **Exit days are targets, not hard limits.** When a task is likely to miss its target day, the integrator says so once, with a short re-plan (move the target, cut the task down, or drop it), and the owner chooses. Nothing is cut or extended automatically, and the owner is not chased for a date.
- **The plan keeps its size and order.** Stage 2 is still planned as about 20 working days with the same sub-stages and handoffs, the bare Direct3D 12 probe first, the go/no-go on window day 7 and the 12-day renderer window, all counted in working days.
- **Relaxing time never relaxes a gate.** The [renderer selection gate](renderer-candidates.md), the Astra gate rulings, the differential oracle, the review rules and every owner sign-off are unchanged. A late result is a late result, not a pass.
- Agent work that needs no owner input (for example the inventory shards and reviews) may run on any day. It does not advance the day count, and anything that needs the owner, the owner's computer or a sign-off waits for a working day.

## Future macOS and Linux versions

The owner plans macOS and Linux versions after 1.0 and asked for the cheapest path. 1.0 still ships on Windows first, and Direct3D 12 stays the base of the dedicated renderer ([29 September](owner-decisions-2026-09-29.md)). This restores the intent of the 1.0 architecture document's "Windows first; Linux/macOS later" (line 748), which survives the superseded egui/wgpu stack choice on line 746. The 1.0 architecture keeps a later port cheap:

- **Platform separation.** The engine, session store, command layer and view model contain no Windows-only code or APIs. Windows-specific code (windowing, file locking, process ownership, paths, Direct3D 12) sits behind narrow platform interfaces.
- **One shader source.** Shaders are written once, in HLSL, and compiled for Direct3D 12 now. The source stays portable so that it can later be compiled to SPIR-V for Vulkan (Linux) and translated to Metal (macOS) without a rewrite.
- **A small renderer backend interface.** The renderer keeps a thin internal backend interface, so that a Vulkan or Metal backend can be added later without changing the layers above it.
- **No new Windows lock-in.** A new Windows-only dependency outside the platform layer needs a recorded reason and a named replacement path for macOS and Linux.
- **Headless checks stay portable.** The platform-independent layers and the differential oracle keep running headless on Linux and macOS, as the cloud sessions already do on Linux. Whether to add macOS runs to continuous integration is decided when CI is set up, since it may carry a cost.

Out of scope for 1.0: building, testing or releasing a macOS or Linux version. This constraint takes no time from the stage 2 schedule and does not change the renderer selection gate; the stage 2.5 architecture freeze checks it. The 0.4 migration and the H6 clean start stay Windows-specific, because 0.4 ran only on Windows. Evidence rules are unchanged: a headless or Linux result is never Windows/DirectX, macOS or performance evidence.

## Taste Lab: build now, on a private Artifact page, with a learning model

The owner chose to build Taste Lab now (option (a) of the re-plan), with two changes to the 30 September design:

- **The front end is a private Artifact page on claude.ai**, not a local window. Choosing it approves putting exactly this on claude.ai: the page code, thumbnails of openly licensed (class A) images with their source, licence and author, the embeddings of those images as numbers, and the owner's choices, ratings and notes in the page's private storage. Copyrighted (class B) images, anything from the owner's sessions, keys and benchmark works never go there; anything else needs a new approval. Only code is committed; data copies stay in `work/loop-memory/`.
- **The model learns from few choices and draws new variants itself.** A parameter studio renders looks of the 600-cell in the browser, two at a time; the owner picks one, and a Gaussian-process preference model chooses the next pair (preferential Bayesian optimization). Hard checks (adjacent-cell colour difference, colour-vision simulation, parameter ranges) run before a look is shown. The image half keeps like and dislike ratings on class A images.
- Phase 1 (the page) needs no install or download on the owner's computer. Phase 2, a local tool for class B images, is optional and needs the installation approval of the original design.
- Results are starting points for the design track: G3 repeats the search in Look Lab at full detail with the cost table. The model assists; the owner decides.

## 1.0 product thesis, target users and human-solve boundary

From the owner's Taste Lab notes and clarifications; the full wording is in [charter section 5](../../progress/1.0/charter-2.0-draft.md), which the owner signs.

- **Thesis.** A complete 600-cell solve is harder in practice than its mathematics suggests. 1.0 gives tools that compensate for the complexity while the result stays a human solve with appropriate tools. Once learned, it is as efficient as a keyboard-driven office application (reference: the Hyperspeedcube keybind workflow); learning it feels like progressing through a game with rewards for real progress. The tools are built backward from orbit-first block building, the only practical full-solve method ([solving-workflow](../../progress/1.0/solving-workflow.md)).
- **Target users.** Core: people who seriously intend to solve the complete 600-cell, at present the owner and, as far as the owner knows, two or three others. Secondary: people who know hypercube puzzles and want to understand and explore the 600-cell.
- **Human-solve boundary.** Allowed: tools outside the puzzle itself, such as entering and editing macros, macro analysis, templates and reuse of the solver's own macros, tracking, protection and view aids. A macro is entered by the solver and the tool outputs its analysis. Not allowed: the program outputting a solving macro by itself, or solving automatically. Open: whether a setup tool may search setups for the solver (the 0.4 contract forbade automatic setup search).
- **Views.** The Global and Local views are redesigned from scratch.

## Several themes on one design language

1.0 offers several visual themes rather than one. They share one design language of high quality that is efficient in use and compatible with the engineering (renderer budget, picking, legibility), and users can tune their own experience within safe ranges. Every shipped theme must pass the palette checks and the renderer selection gate in its own look. Subtitles remain undecided.

