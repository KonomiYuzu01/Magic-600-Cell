# LP4 human tests: form and results

The owner set this form on 25 November 2026, taking every recommended option of the integrator's one-page draft, and
amended its grading the same day with the three changes the mathematician's pre-check brought: this is form version 2
([decision](../../wiki/decisions/owner-decisions-2026-11-25.md), items 2 and 7). EV5.12 leaves the form and its success
criteria to the owner, and the mathematician checks their mathematical content. The tests run at minimal scale
(decision of 23 November 2026, item 2.2; PHS.27). No pass rate, timing threshold or participant result is invented
here: a row with no run says so.

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
- **Critical mistakes:**
  - reading an empty or not-found-within-budget answer as "nothing exists";
  - reading a fixture pass or a digest match as proof of a general statement or as a checked certificate;
  - giving a fork its parent's evidence;
  - extending a statement beyond its hypotheses;
  - treating the decomposition or the fixture as the reason (EV5.4);
  - reading a check, a preview or a view as a recommendation.
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
- **Criterion, by test:**
  - CT-LB6-04: (a) and (b) are met at help level 0 or 1, for every instance, with no critical mistake in them.
  - CT-EV5-05, human half: (c) and (d) are met at help level 0 or 1, for every instance, with no critical mistake in
    them.

  Time is reported with no bar.
- **Step-count ceiling (LB6.2):** none until these results report.
- **Length:** about 45 minutes: about 10 to read the glossary rows the sheet prints, then about 10 per instance.
- **Review half of CT-EV5-05:** the mathematician reviews the three explanations and the shared grammar, and no blocker
  or major stays open. Each principle record's `review` field records its passes:
  - `jumble_checker`: the pass of 23 November 2026 found M6, and its scoped re-check that day confirmed the fix. After
    the human test, the record took M25 (below), and the scoped check of 26 November 2026 found the new text true and
    its four minor points applied.
  - `full_600_cell`: the pass of 23 November 2026 found nothing, and the re-check recounted its generator claims. The
    pre-check below corrected its step map.
  - `representations`: the pass of 23 November 2026 found M15, and the scoped check of 25 November 2026 confirmed the
    fix. After the human test, the record took M30 (below), and the scoped check of 26 November 2026 found the new
    clause true of all six entries and corrected the text for `effect_scope`.

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

Three findings changed the form the owner had set, so they went to the owner, who took every recommended option the
same day (decision item 7). Version 2 carries them:
- **M2:** three more critical mistakes join the list: extending a statement beyond its hypotheses, treating the
  decomposition or the fixture as the reason, and reading a check, a preview or a view as a recommendation.
- **No critical mistake:** the criterion of CT-LB6-04 and CT-EV5-05 requires it, as CT-EV5-03's does.
- **m5:** the criterion is split by test. As first set, it was joint, so a miss in (d) also failed CT-LB6-04.

## Records and evidence

- **Private material:** the raw sheets and the keys stay under `work/loop-memory/human-tests/`, which Git ignores. A
  participant sees no key until their sheets are done.
- **This page:** the sanitized table: codes, per-part columns, the pack's digests and the form version.
- **What results are:** human evidence only, while fixture passes stay source evidence (EV5.4). No result certifies
  understanding (EV5.16) or grants a capability (EV5.9).

### Session pack O, 25 and 26 November 2026

The first sheet was built on 25 November 2026 under form version 1. Version 2 changes the grading only, and no text the
sheet shows.

The owner withdrew that sheet on 26 November 2026, before answering any part and before opening any hint, because its
material was too thin and not in plain language. This is a finding against the material, not a result: the printed
records alone did not serve as the explanation view that EV5.6 still owes.

Sheet version 2 (26 November 2026) asks the same questions in plain Chinese and checks against version 1's key. Ahead of
each record it adds a hand-written plain-language guide in Chinese, standing in for the owed explanation view. It also
folds the record's English text after the guide, and adds a reading guide to the decomposition's format and two
diagrams. It hides what version 1 hid. The mathematician checked the guide, the hints and the key before the run: no
blocker, and the two majors (wording that would have marked a right part (b) wrong) were fixed and re-checked.

The records and the generated material are the same for both sheets. A source digest equals its record's
`binding.source_sha256`. A changed input reruns only the tasks that read it (PHS.4).

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
| the self-run sheet of O, version 1, withdrawn (private) | `16c3f38b0aed8bd8579c508b7e0f2089d92abfdf773c08769b745e2725de59f6` |
| the self-run sheet of O, version 2 (private) | `8a756256f0b68eb76298db1e24af4c92d4c12ee16b8130874d4db47734256521` |

### Results

The owner ran sheet version 2 on 26 November 2026. The answers were graded twice and independently: first by LP4's line
main against the key, then by the mathematician, who also classified each miss. Where the two gradings differed, the
mathematician's ruling stands, and the section below lists each ruling. M24 turned a verdict from not met to pass after
the run, on the owner's own result, so it went to the owner. On 26 November 2026 the owner let it count as ruled, set
no rerun after the fixes, and closed the human-test line with this record
([decision](../../wiki/decisions/owner-decisions-2026-11-26-human-tests.md)).

The owner opened no hint on version 1, so nothing carries over. The owner never pressed "done": each time runs from that
instance's start to the next start or to the export, and the owner went back to `full_600_cell` once after starting
`representations`.

