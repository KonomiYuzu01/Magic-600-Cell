# LP4 human tests: form and results

The owner set this form on 25 November 2026, taking every recommended option of the integrator's one-page draft (form
version 1). EV5.12 leaves the form and its success criteria to the owner, and the mathematician checks their
mathematical content. The tests run at minimal scale (decision of 23 November 2026, item 2.2; PHS.27). No pass rate,
timing threshold or participant result is invented here: a row with no run says so.

## Who runs which test

- **CT-LB6-04 and CT-EV5-05:** the owner alone, as the person, coded O, on self-run sheets with sealed hints. Both
  clauses ask for "a person" or "people", not for non-authors.
- **CT-EV5-03:** its clause requires non-authors. It stays waiting until people are available, under the form below.

## Common rules

- **Help ladder:** 0 none; 1 a pointer to a glossary row or manual section; 2 the field to read; 3 the answer. On a
  self-run sheet each part's hints are sealed and revealed one level at a time, and the sheet logs the highest level
  opened. A part with no hint opened is at level 0.
- **Attempts and changes:** one attempt per task, corrected freely until "done". Results bind to the session pack's
  digests; a changed file reruns only its own tasks (PHS.4).
- **Material:** printed explanations and generated decompositions of the three built-in tools, since the product's
  explanation view is still owed (EV5.6, step 1). The sheet prints each record's specification, justification, worked
  instance with its givens, generated decomposition, leaf specifications and prerequisites, with the glossary rows the
  tasks name. It hides what the person is asked to produce: the steps' results, the step map, the reason, the failure
  example, what an answer does not mean and the expected claims.
- **Critical mistakes:** reading an empty or not-found-within-budget answer as "nothing exists"; reading a fixture pass
  or a digest match as proof of a general statement or as a checked certificate; giving a fork its parent's evidence.
- **Columns:** every recorded value has its own column. There is no combined score.
- **Boundary:** the tasks only read stored values or check given steps. None asks the program for a word or macro, and
  none opens a personal session (EV5.7).

## CT-LB6-04 and CT-EV5-05: one real instance per built-in tool

- **Tasks.** With one person, the owner takes all three instances. The draft split the second and third between two
  people. The instances belong to:
  - the jumbling tool (`jumble_checker`);
  - the 600-cell-Full tool (`full_600_cell`);
  - the representations tool (`representations`, whose instance uses `compare_cells`).

  For each instance the person does four things:
  - (a) checks each step from its named givens: the sheet hides the record's step texts and results, so the person
    redoes each step and gives its value with a one-line reason;
  - (b) maps each step to a decomposition node, or to "none", against the record's step map;
  - (c) expands the entry to its primitive leaves;
  - (d) states the reason and what an answer does not mean.

  Parts (a) and (b) answer CT-LB6-04's clause: check the steps by hand and relate them to the generated
  decomposition. Parts (c) and (d) answer the human half of CT-EV5-05: follow a real example to its primitives and its
  reasons.
- **Recorded per instance:** (a) to (c) right or wrong; (d) yes, partly or no against the mathematician's key; the help
  level of each part; critical mistakes; time from start to "done".
- **Criterion:** (a) to (d) are all met at help level 0 or 1, for every instance. Time is reported with no bar.
- **Step-count ceiling (LB6.2):** none until these results report.
- **Length:** about 45 minutes: about 10 to read the glossary rows the sheet prints, then about 10 per instance.
- **Review half of CT-EV5-05:** the mathematician reviews the three explanations and the shared grammar, and no blocker
  or major stays open. Each principle record's `review` field records its passes:
  - `jumble_checker`: the pass of 23 November 2026 found M6, and its scoped re-check that day confirmed the fix.
  - `full_600_cell`: the pass of 23 November 2026 found nothing, and the re-check recounted its generator claims. The
    pre-check below corrected its step map.
  - `representations`: the pass of 23 November 2026 found M15, and the scoped check of 25 November 2026 confirmed the
    fix.

  The glossary's rows were reviewed row by row on 24 November 2026. The shared grammar of EV5.11 waits for the theory
  book, which is not written, so CT-EV5-05 stays part after the human half passes, until that review.

## CT-EV5-03: fixture competence and transfer (form set; waiting for non-authors)

- **Participants:** 2 non-authors, coded P1 and P2, one 45-minute session each, after 15 minutes of reading at home
  (manual chapter 1 and the glossary rows the tasks name). A non-author wrote or reviewed none of the learning path,
  glossary, principle records, tools or fixtures, and has seen no answer key. The owner and agents do not count.
- **Moderator:** the owner or a person the owner names.
- **Tasks:**
  - Each person does step S1 of `learning-path.json` and three of S2 to S7, so that together they cover all seven.
  - Then two transfer tasks on new targets: S1 on `s7303-pstar.json`; then S4 on seed atlas orbit 12, or S7 on the
    600-cell-Full record's manifest digest.
- **Recorded per task:** correct, partly or wrong; critical mistake; help level; time to "done".
- **Criterion:** every task correct at help level 0 or 1, with no critical mistake. Time is reported with no bar. A
  missed task leaves the ID part, naming the task.

## The mathematician

- **Before the run:** the mathematician checks this form, the answer keys and the critical-mistake list.
- **After the run:** the mathematician classifies each wrong answer as the person's or the material's.

