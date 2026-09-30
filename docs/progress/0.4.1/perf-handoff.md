# B4-12 performance work: handoff

Where the 0.4.1 B4-12 work stands, for the next session. The goal is a measurement-ready 0.4.1: harness, baseline, fixes, and a one-command formal run. Rules: `AGENTS.md`; method: [harness contract](b4-12-harness.md) and [measurement decisions](../../wiki/decisions/b4-12-measurement-decisions.md).

## Milestones

| Milestone | State | Next action |
|---|---|---|
| A. Harness | Contract, fixture builder, turn probe and summary tool on branch `claude/0-4-1-b412-perf`. Native harness (packet A1) and runner (packet A2) not started. | Launch A1 and A2 through the wrapper from the B4-12 checkout, when no implement call from another checkout is running. Integrate: `print_identity.py` counts become 23 C# and 28 harness files with `B412Checks.cs`. |
| B. Baseline | Not started. Needs A, the owner's PresentMon pin and toolchain approval, and a desktop run (approved). | One smoke run, then `run_b412.py series --runs 3`; publish the summary table and a ranked hotspot list. |
| C. Fixes | C1, X-02 (checkpoint list in `Session.status()`, `session.py:326`): the Astra plan check found four minor issues and no blocking ones, and all four were adopted into the packet. They are the reviewer's own suggestions, so the scoped re-check is skipped under the owner exception of 2026-09-30. C0, a journal-equivalence recorder with golden records from the unmodified engine, is drafted as the acceptance base for engine-side fixes. C2 (a live turn tests protection without the full draft review) is withdrawn after its Astra plan check found four major issues: the skipped review also saves the session timer on the browser route, raises validation errors that stop a live turn today, and leaves a different residual cache after a failed turn, and its planned test shared the code under test. Those cases were added to C0's scenarios. | Launch C0 and C1 through the wrapper under the engine interpreter. Measure X-02 with the `history-400` attribution fixture before and after. Rank the remaining engine leads (residuals, `score_current`, `clone`, `PuzzleState` reuse, `canonical`, transport polling) only after the native baseline. |
| D. Measurement-ready | Not started. | Harness unchanged since the baseline, a one-command 3 x 100 runner, and the result template filled from a dry run. |

## Findings that shape the harness

- 0.4 protects every orbit that a commit re-solves, and then rejects a live turn that moves it. Every M1 inverse pair re-solves orbits, so each pair is followed by an untimed protection release (the 0.4 latency harness does the same).
- A step 3 session database has a pinned `sqlite_master`, so a performance fix must not add an index or any other schema object.
- The turn probe is attribution only. It never yields B4-12 results.
- Engine-side fixes on the live turn path must keep failure behaviour too, not only committed results: a live turn today also runs goal and residual validation and can save the session timer, so skipping work changes what a failed or browser turn does. Packet C0 records these cases.

## Coordination

- A new implement call creates a `codex/<id>` branch. A running call from another checkout counts it as a ref change and becomes invalid. So launch only when no other checkout has an implement call running, and do no commit, fetch, branch, merge, push or cleanup while any implement call runs.
