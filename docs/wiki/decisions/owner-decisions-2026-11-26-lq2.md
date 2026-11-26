---
id: owner-decisions-2026-11-26-lq2
type: decision
status: verified
visibility: public
summary: Owner decision of 26 November 2026 at LQ2's joint setting, every recommended option - option set BS2 (the base tier); no exclusion for a preview that completes the puzzle; `imported` is a class of its own, refused at preview in a solving session, and `forked_built_in` is admitted like `own` within three limits; the flow rule is on with `own` and `forked_built_in` admitted; the `display` sink is checked only for imported tools that read the state; static facts split into state-independent and state-dependent ones; a preview names its source and the solve record holds the summary; BND.55's pick rule stands; the practice copy is a solving session. Overlay OV-5 records policy table version 1, and LP6's last start condition is met.
related: [owner-decisions-2026-11-03, owner-decisions-2026-11-06, owner-decisions-2026-11-14-language-route, owner-decisions-2026-11-18, owner-decisions-2026-11-22-adr-d01-overrules, owner-decisions-2026-11-24-lp4-lp5-exit]
supersedes: []
claims:
  - {id: lq2-option-set-bs2, evidence_kind: decision, checked_at: 2026-11-26}
  - {id: lq2-questions-answered, evidence_kind: decision, checked_at: 2026-11-26}
  - {id: lq2-policy-table-v1, evidence_kind: decision, checked_at: 2026-11-26}
  - {id: lp6-start-conditions-met, evidence_kind: decision, checked_at: 2026-11-26}
---

# Owner decision, 26 November 2026: LQ2's joint setting

**How the decision came about.**
- The owner asked when LP6 can start. The integrator answered that LQ2's joint setting was LP6's one open start
  condition (PHS.33, PHS.36, BND.71). Everything else was in place: the renderer was frozen on 16 November, the
  migration format on 15 November, and the design-system ADR was accepted on 23 November.
- The owner asked whether LQ2 could be decided now, then asked for the card the same day.
- The integrator drafted a recommendation for the option set and for each open question of BND.74 from
  [chapter 07](../../architecture/1.0/language/architecture/07-boundary.md).
- The mathematician checked the draft. Four changes were adopted:
  - question 1's reason now rests on BND.8;
  - question 2 gains limits on forks;
  - question 4 takes the rule that capability separation can enforce;
  - question 5 splits static facts in two.
- The card, one page in plain language, explained the option sets and the facts of chapter 07's section 2, as BND.71
  asks.
- The owner answered in chat: "All as recommended".

**Stated to the owner with the card.**
- Software cannot tell who thought of a word (BND.8).
- With `own` admitted, nothing in the solving profile stops a solver's own tool from searching and previewing. One
  program reads the state and writes a draft; another previews the draft's word. So the line rests on BND.3's reading,
  "the solver's own development", and on the records (BND.78), not on a technical barrier.
- The refusal of `imported` words is a label. Text retyped into a new module reads `own` (BND.20), and that channel
  cannot be closed (BND.47).

## Decisions

0. **The option set is BS2, the base tier.**
   - In the solving profile a solver's own tool calls only the base tier: an enumerated list of shipped functions.
   - No shipped function searches, minimises, ranks or selects over twists, words or caps (BND.64). The solver may
     build such a function from base parts. The research profile is unchanged.
   - Why not the other sets:
     - BS1 grants every read row, so "through the built-in base tools" (BND.3) would mean nothing.
     - BS3 gates previews on competence checks, which LQ14 says guide and never lock (BND.73).
     - BS4 refuses the solver's own programs' previews outside named targets, against ADR-D01.8.
   - The enumerated list, each entry with the mathematician's ruling (BND.67), is LP6's work.
1. **A preview that completes the puzzle is not excluded.** Whether the program chose a word is no function of
   (word, state) (BND.8). The last step of every human solve is such a preview, so the outcome is the wrong test.
2. **Author classes.**
   - `imported` stays a class of its own. In a solving session its words are refused at preview (BND.84, as
     ADR-D01.8 says), and it runs freely in tool authoring and research.
   - An edit never changes a class (BND.19).
   - `forked_built_in` is admitted like `own`, within three limits:
     - a fork of a research-tier entry does not enter the solving profile (BND.61, BND.64);
     - a fork of the repair list or of setup search is a program, not a fixed entry: `PickRecord.fixed` is set only
       for an entry whose digest is in the shipped manifest;
     - a fork inherits no declassifier (BND.53).
