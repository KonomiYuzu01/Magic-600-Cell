# LX-4's L completion and L diagnostics: the slowdown of 25 November 2026, found headless and fixed

The [owner-machine sitting of 25 November 2026](../../../docs/wiki/evidence/owner-sitting-2026-11-25-results.md)
measured LX-4 (U8) on main `bb416a2c`, whose code equals LP4's candidate `0208e78d`. L completion and L diagnostics
were about 50 ms slower than on 21 November's candidate `013900ff` (p50 about 141 ms against 90.5, and about 159 ms
against 107.6). Every other kind and size was within a few milliseconds. These records find the cause headless and
time the fix. All of it is headless timing of the language work that the shell's frames wait for. It is no display
latency and no Windows/DirectX, input, on-screen or long-session evidence. The bars of LX-4 do not change (owner,
22 November 2026).

## The cause

LP4-3 (`08f2dfd2`) re-pointed LX-4's editor library (`renderer/bridge/lx4`, the shell's `c600_shell_lx4.dll`) from
its own name list and a direct lower and check to EV1's editor services in `rust/c600-editor`. For each buffer
revision the worker runs syntax, then completion when the caret follows an identifier, then diagnostics when the
syntax is clean. On 25 November's code:

- `c600_editor::complete` read the buffer again: a parse and a lex of it, a second lex for its tokens, and a name
  index whose `Index::build` scanned every binding of the file for each name it resolved. On L (about 5,000 lines)
  that scan alone cost about 20 ms.
- `c600_editor::diagnose` parsed and lexed the buffer once more before lowering and checking it.

On 21 November's code, completion filtered a fixed name list (about 0.1 ms) and diagnostics lowered and checked the
syntax stage's parse (about 10 ms on L). On S and M the same extra work costs a few milliseconds, which is why only L
moved.

`stage_profile.rs` times the pieces on the L buffers (release; p50 of 15 calls on one document, 25 November's code,
on this machine on 26 November 2026 before the recorded rounds, power state not recorded):

| Piece | ms |
| --- | --- |
| `c600_syntax::parse` | 9.6 to 10.1 |
| `c600_syntax::lexemes` | 3.9 to 4.3 |
| `c600_syntax::lower` plus `c600_check::check` | 10.2 to 10.6 |
| `c600_editor::complete` | 38.4 to 41.5 (reading the buffer 12.2, `Index::build` 19.4 to 19.9, lexing again 3.0 to 3.2) |
| `c600_editor::diagnose` | 30.4 to 31.1 |

## The fix

In `rust/c600-editor`, with no change to any service's answer:

- The name index lists each binding under its owner (the module, a complete declaration or a recovery hole) and maps
  the module's first declaration and first import alias of each name. A lookup reads its owner's bindings, with the
  same rules and tie-breaks. A unit test compares every lookup with the old scan at every position of a source with
  shadowing, duplicate names, imports and a recovery hole.
- A `Document` makes its parse once, when a service past the parse needs it, and shares it with the later services on
  the same document. `complete` makes it, and `diagnose` then reads it. `parse` reads it only when another service
  has made it; otherwise it parses on its own, because sharing would mean a copy of the parse (about 1.7 ms on L) on
  the path of the syntax answer, which LX-4's L syntax bar (p95 95.0 against 100 ms on 25 November) cannot spare.
  Tokens are lexed only when a service reads them, so `diagnose` no longer lexes.
- `tests/shared_reading.rs` checks that parse, complete and diagnose answer a document whose parse is shared, in
  three orders, exactly as they answer a fresh one, on complete, broken, importing and unpinned sources.

## Method

