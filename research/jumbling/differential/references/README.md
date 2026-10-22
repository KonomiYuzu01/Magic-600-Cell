# Jumbling reference values (EN2 slice 4)

Status: acceptance items 3 (its remaining refusals), 7 and 8 of packet ENG-0J
revision 2 (`docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md`)
and amendment A5, as reference values from J1 and the research scripts, and
the optional-state session prototype in
[`../candidate/session.py`](../candidate/session.py). Full-mode results:
[`results/summary.md`](results/summary.md).

**Evidence kind.** Source and fixture replay with the J1 reference engine
(`research/jumbling/sim/`) and the research scripts, run headless in a Linux
cloud container. A time is a cloud timing of single-threaded research code: a
lead, never performance evidence for a product engine or for Windows/DirectX.

**Boundary.** A value answers about the state, the cap, the word or the
correction set that the caller names. Nothing here chooses, ranks or proposes a
twist, a cap, a correction or a word (the human-solve boundary of
`docs/development-guide/AGENT_BRIEFING.md` section 2). Until a candidate engine
exists, each comparison runs against these references. A planted wrong value
must make its comparison fail, and the failure names the first difference.

**Imports and writes.** These modules may import J1, the research theory and
restoration scripts, the harness (`differential`, `differential.leads`) and
NumPy, but never `differential.candidate`; `tests/test_jumbling_differential.py`
checks that. Each imports `differential.leads.census` before any research
module, so the research centroid cache goes to a temporary directory. Every
command runs from the repository root with `python -B`, takes `--out DIR` and
refuses a directory, or an output file, that resolves inside the repository
before it computes anything.

## ν and label certificates (`nu.py`)

`nu(state)` gives ν(X) of a lattice configuration as 45 integers in the order
of `NAMES`: the sign bits b0 to b34, the frame bits f0, f3, f6, f8, f11, f22,
f25 and f32, and C5_17 and C5_28 modulo 5 (amendments A3 and A5). It reads
π(X) from J1's labelled stickers and evaluates it with the stage 03 research
reference `nucoords.py`. The C5 values are read in the generator r0 that
`nucoords.py` picks, which is the C5 convention of A5. Off the lattice, ν and
the labels are undefined (`NuUndefined`).

- `check_fixed(value)`: under I_a, b0, b1, b2, b3, b5, b6, f0, f3 and f6 are 0
  on every reachable lattice configuration (stage 04). A nonzero one is a
  consistency error of the engine (`NuConsistencyError`), never a state.
- `label_certificate(state)` checks the lattice test, the centres in their own
  chambers, the frame agreement and ν = 0. It returns the fields of the store
  prototype's certificate: the pose digest, the labels digest, ν, the C5
  convention, the SHA-256 of the certificate program and the contract
  revision. A refusal (`LabelCheckpointRefused`) names the fact (`off_lattice`,
  `centre_displaced`, `nu_nonzero`) and the kind of action, never a twist or a
  correction. Reachability is the caller's check.

## Corrections and recorded ν (`corrections.py`, `nu_refs.py`)

`load_library()` checks the SHA-256 of the decompressed correction library
(`research/jumbling/restoration/09-general-method/data/library-merged.json.gz`)
before it parses it (`LibraryDigestError` names both digests). It gives the 36
corrections in id order, each with its family, length, exact J1 twist records,
letters in the pinned I_a order and 45-coordinate ν row.

- `play(state, ids)` plays the corrections the caller names, in the caller's
  order. It commits only when every prefix is admissible; otherwise
  `InadmissiblePrefix` names the correction and the letter index, and the state
  is unchanged. On a lattice state it transports each correction's exact action
  from its replay from solved. This is exact because every lattice state fills
  the same piece regions as solved (stage 03, section 3.2), so a prefix is
  admissible there exactly when it is admissible from solved. A test checks
  the transported state against a literal replay, journal included.
  `literal=True` (`--literal`) replays every twist on the state itself.
- `check_row(state, cid)` compares ν after a correction with ν before plus the
  correction's row (EH-11), on a copy of the state.
- `drafted_nu(value, ids)` is ν plus the rows of the corrections the caller
  drafted (O6 (b)). It plays nothing and never lists, orders or proposes a
  correction.
