# LX-9 follow-up: what the Linux result shows

Integrator note, 5 November 2026. Evidence class: synthetic. Decision: **open**; decider: the owner, after EN1's Astra
check. Nothing here changes the state contract, a format or any product code. `RESULT.md` was written before the Linux leg
and covers the Windows legs only; this note and `LINUX-COMPARISON.md` carry the cross-platform result.

## Result

In step 2, six near-bisector requests chose a different exact rotation on Linux than on all three Windows legs:
`b1205`, `b1207`, `b1209`, `b1211`, `b1213` and `b1215`. `AMENDMENT.md` quotes the first three per leg;
`linux-differences.tsv` holds all six. Under RTE.65 this stops an unqualified claim of cross-runtime agreement on fresh
requests and opens the amendment path. RTE.66 still separates this from replay, which reads the recorded exact rotation
and does not run the chooser.

All six share one angle, 71.56505117707799 degrees, with N = 1 and D = 16, on the axes +x, -x, +y, -y, +z and -z. This is
one float computation seen on six axes, so it counts as one event, not six independent ones. Windows chooses `(1, e)`, the
90 degree rotation. Linux chooses `(2, e)`, the rotation of about 53.13 degrees. The request sits on the exact bisector of
those two rotations to within one ulp, as the corpus intended.

## Control

`trig_control.py` compares every intermediate float on the 14,816 distinct requests (13,600 of step 1 and the 1,216
near-bisector requests of step 2) between Windows and Linux.

| Check | Requests |
| --- | --- |
| Unit axis bits differ | 0 |
| Half-angle bits differ | 0 |
| Sine bits differ | 437 |
| Cosine bits differ | 481 |
| Target bits differ | 864 |
| Target differs while sine and cosine are equal | 0 |
| Sine or cosine differs while the target is equal | 25 |
| Windows sine, cosine not correctly rounded | 432, 488 |
| Linux sine, cosine not correctly rounded | 19, 17 |

Every target-bit difference between the platforms comes with a difference in `math.sin` or `math.cos`. Everything before
those two calls is bit-identical. Windows CPython 3.15.1 and 3.14.7 produced byte-identical tables, so the difference
follows the platform's mathematics library and not the Python version. The 864 equals step 1's 852 plus 12 near-bisector
requests; 6 of those 12 chose a different rotation.

## The one input behind the six

At the half angle 0.6245228861991272 both platforms return the same sine. Windows returns the cosine
`3fe9f5b22bcede15`, Linux `3fe9f5b22bcede16`; the correctly rounded value is `3fe9f5b22bcede16`, computed at 110 decimal
digits.

The sign of `5(c + s)^2 - 2(2c + s)^2` decides between the two rotations: positive picks `(1, e)`, negative `(2, e)`.

| Evaluated at | Value | Picks |
| --- | --- | --- |
| Windows target, after normalization | +4.536e-16 | `(1, e)` |
| Linux target, after normalization | -1.160e-16 | `(2, e)` |
| Exact sine and cosine of the float half angle | +6.945e-18 | `(1, e)` |
| Exact sine and cosine of the real half angle | +3.683e-17 | `(1, e)` |

So the exact answer for this request is the one Windows gave, and the platform whose cosine was correctly rounded gave the
other one. The request's true distance from the tie is far smaller than the rounding noise of a double-precision target.

## What this does and does not show

Integrator's reading, for EN1's Astra check to test:

- Correctly rounded sine and cosine would not make the float path exact. Near a bisector the rounding of the target itself
  outweighs the distance to the tie.
- Bit-identical sine and cosine on every platform would remove this difference without making the choice exact, because
  every other operation in the path is correctly rounded.
- Only an exact comparison decides the side the mathematics gives.

Not shown: how often a real input lands this close to a bisector; the product default N = 16, D = 1000; world-frame input;
the 60-candidate menu-mode drag; native input. Windows comes first and macOS and Linux product builds follow 1.0, so a
Windows-only 1.0 does not meet this difference between platforms. The headless Linux checks would meet it if they ever
recompute a fresh choice instead of reading the recorded one.

## Reproduction

From the repository root, once under Windows and once under Linux, then compare:

```text
python -B work/experiments/language-lx/lx9-input-map/followup/trig_control.py <windows.tsv>
wsl.exe --exec python3 -B work/experiments/language-lx/lx9-input-map/followup/trig_control.py <linux.tsv>
python -B work/experiments/language-lx/lx9-input-map/followup/trig_control.py --compare <windows.tsv> <linux.tsv>
```

The tables are regenerated, not kept. Their SHA-256 digests: Windows
`d06d368a552e9e60e58f79c6f24065bc6312c421518051ad680fa375a0901e0b`, Linux
`ce34e7343763686d8829b5741157c6ad3634c16e77b68a0bab6a41bb03ceb4c7`.