| Person | Instance | (a) | (b) | (c) | (d) | Help a, b, c, d | Critical mistakes | Time |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| O | `jumble_checker` | wrong: step 2's reason covers the first letter only (the person's) | wrong as keyed, step 1; right with the key fixed (M22) | right | yes | 0, 0, 0, 1 | none | about 13 min |
| O | `full_600_cell` | right; step 5 on the sheet's wording (M23) | wrong as keyed, steps 1 and 3; right with the key fixed (M22) | right | yes on post-run ruling M24, which the owner let count; partly under M3 as written | 1, 0, 0, 0 | none | about 76 min, with the shared reading |
| O | `representations` | wrong: step 1 gives no reason (the person's) | wrong as keyed, step 2; right with the key fixed (M22) | right | yes | 0, 0, 0, 0 | none | about 13 min |

| Test | Graded on | Verdict |
| --- | --- | --- |
| CT-LB6-04 | (a) and (b) of every instance | not met, run of 26 November 2026: (a) of `jumble_checker` and `representations`, both the person's; it holds under every ruling. The owner set no rerun |
| CT-EV5-05, human half | (c) and (d) of every instance | pass, run of 26 November 2026: (c) right and (d) yes for every instance at help level 0 or 1, no critical mistake. `full_600_cell`'s (d) is yes on post-run ruling M24, which the owner let count on 26 November 2026; under M3 as written it would be partly, and the half not met |
| CT-EV5-05, review half | the explanations and the shared grammar | part: the shared grammar waits for the theory book |
| CT-EV5-03 | S1 to S7 and two transfers, P1 and P2 | waiting for non-authors |

The results bind to the pack's digests above. After the run, and by the owner's decision without a rerun,
`jumble_checker.json` and `representations.json` changed for M25 and M30 (below). Their tasks would rerun under PHS.4,
but the owner set no rerun.

### Post-run check of 26 November 2026

The mathematician applied these rules to every answer:
- a misspelt node name counts when it names exactly one node;
- a reason that the sheet asked for but that appears nowhere in the instance is not met (the sheet asks for one on every
  step);
- a reason written in another part of the same instance counts for the step it names;
- a node set larger than the key accepts is wrong as the key is written;
- when a question has two readings that both fit the sheet, the answer is graded on the reading under which it is right,
  and the ambiguity is filed against the material.

There is no critical mistake. `jumble_checker`'s step 2 comes close to extending a statement beyond its hypotheses: the
answer leaves out the second letter's argument, but it makes no claim about the state after the first letter. The
verdict is the same either way.

The misses and their findings:
- **The person's:** `jumble_checker` (a) step 2 and `representations` (a) step 1. The sheet gave what each needed. Its
  background says that a cap's pieces stay on the cap's side of its cut, and its preamble asks for a reason on every
  step. M25 and M29 below are smaller contributions from the material.
- **The material's, part (b):** every (b) miss. These are the added `kernel:propose(operation.preview)` in steps 1 of
  `full_600_cell` and `jumble_checker`, the "none" in `full_600_cell`'s step 3, and the added `kernel:publish_view` in
  `representations`' step 2.

| Finding | Severity | What | Disposition |
| --- | --- | --- | --- |
| M21 | blocker for a rerun of (b) | The givens tables' source column ("this value is given by") names a leaf for the input, which contradicts the sheet's "none" rule for steps the person derives from the input. | not fixed: it concerns the sheet only, and no sheet runs again; any future sheet fixes it first |
| M22 | major | Four accepted sets are narrower than M1's rule that the key accepts every defensible node set: `full_600_cell` step 1 (add the preview leaf) and step 3 ("none"), `jumble_checker` steps 1, 4 and 5 (add the preview leaf), `representations` step 2 (add `kernel:publish_view`). | adopted as a ruling: the table shows both gradings, and the verdict does not change |
| M23 | major | `full_600_cell`'s step 5 also reads as the puzzle's state after the preview, which is always the base. | adopted as a ruling; the rewording is for any future sheet |
| M24 | major | The (d) question does not say what counts as a limit. M3's rule in the key also contradicts `representations`' accepted answer, which includes "it changes no state". A clause that rules out a misreading counts as a limit; one that repeats the input, output or domain row does not. | adopted as a ruling for `full_600_cell`'s (d): the owner let it count on 26 November 2026, so the human half of CT-EV5-05 passes. The wording is for any future sheet |
| M25 | minor | `jumble_checker`'s same-cap argument for step 2 is in no justification statement, and the sheet does not say that item 12 is a jumble. | fixed in the record without a rerun: a fourth statement, a lemma pinned to card 6 at its last commit, states the same-cap argument; the step map maps step 2 to the preview leaf on it; and step 2's text says that item 12 is a jumble |
| M26 | minor | `jumble_checker`'s first given holds a fixture path in its leaf field. | kept, with evidence: the record format defines a given's leaf as "the named leaf call or certificate the given is read from", and the pinned menu is that certificate |
| M27 | minor | With "done" never pressed, there is no time to done and the answers never locked; the export should warn. | not fixed: the sheet only; for any future sheet |
| M28 | minor | This page listed only sheet version 1's digest. | adopted: the pack table lists both sheets |
| M29 | minor | "Why?" is printed on some (a) steps only. | not fixed: the sheet only; for any future sheet |
| M30 | minor | `representations`' first justification says that the entries change no state, but three of them write the workspace. | fixed in the record without a rerun: the entries change no puzzle state, and an entry that keeps a value writes only that workspace record; the scoped check also corrected the text for `effect_scope` |

## Still open

- BS3's per-principle gate (EV5.9) waits for LQ2's joint setting.
- The shared grammar and the relationship strip (EV5.11) wait for the theory book.

## Owner hours

Planned: about 1 hour, about 45 minutes for the sheets and about 15 to read the result. Actual, on 26 November 2026:
about 1 hour 41 minutes from the first start to the export of sheet version 2. Before that, the owner spent some time on
version 1 before withdrawing it.
