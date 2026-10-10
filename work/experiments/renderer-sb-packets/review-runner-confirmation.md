# Review packet: operator confirmation in the three capture runners

Review only; do not perform follow-up work.

`python tools/agents/codex_review.py --kind review --model gpt-6.1-sol --effort max --speed fast --packet work/experiments/renderer-sb-packets/review-runner-confirmation.md`

## 1. Goal and acceptance
- Goal: two small fixes to the post-run operator confirmation of the attended PresentMon runners. The fixes came out of the 10 October GPU session, where the owner re-ran W3 after the area-centroid anchor change. This is non-critical experiment tooling, so it gets this one Sol review.
  1. W-J runner (`work/experiments/renderer-wj/run_scene.ps1`): the confirmation step assigned its text to `$overlay`. PowerShell variable names ignore case, so this is the parameter `$Overlay`, which carries `[ValidateSet('none','grips','certificates','straddles','angle')]` (line 5). The assignment threw, so every series stopped after run 1, and run.json kept its `"OPERATOR-CONFIRMATION-PENDING"` and `"FILL-FROM-CSV"` placeholders. The fix renames the variable to `$declaredOverlays` (lines 175-189).
  2. All three runners (`work/experiments/renderer-sb/probe/run_scene.ps1:172-175`, `work/experiments/renderer-l2/run_scene.ps1:255-258`, `work/experiments/renderer-wj/run_scene.ps1:178-181`): before the `Read-Host` confirmation, call `try { $Host.UI.RawUI.FlushInputBuffer() } catch { }`. Keys typed or pasted while the run is on screen then cannot become the answer, or later commands at the PowerShell prompt. Pastes come from a right-click, or a two-finger tap on the touchpad, in the console.
- Acceptance: one JSON result matching `schemas/review-result.schema.json` that answers:
  1. Does any other assignment in the W-J runner still write to a parameter variable under a different case, or is any other read of the old `$overlay` left? Check the whole file, not only the changed lines.
  2. Does the rename keep the record contract? The placeholder replacement must still produce the same `declared.overlays` text as the S-B runner (`work/experiments/renderer-sb/probe/run_scene.ps1:170-183`) for a confirmed run and for a fault-injection run, and `tools/perf/renderer_gate.py` must still read it.
  3. Can the flush weaken the confirmation? For example, could it accept an answer that was not typed at the prompt, skip the prompt, change `$answer`, or turn a non-`yes` answer into a kept run? Could the empty `catch` hide a failure that should stop the run? The intended behaviour: on a host without a console input buffer, the prompt works as before.
  4. Does the flush run on every path that reaches `Read-Host`, and only there? It must not run for fault-injection runs, which skip the prompt.
- Verdict `pass` when nothing reaches `major`.
- Out of scope: the rest of the runners (capture, guard, mutex, trace-session ownership and the CSV checks were reviewed earlier), the probe and framework sources, the gate tool, and the measured results.

## 2. Actual problem and reproduction
- Observed (W-J, first attempt, unmodified runner): after run 1 of each series:

```
Cannot validate argument on parameter 'Overlay'. The argument "none running; operator-declared after the run: watched throughout, no visible obstruction; automatic checks sample visibility every 100 ms and do not prove continuous full-area visibility" does not belong to the set "none,grips,certificates,straddles,angle" specified by the ValidateSet attribute. Supply an argument that is in the set and then try the command again.
```

  - The run directory kept both placeholders.
  - The S-B and L2 runners have no `-Overlay` parameter. They passed the same session with `$overlay`, and they keep it.
- Observed (W-J, I_a series, fixed name, before the flush): run 2 was refused. The text at the prompt was the I_a command line, which the operator had copied earlier, followed by `yes`. The session event log shows no tool input from any agent in that window. The likely source is a paste that the console buffered while the probe was on screen, which echoed when `Read-Host` started. The runner refused it correctly (`.Trim() -ne 'yes'`). The flush makes this refusal unlikely.
- Expected: a confirmed run fills both placeholders and keeps the run; any answer other than `yes` typed at the prompt refuses it.

## 3. Environment and versions
- Base: `claude/anchor-reacceptance` at `e50c6d6`, plus the uncommitted changes to the three runners.
- Windows 11, Windows PowerShell 5.1 (the runners' host), administrator console in Windows Terminal; PresentMon 2.6.
- The review sandbox is read-only, without network or GPU.

## 4. Necessary source and evidence
- `git diff` of the three runners against `e50c6d6` (13 insertions, 3 deletions).
- The W-J runner's parameter block (lines 1-10), its confirmation and record step (lines 170-200), and the S-B runner's matching block (lines 165-190).
- `tools/perf/renderer_gate.py`, which reads `environment.declared.overlays`.
- Local checks after the change:
  - The PowerShell parser reports 0 errors for each of the three runners.
  - Replay of the W-J confirmation step: lines from `if ($Inject)` to the probe-exit line, with `Read-Host` stubbed to `yes` and the `ValidateSet` parameter `$Overlay` in scope. The replay used a copy of an unconfirmed first-attempt run directory. The unmodified runner fails with the error above. The changed runner fills `swap_chain` `0x00000208F0F2A590` and the confirmation text, leaves no placeholder, and `$Overlay` stays `none`.
  - `$Host.UI.RawUI.FlushInputBuffer()` returns without error in a non-interactive Windows PowerShell 5.1 host.
- Attended results with the renamed variable (before the flush was added; private records): 12 W-J runs (S4, I_a, I_b, W3; 3 each) filled both placeholders and passed `tools/perf/renderer_gate.py` as `met`.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | `$overlay` is the `ValidateSet` parameter `$Overlay` | rename to `$declaredOverlays` | replay; 12 attended runs | the old script fails as observed; the new one passes |
| 2 | Buffered console input reaches the prompt | flush before `Read-Host` | parser; non-interactive host call | parses, no error; not yet tested in an attended run |

## 6. Constraints and owned files
- Only the three runners change. No raw PresentMon output, traces or machine diagnostics are committed.
- The confirmation must stay an exact, operator-typed `yes`. Fault-injection runs keep skipping the prompt and stay non-gate evidence.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
