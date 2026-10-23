# P4 refresh: change log

Draft: `i0/packets/P4.md`. Refreshed: `i0/packets/v2/P4.md`. Base: `97815b1`
(`97815b19cd3243d30ed7c1632bc95b6e1751541e`), the I0 commit, clean tree. The repository was only read. One
measurement script ran outside the repository with `python3 -I -B` and its own cache; afterwards only the hooks'
private event file under `work/loop-memory/workbench/` was newer.

## 1. Base filled

- Every `BASE` placeholder now reads `97815b1`. The full sha appears once, in section 3.
- The draft had no "symbol governs" sentence. Added after the path abbreviations: "Line numbers were checked at
  commit `97815b1`. If a line has moved, the named symbol governs."
- New abbreviation `README` for `research/jumbling/restoration/13-tutorial-runner/README.md`.
- The hedge in section 4.1 ("the working tree that becomes BASE (I0, not yet committed) … if BASE differs, stop") is
  gone. The hooks are committed and match the packet.

## 2. Citations

The draft holds 268 line references (repeats included). All were checked at `97815b1`. Unchanged: the owner
decision page (:54-57, :68, :74-76, :77-80), `codex_review.py:105`, the stage 12 rule and kernel scripts
(`r22d25_state.py`, `r22d25_search.py`, `r22d81_rule.py`, `r22d81_pairs.py`), the stage 12 cloud scripts
(`h4_unlock.py`, `two_letter_u.py`, `r22d75_from_snapshot.py`), `eb6.py`, `test_r22d25.py`, `sim/kernel.py`,
`sim/regions.py`, `restore.py`, `rim.py`, `trees.py`, `seams.py:66`, `check_r22d25.py`, `check_contracts.py:18`,
`:29-30`, and in `check_run.py` the header, `SOLVED`, `PATH_KEYS`, `FORMATS`, `UNCHECKED`, `record_paths`,
`load_table`, `Snapshot`, `inverse_items`, the contract labels of `check_v3_step` and the matrix comparison in
`check_unit`. Everything at or after line 1000 of `check_run.py`, and everything cited in `runner.py`,
`test_check_run.py` and `test_runner.py`, had moved.

