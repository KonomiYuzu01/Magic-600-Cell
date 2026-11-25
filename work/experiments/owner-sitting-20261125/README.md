# Owner-machine sitting of 25 November 2026: records

These files record the owner-machine sitting of 25 November 2026 that item 6 of the
[LP4 and LP5 exit decision](../../../docs/wiki/decisions/owner-decisions-2026-11-24-lp4-lp5-exit.md) agreed. The results
page is [owner-sitting-2026-11-25-results](../../../docs/wiki/evidence/owner-sitting-2026-11-25-results.md).

Every build was made from main `bb416a2c8336c1e73609dcf7aeb05799373770f9`, whose code equals LP4's candidate
`0208e78d` (the two differ only in `docs/`):

| Build | Identity | Made by |
| --- | --- | --- |
| Product (shell, host, core, LX-4 and editor libraries, runtime) | `c9-product-a9d908ad262ee306` | `python -B renderer/shell/build_product.py` |
| Route-B scene build (U4) | `work/build/c9-acceptance/20261125T165720Z` | `python -B renderer/acceptance/prepare_host.py --route surface` |

Each record in `results/` is written by `summarize_run.py` (in this folder) from one run folder that
`renderer/shell/launch.py --out` wrote. A record holds no local path. The raw logs stay on the owner's machine:
`events.jsonl`, the session folder and `godot.log`. Window captures are not published.

| File | Run | Who drove it | Launch |
| --- | --- | --- | --- |
| `results/owner-ev4-05.json` | Step 1, CT-EV4-05's keys, 17:16 to 17:18 UTC | The owner | `launch.py --out work/sitting-1125/ev4-05 --editor` |
| `results/owner-greybox.json` | Step 2, LP4-3's flow, 17:18 to 17:20 UTC | The owner | `launch.py --out work/sitting-1125/greybox --editor` |
| `results/integrator-greybox-ev4.json` | LP4-3's flow and CT-EV4-05's text legs, 17:27 to 17:37 UTC | The integrator, through Windows input (computer use) | `launch.py --out work/sitting-1125/greybox-2 --editor --resolution 2400x1400` |

The production command for each record:

    python -B work/experiments/owner-sitting-20261125/summarize_run.py work/sitting-1125/<run> --label <label> --out work/experiments/owner-sitting-20261125/results/<label>.json

## The unattended part (U4, U7, U8)

The owner started the chain in an administrator PowerShell, and it ran unattended from 17:47:53 to 22:20:07 UTC:

    & tools\perf\c9\unattended.ps1 -Steps U4,U7,U8 -SceneBuild work\build\c9-acceptance\20261125T165720Z -ProductBuild work\build\c9-product\20261125T165329Z -Suffix s1125

Its records are copied byte for byte from where the tools wrote them. `work/c9-runs/` and
`work/loop-memory/c9-acceptance/` are ignored by Git.

| File | What it is | Written by |
| --- | --- | --- |
| `results/unattended.log` | The chain's step log | `unattended.ps1` |
| `results/S4-summary-s1125.json`, `results/I_a-summary-s1125.json`, `results/I_b-summary-s1125.json`, `results/w3-summary-s1125.json` | U4's renderer gate summaries for W-J S4, I_a and I_b, and for W3 | `renderer_gate.py` through `unattended.ps1` |
| `results/w3-2-refusal-s1125.json` | The finalizer's refusal of W3's second run (run `20261125T183057626Z-w3`) | the L2 harness |
| `results/u7-s1125-faults.json` | LX-2's faults record (conditions 1 and 2) | `c600-lx2-faults` |
| `results/u7-s1125-lx2.json` | LX-2's analysis of U7 | `tools/perf/c9/lx2.py analyze` |
| `results/u8-s1125-e2.json` | Charter E2 over U8's turn keys | `tools/perf/c9/key_to_frame.py` |
| `results/u8-s1125-lx4.json` | LX-4's analysis of U8, with U4's W3 summary as the gate reference | `tools/perf/c9/lx4.py analyze` |

The raw run folders stay on the owner's machine: `events.jsonl`, `presentmon.csv`, the posts and the sessions.

## Frames confirmed as presented

`presented_share.py` (in this folder) counts, per run, the frames without a `presented` row. It reads only
`launch.json` and `events.jsonl`, and its record holds run IDs and counts.

| File | Runs |
| --- | --- |
| `results/presented-share-s1125.json` | Tonight's runs: the owner's steps 1 and 2, the integrator's run, U7, and U8 with Jumble off and on |
| `results/presented-share-c9-20261121.json` | The C9 sitting's U7 and U8 runs of 21 November 2026 (`c9-product-2efda50d54592f20`), read from their raw folders on the same machine |

The production commands:

    python -B work/experiments/owner-sitting-20261125/presented_share.py work/sitting-1125/ev4-05 work/sitting-1125/greybox work/sitting-1125/greybox-2 work/c9-runs/u7-s1125 work/c9-runs/u8-off-s1125 work/c9-runs/u8-on-s1125 --out work/experiments/owner-sitting-20261125/results/presented-share-s1125.json
    python -B work/experiments/owner-sitting-20261125/presented_share.py <C9 run folders u7-final, u8-off-final, u8-on-final> --out work/experiments/owner-sitting-20261125/results/presented-share-c9-20261121.json

A record states what the shell's logs show. It does not establish what the screen showed, the machine or the
conditions (PHS.64), and it is never a phase pass on its own (PHS.66).
