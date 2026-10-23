Implemented P5 in:

- [check_lattice.py](research/jumbling/restoration/13-tutorial-runner/scripts/check_lattice.py): streaming readers, independent exact lattice model, guarded dispatch, tree comparisons and literal replay.
- [test_check_lattice.py](research/jumbling/restoration/13-tutorial-runner/scripts/test_check_lattice.py): the required fixtures, mutations and worktree checks.

The prescribed acceptance command exited **0**: **18 tests passed**, with **93.702 s total elapsed**. The worktree remained unchanged during acceptance, including ignored files. Stopped as requested.

| Test | Seconds |
|---|---:|
| Independence | 0.022 |
| Malformed files/range bounds | 0.002 |
| Decoders/overlapping ranges | 0.457 |
| Model guard/off-lattice refusal | 0.846 |
| Invisible rotation, retained word | 1.839 |
| Three model prefixes | 2.245 |
| Two positive replay steps | 0.942 |
| w9 non-solved digest | 1.172 |
| Changed correction/partial state | 1.036 |
| Changed retained/repeated letters | 1.800 |
| Truncation/count/hash mutations | 0.980 |
| Forced fast-path refusal | 0.280 |
| Row guards/end claims | 3.524 |
| Tree reference mutations | 2.236 |
| Sanitized failures/end frames | 0.948 |
| Representation switching | 0.768 |
| Tree location/read errors | 5.840 |
| Tree structure/depth | 0.016 |

Fixture construction took **61.701 s**; suite time including that setup was **86.749 s**.

The base interfaces matched §4.1. Compatibility names and signatures remain; replay adds optional `stats` and replaces the stub failure. Word hashes use the packet’s lowercase guard, stricter than I0’s syntax guard. No I0 interface change was needed.

Tree location uses `Path(path).parents[n - 1]`, where `n` counts the relative word path’s parts, verifies recombination, then resolves the tree within that root.

Cloud research timings: test D replay **1,770.7 letters/s**, including files, conversions and end checks. Conversion into the model: **0.211932 s**; out: **0.168668 s** averaged over three conversions.

No fixture-number disagreements were found. The integrator’s three I0 test updates and P3/I1 integration remain outside this packet. Windows/DirectX checks were not run.