---
id: owner-decisions-2026-11-16-phs52
type: decision
status: verified
visibility: public
summary: Owner decisions of 16 November 2026 on five LP2 frozen-text readings (PHS.52) - the retry lookup keeps its frozen place before the token checks and the host stamps the caller from the channel an envelope came on, never from the envelope; a press whose preview slot is empty answers `KN5:stale.no_fresh_preview` and LI.33's vector is amended; an ungranted clock read, file read or network request is `LG3:capability.ambient_read` and dynamic code is `LG3:capability.not_granted`, with one chapter 13 amendment at LP3's command-table freeze; goal and evidence sinks take portable exact data, refused at admission and at run time; and EN4's layer interfaces are frozen, so changes to them follow PHS.52.
related: [owner-decisions-2026-11-16-lp2-questions, owner-decisions-2026-11-15-freeze, owner-decisions-2026-11-14-language-route, lg3-ambient-read-code, dynamic-code-refusal-code]
supersedes: []
claims:
  - {id: phs52-host-stamps-caller, evidence_kind: decision, checked_at: 2026-11-16}
  - {id: phs52-li33-follows-ruling-1, evidence_kind: decision, checked_at: 2026-11-16}
  - {id: phs52-chapter-13-codes, evidence_kind: decision, checked_at: 2026-11-16}
  - {id: phs52-exact-sinks, evidence_kind: decision, checked_at: 2026-11-16}
  - {id: phs52-li-frozen, evidence_kind: decision, checked_at: 2026-11-16}
  - {id: layer-stamps-caller, evidence_kind: source, path: rust/c600-command/src/layer.rs, checked_at: 2026-11-16}
  - {id: checker-capability-codes, evidence_kind: source, path: rust/c600-check/src/lib.rs, checked_at: 2026-11-16}
  - {id: interpreter-encodes-exact-sinks, evidence_kind: source, path: rust/c600-run/src/interp.rs, checked_at: 2026-11-16}
  - {id: checker-portable-sinks, evidence_kind: source, path: rust/c600-check/src/lib.rs, checked_at: 2026-11-16}
  - {id: kn5-retry-key, evidence_kind: source, path: docs/architecture/1.0/language/architecture/01-kernel.md, line: 486, checked_at: 2026-11-16}
  - {id: li5-digest-list, evidence_kind: source, path: docs/progress/1.0/layer-interfaces.md, line: 54, checked_at: 2026-11-16}
  - {id: li33-vector, evidence_kind: source, path: docs/progress/1.0/layer-interfaces.md, line: 388, checked_at: 2026-11-16}
  - {id: ambient-read-row, evidence_kind: source, path: docs/architecture/1.0/language/architecture/13-errors.md, line: 396, checked_at: 2026-11-16}
  - {id: li-frozen-with-architecture, evidence_kind: decision, checked_at: 2026-11-15}
---

# Owner decisions, 16 November 2026: five frozen-text readings (PHS.52)

While building LP2, the integrator met five places where the frozen text was silent, contradicted itself or was read
in a draft. They went to the owner as PHS.52 readings on the milestone's one decision card, with the recommended
option first ([time-saving decision](owner-decisions-2026-11-16-time-saving.md)). The owner chose the recommended
option each time, in Chinese, on 16 November 2026. This page records the answers in English, what was done, and the
amendment that carries them into the frozen text.

No frozen file is edited by these answers. The changes to frozen text go into one amendment that LP3 makes at its
command-table freeze (last section), under the revision, review and owner acceptance process of PHS.52.

The code that carries the answers out is commit `877f7925` (`rust/`): the command layer, the checker, the interpreter,
the runtime bridge, the module approval and their tests, with the open amendment items listed in `rust/README.md`.

## 1. The retry lookup keeps its place, and the host stamps the caller

