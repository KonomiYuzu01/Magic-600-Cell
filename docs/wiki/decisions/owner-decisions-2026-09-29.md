---
id: owner-decisions-2026-09-29
type: decision
status: verified
visibility: public
summary: Owner decisions of 29 September 2026 on 0.4.1, the 1.0 program, migration, stage 2 and the development system.
related: [codex-plan-review-2026-09-29, development-loop]
supersedes: []
claims:
  - {id: release-0-4-1, evidence_kind: decision, checked_at: 2026-09-29}
  - {id: program-1-0, evidence_kind: decision, checked_at: 2026-09-29}
  - {id: development-system, evidence_kind: decision, checked_at: 2026-09-29}
---

# Owner decisions, 29 September 2026

Approved by the owner after three Codex review rounds (see [the dialogue summary](../dialogues/codex-plan-review-2026-09-29.md)). The current operational summary is in `docs/development-guide/AGENT_BRIEFING.md`.

## Releases

- **0.4.1** is the next release and the historical closing release of the 0.4 line. Its scope is optimization of 0.4, debugging, performance and bug fixes, code optimization, and a dedicated local development workbench.
- **B4-12** performance acceptance closes in 0.4.1. It is no longer deferred to 1.0.
- Internal order of 0.4.1: reproducible identity and harness; baseline; exporter and fixtures; screening and fixes; freeze candidate; formal measurement (3 x 100); release.
- **1.0** is a new program, not a continuation of the 0.4 runtime. Of 0.4.1, only its optimization records inform 1.0 engine selection. The mathematical contract, protection rules and model identity remain binding.

## 1.0 direction

- The Rust-only 1.0 boundary is dropped. Godot and Blender are in scope, following the owner's art direction.
- Direct3D 12 is the base of the dedicated 600-cell renderer and must be testable.
- Themes and subtitles are undecided.

## Migration

- User data migrates one way: a 0.4.1 exporter writes a neutral, versioned `.c600migrate` package; 1.0 verifies and imports it. The exporter never modifies the original session directory.

## Stage 2 (inventory, teardown and redesign)

- Stage 2 is a decision process that removes named uncertainties in order: charter and authority, two independent inventories, dispositions, redesign, experiments, freeze.
- The experiment window is 15 working days with a go/no-go on day 7. A bare Direct3D 12 interop probe is mandatory before deeper Godot work.

## Development system

- Claude Opus is the main developer and the only integrator.
- Codex is the independent reviewer: default model GPT-6.1 Sol at `max` effort, `ultra` for escalation and joint attacks. Codex Astra gives the final ruling at three gates only: the stage 2.4 day-7 go/no-go, the migration format freeze and the 1.0 architecture freeze. Every non-trivial task gets a Codex plan check and a Codex review of the current candidate.
- Fable is the escalation solver.
- The agent-team dialogue mode and the API-team mode are kept. The API team may run on OpenAI models; the owner confirms each run's provider, models and budget first.
- Missing tools and skills are installed automatically only from the reviewed, pinned allowlist, after the owner approves the installer revision.
- Every formal non-software output is made with local tools and keeps its source, tool versions and production command.
- This wiki is the shared engineering memory; model calls stay in a private ledger.

## Superseded text

In `docs/architecture/1.0/10_V1_ARCHITECTURE.md` (1.0-A3), the runtime and technology-stack choices, the deferral of B4-12 to 1.0, and the rule that unspecified 1.0 behaviour inherits from 0.4 are superseded. Its mathematical and protection invariants remain in force.
