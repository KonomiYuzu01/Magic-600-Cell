---
id: owner-decisions-2026-10-02-migration
type: decision
status: verified
visibility: public
summary: Owner decision of 2 October 2026 - the 1.0 migration path is option B, a 1.0 importer reading a locked copy of the 0.4 database, subject to an Astra plan check and the Windows probes P1 to P3.
related: [owner-decisions-2026-10-01, owner-decisions-2026-09-29]
supersedes: []
claims:
  - {id: path-b, evidence_kind: decision, checked_at: 2026-10-02}
  - {id: options-page, evidence_kind: source, path: docs/progress/1.0/migration-options.md, checked_at: 2026-10-02}
  - {id: design-page, evidence_kind: source, path: docs/progress/1.0/migration-exporter-proposal.md, checked_at: 2026-10-02}
---

# Owner decision, 2 October 2026: migration path B

With no 0.4.1 release ([1 October](owner-decisions-2026-10-01.md)), the exporter vehicle of [29 September](owner-decisions-2026-09-29.md) had no release to ship in. The owner chose **option B** of [migration-options](../../progress/1.0/migration-options.md): the 1.0 importer takes a locked, read-only copy of the 0.4 database, builds the same migration package internally, validates and replays it with the 1.0 engine, imports it and keeps the package as the user's archive. The user picks the 0.4 session folder in 1.0; no further 0.4-line release is built.

- The invariants are unchanged: one-way migration; copy under the session lock with the SQLite online backup API, never `immutable=1` and never a file copy; the original session directory is never modified; the model identity must match; history stops at the migration point.
- The design is [migration-exporter-proposal](../../progress/1.0/migration-exporter-proposal.md) (charter section 6).
- Before any schema under `schemas/` or importer code: a Codex Astra plan check of path B with no open `blocker` or `major`, and the Windows probes P1 (source preservation), P2 (publication) and P3 (lock compatibility) on synthetic sessions, run on the owner's machine. If P1 finds no strategy, the owner decides between the conflicting invariants.
- The Astra plan check of path B (2 October 2026) raised four findings on watch overflow, witnesses for abandoned checkpoints, durability before the pointer switch and the model check before backup; all were adopted into the design and the scoped re-check passed. The Windows probes P1 to P3 remain prerequisites.
- The package format is frozen only at the stage 2.5 migration-format-freeze gate (Astra ruling).
