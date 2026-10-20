# P1 refresh: change log

Draft: `i0/packets/P1.md`. Refreshed: `i0/packets/v2/P1.md`. Base: `97815b1` (`97815b19cd3243d30ed7c1632bc95b6e1751541e`),
the I0 commit, clean tree. The repository was only read. The working tree's status and ignored files were the same
before and after the measurements.

## 1. Base filled

- Every `BASE` placeholder now reads `97815b1`. The full sha appears once, in section 3.
- The sentence "If a line has moved, the named symbol is authoritative" is kept.
- New abbreviation `README` for `research/jumbling/restoration/13-tutorial-runner/README.md`. It replaces references to
  the private plan file where the README records the same text (result table, errors, clocks, plugin interface, I0
  review).

## 2. Citations

About 230 line references were checked at `97815b1`. The rule programs, fixtures, `O/H/RESULT.md`, `PROOFS-B.md`,
`seams.py`, `exact.py`, `test_r22d25.py`, the owner decision page, the checker's hygiene and v3 step checks and
`codex_review.py:105` were unchanged. Everything in `S/runner.py`, `S/runner_api.py` and `S/test_runner.py` had moved.

| Old | New | Symbol |
|---|---|---|
| `S/runner.py:588-597` (stub to stuck) | `:636-641`, reason stored `:644-645` | `Runner.plugin_phase` |
| `S/runner.py:593` | `:641` | `self.component(f'{phase}_stub')` |
| `S/runner.py:357-360` | `:399-402` | `Runner.child_env` |
| `S/runner.py:447` | `:489` | `Popen(..., cwd=cwd or ROOT)` in `execute` |
| `S/runner.py:50`, `:343-347`, `:311-313` | `:51`, `:385-389`, `:353` | `PLUGINS`, `plugin_modules`, `Runner.__init__` |
| `S/runner.py:392-393`, `:407-408` | `:434-435`, `:449-450` | `scripts` list and `config.scripts` hash in `initialize` |
| `S/runner.py:553-609` | `:601-657` | `Runner.plugin_phase` |
| `:560-567`, `:536-542`, `:565-567` | `:608-615`, `:584-590`, `:613-615` | entry test calls, `Runner.entry`, handoff re-test |
| `:572-578`, `:576-577`, `:579-580` | `:620-626`, `:624-625`, `:627-628` | Proposal branch, lattice, type check |
| `:581-582`, `:585-587`, `:588-590` | `:629-630`, `:632-635`, `:636-638` | `timed_out`, false handoff/lattice, non-stub `not_applicable` |
| `:594-597`, `:604-605`, `:607-608` | `:644-645`, `:652-653`, `:655-656` | stored End, outer `error`, `RuntimeError` |
| `S/runner.py:1259-1266` | `:1390-1397` | `Runner.run` except branch |
| `:611-622` | `:659-670` (and `run()` `:1382-1384`) | `Runner.phases_between` |
| `:544-551`, `:547`, `:545-546` | `:592-599`, `:595-596`, `:593-594` | `Runner.context` |
| `:48-49` | `:49-50` | `CLOCKS` |
| `:1384-1386`, `:1381-1382` | `:1515-1517`, `:1512-1513` (positivity `:1532-1536`) | `parse_args` |
| `:424-464` | `:466-506` | `Runner.execute` |
| `:429`, `:446-448` | `:471`, `:488-490` | log path, Popen |
| `:431-432` | `:473-474` | cancelled early return |
| `:437-457`, `:450-454` | `:479-486`, `:492-499` | `watch`, polling loop |
| `:443-444` | `:485-486` | complete lines in `watch` |
| `:264-271` | `:301-308` (returned `:506`) | `Child` |
| `:1011-1047` | `:1070-1113` | `Runner.choose` |
| `:643-674` | `:691-720` | `Runner.record_step` |
| `:646-647` (core keys inline) | `ROW_KEYS` `:66-67`, test `:694-697` | `ROW_KEYS` |
| `:648-651` | `:694-697` | one rejection test in `record_step` |
| `:152-172`, called `:655` | `:182-202`, called `:701` | `literal` |
| `:656-657` | `:702-703` | claimed `output_digest` |
| `:659-660`, `:659` | `:705-706`, `:705` | tree check, default `single` |
| `:663-666`, `:667-672` | `:709-712`, `:713-718` | word, tree, table writes; row |
| `:638-641`, `:664` | `:686-689`, `:710` | `Runner.write_tree` |
| `:175-235` | `:212-272` | `Mechanics` |
| `:195-198`, `:203-212`, `:214-216`, `:221-230`, `:232-235` | `:232-235`, `:240-249`, `:251-253`, `:258-267`, `:269-272` | `load`, `heights`, `snapshot`, `apply_word`, `inverse_word` |
| `:228-229` | `:265-266` | `ValueError('word output digest mismatch on own replay')` |
| `S/runner.py:33-34`, `:34`, `:572`, `:579` | `:34-35`, `:34-35`, `:620`, `:627` | `runner_api` import, isinstance tests |
| `S/runner_api.py:31-43` | `:34-46` | `Proposal` |
| `:46-55` | `:49-58` | `End` |
| `:20`, `:25`, `:27` | `:23`, `:28`, `:30` | `RESULTS`, `CLOCK_SENTINEL`, `FORBIDDEN_KEYS` |
| `:58-80`, `:75-77`, `:79-89` | `:90-112`, `:107-109`, `:111-112` and `:115-121` | `Ctx`, `Ctx.wall`, `Ctx.child_clock` and `child_clock` |
| `:101-104`, `:107-110`, `:113-126`, `:129-130` | `:133-136`, `:139-142`, `:145-158`, `:161-162` | `span`, `group`, `flat`, `single` |
| `:147-175` | `:179-207` | `check_tree` |
| `S/check_run.py:606-634` | `:612-648` (null branch `:617-629`) | `check_plugin_phase` |
| `check_run.py:1168-1202` | `:1197-1231` | height and kernel contract hooks |
| `check_run.py:991-1017` | `:1020-1046` (option `:1358`) | `check_run(allow_unchecked)` |
| `S/test_runner.py:1014-1018` | `:1091-1095` | stub dependency in the plugin-phase tests |
| `S/test_runner.py:466-472`, `:470` | `:503-509`, `:507` | `TemporaryTest.setUp`, `patch.dict` |
| `S/test_runner.py:2028-2036` | `:2410-2417` (`patch.dict` `:2413`) | `ExactTests.setUpClass` |
| `S/test_runner.py:2382-2427` | `:2816-2861` | worktree comparison |
| `:2383`, `:2386-2401`, `:2404-2406`, `:2409-2427` | `:2817`, `:2820-2835`, `:2838-2840`, `:2843-2861` | `SKIP`, `inventory`, `status`, `main` |
| `S/test_runner.py:32-35` | `:32-36` | import of `runner.py` as `tutorial_runner`, `api = R.api` |