**Question.** KN5.21 answers an identical retry from its receipt before the token checks
([01-kernel.md](../../architecture/1.0/language/architecture/01-kernel.md), line 486; LI.6). The command layer's
draft ruling 4 scopes command IDs per caller, puts the caller in the request digest and keeps the refusal of an
envelope that does not authenticate under the envelope's full digest, so a forged or runtime envelope sent first
cannot block the solver's press. The caller, though, was a field of the envelope, which a sender could set. The
options were to keep the frozen order and add a host rule (recommended), or to move authentication before the retry
lookup, which changes frozen text.

**Answer.** "Keep the order and add the host rule."

**Carried out.**

- The frozen order stays: the retry lookup comes before lane admission and before a token is checked or consumed.
- The host stamps the caller from the channel the envelope came on and never takes it from the envelope.
  `c600_command::Invocation` (LI.5's envelope) has no caller field. `CommandLayer::invoke` takes the channel's caller
  beside the envelope, and the request digest, the retry scope and the runtime's row checks use the stamped caller.
  The shell's channel stamps `Caller::Shell`, and LP3's runtime bridge stamps `Caller::Runtime` for its run.
- Tests that forged a caller in the envelope now send the same envelope on another channel.

**A note for the amendment.** LI.5's digest list does not name the caller
([layer-interfaces.md](../../progress/1.0/layer-interfaces.md), lines 54 to 56), while ruling 4's request digest
covers the stamped caller. The amendment adds that the engine's request digest covers the caller the host stamps from
the channel, and that the envelope names none.

**Follow-up.** M6's store keeps its own envelope with an `origin` field that the envelope supplies
(`c600-session`). The same host rule applies to it; M6's session owns that change.

## 2. LI.33's vector follows ruling 1

**Question.** The command layer's draft ruling 1 answers `KN5:stale.no_fresh_preview` for a press whose preview slot
is empty and `KN5:stale.preview_replaced` when the slot holds another preview. LI.33's vector answers
`preview_replaced` for a token whose preview was committed and whose slot is now empty
([layer-interfaces.md](../../progress/1.0/layer-interfaces.md), lines 388 to 389).

**Answer.** "Follow ruling 1."

**Carried out.** The command layer already answers `no_fresh_preview` for an empty slot; nothing in the code changes.
LI.33's sentence is amended in the amendment below.

## 3. Chapter 13 codes

**Answer.** "All as recommended."

**(a) An ungranted ambient read is `LG3:capability.ambient_read`.** Chapter 13 registers the code for a clock, file
or network read requested without a grant ([13-errors.md](../../architecture/1.0/language/architecture/13-errors.md),
line 396), LX-3's frozen corpus expects it for the probes P01 to P03, and no corpus case may be excused after its
result is observed. The checker now refuses with it:

- a clock capability or a network capability the run does not hold, and a file capability the run does not hold when
  the entry may read a file (a file read, or a call of a callable whose effect bound holds the file capability);
- a network capability a module manifest requests (network remains barred in 1.0).

Process access, session-file access, randomness and a file the entry only writes keep `LG3:capability.not_granted`
(LG3.8, LG3.9). One mapping in `c600-check` serves the checker's own admission, its built-in column and LP3's
approval of a built-in tool. CT-RTE-07 on 600-cell-Full now gives the corpus's code for P01 to P03. This answers the
question [lg3-ambient-read-code](../questions/lg3-ambient-read-code.md).

**(b) Dynamic code is `LG3:capability.not_granted` in both places.** The module loader already refused it with that
code. The checker reported an unresolved call with `LG3:effect.undeclared`; it now reports it with
`LG3:capability.not_granted` at the call's path (DET.36, RTE.45). The amendment adds DET.36 to that row's rule source.
This answers the question [dynamic-code-refusal-code](../questions/dynamic-code-refusal-code.md).

**(c) One amendment.** Chapter 13 is amended once, at LP3's command-table freeze, with the six unregistered draft
codes, the four M8 drafts and the errata below. Until then the drafts stay in use.

**(d) The four M8 drafts.**

- A repair list's `max_pairs` can take `KN7:budget.kernel_cost` once JI declares it part of that allowance.
- A blocked or closed named cap needs a new row, and so does a repair list in free mode. `named_object_invalid` and
  `menu_binding_mismatch` are not widened (ERR.5).
- A cancelled repair list needs no code (KN7.3's `cancelled`); `KN7:cancelled.by_user` exists if a code is wanted.

## 4. Goal and evidence sinks take portable exact data

**Question.** The frozen text does not say what a goal or evidence sink accepts. The checker admitted `use_evidence(f)`
with a function value, and the interpreter passed the value to the host's exact sink without encoding it, so a
function or a receipt could reach the host (against RPI.1 and RPI.54).

**Answer.** "Refuse it both at admission and at run time."

**Carried out.**

- At admission: the checker takes only portable exact data at `use_goal` and `use_evidence`: a value whose type holds
  no function, receipt, word equality key (LG2.13) or compressed word (DET.59) at any depth and is not a type variable.
  Any other value is refused with `LG2:type.mismatch` at the sink. An approximate value keeps its exactness codes
  (LG2.22). The checker does not unify type variables (M1), so it cannot see what a generic body's type variable
  stands for at a call; a value of a type variable is refused at the sink, and the exactness rules still check each
  call of such a body as before.
- At run time: the interpreter encodes the value at the exact sink with DET.59's priced encoding pass (an `encode` row
  before the `use_exact` row) and hands the host the canonical bytes, as at every other sink. A value with no
  canonical form stops the run with `PS1:input.noncanonical_value`, the code every other sink gives for it, so a
  function or a receipt never reaches the host.
- Whether a receipt may count as evidence is not decided. It goes on the next milestone's decision card.

## 5. Correction: EN4's layer interfaces are frozen

The command layer's crate documentation called LI's clauses drafts (draft ruling 8). The freeze record makes EN4's
layer interfaces part of the frozen architecture
([owner-decisions-2026-11-15-freeze](owner-decisions-2026-11-15-freeze.md)). The note now says so: a change to LI
follows PHS.52, and where LI is silent the layer's drafts stay drafts.

## The amendment at LP3's command-table freeze

One change to frozen text, made by LP3 at its command-table freeze under PHS.52, carries these items:

1. Chapter 13 registers the six draft codes `c600-command` emits: `KN2:consistency.star_certificate`,
   `KN5:input.unknown_row`, `KN5:input.argument_form`, `KN5:input.row_not_built`, `KN5:input.overlay_row_conflict`
   and `KN4:input.session_exists`.
2. Chapter 13 takes the four M8 drafts as section 3 (d) above settles them: new rows for a blocked or closed named
   cap and for a repair list in free mode, and `max_pairs` under `KN7:budget.kernel_cost` once JI declares it.
3. Chapter 13's errata: the LB6 closure row `evidence.decomposition_mismatch` moves to the head of ERR.31's table, and
   ERR.31 comes before ERR.29 and ERR.30; the rule source of `KN5:stale.filter_preview` adds KN0.9; the Used-by cells
   of `KN2:input.recipe_shape`, `KN2:resource_limit.primitive_ceiling` and `KN2:input.word_blocked_prefix` name their
   users; the Used-by cell of `LG3:capability.ambient_read` names the checker.
4. Chapter 13's row `LG3:capability.not_granted` adds DET.36 to its rule source.
5. LI.33's sentence answers `KN5:stale.no_fresh_preview` for the delivery whose preview C discarded.
6. LI.5 says that the engine's request digest covers the caller the host stamps from the channel, and that the
   envelope names no caller.

## What stays open

- Whether a receipt may count as evidence (next milestone card).
- The run-time answer to an ungranted clock or file read. The answer in section 3 (a) covers the checker's admission;
  a host that grants nothing still answers the request at run time with `LG3:capability.not_granted`
  (`c600_run::interp::Host`), as before.
- M6's envelope `origin` under the host rule (M6's session).
