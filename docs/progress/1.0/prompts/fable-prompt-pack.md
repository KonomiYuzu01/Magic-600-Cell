# Fable prompt pack (stage 2.3 / 2.4)

Status: **draft for the owner**. Nothing here is an authorization: each prompt runs only when the owner starts it, and every rule in `AGENTS.md` still applies to what it produces.

Prompts for a Fable session, single-agent or as an Ultracode workflow. They are ordered: **P1 (reverse engineering and the interactive system graph) runs first**, because every later prompt reads its outputs instead of re-reading the repository.

## How to use

1. Fill in the **owner block** in P0 (your own thoughts, priorities and the assignment line). Leave a field empty rather than guess.
2. Paste **P0 + one prompt** into a fresh session. P0 is identical every time so it stays a cache hit; put changing text only in the owner block at its end.
3. Ultracode: add the word `ultracode` to the message. Each prompt lists its phases and a suggested agent count (keep under 10 unless you raise the size in `/config`).
4. Every prompt ends in a **return card** (section "Return card"). Paste the card, not the transcript, into the next prompt or into the round table (P10).

| # | Prompt | Needs | Output (repository-relative) | Mode |
|---|---|---|---|---|
| P1 | Reverse engineering and interactive system graph | — | `docs/progress/1.0/system-graph/` | Ultracode, 6–8 agents |
| P2 | Code review against the graph | P1 | review JSON under `work/reviews/`, summary page | Ultracode, 4–6 shards |
| P3 | Renderer data review and smoothness | P1 | analysis page + implement packets | single or 4 agents |
| P4 | GPU math and time-efficient workflow | P1 | proposal page + packets | single or 3 agents |
| P5 | Taste Lab and Look Lab improvements | P1 | design note + packets | Ultracode, 5 agents |
| P6 | Taste Lab and Look Lab in the product; studio compartment | P5 | design-table topic | design table (4 roles) |
| P7 | UI polishing workflow | P1, P5 | workflow page draft | single |
| P8 | Movie mode and publicity video | P3, P6 | plan + shot list | single or 3 agents |
| P9 | Agent workflow and token efficiency (`AGENTS.md` proposals) | P1 | proposal only | single |
| P10 | Round table | cards of P1–P9 | decision sheet for the owner | design table + engineer |

---

## P0 — Shared header (paste first, unchanged)