New citations added: `S/runner_api.py:21-22` (`PHASE_ORDER`, `SUBPHASES`), `:56-58` (`End.__post_init__`), `:92-95`
(the `Ctx` docstring rule), `:210-214` (`api.write_tree`); `S/runner.py:317-319` (`RunError`), `:326-349`
(`kill_child`), `:237-238` (`export`), `:637` (`STUB` read), `:818-820` (the `jordan_target.py` CPU-cap reading);
`S/check_run.py:60` (`SUBPHASES`); `README:86-94`, `:101-129`, `:253-283`, `:285-321`, `:656-669`, `:925-955`,
`:942-951`.

## 3. Interface conflicts and corrections

1. **Pair script on the solved state (test 3).** The draft expected `r22d81_pairs.py --target 0` on a solved-state
   snapshot to print UNLOCKED with rc 0. Measured: it crashes with `IndexError` before START
   (`R/r22d81_pairs.py:195` → `eb6.py:125` → `sim/state.py:171`, `_groups` with an empty off set). The test now runs
   it on S2 (`7cd868f6…`) with `--target 35`: measured START tau 35, u 0, then UNLOCKED steps 0, rc 0, 8.1 s.
   4.2 (c) notes the crash; section 5 lists it as a plan correction.
2. **Cancelled child has no log.** `execute` returns `Child(-1, 0.0, log, cancelled=True)` at `S/runner.py:473-474`,
   before the log file is opened. The draft read `child.log` first and tested `cancelled` afterwards, which would
   raise `FileNotFoundError` out of the plugin. 6.3 now tests `cancelled` first and never opens the log then; 6.5 and
   test 4 follow.
