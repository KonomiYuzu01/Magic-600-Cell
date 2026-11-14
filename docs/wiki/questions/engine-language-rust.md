---
id: engine-language-rust
type: question
status: verified
visibility: public
summary: Answered on 14 November 2026 - the 1.0 engine, session store, command layer and the language interpreter's host are written in Rust (LQ8), after the decisive LX-5 and LX-6 runs on the owner's machine; the GUI language still follows the renderer selection.
related: [owner-decisions-2026-09-29, renderer-candidates, owner-decisions-2026-11-14-language-route]
supersedes: []
claims:
  - {id: rust-only-dropped, evidence_kind: decision, checked_at: 2026-10-07}
  - {id: engine-proven-by-oracle, evidence_kind: source, path: docs/progress/1.0/charter-2.0-draft.md, checked_at: 2026-10-07}
  - {id: lq8-answered-rust, evidence_kind: decision, checked_at: 2026-11-14}
---

# Engine language: Rust for the platform-independent layers?

Status: **answered.** On 14 November 2026 the owner chose Rust for the engine, the session store, the command layer and
the interpreter's host of the project's language (LQ8;
[owner-decisions-2026-11-14-language-route](../decisions/owner-decisions-2026-11-14-language-route.md)). On 7 October
2026 the owner had asked whether Rust suits 1.0 and asked for the proposal to be recorded without starting the work.

## Where it stands

- 0.4 runs a Python and NumPy engine (`core.py`, `session.py`, `server.py`) behind a C# native host. 1.0 does not inherit that runtime ([owner-decisions-2026-09-29](../decisions/owner-decisions-2026-09-29.md)).
- The 1.0 charter leaves the engine language open: the engine is re-implemented or wrapped and proven equal to 0.4 by the differential oracle (charter section 4).
- The Rust-only boundary was dropped on 29 September 2026 so that Godot and Blender fit; Rust itself was not ruled out.
- The GUI language follows the stage 2.4 renderer selection: C# for Godot, C++ for Qt, and C++ or Rust for the bare Direct3D 12 path ([renderer-candidates](../decisions/renderer-candidates.md)).
- Since the owner's decision of 3 November 2026 the engineering line develops the project's own language first, and this question became owner question LQ8 of its [architecture document](../../architecture/1.0/language/architecture/11-route-and-spikes.md). Spikes LX-5 and LX-6 fed LQ8, and their decisive runs took place on the owner's machine on 14 November 2026.
- LX-6 ported the hottest path to Rust: with equal output digests, the port was 2.3, 5.1 and 7.2 times faster than NumPy on three metrics, and NumPy was 15.2 times faster on the net effect, where it uses a warmed star cache that the port lacks. The port adds that cache before any net-effect comparison.
- LP2 chooses the big-integer implementation on LX-5's data, and a product dependency needs the owner's approval. The Linux legs, whole-engine correctness and the oracle proof of the full engine remain owed.

## Proposal, as recorded on 7 October 2026

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

## Evidence that decided it

The proposal asked for a short spike: port the hottest path (apply a permutation, compose macros) to Rust, prove it equal to 0.4 with the differential oracle, and compare its speed with the NumPy version on the same machine. LX-6 did that for its frozen inputs, and LX-5 compared exact-arithmetic candidates. The owner took the decision; the integrator's [route chapter](../../architecture/1.0/language/architecture/11-route-and-spikes.md) records it as RTE.17.
