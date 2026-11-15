# Decisive CPU timing window, 14 November 2026

Owner-machine timing of LX-5, the LX-5 crate follow-up and LX-6 by the B4-12 method: one smoke run, then three series
of 100 samples per row, each reported by its median and nearest-rank p99. Evidence class: performance on the owner's
machine, Windows, CPU only. No Linux leg ran. Nothing here is Windows/DirectX, renderer, input or long-session evidence,
and nothing here sets a speed threshold.

| Field | Record |
| --- | --- |
| Window | 2026-11-14 18:19:07 to 18:35:28 UTC (`window.json`) |
| Conditions | `conditions.json`, SHA-256 `8d3362c3bac9b9c3b7f1b6a9f586ea262b3377816b8d505a0b6d68d0a1d1bb75` |
| Runner | `../run.py`: refuses to start on battery, above 10% system CPU busy, with a busy Codex process or with a leftover build folder; runs the three steps one after another and records input during each |
| LX-5 | `lx5-full.json`, SHA-256 `d6ccc4752bdc4acc5620260c4cad4698ec0fee04068373d8c18d158b86732563`; 834.1 s; exit 0 |
| Crate follow-up | `crate-full.json`, SHA-256 `e8331356748e9897525d26be10b749a069503b8ed4eb2e4edbca8eda905b1e4d`; 113.8 s; exit 0 |
| LX-6 | `lx6-full.json`, SHA-256 `0272e273eea14fe98f812ace6b58173bd04da835ccf2e3dda650d976fa4b726a`; 33.2 s; exit 0; candidate SHA-256 `2bc30f5ebbce03f6e814223637c9cfb437f4fddd1cd387061ecbb80b3111ce6f` |
| Input during the runs | Recorded for all three runs. Under the runner's rule that made them leads. |
| Ruling | The owner accepted all three runs as decisive evidence, with the input recorded (owner, 14 November 2026). |

## LX-6: the engine hot path, Rust port against NumPy

Same frozen inputs; NumPy 2.3.5 on CPython 3.14.7 in the pinned engine environment. Each metric's output digest is
equal across the two backends. Times are the three series medians and the largest p99 of the three.

| Metric | NumPy medians | NumPy p99 | Rust medians | Rust p99 | Median of medians |
| --- | --- | --- | --- | --- | --- |
| apply_generator | 21.7, 20.35, 21.05 µs | 29.8 µs | 9.0, 8.0, 9.25 µs | 14.2 µs | Rust 2.3 times faster |
| apply_long_word | 10.81, 10.57, 10.25 ms | 11.79 ms | 2.07, 2.18, 2.03 ms | 2.56 ms | Rust 5.1 times faster |
| compose_macros | 13.08, 12.97, 13.07 ms | 16.80 ms | 1.90, 1.75, 1.81 ms | 2.55 ms | Rust 7.2 times faster |
| net_effect | 0.436, 0.434, 0.433 ms | 0.658 ms | 6.67, 6.59, 6.49 ms | 7.19 ms | NumPy 15.2 times faster |

The net_effect row is not like for like: the NumPy reference uses a warmed star cache that the Rust port does not have
(`../../lx6-engine-port/RESULT.md`). The port needs that cache before any net-effect comparison.

## LX-5 and the crate follow-up: exact arithmetic

Every candidate was exact on the frozen cases; the crate follow-up's correctness record is 3,374 of 3,374
(`../../lx5-crate/RESULT.md`). Each candidate has 190 timed rows of three series.

| Candidate | Sum of the 190 row medians | Spread of the three series medians per row: median, 90th percentile, largest |
| --- | --- | --- |
| Rust, the experiment's own limbs | 1,145.8 ms | 1.015, 1.58, 3.50 |
| Rust, `num-bigint` 0.5.1 | 340.7 ms | 1.001, 1.03, 2.00 |
| Node BigInt | 1,410.1 ms | 1.032, 1.52, 4.25 |
| .NET BigInteger | 136.6 ms | 1.041, 2.22, 5.83 |

This record does not attribute the larger spreads to particular rows; `lx5-full.json` and `crate-full.json` hold the
per-row series statistics (count, minimum, maximum, median and p99) and the raw-timing digests, not every sample.

Geometric mean, over sizes, of each candidate's row median divided by the own limbs' row median (below 1 is faster).
The JSON records also hold bits, cmp, shl, shr, q_conj and sum:

| Operation | `num-bigint` | Node | .NET |
| --- | --- | --- | --- |
| add, sub, mul | 0.23, 0.25, 0.21 | 0.31, 0.28, 0.26 | 0.32, 0.32, 0.31 |
| div, rem | 0.68, 0.24 | 0.88, 0.37 | 0.38, 0.34 |
| gcd | 0.90 (largest 183.6) | 0.52 | 0.07 |
| dec, hex | 0.17, 0.05 | 0.22, 0.06 | 0.27, 0.06 |
| q_add, q_mul, q_norm, q_sign | 0.43, 0.35, 0.20, 0.49 (largest 118) | 0.35, 0.32, 0.38, 0.50 | 0.09, 0.09, 0.05, 0.06 |
| rat_add, rat_mul, rat_cmp | 0.27, 0.24, 0.53 (largest 5.04) | 0.41, 0.34, 0.33 | 0.09, 0.09, 0.08 |
| word | 1.01 | 0.63 | 0.40 |

`num-bigint`'s slow rows are on random 22,500-bit inputs; its fast rows include the pose and Pell sizes:

| Row | Own limbs | `num-bigint` | Node | .NET |
| --- | --- | --- | --- | --- |
| gcd, 22,500-bit | 9.0 µs | 1.65 ms | 14.3 µs | 5.5 µs |
| q_sign, 22,500-bit | 18.3 µs | 2.16 ms | 14.8 µs | 15.7 µs |
| rat_cmp, 22,500-bit | 0.68 ms | 3.41 ms | 0.10 ms | 0.21 ms |
| q_add, 22,500-bit | 5.60 ms | 11.79 ms | 1.14 ms | 2.76 ms |
| q_mul, 22,500-bit | 9.83 ms | 12.98 ms | 1.70 ms | 5.36 ms |
| q_mul, pose 22,504-bit | 159.5 ms | 13.0 ms | 251.1 ms | 7.5 ms |
| q_norm, Pell 22,493-bit | 24.5 ms | 1.72 ms | 30.4 ms | 0.57 ms |
| word, 13,231,266 letters | 81.5 ms | 81.3 ms | 37.1 ms | 24.9 ms |

The experiment's own limbs are no audited general-purpose library, and no .NET engine port exists.

## What this record supports and what it does not

It supports the owner's LQ8 choice of Rust for the engine and the interpreter's host (architecture chapter 11,
RTE.17), with the big-integer implementation chosen in LP2 on these rows. It cannot show a speed threshold,
whole-engine or end-to-end correctness, the Linux legs, or any product-dependency approval beyond the crate class the
owner allowed for experiment code (owner, 6 November 2026, decision 13).