- `nu_refs.compare_table(path)` rebuilds a research pose table as stage 04's
  `table_nu.py` does and checks its digest. It compares ν with TABLE-NU
  (`research/jumbling/restoration/04-card5-lattice-invariants/certs/table-nu.txt`,
  matched by the SHA-256 of the decompressed table) and with `nu_input` of
  `research/jumbling/restoration/09-general-method/earlier/x-e3-solved/restore-summary.json`
  (EH-10). `--cert-lb` replays the 40 words of CERT-LB-V2 from solved.

```sh
python -B research/jumbling/differential/references/corrections.py --out DIR [--from solved|E3|I1|I2] [--ids ID ...] [--literal]
python -B research/jumbling/differential/references/nu_refs.py --out DIR [--cert-lb]
```

## Observations (`observations.py`, `obs_refs.py`)

`observe(state)` gives the observation data of one revision (EH-05, EH-26):

- a canonical domain key per piece: the SHA-256 of the sorted exact images of
  the 120 vertices of the 600-cell, independent of how `rim.Domains` numbers
  the domains;
- the left and right tree heights of each pose class, and the counts by left
  height in X5's bins;
- the rim summary of every cap: status, N_c, In_c, Out_c, A_c, the floor
  |In_c − Out_c| and whether N_c is at it, with N and the grip N.

The rim summary comes by two routes, `rim.py` and `seams.balance`, which must
agree. `cut_cells(state, c)` lists the occupied cells of a named admissible
cut. `off_grip_seams(state)` lists the cells occupied on one side only whose
plane is no grip's, each with the caps that contain the whole closed cell and
their status. `compare` names the first difference. Its reference must be
complete and consistent: a domain key for every piece, the heights and piece
count of every occupied pose class, the counts by left height that follow from
them, every cap with the values its status requires, and grip N as the sum of
the admissible caps' N_c.

`obs_refs.py` rebuilds the reference states and checks their recorded values:

- the W-J fixtures at the prefixes of `research/jumbling/theory/results/trees-fixtures.json`
  (both height histograms);
- the I_a@40 stop after its 62 recorded restorer twists
  (`research/jumbling/theory/results/seams-I_a-40-stop.json`: N 1,192, grip N 1,057);
- the deep scramble (`research/jumbling/theory/results/deep-scramble-I_a.json`:
  165,529 pieces off).

With `--extract DIR --height4`, it also replays the six height-4 states of
stage 12 from draft #112's branch.

```sh
python -B research/jumbling/differential/references/obs_refs.py --out DIR [--states fixtures stop40 deep] [--extract DIR --height4]
```

## Word checks and records (`word_check.py`, `records.py`)

`word_check(state, letters, unit, cap=None)` checks a word the solver names at
one revision, on a copy of the state (EH-25; DT-J1 M9, M12). It gives:

- the start and end digests, the number of letters and their SHA-256;
- the unit's size and how many of its pieces end on the lattice;
- the lattice pieces of the start that the word lifts off;
- the other off pieces of the start that keep their exact pose and those that
  land;
- the off counts, ΔN and the change of the counts by left height;
- Δu at a named cap.

`step_flag` is M9's conjunction: every prefix admissible, the whole unit on the
lattice, no lattice piece lifted, and every other off piece landed or at its
exact pose. An inadmissible prefix gives a result with the letter index, not an
exception.

- `release_fields(S, c, x, cap=None)` is slice 3's X5 release check with Δu at
  a named cap. It agrees with the word check of that one letter in every shared
  field.
- `preview` replays the word. `commit` returns the previewed end state, journal
  included, when the start's poses and journal, the menu's identity and order
  and the word are those of the preview, and its end state is unchanged
  (`StalePreviewError` names the field otherwise; `BlockedWordError` names the
  letter of a blocked or uncertain prefix). J1's digest leaves out the journal,
  so both journals are checked on their own. A word that lifts lattice pieces
  or fails the step flag is shown, not refused (DT-J1 A7).
- `compare_step` compares the shared fields with a row of `eb4_replay.py`
  (`EB4_MAPPING`, `STEP_MAPPING`) or of `replay_check.py` (`REPLAY_MAPPING`).
- `records.read_against` reads a repair-list result, a covers result, a stored
  word or the correction library against the input digest, menu identity,
  order digest, contract revision and library digest the caller expects.
  `RecordMismatch` names the field. A missing identity is a mismatch, and so is
  a record that spells one identity twice with different values. A job result
  that did not complete is never read as a list (`IncompleteRecordError`).