| Old | New | Symbol |
|---|---|---|
| `check_run.py:1168-1202` | `:1197-1231` | hook calls in `check_run` |
| `:1170` | `:1199` | `hooked = v3 and phase in ('height', 'high')` |
| `:1157-1166` | `:1186-1195` | lattice-row branch (`check_lattice.replay_lattice_step`) |
| `:1074-1083` | `:1101-1112` | early return before any replay |
| `:1172-1176` | `:1200-1205` | `prepare` call |
| `:1178-1187` | `:1207-1216` | letter loop |
| `:1180-1182` | `:1211` | `admitted = outcome.applied and not outcome.uncertain` |
| `:1185-1187` | `:1214-1216` | `admissible` break |
| `:1188-1193` | `:1217-1222` | `on_letter` call |
| `:1194-1202` | `:1223-1231` | `finish` call and type check |
| `:1174-1176`, `:1191-1193`, `:1197-1200` | `:1203-1205`, `:1220-1222`, `:1226-1229` | hook exception → `contract` |
| `:1143` (`before.n()`) | `:1171-1172` (bulk only); reason now `:191-192` | `Snapshot.__init__` copies; see conflict C3 |
| `:1004-1017`, `:1004-1010` | `:1033-1046`, `:1033-1039` | `step_row`, `merge` |
| `:1329-1331` | `:1358-1360` | `--allow-unchecked-contracts` |
| `:1024-1034` | `:1053-1063` | v3 detection in `check_run` |
| `:1041` | `:1070` | `record_paths(base, record, fails)` |
| `:141-151` | `:141-163` (key check `:146-151`) | `record_paths` |
| `:619` | `:633` (and `:624` in an error run) | `check_plugin_phase` start check |
| `:654-655` | `:668-669` | `starts` in `check_v3_phases` |
| `:1170-1202` (test helper model) | `:1199-1231` | hooks block |
| `check_contracts.py:1-15` | `:1-16` | stub docstring |
| `runner.py:558` | `:605` | `data['start'] = self.m.snapshot(self.state)` in `Runner.plugin_phase` |
| `runner.py:214-216` | `:251-253` | `Mechanics.snapshot` |
| `runner.py:203-212` | `:240-249` | `Mechanics.heights` |
| `runner.py:646-647`, `:667-672` | `ROW_KEYS :65-67`, row `:713-718` | `Runner.record_step` |
| `runner.py:648-651` | `:694-697` | proposal key check (also `FORBIDDEN_KEYS`, `runner_api.py:30`) |
| `runner.py:232-235` | `:269-272` | `Mechanics.inverse_word` |
| `test_check_run.py:1036-1066` | `:1034-1066` (rows `:1055-1057`, heights assert `:1058`) | `build_height3` |
| `test_check_run.py:1721-1772` | `:1834-1894` | `IndependenceTests` |
| `:1723` | `:1836` | `FILES` |
| `:1726`, `:1755` | `:1839`, `:1869` | `ALLOWED`, unlisted import |
| `:1728-1729` | `:1841-1842` (test `:1866`) | `FORBIDDEN` |
| `:1733` | `:1846` | `RESTORE` |
| `:1734`, `:1746-1748`, `:1757-1761` | `:1847`, `:1859-1861`, `:1870-1874` | `RIM`, attribute checks |
| `:1744-1745` | `:1857-1858` | relative import |
| `:1762-1763` | `:1875-1876` | dynamic code names |
| `:1765-1766` | `:1878-1879` | used as a value |
| `:1794-1795` | `:1907-1908` | `unittest.main` |
| `test_runner.py:597-603` | `:619-646` (contract at `:643`) | scripted `Plugin` |
| `test_runner.py:2318` | `:2703-2704` | `test_07` checker pass |
| `test_runner.py:2359` | `:2744-2745` | `test_08` `intact` case |
| `test_runner.py:2302-2303` | `:2687-2688` | run A prefix and heights |
| `test_runner.py:2382-2427` | `:2814-2861` | worktree check |
| `:2383` | `:2817` | `SKIP` |
| `:2386-2401` | `:2820-2835` | `inventory` |
| `:2404-2406` | `:2838-2840` | `status` |
| `:2409-2427` | `:2843-2861` | `main` |
| `sim/state.py:229-275` | `:228-278` | off-piece loop of `State.classify` |
| `sim/state.py:385-396` | `:385-397` | `State._compact` |

New citations added: `check_run.py:123-128` (`inside` rejects a non-string), `:399-437` (`check_v3_step`),
`:440-462` (`load_word_v3`), `:612-648`, `:617-629`, `:624-626` (`check_plugin_phase` and its null branch),
`:1080-1083`, `:1174`, `:1232`, `:1234`, `:1202`, `:1219`, `:1225`; `sim/state.py:229`, `:479`, `:397`; `eb6.py:125`;
`check_lattice.py:30-34`; `test_check_run.py:1040`, `:1338-1352`, `:1354-1386`, `:1388-1404`, `:1406-1415`,
`:1462-1490`, `:1548-1553`; `test_runner.py:2642-2672`; README:337, :536-539, :546-551, :553-563, :778-783, :783, :962.

## 3. Interface conflicts (draft vs `97815b1`)

- **C1. Which words reach the hooks.** Draft: hooks run for height and high rows "whose word is a `json` word".
  `97815b1`: they run for every height or high row, `json` or `ia16`; both go through `load_word_v3`
  (`check_run.py:440-462`), and only lattice rows have no word (`:1080-1083`). Change: section 4.1 says so.
- **C2. `check_high_phase` is not wired.** Draft: "the integrator wires the call (I0 before BASE, or I1)".
  `97815b1`: no call; `check_run.py` references only `prepare`, `on_letter` and `finish` (`:1202`, `:1219`,
  `:1225`). Change: section 4.1 states this plainly; the test's wiring note stays; integrator decision 1.
