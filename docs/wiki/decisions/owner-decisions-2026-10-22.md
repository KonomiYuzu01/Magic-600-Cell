---
id: owner-decisions-2026-10-22
type: decision
status: verified
visibility: public
summary: Owner answers of 22 October 2026, given for now - every recommended option of the re-plan and of design-table topic DT-J1; schedule option B with the 2.5 freeze at about stage day 34; the branch-only jumbling records come to main; ENG-0J starts; W-J runs in the renderer selection; design-table card limits.
related: [owner-decisions-2026-10-02, owner-decisions-2026-10-03, owner-decisions-2026-10-09-jumbling]
supersedes: []
claims:
  - {id: replan-q1-records-to-main, evidence_kind: decision, checked_at: 2026-10-22}
  - {id: replan-q2-eng-0j-starts, evidence_kind: decision, checked_at: 2026-10-22}
  - {id: replan-q3-wj-in-selection, evidence_kind: decision, checked_at: 2026-10-22}
  - {id: replan-q4-option-b, evidence_kind: decision, checked_at: 2026-10-22}
  - {id: dt-j1-batch, evidence_kind: decision, checked_at: 2026-10-22}
  - {id: design-table-card-limits, evidence_kind: decision, checked_at: 2026-10-22}
---

# Owner decisions, 22 October 2026: re-plan answers and the DT-J1 batch

The owner gave these answers in a chat message in Chinese, accepting every recommended option "for now"; this page records them in English. The owner may revisit any of them.

## Re-plan

The questions are in section 5 of the [re-plan](../../progress/1.0/replan-2026-10-22.md).

1. **Q1 (a): the branch-only jumbling records come to `main`.** The six decision records, the five handoff pages and the briefing line with the repair-list exception go to `main` in a documentation-only pull request. Draft pull request #112 stays a draft. This changes the 17 October rule that kept #112's pages off `main` until it merges.
2. **Q2 (a): ENG-0J starts now** on the parts that need no further owner answer. These are EN1 (revision 2, with a proposal for state-contract section 5 through an Astra plan check) and EN2 (the harness and the store).
3. **Q3 (a): the renderer selection runs W-J** on the selected candidate (RH-17).
4. **Q4: option B (cut).**
   - The 2.5 freeze holds 600-cell-Full in full.
   - Jumble enters with its rules, data, interfaces, command rows and renderer feature list. Its look and motion values enter as provisional defaults, which the owner tunes later through an ADR revision.
   - No G-step of 600-cell-Full is waived, and no gate is relaxed.
   - The freeze target moves to about stage day 34.
   - The owner gave no stage day. The count stays at 16, the lower bound on the record, until the owner corrects it.

## Design table

5. **DT-J1 closes on two rounds.** No third round runs.
6. **The DT-J1 batch** (the outcome of [jumble-solver-aids](../../progress/1.0/design-table/topics/jumble-solver-aids.md)):
   - O1 (b): the cap map keeps its fixed layout, with a threshold that the solver sets on N_c or the floor and no default; no sort.
   - O2 (a): the height counts and u are both shown.
   - O3 (b): the covers query takes one cap that the solver names and answers in both directions.
   - O4 (b): the solver's own checks at the named cap are kept for the revision, in the order tried.
   - O5 (a): no unlock lists, existence tests or V search in 1.0.
   - O6 (b): the ν marks and the static table, plus a check of a set of corrections that the solver drafts. The program never shows the combination.
   - O7 (a): the 36 corrections ship as constructions that the solver selects by id. This is the one named exception to the rule that no research word becomes program content, and it extends the 3 October ruling on selected constructions to them.
   - O8 (1): a solve ends on the sticker test. Jumble turns off at N = 0 and ν = 0 through a label checkpoint certified by G = 𝒦, and 600-cell-Full's tools run after the crossing. No separate pose fact is shown. The change to state-contract section 5 goes through EN1's Astra plan check.
   - O9 (a): the repair list opens with no row focused.
   - O10 (b): a repair is committed once, after a preview that the solver can step through twist by twist.
   - O11 (b): each repair row also shows its change in N as a numeral, in a fixed column. This adds one field to the rows that the 17 October exception names.
7. **Card limits from the next topic.** In round 1 each role gives at most six cards. In round 2 each role gives at most four, each answering other cards by ID.

## Effect

- **Pages.** The re-plan (section 7), the DT-J1 topic, the design-table protocol, the renderer selection packet E-2.4-05, the renderer experiment plan and the stage 2 schedule record these answers.
- **Next.** The documentation-only pull request of Q1; EN1 and EN2; the next design-table topic, DT-M1.
- **Unchanged.** The human-solve boundary of 2 October 2026 holds, with the exceptions and extensions named above. The research stays stopped.