`renderer/bridge/lx4/examples/endpoint_timing.rs` drives the LX-4 library through its C interface, as the shell's
module does, on U8's fixtures: per sample a Reset, then per replacement a Select, 250 ms of frames, the insert's keys
in one burst, then frames until the endpoints are queued. On the owner's machine, lx4.py posted a sample's first key
about 330 ms after its Reset (p50 over the 25 November run; p10 250 ms), so by then the work that the Reset started
has ended. The clock runs from just before the timed post (the burst's last key) to the frame update that first
queues the endpoint. A frame period of 1 ms gives the work's own time. A period of 16.667 ms adds 60 Hz polling
quantisation and counts one extra frame, because the driver reads its clock after each frame's wait.

The fixtures are U8's: `MANIFEST.json` sha256 `59fda757…`, `EDITS.json` sha256 `03f50471…`, made by
`c600-lx4-fixtures --out`. Three builds of the driver, each from one tree with its own target directory: `old` from
`013900ff` (21 November), `mid` from `bb416a2c` (25 November) and `new` from this change. `rounds.py` runs them in
alternating rounds, so that a change of machine state falls on every build alike, and reports the median over rounds
of each run's p50 and p95:

    python -B work/experiments/lx4-latency-20261126/rounds.py --fixtures FIXTURES --rounds 6 --samples 8 \
        --targets S,M,L --frame-ms 1 old=OLD_DRIVER mid=MID_DRIVER new=NEW_DRIVER
    python -B work/experiments/lx4-latency-20261126/rounds.py --fixtures FIXTURES --rounds 4 --samples 8 \
        --targets L --frame-ms 16.667 old=OLD_DRIVER mid=MID_DRIVER new=NEW_DRIVER

The machine was the owner's laptop (AMD Ryzen 9 7945HX). It ran on battery power during the recorded rounds, and
every build took about 1.5 to 2 times its earlier times on AC power, so compare columns, not absolute values. Another
session's build ran on the same machine for part of the day; the rounds were run when it had stopped.

## Results

`results/rounds-1ms.txt` (6 rounds of 7 samples per cell, 1 ms frames), p50 / p95 in ms:

| L | old (21 Nov) | mid (25 Nov) | new |
| --- | --- | --- | --- |
| completion (completion-trigger) | 20.9 / 28.4 | 121.0 / 136.7 | 65.4 / 78.8 |
| diagnostics (exported-signature) | 43.6 / 60.1 | 181.0 / 211.5 | 95.4 / 111.2 |
| syntax (syntax-break) | 19.6 / 28.6 | 28.4 / 36.5 | 26.2 / 33.9 |

`results/rounds-60hz.txt` (4 rounds of 7 samples, 16.667 ms frames, one frame added to every value), L, p50 / p95:

| L | old | mid | new |
| --- | --- | --- | --- |
| completion | 50.0 / 50.0 | 141.7 / 158.4 | 91.7 / 100.0 |
| diagnostics | 83.3 / 83.3 | 200.0 / 225.0 | 116.7 / 142.1 |
| syntax | 50.0 / 50.0 | 50.0 / 50.0 | 50.0 / 58.4 |

The fix removes about 55% of the extra completion time and about 63% of the extra diagnostics time in these rounds.
L syntax keeps 25 November's path. S and M stay within a few milliseconds of 25 November or below it.

What remains on L is new work of EV1's completion: it reads the scope at the position, so it parses the buffer (the
syntax stage's parse is not shared, see above), lexes it once (about 3.5 ms) and builds the index (about 1.7 ms).
Diagnostics then lower and check a copy of the shared parse (about 1.7 ms for the copy), as before.

## Limits

- Headless only. On screen, an endpoint shows in the first frame whose update finds the answer, and every frame
  adds the same present latency (echo about 75 ms). On both nights L syntax and S, M and L completion showed one
  frame after echo, and 21 November's L diagnostics two frames after it. Whether the fixed L completion and
  diagnostics land one frame later than on 21 November, or two, depends on the shell's frame timing and the machine's
  state. Tonight's unattended U8 re-run decides it.
- The driver's frame loop is no Godot frame loop, and the worker shares the machine with no renderer.
- One machine, on battery power during the recorded rounds.
