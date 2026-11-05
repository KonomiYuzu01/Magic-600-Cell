# Corpus issue: B10 safety-stop trigger

Decision: **open**. Decider: **owner/integrator**. Corpus `lx3-neutral-v2` is unchanged.

`budgets.json`, B10, freezes ten loop iterations containing one tick and a host
deadline of 1 ms. Its expected outcome is `KN7:resource_limit.safety_stop`, with
no exact position promise. No stimulus keeps the loop live until that deadline.
It may finish before the deadline, even with a correct cooperative observer.
Delaying its ticks or injecting a deadline at an arbitrary deterministic
continuation would change the frozen stimulus and would not demonstrate the
specified wall deadline. The fake resource traps cannot supply a real clock.

The bench retains B10's neutral key in every cell. An invocation reaching its
admission returns `not_run`, reason `corpus_issue`; it is never a pass or a
denominator exclusion. Control-closed can refuse before that admission and is
reported as the wrong budget outcome. The four budget resources, prescribed
loop/chunk cancellation points, cache cases and actual child loss at admission
are independently executable and are retained.

The integrator must decide whether to version the corpus with a sustained
workload or an explicitly synthetic deadline event. This packet changes neither
the stimulus nor its expected outcome. Re-running the same bench on Linux does
not resolve this issue. No other case was declared a corpus issue.

## Integrator's decision, 5 November 2026

Corpus version 2 stays frozen, and B10 stays `not_run: corpus_issue` in every cell of every candidate. A wall-clock stop
is not reproducible by definition (KN7.10), and LX-2's loop with no kernel call gives the safety stop a sustained
workload, so this one case does not justify a third corpus version. The gap stays reported: LX-3 gives no evidence about
the wall-clock safety stop.
