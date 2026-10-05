# Review packet: H-06 cost-table tools (H6-T), shard B

Review only; do not perform follow-up work.

Run from the `claude/renderer-sb` checkout:
`python tools/agents/codex_review.py --kind review --model gpt-6.1-sol --effort max --speed fast --packet work/experiments/renderer-sb-packets/review-4-h06-tools.md`

## 1. Goal and acceptance
- Goal: one Sol review (fast tier, non-critical tools) of the H-06 cost-table tools. Shard A (`review-4-h06-probe.md`) reviews the probe side of the same candidate at the same time.
  - The tools cover four things:
    - feature runs and the cost scene `w3f` in the renderer gate;
    - the gate's trace timing, offered to callers only;
    - the new public keys in `b412_summary.py`;
    - the new table tool `tools/perf/feature_costs.py`.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`. Report only `blocker` or `major` findings. Each finding needs a concrete counterexample: the inputs, then the wrong output.
  - Wrong outputs that count:
    - **A forbidden cost.** A cost number appears in a row that the packet's rules make `unmeasured` or `not-comparable`. The rules cover:
      - unknown or differing controls;
      - mixed builds, or a build that differs from the baseline's;
      - no cost frames;
      - a state mismatch.
    - **A wrong cost.** Any of these:
      - a window that is not c × n consecutive trace states;
      - frames from outside the gate interval or from the preroll;
      - W3 runs used as cost runs;
      - the wrong sign for `gaps`;
      - a statistic other than the stated mean and nearest-rank p99.
    - **A wrong status.** `measured` where the rules require `preliminary` or another status, or `preliminary` lost in trace timing.
    - **Wrong gate fields.**
      - Gate fields taken from the wrong group. The `gaps` row's `verdict` comes from the `none` group.
      - Gate fields taken from W3 runs of another build without the reason `gate-other-build`.
    - **A gate regression.** Any of these:
      - a `w3f` run counted as gate evidence;
      - runs of different features pooled;
      - trace timing reachable from the gate's command line;
      - a gate verdict or number that differs from the current tool's for PresentMon-timed inputs, apart from the added `feature` key.
    - **A leak.** An output file carries an absolute path, a user name, a control value outside `controls`, or raw diagnostics.
    - **A hollow test.** A test passes although the rule it names is broken.
  - Verdict `pass` if there is none.

## 2. Actual problem and reproduction
- H-06 is in packet `docs/progress/1.0/packets/renderer/E-2.4-01-sb-probe.md`, acceptance item 5. For each visual feature it needs:
  - the frame time the feature adds;
  - the W3 gate verdict with the feature on.
- The specification is packet `work/experiments/renderer-sb-packets/H6-T-cost-table.md`, section 1.
  - Its section 2 says why costs come from paired runs of the frame-locked scene `w3f`, not from W3.
  - It also lists the findings of the Astra plan check and its two scoped re-checks that shaped the design.
- Codex implement call `20261003T101044Z-bb85e66b` implemented the packet.
  - The run was valid, and its acceptance check passed. Its report is `work/reviews/20261003T101044Z-bb85e66b/report.md`.
  - Claude reviewed the patch, found no `blocker` or `major`, and applied it unchanged.

## 3. Environment and versions
- Windows 11 and CPython 3.14.7 64-bit, standard library only. The B4-12 fixture tests also use NumPy 2.3.5 from the engine environment.
- The review sandbox is read-only and CPU only. Evidence kind: source and synthetic fixtures.

## 4. Necessary source and evidence
- Files under review are uncommitted changes against `HEAD` (`db97d5d`). `git diff HEAD -- <file>` shows them; the two marked new are untracked.
  - `tools/perf/renderer_gate.py`;
  - `tools/perf/b412_summary.py`;
  - `tools/perf/feature_costs.py` (new);
  - `tests/test_renderer_gate.py` and `tests/test_b412_summary.py`;
  - `tests/test_feature_costs.py` (new).
- The probe facts the tools rely on are in the H6-T packet: section 1 ("The cost scene `w3f`") and section 4.
  - Shard A checks the probe against them.
  - In Claude's integration check on the owner's GPU, every one of the eight `w3f` traces followed the sequence with 0 mismatches (7,695 to 15,713 entries each).
- Integrator's checks on the final source:
  - `python -m unittest -q tests/test_renderer_gate.py tests/test_b412_summary.py tests/test_b412_fixture.py tests/test_renderer_tools.py tests/test_feature_costs.py`: 145 tests OK, 1 skipped (the NumPy fixture class);
  - the same B4-12 fixture tests run with the engine environment's Python and NumPy: 20 tests OK.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 to 6 | see H6-T packet, section 5 | packet versions 1 to 5 | Astra plan check, two scoped re-checks, CPU simulation | the final packet |
| 7 | the final packet is implementable as written | implement call `20261003T101044Z-bb85e66b` | acceptance check; Claude's patch review | 145 tests OK; no `blocker` or `major` |

## 6. Constraints and owned files
- Read-only review.
- Out of scope:
  - the probe, which is shard A;
  - `work/experiments/renderer-sb/RESULT.md`, the results folder and the wiki pages;
  - design choices that the H6-T packet records as decided: paired `w3f` cost runs, the control list and the status rules. They are in scope only if the code produces a wrong output from section 1;
  - timing numbers;
  - style, `minor` and `nit` findings.
- Questions:
  1. **Gate regression.**
     - For PresentMon-timed inputs without `feature` or `w3f`, can `summarize` give any output other than today's plus `"feature": "none"`?
     - Can `main` reach trace timing?
  2. **The window.**
     - For any frame rate, a lag of 0 to 2 and any run length, does each run's window hold exactly c × n consecutive nominal states, centred as specified?
     - Can a frame from outside the gate interval or from the preroll enter a window? Can a window span a broken `frame` series?
  3. **Nominal pairing.**
     - In PresentMon timing, the nominal entry is the last trace entry whose `qpc` is not later than the present tick. In trace timing, it is the entry that ends the step.
     - Is there an off-by-one that shifts the window by more than the stated lag? Can two presents pair with one entry?
  4. **Controls.**
     - Can a missing, null, wrong-type or sentinel value count as known? The sentinel includes `UMD version unavailable (0x…)` anywhere in `presenting_adapter`.
     - Can two unknown values compare equal?
     - Is the `msaa` rule applied as stated: 4 only for `msaa4`, and 1 for every other group, the baseline included?
  5. **Status and gate fields.**
     - Can a row be `measured` in trace timing, with fewer than three runs with cost frames in either group, or with any `not-comparable` condition?
     - Do gate fields come only from W3 groups whose runs all have the cost runs' build?
  6. **Sanitising.** Can any output carry a path, a user name, a control value outside `controls`, or raw diagnostics?

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
