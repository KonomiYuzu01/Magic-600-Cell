---
id: owner-decisions-2026-10-03-migration
type: decision
status: verified
visibility: public
summary: Owner decision of 3 October 2026 - after P1 found no read-only SQLite strategy, the importer copies the database and WAL bytes under the session lock and recovers only the copy (option b); invariant 3 stays whole, and P1 is re-run for this strategy before the importer starts.
related: [owner-decisions-2026-10-02-migration]
supersedes: []
claims:
  - {id: option-b, evidence_kind: decision, checked_at: 2026-10-03}
  - {id: probe-results, evidence_kind: source, path: docs/progress/1.0/migration-probe-results.md, checked_at: 2026-10-03}
  - {id: design-page, evidence_kind: source, path: docs/progress/1.0/migration-exporter-proposal.md, checked_at: 2026-10-03}
---

# Owner decision, 3 October 2026: locked byte copy of the 0.4 database

Probe P1 ([results](../../progress/1.0/migration-probe-results.md), section 5) found no read-only SQLite strategy that leaves a 0.4 session directory unchanged. A session without `-shm` cannot be opened read-only without creating sidecar files in the source, and every cleanly closed session is in that state. Invariant 2 (online backup only, no file copy) and invariant 3 (the source directory is never modified) of the [migration proposal](../../progress/1.0/migration-exporter-proposal.md) could not both hold, so the owner had to choose between them ([path B decision](owner-decisions-2026-10-02-migration.md)).

The owner chose **option (b)**:

- While holding the session lock, and with the write guard refusing every writable open of the source files, the importer reads the bytes of `session.sqlite3` and its `-wal` through read-only handles. It writes them into its own 1.0 temporary directory and lets SQLite recover the WAL in that copy only. The `-shm` file is never opened.
- This relaxes only the "no file copy" part of invariant 2. Everything else in invariant 2 stays in force: the copy is made under the session lock, nothing is read outside the lock, and `immutable=1` is never used. Invariant 3 stays whole: 1.0 never creates, writes or removes anything in the source directory.
- The copied bytes must equal the byte digests taken under the lock, and stage 5 still requires an unchanged source.
- Option (b) was not tested in the first P1 run, because the task's rules excluded file copies. P1 is re-run for it on Windows before any importer code is written. The importer starts only if that run passes. Result, 3 October 2026: it passes as C3 in 12 of 12 attempts ([results](../../progress/1.0/migration-probe-results.md), section 5.1).

Options not taken: (a) let SQLite create `-wal` and `-shm` in the source (relaxes invariant 3); (c) `immutable=1` when no `-wal` exists, and refuse crashed sessions without `-shm`.