- `records.check_menu_closure` checks a menu record for inverses, for
  conjugation by A4₀ and for A4₀ itself, then hands it to J1.
- `records.request_twist` refuses a rotation that is not an exact menu element
  or has no exact representation, with the state unchanged.

```sh
python -B research/jumbling/differential/references/word_check.py --out DIR [--extract DIR] [--cases u1 b20 b06-14 b14 b03 r22b34]
```

## The Jumble session (`../candidate/session.py`, `session_engine.py`)

`Session(engine, path, identities=...)` is a session prototype over the store
prototype. Its jumbling part is optional and Jumble is off by default
(acceptance item 7, amendment A5). It imports only `differential.candidate`,
the standard library and NumPy, and takes its engine through the `Engine`
protocol. `J1SessionEngine` implements the protocol with J1 and `nu.py`. It
applies generator moves to labels alone from `assets/primitives.npz`,
independently of J1's poses.

- Every command takes the revision it was issued at and either writes one whole
  revision through `store.write_session` or changes nothing (`StaleRevision`
  for another revision).
- Load checks every recovery checkpoint as it checks the current retained part
  (model identity, labels digest, journal, checkpoint and, where labels can be
  replayed, their digest), and restore checks the one it restores again.
- A session in which Jumble is never on writes exactly the bytes of
  `RetainedOnlySession`, which has no jumbling code path.
- `jumble_on` enters from a retained word from solved, or resumes from the last
  exit when the chain since it is intact. It refuses labels alone, and refuses
  to resume after a reset, an import or a restore.
- `jumble_off` is allowed at label checkpoints only: on the lattice, the
  centres home, ν = 0, and the stored journal, replayed from its start, reaches
  the stored poses. It stores the label certificate and the exact exit poses
  for every crossing, also when a witness is supplied. It hands π(X) to the
  600-cell-Full part as a label checkpoint, or the witness word as a retained
  checkpoint.
- Refusals are typed (`kind`). Their messages name the fact and the kind of
  action, never a twist or a correction.
- In this single-menu prototype, `menu_change` accepts only the engine's own
  identities; its test covers the refusal inside an episode.

## Gaps

- **Exit at the solved end with a non-solved pose digest** (item 7, case 5): no
  reachable state with identity labels and a non-solved digest exists on
  `main`, and the one stage 13 measured follows 8.7 million retained twists.
  The test exits at solved reached through jumble twists; the non-solved case
  is not run.
- **The view-model boundary test of item 3** waits for EN3, which names the
  view-model fields. The piece filters wait for DT-J2 (the design table is on
  hold).
- **R22B-34's negative controls** (a wrong inverse order, dropped frame
  rotations) are described but not stored as words, so `word_check.py`
  reports that case unavailable.
- **Identities missing from slice 3's results**: a covers result has no
  contract revision, and a repair-list result no library digest. Asking for
  one is a mismatch (`RecordMismatch` with the recorded value `None`).
- **The I_a@40 stop's "20 caps that hold off-grip seams"**
  (`research/jumbling/theory/open-problems.md`, R1 in detail) is not
  reproduced. By the reading of `off_grip_seams`, 39 caps hold an off-grip
  cell and 25 hold a whole group, and no count of that reading is 20; the
  experiment script behind the prose was not kept. The other prose values at
  that stop agree ([results](results/summary.md)).

## Inputs outside this repository

The full-mode runs of `word_check.py` and `obs_refs.py --height4` read words,
tables and records that exist only on draft #112's branch (commit `ec0f0ac`),
extracted with `git show` into a scratch directory outside the repository and
never committed. Tests never read them. [`results/summary.md`](results/summary.md)
lists each with its path at `ec0f0ac` and its SHA-256.

## Tests

`tests/test_jumbling_nu.py`, `tests/test_jumbling_observations.py`,
`tests/test_jumbling_word_check.py` and `tests/test_jumbling_session.py` run
the bounded checks with NumPy, in one isolated worker each under a shared
unittest command (`tests/isolated_numpy.py`). Each writes only in temporary
directories outside the repository, and an audit hook fails any other write.
The Windows CI job runs them with the other headless checks; a pass there is
not Windows/DirectX evidence.
