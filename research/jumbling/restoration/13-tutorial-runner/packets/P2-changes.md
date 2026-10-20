# P2 refresh: change log

Packet P2 (R-K: the high phase, the R22D-25 kernel with D19, runner side). The draft was checked against base
`97815b1` (`97815b19cd3243d30ed7c1632bc95b6e1751541e`). The refreshed packet is `v2/P2.md` next to this file.

Abbreviations: S = `research/jumbling/restoration/13-tutorial-runner/scripts`;
K = `research/jumbling/restoration/12-low-height/scripts/round22`.

## 1. Base filled

- The placeholder is now `97815b1`. The full sha appears once, in section 3.
- The sentence "if a line has moved in your worktree, the named symbol governs" is kept.
- Section 3 now says that `97815b1` is I0 with the fixes of its review (I0-01 to I0-04, V1-B2) and of its scoped
  verification (I0-V1-01). The second verification round passed (stage 13 README, section 7).
- The draft's claim that I0 was still changing `test_runner.py` and `check_run.py` is removed. At BASE they are final.

## 2. Citations

About 209 citations were checked: named `path:line` ones and bare `:NNN` ones. Every K citation was correct. K is
unchanged since `31b78a5` and `e37f96b`. So were the stage 12 README (:1343-1378), PROOFS-B (:2413-2423), the
decision page (:54-57, :68, :77-80, :89-96), `seams.py:66` and `codex_review.py:105` and :499.

Re-anchored, by symbol (old -> new):

S/runner.py
- ROOT: 31 -> 32. `sys.path` insert: 33 -> 34. CLOCKS: 48-49 -> 49-50. PLUGINS: 50 -> 51. SCRIPTS: 54-61 -> 68-75.
- `atomic_json`: 69-78 -> 83-92. `Mechanics`: 175-235 -> 212-272. `Mechanics.snapshot`: 214-216 -> 251-253.
  `Mechanics.apply_word`: 221-230 -> 258-267. `Child`: 264-271 -> 301-308.
- `Runner.__init__`: 311 -> 353. `plugin_modules`: 343-347 -> 385-389. `child_env`: 357-360 -> 399-402.
- `initialize` scripts: 392-393 -> 434-435. `config.kernel`: 402-403 -> 444-445. Scripts hash: 407-408 -> 449-450.
- `execute`: 424-464 -> 466-506. Log open: 429/446 -> 471/488.
- `entry`: 536-542 -> 584-590. `context`: 544-550 -> 592-599. Work dir: 546 -> 595.
- `plugin_phase`: 552-608 -> 601-657. Inside it:
  - handoff: 564-566 -> 613-615;
  - `high_steps`: 567-568 -> 616-617;
  - Proposal branch: 571-577 -> 620-626, and 575-576 -> 624-625;
  - `isinstance` checks: 571-578 -> 620 and 627;
  - `timed_out` check: 580-581 -> 629-630;
  - false claims: 583-586 -> 632-635, and 583-589 -> 632-638;
  - `not_applicable` stub branch: 587-589 -> 636-638, and 587-592 -> 636-641;
  - reason, details and attempts: 595-596 -> 644-645;
  - error raise: 606-607 -> 654-656.
- `phases_between`: 610-621 -> 659-670. `record_step`: 642-671 -> 691-720. Its checks: 645-656 -> 694-703. Tree
  and replay: 655-657 -> 701-707.
- `run` budget path, tail and lattice: 1245-1251 -> 1382-1388. `finish`: 1268-1285 -> 1405-1422.
- `parse_args` options: 1375-1383 -> 1512-1520. Non-negative check: 1390-1394 -> 1527-1531. Positive check:
  1395-1399 -> 1532-1536.

S/runner_api.py
- FORMAT 17 -> 20. RESULTS 20 -> 23. STANDINGS 21 -> 24. MODES 22 -> 25. CLOCK_SENTINEL 25 -> 28. FORBIDDEN_KEYS
  27 -> 30.
- `Proposal` 31-43 -> 34-46. `End` 46-55 -> 49-58. `Ctx` 58-80 -> 90-112. `Ctx.wall` 75-77 -> 107-109.
  `Ctx.child_clock` 79-80 -> 111-112. `child_clock` 83-89 -> 115-121.
- `span` 101-104 -> 133-136. `group` 107-110 -> 139-142. `flat` 113-126 -> 145-158. `single` 129-130 -> 161-162.
  `check_tree` 147-172 -> 179-207. `write_tree` 175-179 -> 210-214.

S/test_runner.py
- Runner import: 24-29 -> 30-36. `FakeState` and `FakeMechanics`: 206-245 -> 527-599.
- Worktree check model: 1122-1164 -> 2816-2861. `SKIP` is :2817. `TimedResult` is :2805-2813 (newly cited).

S/check_run.py
- PATH_KEYS 41 -> 50. `record_paths` 105-127 -> 141-163. Its call: 530 -> 1070.

