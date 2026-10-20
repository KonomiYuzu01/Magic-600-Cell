# P5 refresh: change log

Packet P5 (C-L: literal lattice replay, checker side). The draft was checked against base `97815b1`
(`97815b19cd3243d30ed7c1632bc95b6e1751541e`). The refreshed packet is `v2/P5.md` next to this file.

Abbreviations: S = `research/jumbling/restoration/13-tutorial-runner/scripts`; J = `research/jumbling`.

## 1. Base filled

- The placeholder is now `97815b1`. The full sha appears once, in section 3.
- The sentence "if a line has moved in your worktree, the named symbol governs" is kept.
- Section 3 now says that `97815b1` is I0 with the fixes of its review (I0-01 to I0-04, V1-B2) and of its scoped
  verification (I0-V1-01). The second verification round passed (stage 13 README :925-956).
- The draft treated parts of the I0 interface as not yet fixed ("if BASE passes the run directory or the tree").
  At BASE it is fixed, and the packet says what it is.

## 2. Citations

161 citations were checked: named `path:line` ones and bare `:NNN` continuations. The following were unchanged and
correct: everything in `sim/` (model, kplus, twists, state, accept, regions); `theory/restore.py:45-91`;
`theory/rim.py:29`; `theory/seams.py:66`; `theory/buffer-transport.md:10`; the decision page (:54-59, :68,
:74-76, :89-96, :93-94); `tools/agents/codex_review.py:105` (`ACCEPTANCE_TIMEOUT = 1800`); `literal_lattice.py`
`LatticePlay` (:224-336); the stub `check_lattice.py` (:30-34, :40-41, :44-62, :65-87); most `check_run.py`
lines below :900 (`inside`, `Snapshot`, `check_v2_step`, `check_v3_step`, `load_word_v3`, `check_tree`).

Re-anchored, by symbol (old -> new):

S/check_run.py
- `replay_lattice_step` call block: 1157-1166 -> 1186-1195. The call itself is :1189. Type check and exception
  path: 1159-1164 -> 1190-1193, and 1163-1164 -> 1192-1193.
- `step_row` and `merge`: 1004-1017 -> 1033-1046.
- `lattice_check`: 859-988 -> 888-1017. Its call: now :1307-1311. No end digest for composition: :1315-1316.
- Main letter loop: 1178-1187 -> 1207-1216. `ctx` and `element` in it: 1180 -> 1209.
- `check_tree` call: 1065 -> 1094.
- `menu = restore.menu('I_a')`: 1048 -> 1077.
- Checks before any replay: 1050-1083 -> 1079-1112. Path check: 1052-1058 -> 1081-1087. Invalid-data gate:
  1072-1083 -> 1101-1112.
- `before.check`: 1141-1145 -> 1173-1174. `after.check`: 1203-1205 -> 1233-1234, and 1204 -> 1233.
- End of a played lattice: 1264-1276 -> 1293-1305. Last-row `labels_identity`: 1272-1275 -> 1301-1304.
- Played-result skip (newly cited): :1317-1324.
- `word_literal` check: 248-251 -> 248-250.

S/runner.py
- `SOLVED`: 43 -> 44.
- `word_sha256` of a `json` word: 86-87 -> 100-101.
- Where a lattice row is written: 646-647 (`record_step`) -> 1357-1361 (`played_lattice`). `record_step`'s own row
  is now :713-718.
- Lattice snapshot and heights: 203-212 -> 205-209 (`lattice_snapshot`) and 240-249 (`Mechanics.heights`).
- Newly cited: `ROW_KEYS` :66-67; `played_lattice` :1319-1372 (docstring :1319-1324, checks :1325-1349, zero-step
  refusal :1326-1328, table on the last row :1363-1365, result `solved` :1368); `lattice()` End error :1271-1275.

S/runner_api.py
- `Ia16Writer`: 193-224 -> 234-265. `GzipFile`: 200 -> 241. Code check: 206-207 -> 247-248. Byte swap: 208-209 ->
  249-250. sha256 update: 211-212 -> 252-253. `close`: 218-224 -> 259-265.
