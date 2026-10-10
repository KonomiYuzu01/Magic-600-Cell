# Packet J-WJ: the W-J scene in the gate judge

Run from the `claude/renderer-l2-followup` checkout, with no gate capture active:
`python tools/agents/codex_review.py --kind implement --model gpt-6.1-sol --effort max --timeout 3600 --packet work/experiments/renderer-wj-packets/judge-wj.md`

Source: renderer packet [E-2.4-0J](../../../docs/progress/1.0/packets/renderer/E-2.4-0J-jumbling.md), acceptance item 5 and section 4 ("Gate and judge"): three cold W-J runs per fixture are judged by `tools/perf/renderer_gate.py`, with the pose revision check passing in every run. The judge has no `wj` scene today, so it refuses every W-J run as unreadable. E-2.4-0J owns only `work/experiments/renderer-wj/*`, so the judge change is this separate packet. The two packets run in parallel and share the run format of section 6 below.

## 1. Goal and acceptance
- Goal: `renderer_gate.py` judges W-J runs as gate runs with the unchanged selection gate, and accepts a W-J run only when its exact label check and its pose revision check pass.
- Acceptance check: `python -B tests/test_renderer_gate.py` exits 0.
- Done when:
  - section 6 is implemented in `tools/perf/renderer_gate.py`;
  - `tests/test_renderer_gate.py` keeps every existing test unchanged and passing, and adds the tests listed in section 6;
  - the module docstring and `METHOD` describe the `wj` scene.
- Non-goals: any threshold, interval, warm-up, run count, VRAM or condition rule (all unchanged); the probe (E-2.4-0J); `b412_summary.py`, `feature_costs.py` and every other file.

## 2. Actual problem and reproduction
- `validate_run` requires `run['scene'] in SCENES` with `SCENES = ('w1', 'w2', 'w3', 'w3f', 'w4', 'w5')`, so a run with `"scene": "wj"` is unreadable.
- The judge has no record for the pose revision check of E-2.4-0J acceptance item 3, and no way to judge the three W-J fixtures separately.

## 3. Environment and versions
- Base: this worktree's HEAD. Windows 11, CPython 3.14, standard library only.
- The acceptance check runs in the Codex Windows sandbox. The existing tests already build their fixture folders in a way that works there; keep that style (`tempfile.TemporaryDirectory()` and `mkdtemp()` fail there with WinError 5; use `Path(tempfile.gettempdir()) / <unique name>` with a plain `mkdir()` for anything new).

## 4. Necessary source and evidence
- `tools/perf/renderer_gate.py` (all of it) and `tests/test_renderer_gate.py` (its fixture helpers and the W3 gate tests).
- `docs/progress/1.0/packets/renderer/README.md`, "Fixed inputs every packet repeats": the gate, which this packet does not change.
- E-2.4-0J section 4: the W-J trace (the swept twist and its inverse alternate with the W3 timing; pose revision t is the number of completed turns) and acceptance items 3 and 5.
- `tools/perf/check_renderer_packets.py` reads `FPS_MIN`, `P99_MAX_MS`, `INTERVAL_S`, `WARMUP_S` and `RUNS_MIN` from the judge; they stay as they are.

## 5. Attempts so far
None. This is the first call.

## 6. Constraints and owned files
Owned: `tools/perf/renderer_gate.py`, `tests/test_renderer_gate.py`. Change nothing else.

**Run format additions** (`magic600-renderer-run-v1` stays the format name; W3, W5 and every other scene behave exactly as now):
- `"scene": "wj"` is a gate scene and a label scene: it needs `turn_ms` as W3 does, passes the same trace checks (`trace_reasons`), the same condition checks and the same thresholds, and needs a passing `label_check`. The trace entries are those of W3 (`qpc`, `turn`, `phase`, `revision`), where `revision` is the pose revision.
- `"fixture"`: required when the scene is `wj`, refused (unreadable, reason `bad-fixture`) on any other scene and when it is not a string matching `NAME`. The three fixtures are named `wj-s4`, `wj-i-a` and `wj-i-b`; the judge accepts any `NAME`.
- `"pose_check"`: required when the scene is `wj`: `{"status": "pass" | "fail", "revisions": [<integer >= 0>, ...]}`. `revisions` lists, sorted ascending and without repeats, every pose revision whose first-use readback matched the engine arrays of that revision by SHA-256 and element by element. A missing or malformed record (another status value, a non-list, a non-integer or boolean entry, a negative, unsorted or repeated entry) makes the run unreadable with reason `missing-pose-check`.
- The judge reconciles the record with the trace. Let D be the set of `revision` values of every trace entry whose `qpc` lies in [`trace_start_qpc`, `trace_stop_qpc`] (the whole capture, not only the measured interval). The run is invalid with reason `pose-check-failed` unless `status` is `pass`, D is not empty and `revisions` equals the sorted D exactly. So a drawn revision without its readback fails, an under-reported or over-reported list fails, and a revision adopted but never drawn is neither required nor allowed.
- In a `wj` run, an entry whose `revision` differs from its `turn` gives the reason `pose-adoption-missed` instead of `label-adoption-missed`; the rule itself is unchanged.
- Grouping: runs are judged per (candidate, scene, feature, fixture), with fixture `none` when absent, so the three fixtures get three separate verdicts. Each scene entry of the result gains `"fixture"`. Sorting stays deterministic. Sanitizing is unchanged.

**Tests to add** (in the existing style, with the existing helpers):
- three valid `wj` runs of one fixture and one build meet the gate; the same frames as W3 give the same figures;
- a `wj` run without `fixture`, with a bad `fixture`, or a non-`wj` run with `fixture` is unreadable (`bad-fixture`);
- a `wj` run without `pose_check`, or with a malformed one (wrong status value, negative or non-integer counts, booleans as counts), is unreadable (`missing-pose-check`);
- `pose_check` with `status` `fail`, with an empty list, with one drawn revision missing, with an extra revision that no trace entry in the capture shows, and with a complete passing trace of many revisions but a list of only the first revision is invalid (`pose-check-failed`); a list equal to the drawn revisions passes;
- a failed `label_check` invalidates a `wj` run (`label-check-failed`);
- a `wj` run whose trace misses an adoption is invalid with `pose-adoption-missed`, and a W3 run with the same trace still gives `label-adoption-missed`;
- runs of two fixtures are judged separately and do not pool, and two runs of one fixture with a third of another give `insufficient-runs` for both;
- an idle gap, a skipped turn and a short capture invalidate a `wj` run as they do a W3 run.

Rules: Python files keep LF; `sys.dont_write_bytecode` stays as the test file sets it; the test writes nothing in the worktree and removes its fixture folders on every exit.

```implement-contract
{"allowed_files": ["tools/perf/renderer_gate.py", "tests/test_renderer_gate.py"], "acceptance_check": ["python", "-B", "tests/test_renderer_gate.py"], "stop_condition": "renderer_gate.py judges wj runs as section 6 says with every threshold unchanged; test_renderer_gate.py keeps every existing test and adds the section 6 tests; the acceptance check passes"}
```

## 7. Required return format
- Implementation: changes only in the assigned worktree. The final message lists the changed files, the acceptance result with the test count, each choice left open and the choice made, and anything in section 6 that conflicts with the existing judge code, with what you did.
