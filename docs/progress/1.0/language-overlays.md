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

## OV-2. The 1.0 log file's record (chapter 05's catalogue; ADR-D01.80 to .84)

- **Authority.**
  - ADR-D01.80, .81 and .84 (`docs/architecture/1.0/design/adr/ADR-D01-rev5-documents.md`, frozen at A7): LP4
    builds the 1.0 log format and its compiler, and names the extension.
  - The rule of [22 November 2026](../../wiki/decisions/owner-decisions-2026-11-22-adr-d01-overrules.md): where
    ADR-D01 conflicts with a frozen clause, ADR-D01 wins.
  - The integrator's adoption of this record on 23 November 2026.
- **Clauses.** Chapter 05's PS1.12 and its catalogue of formats, and FP.4's list of format identifiers.
- **Reading.**
  - PS1.12 says that nothing in the catalogue freezes a format. The log's record is `c600-log-draft`, version 1.
  - Like the descriptors PS1.12 names, it keeps `-draft` until its own packet.
  - The file is plain canonical JSON, or stored gzip above 250,000 bytes, with the extension `.c600log`.
- **What it changes.**
  - The log is a product exchange file written from the session store. It adds no session-store record kind.
  - It changes no byte of the frozen migration format or of chapter 05.
  - Its FP.4 row and its catalogue row follow at the format's next versioned change.
  - The extension, the gzip threshold and the MPUlt cap (0.4's: 10,000 native moves, 20,000 sequence entries,
    16 MiB) are the integrator's defaults.
  - `c600-log` has path dependencies only. A real compressor would be a product dependency and goes to the owner.
- **Tests.** `rust/c600-log/tests/` (compile gates, dependencies, interchange).
