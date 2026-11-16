---
id: dynamic-code-refusal-code
type: question
status: verified
visibility: public
summary: Answered on 16 November 2026 - dynamic code is refused with `LG3:capability.not_granted` in the checker and in the module loader alike, and the PHS.52 amendment at LP3's command-table freeze adds DET.36 to that row's rule source.
tags: [contradiction]
related: [owner-decisions-2026-11-14-language-route, owner-decisions-2026-11-15-freeze, owner-decisions-2026-11-16-phs52]
supersedes: []
claims:
  - {id: refusal-required, evidence_kind: source, path: docs/architecture/1.0/language/architecture/09-determinism.md, line: 264, checked_at: 2026-11-16}
  - {id: conformance-row, evidence_kind: source, path: docs/architecture/1.0/language/architecture/11-route-and-spikes.md, line: 780, checked_at: 2026-11-16}
  - {id: no-registry-row, evidence_kind: source, path: docs/architecture/1.0/language/architecture/13-errors.md, checked_at: 2026-11-16}
  - {id: lx3-open-gap, evidence_kind: fixture, path: work/experiments/language-lx/lx3-sandbox-flow/corpus/policy.json, checked_at: 2026-11-16}
  - {id: phs52-dynamic-code, evidence_kind: decision, checked_at: 2026-11-16}
  - {id: module-gives-not-granted, evidence_kind: source, path: rust/c600-module/src/approval.rs, checked_at: 2026-11-16}
  - {id: checker-gives-not-granted, evidence_kind: source, path: rust/c600-check/src/lib.rs, checked_at: 2026-11-16}
---

# Which code refuses dynamic code?

Status: **answered.** On 16 November 2026 the owner chose `LG3:capability.not_granted` in both places, with DET.36
added to that row's rule source, as one of the PHS.52 readings
([owner-decisions-2026-11-16-phs52](../decisions/owner-decisions-2026-11-16-phs52.md), section 3). The conflict was
found while the LP2 checker was extended the same day.

## The answer as carried out

- `c600-check` reports an unresolved call with `LG3:capability.not_granted` at the call's path (DET.36, RTE.45)
  ([lib.rs](../../../rust/c600-check/src/lib.rs)). `c600-module` already refused an unresolved call anywhere in a
  module's closure with that code, before any other finding
  ([approval.rs](../../../rust/c600-module/src/approval.rs)). One program now carries one code in both layers.
- A call of a function value whose callable the checker cannot see is unchanged: it keeps `LG3:effect.undeclared`
  (LG2.19, LG3.2). The answer covers an unresolved call only.
- Chapter 13 gets no new row. The PHS.52 amendment at LP3's command-table freeze adds DET.36 to the rule source of
  `LG3:capability.not_granted`; the frozen text is not edited before then.

## The conflict, as found

- DET.36 requires the refusal: dynamic code cannot supply an ungranted access path, and unresolved imports, dynamic
  code and unmodelled native calls refuse admission
  ([09-determinism.md](../../architecture/1.0/language/architecture/09-determinism.md), line 264). CT-RTE-08 checks
  that dynamic code is refused ([11-route-and-spikes.md](../../architecture/1.0/language/architecture/11-route-and-spikes.md),
  line 780).
- Chapter 13 registers no code for it
  ([13-errors.md](../../architecture/1.0/language/architecture/13-errors.md)).
- LX-3's policy keeps the open gap `dynamic_code`: no dedicated registry row, clauses LG3.12 and DET.36, nearest code
  `LG3:capability.not_granted` ([policy.json](../../../work/experiments/language-lx/lx3-sandbox-flow/corpus/policy.json)).
  LG3.12 records the unselected flow-label alternative and binds nothing since LQ21 was answered (owner,
  14 November 2026).
- On route RT1 an unresolved call in the typed IR is dynamic code; the surface cannot write one, since lowering refuses
  an unbound name. `c600-module` refused it with `LG3:capability.not_granted`, while `c600-check` reported the same
  call with `LG3:effect.undeclared`, because an unresolved callable is neither exact nor pure (LG2.19, LG3.2). One
  program thus carried two codes by layer, and neither row named DET.36 in its rule source.
