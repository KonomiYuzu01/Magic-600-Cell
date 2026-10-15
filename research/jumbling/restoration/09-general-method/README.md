# 09. A state-only general method, 15 October 2026

The method reads only the current pose table and restores a jumbled configuration in two phases:
1. clean steps take the pieces that are off the lattice back onto it;
2. a lattice phase solves the remaining lattice configuration.

[METHOD.md](METHOD.md) is the main document:
- the standard of "restored" (owner decision, 15 October 2026);
- the method;
- the status of every part it rests on;
- what the new standard removes from the earlier programme;
- the gaps;
- the results;
- reproduction commands;
- the checks that were not run.

[SENIOR-REVIEW.md](SENIOR-REVIEW.md) records the Codex senior review of 15 October 2026: gap M1 (existence, the
s61c obstruction and certificate, bounds), Theorem C (the class-cell pure twist, at most 720 letters) and the other
cell kinds, the review of the clean-step and lattice-phase proofs with their corrections, and a milestone audit of
the method as a whole: Theorem M\* (with an exhaustive fallback the method is complete), the toolkit theorem with
its bounds, and a bound of 1,324,686,034 twists for the lattice phase. Round 5 narrows the coverage question of
gap M1; round 6 gives gap M2 lower bounds and a construction of at most 356,565,166 twists, and records the
integrator's searches for short pure three-cycles (33 of 35 orbits). Round 7 shows that a seed of the lattice
cascade needs no fibre alignment and replaces 31 of the 35 seeds by shorter relocated elements: the cascade then
restores the two synthetic inputs in 4.5 × 10^7 twists instead of 2.3 × 10^8, and needs at most 71,516,526 twists
for every input, plus at most 25,852 for the corrections. Round 8, a closing audit, corrected seven statements of
these pages and the summaries, and showed that the round 7 choice of seeds is optimal over the recorded candidates.
Round 9 asked whether the reachable set is finite (which decides a uniform bound for gap M1): still open, with a
uniform period for repeated words, an exact one-letter check at the deep state H6, and a correction to stage 04.
Round 10 performs each three-cycle of the lattice cascade with one application of its seed, conjugated by a short
word: a construction then needs at most 50,560,184 twists for every input (not implemented). Setting the frames
during the position loop instead fails as proposed.

The driver implements M\* (`scripts/gm/mstar.py`, `scripts/gm/fallback.py`): when its heuristic steps find
nothing, an exhaustive search over the menu words takes the step. The runs of 15 October used the earlier driver
v4, which had no fallback.

## Contents

| Path | What it is |
| --- | --- |
| `scripts/gm/` | The driver `gdrive.py` with its outer loop `mstar.py` and exhaustive fallback `fallback.py` (Theorem M\*), its step scripts, the tests `test_mstar.py` and `test_fallback.py`, `gen_scramble.py` and `omega_check.py` (Bad(W) against the cell, Theorem C) |
| `scripts/echain/` | Modules the step scripts share (stage 01) |
| `scripts/lattice/` | The lattice restorer `restore_lattice_late.py` and the independent checker `verify_late.py` |
| `scripts/run/` | The run commands used on 15 October; paths are neutral tokens |
| `data/library-merged.json.gz` | The 36-macro correction library of the lattice phase (rank 34 over F2 and 2 over F5) |
| `earlier/stop62-860-to-0/` | stop62, 860 off pieces, to 0 in 14 clean steps, then to solved. Contents: the chain (start table and step words), the independent replay, the lattice restore and verify reports with the solution record, and `step014-pure-twist-omega.json` (Bad(W) = Ω_X at s86a) |
| `earlier/x-e3-solved/` | Lattice phase and end-to-end check for E3 (the end of the X chain of stage 01) and for I1 and I2 (stage 02) |
| `runs/stop62-v4/` | The state-only v4 run on stop62: the chain (start table and step words), the driver log with private paths replaced by `<working tree>`, and the independent replay (`verify_u.jsonl`, `all_ok`, final off 269) |
| `runs/e2run24-v4/` | The state-only v4 run on e2run24 up to the closing time of section 6.2: the chain so far (start table and step words), the driver log with private paths replaced by `<working tree>`, and the independent replay of those steps (`verify_u.jsonl`, `all_ok`) |
| `scripts/short-cycles/` | The exact K⁺ action (`kplus.py`), the single-overlap search for short pure elements (`short_cycles.py`), the two-level search for the orbits it leaves (`short_cycles2.py`) and the independent checker of their words (`check_cycles.py`); `SENIOR-REVIEW.md` section 11 |
| `data/short-cycles.json.gz` | The shortest verified element per orbit and kind that `short_cycles.py` found, with its word |
| `data/short-cycles-2.json.gz` | The same for `short_cycles2.py` on the orbits O00, O02, O06, O15, O17, O28 and O29 |
| `scripts/realign/` | Round 7: `realign.py` relocates the shorter candidates onto the cascade's buffers, certifies them on all slots and computes the bound; `restore_realigned.py` runs the stage 03 cascade with them and writes records that the stage 03 checker replays; `SENIOR-REVIEW.md` section 12 |
| `data/realign.json.gz` | The certified replacement per orbit (candidate, relocation, word, length, collateral) and the bound, as `realign.py` wrote it |
| `runs/realigned-cascade/` | The round 7 run on the committed synthetic inputs: logs, the summaries of the run and of the control run with no seed replaced, the stage 03 checker's report, and the commands with the SHA-256 of the files not stored |
| `IMPORT-MANIFEST.json` | The files stored gzipped, with the SHA-256 of their uncompressed bytes |

## Paths in commands

- `<research/jumbling>` is `research/jumbling` of this repository.
- `<repo>` is the repository root. The lattice scripts read it and never write to it.
- `<echain>` is `scripts/echain`.
- `<library>` is `data/library-merged.json` after decompression.

Decompress every `.gz` file before use, for example with `gunzip -k`. The runs used Python 3.13 with NumPy 2.5 on
Linux, with `ulimit -v 7000000` per process.

## Evidence

All of it is exact computation on synthetic geometry in headless Linux runs. None of it is evidence for Windows,
DirectX, input, long sessions or performance. A Codex senior review on 15 October 2026 checked the new proofs,
corrected several statements, proved Lemma C as Theorem C and proved Theorem M\* and the conditions for replacing a
seed of the lattice cascade ([SENIOR-REVIEW.md](SENIOR-REVIEW.md)); no human has reviewed them.
