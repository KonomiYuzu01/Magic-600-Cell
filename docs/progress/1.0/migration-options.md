# 1.0 migration path: options (stage 2.0 decision)

Status: **proposal**. The owner chooses the path in the 2.0 charter. The migration schema is a critical path, so the chosen path gets a Codex Astra plan check before any schema or code is written, and the format is frozen at the 2.5 migration-format-freeze gate.

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
| Large histories | yes | yes | no: 0.4 refuses to export a record its import would reject (screening fix A-02) |
| Extra release to build and sign | yes (a 0.4-line tool, which the owner just cancelled) | no | no |
| 1.0 must understand the 0.4 database schema and lock | no | yes; 0.4 is frozen, so the schema is a fixed input | no |
| Replay check uses | the 0.4 engine | the 1.0 engine, already proven equal by the differential oracle | the 1.0 engine |
| Cost in the 20-day window | design only; tool built after stage 2 | design only; importer built after stage 2 | none, but loses checkpoints and large histories |

Recommendation: **B**. It needs no further 0.4-line release, keeps every invariant and the validator, and the user does one step. The package format stays, so the 2.5 freeze and fixtures F1 to F9 apply unchanged; only the producer moves from 0.4.1 to the 1.0 importer (`producer.name` records which).

## Points for the plan check (any option that reads the session directory)

1. **Lock without writing.** `SessionLock` (`session_lock.py:6`) opens `engine.lock` with `a+b` and writes one byte before locking. If 1.0 used the same code, it would create `engine.lock` when it is absent and update its modification time, which breaks "never modify the original session directory". The importer must lock an existing `engine.lock` without writing, and refuse (not create) when the file is missing and the directory looks like a live session. Byte-range locking of byte 0 (`msvcrt.locking` / `LockFileEx`) is compatible with the 0.4 engine as long as both lock the same range.
2. **Engine running.** If the 0.4 engine holds the lock, refuse with "close Magic 600 Cell 0.4 first"; there is no live export path in 1.0 (exporter Q1 becomes: refuse).
3. **WAL content.** The backup API copies committed WAL-only content (fixture F4). The copy goes to a 1.0-owned temporary directory.
4. **Workspace JSON.** `prefs.layout.magic600_experiment` is parsed by a 1.0 reader with the typed sections of the exporter draft; an unknown or failing section is reported by name and kept in the raw copy, not dropped silently.
5. **Exporter Q2 to Q5** (size limits, the undo tree, the optional raw copy, where the code lives) carry over unchanged.

## Acceptance for this decision

- The owner's choice is written into charter section 6.
- The chosen path passes a Codex Astra plan check with no open `blocker` or `major` before any schema under `schemas/` is added.
