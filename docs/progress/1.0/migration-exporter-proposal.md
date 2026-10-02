# 1.0 migration: exporter re-plan (design proposal)

Status: **design proposal, not an ADR and not a format freeze.** It re-plans the exporter vehicle after the owner cancelled the 0.4.1 release (owner decision, 1 October 2026). The owner chose path B of [migration-options](migration-options.md) on 2 October 2026 (charter section 6); this page is its design. The migration schema is a critical path (`AGENT_BRIEFING.md` section 3): nothing under `schemas/` and no code is written until the owner has chosen a path and a Codex Astra plan check of that path has no open `blocker` or `major`. The package format is frozen only at the stage 2.5 migration-format-freeze gate (Astra ruling); until then every format name on this page carries a `draft` marker.

Review record: Astra plan check `20261002T192706Z-cd51f338` (MEXP-01 to MEXP-04 adopted), scoped re-checks `20261002T193715Z-36968ebd` (MEXP-01 follow-up adopted) and `20261002T194011Z-2188132b` (MEXP-05 adopted: the pre-lock snapshot reads metadata only). The MEXP-05 fix was not re-checked: the round limit was reached and the owner approved committing this draft under an exception (2 October 2026). The next Astra plan check, after the owner's path choice, covers it.

## 1. Invariants (binding, unchanged)

1. **One way.** 1.0 imports 0.4 data. 1.0 never writes a 0.4 session, never writes a 0.4 format and offers no export back to 0.4.
2. **Copy under the session lock.** The database is copied while the importer holds the session lock, with the SQLite online backup API. No file copy, no `immutable=1` (it misses WAL-only commits), no read of the live database outside the lock.
3. **The original session directory is never modified by 1.0.** The importer creates, writes, renames, truncates or deletes nothing there, not even transiently, and changes no modification time; this includes `engine.lock` and the SQLite sidecar files (`-wal`, `-shm`). A change made during the import by another program (section 4.1) is detected and makes the import fail; it is reported, never hidden.
4. **Model identity.** The session's stored `model` must equal the 1.0 model identity, or the import is refused.
5. **History stops at the migration point.** 1.0 starts from the imported head; the 0.4 event chain and checkpoints are verified and archived, not continued as live 0.4 history.

## 2. What changes from the 0.4.1 exporter draft

| | 0.4.1 exporter draft | This proposal (path B) |
|---|---|---|
| Producer | a 0.4.1 release tool | the 1.0 importer, in-process |
| Package | `.c600migrate` | the same layout, built in a 1.0-owned temporary directory and kept as the user's archive |
| Lock | `SessionLock` (writes one byte) | a read-only lock of the existing `engine.lock` byte 0 (section 4.1) |
| Engine running | live export (Q1) | refused: "close Magic 600 Cell 0.4 first" |
| Replay check | 0.4 engine | 1.0 engine, after the differential oracle has shown it equal to 0.4 `core.py` on the same inputs |
| Failing workspace section | stops the export | the per-section table of migration-options item 4 |

The content list, the package layout, the validator checks and fixtures F1 to F9 of the [exporter specification draft](../0.4.1/exporter-spec-draft.md) are reused, with the extensions in section 6.

## 3. Pipeline

Each stage either completes or leaves the source unchanged and the destination at its previous active generation.

