---
id: owner-decisions-2026-11-18
type: decision
status: verified
visibility: public
summary: Owner decisions of 18 November 2026, each the recommended option - five readings of the frozen text that the 17 November attack on the command layer's commit boundary left open (only the solver's action ends a shown preview, the command layer alone admits a run's preview, protection holds inside Jumble, a reopen re-checks protection-moving commits and unconfirmed releases, a run never writes the workspace, the protection policy is never played unattended); the session lock becomes a Windows platform lock with an explicit "open anyway" entry; DXC ships as the run-time compiler for user stages; LP3's on-screen half (C9) starts now as its own line without a Godot source build; charter E3 is 3 s and LX-4's time box is 4 hours.
related: [owner-decisions-2026-11-16-phs52, owner-decisions-2026-11-16-time-saving, owner-decisions-2026-11-16-renderer-close, owner-decisions-2026-11-15-freeze, owner-decisions-2026-11-06, renderer-freeze]
supersedes: []
claims:
  - {id: owner-18nov-boundary-readings, evidence_kind: decision, checked_at: 2026-11-18}
  - {id: owner-18nov-platform-lock, evidence_kind: decision, checked_at: 2026-11-18}
  - {id: owner-18nov-dxc-ships, evidence_kind: decision, checked_at: 2026-11-18}
  - {id: owner-18nov-c9-line, evidence_kind: decision, checked_at: 2026-11-18}
  - {id: owner-18nov-e3-lx4, evidence_kind: decision, checked_at: 2026-11-18}
  - {id: kn5-12-open, evidence_kind: source, path: docs/architecture/1.0/language/architecture/01-kernel.md, line: 569, checked_at: 2026-11-18}
  - {id: one-52-run-in-command, evidence_kind: source, path: docs/architecture/1.0/language/architecture/08-one-state.md, line: 327, checked_at: 2026-11-18}
  - {id: rpi-52-platform-locking, evidence_kind: source, path: docs/architecture/1.0/language/architecture/10-renderer-platform.md, line: 548, checked_at: 2026-11-18}
  - {id: rpi-80-dxc-owner-item, evidence_kind: source, path: docs/architecture/1.0/language/architecture/10-renderer-platform.md, line: 450, checked_at: 2026-11-18}
  - {id: e3-proposal-until-set, evidence_kind: source, path: docs/architecture/1.0/language/architecture/11-route-and-spikes.md, line: 463, checked_at: 2026-11-18}
  - {id: rte-52-time-box-owner, evidence_kind: source, path: docs/architecture/1.0/language/architecture/11-route-and-spikes.md, line: 574, checked_at: 2026-11-18}
  - {id: bnd-71-joint-setting-before-lp6, evidence_kind: source, path: docs/architecture/1.0/language/architecture/07-boundary.md, line: 592, checked_at: 2026-11-18}
  - {id: freeze-left-dxc-and-godot, evidence_kind: source, path: docs/wiki/decisions/renderer-freeze.md, line: 100, checked_at: 2026-11-18}
  - {id: pr237-runtime-rows, evidence_kind: source, path: rust/c600-command/src/layer.rs, line: 1540, checked_at: 2026-11-18}
  - {id: pr237-lock-refusal, evidence_kind: source, path: rust/c600-session/src/disk.rs, line: 111, checked_at: 2026-11-18}
---

# Owner decisions, 18 November 2026: the commit boundary, the session lock, DXC, C9, E3 and LX-4

The integrator put one page of five decisions to the owner in chat in Chinese on 18 November 2026, each with a
recommended option and an estimate of the owner's time, as the time-saving decision of 16 November provides. The owner
answered "all as recommended": every item takes its recommended option. This page records the answers in English.

## 1. Five readings of the frozen text at the commit boundary (PHS.52)

On 17 November the owner asked for a Fable attack on the command layer against the rule that only the solver's own
action commits. Pull request 237 closed its three blocker findings. Seven attack tests stayed ignored, because the
frozen text is silent at the points they test and both readings were open. The owner chose:

