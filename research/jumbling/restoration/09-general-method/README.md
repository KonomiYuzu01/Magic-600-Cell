# 09. A state-only general method (v4), 15 October 2026

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

## Contents

| Path | What it is |
| --- | --- |
| `scripts/gm/` | The driver `gdrive.py`, its step scripts and `gen_scramble.py` |
| `scripts/echain/` | Modules the step scripts share (stage 01) |
| `scripts/lattice/` | The lattice restorer `restore_lattice_late.py` and the independent checker `verify_late.py` |
| `scripts/run/` | The run commands used on 15 October; paths are neutral tokens |
| `data/library-merged.json.gz` | The 36-macro correction library of the lattice phase (rank 34 over F2 and 2 over F5) |
| `earlier/stop62-860-to-0/` | stop62, 860 off pieces, to 0 in 14 clean steps, then to solved. Contents: the chain (start table and step words), the independent replay, and the lattice restore and verify reports with the solution record |
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
DirectX, input, long sessions or performance. The new proofs have no independent mathematical review.
