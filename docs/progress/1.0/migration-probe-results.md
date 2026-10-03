# Migration probes P1 to P3: Windows results

Status: **probe evidence for [migration-exporter-proposal](migration-exporter-proposal.md) section 5, not an ADR.** P3 and the pipeline fixtures pass. **P1 passes with C3**, the locked byte copy that the owner chose on 3 October 2026 ([decision](../../wiki/decisions/owner-decisions-2026-10-03-migration.md)): all 12 attempts leave the source unchanged, and every successful copy holds the expected head, including a WAL-only commit without `-shm`. No read-only SQLite strategy passes: for a session without `-shm` (`clean`, `crash-noshm`), each one either writes a sidecar into the source directory or cannot open the source, and three of them open in no state. That conflict between invariants 2 and 3 is what the owner settled (section 5.3). P2 passes: the publication order survives a process interruption at every step, and the barriers are named in section 6.3. The importer may start on C3.

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
| P3, F20, pipeline fixtures, P1 with F4 and controls, P2 with F17 | `20261003T005400Z` | `tools/.venv/engine/Scripts/python.exe tools/migration_probes/run_probes.py p3 pipeline p1 p2 --out docs/progress/1.0/migration-probe-results.json` |
| Headless checks | | `python tests/test_migration_probes.py`, `python tests/test_migration_p2.py` |

All results here come from that one run, made on the final harness after the review fixes (section 9). Four earlier runs, before C3 or before the last fixes (`20261003T000632Z`, `20261003T000735Z`, `20261003T002250Z`, `20261003T004448Z`), gave the same pass or fail in every case they share; they are superseded.

`run_probes.py` writes the public result only when the whole merged, sanitised document contains no absolute path, file URI, account SID, user name or host name (`tools/migration_probes/sanitize.py`).

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
| P1 source preservation | **pass**; chosen strategy: **C3** (12 of 12 attempts). C1a to C2b fail |
| F4 WAL-only commit | C3 copies it, with no source change, from `crash` and from `crash-noshm`. C1a, C1b and C1d copy it only when `-shm` already exists |
| P1 controls (negative, overflow, guarded mapped write, deny ACE, external append, forwarding) | **pass**; the unguarded mapped-write control is recorded as a detection limit (section 5.2) |
| P2 publication, F17 | **pass**: the test double (buffered-write loss, flush failures) and 36 Windows process interruptions; the barriers are in section 6.3 |

## 3. P3: lock compatibility (run first)

| Case | Result | Evidence |
|---|---|---|
| P3a: 0.4 running; the importer tries the lock | pass | refused with `engine-running`; in the attempt window, `engine.lock` has no watch event, unchanged metadata and unchanged bytes from byte 1 on (byte 0 is locked by 0.4 and cannot be read; the size is unchanged); 0.4 then stopped through its graceful request (exit 0, not forced) |
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

Seven strategies were tested, in the proposal's order: candidate 1 (`mode=ro` with the WAL), candidate 2 (`nolock=1` under the importer's own lock), then C3, the owner's choice after candidates 1 and 2 had failed (section 5.3). Each runs on the three fixture states, in two modes and with two outcomes, giving 12 attempts per strategy:

- watch mode: snapshot plus `ReadDirectoryChangesW`;
- deny mode: a deny-write ACE for the probe account on the source directory and its files;
- outcomes: success, and an injected failure after the first backup step.

Every attempt runs in a fresh child process on a fresh source. It holds the read-only lock and the write guard (section 5.2). The backup is bounded (MPR-07): a busy stall of 5 s, 100 steps without progress, or a 60 s deadline. A bound with no observed change makes the attempt inconclusive; a bound with an observed change fails it. No attempt was inconclusive.

An attempt passes only if both hold:

- the decision accepts: no event, no overflow, no watch error, an armed watch, and equal snapshots and digests;
- the case is reached: on success, the copy passes `integrity_check` and holds the expected head and its state hash; on failure, the injected failure was reached. For C3, in both outcomes, every copied file's digest also equals its digest under the lock.

A strategy passes only if all 12 attempts pass.

