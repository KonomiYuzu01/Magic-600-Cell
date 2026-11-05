# LX-5 exact-arithmetic result

Experiment code and source/synthetic evidence only. No route, engine language, syntax, dependency or LQ is selected.

| CT-RTE-05 field | Record |
| --- | --- |
| LX card / experiment ID | LX-5 (RTE.54, RTE.55) / E-2.4-L5 |
| Owner allowance | `docs/wiki/decisions/owner-decisions-2026-11-05.md`, decision 2; assigned implementation packet |
| Time box | what can be done now; one wrapper call of at most two hours |
| Source scope | Packet base `1e7d0074`; only `work/experiments/language-lx/lx5-arithmetic/` added |
| Run UTC | 2026-11-05T06:48:43.420184+00:00 |
| Frozen candidate / harness digests | `SOURCE-MANIFEST.json`, SHA-256 `f004cae740af45839f798ddbf4cc0f1c9fcd774f9be09ad9fb9b536aedb6a1a2`; executable digests in `check-results.json` |
| Frozen input digests | `fixtures/MANIFEST.json`, SHA-256 `75d464b4965c5baf4dc00bac58acf2e1aa0d3a958ca5c60c20e9f62b03e955bc`; decoded reference `529252f51f3699be09c5d5a7a66fd6864f88e666cfd393ebd0a3f780d126d18b` |
| Evidence class | source and synthetic; smoke timings are leads on a shared, sandboxed machine |
| Kill criteria | Any exact difference removes that candidate globally; keep operation and operands. Timeout leaves affected sizes untested. A reached limit is no pass. |
| Actual result | Reference self-test: pass; comparer planted-fault tests: pass; report complete: true |
| Decision / decider | open / owner (all route, LQ8, LQ9, syntax, dependency and licence decisions) |

Record of runs: The first run's acceptance passed and was refused only because its check rewrote `RESULT.md`, `check-results.json` and `smoke.json`; this run landed the same work with a check that writes nothing in its verify mode.

Acceptance command: `python work/experiments/language-lx/lx5-arithmetic/check.py`.

Only `python work/experiments/language-lx/lx5-arithmetic/check.py --record` refreshes the stored check artifacts. Verify mode compares fresh results, requires all candidates to remain exact, removes `build/` and checks every retained file's SHA-256. The frozen card is unchanged; this packet's acceptance additionally rejects completed negative numeric results.

Check comparison excludes only `CHECK_VOLATILE_FIELDS` in `check.py`: the run timestamp, the smoke digest and the Rust executable digest (two clean local builds differed; the .NET assembly was byte-identical). Smoke comparison excludes its timestamp, timing summaries/samples and raw timing digest/byte count; sample counts, series IDs and all correctness and identity fields stay compared. `RESULT.md` is checked against its stored check record.

Tools: dotnet_runtime `10.0.12`; dotnet_sdk `10.0.401`; node `v24.21.0`; platform `win32`; python `3.15.1`; rustc `rustc 1.98.1 (48a229cea 2026-09-01)`.

The common frozen list has 3,374 cases. Equality is byte-for-byte over canonical answer files, including signs, reduced denominators, decimal/hex encodings, full letter histograms, counts, sums and stream SHA-256. Large answer files are deleted; their digests and regeneration command are retained. The frozen reference is gzip-compressed with mtime 0.

| Candidate | Overall status | Equal cases | Different cases |
| --- | --- | ---: | ---: |
| node | exact | 3374 | 0 |
| dotnet | exact | 3374 | 0 |
| rust | exact | 3374 | 0 |

Passing correctness candidates: node, dotnet, rust. This is no dependency approval.

