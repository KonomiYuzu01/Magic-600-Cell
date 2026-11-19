# Full 600-cell base theory in Lean

A Lean 4 and Mathlib project for the base theory in [`research/ARCHON-HANDOFF-THEORY.md`](../../ARCHON-HANDOFF-THEORY.md). This first version holds Milestone 1, the permutation core (S1 to S3, L14, the abstract L9 to L11, and the checks N5 to N9), as statements only.

## State

- `ArchonFull600Theory/Basic.lean` defines the objects Milestone 1 needs and states its 16 theorems. Every theorem body is still `sorry`, so `#print axioms` lists `sorryAx` for each one. Nothing is proved yet.
- `lake build` passes with the 16 `declaration uses 'sorry'` warnings (Windows, 19 November 2026).
- The statements were written by an AI agent and have not yet been checked against the handoff. Treat them as drafts until the proofs land.
- `blueprint/src/` is the leanblueprint source. Its chapter `chapters/ArchonFull600Theory_Basic.tex` gives each target's statement, informal proof and dependencies, and the integer certificates for the comparisons N7 to N9.

## How it was produced

- Tool: Archon 0.3.3, an automated Lean formalization loop, run outside this repository.
- Model: OpenAI GPT-6.2 Sol at effort `max`, through the Codex CLI.
- Run: one iteration (iteration 19) of the `autoformalize` stage, with plan, prover and review phases taking about 50 minutes in total. Earlier iterations produced no Lean because of Windows launcher and sandbox problems in the run setup.
- The agent logs and Archon's state directory are not included.

## Build

The toolchain is pinned in `lean-toolchain` (Lean v4.36.0), and `lakefile.toml` requires Mathlib at `v4.36.0`.

```
lake exe cache get
lake build
```

## Next

Milestone 1's proofs continue in Claude Code sessions, not in Archon, and land as a follow-up pull request that also checks each statement against the handoff. Milestones 2 to 6 of the plan are not started.