- `ia16_code`: 187-190 -> 228-231.
- `walk`: 133-140 -> 165-172. `check_tree`: 147-175 -> 179-207; its references: 165-171 -> 197-203.
- `write_tree`: 178-182 -> 210-214. `TREE_MAX_DEPTH`: 26 -> 29. `span`: 101-104 -> 133-136. `group`: 107-110 ->
  139-142.
- Newly cited: `LatticeStep` :61-79; `ia16_digest` :268-290; `read_tree` :217-223.

S/test_check_run.py
- `test_v3_played_lattice`: 1542-1548 -> 1655-1661. Its stub assertion: 1545 -> 1658. Both played tests:
  1542-1576 -> 1655-1689.
- `test_v3_lattice_labels_not_digest`: 1563-1576 -> 1676-1689.
- `IndependenceTests`: 1721-1781 -> 1834-1894. `ALLOWED`: 1726 -> 1839. `FORBIDDEN`: 1728-1729 -> 1841-1842.
  `RESTORE`: 1733 -> 1846. `RIM`: 1734 -> 1847. `scan`: 1736-1767 -> 1849-1880. Dynamic-import check: 1762-1763 ->
  1875-1876.
- Newly cited: `build_played` :1096-1117; `played_case` :1146-1164; the patch of `replay_lattice_step` :1673 and
  :1682-1687.

S/test_runner.py
- `SKIP`: 2383 -> 2817. `inventory`: 2386-2401 -> 2820-2835. `status`: 2404-2406 -> 2838-2840. `TimedResult`:
  2371-2379 -> 2805-2813. `main`: 2409-2427 -> 2843-2861.
- Newly cited: `ExactTests.test_09_played_lattice_passes_the_checker` :2749-2774 (stub assertion :2773-2774).

Stage 13 README
- Composition verifier quote: 146-147 -> 564-565. Invisible rotation: 176-179 -> 594-597.
- Newly cited: P5's range check :510-511; I0 review :925-956; deferred items :957-967 (lattice stuck, budget and
  timeout :959-960; played result "with packets P3 and P5" :966-967).

S/test_literal_lattice.py
- Construction of T and S: 646-676 -> 649-676. Final check: 697-702 -> 694-702.

## 3. Interface conflicts (BASE governs)

1. `UNCHECKED`. The stub defines `UNCHECKED = 'unchecked_contract'` (`check_lattice.py:41`). The draft said "keep the
   stub's other names", but test A forbids that string in the module. No file reads `check_lattice.UNCHECKED`
   (`check_run.py` has its own, :81). The packet now keeps `J`, `GRIPS` and `ITEMS` and drops `UNCHECKED`.
2. Who writes lattice rows. The draft said P3's plugin and I1 would write them through `record_step`. At BASE the
   runner's `played_lattice` writes them (I0-02, `runner.py:1357-1361`): core keys with `attempts: []`, then the
   `LatticeStep` fields; only the last row has `table` (:1363-1365). The runner checks the files and the digest chain
   (:1325-1349) but never replays (:1319-1324). Section 4.2 is rewritten. A new section 4.2 item says the runner's
   checks are not evidence for the checker.
3. The interface gap. The draft left open whether BASE would pass the run directory or the tree. The call at
   `check_run.py:1189` passes neither. The packet keeps its derivation of the run directory from `path` and
   `row['word']`, and says the gap is decided.
4. A third I0 test asserts the stub. `test_runner.py` `ExactTests.test_09` (:2773-2774) expects
   `unchecked_contracts` 2. With the real module the count is 0. It joins the two `test_check_run.py` tests in the
   list of tests the integrator changes at integration. `test_runner.py` is added to the do-not-edit list.
5. The played result comparison. README :966-967 defers it "with packets P3 and P5", but the code is in
   `check_run.py` (:1317-1324), which P5 may not edit. It is out of P5's scope and goes to I1. The runner writes
   `solved` (`runner.py:1368`).
6. Lattice `stuck`, `budget`, `timeout` and zero-step phases. At BASE the runner ends such a run `error` and records no
   lattice row (`runner.py:1271-1275`, :1326-1328). README :959-960 defers the checker's rules. Out of scope.
7. New runner readers. `runner_api.ia16_digest` (:268-290) and `read_tree` (:217-223) are new at BASE. The packet
   names them as test oracles only: test B also compares `ia16_info` with `ia16_digest`. `check_lattice.py` must not
   import them (reading 1).

