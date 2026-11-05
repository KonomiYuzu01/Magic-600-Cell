# LX-9 exact-request follow-up result

**Parts 1 to 4 completed; acceptance passed. Decision: open, owner.**
The two certified interpretations of the reported input both select the
90-degree rotation. The 45-degree tie is exact. The repaired D argument gives
termination with the qualifications below, and B's two free-mode meanings
produce different results on a concrete request. No option is selected.

| CT-RTE-05 field | Record |
| --- | --- |
| LX card / experiment ID | LX-9, RTE.64 to RTE.66 / E-2.4-L9; exact-request follow-up |
| Owner allowance | `docs/wiki/decisions/owner-decisions-2026-11-05.md`; assigned implementation packet |
| Time box | what can be done now; one wrapper call of at most two hours |
| Frozen candidate | `CANDIDATE.json`, SHA-256 `9f5bc1b7348c58ff019cb1326b32d5897e9b9afce3a3ef74a00b4d1f635542d0` |
| Frozen input and result manifest | `MANIFEST.json`, SHA-256 `2e86e84dd9ec84d963f2ca59fdb5b509bd5645521f9110a097c2739ca471c481` |
| Request input digest | `INPUTS.json`, SHA-256 `0e72cc9edde16661373067f7d9158942c72c0d4ef91dade31399ceb61f3767b8` |
| Captured menu digest | `MENU.json`, SHA-256 `ba53093ed3b7c3bd0845af57694512f4c7ac850eac8c74570610b3c19ddd7df1` |
| Commands / tool versions | Commands below; Windows CPython 3.15.1; rustc 1.98.1 (48a229cea 2026-09-01), x86_64-pc-windows-msvc; standard libraries only |
| Evidence class | source and synthetic; actual Windows headless execution, no native-input, DirectX or performance claim |
| Kill criteria | Uncertified sign, disagreement between computations, changed frozen bytes, failed tool or reached limit means no pass |
| Actual result | Full acceptance exited 0; all 12 stored tables reproduced with independent checks; `build/` removed; retained-file snapshot unchanged |
| Decision with decider | **open**, owner; A/B/C/D, free-mode meaning, route, language, syntax and every LQ remain unselected |

Source scope: the packet's base is `b91fd208`; this experiment's own sources,
proof, request description and card are pinned by `CANDIDATE.json`. The check
pins no moving chapter, decision page or card outside this experiment. The
original LX-9 folder and its manifest are unchanged.

## 1. Certified platform-difference case

For each signed coordinate axis, N=1 and D=16, all 432 Cayley tuples were
compared under both semantics. The exact degree input is
5035942778341233/70368744177664; the captured binary64 radian input is
175787564848171/140737488355328.

Let S be squared normalized quaternion dot product, so squared Frobenius
distance is 8(1-S). For candidates (1,a) and (2,a):

| Semantics | Certified interval for S(1,a)-S(2,a) | Sign / global winner |
| --- | --- | --- |
| Real rotation by exact dyadic degrees | [291814066782,291814066783] / 2^96 | positive; (1,a), 90 degrees |
| Real rotation by binary64 radian value | [55024008088,55024008089] / 2^96 | positive; (1,a), 90 degrees |

`case.json` and `PROOF.md` give every pi, angle, sine, cosine and score interval
and the rational remainder bounds. `case_axes.tsv` has all twelve global
certificates. An independent pi identity, alternating trigonometric bounds and
half-angle score formula reproduce the retained intervals exactly. This proves
these cases only; it does not establish correctness of Windows' float path in
general or redefine A1 to mean the rounded target quaternion.

## 2. Exact tie and proposed D repair

At +x, exactly 45 degrees, N=1,D=16, (2,1,0,0) and (3,1,0,0) are globally
nearest. Their score difference is (sin(t)-cos(t))/10=0, and denominator 2 wins.
Both independent interval routes continue to contain zero through the
256-bit probe. Exact equality, rather than more interval precision, decides it.

The nontrivial-root degree bound <=16 gives 21 possible root orders and **152
angles modulo 360 degrees**. Every listed angle is checked against a brute
exact half-angle computation; all winners and complete minimiser lists agree.
The full list is in `exceptional.tsv`, with its derivation in `PROOF.md`.

The proposal needs two explicit qualifications: detect identically-zero
comparisons before refinement, and do not assume every axis radical lies in
one fixed cyclotomic field. The implemented equality test handles the latter
by X^2=qY^2 in Q(zeta_1440), plus an essential opposite-sign guard. Four exact
radical controls check those branches. With these qualifications the argument
proves termination; **it gives no affordable precision or admission-cost bound**.
It is not a general product D implementation.

