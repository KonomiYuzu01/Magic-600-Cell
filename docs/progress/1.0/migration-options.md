# 1.0 migration path: options (stage 2.0 decision)

Status: **proposal, revised after the Astra plan check of 1 October 2026** (call `20261001T020112Z-dc9304aa`; findings MIG-01 to MIG-06 adopted). Two scoped re-checks followed: `20261001T021256Z-41eb8339` raised MIG-07 and MIG-08 and kept MIG-06 open, all adopted, and `20261001T021627Z-401defb2` passed. No `blocker` or `major` is open. The source-preservation probe (item 3) and the publication probe (item 7) remain prerequisites for building the importer; they are not evidence yet. The owner chooses the path in the 2.0 charter. The migration schema is a critical path, so the chosen path gets a Codex Astra plan check before any schema or code is written, and the format is frozen at the 2.5 migration-format-freeze gate.

## Why this is open again

The 29 September decision had a 0.4.1 exporter write a neutral `.c600migrate` package that 1.0 verifies and imports. With no 0.4.1 release (owner decision, 1 October 2026), that exporter has no release to ship in. The invariants do not change:
- one-way migration; 1.0 never writes 0.4 data;
- copy the database under the session lock, with the SQLite online backup API, never `immutable=1` and never a file copy;
- never modify the original session directory;
- the model identity must match, or the import is refused;
- undo history stops at the migration point.

The [exporter specification draft](../0.4.1/exporter-spec-draft.md) still describes the content, the package layout, the validator checks (section 4) and the fixtures F1 to F9 (section 5). Every option below reuses them.

## Options

| | A. Standalone 0.4 exporter | B. 1.0 importer reads a locked copy (recommended) | C. Manual export from 0.4 |
|---|---|---|---|
| How | A small separate tool, built from the 0.4 source, writes `.c600migrate`; 1.0 validates and imports it | 1.0 takes a locked copy of the 0.4 database, builds the same `.c600migrate` package internally, validates it, imports it and keeps the package as the user's archive | The user uses the existing 0.4 "Export" (`C600-STUDIO-SESSION-v1`, `session.py:331`) and 1.0 imports that file |
| User steps | Download and run a second tool, then import | Pick the 0.4 session folder in 1.0 | Open each session in 0.4, export, then import |
| Carries the workspace (macros, keybinds, filters, templates) | yes, as typed sections | yes, as typed sections | only untyped, inside `prefs` (`session.py:336`) |
| Checkpoints | yes | yes | no: the export carries the head chain only |
| Large histories | yes | yes | uncertain. The released 0.4 encoder has no transaction or export-budget check (screening fix A-02 came after the release), and it encoded a synthetic 10,001-entry record. Whether such a record replays, and what limits a 1.0 importer sets, are separate questions that are not tested. |
| Extra release to build and sign | yes (a 0.4-line tool, which the owner just cancelled) | no | no |
| 1.0 must understand the 0.4 database schema and lock | no | yes; 0.4 is frozen, so the schema is a fixed input | no |
| Replay check uses | the 0.4 engine | the 1.0 engine, already proven equal by the differential oracle | the 1.0 engine |
| Cost in the 20-day window | design only; tool built after stage 2 | design only; importer built after stage 2 | none, but loses checkpoints; large-history support is untested |

Recommendation: **B**. It needs no further 0.4-line release, keeps every invariant and the validator, and the user does one step. The package format stays, and the 2.5 freeze applies. Only the producer moves from 0.4.1 to the 1.0 importer (`producer.name` records which). The fixtures F1 to F9 apply with the extensions in items 3 to 7 below. B depends on the source-preservation probe in item 3 passing.

The importer pipeline, platform boundary, probes and extra fixtures for path B are detailed in the [exporter re-plan proposal](migration-exporter-proposal.md).

## Points for the plan check (any option that reads the session directory)

1. **Lock without writing.** `SessionLock` (`session_lock.py:6`) opens `engine.lock` with `a+b` and writes one byte before locking. If 1.0 used the same code, it would create `engine.lock` when it is absent and update its modification time, which breaks "never modify the original session directory". The importer must lock an existing `engine.lock` without writing, and refuse (not create) when the file is missing and the directory looks like a live session. Byte-range locking of byte 0 (`msvcrt.locking` / `LockFileEx`) is compatible with the 0.4 engine as long as both lock the same range.
2. **Engine running.** If the 0.4 engine holds the lock, refuse with "close Magic 600 Cell 0.4 first"; there is no live export path in 1.0 (exporter Q1 becomes: refuse).
3. **WAL content and an unchanged source.** The backup API copies committed WAL-only content (fixture F4), and the copy goes to a 1.0-owned temporary directory.
   - Locking and the backup API alone do not prove that the source directory stays unchanged. The exporter draft's ordinary read/write connection (`sqlite3.connect(..., uri=False)`) can checkpoint the WAL or remove sidecar files when the last connection closes. Read-only WAL access can create sidecars.
   - Before option B is relied on, a probe on synthetic Windows sessions must establish a connection strategy that leaves the source unchanged, without `immutable=1` and without a file copy. The probe covers three cases: a clean shutdown, a crash with WAL-only content, and a missing `-shm` file. It runs lock acquisition, open, backup and close, on both success and failure paths. It compares directory membership, file bytes and modification times before and after.
   - If no strategy passes, the invariants conflict, and the owner decides. The importer is not built until then.