## 4. Clarifications added

- Section 1 says why the checker's replay matters: the runner checks a played phase but does not replay it, so P5's
  replay is the only literal replay of a lattice word.
- Section 5 gains row 5 (I0-02, `played_lattice`) and an "I0 review" history bullet.
- Section 4.6 adds the re-check results at BASE, each labelled (see section 6 below).
- The other packet text, the seven-part structure, the tests and the failure-name table are unchanged apart from
  line numbers.

## 5. Integrator decisions and open points

None is owner-level. Each needs only the integrator's confirmation.

1. Label ownership (section 4.1). P5 owns the word file, the ranges, the replay, the end state and
   `labels_identity`; `check_run.py` owns the row keys, the structure of the tree and the before/after snapshots.
   Recommendation: keep it.
2. The run-directory derivation. Recommendation: keep it. Optionally, I1 passes `run_dir` as a keyword argument, which
   would be an I0 interface change, not a P5 one.
3. "Stay on `State.apply` for the rest of a short correction step". Recommendation: keep it. A switch costs about
   0.1 s, and an orbit step must still run on the model (test D).
4. The `FastPathError` guard with the patchable `fast_letter` predicate. Recommendation: keep it. It is what lets test
   F show "a non-retained letter on the fast path" fails.
5. The failure names of section 4.5. Recommendation: keep them. They reuse `check_run.py`'s names where they overlap.
6. Tree failures do not stop the replay. Recommendation: keep it. The word is verified by its sha256; the tree only
   presents it.
7. P5 checks both `inverse_of` and `same_as`. Plan section 2 names only `inverse_of`, but README :510-511 and plan
   section 1 require both, and `check_run.py` compares neither on a lattice word. Recommendation: check both.
8. Test D's fixture (W = A·B·A⁻¹·B, then w9). The re-check confirmed T, Y and the counts (section 6 below).
   Recommendation: keep it.
9. The played result comparison (I1). Recommendation: I1 compares `phases.lattice.result` with `solved`, as
   `runner.py:1368` writes, and fixes the `test_check_run.py` fixture that writes `'lattice'` there.
10. The three I0 tests. Recommendation: change them in the same candidate that integrates P5, so no commit has a
    failing suite.
11. Zero-step phases. If P3 lets a played phase have no step, `check_run.py` will need `labels_identity` for the
    table state. Recommendation: either P5 also exposes a public `labels_identity(state)` helper, or I1 computes it
    itself. This is not needed for P5's acceptance.

## 6. Acceptance check, fixtures, times

- `test_check_lattice.py` does not exist at BASE. P5 creates it, so the check cannot run until P5 lands. That is
  intended. The command form is the same as the other packets'; the wrapper stops a check at 1,800 s.
- The fixtures exist at BASE: `J/restoration/09-general-method/data/library-merged.json.gz` (36 macros, `w9` first,
  9 records), `assets/model.npz`, `assets/primitives.npz`, the stub `check_lattice.py`, `runner_api.py` (the writers
  and readers) and `test_check_run.py` (the scan).