```text
You are working on Magic 600 Cell (repository KonomiYuzu01/Magic-600-Cell). Read, in this order and nothing else
before your first plan: AGENTS.md, CLAUDE.md, docs/development-guide/AGENT_BRIEFING.md, docs/wiki/index.md,
the first 60 lines of docs/wiki/log.md, docs/progress/status.json. If docs/progress/1.0/system-graph/graph.json
exists, read its summary.md and query graph.json instead of re-reading source files.

Fixed facts (do not re-derive):
- Stage 2 (status.json, 3 Oct): 2.0–2.2 done; 2.3 redesign in progress, G1–G5 and design tokens open, no
  design-table topic run yet (design-table/topics/ is empty), the owner alone takes G1–G6.
- 2.4 renderer: S-B bare Direct3D 12 meets the W3 gate (778 fps pooled, p99 1.55 ms, 81.7 MB VRAM, 2560x1600,
  tearing on; docs/wiki/evidence/s-b-probe-results.md); W1, W2, W4, W5 and the H-06 cost table are not measured
  (seven features implemented, no costs). Godot 4.7.2 and Qt 6.10.3 pass level-1 interop, no timing. Level 2
  harness and packets merged (PR #57), no level-2 run yet. Day-7 go/no-go packet empty
  (docs/progress/1.0/packets/renderer/E-2.4-04-day7-gate.md). No smoothness data beyond p99.
- Turn animation: 190 ms smoothstep, rotation in the vertex shader; the CPU applies each move to the label
  array and re-uploads all 259,800 labels per revision (work/experiments/renderer-sb/SPEC.md section 4).
- Taste Lab: private Artifact page; phase 1a learner accepted with recorded limitations (low setting needs
  recalibration; cross-owner ranking only a hint); phase 1b class A library + CLIP ViT-B/16 (NumPy) merged
  (PR #56), bulk fetch waits on the owner's C-02 choice; nexus cards tool in tools/tastelab/refs (local only).
- Look Lab: specified (stage-2-experiment-protocol.md section 4.2, Godot) but NOT built; no code exists.
- Migration path B (locked byte copy, probes P1–P3 passed).
- Binding: full 600-cell-Full profile (259,800 sticker slots, 1,200 generators); assets/manifest.json immutable;
  human-solve boundary (the program never chooses, outputs or executes a solving macro on its own);
  renderer gate (>= 30 fps average, p99 <= 33.3 ms, full detail, owner's RTX 4070 Laptop GPU);
  owner decisions in docs/wiki/decisions/ outrank every proposal you make.
- Evidence kinds stay separate: source/fixture, synthetic geometry, actual Windows/DirectX, performance.
  A Linux or cloud run proves only headless checks; say which Windows checks you did not run.

Working rules for this session:
- Assignment: see the owner block. Unless it says otherwise you analyse and propose; you edit only files the
  prompt lists as outputs, on your own branch; critical paths (briefing section 3) get a proposal, not an edit.
- Token budget: read with grep/ranges, not whole files over 400 lines; delegate broad sweeps to subagents that
  return <= 300-word digests with path:line evidence; never paste raw logs, transcripts or diagnostics.
- Context sharing: write facts once, to the prompt's output file, and link it; do not restate them in chat.
- Every claim carries path:line or a command and its result. Mark inference as inference.
- Taste, scope, UX and release choices are the owner's: present 2–3 options in a few lines, recommend one.
- Never publish personal data, private paths, screenshots of the owner's sessions or credentials.
- Stop when the prompt's acceptance is met. Do not widen scope; list follow-ups instead.

Return card (end every run with exactly this, <= 250 words):
  prompt: <id>   branch/commit: <...>   evidence kinds: <...>
  done: <bullets with paths>
  findings or options needing the owner: <id, one line each, recommended option first>
  not run (and why): <...>
  next prompt should know: <<= 5 bullets>

Owner block (the owner edits only this part):
- Assignment: [analyse-and-propose | implement on own branch, Claude Opus integrates | Fable integrates this PR]
- My thoughts / priorities this run:
- Must not touch this run:
- Budget: [subscription only | paid API ceiling USD ...]
```

---

## P1 — Full reverse engineering and the interactive system graph (run first)

```text
Goal: one reproducible, queryable map of the whole system that later prompts read instead of the source.

Build:
1. tools/sysgraph/extract.py — a deterministic extractor (stdlib Python; no network; no new dependency unless it
   is already in tools/toolchain.lock.json). It walks the repository at HEAD and emits
   docs/progress/1.0/system-graph/graph.json with:
   - nodes: module/file, class, function, CLI entry, test, schema, hook, tool, experiment, doc/decision page,
     owner decision, handoff (H-01..H-07), gate (G1..G6, renderer, day-7, freezes), runtime process, data store;
     each with path:line, language, size, critical-path flag (from tools/agents/critical_paths.json),
     evidence kind, last commit touching it.
   - edges: imports, calls (best effort, marked "static"), reads/writes (files, SQLite tables), spawns
     (process), tests (test -> target), documents (doc -> code), decides (decision -> rule/doc),
     hands-off (H-xx producer -> consumer), gates (gate -> deliverable), supersedes.
   - provenance: HEAD sha, extractor digest, file digests, generation command. Re-running on the same HEAD
     must give a byte-identical file (sort keys, no timestamps besides the sha).
2. docs/progress/1.0/system-graph/viewer.html — one self-contained offline page (no CDN; inline JS; reads
   graph.json via a file picker or an embedded copy): force/hierarchical layouts, filters by node kind,
   critical path, evidence kind and track, search, neighbourhood expand, path finder between two nodes,
   "what breaks if I change X" (reverse dependency closure), "where is this decided" (decision edges), and
   a gate/handoff timeline view. Dark and light themes. Works on a laptop screen.
3. docs/progress/1.0/system-graph/summary.md — <= 2 pages: subsystems, data flow of a turn from input to
   pixels (0.4 path and the 1.0 candidate paths), process topology, persistence, the agent/tooling layer,
   open handoffs, dead or orphaned code, the 20 most connected nodes, and every place where docs and code
   disagree (a contradiction list, not edits).
4. tests/test_sysgraph.py — determinism (two runs equal), schema check, every critical path present, no
   private path or personal data in the output.

Ultracode phases (6–8 agents): Inventory (shard by top-level directory) -> Extract edges (Python, C#/native,
web/JS, tools/hooks, docs/decisions) -> Merge and verify (one agent checks 30 random edges against source)
-> Viewer -> Summary. Each shard returns JSON fragments, not prose.

Acceptance: python tests/test_sysgraph.py passes; viewer opens offline and answers the four questions above
for core.py, session.py and the Look Lab; summary.md lists its contradictions with path:line on both sides.
Out of scope: changing any code the graph describes; publishing the viewer.
```