Stage 13 README
- Height 4 not run with D19: 245-251 -> 835-842.

K files
- `r22d25_state.py` imports: 21-27 -> 20-30.
- `r22d19_rule.py` imports: 52-56 -> 51-58.

## 3. Interface conflicts (BASE governs)

1. `config.kernel.d19`. The draft said the runner does not write it. BASE writes `'d19': None`
   (`runner.py:444-445`); README section 2 says it is null in every record. The packet now says so and leaves
   filling it to I1. P2 pins D19 through `SCRIPTS` (into `config.scripts`) and `kernel.d19_sha256`.
2. Kernel timing. The draft quoted 107 s as a private figure for I0 to re-measure. README :779-783 records the I0
   measurement: `--seconds 1` at R22D-36, 18.6 s, result `budget`, no step. The packet quotes that, labelled
   measured, with its conditions.
3. `Ctx.budgets` also holds `lattice_restorer` (`runner.py:593-594`). The packet lists it.
4. Section 3 said I0 was incomplete. At BASE it is finished and reviewed. Rewritten.
5. The stub label. `component` keeps only the first reason, and r22d75's entry test equals high's. So with both
   stubs the label is `height_stub`, not `high_stub`. The packet's tests no longer expect `high_stub` there.
6. The capture-timeout limit. The draft said capturing calls fail from 10^7 s. README :280-283 records the measured
   limit: 2^31 - 1 ms (2,147,483.6 s passed, 2,147,483.7 s failed). Replaced.
7. `execute` returns `cancelled` without starting the child when a signal is pending (`runner.py:473-474`).
   Added to the cancellation row.
8. Test B9. A replay digest mismatch raises `ValueError` in `record_step` (`apply_word`, `runner.py:258-267`). So
   `phases.high.result` stays null and `record.error` names the mismatch. It is not `End('error')`. The test now
   says this and cites `check_run.py:617-629` (the null branch, accepted only in an error run).
9. `runner.SCRIPTS` also lists `restore.py` and `trees.py`. The packet's list of modules already covered is updated.
10. The worktree check. `test_runner.py`'s `status()` now uses `--untracked-files=all` and excludes the private
    trees. The packet's worktree check copies that.
11. The fakes. The draft cited a smaller `FakeState`/`FakeMechanics` (206-245). BASE's fakes (527-599) already
    model heights (letter items 2, 4, 6), landing (item 1) and the end-digest check in `apply_word`. The packet
    describes them. P2's test still defines its own fakes, modelled on these, and does not import `test_runner.py`
    (it belongs to I0).

## 4. Clarifications added

- How a plugin ends the run `error`. It cannot import `runner.py` or raise `RunError`. It returns
  `End('error', reason)`. The runner stores result `error`, reason, details and attempts in `phases.high`, saves,
  and raises `RuntimeError('high phase error: <reason>')` (`runner.py:644-656`). `run()` then records result
  `error` with `error = safe_reason(exc)` (`runner.py:1390-1397`). Sources: README :94 and :101-129.
- An invariant in section 6: every kernel crash, missing or inconsistent record and digest mismatch is
  `End('error', reason)`. Never `stuck`, `budget` or a failed attempt, and never a raise.
- A mapping row: a nonzero exit while the record still holds the start placeholder gives `kernel_exit`, never
  `budget`. Test B8 gains a sub-case for it.
- The kernel's own error reasons: `d19_exit` (:351-352), `d19_rejected` (:376-378), `d19_invalid_output`
  (:381-382), `accepted_replay_failed` (:600-602).
- Section 5 gains the I0 review history as it concerns P2 (I0-01, V1-B2, I0-V1-01, I0-03; I0-02 and I0-04 do not
  apply).
- Attempt row 3 names commit `97815b1` and its measured results: `test_runner.py` 75 tests, 1 skipped, 1,140 s;
  `test_check_run.py` 97 tests, 818 s.
- P4's hook signatures `prepare`, `on_letter`, `finish` (`check_contracts.py:21-30`, called from
  `check_run.py:1197-1231`).
- Test B3 reads the tree with `runner_api.read_tree`.
- Test E falls back to END36, built on its own, when test D's step left no piece at height 4 or 6.
- Every time is labelled measured (with source and date) or estimate. The R22D-36 and END36 kernel calls of this
  packet are marked not measured.
- The interface note on `check_run.py` PATH_KEYS `unit` (see decision 1).

## 5. Integrator decisions and open points

1. PATH_KEYS `unit` against an integer `params.unit` (I1). `check_run.py:50` lists `unit`; `record_paths`
   (:141-163, applied at :1070) treats every non-null `unit` as a path. Kernel d19 and F3 rows hold an integer
   `unit` in `params`, which P2 must copy verbatim. Such a record fails the checker with `path`. Recommendation: in
   I1, limit the `unit` path check to the tail and fallback rows and `stuck.probe`. Neither P2 nor P4 can fix it:
   P4's allowed files exclude `check_run.py`. P2's own tests do not hit it (test D is F1, no `unit`).