| Strategy | `clean` | `crash` (F4) | `crash-noshm` | Verdict |
|---|---|---|---|---|
| C1a `mode=ro`, no checkpoint on close | watch: creates `-wal` and `-shm`, which are left behind; deny: `unable to open database file` | **pass** (4/4) | watch: creates `-shm`, which is left behind; deny: cannot open | fail (4/12) |
| C1b C1a plus `readonly_shm=1` | as C1a; on success the backup also hits the no-progress bound (100 steps without progress) | **pass** (4/4) | as C1a | fail (4/12) |
| C1c C1a plus `locking_mode=EXCLUSIVE` | `disk I/O error` on open; source unchanged | same | same | fail (0/12) |
| C1d C1a plus `SQLITE_FCNTL_PERSIST_WAL=1` (C API) | as C1a | **pass** (4/4) | as C1a | fail (4/12) |
| C2a `mode=ro`, `nolock=1` under the importer's lock | `unable to open database file`; source unchanged | same | same | fail (0/12) |
| C2b C2a plus `readonly_shm=1` | same | same | same | fail (0/12) |
| C3 locked byte copy: `session.sqlite3` and `-wal` read through handles that share read only, under the lock and the guard; SQLite opens only the copy | **pass** (4/4); one file copied | **pass** (4/4); database and `-wal` copied | **pass** (4/4); database and `-wal` copied | **pass (12/12), chosen** |

Result for candidates 1 and 2: **none passes.** The pattern matches SQLite's documented rule for read-only WAL databases: such a database opens only if `-wal` and `-shm` already exist, or can be created, or the database is opened as immutable. With both sidecars present (`crash`), C1a, C1b and C1d copy the WAL-only commit and leave every byte unchanged. When a sidecar is missing (`clean`, `crash-noshm`), they create it in the source directory, which breaks invariant 3. Under the deny ACE they cannot create it and fail visibly, preserving the source but copying nothing. `nolock=1` and exclusive locking do not open the WAL database at all. Every failed open left the source unchanged.

Result for C3: **pass; C3 is chosen.** In all 12 attempts there was no event, no overflow and no watch error, and snapshots and digests were equal, in watch mode and under the deny ACE. The digest of every copied file equals its digest under the lock. Every successful copy passes `integrity_check` and holds the expected head and its state hash, so the WAL-only commit of `crash` and `crash-noshm` is recovered in the copy, without `-shm`. The injected failure was reached in every failure attempt, and its partial copy was removed from the importer's own directory.

Not tested: `immutable=1`, which stays excluded.

### 5.2 Controls

| Control | Result | Evidence |
|---|---|---|
| Negative: a deliberate read-write open that creates and removes `-wal` and `-shm` | pass | the watch reports `-shm` added, modified and removed; rejected (`event`, `snapshot-diff`) |
| Overflow: 64-byte notification buffer, a burst of 300 changes, timestamps restored | pass | 895 zero-byte successful completions (last error 1022, `ERROR_NOTIFY_ENUM_DIR`); snapshots equal; rejected (`overflow`) |
| Mapped write, unguarded (recorded, no pass rule) | detection limit | one byte of an existing `-shm` changed through a writable mapping and restored: no watch event, equal snapshot, equal digest, so the decision accepts. A mapped write is invisible to `ReadDirectoryChangesW` and to metadata and digest checks |
| Mapped write, with the write guard | pass | the writable open is refused (sharing violation); accepted with nothing written |
| Deny ACE prevention | pass | creating a file and opening an existing file for writing are both denied; the ACE was removed afterwards |
| External append (MPR-06): during a C3 `crash` backup, another process runs 0.4's `SessionLock` | pass | that process is refused (exit 3); the import is rejected (`event`, `snapshot-diff`, `digest-diff` on `engine.lock`) with no timing input and no writer attribution |
| Forwarding: the fixture worker is started through the venv launcher | pass | the bound worker (by its own reported PID) was terminated and its exit confirmed; the launcher exited by itself |

The write guard (MPR-04): from before the importer takes the lock until after it releases it, the importer holds `GENERIC_READ` handles that share only reads on every existing source file except `engine.lock`. Every writable open by anyone then fails with a sharing violation. This includes SQLite's own `-shm` open, which then falls back to read-only, as SQLite's Windows VFS does. The guard is compatible with SQLite's read-only opens, with C3's reads and with the digest reads. It closes the mapped-write gap that the watch cannot see. It cannot stop the creation of a new file, which is why candidates 1 and 2 still fail on `clean` and `crash-noshm`. The guard ran in every P1 attempt and every pipeline fixture.

### 5.3 The owner's decision

With candidates 1 and 2 only, invariant 2 as first written (online backup under the lock; no file copy, no `immutable=1`) and invariant 3 (the source directory is never modified, not even transiently) cannot both hold on Windows for a session without `-shm`. A clean 0.4 shutdown always leaves a session in that state. The owner was asked once (workbench ask `20261003T001232Z-b3044edd`) to choose:

- (b), recommended: under the lock and the write guard, copy the bytes of `session.sqlite3` and `-wal` through read-only handles into the 1.0 temporary directory, and let SQLite recover the WAL in the copy only. This relaxes "no file copy" in invariant 2 and keeps invariant 3.
- (a): allow SQLite to create `-wal` and `-shm` in the source under the lock and leave them there. This relaxes invariant 3.
- (c): use `immutable=1` when no `-wal` exists, and refuse a crashed session without `-shm`. This relaxes "no `immutable=1`" and does not cover every state.

