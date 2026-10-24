---
id: owner-decisions-2026-10-24-theorem-program
type: decision
status: verified
visibility: public
summary: Owner decision of 24 October 2026 - the jumbling research on height 4 to height 2 changes method, from searching rule catalogues to a constructive theorem programme in the style of the X1 proof; the first problem is the blocking structure of deep states; the second sealed analysis of each joint attack is made by Claude Opus instead of the Fable solver.
related: [owner-decisions-2026-10-23-depth, owner-decisions-2026-10-22-tutorial-first, owner-decisions-2026-10-20-tutorial-acceptance, owner-decisions-2026-10-16-x1-api-budget]
supersedes: []
claims:
  - {id: theorem-programme, evidence_kind: decision, checked_at: 2026-10-24}
  - {id: blocking-structure-first, evidence_kind: decision, checked_at: 2026-10-24}
  - {id: claude-opus-second-analysis, evidence_kind: decision, checked_at: 2026-10-24}
---

# Owner decision, 24 October 2026: a theorem programme instead of rule searches

The owner gave this decision in a chat message in Chinese; this page records it in English.

## Evidence put to the owner

The owner asked for a way to reach an acceptance run that passes once, instead of a loop of runs, rule patches and
reruns, and asked the integrator to re-read the whole research record first. The integrator's report (chat, 24 October
2026), headless on Linux:

- No method is guaranteed to restore every reachable state within a bound: a computable bound valid at every
  reachable state exists exactly when reachability is decidable (Theorem D,
  [general method](../../../research/jumbling/restoration/09-general-method/METHOD.md) section 5), and the complete
  method M\* relies on an exhaustive fallback of about 300 process-days at word length 2.
- The six start states of the height 4 to height 2 research (D2) all come from scramble census runs whose bulk phase
  ended on its clock; at seed 7304 the runner ran with the bulk phase off, and its three starts with the bulk phase on
  were killed by container restarts
  ([d2-s7304](../../../research/jumbling/restoration/13-tutorial-runner/runs/dev/d2-s7304/RESULT.md)). No full runner
  run deeper than 40 twists has finished, so none of the deep stops is a stop of the full pipeline.
- The seven rounds of rule research for height 4 (single letters, CL2, PEEL, PREP1, PREP2) each proved that their rule
  terminates, never that it covers the states it meets; each new state could stop it again.
- The results that did close problems were constructions with proofs: the X1 family and its certificate (found after
  the run was told to stop searching and to study the mechanism, [stage 10](../../../research/jumbling/restoration/10-x1-unbounded/README.md)),
  the lattice phase (G = 𝒦, A_J, the round 11 schedule with its proved bound) and the tools of depth one (class
  puzzles, pure twists, transport words, vacancy certificates).

The integrator proposed to treat each obstacle as a mathematical problem: state it as a theorem with hypotheses that
can be checked at a state, give a construction with an explicit length bound and exact certificates, and use
computation only to test the hypotheses and check the certificates. The first problem proposed was the blocking
structure of deep states (which grips are blocked, by which pieces, and in which order they can be freed without
raising heights), because it underlies both the stops of the height phase and the slow bulk phase.

## Decision

1. The research on height 4 to height 2 changes method as proposed: a constructive theorem programme replaces the
   search for rule catalogues.
2. The first problem is the blocking structure.
3. In the joint attacks of this programme, the second sealed analysis is made by Claude Opus instead of the Fable
   solver. The first is made by Astra, as before.

## Integrator readings (the owner may change them)

1. **What stops.** No new rule search for height 4 starts. The three audits of the HD2P/2 campaign still running
   (seeds 7302, 7306 and 7307) finish and are recorded with the campaign's results.
2. **What continues.** The development runs D1 (depth 40) and D3 (depth 100) keep running. No new deep development run
   starts until the theory says what to test.
3. **Format.** Each problem gets a card in the style of cards 3 to 5 (model, proved facts, definitions, the theorems
   to prove, the certificates, exact facts at test states). A joint attack follows `AGENTS.md`: two analyses of the
   same clean card, sealed before exchange, then cross-examination, then an exact experiment decides. Reachable deep
   states, the clock-cut census stops included, are valid test cases for a theorem about reachable states.
4. **Unchanged.** The acceptance test keeps its seven depths ([23 October](owner-decisions-2026-10-23-depth.md)). How
   a stop at a deep batch is scored is still to be decided before the acceptance run. The work uses subscription quota
   only.

## Effect

- The method of the research restarted on 23 October changes; its scope (the rules the deeper batches need) does not.
- The Fable solver's role in `AGENTS.md` is unchanged; this decision applies to the joint attacks of this programme.