Largest adaptive deciding precision: **64 output enclosure bits** for part 1;
all exceptional-angle nonzero comparisons resolved at 16 bits. Analytic guard
bits, the fixed 96-bit retained certificates and the 256-bit nondeciding tie
probe are distinguished in `PROOF.md`. These counts are not timings.

## 3. Option B prototype

`REQUEST.md` specifies exact dyadic C bytes, an explicit canonical half-turn H
branch, finite/signed-zero/size validation and bound menu-letter M bytes.
The two free variants are both implemented: direct R(omega), and quantisation
against A1's bounded grid. For omega=(3/8,0,0), direct preserves 3/8 and the
N=1,D=16 grid chooses 1/3.

The prototype captures the repository's ordered 60-element I_a menu with its
existing set and order hashes. All 60 exact self-selections pass. Eleven C/H
requests pass matrix/trace versus projective-quaternion comparisons, and 29
validation rows agree between independent decoders. Surd signs are checked
by separate norm and integer-square-root methods.

The quarter-turn (1,0,0) is refused as a direct menu request, and as a drag
ties indices 15,29,40,43 and selects 15. Reversing the menu keeps its set hash
but changes its order hash; the selected rotation becomes the old index 43.
Old bound menu requests are refused. Reconstructed identical bytes give
identical bound choices under the same operation, variant, bounds and menu.

Most exact menu rotations are not dyadic Cayley vectors. Their self-selection
fixtures use exact matrices and M letters, not fictional dyadic encodings.
**Identical gestures need not produce identical bytes** unless the producer
of omega is specified. That producer and any actual contract/migration remain
outside this prototype.

## 4. Default grid

All six constructed requests passed. These are exact C/H targets on either
side of three certified global bisectors, within 2^-32 in the stated
Cayley/axis-slope coordinate. They are not a natural-input corpus or a claim
about binary64-degree ulps. There were **215,693,872 complete tuple visits**.

| Boundary pair | Full tuples per side | Pruned candidates per side | Winning tuples, below / above | Largest score operand bits, below / above |
| --- | ---: | ---: | --- | --- |
| C: 1/1000, 1/999 | 35,937,000 | 71,875 | (1000,1,0,0) / (999,1,0,0) | 100 / 104 |
| C: 15/16, 16/17 | 35,937,000 | 106,721 | (16,15,0,0) / (17,16,0,0) | 92 / 94 |
| H axis slopes: 1/16, 1/15 | 35,972,936 | 107,810 | (1,-16,-1,0) / (1,-15,-1,0) | 92 / 90 |

The tuple starts with the recorded denominator; H scores use scalar zero.
Rust exhausted the full domain with checked u128 products. Python independently
reimplemented the existing denominator-pruning formula. A third computation
used Frobenius residuals and complete z-block symmetry to check both the full
winner and the observed operand-bit maximum. All six pruned searches retained
the global winner. Counts deduplicate pruned denominators per numerator triple;
they do not include or test floating finalist screening. The maximum **104 bits**
describes integer score operands, not the larger certificate rationals.

## Commands and remaining work

Production command (explicitly writes frozen files):

```text
python -B work/experiments/language-lx/lx9-exact-request/freeze.py
```

Final acceptance command, **run here, exit 0**:

```text
python work/experiments/language-lx/lx9-exact-request/check.py
```

The checker compiles `grid.rs` with:

```text
rustc --edition=2024 -C opt-level=2 -C debuginfo=0 -C panic=abort work/experiments/language-lx/lx9-exact-request/grid.rs -o work/experiments/language-lx/lx9-exact-request/build/grid.exe
```

It sets TMP/TEMP under `build/tmp`, runs the generated input file and removes
all build products. Failure controls exercised the unchanged checker: an
injected oracle disagreement and an altered stored sign with a matching digest
both exited nonzero and removed `build/`. No failure was relabelled as a pass.

Linux execution is **not run here**. The integrator must run this from the
repository root in a WSL checkout with Python 3.14 and Rust available on PATH:

```text
wsl.exe --exec python3 -B work/experiments/language-lx/lx9-exact-request/check.py
```

If Rust is unavailable there, the exact partial command is below. It checks
parts 1 to 3 with only the Python standard library, explicitly reports that
part 4 was not rerun, and does not substitute for full Linux acceptance:

```text
wsl.exe --exec python3 -B work/experiments/language-lx/lx9-exact-request/check.py --python-only
```

No implementation part remains unfinished. Integrator review and the owner's
choice remain open. Native input, world-frame gesture production, GPU/device
runs, timing, natural-input frequency and universal cross-platform agreement
are not established. Cannot show: equality for every request from a finite
sample, input latency, or an approved change to state contract A1. No product
code, chapter, contract, model, journal or personal session was changed.