1. **Discover.** The user picks a 0.4 session folder and, optionally, saved log files to include (`include_logs`). The importer checks that `session.sqlite3` and `engine.lock` exist and that no 1.0 import of the same source digest is already active (re-import is allowed and creates a new generation). It asks the user to keep 0.4 closed during the import.
2. **Watch and snapshot the directory, before locking.** Start a change watch on the directory (`ReadDirectoryChangesW`; `inotify` in the POSIX smoke tests), then record membership, sizes and modification times of every entry and of the directory itself. This snapshot reads metadata only: no file content is read before the lock is held. The watch stays armed from before the lock is taken until after it is released (stage 5), so a file that is created and removed again, or a write by another program at any moment of the lock-held interval, is seen; an end-state comparison alone cannot see it.
3. **Lock.** Acquire the read-only lock of section 4.1. If the lock is held, refuse with the "close 0.4 first" message. Once the lock is held, add the byte digests of every entry to the snapshot. If `engine.lock` is missing, refuse; never create it.
4. **Copy.** Open the source with the connection strategy chosen by probe P1 (section 5) and run the online backup into `<1.0 temp>/<import id>/copy.sqlite3`. Close the source connection while still holding the lock. When logs are included, copy each selected log file into `<import id>/logs/` through a read-only handle and record its digest; after the lock is released no stage reads the source again.
5. **Verify the source is unchanged.** Repeat the snapshot, release the lock, then stop the watch and drain every pending event. Any watch event or any difference aborts the import and is reported: as an importer invariant breach when the event falls inside the importer's own operations, otherwise as a change by another program (for example a 0.4 start, section 4.1). Only a drained, empty event list and an equal snapshot accept the source.
6. **Build the package** from the staged copies only (`copy.sqlite3` and `logs/`; never from the source): record, checkpoints, typed workspace sections and the optional raw copy, as in the exporter draft section 3.
7. **Validate** the package with the validator list (exporter draft section 4, migration-options items 4 to 6).
8. **Replay** the head chain and every offered checkpoint's own path with the 1.0 engine; the replayed labels must equal the stored labels (migration-options item 5).
9. **Publish** a new destination generation and switch the pointer (migration-options item 7).
10. **Report** what was imported, what is kept only in the raw copy, and what was refused, by name.

## 4. Platform boundary

Windows comes first, but the importer's logic must stay free of Windows-only code (owner decision, 2 October 2026). Three narrow platform operations carry the Windows specifics; everything else is portable and runs headless on Linux against synthetic sessions.

### 4.1 `lock_session_readonly(dir)`
- Opens the existing `engine.lock` without write access and without the create flag, and takes a non-blocking lock of byte 0, the range the 0.4 engine locks (`session_lock.py:6-10`, `msvcrt.locking(..., 1)` after `seek(0)`).
- Windows: `CreateFileW(GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE, OPEN_EXISTING)` and `LockFileEx(LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY)` on offset 0, length 1. Whether an exclusive byte-range lock on a read-only handle conflicts with the 0.4 engine's `msvcrt.locking` exactly as needed is a probe question (P3), not an assumption.
- POSIX (headless tests only): `flock` on a read-only descriptor, which matches the 0.4 POSIX branch.
- **A refused 0.4 start still writes.** `SessionLock` opens `engine.lock` in append mode and writes one byte before it tries the lock (`session_lock.py:6`). If the user starts 0.4 while the importer holds the lock, 0.4 is refused but has already appended a byte. The importer cannot prevent this write by another program. It detects it (stage 5), fails the import, reports "0.4 was started during the import; close it and retry", and publishes nothing. Probe P3 runs this case first, before the broader probes, at the barriers of fixture F20.

### 4.2 `open_source_for_backup(path)`
- Returns a SQLite connection that sees committed WAL-only content and leaves every byte of the directory unchanged on open, backup and close, on success and on failure. Candidates for probe P1, in order of preference:
  1. `file:...?mode=ro` URI with the WAL present, the lock held, and no checkpoint on close (`PRAGMA` settings and `SQLITE_FCNTL_PERSIST_WAL` to be tested);
  2. a read-only connection with `nolock=1` while the importer's own lock excludes the 0.4 engine;
  3. none passes: the invariants conflict and the owner decides (migration-options item 3). The importer is not built before this is settled.
- `immutable=1` and file copies are not candidates.

### 4.3 `publish_generation(root, generation)`
- Writes the pointer file through a temporary file, flush and `MoveFileExW(MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)`; POSIX uses `os.replace` plus a directory `fsync`. Probe P2 tests the Windows primitive.

## 5. Probes (prerequisites, owner's Windows machine)

The probes use fresh synthetic sessions only, created by the 0.4 engine in a disposable directory; never a personal session. A cloud session can write the probe scripts and run their POSIX branch, but only the Windows run is evidence.