- **C3. Why `prepare` must run first.** Draft: "`before.n()` at :1143 is called before the replay for the same
  reason". `97815b1`: `before.n()` runs only for bulk rows (`:1171-1172`). For height and high rows the snapshot copies
  pose ids and matrices at creation (`:191-192`) and keeps the live state only until `before.st = None` (`:1232`).
  Change: section 4.1 gives that reason.
- **C4. A high phase can end null with steps.** Draft: the start is compared at `check_plugin_phase` `:619`.
  `97815b1` (review fix I0-V1-01): in an error run a phase may keep `result` null with a `start` and steps; then the
  start is checked at `:624` and the entry test at `:625-626` (`:617-629`). Change: section 4.1 says the letter bound
  applies to such a phase too, and `check_high_phase` does not read `result`; integrator decision 1.
- **C5. More tests at `97815b1` rely on the stub.** Draft listed `test_check_run.py:1338-1352` and the
  `test_runner.py` plugin with `:2318`, `:2359`. `97815b1`: also `test_v3_interrupted_phase_entry` (first record,
  `:1462-1490`) and `test_v3_verdict_earlier_timeout` (`:1548-1553`) expect the `build_height3` rows to pass with
  `allow_unchecked=True`. In `test_runner.py`, run A is an `r22d81/H6` step and run B a three-letter `r22d75/T2` step
  (`table_runs`, `:2642-2672`), so run B would also fail `height_length`. The tests that still pass are named.
  Change: section 1 lists them; integrator decision 2.
- **C6. Empty off set.** Draft: build the count from `st._groups(off)`. `97815b1`: `State._groups` raises
  `IndexError` on an empty array (`sim/state.py:166-171`); the simulator guards it (`:229`, `:479`) and
  `eb6.straddler_set` does not (`eb6.py:125`). This bites a height row whose end state has no off piece. Change:
  section 4.5 requires an early empty result; test 1 adds an S0 case without the oracle.
- **C7. Forbidden keys in fields.** Draft: proposal fields "may not reuse a core key". `97815b1`: they may also not use
  `FORBIDDEN_KEYS` (`runner.py:694-697`, `runner_api.py:30`). Change: section 4.2 says so. No effect on P4's fields.

No conflict comes from the other I0 review changes. `RunError`, the `ended` rule (`check_run.py:730-745`), the probe
(`:821-856`) and the certificate gate order are outside the hooks. P4 has no runner-side crash mapping.

## 4. Additions (not conflicts)

- Test 4 adds the row that the real `h4/L4` rule writes at A, from packet P1's measurement (not in the repository):
  tau_before 35, tau_after null, u_before 0, u_after 0. It asserts oracle u(A, 35) = 0 first. Since (35, 13) is
  admissible at A, no piece straddles cap 35, so the row must pass.
- Section 1 adds "a claimed `budget` or `timeout` and its evidence" to the out-of-scope list (deferred, README:962).
- Section 3 notes that `check_contracts.py` may set its own `sys.path` like `check_lattice.py:30-34`, but need not.
- Section 4.2 explains how an integer `unit` fails: `inside` rejects a non-string (`check_run.py:123-128`).
- Section 6.4 now names `test_check_run.py` and `test_runner.py` among the files that must not change, and says that
  `test_check_contracts.py` does not exist at `97815b1`.
- Attempt 10 in section 5 records this refresh.

## 5. Integrator decisions and open points

None is an owner-level question.

1. **Wire `check_high_phase` (I1).** Where: `check_v3_phases` (`check_run.py:651-761`). Options: (a) pass the
   checker's own `starts(4)` as `{'off': snap.count, 'heights': snap.heights()}` plus the high rows; (b) pass the
   record's `phases.high.start`; (c) leave it unwired. Recommendation: (a), whenever the high phase has steps, error
   runs included. Reason: the checker's own snapshot is the independent value, and `check_plugin_phase` already ties
   the record's start to it (`:633`, or `:624` in an error run, I0-V1-01). There is no call at `97815b1`.
