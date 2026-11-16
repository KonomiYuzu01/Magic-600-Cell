---
id: dynamic-code-refusal-code
type: question
status: draft
visibility: public
summary: Contradiction - the frozen chapters require dynamic code to be refused, but chapter 13 registers no code for it; LX-3 records the gap with `LG3:capability.not_granted` as its nearest code, `c600-module` refuses with that code, and `c600-check` reports the same call with `LG3:effect.undeclared`.
tags: [contradiction]
related: [owner-decisions-2026-11-14-language-route, owner-decisions-2026-11-15-freeze]
supersedes: []
claims:
  - {id: refusal-required, evidence_kind: source, path: docs/architecture/1.0/language/architecture/09-determinism.md, line: 264, checked_at: 2026-11-16}
  - {id: conformance-row, evidence_kind: source, path: docs/architecture/1.0/language/architecture/11-route-and-spikes.md, line: 780, checked_at: 2026-11-16}
  - {id: no-registry-row, evidence_kind: source, path: docs/architecture/1.0/language/architecture/13-errors.md, checked_at: 2026-11-16}
  - {id: lx3-open-gap, evidence_kind: fixture, path: work/experiments/language-lx/lx3-sandbox-flow/corpus/policy.json, checked_at: 2026-11-16}
  - {id: module-gives-not-granted, evidence_kind: source, path: rust/c600-module/src/approval.rs, checked_at: 2026-11-16}
  - {id: checker-gives-effect-undeclared, evidence_kind: source, path: rust/c600-check/src/lib.rs, checked_at: 2026-11-16}
---

# Which code refuses dynamic code?

Status: **open.** Found while the LP2 checker was extended on 16 November 2026. This page records the conflict; it
changes no rule and accepts no decision.

## The conflict

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
  an unbound name. `c600-module` refuses an unresolved call anywhere in a module's closure with
  `LG3:capability.not_granted`, before any other finding ([approval.rs](../../../rust/c600-module/src/approval.rs)).
- `c600-check` reports the same call with `LG3:effect.undeclared` at its path, because an unresolved callable is
  neither exact nor pure (LG2.19, LG3.2). Its solving-profile admission and its built-in column also refuse the entry
  with `LG3:capability.not_granted` ([lib.rs](../../../rust/c600-check/src/lib.rs)). One program thus carries two
  codes by layer, and neither row names DET.36 in its rule source.

## What needs deciding

Which registry code a dynamic-code refusal carries: a dedicated chapter 13 row, or `LG3:capability.not_granted` with
DET.36 in its rule source. Either changes frozen text and follows the revision, review and owner acceptance process of
PHS.52. Until then the implementations keep the nearest code LX-3 recorded, and the refusal itself holds.