1. **A shown preview.** A program cannot replace or cancel a preview that the solver is shown; only the solver's own
   action ends it (KN5.12 and ONE.42 read strictly). ONE.34 is enforced once, in the command layer: a run's preview is
   admitted only inside a command that the solver invoked, not left to each host. A command ID lives in its caller's
   namespace, so one run cannot use up another run's IDs.
2. **Protection inside Jumble.** Protection holds inside Jumble: a round trip through Jumble cannot move a protected
   orbit (KN1.11, JI.5). The alternative, to record the crossing and tell the solver that protection is paused
   (JI.27), is not taken.
3. **What a reopen checks.** A reopen checks two kinds of journal row again: a commit that moved a protected orbit,
   and a protection release without its confirmation. A journal that fails is not opened as an intact, writable
   session. Journal authentication, which would need a format change and a key, is not taken, so a journal edited to
   turn a program's action into the solver's is not detected; this is a known limit. Every product store takes the
   session lock; a store without one is for tests only.
4. **A run and the workspace.** A run never writes the workspace, including a run that a solver command started
   (ONE.52 read narrowly). Pull request 237's draft rule becomes this reading. It is to be revisited only if a
   built-in tool needs it.
5. **The protection policy.** `protection.policy.set` cannot run unattended: it needs the solver's own press with a
   confirmation, like the protection-release rows (EV4.22), which matches the recording rule that a recording never
   commits (LQ11).

The seven ignored attack tests become tests of these readings. The command layer and the session store are changed to
match; no frozen text is edited.

## 2. A session whose `lock` file is lost

Since pull request 237 the engine refuses a session directory that holds a journal but no `lock` file. The owner
chose two parts:

- **A Windows platform lock.** While a session is held, its `lock` file cannot be deleted or renamed (RPI.52 puts file
  locking behind a narrow platform interface). This is the first platform layer in the Rust workspace, a layer-boundary
  change approved here under PHS.52. Builds on other systems keep pull request 237's behaviour.
- **"Open anyway".** The engine gets an explicit entry that refuses while this machine holds the lock and otherwise
  puts the `lock` file back and opens the session. It cannot see another machine or a synchronised folder.

## 3. DXC ships with the product

DXC ships as the run-time compiler for user shader stages (RPI.80, ADR-R01.4): checked code is compiled outside the
frame within its budget, and a failure falls back to the built-in drawing. A pinned version enters the toolchain
allowlist through the reviewed lockfile change, followed by the owner's approval run. This carries out the owner's
6 November answer that users get a shader language (LQ7).

## 4. C9 runs now as its own line

LP3's on-screen half (C9) starts now as its own line beside LP3's offline work and LP2's remaining work: the selected
core's missing functions, a bridge from the Rust engine to the core, the product shell and the measurement tools. No
Godot source build is made for now: the confirmed present is tried through Godot's public API first, and only if that
cannot work does the question return to the owner (renderer freeze record). The owner-machine runs stay one batch, as
decided on 16 November.

## 5. Charter E3 and LX-4's time box

- **Charter E3** is set at 3 s: cold start, with an existing session and runtime loaded, to the first correct frame,
  in each of three cold runs. It was a proposal until now.
- **LX-4's time box** is 4 hours: about 3 hours for its 10,800 samples (RTE.52) plus setting up. It runs on the same
  day as the other owner-machine runs.

## Stated to the owner, not asked

- **LQ2's option set** (what a solver's own tools may do in a solving session) is settled at the joint setting before
  LP6 (BND.71, PHS.38). It holds up nothing now.
- **The exactness form.** LP2 builds only form 1 of LG2.5. This is the integrator's recorded selection (LP2 plan,
  section 3), and the owner did not overrule it. CT-RTE-04's form-2 traces are therefore not applicable, and
  non-applicability is no pass (PHS.63).
- **Word keys** were answered on 17 November (`trace-key/1`); LP3 records that answer.

## What this does not do

No frozen text is edited, no ADR is accepted and no bar moves. Charter E3 is set, not relaxed. A changed format,
layer boundary or renderer feature beyond those named here still follows PHS.52.
