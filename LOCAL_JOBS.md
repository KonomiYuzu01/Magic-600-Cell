# Local jobs for the owner's laptop (WSL, Ubuntu)

Written by the integrator session of draft PR #112. Revision 3, 2026-10-20 05:05 UTC (adds job A5). Re-read this file from
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

## Phase B (after the integrator's next push; this file will name the commit)

Not yet available: second-machine reruns of the five implement acceptance tests and of the slow tests that their
acceptance skipped, and R22D-81 continuations from the end states of the cap 311 and cap 290 routes.

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