3. **How the plugin reports an error.** The draft said only "→ `error`". At `97815b1` the runner's own integrity
   failures raise `RunError` (`S/runner.py:317-319`; I0-01), which the plugin cannot import (it imports only
   `runner_api`). 4.1 now states the rule: return `api.End('error', reason, details, attempts)`; never map a crash,
   truncated or malformed output, a missing STEP, a digest mismatch, inconsistent shard output or a cancelled child to
   `stuck`, `budget`, `timeout`, a failed attempt or a retry; do not raise for them (a raised exception loses the
   attempts and leaves the sub-phase result null); `stop_when` must not raise (`execute` calls it inside its loop,
   `:496`).
4. **SIGXCPU.** The draft mapped every `rc == -SIGXCPU` to `budget`. Every soft CPU limit is at least `int(cpu) + 600`
   CPU seconds and the children are single-threaded, so a SIGXCPU earlier than `cpu` wall seconds cannot be the
   child's own clock. 6.3 now requires `child.seconds` ≥ the `--cpu` value; an earlier SIGXCPU is a crash. Test 4 has
   both cases.
5. **Missing STEP.** The plan lists "a missing STEP" as an error; the draft had no rule for it. Every end line except
   NOT_APPLICABLE carries `steps` (checked in all four dialects). 6.3 now requires `steps == 0` on an end line with no
   deciding line before it; otherwise `error`, `missing_step`. Test 4 has the case.
6. **Exceptions out of `plugin_phase`.** The draft said an exception ends the run "the same way" as `End('error')`.
   At `97815b1` it does not: the sub-phase's `result` and `end` stay null, its reason, details and attempts are not
   stored, and the height result is not set (V1-B2; checker null branch `S/check_run.py:617-629`). 4.1 says so; the
   wrong-digest test now also expects `phases.height.result` null.
7. **Row keys.** The draft listed the core keys inline at `:646-647`; at `97815b1` they are the constant `ROW_KEYS`
   (`S/runner.py:66-67`), tested together with `FORBIDDEN_KEYS`, standing and `timed_out` in one test (`:694-697`).
   `literal` also limits items to 0 ≤ item < 60; the plugin's own check (1 to 59) is stricter.
8. **Tree checks.** The runner checks the tree at `:705-706` before `apply_word` and again inside `api.write_tree`
   (`S/runner_api.py:210-214`, via `:710`), not at the draft's lines.
9. **h4 prediction.** The draft said the `h4` call at S2 was "expected by reading the code, not measured" and that
   test 2 should not assert the family or letter. Measured: START tau 35, u 0, targets [[35, 1], [122, 1]]; STEP L4
   [35, 13] to `3ab25d21…`, tau null, u 0; HANDOFF. Test 2 now asserts the rule, word, contract, gate, after digest and
   tau/u fields, with the instruction to stop and report on any difference.

None of these needs a change outside `allowed_files`.

## 4. Measurements (20 October 2026, this container; not in the repository)

Scripts run unchanged as children with `--cpu 1e9`, from a scratch directory, with a warm home-centroid cache; states
built with the runner's `Mechanics`; snapshots in the `load()` format. Four shared cores, Python 3.13.16, NumPy 2.5.3.
Wall times include interpreter start and imports.

| Script | State | Result | rc | Wall |
|---|---|---|---|---|
| `r22d75_from_snapshot.py` | Z36 | STEP (12, 14) → `7cd868f6…`, STEP (35, 13) → `3ab25d21…`, HEIGHT2 | 0 | 14.1 s (12 s in an earlier run) |
| `h4_unlock.py` | `7cd868f6…` | START tau 35 u 0; STEP L4 (35, 13) → `3ab25d21…`; HANDOFF | 0 | 9.7 s (11 s earlier) |
| `r22d81_pairs.py --target 35` | `7cd868f6…` | START tau 35 u 0; UNLOCKED steps 0 | 0 | 8.1 s |
| `r22d81_rule.py` | solved | START tau null; HANDOFF steps 0 | 0 | 8.4 s |
| `r22d81_pairs.py --target 0` | solved | `IndexError` before START | 1 | – |
| all five, missing snapshot | – | `FileNotFoundError`, no `OverflowError`/`ValueError` | 1 | 5.6 to 6.6 s |

