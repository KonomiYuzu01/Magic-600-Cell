# C9 sitting on the frozen candidate: records

These files record LP3's C9 sitting on the owner's machine. The sitting ran on the frozen candidate, main
`013900ffc013d4c63f0dd8c39d83836de2b7c443`, and on builds made from that commit (owner, 21 November 2026:
[decision](../../../docs/wiki/decisions/owner-decisions-2026-11-21-ceilings-e3.md)). The results page is
[c9-sitting-results](../../../docs/wiki/evidence/c9-sitting-results.md).

- The unattended part, U1 to U8, ran from 11:39 to 16:57 UTC on 21 November 2026 with
  `tools/perf/c9/unattended.ps1 -Suffix final`.
- The attended part, A1 to A3, ran from 01:52 to 01:57 UTC on 22 November 2026 with `renderer/shell/launch.py`.

Builds:

| Build | Identity |
| --- | --- |
| Product (shell, host, core) | `c9-product-2efda50d54592f20` |
| Route-B scene build (U4) | `fb7dbd6e82e5edd7e7beb1447b90ac27f081e997f9101e4fd3681ddfb02afab0` |
| Stage build (U5) | `f5b7531a5cd9b6dfd49e44cb1c4f2686c523fe588de77098b4410ab92c1dab21` |

Each file in `results/` is copied byte for byte from the run folder. Raw logs and captures stay on the owner's
machine: `events.jsonl`, `input.jsonl`, `presentmon.csv`, the session folders and the console output, which names
local paths. Each record names its inputs with their SHA-256 values.

| File | Step | Made by |
| --- | --- | --- |
| `final-a1-launch.json`, `final-a2-launch.json`, `final-a3-launch.json` | A1, A2, A3 | `renderer/shell/launch.py --out work/c9-runs/final-aN` (A2 with `--echo`) |
| `final-a3-e2.json` | A3, charter E2 | `python -B tools/perf/c9/key_to_frame.py work/c9-runs/final-a3 --out work/c9-runs/final-a3-e2.json`, after the capture was copied into the run folder as `presentmon.csv` |
| `unattended.log` | U1 to U8 | `unattended.ps1`: one line per step with its exit code and output |
| `u2-1-probe-20261121T120338Z.md`, `u2-2-probe-20261121T120451Z.md`, `u2-3-probe-20261121T120604Z.md` | U2 | `summary.md` of each present-probe run (`work/experiments/c9-present-probe/`) |
| `u3-final-skew.json` | U3, route B check B2 | `tools/perf/c9/ui_skew.py` |
| `S4-summary-final.json`, `I_a-summary-final.json`, `I_b-summary-final.json`, `w3-summary-final.json` | U4 | `tools/perf/renderer_gate.py` over three runs each |
| `w3-stages-summary-final.json` | U5, RPI.79 | `tools/perf/renderer_gate.py` over three runs |
| `e3-final-warmup.json`, `e3-final.json` | U6, charter E3 | `tools/perf/c9/cold_start.py`: the untimed warm-up launch alone, then the three timed runs |
| `u7-final-faults.json`, `u7-final-lx2.json` | U7, LX-2 | `c600-lx2`'s faults record and `tools/perf/c9/lx2.py analyze` |
| `u8-final-e2.json`, `u8-final-lx4.json` | U8, charter E2 and LX-4 | `tools/perf/c9/key_to_frame.py` and `tools/perf/c9/lx4.py analyze` over both puzzles' runs |

A record states what its logs show. It does not establish the machine, the conditions or the build (PHS.64), and it
is never a phase pass on its own (PHS.66).
