# P3 refresh: change log

Packet P3 (R-L: the literal lattice phase, runner side). The draft was checked against base `97815b1`
(`97815b19cd3243d30ed7c1632bc95b6e1751541e`). The refreshed packet is `v2/P3.md` next to this file.

Abbreviations, as in the packet: S = `research/jumbling/restoration/13-tutorial-runner/scripts`;
G9 = `research/jumbling/restoration/09-general-method`; LAT = `G9/scripts/lattice/scripts` (the late restorer,
`runner.py:41`); SCH = `G9/scripts/schedule11`.

## 1. Base filled

- The placeholder is now `97815b1`. The full sha appears once, in section 3.
- The sentence "if `97815b1` moved a line, the named symbol governs" is kept (section 3).
- Section 3 now says that `97815b1` is I0 with the fixes of its review (I0-01 to I0-04, V1-B2) and of its scoped
  verification (I0-V1-01). The second verification round passed (stage 13 README :925-955).
- No private path, model identifier or scratch location is in the packet text. Paths are repository-relative.

## 2. Citations

283 citations were checked: 84 named `path:line` citations and 199 bare `:NNN` ones.

Correct at BASE, unchanged:
- `S/literal_lattice.py` and `S/test_literal_lattice.py` are unchanged since `31b78a5` and `86659a5`. Every citation
  was correct (for example `Program` :115-200, the simulator :203-212, `LatticePlay` :224-336, `play` :350-559,
  `main` :562-574; `setUpModule` :78-85, `ReportChecks` :88-164, `PlayerTests.setUpClass` :275-338,
  `build_fixture` :340-359, the word tamper test :432-453, the budget test :492-521, `corrections_first` :585-607,
  `InvisibleRotationTests` :637-702).
- `LAT/restore_lattice_late.py`, `SCH/restore_schedule11_late.py` and `SCH/sched11.py:13` (`BOUND`).
- `sim/twists.py` :348-357, :474-478, :488-489, :507-509; `sim/kplus.py` :69, :115; `theory/restore.py` :61-64,
  :67-69; `theory/buffer.py` :150-160, :829-834; `theory/seams.py:66`.
- `S/check_run.py` :50, :57, :78, :123-132, :135-138, :141-163, :411-412, :431-437, :465-557, :503-505.
- `S/check_lattice.py` (docstring :1-22, interface :9-18, `read_ia16` :48-62, `replay_lattice_step` :65-87).
- `tools/agents/codex_review.py:105` (`ACCEPTANCE_TIMEOUT = 1800`).
- The owner decision page `docs/wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md` :54-57, :68, :77-80,
  :89-96.

Re-anchored, by symbol (old -> new):

S/runner.py
- ROOT 31 -> 32. `sys.path` insert 33 -> 34. LAT 40 -> 41. LAT, CHECKER, LIBRARY 40-42 -> 41-43. SOLVED 43 -> 44.
  CLOCKS 48-49 -> 49-50. PLUGINS 50 -> 51.
- Core row keys: `record_step` 646-647 -> the constant `ROW_KEYS` 66-67.
- `Mechanics` 175-235 -> 212-272. Inside it: imports 178-186 -> 214-226; `load` 195-198 -> 232-235; `export`
  200-201 -> 237-238; `heights` 203-212 -> 240-249; `snapshot` 214-216 -> 251-253; `apply_word` 221-230 -> 258-267;
  `inverse_word` 232-235 -> 269-272.
- `Child` 264-271 -> 301-308. `Runner` 310-325 -> 352-367; its directories 318-319 -> 360-361.
- `plugin_modules` 343-347 -> 385-389. `child_env` 357-360 -> 399-402.
- `initialize`: plugins 392-393 -> 434-435; `config.lattice` 404 -> 446; scripts hash 407-408 -> 449-450.
- `execute` 424-464 -> 466-506; its log 429 -> 471. `script` (`--_script`) 466-471 -> 508-513.
- Bulk, no partial step: 517 -> 565-566.
- `context` 544-551 -> 592-599; budgets 545-546 -> 593-594.
- `record_step` 643-674 -> 691-720.
- `lattice` 1188-1241 -> 1255-1317. Inside it:
  - SOLVED return 1189-1192 -> 1256-1259;
  - `--no-lattice` return 1193-1196 -> 1260-1263;
  - the stub requirement 1197-1199 -> 1271-1275 (with the new `LatticeResult` dispatch at 1266-1268);
  - `lattice_composition` 1201 -> 1277;
  - library decompress 1205-1206 -> 1281-1282;
  - restorer and verify 1214-1220 -> 1290-1296.