2. **Update the stub-dependent tests with P4 (I1).** Where: `test_check_run.py` `build_height3` (`:1034-1066`) and the
   tests at `:1338-1352`, `:1462-1490`, `:1548-1553`; `test_runner.py` `Plugin` (`:643`) and `table_runs`
   (`:2642-2672`) with `test_07` and `test_08`. Options: update in the same integration commit as P4, or keep
   `check_contracts.py` as a stub until they are updated. Recommendation: update them together. Relabel the
   `height3` rows `r22d75/T0` then `r22d75/T1`, with `contract` = rule and null tau and u fields. Give the scripted
   `Plugin` `contract` = rule and P1's fields. Run B's three-letter `r22d75/T2` step cannot pass `height_length`:
   either keep it for the tree test and let `test_08` assert "no tree failure" instead of a pass, or split its word.
   Reason: under P4 these rows fail `height_rule` and `height_fields` (and `height_tier` or `height_length`), and the
   tests expect a pass.
3. **Integer `unit` in kernel `params` fails `path`.** Where: `PATH_KEYS` (`check_run.py:50`), `record_paths`
   (`:141-163`, applied at `:1070`), `inside` (`:123-128`), early return (`:1101-1112`). Options: limit the `unit`
   path check to the places the format defines a path; or have P2 rename the key. Recommendation: limit the check in
   `check_run.py` (I1). Reason: P2 copies the kernel's fields verbatim (`r22d25_search.py:379`, `:278`, `:298`), and
   otherwise a real d19 or F3 row never reaches P4's hooks.
4. **I1's "a mutated `high_exact` fails" at R22D-36.** Options: a non-bool `high_exact` (→ `kernel_fields`), or
   `params.high_exact_pieces` naming a piece that lands (→ `kernel_high_exact_pieces`). Recommendation: the second.
   Reason: at Z36 the only piece at height ≥ 4 is 58388 (`test_r22d25.py:286-287`), the target of every height-6
   step there, and the gate drops the target (`r22d25_state.py:305`), so `high_exact` True cannot fail.
5. **Confirm the "decided here" items.** Null tau, field types, one tau for the "=" rows, letters, fail closed,
   `kernel_fields`/`kernel_counts`, the failure names and the `check_high_phase` signature. Recommendation: keep them.
   Reason: each matches the rule code cited in sections 4.3 and 4.4 and the hook interface at `97815b1`.

## 6. Acceptance and fixtures

- The acceptance check file `S/test_check_contracts.py` does not exist at `97815b1`; it is the new file this packet
  asks for. So the check becomes runnable only after the implementer writes it. `python3` (3.13.16) and NumPy
  (2.5.3) are present.
- Named fixtures and sources exist at `97815b1`: `S/check_contracts.py` (stub), `S/check_run.py`, `S/test_runner.py`,
  `S/test_check_run.py`, `K/r22d25_state.py`, `K/r22d25_search.py`, `K/test_r22d25.py`, `eb6.py`, the cloud scripts
  in `C`, `sim/`, `theory/`, and
  `research/jumbling/restoration/11-scramble-census/runs/round2/ia-k100-s7102.json` (100 `history` and 272 `moves`).
- The `implement-contract` block is unchanged and parses as the wrapper expects (one block; keys `allowed_files`,
  `acceptance_check`, `stop_condition`).

## 7. Measurements

- I0 (README:778-783, 20 October 2026, headless Linux, four cores, two other single-threaded jobs):
  `eb6.straddler_set` on all 600 caps, 2.5 s at Z36 (6,489 straddlers) and 18.9 s at Z7102 (29,212); the kernel with
  `--seconds 1` at Z36, 18.6 s; `r22d75_from_snapshot.py` at Z36 to HEIGHT2 in two steps, 13.6 s.