The full digests of R22D-36 (`Z36`), S2 and S3 were reproduced by replay with the runner's `Mechanics`.

## 5. Integrator decisions (open points)

See the structured result for options and reasons. Summary:

1. h4 START with tau null → `stuck`, `no_target`: keep (draft decision).
2. SIGXCPU → `budget` only when `child.seconds` ≥ `--cpu`: adopt.
3. `contract` = rule name, `gate` = script stem: keep; freeze with P4.
4. 600 s wall clock in test 2: keep.
5. NOT_APPLICABLE → `End('not_applicable')`, which the runner turns into a `ValueError`: keep the plan's mapping.
6. Cancelled child → `error`, `cancelled`: keep.
7. Assert the measured `h4` step in test 2: adopt.
8. `S/test_runner.py:1091-1095` depends on the stub: the integrator updates it after integration.
9. Missing-STEP check through `steps`: adopt.
10. Plan corrections (part 1 file ends; pairs clock test state): adopt and record in the plan.
11. P1/P4 field contract (section 6.4): freeze before launching both packets.

## 6. Acceptance runnability at `97815b1`

- `S/test_rules_height.py` does not exist; the packet creates it. `S/rules_height.py` exists (the stub).
- Every fixture named in 4.3 exists in the tree; the rule programs and `R/r22d31_snapshots.py` exist.
- `S/rules_kernel.py` and `S/lattice_phase.py` exist and are stubs (`STUB = True`), as test 2 needs.
- The wrapper's acceptance timeout is 1,800 s (`tools/agents/codex_review.py:105`).
- The acceptance check itself was not run (repository read-only for this refresh).

## 7. Adversarial verification fixes (P1-V-01 to P1-V-07), checked at `97815b1`

- P1-V-01 (major, adopted): `R/r22d81_rule.py:295-301` and `R/H/h4_unlock.py:308-312` play a letter, run `steps += 1`, then call `target()`, whose `checktime()` (`:155`, `:154`) can raise Budget before the STEP line; the BUDGET then has `steps` 1 with no STEP line (`:309-310`, `:319-320`). r22d75 (`:98`, one line) and r22d81_pairs (`:290-292`) report STEP right after `steps += 1`, so they cannot. 6.3 now accepts exactly two BUDGET exceptions (r22d81_rule with `played_unreported` step 1 family H6; h4_unlock with `families == {"L4": 1}`), mapped to `budget`/`cpu_budget`/`timed_out` with no step; every other mismatch stays `missing_step`. Test 4 gains both positive cases and a negative one (r22d81_rule BUDGET steps 1 without `played_unreported`); the old common test now uses non-BUDGET end lines plus the r22d75 and pairs BUDGET cases.
- P1-V-02 (minor, adopted): measured on the fixtures: `O/r22d81_from_stop.txt` STEPs 4-5 have tau 104 (START 274); census s7304 STEP 5 and s7306 STEPs 2-9 match the 4.3 tiers only against the previous line. 6.3 defines "the previous line" (previous START or STEP; for the deciding STEP it is START) and uses it for `input_digest`, tau and the tier; the 6.4 table, the 4.5 parsing note and test 1 (explicit assertions on r22d81_from_stop STEP 4 and census s7304 STEP 5 / s7306 STEP 2) follow. The `input_digest` check was changed with it (same defect: a START-relative check rejects every later fixture STEP).
- P1-V-03 (minor, adopted): `check_run.private_path` (`S/check_run.py:135-138`) is applied to every string by `record_paths` (:141-163), so a `rule_script` taken from a patched `RULE_SCRIPTS` would be a temporary absolute path. 6.1 adds a fixed, unpatched `RULE_SOURCES` mapping; 6.4 records `RULE_SOURCES[name].as_posix()`; test 5 asserts it in stub tests too.
- P1-V-04 (minor, adopted): `execute` returns before opening the log for a cancelled call (`S/runner.py:473-474`); `two_letter_u.py` prints CONTEXT only after its load and setup (`:46`, `:160-176`). 6.5 now applies the CONTEXT checks only to shards that printed one, judges a CONTEXT-less shard by its end (cancelled, timeout/stopped, budget, crash, truncated), makes "no CONTEXT at all" `undecided`, and choose_pair step 1 skips outputs without CONTEXT; one stub test added.
- P1-V-05 (minor, adopted): this container has RLIMIT_CORE soft 0, hard unlimited, `core_pattern` `core`; SIGXCPU's default action dumps core and children run in ROOT (`S/runner.py:489`). 6.6 requires each SIGXCPU stub to set `RLIMIT_CORE` to `(0, 0)` first.
- P1-V-06 (minor, consequence documented; mapping left open): a non-stub `not_applicable` always raises in `plugin_phase` (`S/runner.py:636-638`), so the run ends `error` with the sub-phase result null and the call's attempts dropped (`:1390-1397`; `S/check_run.py:617-629`). 6.3 now states this and test 4 expects the plugin-level `End` with its attempt. Whether to map NOT_APPLICABLE to `End('error', 'inconsistent_output')` instead (keeps the attempts, departs from the plan's "NOT_APPLICABLE → not_applicable") is open for the integrator.
- P1-V-07 (minor, adopted): no `seams` import in the five programs or their import chain (`eb6.py`, `r22d31_snapshots.py`, `sim/`, `theory/restore.py`, `theory/rim.py`, `theory/trees.py`, `fixtures/wj.py`); an isolated import of that chain (`python3 -I -B`, cwd outside the repository) left `seams` out of `sys.modules`. 6.6 drops the warm-up and keeps the temporary `--cache`; 4.4 notes the cache is not read. The verifier's 39.4 s warm-up time was not reproduced and is not cited in the packet.

