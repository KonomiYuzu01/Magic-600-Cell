# `.c600migrate` exporter: specification draft (0.4.1 step 3)

Status: **draft for a Codex Astra plan check**. Nothing here is accepted. The migration schema is a critical path (`AGENT_BRIEFING.md` section 3), so this file proposes. It adds nothing under `schemas/` and changes no code. Section 7 is the plan-check packet.

Binding inputs:
- [owner-decisions-2026-09-29](../../wiki/decisions/owner-decisions-2026-09-29.md): one-way migration; the 0.4.1 exporter writes a neutral, versioned `.c600migrate` package, and 1.0 verifies it and imports it.
- The exporter never modifies the original session directory.
- It copies the database under the session lock and never opens a user database with `immutable=1`.
- `HUMAN_GUIDE.md`: undo history stops at the migration point, and 0.4.1 can stay installed alongside 1.0.

## 1. What a 0.4 session contains (source inspection)

The session directory is created by `session.py`. In development it is `work/experiments/magic600-04/sessions/<mode>-<name>/`. In the package it sits under the launcher data directory (`launcher.py:186`).

| Item | Where | Migrate? |
|---|---|---|
| `session.sqlite3`: `meta` table | `session.py:16` | yes. Keys: `model`, `head`, `prefs`, `redo:<id>`, `event:<id>`, `magic600_session_attempt_v1` |
| `session.sqlite3`: `events` tree (id, parent, recipe, pre/post hash, certificate, counts, created, note, assistance) | `session.py:17` | yes: the chain from root to `head` becomes the neutral history; the full tree goes only into the raw copy |
| `session.sqlite3`: `snapshots` (checkpoints: name, head, zlib labels, hash, prefs) | `session.py:18` | yes |
| `prefs.layout.magic600_experiment`: the workspace, including `personal_macros`, `keybinds`, `templates`, `captures`, `grip_frames`, `saved_filters`, `bank_overrides`, `contexts` and drafts | `adapter.py:1258` | yes, as typed sections (section 2) |
| `-wal` / `-shm` files | SQLite WAL mode (`session.py:15`) | never copied as files; their committed content reaches the copy through the backup API |
| `logs/*.c600.json.gz`, `logs/*.log` (files the user saved) | `session.py:372` | optional, off by default; if included, copied byte for byte with their digests |
| `backup_*.sqlite3` | `session.py:381` | no |
| `engine.lock`, `launch.json`, `native_keys.json`, `engine.log`, diagnostics, `native-host-*` cache | runtime | no, never |

## 2. Package layout

A `.c600migrate` file is a ZIP with UTF-8 names, no absolute or `..` paths, no links and no duplicate names. Entries are listed in the manifest order.

```
manifest.json                  format, versions, source identity, entry digests
session/record.json            neutral logical record (the import source for 1.0)
session/checkpoints/<n>.labels little-endian int32 labels, uncompressed, one per checkpoint and one for the current state
workspace/macros.json          Macro Base entries in the existing exchange format (macro_library.py:259)
workspace/keymap.json          bindings in the existing keymap format (keymap_catalog.py:266)
workspace/workspace.json       remaining workspace sections, typed and versioned
raw/session.sqlite3            consistent copy of the database (evidence and fallback only; 1.0 does not import from it)
logs/<name>                    optional user-saved logs
```

Proposed `manifest.json` fields (a draft for the schema file that will later be added under `schemas/`):

| Field | Meaning |
|---|---|
| `format` | `"c600migrate"` |
| `format_version` | integer, starting at `1`; readers reject unknown major versions |
| `producer` | `{name: "Magic 600 Cell", version: "0.4.1", source_identity: <build id from step 1>}` |
| `model_id` | must equal the manifest model identity; mismatch blocks export |
| `created_utc` | ISO 8601 |
| `head_state_hash`, `head_event_id` | current state at export |
| `entries` | `[{path, bytes, sha256, role}]` for every entry except `manifest.json` |
| `options` | `{include_logs: bool}` |
| `privacy` | fixed statement: no absolute paths, host names, user names or machine diagnostics inside the package |

`session/record.json` extends the existing `C600-STUDIO-SESSION-v1` export (`session.py:331`). It holds:
- the root-to-head event chain (recipe, pre, post, counts, note, assistance, created);
- the checkpoints (name, head event, state hash, label file digest, prefs);
- preferences without the workspace;
- the attempt/timer record.

Undo branches outside the head chain are not carried forward.

## 3. Export procedure

