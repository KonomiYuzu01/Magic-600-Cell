---
id: owner-decisions-2026-11-14-language-route
type: decision
status: verified
visibility: public
summary: Owner decisions of 14 November 2026 on the language - the owner-machine timing window of that day counts as decisive; LQ8 is Rust for the engine, the session store, the command layer and the interpreter's host; LQ9 is route RT1, an own interpreter over the typed IR, with the runtime in a process of its own; LQ21 is capability separation. The decisive LX-1, LX-2 and LX-4 runs become LP3 acceptance checks with unchanged bars, and a failing one reopens LQ9. No ADR is accepted and nothing is frozen.
related: [owner-decisions-2026-11-06, owner-decisions-2026-11-05, owner-decisions-2026-11-03, owner-decisions-2026-11-14, engine-language-rust]
supersedes: []
claims:
  - {id: timing-window-decisive, evidence_kind: decision, checked_at: 2026-11-14}
  - {id: lq8-rust, evidence_kind: decision, checked_at: 2026-11-14}
  - {id: lq9-rt1-own-process, evidence_kind: decision, checked_at: 2026-11-14}
  - {id: lq21-capability-separation, evidence_kind: decision, checked_at: 2026-11-14}
  - {id: lx5-lx6-owner-machine-timing, evidence_kind: performance, path: work/experiments/language-lx/quiet-window/2026-11-14T1818Z/RESULT.md, checked_at: 2026-11-14}
---

# Owner decisions, 14 November 2026: engine language, route and flow control

On 14 November 2026 the owner said in Chinese that the machine was free, that the experiments should all run together
and that the architecture should then be frozen as soon as possible. The CPU timing runs went first. The integrator then
put four questions in the session, and the owner picked the recommended option in each. This page records the answers
in English.

## Decisions

| Question | Answer | Follows the recommendation |
| --- | --- | --- |
| Timing window | The window of 14 November 2026 counts: LX-5, the LX-5 crate follow-up and LX-6 are decisive evidence, although input was recorded during each run | Yes |
| LQ8 Engine language | Rust for the engine, the session store, the command layer and the interpreter's host. LP2 chooses the big-integer library on this data, and the Rust port adds the net-effect star cache | Yes |
| LQ9 Route and runtime placement | Decide now: route RT1, an own interpreter over the typed IR, with the runtime in a process of its own; the shell-process placement is not admitted. The decisive LX-1, LX-2 and LX-4 runs become LP3 acceptance checks with unchanged bars; if one fails, LQ9 reopens | Yes |
| LQ21 Flow control in the solving profile | Capability separation: a program either reads state or prepares previews. LX-3's two false refusals are handled by fixed declassifiers or by splitting the tool, and the decisive checks also go into LP3 acceptance | Yes |

1. **The timing window.** The runner records input during each run, and under its own rule that makes a run a lead.
   The owner accepted all three runs as decisive. On the owner's machine (Windows, CPU only), the Rust port of the
   engine's hot path was 2.3, 5.1 and 7.2 times faster than NumPy on three metrics with equal output digests; NumPy was
   15.2 times faster on the net effect, where it uses a warmed star cache that the port lacks. Every exact-arithmetic
   candidate was exact on the frozen cases, and the sums of the 190 row medians were 1,145.8 ms for the experiment's own
   limbs, 340.7 ms for `num-bigint` 0.5.1, 1,410.1 ms for Node BigInt and 136.6 ms for .NET BigInteger. The record is
   `work/experiments/language-lx/quiet-window/2026-11-14T1818Z/RESULT.md`.
2. **LQ8.** The 6 November answer was to decide after LX-5 and LX-6. The owner chose Rust on those runs.
3. **LQ9.** The 6 November answer was to decide after LX-1 to LX-5, together with LQ21. The owner chose on the first-pass
   leads instead of waiting for decisive LX-1, LX-2 and LX-4 runs, which need the Rust host, experiment versions of the
   LP3 tools and a renderer candidate presenting frames. Those runs keep their bars and become LP3 acceptance checks.
4. **LQ21.** On LX-3's RT1 stand-in, capability separation refused all 20 must-refuse cases with the expected code and
   falsely refused 2 of the 35 allowed tools, A25 and A30. Both flow-label forms refused 16 of the 20 with no false
   refusal.

## Integrator readings (the owner may change them)

- The engine-process placement is not admitted either, since the owner chose a process of its own.
- The decisive LX-3 checks that go into LP3 acceptance are its remaining ones: admitted-executor and platform coverage,
  the execution-contract mapping and product sandbox admission. The owner named LQ9 as what reopens on a failing LX-1,
  LX-2 or LX-4 run, so a failing LX-3 check goes to the owner.
- The fixed declassifiers are those that LQ2's joint setting enables. Until that setting, solver-tool solving previews
  stay disabled, so A25 and A30 wait for the setting or are split.
- The unselected routes, placements and flow-label forms stay in the architecture as records that bind nothing, so a
  reopened LQ9 finds them there. No clause ID was removed.
- A product dependency, a big-integer crate included, still needs the owner's approval. The allowance of 6 November
  covered experiment code only.

## What these answers do not settle

- LQ13 waits for LX-8 before LP2's parser, and LQ2's option set waits for the joint setting. The freeze rulings needed
  only LQ8, LQ9 and LQ21 first.
- The Linux legs of LX-5 and LX-6, whole-engine correctness and a net-effect comparison with the star cache remain
  owed, and no speed threshold is set.
- The decisive runs of LX-7 to LX-9 still wait for the owner's window.
- No ADR is accepted, nothing is frozen and no renderer is selected.

## Effect

- Superseded in part: the [6 November](owner-decisions-2026-11-06.md) answers to LQ9 and LQ21, which waited for the
  decisive LX-1 to LX-4 runs. The answer to LQ8 was followed.
- Answered: [engine-language-rust](../questions/engine-language-rust.md).
- The [route chapter](../../architecture/1.0/language/architecture/11-route-and-spikes.md) (RTE.17, RTE.21, RTE.24),
  the other chapters, the [ADR drafts](../../architecture/1.0/language/adr/README.md) and the briefing carry the answers.
- Unchanged: the model identities and mathematical contracts, the protection rules, the renderer selection gate, the
  human-solve boundary for built-in tools, the design-line hold and the Codex minimum.
- Next: the architecture-freeze and migration-format-freeze rulings, then the owner's ADR acceptance and sign-off.