- `played_lattice` 1319-1372 (new at BASE, cited for the first time): zero-step refusal 1326-1328, checks 1329-1349,
  file moves and rows 1350-1366, result 1367-1372.
- `run`: off = 0 entry 1256-1257 -> 1387-1388; the exception handler is 1390-1397 (new citation).
- `parse_args` 1361-1408 -> 1492-1539; source required 1363-1365 -> 1494-1496; `--no-lattice` 1380 -> 1511;
  `--lattice-restorer` 1383 -> 1514.
- `main` `--_script` 1413-1420 -> 1544-1551.
- `RunError` 317-319 (new citation).

S/runner_api.py
- FORMAT 17 -> 20. PHASE_ORDER 18 -> 21. RESULTS 20 -> 23. STANDINGS 21 -> 24. MODES 22 -> 25. CLOCK_SENTINEL
  25 -> 28. TREE_MAX_DEPTH 26 -> 29. FORBIDDEN_KEYS 27 -> 30. GRIPS and ITEMS 28 -> 31.
- `End` 46-55 -> 49-58. `Ctx` 58-80 -> 90-112. `Ctx.wall` 75-77 -> 107-109. `Ctx.child_clock` 79-80 -> 111-112.
  `child_clock` 83-89 -> 115-121.
- `LatticeStep` 61-79 and `LatticeResult` 82-87 (new at BASE).
- `span` 101-104 -> 133-136. `group` 107-110 -> 139-142. `flat` 113-126 -> 145-158. `single` 129-130 -> 161-162.
  `walk` 133-140 -> 165-172. `tree_size` 143-144 -> 175-176.
- `check_tree` 147-175 -> 179-207; its kind check 154-156 -> 186-188; its reference check 165-171 -> 197-203.
- `write_tree` 178-182 -> 210-214. `read_tree` 217-223 (new at BASE).
- The ia16 block 185-224 -> 226-290. `ia16_code` 187-190 -> 228-231. `Ia16Writer` 193-224 -> 234-265;
  `write_codes` 204-213 -> 245-254; `write_letters` 215-216 -> 256-257; `close` 218-224 -> 259-265.
  `ia16_digest` 268-290 (new at BASE).
- `atomic_bytes` 227-234 -> 293-300.

S/check_run.py
- Tree checks 537-551 -> 537-557.
- Lattice word not loaded 1052-1058 -> 1081-1087, and its replay through `check_lattice` -> 1186-1195.
- `check_tree` call 1160 -> 1094.
- Played lattice end digest 1265-1276 -> 1293-1305; `record.lattice` must be null at 1299-1300 (new citation).
- `solved_digest` 1283-1284 -> 1312-1313.
- The "played" flag 706 -> 724.

S/test_runner.py
- `plugins()` 606-610 -> 648-652.
- `test_lattice_pass_requires_exactly_the_right_run` 791 -> 868-906.
- `test_lattice_uses_relative_paths_without_chain_after_tail` 831 -> 908-929.
- `ExactTests.test_02_small_lattice_configuration` 2087 -> 2484-2546; its library without a menu 2117-2120 ->
  2502-2505; its `LIBRARY` patch -> 2509.
- `ExactTests.test_04_slow_scramble_20` opt-in 2365 -> 2798-2802.
- Worktree check model 2382-2427 -> 2816-2861; `SKIP` 2383 -> 2817.
- New citations: `LatticeTests.test_lattice_ends_are_errors_until_p3`, last part, 1414-1423; the `LatticePlay`
  scripted model 655-687; `TemporaryTest.args` with `--no-lattice` 511-514.

Stage 13 README
- The literal lattice rule 71-77 -> 183-192; 71-73 -> 183-188 and 356-357 ("only lattice programs have none").
- The player section 154-198 -> 572-616; the junction 173-175 -> 590-593; the final digest 176-180 -> 594-598 and
  179-180 -> 596-598; the speeds 191-198 -> 609-616.