3. **The flow rule is on, and `admitted` is {`own`, `forked_built_in`}.**
   - An admitted tool's word reaches a preview through BND.53's admitted-class declassifier. This holds even when
     another admitted program computed the word from the state and it arrived by a draft. For these classes the
     separation of reading from previewing is waived through that declassifier, so the stop and emission channels no
     longer matter for them.
   - The rule still refuses `imported`, `unknown` and `research` words (BND.84, BND.21, BND.61).
   - BND.84's refusal no longer applies to `own` and `forked_built_in` (a PHS.52 revision, accepted by the owner
     with this decision). It stays in force for `imported` and `unknown`.
4. **The `display` sink is checked only for imported tools.** Under capability separation the provenance block holds
   only whether a run read the state (BND.39), so the checkable rule is coarse: in a solving session, an `imported` or
   `unknown` tool whose run read the state publishes nothing.
   - Built-in tools keep rule 6 and DT-J1's filter, unchanged.
   - Own and forked tools are not checked.
   - The cost: another author's view tool that reads the state does not run in a solving session. It still runs in
     tool authoring and research. This fills BND.59.
5. **Static facts come in two kinds.**
   - State-independent facts can be read by any program, a previewing one included. These are the 36 corrections
     (read per item, by name), the certified setup tables and the shipped seeds. A shipped entry that returns the
     correction combination stays out (BND.11).
   - State-dependent facts are N_c with its floor and a piece's domain and height. They are static only in rule 6's
     sense, so only a program that prepares no preview reads them. Built-in tools show them per cap or for what the
     solver names (BND.4, BND.10, BND.66).
   - Facts are classed by their information content, not by the row's name. A seed with a `derive` or replay row is
     state-equivalent, and a previewing program holds no `derive` row (BND.57).
6. **Labels.**
   - A shown preview names its source: the program and its author class.
   - A refused word shows its reason (ADR-D01.8).
   - The full `AssistanceSummary` (BND.35) is in the solve record.
   - No standing badge sits on the solving view.
9. **The pick rule of BND.55 stands.** A pick declassifies the solver's choice and not the option set. The picked word
   keeps the list's labels unless the list is a fixed entry (setup search, the repair list) or a fixed object set. A
   picked word takes the class of the list's producer, so a pick from an `imported` tool's list reads `imported`.
   BND.85's traversal enforces this. A pick from a built-in list that is not fixed keeps that list's labels.
10. **The practice copy is a solving session (BND.15).** One policy covers it. A tool is tried before a solve in tool
    authoring, where `propose` goes to a capture (BND.60).

Answered earlier, not asked again:
- question 7: the research library and archive checkers ship in 1.0 (LQ16);
- question 8: capability separation (LQ21).

There is no competence gate (LQ14). A 0.4 macro imported through path B (`producer: import`) is not an imported tool,
and BND.84 treats it as before.

## What follows

- **Policy table version 1.** Overlay [OV-5](../../progress/1.0/language-overlays.md) records it. Chapter 07 is an input
  of the frozen migration format packet, so its bytes stay as they are.
- **Implementation.** The host and command layer implement version 1 as LP6's boundary policy product (PHS.34), with
  the base-tier list. Until version 1 ships, the interim reading (BND.56, BND.69) stays in force in code. ADR-D01.8's
  program-prepared preview, which LP4's reading 1 held until this setting, opens with that implementation.
- **Tests.** CT-BND-26 to CT-BND-28 assert refusal for every program-produced word today, and change with the
  implementation. CT-LG3-06 and CT-LG3-07 also wait for BND.37's class review.
- **LP6.** Its start conditions are now met (PHS.36). The owner set its start the same day: when the ADR-D03 and K2
  lines start ([decision](owner-decisions-2026-11-26-log-flow.md)).
- **Briefing.** The briefing records this decision (`f7ea074d`). The briefing is an input of the frozen migration
  format packet, so its pin is refreshed under FP.1 in a separate commit: its SHA-256 goes from `f930b089...cee5` to
  `e15e6622...4405`, and `input_commit` from `1f941e6f` to `f7ea074d`. The packet cites none of the edited lines; the
  candidate files, `candidate_sha256` and every other pin are unchanged.

## What this does not do

- It accepts no ADR.
- It edits no frozen chapter text.
- It implements nothing.
- It rules on no base-tier entry.