This is review, not human evidence (PHS.5, PHS.6).

### Pre-check of 25 November 2026

The mathematician found no blocker. Every key value is true and matches its record: item 45 is the inverse of item 12
by exact arithmetic on the fixture's menu, and generator 1 is H_0. These findings were adopted:
- **M1, major:** the sheet did not decide part (b). The sheet now says what a step maps to: the function or kernel leaf
  that makes or gives the step's conclusion, or "none" for a step the person derives alone. The key accepts every
  defensible node set. A step counts as right when it names a non-empty subset of its accepted set, or "none" where
  that is accepted. `full_600_cell`'s step map now maps step 5 (the end state is the base) to
  `kernel:propose(operation.preview)` on the certificate statement. It no longer maps it to `kernel:receipt(Base)`,
  which is the revision the run read.
- **Rubrics:**
  - part (a) is right only when every value and its reason are both true;
  - part (d) is "yes" for the right statement with a true reason, plus at least one true limit, and no false claim, and
    "partly" for only one of the two;
  - each part (d) names the step whose result its reason explains.
- **Smaller fixes:**
  - the printed evidence shows its check and, for imported evidence, that the producer is absent;
  - the printed glossary adds Cut chamber and Admissible, and the level-1 hint points to manual sections 1.1 to 1.3;
  - "without cancelling" is added to the expansion count;
  - the owner-facing hint says "the cap's side of its cut" for H_c⁺.

Pending the owner, so the form stays as set until the owner decides:
- **Three more critical mistakes (M2):**
  - extending a statement beyond its hypotheses;
  - treating the decomposition or the fixture as the reason (EV5.4);
  - reading a check, preview or view as a recommendation.

  They are watched and recorded in a column of their own, apart from the owner's list.
- **Adding "no critical mistake" to the criterion.**
- **Splitting the criterion per test (m5):** CT-LB6-04 on (a) and (b), CT-EV5-05's human half on (c) and (d). As set,
  the criterion is joint, so a miss in (d) also fails CT-LB6-04.

## Records and evidence

- **Private material:** the raw sheets and the keys stay under `work/loop-memory/human-tests/`, which Git ignores. A
  participant sees no key until their sheets are done.
- **This page:** the sanitized table: codes, per-part columns, the pack's digests and the form version.
- **What results are:** human evidence only, while fixture passes stay source evidence (EV5.4). No result certifies
  understanding (EV5.16) or grants a capability (EV5.9).

### Session pack O, 25 November 2026

Form lp4-human-tests v1 (owner, 25 November 2026). A source digest equals its record's `binding.source_sha256`. A changed input reruns only the tasks that read it (PHS.4).

| Input | SHA-256 |
| --- | --- |
| principle record `jumble_checker.json` | `3c3491f297b02bcbc08bca252e496c1201c5c10a3bf4fe6055af8555d1736bb7` |
| principle record `full_600_cell.json` | `0109449c7b387d7d78192f2ed6cb211fe34ab1327ddc758f8c6ee42fceb9a416` |
| principle record `representations.json` | `3e9610762f7c19ef4ff02a99e607be31459f187f6226bcf3ddd5c798a0742521` |
| glossary `terms.json` | `b55d533ff0b039ffedbf1e1bdb1acae642e904952229d02cd686f036b547201e` |
| `jumble_checker` source | `dddba17a57b35e56dc8bb64a6ef4ce12fe0e3e057f07f1ce1e9ea8a852f891fb` |
| `jumble_checker` generated decomposition | `166ecf43ed8434e478e4c2bbdf83e2e6d4071d91a489f285da922dc726e146e0` |
| `full_600_cell` source | `5d9e13fe6219fd88abe421660762bb5bfbe78694e4a3da1ed3390ca33ad76d2d` |
| `full_600_cell` generated decomposition | `2ef91cd6a0906f7196ee3dc3dbbf1401280cee157ba88704a4b9b00fe161d316` |
| `representations` source | `7332a75475c8971fa0688462d62736d1ec1b63c5d437d5bd07f890817479cc06` |
| `representations` generated decomposition | `b0eaddf5df284e419825deeef8c97fa94da6c95e85bbd38046dc6fe78e85cf5c` |
| the self-run sheet of O (private) | `16c3f38b0aed8bd8579c508b7e0f2089d92abfdf773c08769b745e2725de59f6` |

### Results

| Tests | Person | Instance | (a) | (b) | (c) | (d) | Help a, b, c, d | Critical mistake | Watched (pending the owner) | Time |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| CT-LB6-04, CT-EV5-05 | O | `jumble_checker` | not run | not run | not run | not run | not run | not run | not run | not run |
| CT-LB6-04, CT-EV5-05 | O | `full_600_cell` | not run | not run | not run | not run | not run | not run | not run | not run |
| CT-LB6-04, CT-EV5-05 | O | `representations` | not run | not run | not run | not run | not run | not run | not run | not run |
| CT-EV5-03 | P1, P2 | S1 to S7 and two transfers | waiting for non-authors | | | | | | | |

## Still open

- BS3's per-principle gate (EV5.9) waits for LQ2's joint setting.
- The shared grammar and the relationship strip (EV5.11) wait for the theory book.

## Owner hours

About 1 hour: about 45 minutes for the sheets and about 15 to read the result.