The owner chose **(b)** on 3 October 2026 ([owner-decisions-2026-10-03-migration](../../wiki/decisions/owner-decisions-2026-10-03-migration.md)); the proposal's invariant 2 and section 4.2 now carry it. P1 was then re-run with (b) as C3 (section 5.1), and C3 passes.

## 6. P2: publication

`tools/migration_probes/p2_publish.py` implements the publication contract. It was implemented by Codex (Sol) under the plan-check rules MPR-01, MPR-02 and MPR-05, and integrated and tightened by Claude. The steps:

- S0: create missing ancestors, then flush `parent(dest)`, `dest` and `generations/` on every call.
- S1: create the generation directory.
- S2: write the files; S3: flush them.
- S4: write and flush `MANIFEST.json`.
- S5: flush the generation directory and `generations/`.
- S6: write and flush `ACTIVE.tmp`.
- S7: `MoveFileExW(MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)` onto `ACTIVE`. This is the commit point.
- S8: flush `dest`.

`recover` accepts a pointer only if its generation's manifest digest and every listed file's size and SHA-256 match. It also removes a stale `ACTIVE.tmp`.

### 6.1 Loss of buffered writes (test double, headless)

`python tests/test_migration_p2.py`: 20 tests pass. The double treats every directory entry as volatile until its parent directory is flushed (the S7 move makes only its own two entries durable), and every file's data as volatile until the file is flushed. `power_loss()` drops everything not yet durable. Covered:

- An interruption at each of the 18 barriers (`before-S0` to `after-S8`). Start states: no destination; an empty destination; a previous generation `A`. Each case runs with and without power loss. Recovery always gives the previous state or the complete new generation, never an invalid pointer, and a retry under a new name publishes.
- A flush failure at each of the 9 flushes before S7: `aborted`, `ACTIVE` unchanged, the same after power loss, and the retry publishes.
- An S8 failure: `switched-durability-unconfirmed`; the new generation is active before and after power loss.
- An S7 failure before the move took effect gives `aborted`. A failure after it took effect publishes after synchronous recovery. A read failure during that recovery gives `publication-unknown`, and recovery must run before a retry.
- MPR-02: a retry on the same live state, after an S0 directory-flush failure, re-establishes every ancestor barrier. A power loss right after its S7 leaves a complete, valid generation.
- Refusals: an existing generation name, and invalid file or generation names.

These are model results. They show that the publisher's order is correct if the barriers below make data and directory entries durable. They do not show that NTFS does so.

### 6.2 Process interruption (actual Windows)

Run `20261003T005400Z` (command in section 1).

Qualification on the run root's NTFS volume:

| Operation | Result |
|---|---|
| `FlushFileBuffers` on a file handle opened with `GENERIC_WRITE` | ok |
| `FlushFileBuffers` on a directory handle opened with `FILE_ADD_FILE \| FILE_ADD_SUBDIRECTORY` and `FILE_FLAG_BACKUP_SEMANTICS` | ok (used) |
| the same with `GENERIC_WRITE` | ok |
| the same with `GENERIC_READ` | refused, Win32 error 5 (access denied) |
| `MoveFileExW(MOVEFILE_REPLACE_EXISTING \| MOVEFILE_WRITE_THROUGH)` replacing an existing pointer | ok, content replaced |

F17, interruptions around publication. Two disposable non-empty generations, `A` and `B`, each of three files of 4, 16 and 64 KiB. Each case runs as follows:

- a worker bound by its own PID publishes `B` and stops at the target barrier;
- the parent terminates the worker through the bound handle and confirms its exit;
- recovery runs in a fresh process;
- a fresh process retries under the name `B2`;
- recovery runs again.

| Start | Barriers | Result |
|---|---|---|
| no previous generation | all 18 | **pass**. Up to `before-S7`, recovery gives "nothing published" (valid). From `after-S7`, it gives `B`, complete and valid. The retry publishes `B2` in every case |
| previous `A` active | all 18 | **pass**. Up to `before-S7`, recovery gives `A`, complete and valid. From `after-S7`, it gives `B`. The retry publishes `B2` in every case |

A case passes only if the recovered generation is exactly the expected one, so "previous or new" is not enough. Expected: the previous generation until the S7 move has returned, and `B` from `after-S7`. All 36 cases pass.

### 6.3 The barrier

The Windows barriers for files and directory entries on NTFS:

