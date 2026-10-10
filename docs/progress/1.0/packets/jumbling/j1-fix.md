# Packet: fix the J1 review findings (implementation)

Run as an implementation with the default reviewer model, effort `max` and speed tier `fast`, in the worktree the wrapper creates. Change only the allowed files.

## 1. Goal and acceptance
- Goal: fix the six blocking findings of the J1 review (shards `20261009T210942Z-04ccee5c` and `20261009T210943Z-d2ec15dd`, candidate `38652f0`). Five fixes are needed, because J1A-01 and J1B-001 are the same defect.
- **F1 — J1A-01 and J1B-001: unchecked K⁺ index.**
  - `Twist(ctx, grip, matrix, kidx=k)` must refuse an index whose K⁺ matrix differs exactly from `matrix`. Without an index, the index is derived from the matrix as today.
  - Counterexamples:
    - `Twist(ctx, 0, ctx.kplus.matrix(0), kidx=int(ctx.kplus.gen_idx[26]))` applied to `State(ctx)` was admissible and corrupted the projection;
    - `Twist(ctx, 0, ctx.kplus.matrix(0), kidx=3)` in an A4 menu state moved centre 0.
  - Both must now raise `TwistError` with the state unchanged. A matching index keeps working.
- **F2 — J1A-02 and J1B-004: overflow in display floats.**
  - Converting exact values to floats for display (certificate `h_*_float`, `angle_deg` in records, reports) must never raise.
  - Use an overflow-safe conversion for every display float in the simulator: for example exact rational conversion of a/d and b/d with `fractions.Fraction`, keeping `kernel.q5_float`'s cancellation-free form. Return `None` only where the value itself is not finite. No exact decision may depend on these floats.
  - Counterexamples that must now pass:
    - Apply `sim.plane(ctx, 0, 13, degrees=10)` 30 times. Then `classify(1)` and `survey([1])` complete with the filter off and on, and a straddle found at that state carries a checked certificate.
    - `sim.cayley(ctx, 0, [Q5(10**160 + 1, 0, 10**160), 0, 0])` applies with journaling on, serialises, replays and undoes exactly.
- **F3 — J1B-002: tolerance ties in the input map.** Amendment A1 asks for the exact global minimum of the Frobenius distance, with ties broken by the smaller denominator and then by the lexicographically smaller numerators.
  - The float search may keep proposing finalists. The final choice among all candidates whose float distance is within a safe margin of the best must be made by exact comparison.
  - Treat the float target quaternion components as exact rationals (`Fraction(float)`) and compare the squared quaternion dot products exactly: (d·t₀ + p·t)² / ((d² + |p|²) · |t|²), with the half-turn branch d = 0.
  - Only exactly equal distances are ties. Choose the margin so that no true minimum can fall outside it (state the bound in the README), or widen it until the exact winner is strictly inside.
  - Counterexample that must now pass: `nearest_parameter([1, 0, 0], 45.00000000002, max_den=1, max_num=1)` returns numerators (1, 0, 0) with denominator 1. Add requests just either side of 45° and an exactly tied case.
- **F4 — J1B-003: corrupted JSON numbers coerced.** `q5_from_json` (and every reader of exact records: matrices, Cayley ω, plane s, half-turn axis, menu records) must refuse a component that is not an `int`, `bool` included. Counterexample: change `records[0].matrix[0][0]` of a generator journal from `[1, 1, 4]` to `[1, 1, 4.25]`. Replay must refuse it.
- **Acceptance.**
  - The acceptance check below passes inside the sandbox.
  - New tests in `tests/test_jumbling_sim.py` cover F1–F4 with the counterexamples above. The whole file stays under about 60 s.
  - `python research/jumbling/sim/accept.py` still passes all 31 flags. Add flags `review_fixes_F1_to_F4` for the counterexamples. If `accept.py` does not finish in the sandbox, the integrator runs it.
- Out of scope:
  - everything outside the findings;
  - `exact.py` (read only; wrap it instead);
  - the viewer, the explorer and `witness.py`;
  - performance;
  - `minor` and `nit` issues.

## 2. Actual problem and reproduction
- The counterexamples in section 1 reproduce each finding at `38652f0`.

## 3. Environment and versions
- Branch `claude/jumbling-1-0`, committed head. Linux, Python 3.11 or later, NumPy.

## 4. Necessary source and evidence
- `research/jumbling/sim/`:
  - `twists.py`: `Twist.__init__`, `angle_deg`, `record`, `from_record`, `nearest_parameter`, `_frobenius`, `INPUT_TIE`;
  - `state.py`: `_straddle_certificate` and the report fields;
  - `kernel.py`: `q5_float`;
  - `kplus.py`: `q5_from_json`, `matrix_from_json`;
  - `README.md`.
- `research/jumbling/state-contract.md`, amendment A1.
- The review results in `work/reviews/20261009T210942Z-04ccee5c/review.json` and `work/reviews/20261009T210943Z-d2ec15dd/review.json`, if available; their counterexamples are repeated above.

## 5. Attempts so far
| # | Step | Result |
|---|---|---|
| 1 | Extension `38652f0` | 31 flags and 32 tests pass |
| 2 | Senior review, two shards | six major findings, five distinct defects; all adopted |

## 6. Constraints and owned files
- Change only the allowed files below. Do not commit; the wrapper collects the patch.

```implement-contract
{"allowed_files": ["research/jumbling/sim/__init__.py", "research/jumbling/sim/state.py", "research/jumbling/sim/twists.py", "research/jumbling/sim/kernel.py", "research/jumbling/sim/kplus.py", "research/jumbling/sim/accept.py", "research/jumbling/sim/acceptance.json", "research/jumbling/sim/README.md", "tests/test_jumbling_sim.py"], "acceptance_check": ["python", "tests/test_jumbling_sim.py"], "stop_condition": "F1-F4 are fixed with tests for every counterexample, and the acceptance check passes"}
```

## 7. Required return format
- Changes only in the assigned worktree. The final message lists:
  - the changed files;
  - per fix F1–F4, what changed and which test covers each counterexample;
  - the margin argument for F3;
  - the acceptance result, and whether `accept.py` ran;
  - open points.