- `test_literal_lattice.py` timing 217-224 -> 784-791; 224 -> 791.
- New citations: clocks 253-283 (:277, no deadline for `literal_lattice`), I0 review 925-955, deferred work
  957-967, `test_runner.py` timing :751, R3-05 note 819-822.

## 3. Interface conflicts (BASE governs)

1. `LatticeStep` and `LatticeResult`. The draft said `runner_api.py` has no `LatticeResult` and told P3 to define
   both in `lattice_phase.py`. BASE defines both in `runner_api.py:61-87`, with defaults `contract
   'lattice_literal'`, `gate 'player'`, `standing 'rule'`, `encoding 'ia16'`. The packet now says to use them and
   never define its own.
2. The runner side of the played lattice. The draft said that I1 switches `runner.lattice()` to the plugin and that
   until then the runner raises for a real plugin (`runner.py:1197-1199`). At BASE, `lattice()` sends a
   `LatticeResult` to `played_lattice` (:1266-1268), which checks and records it (I0-02, :1319-1372). So P3 switches
   the runner by itself. Sections 1, 2 and 4.7 now say so.
3. Zero-step result. The draft returned `LatticeResult([], end_table, report)` when the cascade has no step. BASE
   refuses a result with no step (`runner.py:1326-1328`), so such a run ends `error`. The packet keeps the zero-step
   return, says what the runner does with it, and leaves its recording to I1.
4. How an `End` is handled. The draft implied that `stuck`, `budget` and `timeout` are recorded. At BASE every `End`
   except the stub's `not_applicable` raises "lattice End <result> is integrated after packet P3"
   (`runner.py:1271-1275`); the run ends `error` and the reason, details and attempts are not recorded. Since I0-01,
   crashes, malformed output and digest mismatches are run errors. `RunError` lives in `runner.py:317-319` and the
   plugin does not import it. The packet now tells the plugin to return `End('error', <reason>, details)` for every
   integrity failure, never `stuck` and never an attempt, and not to raise for an expected failure. The result
   table is kept as the interface I1 records.
5. What the runner checks. The draft's step 7 self-check did not match `played_lattice`. BASE checks each step's
   contract, gate, standing and encoding, `rule` starting with `lattice/`, `before` equal to the running snapshot,
   `after` as a lattice snapshot, `fields` without `ROW_KEYS` or `FORBIDDEN_KEYS`, finite non-negative `seconds`,
   `ia16_digest(word_file)`, `read_tree` and `check_tree`, `labels_identity` true on the last step, and the end
   table's digest and loaded snapshot (:1329-1349). Step 7 now runs the same checks with `api.ia16_digest` and
   `api.read_tree`.
6. Core row keys. The draft cited `record_step` (:646-647). BASE has the constant `ROW_KEYS` (`runner.py:66-67`),
   and `played_lattice` refuses fields that use one. Section 4.6 now cites it.
7. The report. The draft said the runner records `LatticeResult.report`. At BASE it is not recorded, and
   `record.lattice` must be null for a played lattice (`check_run.py:1299-1300`). The packet says to write the report
   anyway; where it goes is I1's.
8. Child start under `-I`. The draft had the child import `runner_api` directly. Under `python3 -I` the script
   directory is not on `sys.path`, so the module now inserts its own directory before `import runner_api`, as
   `runner.py:34` does.
9. Forbidden field keys. The draft listed core row keys, `FORBIDDEN_KEYS` and the path keys. `check_run.py` also
   rejects the closure keys (`parts`, `V_source`, `V_letters`, `V_sha256`, `construction`, `certificate_sha256`,
   :334-338 and :423-425) on lattice rows. Section 4.6 adds them.
10. Tests that break. The draft said every `test_runner.py` test that reaches the lattice phase fails "until I1's
    switch". At BASE the switch exists, and these tests break because they expect the stub's composition path:
    `RecordTests` :868-906 and :908-929, the last part of `LatticeTests.test_lattice_ends_are_errors_until_p3`
    (:1414-1423), `ExactTests.test_02` (:2484-2546) and the opt-in `ExactTests.test_04` (:2798-2802). Section 4.7
    lists them.

## 4. Integrator decisions and open points