- Re-checks run at BASE for this refresh (read-only, outside the repository, `python3 -I -B`; cloud timings of research
  code):
  - import of `restore` plus the I_a menu: 6.4 s (measured). The menu has 60 items; items 0 to 11 are retained;
    item 0 is the identity;
  - the `a4_word` words at grip 5 match the packet (measured);
  - `slot_piece` takes 177,120 distinct values; `move_src` has shape (1200, 3097) (measured);
  - w9: the nine letters and the off counts match the packet; the nine applies took 0.44 s (measured; the draft's
    0.94 s is kept as the integrator's earlier measurement);
  - T = solved·w9⁻¹ is on the lattice with labels that are not the identity; w9 played from T gives the same off
    counts and ends at `SOLVED`, which confirms test D's `simulated == 9` (measured).
- Time labels: the 15 ms per retained letter, 0.1 s and 0.03 s are integrator measurements; 36 hours, 7 minutes and
  "about 20 minutes or less" are estimates; the plan's 35-60 ms per twist is now labelled a planning figure that was
  not measured for this packet.
- Environment: Python 3.13.16, NumPy 2.5.3, four cores.
- The implement-contract block (allowed files, acceptance check, stop condition) is unchanged.

## 7. Scans

- No private paths and no model identifiers in the packet. The role names stay as they were.
- The seven-part structure is kept.

## 8. Adversarial verification (P5-V-01 to P5-V-05), all adopted

Each finding was re-checked at `97815b1` (read-only, `python3 -I -B` outside the repository; probes copied to
`verify-p5-fix/` next to this file). The repository status was identical before and after.

- P5-V-01 (major) adopted. Re-run of the verifier's w9 probe: w9 from T ends solved; (139, 26) -> (139, 12) or
  (139, 14) at the fourth letter is blocked at the fifth, (16, 1); (0, 12) -> (0, 13), (0, 16) or (0, 14) at the first
  letter and (0, 45) -> (0, 14) at the last end on the lattice with labels not the identity; (0, 45) -> (0, 12) or
  (0, 13) ends with 3,097 pieces off. Test F now names two measured mutations with exact sets: fourth letter ->
  (139, 12) gives `{admissible}` with the state after the first four letters; last letter -> (0, 14) gives
  `{after_digest, labels_identity}`. "report which" removed from test F and from section 7.
- P5-V-02 (minor) adopted. The `same_as` mutation now changes each letter of the second `B` to another retained item
  at the same grip, so the replay stays on the lattice and the set `{tree_expansion, after_digest}` is exact (row 0
  has no `labels_identity`; section 4.3: a retained letter keeps a lattice configuration on the lattice).
- P5-V-03 (minor) adopted. Section 4.1 now names the deliberate repeats: step 1's `encoding` and `word_sha256` guards
  (`check_v3_step` emits the same names, `check_run.py:411-414`, and `check_run.py` then never calls P5, :1101-1112)
  and the `after_digest`/`after_off` end comparison; `add` merges duplicates (:96-98). Checked: for a lattice row
  `check_run.py` compares `letters` only with the tree root (`tree_length`, :533-534; `check_v2_step` checks
  `letters` only when it holds the word, :323), so step 1's `letters` guard is P5's own and is said to be no repeat.
- P5-V-04 (minor) adopted. `runner_api.py:277` reads 1 MiB chunks; 200,000 letters are 400,000 bytes, one chunk.
  Section 4.5 now names a module constant `CHUNK_BYTES`, looked up by every reader at call time; test B reads each
  file once with it and once with an odd value such as 4,097, so several chunks and split codes are exercised.
- P5-V-05 (minor) adopted. `literal_lattice.py:215-221` (`state_labels`) requires `frame_agrees`; `sim/state.py`
  `lattice_stickers` returns it per sticker without raising. Step 5 now defines `labels_identity` as on the lattice,
  frames agreeing everywhere and labels equal to `arange(NS)`, and a lattice step end with any frame disagreement
  gives `model_labels` (decided here: the existing name whose table meaning already covers frames; its table row is
  widened). Re-check: frames agree at T and at the 12 labels-equal invisible-rotation candidates of the probe, so
  this is an edge case, not a current failure.

## Integrator decisions (20 October 2026)

- D7 (from P5-V-05): keep `model_labels` for a step end on the lattice with a frame disagreement, with the widened
  table row. Record only: section 4.5 step 5 and the `model_labels` row of the failure-name table already say this, and
  the packet text is unchanged.
- Cross finding X-05: section 4.4, the sentence after the `a4_word` words at grip 5 (one edit). Before: "A lattice word
  written by P3 has one generator or its inverse per letter (items 1, 2 and 5)." After: "... per letter (for example
  items 1, 2 and 5 at grip 5)." The item numbers are the measured words at grip 5. A re-check while verifying this
  edit (`verify-p5-d7/gen_items.py` next to this file, at `97815b1`, read-only, `python3 -I -B` outside the
  repository) found that the signed generator ids of every one of the 600 grips are items 1, 2 and 5, so the example
  also holds at the other grips. The line is re-wrapped; nothing else in the packet changed.

- Integrator, after the decision check: small wording and test fixes from the last verification (no change of behaviour beyond the decisions above).