## P2 — Code review against the graph

```text
Goal: find defects that matter for 1.0, ranked, with a falsifying experiment each. Uses P1's graph to pick
targets: critical paths first, then the 20 most connected nodes, then code with no test edge.

Shards (concurrent, each read-only, each returns schemas/review-result.schema.json JSON):
 A mechanics and protection (core.py, grips.py, enhanced.py, model logic) — invariants of AGENTS.md;
 B persistence and process (session.py, session_lock.py, log_io.py, engine_process.py, server.py);
 C renderer experiments (work/experiments/renderer-*) — correctness of captures, label checks, gate math;
 D Taste Lab (tools/tastelab) — colour math, GP learner, page storage, data safety;
 E workbench and hooks (tools/workbench, .claude/hooks) — bounded, never calls models, never leaks data;
 F the migration importer design vs probe results.
Each finding: id, severity, path:line with file digest, counterexample, smallest experiment, status.
Then a verify phase: one agent per blocker/major tries to refute it with a run on fresh isolated data.
Only findings that survive go into work/reviews/<call-id>/ and a public summary without private paths.

Acceptance: every surviving blocker/major has a reproduction command; minor/nit are listed, not fixed.
This is not a substitute for the Codex review AGENTS.md requires before a commit.
Out of scope: fixing anything; style-only comments.
```

## P3 — Renderer data review and smoothness

```text
Goal: turn the stage 2.4 measurements into a smoothness verdict per candidate and a plan for what to measure
next, without claiming results that were not captured on the owner's GPU.

1. Collect every capture and result card (docs/wiki/evidence/*, work/experiments/renderer-*/RESULT*.md,
   PresentMon CSVs where committed). Tabulate per run: build identity, scene, average fps, p50/p95/p99/p99.9
   frame time, 1% and 0.1% lows, frame-time standard deviation, count of frames > 2x median (hitches), longest
   hitch, present mode, VRAM peak, attended/unattended, evidence kind.
2. Smoothness is not the average: define and justify a smoothness metric set (frame-time variance, hitch rate,
   pacing error vs display refresh, animation phase error of the turn curve) and say which existing captures
   can compute it and which need a new capture.
3. Animation: check how turn animation is sampled (fixed step vs frame-time step, interpolation, easing) in each
   candidate; propose a frame-pacing-safe scheme (fixed-step simulation, interpolated presentation, no
   mechanical state change from sampling — AGENTS.md).
4. Write docs/progress/1.0/renderer-smoothness-review.md: the table, the metric definitions, gaps, and
   2–4 implement packets (templates/problem-packet.md, with implement-contract) for the capture scripts and
   analysis code. Windows capture steps are written for the owner to run; you do not simulate them.

Acceptance: every number in the page links to its source file; every missing number says "not captured".
Known now: S-B W3 ran at 60 Hz with tearing at ~778 fps, so its p99 says nothing about pacing at the display
rate; propose a capped/V-Sync capture variant. Level 2 has no runs: write the owner's run order for
work/experiments/renderer-l2/README.md so its captures also yield the smoothness metrics.
Out of scope: the day-7 ruling (Astra gate); new renderer candidates.
```

