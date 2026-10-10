# Plan check: W-J judge scene and the local amendments to E-2.4-0J

Review only; do not perform follow-up work. Read only.

## 1. Goal and acceptance
- Goal: before two parallel implement calls start, check (a) the new packet `work/experiments/renderer-wj-packets/judge-wj.md`, which adds the `wj` scene to `tools/perf/renderer_gate.py`, and (b) the integrator's amendments to `docs/progress/1.0/packets/renderer/E-2.4-0J-jumbling.md` (diff against `HEAD`: `git diff HEAD -- docs/progress/1.0/packets/renderer/E-2.4-0J-jumbling.md`).
- Acceptance: a result per `schemas/review-result.schema.json`. A finding is `blocker` or `major` only when one of these holds: the judge packet weakens or changes the selection gate; it lets a W-J run with a stale, unchecked or partly checked pose revision count; it breaks an existing W3/W5 rule; the two packets define different run formats; or an amendment makes E-2.4-0J's acceptance check unable to pass on the owner's machine or lets it pass without the replays and digest checks the packet requires.
- Out of scope: the rest of E-2.4-0J (it was written and checked at mid-stage), the jumbling theory, the probe design, wording.

## 2. Actual problem and reproduction
- `renderer_gate.py` has `SCENES = ('w1', 'w2', 'w3', 'w3f', 'w4', 'w5')`, so every W-J run is unreadable; E-2.4-0J may not change files outside `work/experiments/renderer-wj/*`, and `tools/perf/check_renderer_packets.py` refuses an implement contract in the renderer packets folder whose files are outside `work/experiments/renderer-`, so the judge change is a separate packet outside that folder.
- E-2.4-0J was written in a Linux cloud session. On the owner's Windows machine:
  - the system `python` has no NumPy, while `research/jumbling/fixtures/wj.py` and `research/jumbling/sim` import it; the engine environment `tools/.venv/engine` has CPython 3.14.7 and NumPy 2.3.5. The wrapper replaces an acceptance argv starting with `python` by its own `sys.executable` (`tools/agents/codex_review.py`, `run_implement`), and passes the environment to Codex minus credential names (`scoped_env`, `child_env`);
  - `wj.py export` replays the J1 journal from the start for each stage (`export()` in `wj.py`); one replay of `wj-S4` to its end took 172 s here; the wrapper's `ACCEPTANCE_TIMEOUT` is 1,800 s;
  - the Codex sandbox refuses `tempfile.TemporaryDirectory()` and `mkdtemp()` with WinError 5 (observed in earlier calls on this machine).

## 3. Environment and versions
- Windows 11 (build 26200), CPython 3.14.7 (engine environment) and the system CPython 3.14 without NumPy. Branch `claude/renderer-l2-followup` with `origin/main` merged.

## 4. Necessary source and evidence
- `work/experiments/renderer-wj-packets/judge-wj.md` (new).
- `docs/progress/1.0/packets/renderer/E-2.4-0J-jumbling.md` and its diff against `HEAD`.
- `tools/perf/renderer_gate.py`, `tests/test_renderer_gate.py`, `tools/perf/check_renderer_packets.py`, `docs/progress/1.0/packets/renderer/README.md`.
- `research/jumbling/fixtures/wj.py` (`check`, `export`, `replay_with_refs`), `research/jumbling/sim/state.py` (`replay`, `apply`, `_compact`).
- `tools/agents/codex_review.py` (`run_implement`, `child_env`, `ACCEPTANCE_TIMEOUT`).

## 5. Attempts so far
None for these two changes. E-2.4-0J passed `check_renderer_packets.py` at mid-stage and passes it after the amendments.

## 6. Constraints and owned files
- Read only. The gate (README "Fixed inputs every packet repeats") is fixed; no review may propose relaxing it.
- Questions to answer explicitly in the summary:
  1. Is requiring `checked == adoptions` (every adopted revision checked) with `adoptions >= 1` the right acceptance for E-2.4-0J item 3, or does the trace's `revision == turn` rule leave a gap (for example a revision adopted but never drawn)?
  2. Should `wj` be a label scene (exact label check required) as the packet says?
  3. Is grouping by (candidate, scene, feature, fixture) enough to stop runs of different fixtures from pooling?
  4. Does taking all stages from one replay per fixture keep the same evidence as one `wj.py export` per stage?

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`, with findings citing `path:line`. Review only; do not perform follow-up work.
