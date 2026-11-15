# LX-6 result: all seven frozen oracle cases equal

| CT-RTE-05 field | Record |
| --- | --- |
| LX card / experiment ID | LX-6, RTE.56 to RTE.58 / E-2.4-L6 |
| Owner allowance | `docs/wiki/decisions/owner-decisions-2026-11-05.md`, decision 2, and this implementation packet |
| Time box | what can be done now; one wrapper call of at most two hours |
| Frozen candidate | Base `1e7d0074df96cbeb05f55282cf8f6c03fbf4a4ff`; candidate SHA-256 `2bc30f5ebbce03f6e814223637c9cfb437f4fddd1cd387061ecbb80b3111ce6f` |
| Source and input digests | Per-file source hashes and build flags: `CANDIDATE.json`; frozen reference, asset, specification and compressed-case hashes: `INPUTS.json`. `check.py` and `measure.py` changed after the oracle run, to make the check leave every file unchanged: their entries in `CANDIDATE.json` are the first run's digests, and `HARNESS.json` pins the bytes that run now, which `measure.py` verifies |
| Oracle input manifest | `work/experiments/language-lx/inputs/MANIFEST-oracle.json`, SHA-256 `eba3c43f476396ecebee1f3c28dff0e4fee68e131fe7ef278e69734205343612` |
| Exported model SHA-256 | `223edad95f991ea1ada8ed3d464b21909979f0122aa837fc9e3d7e3541949e01` |
| Rust executable SHA-256 | `14030e03922cc32225287bb34e20a9227adaf06d9422a153441c7bb7fdc1ce72` (generated and removed) |
| Commands and versions | Commands below; exact compiler flags and version output in `CANDIDATE.json` |
| Evidence class | **source**: oracle equality on this frozen candidate; **synthetic**: type/limit checks and deliberately corrupted trace control. Smoke timings are shared-machine leads only. |
| Kill criteria | Any oracle difference stops the spike until explained, preserving its first case, step and digests. Timing output differences invalidate that run. A reached time box is no pass. |
| Actual result | Acceptance exit **0** on 5 November 2026: all 3,201 steps in O01 to O07 equal, no omissions; all four timing smoke outputs equal |
| Decision / decider | **open / owner**. No route, engine language, syntax or LQ answer selected. |

The Rust port uses every labelled slot and every legal generator. It validates the complete recipe with the 0.4
limits before any effect, expands words and stars chronologically, applies permutations simultaneously, derives
sorted source-to-destination nets, and computes complete support, conflicts, moved counts and progress. It checks
protection against the net support before applying the effect. All state, expansion, net and progress digests are
computed in Rust. Python exports the frozen arrays, supplies raw decompressed case JSON and invokes the comparator.
The NumPy import is confined to the explicitly pinned reference timing worker.

The frozen reference source hashes match the oracle manifest. The exporter verified every model asset listed by
`assets/manifest.json`, the full frozen-input manifest and all seven compressed-case digests before running; the
acceptance check verified them again after running. The case bytes were never regenerated or edited.

| Case | Required steps | Comparator exit | Trace SHA-256 |
| --- | ---: | ---: | --- |
| O01 | 2,400 | 0 | `f2c14bfeffadcb4bf7ac2bcb2c86a0b04e8a4d59a5e55025254a3170c6e771e1` |
| O02 | 500 | 0 | `0d9d1f0b06f31e0221eff309ef4ec08d3ce9b2724e07aed31ddc64aacae99a73` |
| O03 | 210 | 0 | `d5c8674cb48aaab4c8755889a3ffdfc098071252436b53736018fd00802c62fd` |
| O04 | 25 | 0 | `2781da156ce116654b2edc7f0c3f021b79290ac394870f7cc0e433d1d3729e59` |
| O05 | 50 | 0 | `70f4387855a766473fc39351a1e0d5d2aed0efbc9595276acc05426d5463e3f0` |
| O06 | 6 | 0 | `b6b882f13efc24f3bdc2aa9cd087c876f0bfbd7dc8e7505470c73300ed47c6f2` |
| O07 | 10 | 0 | `76bde3dea914617653479c19571057114c3fe0604b88ce0803cf4f06b469e606` |