## P4 — GPU math and time-efficient engineering

```text
Goal: find which work moves from CPU/NumPy to the GPU in 1.0 and what it buys, measured where possible.

1. From the graph: list every per-frame and per-turn computation (sticker transforms, permutation application,
   projection 4D->3D, orbit highlighting, label/hash checks, preview of collateral effects) with its current
   home (CPU/NumPy/shader), data size and frequency.
2. For each, estimate GPU suitability (compute shader in portable HLSL, buffer layout, readback need, determinism
   requirement). Mechanical state stays authoritative on the CPU unless a bit-exact check proves equality;
   say how the full-state hash is kept honest.
   First candidate already visible: the CPU re-uploads all 259,800 labels per revision; evaluate a GPU-side
   permutation (upload the move id, apply the generator's permutation table in a compute pass) against a
   delta upload, with the label readback check kept as the correctness gate.
3. Prototype the top one or two as a headless reference (NumPy) plus an HLSL compute kernel spec, and a
   differential test plan against the oracle (oracle/). Timing claims only from the owner's GPU.
4. Write docs/progress/1.0/gpu-math-proposal.md with a cost/benefit table and implement packets.

Acceptance: each proposal has an equality test and a measurement step; nothing changes the model identity.
```

## P5 — Taste Lab and Look Lab improvements

```text
Goal: make the two instruments learn the owner's taste faster and show more of why, without breaking the
accepted learner or its recorded limitations.

Read: docs/progress/1.0/taste-lab-plan.md, taste-lab-images-plan.md, taste-lab-experiment.md,
docs/wiki/decisions/owner-decisions-2026-10-03-taste-lab.md and -image-library.md, stage-2-experiment-protocol
section 4.2, tools/tastelab/refs/ (nexus cards). Respect the recorded limitations: no mode that narrows the
parameter space before the low setting is recalibrated; the cross-owner ranking stays a hint.

Note: the Look Lab is specified but not built. Item 0 below comes first.

Produce options (2–3 each, recommended first, cost in owner minutes and agent days) for:
 0. Look Lab build plan — the smallest Godot build that meets the build acceptance of protocol section 4.2,
    split into Codex implement packets (disjoint files), reusing the Taste Lab preset JSON shape so presets
    flow Taste Lab -> Look Lab -> design tokens. Add a "Taste Lab import" so phase 1a results are G3 priors.
 a. Nexus++ — nexus cards as a graph: link annotated references, presets, decisions and anti-goals; show why a
    look scores (which reference, which parameter); inspect paths "reference -> parameter -> preset".
 b. Self-training — active learning that asks fewer questions (expected information gain, duelling bandits
    on the GP), image embeddings as priors for parameters, self-consistency checks (repeat pairs to measure the
    owner's noise), drift detection between sessions. Simulated-owner experiment for each before any owner time.
 c. Artistic soul — how the metaphor/manifesto (G2) and anti-goals become measurable constraints and a
    "signature" layer (one motion, one material, one light) that survives every preset.
 d. Two contrasting UI sets — two deliberately opposed shells (for example calm instrument vs expressive
    stage) on the same command table, so the owner judges by contrast; both obey the text budgets and tokens.
 e. Further ways to inspect — per-parameter sensitivity, partial-dependence strips, uncertainty map, "what
    would change my mind" pairs, preset diff view, colour-vision-deficiency side-by-side, cost meter overlay.
Write docs/progress/1.0/lab-improvements.md and implement packets for the recommended options, each with its
simulated-owner or headless acceptance check.

Constraints: class B images stay local; the Artifact page stays private; no personal data in storage.
```

## P6 — Labs in the product and the studio compartment