1. Resolve the session directory from an explicit argument. Refuse a directory inside the application folder and any path the user did not name.
2. Acquire the session lock (`SessionLock`). If the engine holds it, either run the export as an engine command on the live connection, or refuse with "close the session first". **Open question Q1**: which of the two, or both.
3. Copy the database with the SQLite online backup API from a normal read connection (`sqlite3.connect(..., uri=False)`; never `immutable=1`, never a file copy). Write the copy to a temporary directory outside the session directory.
4. Release the lock. Everything that follows reads only the copy.
5. On the copy: `PRAGMA integrity_check` must be `ok`, and `meta.model` must equal the model identity.
6. Replay the head chain from the solved root with `core.py`. Every event's `post` hash must match, and the final hash must equal `head_state_hash`. Recompute each checkpoint's labels hash.
7. Validate the workspace sections with the existing validators (macro exchange, keymap check). If a section fails, stop the export, name the section, and write nothing.
8. Write the ZIP to `<target>.partial`, `fsync` it, verify it with the validator (section 4), then rename it atomically to `<target>.c600migrate`.
9. Report: path, bytes, package sha256, head hash and entry count. Never print absolute paths into the package itself.

Invariant: the byte digests of every file in the original session directory are the same before and after the export (fixture F4).

## 4. Validator checks (shared by the 0.4.1 exporter and the 1.0 importer)

1. ZIP structure: allowed names only, no traversal, links, duplicates or encrypted entries. Total and per-entry size limits (**Q2**: the limits).
2. `manifest.json` is schema-valid; `format` and `format_version` are supported.
3. Every entry is listed with a matching size and sha256, and no entry is unlisted.
4. `model_id` equals the importer's model identity.
5. `record.json` is schema-valid. Replaying the chain reproduces every `post` hash and `head_state_hash`.
6. Each checkpoint label file has 259,800 int32 values and its hash matches.
7. The macro and keymap documents pass their existing format checks for this `model_id`.
8. `raw/session.sqlite3` opens read-only and passes `integrity_check`. Its `meta.head` and `meta.model` agree with the manifest.
9. Privacy scan: no absolute paths (`C:\`, `/home/`, `/Users/`), user-profile names or `engine.log` content in any JSON entry.

A failure produces one named error. The validator never repairs anything.

## 5. Fixtures (fresh synthetic sessions only)

| ID | Fixture | Expectation |
|---|---|---|
| F1 | Solved root only | export and validate pass; empty chain |
| F2 | Scrambled session with about 50 committed events, 2 checkpoints, an undo branch | chain and checkpoints pass; the branch is absent from `record.json` and present in `raw/` |
| F3 | Session with personal macros, custom keybinds and saved filters | workspace sections round-trip through the existing formats |
| F4 | Uncommitted WAL: a commit still only in `-wal` at export time | the commit is present in the copy; original directory digests unchanged |
| F5 | Engine running (lock held) | the Q1 behaviour: live export succeeds, or a clear refusal with no partial file |
| F6 | Corrupted copy (flipped byte in `raw/`), wrong `model_id`, extra ZIP entry, `../` name, oversized entry | each rejected with its named error |
| F7 | Tampered event `post` hash in `record.json` | replay mismatch rejected |
| F8 | `include_logs` on and off | logs present only when requested, digests match |
| F9 | Export interrupted before the rename | no `.c600migrate` file; the original is unchanged |

Checks after implementation: `tests/test_core.py`, `tests/test_reference_maps.py`, `tests/test_crash.py`, plus a new `tests/test_migrate_export.py` covering F1 to F9.

## 6. Open questions for the owner or Astra

- Q1: lock policy, as described in section 3 step 2.
- Q2: size limits.
- Q3: whether the full undo tree should also be neutral data, beyond the raw copy. The current proposal follows the HUMAN_GUIDE statement that history stops at the migration point.
- Q4: whether `raw/session.sqlite3` should be optional, for privacy and size.
- Q5: where the exporter lives: a new root module (it would be critical because it reads the session), or in `work/experiments/magic600-04/`.

## 7. Plan-check packet (for the local session)

Run it with `python tools/agents/codex_review.py --kind plan --model gpt-6-astra --effort max --packet docs/progress/0.4.1/exporter-spec-draft.md`.

1. **Goal and acceptance.** Approve or correct this exporter plan before implementation. Acceptance: every Q1 to Q5 answered or deferred with a reason, and no `blocker` or `major` left in the package layout, the export procedure or the validator list.
2. **Problem.** 0.4.1 must ship a minimal exporter, migration schema and validator (briefing section 1). 1.0 does not inherit the 0.4 runtime, so the package is the only data bridge.
3. **Environment.** 0.4 source on `main`, CPython 3.12 64-bit, SQLite in WAL mode.
4. **Source and evidence.** `session.py:12-33`, `session.py:331-383`, `session_lock.py`, `work/experiments/magic600-04/adapter.py:1245-1265`, `macro_library.py:250-280`, `keymap_catalog.py:266-300`, and the wiki decisions linked above.
5. **Attempts so far.** None; this is the first draft.
6. **Constraints.**
   - Read-only review; review only, do not perform follow-up work.
   - Never touch a personal session.
   - No `immutable=1`.
   - The original session directory is never modified.
   - Mechanics and model identity are unchanged.
   - Out of scope: the 1.0 importer implementation, UI and release packaging.
7. **Return format.** `schemas/review-result.schema.json`.
