# Local jobs for the owner's laptop (WSL, Ubuntu)

Written by the integrator session of draft PR #112. Revision 4, 2026-10-20 08:50 UTC (adds Phase B on `a9df9c0`). Re-read this file from
`origin/claude/local-handoff` every 30 minutes; a newer revision replaces this one, and a job it marks `cancelled`
is not started (a running one finishes and is recorded).

Research only, headless Linux, exact geometry. Nothing here is Windows, DirectX, input or performance evidence; CPU
times are laptop timings of research code.

## Setup (once)

- Clone: `git clone https://github.com/KonomiYuzu01/Magic-600-Cell.git ~/m600-local`, then in it
  `git checkout -b claude/local-runs-20261020 origin/claude/repair-pair-list` (expect `5fad028`).
- Drafts: copy the two files under `drafts/` of this branch into
  `research/jumbling/restoration/12-low-height/scripts/round22/drafts/` of the clone
  (`git fetch origin claude/local-handoff && git show origin/claude/local-handoff:drafts/<name> > <dest>`), and copy
  `r22d75_census_k3000.py` from
  `origin/claude/census-k3000-s7305:research/jumbling/restoration/12-low-height/scripts/round22/cloud/census-k3000-s7305/r22d75_census_k3000.py`
  into the same `drafts/` directory. Record the sha256 of each copy; expected:
  `r22d81_rule.py` `308145bb5d61bb56f5f09846fb4688b901f06f115e27569d4a897d85b1ff22ac`, `r22d81_pairs.py` `204068c5d60d323aecbb79ef99f65a2b9f61fb4cede1d652d60d421457094872`, `r22d75_census_k3000.py`
  `085d6cf497f7e792d9f62ec1fe04bfb3be58d8a1cfc1620c7fba05b8e0de0c0e`.
- Python: a virtual environment `~/m600-venv` with Python 3.13 if available, else 3.12, and `numpy==2.5.3` (if no
  wheel exists for the interpreter, the newest NumPy 2.x; record the versions). Nothing else is needed.
- Every run, from `~/m600-local`:
  `OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 JUMBLING_HOME_CACHE=$HOME/m600-cache/<job>.npy
  nice -n 10 $HOME/m600-venv/bin/python -I -B -u <script> <args> > $HOME/m600-runs/<job>/out.txt 2>&1`,
  started detached (tmux or `setsid nohup`), with `<job>.start`, `<job>.done` (exit code and UTC time) next to
  `out.txt`. Snapshots, census records and caches stay under `~/m600-runs` and `~/m600-cache`, never in the clone.
- Parallel jobs: at most min(6, logical cores - 2). Each job needs about 0.5 to 1 GB of memory.
- Smoke check: job A1 is the smoke check; its two digests below must match before any other job starts.

## Phase A (start now, on `5fad028` plus the drafts)

| Job | Command (script path under `research/jumbling/restoration/`) | Expect | CPU here |
| --- | --- | --- | --- |
| A1 snapshots | `12-low-height/scripts/round22/r22d31_snapshots.py ~/m600-runs/snap` | writes `s7103-before74.pkl` (`48ef2f68…`, 94,981 off) and `s7302-stop.pkl` (`015d839ecbb37bbdde74ae9391d5d7f274dea7fee4b4fdf0092d224eef907a1f`, 100,584 off) | about 2,200 s |
| A2-s7308 … A2-s7311 | census pipeline below, seeds 7308, 7309, 7310, 7311 | see below | up to about 11 h each |
| A3 no-meet-prune | `12-low-height/scripts/round22/drafts/r22d81_pairs.py --snapshot ~/m600-runs/snap/s7302-stop.pkl --prefix '[[64,15],[185,14],[357,12],[471,12],[263,14],[407,13],[523,12],[185,1],[263,1],[263,3]]' --expect fe273a8c88d8a8fe7f030f95e6130fc76a57da6d708dea110a1cb31125276122 --target 311 --no-meet-prune --cpu 14400` (after A1) | the same STEP letters as `ref/u2-iii.out` of this branch ((364,3),(263,6) U2, then (263,40) U, UNLOCKED); a different first pair refutes the meet prune | pruned run: 586 s; unpruned: unknown, budget 14,400 s |
| A4 schl | SCH-L acceptance in a separate worktree, see below (independent of A1) | the test prints `OK`: every test passes, including `setUpClass` | here it timed out after 2,703 s under heavy load (deadline 45 min); unknown unloaded |
| A5 no-meet-prune at cap 290 | `12-low-height/scripts/round22/drafts/r22d81_pairs.py --snapshot ~/m600-runs/snap/s7302-stop.pkl --prefix '[[64,15],[185,14],[357,12],[471,12],[263,14],[407,13],[523,12],[136,2],[136,6],[136,1],[301,1],[136,1],[301,3],[136,1],[136,3],[136,11]]' --expect 8ac0a1ce8f3f5af20b376421431e6d1510e5e0ce3408a0fe7a4d40bdedf18116 --target 290 --no-meet-prune --cpu 14400` (after A1) | STUCK with u = 4, as the pruned run `ref/u2-ii.out` of this branch ends; any STEP line refutes the meet prune | pruned: about 1,100 s for this last search; unpruned: unknown, budget 14,400 s |

