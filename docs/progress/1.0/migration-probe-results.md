# Migration probes P1 to P3: Windows results

Status: **probe evidence for [migration-exporter-proposal](migration-exporter-proposal.md) section 5, not an ADR.** P3 and the pipeline fixtures pass. **P1 finds no passing strategy:** every read-only SQLite open either writes a sidecar into the source directory or cannot open the source. Invariants 2 and 3 conflict. The owner decides which one gives way (proposal section 9). The importer is not built until then. P2: see section 6.

Evidence class: **actual Windows**, run on synthetic sessions only. Each session was created by the 0.4 engine in a fresh, marked run root under `work/migration-probes/runs/`. No personal session was opened, copied, locked or named. No performance claims are made: the harness timings are bounds, not measurements. Machine-readable results, sanitised: [migration-probe-results.json](migration-probe-results.json). Raw records stay in the run root, which is ignored by Git.

## 1. Environment and commands

| Item | Value |
|---|---|
| OS | Windows 11, build 10.0.26200 |
| File system | NTFS (internal volume of the run root) |
| Python | CPython 3.14.7, 64-bit, the pinned engine environment `tools/.venv/engine` (with NumPy, as 0.4 needs it) |
| SQLite | 3.50.4 (Python `sqlite3`) |
| 0.4 engine | `server.py` of this checkout, started through `EngineProcess` (port 0, authenticated readiness, owned shutdown) |

| Probe | Run | Command (from the repository root) |
|---|---|---|
| P3, F20, pipeline fixtures | `20261003T000632Z` | `tools/.venv/engine/Scripts/python.exe tools/migration_probes/run_probes.py p3 pipeline --out docs/progress/1.0/migration-probe-results.json` |
| P1, F4, controls | `20261003T000735Z` | `tools/.venv/engine/Scripts/python.exe tools/migration_probes/run_probes.py p1 --out docs/progress/1.0/migration-probe-results.json` |
| P2 | see section 6 | see section 6 |
| Headless checks | | `python tests/test_migration_probes.py`, `python tests/test_migration_p2.py` |

`run_probes.py` writes the public result only when the sanitised text contains no absolute path, account SID, user name or host name (`tools/migration_probes/sanitize.py`).

Fixture states (`tools/migration_probes/fixtures.py`). Every session has two committed moves and a non-empty `engine.lock`:

- `clean`: 0.4 closed normally. The directory holds `engine.lock` and `session.sqlite3`, with no sidecars.
- `crash` (fixture F4): the WAL was checkpointed and then one more move was committed. The fixture build checks that this last commit is WAL-only: the main file's SHA-256 is unchanged by it and the WAL is non-empty. The worker was then terminated through its bound process handle, with no close and no checkpoint. `-wal` and `-shm` remain.
- `crash-noshm`: the same, with `-shm` removed after the worker's exit was confirmed. This is the proposal's "crash with WAL-only content and a missing `-shm`".

## 2. Summary

| Probe or fixture | Result |
|---|---|
| P3 lock compatibility (P3a, P3b, P3c) | **pass** |
| F20 at B1, B2 and B3 | **pass** |
| F15, F18, F21 (zero-byte completion and watch error), two baselines | **pass** |
| P1 source preservation | **fail: no strategy passes**; chosen strategy: none |
| F4 WAL-only commit | copied correctly, with no source change, only when `-shm` already exists (C1a, C1b, C1d); no strategy copies it from `crash-noshm` without writing |
| P1 controls (negative, overflow, guarded mapped write, deny ACE, external append, forwarding) | **pass**; the unguarded mapped-write control is recorded as a detection limit (section 5.2) |
| P2 publication, F17 | see section 6 |

## 3. P3: lock compatibility (run first)

| Case | Result | Evidence |
|---|---|---|
| P3a: 0.4 running; the importer tries the lock | pass | refused with `engine-running`; in the attempt window, `engine.lock` has no watch event, unchanged metadata and unchanged bytes; 0.4 then stopped through its graceful request (exit 0, not forced) |
| P3b: the importer holds the lock; 0.4 starts | pass | 0.4 refused with its own message ("This session is already open in another C600 Studio …"), exit 1; it appended exactly one byte `0` (size 1 to 2). This is expected 0.4 behaviour (`session_lock.py:6`). The importer's decision rejects the import (`event`, `snapshot-diff`, `digest-diff`) |
| P3c: the importer alone (lock, digests, release) | pass | no event; snapshots and digests equal; no change after release |

Observation in P3a: while 0.4 runs and holds the handle it wrote through, the first open of `engine.lock` by any other handle raises one `modified` notification. This happens even for a handle that can only read attributes. Bytes and metadata are unchanged. Later opens and lock attempts raise none. NTFS is publishing 0.4's own pending directory-entry update, not a write by the importer. P3a therefore lets it settle in a separate watch window, records it (`settle_events`), and then requires an empty window for the lock attempt. This matters only while 0.4 runs, and the import is refused then anyway.

