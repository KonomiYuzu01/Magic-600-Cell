# LX-5 crate follow-up result

Experiment code; source and synthetic evidence only. Nothing here selects a route, an engine language, a dependency or an LQ answer; the owner decides.

| Field | Record |
| --- | --- |
| Card | `CARD.md`: LX-5 follow-up (RTE.54, RTE.55) under decision 13 of `docs/wiki/decisions/owner-decisions-2026-11-06.md` |
| Run UTC | 2026-11-14T17:10:27.965371+00:00 |
| Frozen LX-5 sources | candidate SHA-256 `743c96a70e8a445aa13ce4e6c6223bd8f34bbc9164083fdbd0f42da0830c2619`; source manifest `f004cae740af45839f798ddbf4cc0f1c9fcd774f9be09ad9fb9b536aedb6a1a2` |
| Frozen fixtures | `fixtures/MANIFEST.json` SHA-256 `75d464b4965c5baf4dc00bac58acf2e1aa0d3a958ca5c60c20e9f62b03e955bc` |
| Crate archives, checked against `Cargo.lock` | autocfg 1.5.1 `f2032f911046de80f0a198e0901378627c33f59ea0ac00e363d481118bd70a53`; num-bigint 0.5.1 `93e7820bc0a80a0238e650327316f929ba18d5be054b647490a3a6a339f3e7c0`; num-integer 0.1.47 `7ce2d95d4b3734dc35aa2f45e1aa22cd416814592a4f9d9205e11affd5b8e10b`; num-traits 0.2.19 `071dfc062690e90b734c0b2273ce72ad0ffa95f0c74596bc250dcfd960262841` |
| Licences, read from the verified archives | autocfg-1.5.1: Apache-2.0 OR MIT (LICENSE-APACHE, LICENSE-MIT); num-bigint-0.5.1: MIT OR Apache-2.0 (LICENSE-APACHE, LICENSE-MIT); num-integer-0.1.47: MIT OR Apache-2.0 (LICENSE-APACHE, LICENSE-MIT); num-traits-0.2.19: MIT OR Apache-2.0 (LICENSE-APACHE, LICENSE-MIT) |
| Tools | cargo `cargo 1.98.1 (797e8a9bc 2026-08-05)`; platform `win32`; python `3.15.1`; rustc `rustc 1.98.1 (48a229cea 2026-09-01)` |
| Correctness | exact; 3,374 of 3,374 frozen cases byte-identical to LX-5's reference |
| LX-5 Rust self-test | Rust self-test: 512 native i128 cases, 3 SHA-256 vectors, 2904 frozen differential cases |
| Smoke timing | exact; three samples in one series on a shared machine, a harness check that decides nothing |
| Complete / build removed | true / true |
| Decision / decider | open / owner |

| Size / corpus | Cases | Equal | Different |
| --- | ---: | ---: | ---: |
| 10000000-letters | 1 | 1 | 0 |
| 13231266-letters | 1 | 1 | 0 |
| 22500-bit | 70 | 70 | 0 |
| 2352984-letters | 1 | 1 | 0 |
| 2^256 | 70 | 70 | 0 |
| 2^64 | 70 | 70 | 0 |
| 672417-letters | 1 | 1 | 0 |
| 6775-digit | 70 | 70 | 0 |
| 871582-letters | 1 | 1 | 0 |
| empty-stream | 1 | 1 | 0 |
| length-166788405 | 22 | 22 | 0 |
| limb-boundaries | 88 | 88 | 0 |
| pell-22493-bit | 4 | 4 | 0 |
| pell-22495-bit | 4 | 4 | 0 |
| pell-256-bit | 4 | 4 | 0 |
| pell-3030-bit | 4 | 4 | 0 |
| pell-64-bit | 4 | 4 | 0 |
| pose-nc-22504-bit | 35 | 35 | 0 |
| pose-nc-260-bit | 35 | 35 | 0 |
| pose-nc-3022-bit | 35 | 35 | 0 |
| pose-nc-70-bit | 35 | 35 | 0 |
| random-1-bit | 264 | 264 | 0 |
| random-1024-bit | 264 | 264 | 0 |
| random-22500-bit | 88 | 88 | 0 |
| random-256-bit | 264 | 264 | 0 |
| random-3036-bit | 88 | 88 | 0 |
| random-31-bit | 264 | 264 | 0 |
| random-32-bit | 264 | 264 | 0 |
| random-33-bit | 264 | 264 | 0 |
| random-63-bit | 264 | 264 | 0 |
| random-64-bit | 264 | 264 | 0 |
| random-65-bit | 264 | 264 | 0 |
| random-96-bit | 264 | 264 | 0 |
| recorded-length-sum | 2 | 2 | 0 |

`candidate.rs` is LX-5's frozen Rust candidate with only its header, imports and `Int` type replaced. `Int` is a sign and a num-bigint `BigUint` and keeps every method's contract; `make_candidate.py` checks that everything from `fn rat` on is byte-identical. One known difference: the parser accepts only ASCII digits after an optional minus sign, while LX-5's own parser also let Rust's integer parser take some plus signs. The frozen inputs have none.

Equality is LX-5's: byte for byte over the canonical answer file, through LX-5's own comparer. Verify mode compares fresh records with the stored ones except the run time, the executable digest and the smoke timings, and regenerates this page from the stored record. The build uses cargo's default release settings at opt-level 2, like LX-5's `rustc -O`, without cross-crate link-time optimization.

Full mode: **not run here**. It belongs on the owner's quiet machine with a D6 conditions record, right after LX-5's own `measure.py`, so that both Rust candidates are timed by the same bench code under the same recorded conditions: `python -I -B work/experiments/language-lx/lx5-crate/check.py --full --conditions <record> --output <file>`.

Linux leg: **not run here**; it waits for the owner's WSL installation (decision 14 of the same record).

Cannot show: a speed threshold, that num-bigint is fit as a product dependency, whole-engine correctness, or Windows/DirectX, renderer, input or long-session behaviour.
