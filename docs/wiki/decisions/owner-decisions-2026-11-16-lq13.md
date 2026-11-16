---
id: owner-decisions-2026-11-16-lq13
type: decision
status: verified
visibility: public
summary: Owner decision of 16 November 2026 - LQ13 is answered - the source syntax of 600 Cell Script follows LX-8 candidate A (declaration keywords, braces, local let and an explicit return), chosen from a one-page comparison of the three LX-8 candidates without a human test. The meaning is unchanged, CT-RTE-18 stays not run, and the details are settled with M7's parser and freeze with the syntax.
related: [owner-decisions-2026-11-16-time-saving, owner-decisions-2026-11-06, owner-decisions-2026-11-15-freeze]
supersedes: []
claims:
  - {id: lq13-candidate-a, evidence_kind: decision, checked_at: 2026-11-16}
  - {id: lx8-source-equivalence, evidence_kind: source, path: work/experiments/language-lx/lx8-surfaces/RESULT.md, checked_at: 2026-11-16}
---

# Owner decision, 16 November 2026: LQ13, the syntax is candidate A (braces)

The owner gave this decision in chat in Chinese on 16 November 2026. This page records it in English.

## Decision

As point 1 of the [time-saving decision](owner-decisions-2026-11-16-time-saving.md) provides, the owner received a
one-page comparison of the three LX-8 candidates: A (braces), B (indentation) and C (expressions). It showed the same
two programs in all three spellings, a row-by-row table of their notation, the structural awkward spots that each
candidate's description records, what stays the same under any choice and the cost of choosing without a human test.
The owner read it and answered "pick A, braces".

LQ13 is answered: the source syntax of 600 Cell Script follows LX-8's candidate A, described in
`work/experiments/language-lx/lx8-surfaces/candidate-a/DESCRIPTION.md`:

- declaration keywords, braces, local `let` bindings and an explicit `return`;
- spaces and newlines separate tokens but do not end statements;
- top-level declarations start in column 1, so a broken draft can recover at the next declaration.

The extension `.lxa8` and the header `edition "lx8-0";` are experiment spellings only. Product source files stay
`.6cs` (LQ17), and the edition header's spelling is settled with the syntax (LG1.1).

The comparison recommended A and labelled that as its author's judgement, an agent's, not evidence: function bodies
do not depend on indentation, so code copied from chat, web pages or documents keeps its structure; the style is the
most familiar of the three; and a missing semicolon is reported as an error instead of silently changing the meaning.
No human evidence supports the recommendation, and the LX-8 record ranks and recommends nothing.

## What the choice leaves unchanged

- **The meaning.** In the LX-8 record (`work/experiments/language-lx/lx8-surfaces/RESULT.md`), each of the 17
  programs lowers to the same typed IR in all three spellings, and that IR equals the independently written expected
  JSON; the 48 broken drafts cover all six `LG1:syntax` codes, and 45 semantic mutation controls pass. Its check passed
  again on the main branch on 16 November 2026, with the frozen candidate and input digests unchanged. This is source
  evidence only.
- **M1**, the checker and the IR, is unaffected. Only M7's parser depends on the choice.
- **Details.** Keyword and punctuation details are settled when M7 builds the parser, and they freeze with the syntax
  and the IR later (PHS.49).
- **Editions** follow LG1.1, LG1.2, LG5.3 and LG5.7: every file names its edition; an unsupported edition is refused
  and never read under a default; a lockfile fixes each dependency's edition by content digest; and separate editions
  interoperate only through compatible typed contracts, never by reparsing a dependency under the importer's edition.
- **LG1's obligations.** The semantic, recovery and edition obligations remain (RTE.63). Only the human-test
  prerequisite is removed.

## The cost

As point 1 of the time-saving decision states: CT-RTE-18 is recorded as not run, never as passed, and there is no
human evidence on which syntax is easier to learn.

## Clauses

- **[P LQ13].** LG1.8 is the only clause with this tag, and the owner's answer to LQ13 settles it (chapter 00's tag
  table). Its requirements on a candidate, a parser, recovery diagnostics and lowering to the same typed IR, stand, and
  the LX-8 record meets them at source level for all three candidates. Its sentence that the choice waits for LX-8 is
  superseded by the time-saving decision. Within this decision's scope, the status lines that call [P LQ13] clauses
  conditional on the waiting answer, in chapter 00 and in the ADR index, are superseded too.
- **PHS.38.** LQ13's state is now A, reached before the parser, as its LP2 column requires.
- **Open for any later human test.** RTE.63 [O] (no LX-8 decision rule until the owner sets the protocol) and the six
  owner fields of `work/experiments/language-lx/lx8-surfaces/PROTOCOL-NEEDS.md` stay open: participants, tasks,
  sample plan, evaluation, stopping rules and inconclusive results. LQ13 no longer needs them; a later human test of
  the syntax would. CT-RTE-18 stays not run.
- **Old procedure.** The frozen restatements of the 6 November procedure that the time-saving decision lists stay
  superseded, among them chapter 11's LQ13 row, which says that the owner decides from their evidence: the owner
  decided from the comparison and the LX-8 source record.
- **No frozen text is edited.** The architecture keeps owner answers in the wiki decision record (PHS.48's
  owner-answer record), and no clause says that an answer is entered by editing the chapter text. The chapters'
  wording follows with M7, at the later syntax freeze (PHS.49).

## Where it is recorded

- This page.
- The briefing still says that LQ13 waits for LX-8. Its LQ13 phrases change in one later commit, followed by an FP.1
  pin refresh of the migration format packet, which pins the briefing.
- No progress item is ticked for LX-8 or CT-RTE-18. The checklist has no LQ13 item, and its LP0 item for the owner's
  answers to LQ1 to LQ21 stays open, because LQ2's option set still waits for the joint setting.