## 4. F20 and the pipeline fixtures

Stages 2 to 5 of the pipeline run on a fresh `crash-noshm` source. The copier opens nothing, so these results do not depend on the P1 strategy. Publication is a stub that runs only on acceptance.

| Fixture | Result | What the importer saw |
|---|---|---|
| F20-B1: 0.4 started right after lock acquisition | pass | watch event `engine.lock` modified; snapshot difference (size and times); 0.4 refused with its own message; nothing published |
| F20-B2: 0.4 started between stages 4 and 5 | pass | watch event; snapshot and digest difference; 0.4 refused; nothing published |
| F20-B3: 0.4 started right after `engine.lock` was read for the final digests | pass | watch event only (the final snapshot and digest were taken before the append); 0.4 refused; nothing published |
| baseline, `crash-noshm`, copier opens nothing | pass | accepted and published |
| baseline, `crash`, C1b copier | pass | accepted and published |
| F15: an importer-bug test double changes the database's last-write time between stages 4 and 5 | pass | rejected (`event`, `snapshot-diff`); nothing published; destination unchanged |
| F18: a test double creates and removes `-shm` during stage 4 | pass | watch reports `added` and `removed`; rejected; nothing published |
| F21: an injected zero-byte successful completion, equal snapshots | pass | rejected (`overflow`); nothing published |
| F21: an injected watch error, equal snapshots | pass | rejected (`watch-error`); nothing published |

F20-B3 shows why the watch must stay armed until after the lock is released and the events are drained. There, the watch is the only witness.

## 5. P1: source preservation

### 5.1 Matrix

Six strategies were tested, in the proposal's order: candidate 1 (`mode=ro` with the WAL), then candidate 2 (`nolock=1` under the importer's own lock). Each runs on the three fixture states, in two modes and with two outcomes, giving 12 attempts per strategy:

- watch mode: snapshot plus `ReadDirectoryChangesW`;
- deny mode: a deny-write ACE for the probe account on the source directory and its files;
- outcomes: success, and an injected failure after the first backup step.

Every attempt runs in a fresh child process on a fresh source. It holds the read-only lock and the write guard (section 5.2). The backup is bounded (MPR-07): a busy stall of 5 s, 100 steps without progress, or a 60 s deadline makes the attempt inconclusive, not failed. No attempt was inconclusive.

An attempt passes only if both hold:

- the decision accepts: no event, no overflow, no watch error, an armed watch, and equal snapshots and digests;
- the case is reached: on success, the copy passes `integrity_check` and holds the expected head and its state hash; on failure, the injected failure was reached.

A strategy passes only if all 12 attempts pass.

| Strategy | `clean` | `crash` (F4) | `crash-noshm` | Verdict |
|---|---|---|---|---|
| C1a `mode=ro`, no checkpoint on close | watch: creates `-wal` and `-shm`, which are left behind; deny: `unable to open database file` | **pass** (4/4) | watch: creates `-shm`, which is left behind; deny: cannot open | fail (4/12) |
| C1b C1a plus `readonly_shm=1` | as C1a; on success the backup also hits the no-progress bound (100 steps without progress) | **pass** (4/4) | as C1a | fail (4/12) |
| C1c C1a plus `locking_mode=EXCLUSIVE` | `disk I/O error` on open; source unchanged | same | same | fail (0/12) |
| C1d C1a plus `SQLITE_FCNTL_PERSIST_WAL=1` (C API) | as C1a | **pass** (4/4) | as C1a | fail (4/12) |
| C2a `mode=ro`, `nolock=1` under the importer's lock | `unable to open database file`; source unchanged | same | same | fail (0/12) |
| C2b C2a plus `readonly_shm=1` | same | same | same | fail (0/12) |

Result: **no strategy passes; no strategy is chosen.** The pattern matches SQLite's documented rule for read-only WAL databases: such a database opens only if `-wal` and `-shm` already exist, or can be created, or the database is opened as immutable. With both sidecars present (`crash`), C1a, C1b and C1d copy the WAL-only commit and leave every byte unchanged. When a sidecar is missing (`clean`, `crash-noshm`), they create it in the source directory, which breaks invariant 3. Under the deny ACE they cannot create it and fail visibly, preserving the source but copying nothing. `nolock=1` and exclusive locking do not open the WAL database at all. Every failed open left the source unchanged.

Not tested, because the hard rules exclude them: `immutable=1`, and any plain file copy of the database.

### 5.2 Controls

