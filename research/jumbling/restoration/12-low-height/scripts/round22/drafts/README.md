# Draft scripts for the round 6 cloud lines (not reviewed for the repository; the integrator replaces this directory)

These files are the integrator's drafts and the senior reviewer's commands from line (d) round 6, shard U
(R22D-80 to R22D-86), handed to the cloud lines before they are committed with headers on
`claude/repair-pair-list`. Do not edit them in place: copy one into your line's own directory if you need to change
it, and say what you changed. Headless Linux research code; nothing here is Windows, DirectX or performance evidence.

| File | What it is | Status |
| --- | --- | --- |
| `r22d81_rule.py` | The integrator's implementation of the unlock rule R22D-81 (families H6, U, H4, H2; target cap τ; potential Φ = n6·B³ + u·B² + n4·B + n2, B = 177,121) for any snapshot or census record, with an optional prefix of letters | Re-run by the integrator at the 3000-twist stop of seed 7302 after R22D-75's seven letters: it chooses exactly R22D-80's five letters with every digest, the 59-item u vector of cap 124 and the target table; handoff at `bbef1c8e…`. The `--record`/`--save` options were added after that run |
| `r22d80_check.py` | The senior reviewer's replay of R22D-80's five letters (arguments: the `s7302-stop.pkl` snapshot) | Re-run by the integrator; output identical |
| `r22d82_unlock_graph.py` | The senior reviewer's unlock-graph computation (complete in/out/straddle tables, closure layers, unit rows of the blockers of caps 274, 104, 290 and 311, holder status) | Re-run by the integrator; output identical |
| `r22d85_two_letters.py` | The integrator's replay of R22D-85: (124, 2), (274, 13) from both 015d839e and 1ff0061f | Written, not yet run |
| `r22d86_cap290.py` | The integrator's replay of R22D-86: (136, 2), (136, 6) towards cap 290, the holders of the 38 remaining blockers, and every item of every clean holder | Written, not yet run |

Snapshots: `python3 -I -B research/jumbling/restoration/12-low-height/scripts/round22/r22d31_snapshots.py <dir>` writes
`<dir>/s7302-stop.pkl` (digest `015d839e…`, 100,584 off) in about 30 CPU minutes. Every script runs from the
repository root with `OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 JUMBLING_HOME_CACHE=<a file outside
the repository>`.

## Shard U facts (verified by the integrator's re-runs)

- 015d839e (the stop) → R22D-75's seven letters (64, 15), (185, 14), (357, 12), (471, 12), (263, 14), (407, 13),
  (523, 12) → `1ff0061fc5863c8ccbc1d734b003ff49afa5b58969b0031e51fd1e5cf1a07d81`, (n2, n4, n6) = (103,103, 1,019, 172),
  104,294 off, N = 132,702.
- R22D-81's rule from there: (124, 2) U, (274, 13) H6, (46, 1) U, (46, 3) U, (104, 14) H6 → digests `9a227291…`,
  `c9cefc31…`, `7465613d…`, `198e936f…`, `bbef1c8ec0de78dead1473322a8c1236e48f9ccfe0017ce724461bb7f37ae299`; at the end
  (n2, n4, n6) = (102,960, 936, 0), 103,896 off, N = 132,930; all 172 height-6 pieces end at height 4; 1,052 pieces
  that were on the lattice at 1ff0061f are off. Not protected.
- Target table at 1ff0061f as (cap, k): (104, 13), (150, 84), (274, 159), (290, 13), (311, 13); τ = 274 with 3
  straddlers 44706, 44712, 57467 (unit 30989, branch 4), held whole by caps 114, 118, 124, 150, 160, 235, 258, 299.
- Unlock closure L (caps whose straddlers can all be held whole by caps already in L, iterated from the clean caps):
  all 600 caps at both states; layers [38, 107, 130, 86, 93, 84, 15, 47] at 1ff0061f and
  [39, 118, 125, 80, 92, 84, 15, 47] at 015d839e. Depth-one covers: 274 ← 124; 104 ← 46 (30 blockers);
  290 ← {136, 301} (70 blockers: 20 in 136 only, 38 in 301 only, 12 in both); 311 ← {185, 263, 364} at 1ff0061f and
  {185, 263, 364, 407} at 015d839e. Membership in L is necessary for a first admissible use of a cap, not sufficient.
- Clean holders among all listed holders: [46, 124, 136, 185, 263, 301, 364, 407] (407 only at 015d839e). Cap 124 is
  clean but not free: at 1ff0061f it holds 1,200 lattice pieces, 1,176 own-branch and 395 foreign height-2 pieces.
- Cap 311: 107 blockers at 015d839e, 182 at 1ff0061f (units 1545: 19, 19065: 7, 38265: 99, 75665: 26, 81179: 23,
  96213: 8, all branch 2); every one is held whole by at least one clean cap; no unlocking run was made.
- R22D-85: (124, 2), (274, 13) also works from 015d839e: → (98,713, 992, 26), 99,731 off, N = 130,250,
  `bd44f887fa60da8b2d22ef69a8ea65a24a3c081a88c31df8c173992ed683bee4`, 346 initially lattice pieces off.
- R22D-86: from 1ff0061f towards τ = 290 (u = 70): (136, 2) gives u = 48 at `808aeac4…`, (136, 6) gives u = 38 at
  `9e6b991bf057d3f2c28c3158aed3878b55720c970601019eb819e53bd0601e94` (counts and off unchanged, N = 132,760). Then the
  single-letter U family is exhausted: the only caps holding a remaining blocker whole are
  [250, 260, 276, 283, 301, 302, 307, 348, 355, 356, 362, 375, 392, 405, 453, 466, 472], of which only 301 is clean,
  and none of its 59 items lowers u. Two- and three-letter preparations that keep or raise u at the intermediate
  state were not tried.