4. **Workspace JSON.** `prefs.layout.magic600_experiment` is parsed by a 1.0 reader with the typed sections of the exporter draft. Each section has one defined outcome:

   | Section | Raw copy included | Raw copy omitted |
   |---|---|---|
   | valid, known | imported | imported |
   | unknown | not imported; reported by name; kept in the raw copy | not imported; the import stops and nothing is published, unless the user re-runs with the raw copy included |
   | invalid, known | not imported; reported by name; kept in the raw copy | the import stops and nothing is published |
   | protection requirements that fail to convert | the import stops and nothing is published: protection is never weakened | the same |

   The exporter draft's rule that a failing section stops the export is replaced by this table. Fixtures prove two things: unsupported work stays recoverable from the raw copy, and a failed protection conversion cannot yield a successful import.
5. **Every restorable checkpoint is verified.** The exporter draft's validator checks a checkpoint's length and its self-reported hash, and replays only the current head chain. That accepts a checkpoint whose labels were exchanged with another checkpoint's, once its hashes are recomputed.
   - 1.0 verifies every checkpoint it will offer to restore, including those on abandoned branches. It replays from the root along that checkpoint's own path, and the replayed labels must equal the stored labels.
   - Preferences are not part of the event history. `save_prefs` updates only `meta.prefs`, and each checkpoint captures the preferences current at that moment. So a checkpoint's preferences are validated on their own: the schema and the protection requirements. They are preserved as stored and never compared with a replay.
   - A checkpoint whose path cannot be replayed from the stored witnesses is not offered for restore. It is kept only in the raw copy, and the report names it.
   - Fixture F2 gains a checkpoint on an abandoned branch. It also gains two checkpoints with a preference change between them: both must verify, and restoring each must bring back its own preferences.
   - New rejection cases: two checkpoints with exchanged states, and duplicate or out-of-range labels with recomputed hashes.
6. **An empty workspace.** A new 0.4 session has no personal macros, but the existing macro exchange validators (`export_selected`, `inspect_import`) require 1 to 256 entries.
   - In the package, an empty library is an omitted `workspace/macros.json`, read as zero macros. The exchange validators are not used for that case.
   - Fixture F1 is a session with no saved workspace and no personal macros. Its package must validate and import without inventing entries.
7. **Destination publication and recovery.** The import is transactional, as `AGENTS.md` requires for reset and import:
   1. The import writes a new, fresh generation directory under the 1.0 data root. It never writes into an existing directory.
   2. It validates the complete new generation.
   3. It makes the archive package durable inside that generation.
   4. It switches a small pointer file that names the active generation. The new pointer is written to a temporary file, flushed, and moved over the old pointer with `MoveFileExW(MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)`.
   - A directory cannot be atomically replaced on Windows (`MoveFileExW` refuses an existing destination directory). Only the pointer file is replaced.
   - Until the switch, the previous generation is active and unchanged. After it, the active generation is a complete validated import with its archive.
   - The previous generation is kept until the new one has been opened successfully once. Recovery removes an unreferenced generation only after the switch has completed.
   - Before the importer is built, a probe tests this exact primitive on Windows with two disposable non-empty generations, through success, an interruption and a retry.
   - Fault-injection cases cover an interruption during the generation writes, between archive creation and the switch, and around the switch. They run both with no previous destination and with an existing one. After each, recovery leaves either the previous active generation or the complete new one, and a retry succeeds.
   - These cases join F1 to F9. F9 alone (an interruption before the archive rename) is not enough.
8. **Exporter Q2 to Q5** (size limits, the undo tree, the optional raw copy, where the code lives) carry over, except that item 4 settles when the raw copy is mandatory.

## Acceptance for this decision

- The owner's choice is written into charter section 6.
- The chosen path passes a Codex Astra plan check with no open `blocker` or `major` before any schema under `schemas/` is added.
