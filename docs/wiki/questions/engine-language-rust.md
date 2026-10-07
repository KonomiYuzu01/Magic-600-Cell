---
id: engine-language-rust
type: question
status: draft
visibility: public
summary: Open question for the stage 2.5 architecture freeze - write the 1.0 engine, session store and command layer in Rust, with the GUI language following the renderer selection; a deferred Rust spike would supply the evidence.
related: [owner-decisions-2026-09-29, renderer-candidates]
supersedes: []
claims:
  - {id: rust-only-dropped, evidence_kind: decision, checked_at: 2026-10-07}
  - {id: engine-proven-by-oracle, evidence_kind: source, path: docs/progress/1.0/charter-2.0-draft.md, checked_at: 2026-10-07}
---

# Engine language: Rust for the platform-independent layers?

Status: **recorded, not started.** On 7 October 2026 the owner asked whether Rust suits 1.0 and asked for the proposal to be recorded without starting the work.

## Where it stands

- 0.4 runs a Python and NumPy engine (`core.py`, `session.py`, `server.py`) behind a C# native host. 1.0 does not inherit that runtime ([owner-decisions-2026-09-29](../decisions/owner-decisions-2026-09-29.md)).
- The 1.0 charter leaves the engine language open: the engine is re-implemented or wrapped and proven equal to 0.4 by the differential oracle (charter section 4).
- The Rust-only boundary was dropped on 29 September 2026 so that Godot and Blender fit; Rust itself was not ruled out.
- The GUI language follows the stage 2.4 renderer selection: C# for Godot, C++ for Qt, and C++ or Rust for the bare Direct3D 12 path ([renderer-candidates](../decisions/renderer-candidates.md)).

## Proposal

Write the platform-independent layers in Rust: the engine, the session store and the command layer. Keep the GUI in the language of the selected renderer behind a narrow C ABI.

Expected benefits:
- Speed and no garbage-collection pauses for the permutation, macro-composition and protection work over 259,800 sticker slots.
- Memory safety and type-level invariants for preview revisions, state hashes and protected orbits.
- Portable layers for the later macOS and Linux versions; the oracle keeps running headless on Linux.
- PyO3 lets the existing Python tests and tools call the new engine during the transition.

Known costs:
- An extra language boundary between the engine and the GUI.
- Longer compile times.
- A higher reading barrier for the owner than Python.

## Evidence that would decide it (deferred)

A one- or two-day spike in a cloud session: port the hottest path (apply a permutation, compose macros) to Rust, prove it equal to 0.4 with the differential oracle, and compare its speed with the NumPy version on the same headless machine. The result is input to the stage 2.5 architecture freeze. The language choice is a design decision: it gets an Astra review and the owner decides.
