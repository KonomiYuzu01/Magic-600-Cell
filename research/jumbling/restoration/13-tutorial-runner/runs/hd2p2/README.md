# HD2P/2 campaign: height 4 to height 2 at the six D2 ends (23 to 24 October 2026)

Outcome of the rule HD2P/2 (commit `01e0ae2`, scripts in [`../../scripts/`](../../scripts/)) at the six end states of
the D2 height phases. The rule reads only the current state and plays literal words of at most two letters: SINGLE
(a letter that keeps the maximum height and lowers Q = (P + 1)·n6 + n4), then for each opener its CL2 targets and its
PEEL letter. Every accepted step lowers M = 601·Q + Bk. The campaign started at 21:00 UTC on 23 October; a container
restart killed it at about 21:10, and it ran again from 02:08 UTC on 24 October in two search lanes and one checker
lane. Exact computation on committed tables, headless Linux, one thread per process; no Windows, DirectX or
performance evidence.

## Result

Every case ends **STUCK**: no letter and no two-letter word of the rule's families lowers M at the end state. For five
cases the checker's replay of every accepted letter from the pinned input and its independent audit of the exhaustion
(with item classes it derives itself) are complete and passed; the 7305 audit is still running.

| Case | Input table | Input (n4, n6) | Bk | Steps (SINGLE, CL2, PEEL) | Letters | End (n4, n6) | Bk | Input lattice pieces off at the end | Search CPU s | Audit |
|---|---|---|---|---|---|---|---|---|---|---|
| 7302 | `d2-s7302/component-checkpoint/tables/00021.json` (`2430f4cc`) | (734, 0) | 563 | 10 (0, 3, 7) | 13 | (491, 0) | 557 | 3,086 | 3,352 | passed (10,313 CPU s) |
| 7303 | `d2-s7303/run2/d2-s7303/tables/00138.json` (`d01dc91e`) | (929, 87) | 574 | 24 (1, 7, 16) | 31 | (710, 0) | 562 | 5,265 | 3,520 | passed (5,832 CPU s) |
| 7304 | `d2-s7304/component/tables/00005.json` (`ac127dc3`) | (797, 0) | 559 | 8 (0, 0, 8) | 8 | (797, 0) | 551 | 1,472 | 5,058 | passed (11,209 CPU s) |
| 7305 | `d2-s7305/component/tables/00010.json` (`cc5ef7ec`) | (1,419, 0) | 578 | 49 (4, 18, 27) | 67 | (906, 0) | 558 | 6,213 | 10,977 | running |
| 7306 | `d2-s7306/component-checkpoint/tables/00017.json` (`ac4be82c`) | (1,955, 0) | 574 | 19 (0, 7, 12) | 26 | (908, 0) | 562 | 4,024 | 4,962 | passed (8,899 CPU s) |
| 7307 | `d2-s7307/component/tables/00005.json` (`895b45de`) | (1,585, 0) | 562 | 21 (0, 3, 18) | 24 | (1,332, 0) | 542 | 4,803 | 7,210 | passed (11,803 CPU s) |

Tables are under [`../dev/`](../dev/); digests are the state digests (first eight hex digits). Bk is the number of
blocked caps. CPU figures are single-thread research timings on a shared machine.

## Reading

- The rule lowers the number of height-4 pieces at five of the six states (by 33% at 7302, 36% at 7305 and 54% at
  7306) and removes all 87 height-6 pieces at 7303, but it stops everywhere with hundreds of height-4 pieces left. At
  7304 only PEEL steps were found: eight letters lower Bk from 559 to 551 and n4 never moves.
- Every case takes input lattice pieces off the lattice (1,472 to 6,213), as the earlier height rules do; the rule
  does not protect the lattice.
- STUCK here means the stated families of words up to two letters are exhausted at the end state. It is not a proof
  that the state cannot be lowered. Three letters are enough at least once: at the 7304 input a word of three
  letters, each keeping the depth from rising, lowers n4 by 48
  ([stage 14](../../../14-blocking-structure/README.md)).
- After this campaign the owner changed the method: a constructive theorem programme replaces the search for rule
  catalogues, starting with the blocking structure of deep states
  ([decision of 24 October 2026](../../../../../../docs/wiki/decisions/owner-decisions-2026-10-24-theorem-program.md);
  [stage 14](../../../14-blocking-structure/README.md)).

## Records

- [`campaign.json`](campaign.json): per case the input, the chain of search segments and of audit segments with the
  SHA-256 of every segment manifest, the status and verification of each, and every accepted step (family, literal
  word, (n2, n4, n6), Q, Bk, N, off count, input lattice pieces off, digest, CPU).
- The segment directories themselves (units, tables, checkpoints, about 10 GB) stay outside the repository.

## Reproduction

From the repository root at `01e0ae2`, with `OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1`:

- search, one segment at a time (each segment resumes the previous manifest):
  `python3 -I -B research/jumbling/restoration/13-tutorial-runner/scripts/height_descent_experiment.py --segment
  --case <case> --out <dir> [--resume <dir>/<previous segment>/manifest.json] --cpu-seconds 3300 --max-steps 200
  --wall-seconds 3600 --work-cpu-seconds 3000`;
- replay and audit of a STUCK chain:
  `python3 -I -B research/jumbling/restoration/13-tutorial-runner/scripts/check_height_descent_experiment.py --chain
  <dir>/<last search segment>/manifest.json --out <dir> --audit [--resume-check <dir>/<previous audit
  segment>/manifest.json] --cpu-seconds 3300 --wall-seconds 3600 --work-cpu-seconds 3000`.
