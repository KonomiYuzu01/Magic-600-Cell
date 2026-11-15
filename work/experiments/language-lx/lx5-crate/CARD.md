# LX-5 crate follow-up card

Experiment code, NOT production code. Evidence classes: source and synthetic; a full run on the owner's quiet machine
adds a timing record under the D6 protocol. Nothing here decides the route, the engine language, a dependency or LQ8;
the owner decides.

| Field | Card |
| --- | --- |
| Question | Does LX-5's frozen Rust candidate stay exact when a maintained big-integer crate replaces its own limbs, and how do the per-operation timings compare? |
| Parent | LX-5 (RTE.54, RTE.55), frozen at `1e7d0074`. Its folder, fixtures, reference, comparer and bench code are used unchanged. |
| Owner allowance | Decision 13 of `docs/wiki/decisions/owner-decisions-2026-11-06.md`: a `num-bigint` class crate after the integrator's licence check; a copyleft licence would need the owner. |
| Candidate | `candidate.rs`, written by `make_candidate.py`: LX-5's `candidate.rs` with only the header, imports and `Int` type replaced. `Int` is a sign and a num-bigint 0.5.1 `BigUint` and keeps every method's contract. |
| Dependencies | num-bigint 0.5.1, num-integer 0.1.47, num-traits 0.2.19 and autocfg 1.5.1 (build script only), pinned by version and SHA-256 in `Cargo.lock`. The check records each archive's licence field and licence files from the verified archive. |
| Build | `cargo build --release --offline --locked` in a private cargo home that holds only the verified archives. Opt-level 2, like LX-5's `rustc -O`; no network, inherited cargo configuration or compiler flags. |
| Inputs | LX-5's frozen fixtures (3,374 cases), reference and word streams, checked against their manifests on every run. |
| Acceptance | Byte-identical answers on every frozen case; LX-5's Rust self-test passes; the smoke timing completes exact; `build/` is removed; in verify mode this folder and LX-5's stay unchanged. |
| Kill criteria | Any exact difference removes the candidate. A timeout or another reached limit is no pass. |
| Known difference | The parser accepts only ASCII digits after an optional minus sign. LX-5's own parser also let Rust's integer parser take some plus signs; the frozen inputs contain none. |
| Cannot show | A speed threshold, fitness of num-bigint as a product dependency, whole-engine correctness, or Windows/DirectX, renderer, input or long-session behaviour. |
| Decision / decider | open / owner |

## Commands

From the repository root. `-I` applies because the check reads downloaded archives.

    python -I -B work/experiments/language-lx/lx5-crate/make_candidate.py
    python -I -B work/experiments/language-lx/lx5-crate/check.py
    python -I -B work/experiments/language-lx/lx5-crate/check.py --record
    python -I -B work/experiments/language-lx/lx5-crate/check.py --full --conditions <d6.json> --output <file>

The first verifies `candidate.rs`; `--write` regenerates it. The second is the acceptance check and writes nothing. The
third refreshes `check-results.json`, `smoke.json` and `RESULT.md`. The fourth is the measurement: three series of 100
samples per measured case. It belongs on the owner's quiet machine right after LX-5's own `measure.py`, so both Rust
candidates are timed by the same bench code under the same recorded conditions.

The archives are fetched once, with network access, into a cargo home outside the experiment (POSIX shell syntax):

    CARGO_HOME=work/lx5-crate-fetch/cargo-home cargo fetch --locked --manifest-path work/experiments/language-lx/lx5-crate/Cargo.toml

The check copies only the archives whose SHA-256 matches `Cargo.lock` into its private cargo home; `--crates` names
another fetched cargo home.