| Control | Result | Evidence |
|---|---|---|
| Negative: a deliberate read-write open that creates and removes `-wal` and `-shm` | pass | the watch reports `-shm` added, modified and removed; rejected (`event`, `snapshot-diff`) |
| Overflow: 64-byte notification buffer, a burst of 300 changes, timestamps restored | pass | 900 zero-byte successful completions (last error 1022, `ERROR_NOTIFY_ENUM_DIR`); snapshots equal; rejected (`overflow`) |
| Mapped write, unguarded (recorded, no pass rule) | detection limit | one byte of an existing `-shm` changed through a writable mapping and restored: no watch event, equal snapshot, equal digest, so the decision accepts. A mapped write is invisible to `ReadDirectoryChangesW` and to metadata and digest checks |
| Mapped write, with the write guard | pass | the writable open is refused (sharing violation); accepted with nothing written |
| Deny ACE prevention | pass | creating a file and opening an existing file for writing are both denied; the ACE was removed afterwards |
| External append (MPR-06): during a C1b `crash` backup, another process runs 0.4's `SessionLock` | pass | that process is refused (exit 3); the import is rejected (`event`, `snapshot-diff`, `digest-diff` on `engine.lock`) with no timing input and no writer attribution |
| Forwarding: the fixture worker is started through the venv launcher | pass | the bound worker (by its own reported PID) was terminated and its exit confirmed; the launcher exited by itself |

The write guard (MPR-04): while the importer holds the lock, it also holds `GENERIC_READ` handles that share only reads on every existing source file except `engine.lock`. Every writable open by anyone then fails with a sharing violation. This includes SQLite's own `-shm` open, which then falls back to read-only, as SQLite's Windows VFS does. The guard is compatible with SQLite's read-only opens and with the digest reads. It closes the mapped-write gap that the watch cannot see. It cannot stop the creation of a new file, which is why `clean` and `crash-noshm` still fail. The guard ran in every P1 attempt and every pipeline fixture.

### 5.3 Consequence: the owner decides

Invariant 2 (online backup under the lock; no file copy, no `immutable=1`) and invariant 3 (the source directory is never modified, not even transiently) cannot both hold on Windows for a session without `-shm`. A clean 0.4 shutdown always leaves a session in that state. The owner was asked once (workbench ask `20261003T001232Z-b3044edd`) to choose:

- (b), recommended: under the lock and the write guard, copy the bytes of `session.sqlite3` and `-wal` through read-only handles into the 1.0 temporary directory, and let SQLite recover the WAL in the copy only. This relaxes "no file copy" in invariant 2 and keeps invariant 3. It is untested here; P1 must be re-run for it before the importer starts.
- (a): allow SQLite to create `-wal` and `-shm` in the source under the lock and leave them there. This relaxes invariant 3.
- (c): use `immutable=1` when no `-wal` exists, and refuse a crashed session without `-shm`. This relaxes "no `immutable=1`" and does not cover every state.

## 6. P2: publication

Pending: the Codex implementation of `tools/migration_probes/p2_publish.py` (test double and Windows CLI) is being integrated. This section will record:

- the test-double results (loss of buffered writes);
- the Windows process-interruption results, reported separately;
- F17;
- the qualified barrier.

## 7. Harness rules that the importer design must adopt

- **Decision without attribution (MPR-06).** Stage 5 accepts only an armed watch with no event, no overflow and no watch error, plus equal snapshots and digests. The rule is the same in watch and deny mode. It uses no timing input and names no writer. The proposal's stage 5 text, which classifies an event as an importer breach or as another program by when it falls, is superseded for the decision. The user report may only name causes that the harness observed directly, such as a refused 0.4 start.
- **Write guard (MPR-04)** during the whole lock-held interval (section 5.2). Stage 5 detects changes of name, size, attributes and timestamps, and byte differences at the digest points. It cannot detect a mapped write that is restored before the final digest. "Never writes, not even transiently" therefore rests on prevention, the guard and the deny-mode run, not on detection.
- **Bounded backup (MPR-07):** a bound makes the attempt inconclusive, never accepted.
- **Settled lock observation (P3a)** applies only to the refusal path, never to an accepted import.

## 8. Not covered

- File systems other than local NTFS: ReFS, FAT, exFAT and network shares.
- Antivirus or indexer interference was not controlled. None was observed in these runs.
- Real power loss. P2 models buffered-write loss with a test double; the Windows runs interrupt processes only.
- Performance, input and long-session behaviour: no claims.

## 9. Review record

- Plan check `20261002T231650Z-67c88b8a` (Astra): MPR-01 to MPR-06 adopted.
- Scoped re-check `20261002T235125Z-b4e2f6c8`: MPR-01, MPR-02, MPR-04 and MPR-07 adopted.
- Final review: see the pull request.