2. `config.kernel.d19` is null at BASE. Recommendation: I1 fills it from `rules_kernel.D19`. Until then
   `config.scripts` and `kernel.d19_sha256` pin D19.
3. A cancelled call gives `End('error', 'cancelled')`, so `record.error` reads `high phase error: cancelled`, not
   `signal N`. Recommendation: keep this in P2. Optionally, I1 adds `check_signal()` in `plugin_phase` after
   `next_step` returns, so a signal still ends the run as cancelled.
4. Confirm the packet's "decided here" choices: `End('error')` rather than a raise; `stop_family`; the
   `kernel_record_invalid` shapes and checks; the `kernel` field and attempt row, which avoid PATH_KEYS; the extra
   `SCRIPTS` modules; END36 built on its own; the `KERNEL` and `D19` constants. Recommendation: accept. Each one
   follows BASE's plugin contract and the plan.
5. Deferred, not P2's: the checker does not yet ask for evidence behind a claimed `budget` or `timeout` (README
   section 7). P2 records the timed-out attempt for each.
6. The end-to-end checker run of a real kernel record stays with I1 (plan R3-04).

No owner-level question was found.

## 6. Acceptance check

- `test_rules_kernel.py` does not exist at BASE. P2 creates it, so the check cannot run until P2 lands.
- Everything it relies on exists at BASE: the K files, the six extra `SCRIPTS` modules, `seams.py`, the runner and
  `runner_api.py`, `codex_review.py` (ACCEPTANCE_TIMEOUT 1,800 s at :105), the contract format in
  `templates/problem-packet.md`, and commits `31b78a5` and `e37f96b`.
- Environment: Python 3.13, NumPy 2.5, four cores.
- The implement-contract block (allowed files, acceptance check, stop condition) is unchanged.

## 7. Scans

- No private paths and no model identifiers in the packet. The role names stay as they were.
- The seven-part structure is kept.

## 8. Adversarial verification findings (all four adopted, re-checked against `97815b1`)

- P2-V-01 (adopted): section 4.4's `kernel_clock` row now requires a non-null `stop_family`; a `budget` record with no step and no qualifying cursor (the start placeholder after exit 0 included) is listed under "any other shape" and gives `kernel_record_invalid`. Evidence: every `BudgetStop` sets a qualifying cursor (`K/r22d25_search.py:489-491`; cursors at :128-163, :197-299, :323-357, :419-431); the start placeholder (:541-543, saved at :551) is replaced by `search_step`'s ledger (:564) before any exit-0 return. Test B5 gains the sub-case.
- P2-V-02 (adopted): the B setup now says `FakeState.digest()` returns `runner.SOLVED` when no piece is off (`S/test_runner.py:536-538`), since `lattice()` reports `solved` only on that digest (`S/runner.py:1255-1259`), and that the fake lattice plugin's `play()` raises if called (otherwise a stub End would run `restore_lattice_late` on a fake table, :1274-1296).
- P2-V-03 (adopted): new test B14 for `child.cancelled`: an execute wrapper writes a well-formed `stuck` record into the call's `--out` and returns a cancelled `Child` (as `S/runner.py:473-474` does); expected `End('error', 'cancelled')`, one attempt with `timed_out: false`, `kernel_seconds` and `ledger_status` null (record not read).
- P2-V-04 (adopted): two record checks added to section 4.4: `limits` equals `DEFAULT_LIMITS` (`K/r22d25_search.py:34`; the CLI always uses `Limits` defaults, :38-45, :637-638; stage 13 README:919, R22D25-contract-06), and the ledger has a row for each of the seven `FAMILIES` (:36; no providers from the CLI, :638), so a partial ledger cannot pass "every row exhausted" vacuously. Phrased as "a row for each of" so that B4's provider-cursor case (an extra row) stays valid. B6 gains a stuck-with-missing-`ST`-row sub-case; B8 adds `limits` below the defaults.

## Integrator decisions (20 October 2026)

- D3 (from P2-V-04; record only): option 1. The ledger check requires a row for each of the seven `FAMILIES`
  (`F1, F2, F3, d19, S, ST, T1`) and accepts extra rows. Test B4 keeps its provider case (an extra provider row
  holding the clock cursor). No text of `P2.md` changes: section 4.4 already says "the ledger has a row for each of
  `F1, F2, F3, d19, S, ST, T1`", which accepts extra rows, and B4 already lists the provider cursor. Re-checked at
  `97815b1`: `ledger()` builds the `FAMILIES` rows first and then one row per provider (`K/r22d25_search.py:82-89`),
  `run()` takes `providers=()` (:531), and the CLI passes none (:638). So a record from the CLI never has an extra
  row, and an extra row in a test does not fail the check.

- Integrator, after the decision check: small wording and test fixes from the last verification (no change of behaviour beyond the decisions above).
