# Review packet: the finalizer counts whole seconds in the blind-seconds check

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: review the candidate on branch `claude/renderer-l2-followup`. It is not committed yet: review the working tree against `HEAD` (`git diff HEAD`) and this packet. Three files change:
  - `work/experiments/renderer-l2/finalize_run.py`: `check_blind_seconds` (line 364) counts whole seconds of the checked interval and drops a trailing fraction of a second. It used to round the interval up to whole seconds.
  - `work/experiments/renderer-l2/check_l2.py`: two new fixtures in `extra_cases` (lines 532 to 542).
  - `work/experiments/renderer-l2-packets/HARNESS.md`: the `blind-seconds` row of the refusal table (line 220).
- Acceptance: one JSON result matching `schemas/review-result.schema.json`. The verdict is `pass` if no `blocker` or `major` finding remains. Each finding needs a concrete counterexample: the markers, the PresentMon rows and the wrong outcome.
- Questions:
  1. Run mode with a complete run (`stop >= T0 + 190 s`): is the set of checked seconds identical to before, and identical to S-B's runner (`work/experiments/renderer-sb/probe/run_scene.ps1` lines 145 to 165)?
  2. Can a second without a displayed present now pass the check, other than the trailing fraction of under one second? Check the interval ends, the half-open bounds and integer division with negative or zero lengths.
  3. Short mode is a 30 s functional check, not gate evidence. Is leaving the last fraction of a second (under 1 s, inside `[T0 + 1 s, stop - 1 s)`) unchecked acceptable there? If not, propose the rule that should replace it.
  4. Do the two new fixtures pin the change? One must fail on the old rounding; the other must show that the last whole second is still checked.
  5. Does the `HARNESS.md` row describe the new behaviour exactly?
- Out of scope:
  - L2-V-002, which goes to a separate escalation;
  - every other part of level 2;
  - performance claims;
  - findings below `major`.

## 2. Actual problem and reproduction
On 4 October 2026 the owner ran the attended level 2 steps 4 to 7 on the RTX 4070 Laptop GPU. In step 5 each app makes two deliberate short-mode refusals: half-target (expected `size-mismatch`) and no-VRAM (expected `vram-missing`).
- Qt app: each refusal had exactly the expected reason.
- Godot app: each refusal also reported `blind-seconds`, so the refusal records read `[blind-seconds, size-mismatch]` and `[blind-seconds, vram-missing]`.

Analysis of the private records:
- The short interval is `[T0 + 1 s, stop - 1 s)`. `stop - T0` was 30000.491 ms and 30000.483 ms, so the interval is 28 s plus about 0.49 ms.
- The old code computed `seconds = ceil(length / frequency) = 29`. The 29th "second" was a 0.49 ms sliver.
- Godot presents at about 670 fps, roughly every 1.5 ms. The last displayed present inside the interval fell 1.09 ms and 0.98 ms before its end, so the sliver held none.
- Godot's valid step 4 short run passed by chance: sliver 1.012 ms, last present 0.773 ms before the end.
- Qt's runs also passed by chance: 60 fps, sliver 0.818 ms, last present 0.059 ms before the end.
- In every run, each whole second had displayed presents. The longest gap between presents inside the interval was 12.0 ms for Godot and 17.6 ms for Qt. Each Godot step 5 run had one row with `Dropped` 1.

## 3. Environment and versions
- Branch `claude/renderer-l2-followup`, from `main` at `7057cb8`. The level 2 finalizer came in through PR #57, merged into `main` with PR #54.
- Python 3.14 on Windows 11. QPC frequency in the fixtures: 6,000,000. On the owner's machine it was 10,000,000.

## 4. Necessary source and evidence
- `work/experiments/renderer-l2/finalize_run.py` lines 364 to 377 (`check_blind_seconds`) and the call in `finalize` (around line 449).
- `work/experiments/renderer-l2/check_l2.py`:
  - `fixture` (line 63): presents every `FREQUENCY // 60` ticks with a 1 ms offset; stop at `START + seconds * FREQUENCY`;
  - `refusal` (line 212);
  - the existing `blind-seconds` fixtures (lines 350 to 354);
  - the new fixtures (lines 532 to 542).
- `work/experiments/renderer-l2-packets/HARNESS.md` line 220.
- `work/experiments/renderer-sb/probe/run_scene.ps1` lines 145 to 165, for run mode.
- Results:
  - `python -B work/experiments/renderer-l2/check_l2.py` passes all 172 finalizer cases. The new cases:
    - `blind-seconds-trailing-fraction` (stop moved 0.5 ms later, no present in the sliver) is accepted;
    - `blind-seconds-last-whole-second` is refused with `blind-seconds` only.
  - Mutation check: the same run with the old rounding patched back in fails on `blind-seconds-trailing-fraction`: exit 5 where 0 was expected.
  - Input-only copies of the private Godot step 4 and step 5 records were finalized again with the fix. Results:
    - the half-target run gives `[size-mismatch]`;
    - the no-VRAM run gives `[vram-missing]`;
    - the valid short run is accepted, as before.
    The raw records stay private and are not part of this packet.

## 5. Attempts so far
This is the first fix. No earlier change touched the rounding.

## 6. Constraints and owned files
- Owned files: the three files of section 1 and this packet. No change to S-B's runner, to `tools/perf/renderer_gate.py` or to any app.
- These files are not on a critical path. Risk tier: other code and tools, one Sol review on the fast tier.
- Synthetic labels only. Raw PresentMon output, traces and machine diagnostics are never committed.

## 7. Required return format
One JSON object matching `schemas/review-result.schema.json`. Each finding gives:
- ID;
- severity;
- evidence as `path:line` with the file digest;
- counterexample;
- suggested experiment;
- verification status.