## Integrator decisions (20 October 2026)

Applied to `v2/P1.md` with minimal edits; every other part of the packet is unchanged. Checked against `97815b1`.

### D1 (from P1-V-06 and cross finding X-02): NOT_APPLICABLE maps to `error`, applied

- A NOT_APPLICABLE line from a non-stub rule program (`r22d81_rule`, `r22d81_pairs`, or any other script that prints
  one) now maps to `End('error', 'invariant_height_above_6', details={max_height, digest})`, both values taken from the
  line, carrying the call's attempts. This is the reason of P1's own pre-call guard (6.2) and of P2. Both scripts print
  exactly `{"digest", "max_height"}` (`R/r22d81_rule.py:239-241`, `R/r22d81_pairs.py:191-193`).
- How the runner records it at `97815b1` (`S/runner.py`, `Runner.plugin_phase` and `Runner.run`): the sub-phase's
  `result` is `error`, its `end` the current snapshot, and its `reason`, `details` and `attempts` are stored
  (:642-645); the height phase's result is set to `error` (:652-653); the record is saved (:654); `RuntimeError('<sub>
  phase error: invariant_height_above_6')` is raised (:655-656, the reason through `safe_reason`); `run()` catches it,
  records the run `error` with that message as `error`, and saves (:1390-1397). The attempts are kept; the earlier
  steps stand; later sub-phases keep a null result. The old mapping lost all of this: `End('not_applicable')` from a
  non-stub plugin raises `ValueError` at :636-638 before :642.
- Checker side, `S/check_run.py` (not edited by any packet): `error` is a phase result (`PHASE_RESULTS`, :68). The
  plugin is called only while the entry test holds, so `check_plugin_phase` finds the sub-phase consistent with or
  without earlier steps (:637-647).
- P4 checked: no change. P4 never mentions `not_applicable` or NOT_APPLICABLE. Its only null-result text (P4 section
  4.1, the bullet on a phase "cut off by an exception" in an error run, I0-V1-01) concerns the high phase's letter
  bound and does not expect a null height sub-phase for this case.
- Edits in `P1.md`:
  - 6.3 mapping table. Before: "| NOT_APPLICABLE | `not_applicable` | `height_above_6` |". After: "| NOT_APPLICABLE
    (from any script that prints it: `r22d81_rule`, `r22d81_pairs`) | `error` | `invariant_height_above_6`, with
    `details` `{max_height, digest}` from the line |".
  - 6.3 sub-bullet. Before: "For NOT_APPLICABLE the consequence is fixed: from a plugin that is not a stub,
    `End('not_applicable')` always raises `ValueError` in `plugin_phase` ... This is the plan's mapping; do not change
    it." After: "**Integrator decision (20 October 2026), replacing the plan's "NOT_APPLICABLE → `not_applicable`"
    (4.5):** a NOT_APPLICABLE line maps to `End('error', 'invariant_height_above_6', details={'max_height': ...,
    'digest': ...})` ... For the `End('error', ...)`, `plugin_phase` at `97815b1`:" followed by the three recording
    steps above with their line numbers.
  - 6.3 "Attempts", `details` list: `max_height` added after `limit_cpu` (needed for the new details).
  - Test 4, r22d81_rule. Before: "NOT_APPLICABLE → `End('not_applicable', 'height_above_6')` carrying the call's
    attempt (the runner then raises and drops it, 6.3);". After: "NOT_APPLICABLE → `End('error',
    'invariant_height_above_6')` with `details` `{'max_height', 'digest'}` equal to the line's values, carrying the
    call's attempt (outcome `not_applicable`, `timed_out` false; 6.3). The same from the pair stub after a STUCK with
    tau 311, carrying both attempts;".