Census pipeline for seed `<s>` (the one the cloud lines census-k3000-s7305 and s7307 use; see their `RESULT.md`):

1. `11-scramble-census/scripts/census_seams.py 3000 <s> 18900 ~/m600-runs/census-<s>` (wall-clock budget
   18,900 s) writes `ia-k3000-s<s>.json`. If the process is killed, continue with the script's own option:
   `census_seams.py 3000 <s> <18900 - last step time> ~/m600-runs/census-<s>-part2 --start <part 1 json>` and record
   both parts.
2. `12-low-height/scripts/round22/drafts/r22d75_census_k3000.py ~/m600-runs/census-<s>/ia-k3000-s<s>.json`
   (R22D-75's height-first rule from the census stop; up to 14,400 CPU s).
3. `12-low-height/scripts/round22/drafts/r22d81_rule.py --record ~/m600-runs/census-<s>/ia-k3000-s<s>.json
   --prefix '<R22D-75's letters as a JSON list of [grip, item]>' --cpu 7200` (R22D-81 from where R22D-75 stops;
   the record is replayed first, about 1,400 CPU s). If R22D-75 played no letter, use `--prefix '[]'`.
4. Record for the seed: scramble off, N and left heights; the census stop's digest, off, N and heights; R22D-75's
   letters and end line; R22D-81's STEP lines and its HANDOFF, STUCK, BUDGET or NOT_APPLICABLE line.

Job A4 (the Codex SCH-L candidate, whose acceptance test timed out on the loaded cloud machine):

1. `git -C ~/m600-local worktree add --detach ~/m600-schl 5fad028`; copy `schl/restore_schedule11_late.py` and
   `schl/test_sched11_late.py` of this branch into
   `~/m600-schl/research/jumbling/restoration/09-general-method/scripts/schedule11/` and record their sha256
   (Git blob ids `93445a62c432d5816e82f98c82bf683ef8dc195a` and `466ad7d9bedc7728e741595e69673f62934006d8`).
2. From `~/m600-schl`, with the run environment above and `JUMBLING_HOME_CACHE=$HOME/m600-cache/A4.npy`:
   `$HOME/m600-venv/bin/python -I -B -u research/jumbling/restoration/09-general-method/scripts/schedule11/test_sched11_late.py`.
   The test checks that the worktree gains no files, so write `out.txt`, `.start` and `.done` under
   `~/m600-runs/A4-schl/`, not in `~/m600-schl`. It has its own 45-minute deadline and a 7 GB address-space limit.
3. Record under `local/A4-schl/` of the results branch (in `~/m600-local`, not in `~/m600-schl`): `out.txt`,
   `RESULT.md` with the per-test outcomes, the wall time and the retained lengths and ratios the test prints. Do not
   copy the two schl files into the results branch. Then `git -C ~/m600-local worktree remove ~/m600-schl`.
4. If `setUpClass` times out again on an idle machine, record that; do not raise the deadline or edit the test.

## Phase B (start now, on `a9df9c0`; Phase A jobs keep running unchanged)

`a9df9c0` on `claude/repair-pair-list` commits the round 6 scripts, including `r22d81_rule.py` and `r22d81_pairs.py`
under `scripts/round22/` (the same code as the drafts of Phase A; only comments differ), D19 (`r22d19_rule.py`) and
the five implement patches of 31b78a5. Do not restart or change a Phase A job.

Setup (once): `git -C ~/m600-local fetch origin claude/repair-pair-list`, then two detached worktrees at that commit,
`git -C ~/m600-local worktree add --detach ~/m600-b a9df9c0` for the tests and
`git -C ~/m600-local worktree add --detach ~/m600-c a9df9c0` for the rule runs. Check `git -C ~/m600-b rev-parse HEAD`
starts with `a9df9c0`. Use the run environment of the setup above (one thread, `nice -n 10`, a cache file per job
under `~/m600-cache`, `out.txt`, `.start` and `.done` under `~/m600-runs/<job>/`), never writing into `~/m600-b` or
`~/m600-c`: the tests check that their worktree gains no file.

Rule runs (from `~/m600-c`, after A1; each writes no file; `<snap>` is `~/m600-runs/snap/s7302-stop.pkl`). R22D-81's
rule continued from the end states of the routes of R22D-89; nobody has run these, so there is no expected end line.
Script: `12-low-height/scripts/round22/r22d81_rule.py --snapshot <snap> --prefix '<letters>' --expect <digest> --cpu 7200`.

| Job | Letters (prefix) | Expect after the prefix |
| --- | --- | --- |
| C1 cap 290 route | `[[64,15],[185,14],[357,12],[471,12],[263,14],[407,13],[523,12],[136,2],[136,6],[301,2],[136,7],[301,4],[136,1],[136,4],[136,19],[301,3],[136,1],[301,3],[136,5],[290,15]]` | `21db4f944c71f020f2380e434dc1b37618c9b1437960ef87365f3345aaa34928` |
| C2 cap 311 from the stop | `[[185,1],[185,7],[185,16],[185,3],[263,1],[263,3],[263,8],[364,1],[263,2],[407,1],[407,3],[407,1],[407,2],[407,4],[364,1],[407,2],[407,4],[407,10],[407,15],[364,2],[263,2],[311,14]]` | `2f88c1e617cc22c7f09eef19d6e7c530fdcb7a49c577305f1ca79196533726b1` |
| C3 cap 311 after run (i) | `[[64,15],[185,14],[357,12],[471,12],[263,14],[407,13],[523,12],[185,1],[263,1],[263,3],[364,3],[263,6],[263,40],[311,14]]` | `5296c847f5a19cb1bc6e4cc531786c833dc681adbdc367763378f9325e740138` (START: tau 274, u 3, as in `runs/round22/r22d81_rule_311_14.txt`) |

A wrong digest after the prefix stops the run with an error: record it and do not retry with another prefix. Record
the START line (target table, tau, u), every STEP line and the end line (HANDOFF, STUCK, BUDGET or NOT_APPLICABLE).

Tests (from `~/m600-b`, one at a time: two suites at once in one worktree can see each other's temporary files). Each
is a second-machine run of an acceptance test that passed here, with its slow switch on where it has one (the
acceptance runs here skipped the slow part). Command: `$HOME/m600-venv/bin/python -I -B -u <test>` with the switch in the
environment. Paths under `research/jumbling/restoration/`.

| Job | Test | Switch | Here (headless Linux, 20 October) |
| --- | --- | --- | --- |
| B1 r22d25 | `12-low-height/scripts/round22/test_r22d25.py` | `R22D25_SLOW=1` (rebuilds the seed 7302 stop) | OK, 19 tests, 2 skipped, 561 s without the switch |
| B2 llp | `13-tutorial-runner/scripts/test_literal_lattice.py` | `LATTICE_SLOW=1` (the committed depth-20 handoff) | OK, 18 tests, 1 skipped, 468 s without the switch |
| B3 run4 | `13-tutorial-runner/scripts/test_check_run.py` | none | OK, 66 tests, 482 s |
| B4 schl | `09-general-method/scripts/schedule11/test_sched11_late.py` | none | OK, 8 tests, 931 s (after the integrator's fixes; A4 runs the earlier candidate) |
| B5 run3 | `13-tutorial-runner/scripts/test_runner.py` | `RUNNER_SLOW=1` (depth 20 through the lattice phase) | OK, 34 tests, 1 skipped, 1,222 s without the switch |
| B6 d19 | `12-low-height/scripts/round22/test_r22d19_rule.py` | `R22D19_SLOW=1` (the full enumeration, about two hours) | OK, 10 tests, 1 skipped, 1,925 s without the switch |

Record per test job: the test's last lines (`Ran ... tests`, `OK` or the failures with their tracebacks), every
`skipped` line with its reason, the wall time and any JSON line the test prints (worktree checks, timings). A failure
is a result: record it and go on; do not edit a test, raise a deadline or re-run a failed suite more than once.

Order: C1 to C3 first (after A1), then B1 to B6; at most min(6, logical cores - 2) jobs at once with the Phase A jobs
counted. When Phase B is done, `git -C ~/m600-local worktree remove ~/m600-b` and the same for `~/m600-c`.

## Results

- Branch `claude/local-runs-20261020` only. Per job, under
  `research/jumbling/restoration/12-low-height/runs/round22/local/<job>/`: `out.txt` (paths removed), the census
  records (`ia-k3000-s<s>.json`, and part 2 if any), and a short `RESULT.md` (command, script sha256, Python and
  NumPy versions, CPU and wall times, end lines). Copy the three draft scripts once into
  `research/jumbling/restoration/12-low-height/scripts/round22/local/` with their sha256 in `local/README.md`.
- Before each commit, scan the staged files for `/home/`, `/mnt/`, `/tmp/`, `C:\`, `Users`, the WSL user name and
  e-mail addresses, and remove what you find. No `.pkl`, `.npy` or cache files.
- Commit after each finished job and push: `git push -u origin claude/local-runs-20261020`. If the push fails for
  credentials, keep committing locally and write the reason in `~/m600-local-status.txt`. Never push another
  branch, never open a pull request, never merge.
- At the end (all jobs done, or quota low, or a job you cannot run): a `local/STATUS.md` with what ran, what did not
  and why; commit, push, stop.