| Probe | Question | Pass |
|---|---|---|
| P1 source preservation | Which `open_source_for_backup` strategy copies WAL-only commits and never writes the source, not even transiently? | For a clean shutdown, a crash with WAL-only content and a missing `-shm`, on success and on an injected failure, through lock, open, backup, close and failure cleanup: the copy contains the WAL-only commit, and the change watch records no event. Each case runs twice: once with the snapshot and watch, once on a write-denied source directory (a deny-write ACE for the probe account), where any write attempt fails visibly. Negative control: a deliberate open that creates and removes `-shm` must be reported by the watch |
| P2 publication | Does the pointer switch survive interruption? | With two disposable non-empty generations, an interruption at each step and a retry leave either the previous or the complete new generation active |
| P3 lock compatibility | Do the read-only lock and the 0.4 engine exclude each other, and does the importer itself write nothing? | Run first, with a disposable non-empty `engine.lock` and two processes. 0.4 running: the importer is refused and `engine.lock` is unchanged. Importer holding the lock: the 0.4 start is refused with its own message, its appended byte is recorded as expected 0.4 behaviour, and the importer's stage 5 detects it and fails the import |

## 6. Fixtures

F1 to F9 of the exporter draft, plus:

| ID | Fixture | Expectation |
|---|---|---|
| F10 | Checkpoint on an abandoned branch; two checkpoints with a preference change between them | both verified by their own replay path; each restores its own preferences |
| F11 | Two checkpoints with exchanged labels and recomputed hashes; duplicate or out-of-range labels | rejected by name |
| F12 | New session: no saved workspace, no personal macros | validates and imports with zero macros; `workspace/macros.json` absent |
| F13 | Unknown and invalid workspace sections, raw copy on and off | outcomes of migration-options item 4 |
| F14 | Protection requirement that fails to convert | import stops, nothing published |
| F15 | Directory snapshot difference injected between stages 2 and 5 | import aborts as an invariant breach; destination unchanged |
| F16 | Model identity mismatch | refused before any copy |
| F17 | Interruptions around publication, with and without a previous generation | recovery leaves the previous or the complete new generation; retry succeeds |
| F18 | Transient sidecar: a test double creates and removes `-shm` in the source during stage 4 | the watch reports it; the import fails; nothing is published |
| F19 | Scrambled session with many events imported into a fresh destination | Undo is unavailable at the imported head; after one 1.0 move, Undo returns exactly to the imported head and stops; the same after reopening; later 1.0 edits leave the source directory unchanged |
| F20 | 0.4 started while the importer holds the lock, at barriers right after lock acquisition, between stages 4 and 5, and right after `engine.lock` is read in the final snapshot | at every barrier: 0.4 refused; the watch reports the appended byte; the import fails and publishes nothing |
| F8 (extended) | A selected external log whose bytes are not in SQLite | the package holds its exact bytes from `logs/`; the source is not reopened after the lock is released |

F4 (WAL-only commit), F15, F18 and F20 run on Windows for evidence; their POSIX form is a headless smoke test.

## 7. Relation to the differential oracle

The replay in stage 8 uses the 1.0 engine. It is trusted only for engine builds that pass the differential oracle against 0.4 `core.py` on the oracle's fixtures (protocol section 3). A package whose replay differs is refused; the importer never falls back to trusting the stored labels.

## 8. Format status

- Working names: package `.c600migrate`, format `C600-MIGRATE-draft`, producer `magic600-1.0-importer`. They are not final.
- No file under `schemas/` is added by this proposal. A JSON schema is drafted only after the owner's path choice and the Astra plan check, and frozen at the 2.5 gate with the probe results attached.

## 9. Open questions

- Owner or plan check: exporter Q2 (size limits), Q3 (the full undo tree as neutral data), Q4 (raw copy optional; migration-options item 4 makes it mandatory when a section is unknown or invalid), Q5 (where the importer code lives in the 1.0 layering).
- Probe outcome: if P1 finds no strategy, the owner decides between the conflicting invariants.

## 10. Out of scope

- Importer code, the migration schema, UI and release packaging.
- Any change to 0.4 code, `session.py`, `session_lock.py` or the 0.4 database.
- Running anything against a personal session.