None of these is owner-level. Each has a recommendation grounded in BASE, the plan or an owner rule.

1. Zero-step lattice phase (section 4.5 step 7, 4.7). The runner refuses it at BASE. Options: keep the plugin's
   zero-step `LatticeResult` and let I1 record it; or have P3 return an `End`. Recommendation: keep the zero-step
   result. I1 records the phase with no row and a checker-recomputed labels claim. A step with no twist must not be
   invented, and "every step records a tree" forbids an empty step.
2. Recording a lattice `stuck`, `budget` or `timeout`, and the checker's rules for it. Deferred to I1 by decision
   (README :957-960). Recommendation: P3 keeps the result table as the interface. A failure certificate stays
   `stuck`, which is the owner rule "certificate step = stuck" read for this phase.
3. Where `LatticeResult.report` goes. Not in `record.lattice`, which must stay null (`check_run.py:1299-1300`).
   Recommendation: I1 records it under `phases.lattice` or as a run file; P3 writes it unchanged.
4. P3 alone breaks five `test_runner.py` tests (conflict 10). Options: integrate P3 with the I1 test edits in one
   candidate; or keep a stub switch. Recommendation: do not commit P3 alone. Apply its patch together with I1's
   edits to `test_runner.py` and run the full runner checks on that candidate. Out of P3's allowed files.
5. Test D "the runner takes it" (new in this refresh). It builds a runner context and calls `played_lattice` on the
   corrected variant, with no extra child and no checker. Recommendation: keep it. It proves the I0-02 interface at
   no added cost.
6. Cancellation. A cancelled child gives `End('error', 'cancelled')`, and at BASE `lattice()` does not check the
   signal before reading the `End`, so the run records the generic message instead of `signal N`. Recommendation:
   I1 adds `check_signal()` after `play` returns. P3 keeps the table row.
7. Choices the draft marked "decided here" (observer protocol, step and part names, ia16-only reading, the `End`
   attempt row, test H with its 300 s threshold). Recommendation: keep them. They agree with BASE: the scripted
   `LatticePlay` model and `LatticeTests` use the rule names `lattice/orbit-3` and `lattice/correction-0-0`.

## 5. Acceptance check and fixtures

- The acceptance command is `python3 -I -B research/jumbling/restoration/13-tutorial-runner/scripts/test_lattice_phase.py`.
  The test file does not exist at BASE: the S tree has 14 files without it. P3 creates it, which is intended. The
  command form is valid for the wrapper; its timeout is 1,800 s (`codex_review.py:105`).
- The allowed files are unchanged: `lattice_phase.py`, `test_lattice_phase.py`, `literal_lattice.py`,
  `test_literal_lattice.py` in S. `lattice_phase.py` is the I0 stub at BASE (`STUB = True` :11; it returns
  `End('not_applicable', 'stub')` at :15-16).
- Fixtures at BASE: `assets/execution_trees.json.gz`; G9 `library-merged.json.gz` and `realign.json.gz`;
  `runs/depth20-seed7106/lattice/` with `restore-summary.json`, `solutions.json.gz` and `report.json`;
  LAT `restore_lattice_late.py`, `rlib.py`, `kit.py`, `nucoords.py`; SCH `restore_schedule11_late.py` and
  `sched11.py`; the stage 03 scripts `rlib.py`, `kit.py`, `nucoords.py`; `test_literal_lattice.py`; the owner
  decision page. All exist.