| Size / corpus | Cases | Node | .NET | Own Rust limbs |
| --- | ---: | --- | --- | --- |
| 10000000-letters | 1 | 1/1 exact | 1/1 exact | 1/1 exact |
| 13231266-letters | 1 | 1/1 exact | 1/1 exact | 1/1 exact |
| 22500-bit | 70 | 70/70 exact | 70/70 exact | 70/70 exact |
| 2352984-letters | 1 | 1/1 exact | 1/1 exact | 1/1 exact |
| 2^256 | 70 | 70/70 exact | 70/70 exact | 70/70 exact |
| 2^64 | 70 | 70/70 exact | 70/70 exact | 70/70 exact |
| 672417-letters | 1 | 1/1 exact | 1/1 exact | 1/1 exact |
| 6775-digit | 70 | 70/70 exact | 70/70 exact | 70/70 exact |
| 871582-letters | 1 | 1/1 exact | 1/1 exact | 1/1 exact |
| empty-stream | 1 | 1/1 exact | 1/1 exact | 1/1 exact |
| length-166788405 | 22 | 22/22 exact | 22/22 exact | 22/22 exact |
| limb-boundaries | 88 | 88/88 exact | 88/88 exact | 88/88 exact |
| pell-22493-bit | 4 | 4/4 exact | 4/4 exact | 4/4 exact |
| pell-22495-bit | 4 | 4/4 exact | 4/4 exact | 4/4 exact |
| pell-256-bit | 4 | 4/4 exact | 4/4 exact | 4/4 exact |
| pell-3030-bit | 4 | 4/4 exact | 4/4 exact | 4/4 exact |
| pell-64-bit | 4 | 4/4 exact | 4/4 exact | 4/4 exact |
| pose-nc-22504-bit | 35 | 35/35 exact | 35/35 exact | 35/35 exact |
| pose-nc-260-bit | 35 | 35/35 exact | 35/35 exact | 35/35 exact |
| pose-nc-3022-bit | 35 | 35/35 exact | 35/35 exact | 35/35 exact |
| pose-nc-70-bit | 35 | 35/35 exact | 35/35 exact | 35/35 exact |
| random-1-bit | 264 | 264/264 exact | 264/264 exact | 264/264 exact |
| random-1024-bit | 264 | 264/264 exact | 264/264 exact | 264/264 exact |
| random-22500-bit | 88 | 88/88 exact | 88/88 exact | 88/88 exact |
| random-256-bit | 264 | 264/264 exact | 264/264 exact | 264/264 exact |
| random-3036-bit | 88 | 88/88 exact | 88/88 exact | 88/88 exact |
| random-31-bit | 264 | 264/264 exact | 264/264 exact | 264/264 exact |
| random-32-bit | 264 | 264/264 exact | 264/264 exact | 264/264 exact |
| random-33-bit | 264 | 264/264 exact | 264/264 exact | 264/264 exact |
| random-63-bit | 264 | 264/264 exact | 264/264 exact | 264/264 exact |
| random-64-bit | 264 | 264/264 exact | 264/264 exact | 264/264 exact |
| random-65-bit | 264 | 264/264 exact | 264/264 exact | 264/264 exact |
| random-96-bit | 264 | 264/264 exact | 264/264 exact | 264/264 exact |
| recorded-length-sum | 2 | 2/2 exact | 2/2 exact | 2/2 exact |

`check-results.json` has per-size counts, candidate output digests and any exact differences with their operation, operand references and values. `operation-results.tsv` has each operation/size status. `smoke.json` has each measured operation/size's median, nearest-rank p99 and three raw samples; no threshold is applied.

The first case for each measured operation/size is timed after one warmup; input parsing and process startup are excluded, canonical answer formatting is included, and word measurements include file I/O. Random and limb-boundary cases check correctness only. Three smoke samples do not estimate a performance tail; their p99 is simply their maximum.

Synthetic pose recipe: exact column-vector powers `(g H)^n` of the first two J1 negative-control journal matrices. The trace 4/3, g squared, and all twelve recorded heights are checked before extension. The largest-component entry (row-major first tie) is used. Powers / actual maximum component bits: 44/70, 164/260, 1906/3022, 14197/22504. These are synthetic extensions, not replayed states. The separate Pell cases give exact, extremely small sign margins without a float decision.

Word coverage: recorded word 00014 is read in full (2,352,984 letters); all 53 files are concatenated once in recorded order and read as one 13,231,266-letter stream. The 10,000,000-letter bound uses the frozen xorshift32 recipe. The 672,417- and additional 871,582-letter cases are explicitly labelled prefixes of word 00014: they cover those literal sizes, not the earlier E2/T6 witnesses. Every stream is consumed in 64 KiB chunks with persistent digest and counters. The 166,788,405 length and the recorded-length sum are integer arithmetic only, never a claimed replay.

Rust is own experimental code with no crate and an ongoing audit burden, including its own SHA-256. Multiplication is schoolbook O(n*m) in 32-bit limbs. Division is normalized Knuth Algorithm D with quotient estimation, correction and add-back; it uses O((n-m+1)*m) limb work for n >= m >= 2, O(n+m) scratch space, plus normalization. A one-limb divisor uses a linear scan. Euclidean gcd and decimal formatting repeatedly use division. This implementation is not an audited general-purpose bigint dependency.

Rust self-test: Rust self-test: 512 native i128 cases, 3 SHA-256 vectors, 2904 frozen differential cases.

Licence evidence and unverified recollections are separated in `LICENCES.md`. The unfetched num-bigint, dashu, ibig, malachite and rug candidates remain an open owner question; copyleft needs the owner. No package, crate, network access or installation was used.

Full mode: **not run here**. D6 power source, Windows power mode, vendor performance mode, driver V-Sync, overlays and frame generation were not recorded; no decisive timing is claimed. Record them before an owner-machine measurement. Run `python work/experiments/language-lx/lx5-arithmetic/measure.py` for three series of 100 samples per metric; optionally pass `--conditions work/experiments/language-lx/lx5-arithmetic/measurement-conditions.json` after writing a public D6 record. The full-mode harness has not itself been exercised here.

Linux/WSL leg: **not run here**. With the same candidates already available, run `python3 work/experiments/language-lx/lx5-arithmetic/check.py` in a Linux checkout. No Linux install is authorized by this packet. Integrator review is still open before integration; no Git index, refs, configuration or hooks were changed.

Cannot show: that the synthetic entries occurred in a saved state, that the largest length was played, a general speed threshold, or owner approval of a dependency (the card's limits). It also cannot establish whole-engine correctness, legal witness replay, Windows/DirectX, renderer, GPU, native input or long-session behavior. GPU and replay runs are outside LX-5, so there is no LX-5 GPU command to run.