Each trace passed through `tools/oracle/compare.py`, comparing every required field at every step, not just the final
state. None of these cases declares `optional_fields`. The port supplies every field and the driver rejects any
omission. `ORACLE_RESULTS.json` retains the per-case input/trace hashes, sizes and comparator reports.
The largest trace, O01, was 4,458,448 bytes; all raw traces, the binary model and compiler products were removed.
The check's `finally` block also removes `build/` on failure.

No candidate oracle difference occurred; first differing case/step/digests: **none**. The deliberately corrupted
copy of O07's trace changed only step 8's state hash to zeros. The comparator returned exit **1**, index **8**, as
required. This expected synthetic negative control is separate from the seven unchanged candidate traces.
The Rust self-checks passed SHA-256 known vectors (empty, `abc`, one million `a` bytes and the oracle `[1,-1]`
expansion vector), JSON type preservation and canonical string escaping, boolean/string/float rejection in recipe
fields, default star sign, inverse cancellation, 256/257 recipe parts, 100,000/100,001 word lengths, and
3,000,000/3,100,000 expanded primitive limits. These checks add coverage to O07, whose frozen non-integer is 1.5;
they do not change or extend the frozen cases.

The acceptance command actually run was:

```text
python work/experiments/language-lx/lx6-engine-port/check.py
```

The first run's acceptance passed and was refused only because its check rewrote the timing field in `SMOKE.json`; this run landed the same work with a check that writes nothing outside its temporary `build/` folder and leaves the experiment unchanged.

It builds with direct `rustc`, runs self-checks and all seven cases, then invokes
`measure.py --smoke` with the same Python driver, checks digests again and removes build products. The scripts
suppress Python bytecode so imports leave no generated files outside the experiment.
Tools observed: CPython **3.15.1** driver; CPython **3.14.7**, NumPy **2.3.5** reference; rustc **1.98.1**
(`48a229cea`, host `x86_64-pc-windows-msvc`, LLVM **22.1.8**). The installed Visual Studio toolchain reports
**18.10.3**, linker file/product version **14.51.36260.0**. No crate, registry, download or installation was used.

`SMOKE.json` contains exactly one sample for each of apply-generator, apply-long-word, compose-macros and net-effect,
for each backend, and their equal output digests. The run printed **SMOKE RUN**. It is not a speed judgement: other
runs share this machine, and D6 power, performance, V-Sync and overlay conditions were not established. In particular,
the retained NumPy net path uses a warmed star cache, whereas this Rust candidate replays primitives without one.
`CARD.md` defines each timing boundary and input selection. Full mode includes a separate smoke followed by three
series of 100 samples, retains raw samples and reports each series' median and nearest-rank p99 per metric/backend.
The full-series path is implemented but **not run here**.

Integrator work still open, outside this sandbox:

1. On the quiet owner's Windows machine, record the actual D6 conditions in the experiment's `conditions.json`
   using the fields listed in `CARD.md`, then run exactly:

   ```text
   python work/experiments/language-lx/lx6-engine-port/measure.py --conditions work/experiments/language-lx/lx6-engine-port/conditions.json
   ```

   **Not run here.** Keep `MEASUREMENTS.json`, the recorded conditions and candidate identity together. A changed
   candidate needs its own oracle run; this report cannot be relabelled as evidence for it.

2. For the later WSL/Linux correctness leg, use a separate checkout and an already available Rust toolchain, preserving
   that run's records separately:

   ```text
   python3 work/experiments/language-lx/lx6-engine-port/run_oracle.py
   ```

   **Not run here.** This command needs Python stdlib and Rust only. It makes no Linux timing claim. The Windows
   pinned NumPy environment is not asserted to be a Linux reference environment.

Whole-engine correctness, persistence, native input, Windows/DirectX rendering, GPU execution, long sessions and
end-to-end E2 are **not run here** and are outside this port. The card's "Cannot show" remains: whole-engine
correctness, persistence or input correctness, end-to-end E2, or a language choice from a speed threshold.
Independent integration review and the owner's language decisions remain open.
