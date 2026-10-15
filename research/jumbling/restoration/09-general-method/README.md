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
its bounds, and a bound of 1,324,686,034 twists for the lattice phase.

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
| `runs/` | Records of the state-only v4 runs; added when the runs end |
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
corrected several statements, proved Lemma C as Theorem C and proved Theorem M\* ([SENIOR-REVIEW.md](SENIOR-REVIEW.md));
no human has reviewed them.