```text
Goal: decide how much of Taste Lab and Look Lab becomes part of 1.0, and design a "studio" compartment for
users to tune looks that does not break the artistic direction.

Run as a design-table topic (docs/progress/1.0/design-table/README.md protocol): open
topics/labs-in-product.md; round 1 with front-end-designer, interaction-artist, mathematician,
engineer-advisor in parallel; round 2 cross-examination; outcome settled by the protocol's order.
Questions:
 - Which lab functions ship (theme tuning within the two authored families and three scene looks; preference
   swipe; compare mode; capture) and which stay development-only?
 - Studio compartment: where it lives (one entry, never modal over solving), guard rails (tokens bounded by the
   authored family, contrast and CVD checks, cost meter against the gate, reset to authored), how it feels
   dynamic without becoming a toy.
 - What the solver gains (legibility, comfort) vs what the art direction risks.
Return the topic file with needs_owner items as 2–3 option lines each.
```

## P7 — UI polishing workflow

```text
Goal: a repeatable loop that takes a G6 styled layout to shippable polish with the fewest owner minutes.
Propose: the loop (capture -> automated checks -> owner sketch -> agent change -> diff capture), the
automated checks (text budget counts, contrast/CVD, alignment to the grid, motion timing against the motion
table, flow-script step/time regressions, frame cost), the review cadence, and how owner corrections become
tokens instead of one-off fixes. Reuse the workbench gallery and ask.py for the owner's picks.
Write docs/progress/1.0/ui-polish-workflow.md (draft; the owner accepts it).
```

## P8 — Movie mode and publicity video

```text
Goal: a plan for a short publicity film and an in-app "movie mode" (camera paths, slow turns, symmetry tours).
- Human-solve boundary: movie mode may replay a solver's own recorded log or a synthetic demonstration
  scramble/unscramble only as playback started by the user; it never solves or chooses moves. Flag any shot
  that would need an owner ruling.
- Shot list (60–90 s): structure reveal (Hopf fibres, orbit colourings), a turn at full detail, protection and
  preview, the solved moment; each shot with camera path, look preset, duration and renderer cost.
- Production with local tools only (Godot or the selected renderer for capture, Blender for hero stills,
  FFmpeg for assembly); synthetic geometry only; no personal sessions; credits for Andrey Astrelin's MPUlt.
- Publishing is an owner decision; produce the plan and a capture script spec, not a published video.
Write docs/progress/1.0/movie-mode-plan.md.
```

## P9 — Agent workflow, token efficiency and context sharing

```text
Goal: cut wall-clock and tokens per finished task without weakening a gate.
Measure first from repository evidence (wiki log, review records committed, PR history): reviews per task,
rounds, plan checks reused, governance share against the 70/30 budget.
Propose (each with expected saving and risk):
 - a shared, versioned context pack (P0 + graph summary) that sessions and Codex packets cite by digest;
 - packet templates that reference graph node ids instead of pasting code;
 - which reviews can be scripted checks; which rounds the evidence shows are rarely useful;
 - parallel tracks and when Fable vs Codex Sol vs Astra is the cheapest correct reviewer;
 - GPU-backed checks that replace slow CPU tests (from P4).
Output: docs/progress/1.0/workflow-efficiency-proposal.md. Changes to AGENTS.md or CLAUDE.md are proposals in
diff form for the owner; anything that widens an agent's own authority is marked "owner writes".
```

## P10 — Round table

```text
Goal: one decision sheet for the owner from the return cards of P1–P9.
Run the design-table protocol with five seats: front-end-designer, interaction-artist, mathematician,
engineer-advisor, and fable-solver as moderator-critic. Round 1: each seat ranks the open options from the
cards, sealed. Round 2: cross-examination by id. Settle by the protocol's order; unmeasured objections become
experiments. Output: docs/progress/1.0/design-table/topics/round-table-<date>.md and, at its top, a sheet of
at most 10 owner decisions, each with 2–3 options in one line, the recommendation, cost and what it unblocks.
```

---

## Review record

Reviewed by subagents before the owner's use; see the end of this file for findings and how each was answered.