- Added in verification: 4.5 keeps the plan's line "NOT_APPLICABLE → `not_applicable`" as plan text but now marks it
  "(replaced by the integrator decision of 20 October 2026: NOT_APPLICABLE → `error`, reason
  `invariant_height_above_6`; 6.3)", since 4.5 is labelled binding; 4.1's list of `End('error', ...)` cases now names
  "a NOT_APPLICABLE line (6.3)". 4.1's description of the runner (`not_applicable` from a non-stub
  plugin raises, :636-638) is still true. The attempt outcome `not_applicable` stays in the outcome list: it names the
  child's end, not the result.
- This supersedes item 5 of section 5 above ("keep the plan's mapping") and the open question in P1-V-06 (section 7).

### D2 (from P1-V-01): mid-step BUDGET records no step, recorded only

- Kept as the packet states it (6.3, "Ends without a deciding line"; test 4): a BUDGET from `r22d81_rule` with `steps`
  1 and `played_unreported` (step 1, family H6), or from `h4_unlock` with `steps` 1 and `families` `{"L4": 1}`, records
  no step. The unreported letter is dropped; the sub-phase ends `budget`, reason `cpu_budget`, `timed_out: true`, and
  the run then ends `budget` (`phases_between`, `S/runner.py:659-670`; `run()`, :1382-1384). No edit.

### D9 (cross finding X-04): local fake plugins in test 2, applied

- Test 2 no longer uses the stub modules in the tree for the high and lattice phases. Before: "`plugins={'height':
  rules_height, 'high': <S/rules_kernel stub>, 'lattice': <S/lattice_phase stub>}`". After: "`plugins={'height':
  rules_height, 'high': <fake>, 'lattice': <fake>}` ... The two fakes are defined in `test_rules_height.py` itself, as
  packet P2's test does, and not imported from `S/rules_kernel.py` or `S/lattice_phase.py` (packets P2 and P3 replace
  those stubs): a fake high plugin (`STUB = True`, `next_step` returns `api.End('not_applicable', 'stub')`) and a fake
  lattice plugin whose `play()` raises if it is ever called (test 2 does not reach the lattice phase). Neither defines
  `SCRIPTS` (the runner reads it with a default of `[]`, `S/runner.py:434-435`)."
- The fake high plugin is not called in test 2: at S3 no piece is at height 4 or 6, so the high entry test fails and
  `plugin_phase` returns `not_applicable` without a call (`S/runner.py:608-609`). The expectation "high
  `not_applicable`" is unchanged.
- Added in verification: test 2's second `Runner` (at S2) said only "with the same options", and `plugins` is a
  constructor argument, not an option; without it `initialize()` imports the tree's modules (`plugin_modules`,
  `S/runner.py:385-389`, read at :434-435). It now reads "with the same options, the same `plugins` (the two fakes)
  and the class's `Mechanics`".
- This supersedes the line of section 6 above: "`S/rules_kernel.py` and `S/lattice_phase.py` exist and are stubs
  (`STUB = True`), as test 2 needs." Test 2 no longer needs them.
- Not changed: test 4's wrong-claimed-digest case runs `Runner.run()` without `plugins`, so it imports the tree's
  `rules_kernel` and `lattice_phase`. Neither is called there (the run ends `error` in `r22d75`), but the import and
  the hash of their `SCRIPTS` (`S/runner.py:434-435`, :449-450) still happen. D9 names test 2 only.

- Integrator, after the decision check: small wording and test fixes from the last verification (no change of behaviour beyond the decisions above).