- This refresh (20 October 2026, Python 3.13.16, NumPy 2.5.3, four cores, headless Linux, outside the repository with
  its own cache):
  - Z36: digest `8733a0aa…7261`, 5,800 off; the only piece at height ≥ 4 is 58388 (height 6), inside cap 12.
    Replay 0.77 s after the imports.
  - A = Z36·(12, 14): digest `7cd868f6466953eb6ea1e0d466ed66857d34edaea7dd7c554c1e74975005541b`, 3,098 off, max 4;
    the height-4 piece is 58388, inside cap 12; no piece straddles cap 12.
  - B = A·(35, 13): digest `3ab25d214b6c4ad759ac7158a0fc1cabae1584aa711d3f757af7b91c43971837`, 5,405 off, max 2.
  - The whole script took 4.8 s after the imports.
- The plan's 243 s for test 1 stays labelled as a private integrator measurement.

## 8. Adversarial verification fixes (P4-V-01 to P4-V-05), checked at `97815b1`

- P4-V-01 (minor, adopted): the rules take tau as a cap index (`K/r22d81_rule.py:255`, `C/h4_unlock.py:270`, `E.straddler_set(s, tau)`), so 0..599 is the domain, and test 5 expects `height_tau` for tau `True`, which 4.3 had made a type (`height_fields`) matter. 4.3 now says any invalid tau value (bool, other non-int, outside 0..599, null where an int is required, a differing `tau_after` on a "=" row) is `height_tau`, and `height_fields` covers the u fields, `u_mid` and a non-null tau on an r22d75 row; both names may be reported, and the tests assert membership there. The 6.2 failure-name list follows. The "exact `height_tau`" case (tau_after alone changed) is unaffected.
- P4-V-02 (minor, adopted): the kernel's run loop judges progress from its own refreshed state (`K/r22d25_search.py:586-589`). 4.4 now says `kernel_progress`, `kernel_target_height` and every other kernel check use the checker's own states and counts, and the row's `off_before`/`off_after`/`heights_after` are read only by `kernel_counts` (typed by `kernel_fields`). Test 10 now says "altered" means a wrong value of a valid type (an int + 1, or one histogram count + 1), so the exact `kernel_counts` assertion holds.
- P4-V-03 (minor, adopted): `restore.NEAR` is per cap (`J/theory/restore.py:57`, `np.nonzero(_ANG[d] < 55.7)[0]`, so each list is in increasing order and contains its own cap). The test preamble now says c is the cap of the row's letter, and for a two-letter row the search runs over `NEAR[c1] ∪ NEAR[c2]` in increasing order, with `u_mid` = oracle u(X·L1). Tests 7 and 8 now fix tau as the first cap in `NEAR` of the letter's cap (12 or 35) with no condition on u, and the u fields as oracle counts there. Measured for this fix (outside the repository, `python3 -I -B`, own cache; repository unchanged afterwards): clean adds Z36 (2, 12) 2,808 new, then (16, 12) 2,413; A (2, 12) 2,808, then (6, 12) 2,702; the union search gives tau 1 at Z36 (u 363 → 139, `u_mid` 284) and tau 0 at A (u 245 → 196, `u_mid` 196), the same as "NEAR[c1] then NEAR[c2]" and the verifier's numbers. Tests 7 and 8 at tau 0: Z36 u 332 → 396 for (12, r) with r = item 1; guard rows u 332/196/332 at Z36 and 196/307/196 at A. Only the pair-row numbers are added to the packet, as orientation, not asserted; the test 7 and 8 numbers are not.
- P4-V-04 (minor, adopted): the gate normalizes with `target = sorted(set(U))` and rejects a non-int (bools included, `type(p) is not int`) or out-of-range entry as `invalid_target` (`K/r22d25_state.py:282-284`); for `high_exact_pieces` it has no named reason (`keep[list(high_exact)]`, :306-307). Checked with NumPy 2.5.3: a string, float, bool or too large entry raises `IndexError`, and a negative int wraps silently. 4.4 now states P4 normalizes the target the same way (a non-list, empty or out-of-range target is `kernel_target`) and that a `params.high_exact_pieces` entry that is not an int in `0..len(st.pose_id) − 1` (not a bool) is `kernel_fields`, decided here. The table row cites `sorted(set(U))`.
- P4-V-05 (minor, adopted): K2 (test 3) now lists `rule 'r22d25/T1'`, `gate 'r22d25'` and `subphase None` like the F1 row of test 2, since `kernel_fields` requires `rule == 'r22d25/' + family` and `subphase` None.

