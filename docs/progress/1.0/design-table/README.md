# Design table

Status: **working protocol for stage 2.3** (owner direction, 3 October 2026: the four roles exchange views from the start of the design work, and no role waits for another to finish).

The design table is how the four roles of charter section 5 work on one design question together: the front-end designer, the interaction artist, the mathematician and the engineer's advisor. Their role cards are subagents in `.claude/agents/` (`front-end-designer`, `interaction-artist`, `mathematician`, `engineer-advisor`). The mathematician advises in design rounds now; its own deliverables (the theory book, the formal grammar) start later, when the engineering work gives it a reference.

## Round protocol

Subagents cannot message each other directly. The main session carries the exchange through one round record per topic, and every role reads the whole record before it answers.

1. **Open.** The main session writes `topics/<topic>.md` with the question, the owner's input, the relevant handoff (H-01 to H-05) and what is out of scope.
2. **Round 1 (parallel).** The main session runs all four roles concurrently on the same record. Each returns comments in the card format (`id`, `on`, `kind`, `claim`, `evidence`, `needs_owner`). The main session appends them unchanged under `## Round 1`.
3. **Round 2 (cross-examination).** All four roles run again on the updated record and answer each other's comments by ID: support, object or propose an alternative. The main session appends the replies under `## Round 2`.
4. **Settle.** The main session writes `## Outcome`, settling the round in this order:
   - Filter: an option that breaks the mathematical contract, protection, the human-solve boundary, the renderer gate, data safety or an owner design requirement is out.
   - A demonstrated engineering failure that affects the work or correctness is fixed first.
   - Among the admissible options, the owner's 8B order applies: core workflow (designer), then legibility and delight (artist), then cost beyond the gates (engineer), then notation preference (mathematician).
   - An unmeasured objection becomes an experiment (Look Lab, Taste Lab or the living test), not a veto.
5. **Owner.** Items marked `needs_owner`, and all taste, scope and UX choices, go to the owner once per topic, with two or three options in a few lines. The owner's answer is recorded in the outcome.

Two rounds per topic. A third round runs only when round 2 produced new evidence; two rounds without new evidence end in an experiment, as `AGENTS.md` requires.

## Records

- Round records are design evidence in English; they hold no personal data, screenshots or private paths.
- An outcome that changes a handoff updates the handoff tables in `docs/progress/1.0/stage-2-experiment-protocol.md` and `docs/progress/1.0/renderer-experiment-plan.md`.
- Owner decisions from a topic go to `docs/wiki/decisions/` as usual.
