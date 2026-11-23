# Overlay records on frozen language text

Status: register started on 23 November 2026 by the LP4 line. An overlay record says how a frozen clause is read or
completed after an owner decision, where editing the clause would change bytes that must stay as they are, such as a
file of the frozen format candidate (`docs/architecture/1.0/language/format-packet/candidate.json`). An overlay record:
- edits no frozen text;
- accepts no ADR;
- changes no format.

The command layer's readings of 23 November 2026 are recorded with their rows in [the command table](command-table.md).

## OV-1. What a run record holds for replay (chapter 05; EV2.9)

- **Authority.**
  - Owner, [23 November 2026](../../wiki/decisions/owner-decisions-2026-11-23-lp4-lp5-start.md), item 2.3: replay
    inspects recorded effects and never re-executes them, as a PHS.52 revision of chapters 05 and 09.
  - The integrator's ruling of the same day: chapter 05 is a file of the frozen format candidate, so its bytes stay
    as they are, and its part goes here. Chapter 09's part is DET.7, amended in place.
- **Clauses.**
  - Chapter 05's row for the run record: `c600-run-draft`, reserved under FP.4, in the table of record kinds.
  - Chapter 04's EV2.9.
  - Chapter 09's DET.7 as amended.
- **Reading.** Beside its reproducibility key (`ReproKey`, DET.1), a run record that is to be replayed holds the run's
  host trace in order:
  - each read, with the input it was answered with;
  - each tariff charged, since EV2.4 compares the charges;
  - each preview or sink effect, with its row, its request and the bytes it produced;
  - the outcome.

  Replay answers reads from those captured inputs and compares each effect at the same trace position. Nothing outside
  the record is captured or consulted: there is no isolated replay sink, ambient file or clock.
- **What it changes.** Nothing in the frozen format. `c600-run-draft` stays reserved, and its form is written within
  FP.4's reservation when the record producer is built. A producer whose effectful runs should be replayable captures
  the trace above.
- **A record without the trace** stays inspectable as recorded evidence. Replay refuses an effect the record does not
  hold with `EV2:capability.debug_write_forbidden` and writes nothing.
- **Tests.** CT-EV2-10 (`rust/c600-debug/tests/replay.rs`).