## Integrator decisions (20 October 2026)

Applied to `v2/P4.md` with minimal edits; every other part of the packet is unchanged. Checked against `97815b1`.

### D6 (a): invalid tau, recorded only

- Kept as the packet states it (section 4.3, "Decided here, field types"; section 6.2): every invalid tau value (a
  bool, any other non-int, an int outside 0..599, null where the table requires an int, or a `tau_after` that differs
  where the table says "=") is `height_tau`, and a tau that is not null on an r22d75 row is `height_fields`. Test 5
  keeps its cases (tau 600 and tau `True` → `height_tau`; a tau on an r22d75 row → `height_fields`). No edit.

### D6 (b): negative `params.high_exact_pieces` entry, recorded only

- Kept the stricter rule (section 4.4, `kernel_fields`): a `params.high_exact_pieces` entry must be an int in
  `0..len(st.pose_id) − 1` and not a bool, so a negative entry fails `kernel_fields`, although the gate's
  `keep[list(high_exact)]` (`K/r22d25_state.py:306-307`) would wrap it silently. No edit.

### D6, cross finding X-03: source wording for tau_before and u_before, applied

- Section 4.2, the table of P1's sources. The r22d81 and h4 rows now take tau_before and u_before from the previous
  START or STEP line, and the pairs row its u_before, as packet P1 defines them (P1 section 6.3, "the previous line";
  P1 section 6.4 table). The tau_before of the pairs row (`--target`) and the CONTEXT values of `h4/pair` are
  unchanged.
  - Before (r22d81 row, the h4 row the same): "| `r22d81_rule.py` STEP | START tau | STEP tau | START u | STEP u |";
    after: "| `r22d81_rule.py` STEP | tau of the previous START or STEP line (START for the deciding STEP) | STEP
    tau | u of the previous START or STEP line (START for the deciding STEP) | STEP u |".
  - Before (pairs row): "| `--target` | `--target` | pairs START u | STEP u |"; after: "| `--target` | `--target` |
    u of the previous START or STEP line (START for the deciding STEP) | STEP u |".
- No contract changes with it. P4 never reads the scripts' output: the hooks compare the row's u fields with the
  checker's own count on the replayed start and end states, whichever line P1 took them from.

### D1: height NOT_APPLICABLE, checked, no change

- P4 does not expect a null sub-phase for a height NOT_APPLICABLE. It never mentions `not_applicable` or
  NOT_APPLICABLE. Its only null-result text (section 4.1, the bullet "In an error run, a phase cut off by an exception
  may keep `result` null with a `start` and steps (I0-V1-01)") is about the high phase's letter bound, and
  `check_high_phase` does not read `result` in any case. So P4 does not change for D1.
- How the runner records the new `End('error', 'invariant_height_above_6', details={max_height, digest})` at
  `97815b1` (`S/runner.py`, `Runner.plugin_phase`): the sub-phase's `result` is `error`, its `end` is the current
  snapshot, and the End's `reason`, `details` and `attempts` are stored (:642-645); the height phase's result is set
  to `error` (:652-653); the record is saved (:654); `RuntimeError('<sub> phase error: ...')` is raised (:655-656).
  `Runner.run` catches it, records the run `error` with that message, and saves (:1390-1397). The steps recorded
  before it stand, and the high phase never starts. For P4 this means: the height rows before the error are checked
  by the hooks as usual, and there are no high rows for `check_high_phase` to bound. `error` is a phase result of the
  checker (`PHASE_RESULTS`, `S/check_run.py:68`).

- Integrator, after the decision check: small wording and test fixes from the last verification (no change of behaviour beyond the decisions above).