- The depth-20 summary at BASE: input digest `1b263d3e…`, 35 orbits, 10,731 factors, 8,701,310 twists, no
  corrections. The library has 36 macros, menu `I_a`, `mirror1` and `mirror2` duplicated, no `/`, `\` or `~` in a
  name. The packet quotes these as recorded values.

## 6. Times

- Measured values are labelled "measured" with their source: `test_literal_lattice.py` 468 s and 92,000 twists/s
  (README :784-791); the late restorer 1,525 to 1,578 s and 25,250 to 28,900 twists/s; `test_runner.py` 75 tests,
  1 skipped, 1,140 s (README :751).
- Estimates are labelled "estimate": the observer play about 3 times slower; 35 to 60 ms per twist for the child's
  start.

## 7. Other edits

- Sections 1 and 2: the I0 description and the out-of-scope list are rewritten for BASE.
- Section 5: attempt 3 now names `played_lattice` and `RunError`, and adds the I0 review history.
- Section 6: the invariants add the `sys.path` insert and "no own `LatticeStep` or `LatticeResult`"; test A
  checks the classes come from `runner_api`; test E grounds ia16-only reading in `runner.py:1331` and
  `check_run.py:411-412`; test G checks a boolean `timed_out` on every attempt.
- Section 7: the open points list the zero-step recording as the I0 interface change P3 needs.
- The seven-part structure, the `implement-contract` block, the acceptance check and the stop condition are kept.

## 8. Adversarial verification fixes (P3v2-01 to P3v2-07), checked at `97815b1`

- P3v2-01 (minor, adopted): `S/runner.py:1331-1335` also checks the contract defaults, the `lattice/` rule prefix, `ROW_KEYS`/`FORBIDDEN_KEYS` in `fields` and `type(seconds) in (int, float)` (exact type), and :1347-1349 loads the end table through `Mechanics.load` and compares the snapshot. Section 4.5 step 7 now lists the prefix and defaults, the exact type of `seconds` and the key exclusions, and says that the one check it leaves out is the end-table round trip, which the runner makes and test D's `runner.played_lattice` call covers. "every check" is kept with the stated exception.
- P3v2-02 (minor, adopted): `S/runner.py:471` and :488 open `<out>/logs/<tag>.log` with `'w'`; :595 gives each lattice call `attempts/<NNNNN>-lattice`. Test I now asserts the exact tag `'lattice-restore-' + ctx.work_dir.name`, one `work_dir` per restorer call; the play tag `'lattice-play-' + ctx.work_dir.name` is asserted by the test G wrapper that answers it (with exit 1 from the restorer the play never runs in test I).
- P3v2-03 (minor, adopted): the result table mapped only "no manifest". New row: a present but invalid manifest (bad JSON, wrong `format`, missing or ill-typed key, a file name outside `play/`) is `End('error', 'play_output_invalid')` with `details.check` = `manifest`; step 7 gains the manifest check as its first item; test G gains the two cases (bad JSON, changed `format`), reusing the copied real play output.
- P3v2-04 (minor, adopted): `S/runner_api.py:197-204` registers every part (spans with `inverse_of` included) and accepts `same_as` to any earlier part of equal length and name, so both readings pass. Section 4.6 now states the reading that "whatever its kind" already implies: the trailing span of rule 5 is the first part of (u, us) when (u, us) was not written before, and later occurrences name it with `same_as`; when (u, us) was written before, the trailing span keeps `inverse_of`. Test C gains that case.
- P3v2-05 (minor, adopted): `S/runner.py:425-426` and :571 write `tables/bulk-end.json` (start and bulk end); :711 and :1160 name later tables on height, high and tail rows; :1260-1263 returns under `--no-lattice` with `self.table` as the handoff. Bulk rows name no table (:1480-1483). Test K now takes the table named by the last row that names one, else `tables/bulk-end.json`, asserts `off` 0 before calling the plugin, and reports the rebuilt handoff digest beside `1b263d3e…`. The committed run (`runs/depth20-seed7106/run.json`, format 1) reached off 0 in the bulk with 0 tail steps.
- P3v2-06 (minor, adopted in part): checked with a two-module unittest probe (scratch, outside the repository): `loadTestsFromModule` collects a `TestCase` class imported by name, and `loadTestsFromName` on the other module runs that module's `setUpModule`. Test J now imports `test_literal_lattice` as a module, builds its suite with `loadTestsFromName` for the named tests only, binds no `TestCase` class into the namespace and asserts that no `PlayerTests` test is in the suite (a structural assertion instead of the suggested duration check). Not adopted: the claim that the module fixtures run only through a loader; `TestSuite.run` runs them for any test of that module, which the packet's preamble already says.
- P3v2-07 (minor, adopted in part): section 4.2 already has the player hand codes to the observer (`leaf(node, sign, codes)`, `correction_twist(code)`), but did not name one source or define "the menu's inverse table". `TwistMenu` has no inverse attribute (`research/jumbling/sim/twists.py:433-519`); the same list exists as `Mechanics.inverse_word`'s lookup (`S/runner.py:269-272`) and `check_run.inverse_items` (`S/check_run.py:388-391`). Section 4.4 now says: `literal_lattice.py` computes the codes from its own menu, only on the observer path; the observer, plugin and child map no twist themselves; "the menu's inverse table" is the item → inverse item list of section 4.4. Not adopted: "the plugin and the tests use only that function". Tests B2, B5, C and D keep their own tables from `restore.element` on purpose (B2 says "with its own primitive-to-letter table"), so that they check the player's codes; for exact matrices the tables cannot disagree (fᵀ·gᵀ·f is the transpose of fᵀ·g·f).

## Integrator decisions (20 October 2026)

- D4 (end-table round trip; options (a) and (c)): the plugin leaves the `Mechanics.load` round trip of the end table
  to the runner (`S/runner.py:1347-1349`), as section 4.5 step 7 already said; that text now names the
  `runner.played_lattice` calls of tests D and F as the cover, F with an end digest that is not `SOLVED`. Test F now
  builds its `Ctx` with `runner.context('lattice', None)` of a `Runner` on the w9 start table (shared `Mechanics`,
  `initialize()` run), checks the result as before, then calls `runner.played_lattice(result)`, which must not raise:
  one `ia16` row, result and `phases.lattice.result` `solved`, `totals.lattice_twists` 9, `record.lattice` null, no
  component, and `runner.state.digest()` equal to the `after.digest`, not `SOLVED`. If it raises, the implementer
  stops and reports (section 4.8). Checked at `97815b1`: `initialize()` loads the `--table` with `orient.set_pose_table`
  and exports `tables/bulk-end.json` with `buffer.export_pose_table`, which writes the `digest` field
  (`research/jumbling/theory/buffer.py:829-834`), so the plugin's entry check holds; `played_lattice` does not
  require `SOLVED` (:1343-1372). The Time note now says the runner checks of D and F share one `Mechanics` and add no
  child.
  - Before (test F): "**F. The w9 fixture through the plugin** (R3-05): one step `lattice/correction-0-0`; ... the
    end table has that digest."
  - After: "**F. The w9 fixture through the plugin and the runner** (R3-05): build the `Ctx` as test D's runner check
    does ... Then call `runner.played_lattice(result)`. It must not raise ..."
- D5 (P3v2-04): kept as written. Section 4.6 registers the `conj`'s trailing `inverse_of` span as the first part of
  (u, us) when (u, us) was not written before, and later occurrences of (u, us) name it with `same_as`; test C keeps
  that case. Record only; no packet edit.
- D8 (cross finding X-01): the player's own clock (report `budget`, only with `PLAY_SECONDS` set) is now
  `End('budget', 'player_clock')` with one attempt `timed_out: true`, as P1 (`budget`, `cpu_budget`/`cpu_rlimit`) and
  P2 (`End('budget', 'kernel_clock')`) map a child's own clock. The runner's kill of the restorer or player child
  stays `End('timeout', 'lattice_wall_clock')`. Edited: a sentence before the section 4.5 table, the table row, test G
  (`PLAY_SECONDS = 0`, and the play-tag `Child.timeout` case now written as the full `End`), test H.
  - Before (table row): "`End('timeout', 'player_clock')`, one attempt `timed_out: true`"; after:
    "`End('budget', 'player_clock')`, one attempt `timed_out: true`".
  - Checked at `97815b1`: `lattice()` sends a `LatticeResult` to `played_lattice` (:1266-1268) and raises
    `ValueError('lattice End <result> is integrated after packet P3')` for every other `End` but the stub's
    `not_applicable` (:1274-1275); `run()` records result `error` with that message (:1390-1397). `phases.lattice` is
    written only at :1316 and :1368, so it keeps its initial null result, and the `End`'s reason, details and attempts
    are not recorded. The section 4.5 note now says this, that the runner side of a lattice `budget`, `timeout` or
    `stuck` comes with I1 in `runner.py`, outside P3's allowed files, and that P3's tests assert the value `play(ctx)`
    returns and pass no `End` to the runner. Tests G and H say the same. The out-of-scope list (section 1) and
    section 4.7 already defer the recording to I1; unchanged.

- Integrator, after the decision check: small wording and test fixes from the last verification (no change of behaviour beyond the decisions above).
