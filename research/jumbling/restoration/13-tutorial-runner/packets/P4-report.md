Changed only:

- [check_contracts.py](research/jumbling/restoration/13-tutorial-runner/scripts/check_contracts.py): replaced the stub with independent height and kernel contracts, exact straddler counts and the high-phase letter bound.
- [test_check_contracts.py](research/jumbling/restoration/13-tutorial-runner/scripts/test_check_contracts.py): added fixture, oracle, mutation, independence and worktree-preservation checks.

The specified acceptance command passed: **exit 0, 125.366 seconds**. Fixture setup took 41.065s. Tests 1–11 took, respectively:

`67.126, 0.161, 0.948, 1.406, 2.732, 2.635, 0.329, 1.587, 1.488, 2.307, 2.050` seconds.

Test 12 reported `changed paths: []` and **“Worktree unchanged, including ignored files.”** Optional `IndependenceTests` also passed both tests.

Open points and recorded choices:

- I1 must wire `check_high_phase`: `check_run.py` currently makes no call, so supplies no start. All existing hook signatures match; all recorded fixture values reproduced.
- Z36 clean adds: `(2,12)`, `(16,12)`; single/pair taus `1/1`. A: `(2,12)`, `(6,12)`; taus `1/0`. Retained item: `1`.
- Other selected taus: `0` for H6, L4, plateau, pose and guard tests; `35` for the real-rule L4 case. Mutation caps were `0` and `2`.
- Test 6 printed its permitted skip note: no rising-u tau for `h4/L2`.

Evidence is headless source/fixture only. No commits made.