- Files: `FlushFileBuffers` on the file's own handle, opened with `GENERIC_WRITE`, after its last write.
- Directory entries: `FlushFileBuffers` on a handle to the parent directory, opened with `FILE_ADD_FILE | FILE_ADD_SUBDIRECTORY` and `FILE_FLAG_BACKUP_SEMANTICS`. A read-only directory handle cannot be flushed.
- The pointer switch: `MoveFileExW` with `MOVEFILE_WRITE_THROUGH`, followed by a flush of the destination directory (S8).

Windows accepts all three on this volume, and the publication order survives every process interruption. That these calls also survive a real power cut is the model's assumption, not a measurement. No power cut was run (section 8). One gap in the model is known and deferred (review finding MPB-04, minor): the double treats the S7 move as durable once it is visible, so it does not cover a move that recovery already sees but NTFS has not yet persisted when S7 raises and S8 then fails.

P2 verdict: **pass** for the publication order and process interruption. Loss of buffered writes is covered by the test double only.

## 7. Harness rules that the importer design must adopt

- **Decision without attribution (MPR-06).** Stage 5 accepts only an armed watch with no event, no overflow and no watch error, plus equal snapshots and digests. The rule is the same in watch and deny mode. It uses no timing input and names no writer. The proposal's stage 5 text, which classifies an event as an importer breach or as another program by when it falls, is superseded for the decision. The user report may only name causes that the harness observed directly, such as a refused 0.4 start.
- **Write guard (MPR-04)** over the whole lock-held interval: taken before the lock and released after it (section 5.2). Stage 5 detects changes of name, size, attributes and timestamps, and byte differences at the digest points. It cannot detect a mapped write that is restored before the final digest. "Never writes, not even transiently" therefore rests on prevention, the guard and the deny-mode run, not on detection.
- **Locked byte copy (C3):** the bytes are read through handles that share read only, under the lock and the guard; the digest of every copied file must equal the digest taken under the lock; SQLite opens only the copy, and `-shm` is never opened.
- **Bounded backup (MPR-07):** a bound with no observed change makes the attempt inconclusive, never accepted. A bound with an observed change fails the attempt.
- **Settled lock observation (P3a)** applies only to the refusal path, never to an accepted import.
- **Run roots:** every probe writes only inside a fresh, marked run root. Formal runs use `work/migration-probes/runs/<run id>`; the unit tests use the system temporary directory, which the owner's task allowed ("a disposable directory under work/ or %TEMP%"). Every child process checks its own paths against the run root before it creates a session or writes a result.

## 8. Not covered

- File systems other than local NTFS: ReFS, FAT, exFAT and network shares.
- Antivirus or indexer interference was not controlled. None was observed in these runs.
- Real power loss. P2 models buffered-write loss with a test double; the Windows runs interrupt processes only.
- Performance, input and long-session behaviour: no claims.

## 9. Review record

- Plan check `20261002T231650Z-67c88b8a` (Astra): MPR-01 to MPR-06 adopted.
- Scoped re-check `20261002T235125Z-b4e2f6c8`: MPR-01, MPR-02, MPR-04 and MPR-07 adopted.
- Full review (Astra, three concurrent shards) of the candidate before C3:
  - `20261003T002637Z-98885834` (P1, P3, pipeline): MPR-A01 (the guard did not cover the whole lock-held interval) and MPR-A02 (child processes did not check their own paths) adopted and fixed; MPR-A03 (run roots under the system temporary directory) rejected with evidence, because the owner's task allows them (section 7).
  - `20261003T002640Z-b531522b` (P2, runner, sanitiser): MPB-01 (file URIs and `//server/share`), MPB-02 (short names and every SID family) and MPB-03 (the merged output was not leak-checked) adopted and fixed; MPB-04 (minor) deferred (section 6.3).
  - `20261003T002642Z-4dcb0fc5` (this document): no blocking finding; minors MPR-C01 to MPR-C03 fixed.
- Scoped verification round 1 (Astra, two shards) of the fixes and of C3:
  - `20261003T004616Z-dd08a29e`: MPR-A01 and MPR-A02 resolved, the MPR-A03 rejection accepted; new MPR-A04 (a C3 injected-failure attempt could pass with a copy that differs from the bytes under the lock) adopted and fixed.
  - `20261003T004618Z-d39948db`: MPB-01 and MPB-02 fixed; MPB-03 still open (JSON escaping of newlines and non-ASCII names hid a path or name from the text check), fixed by also checking every decoded key and string.
- Scoped verification round 2 (Astra, three shards), with no finding:
  - `20261003T005949Z-b0418f30`: MPR-A04 fixed.
  - `20261003T005951Z-77bcee85`: MPB-03 fixed.
  - `20261003T005954Z-69c2d9a9`: this document matches the run and states the owner's decision correctly.
