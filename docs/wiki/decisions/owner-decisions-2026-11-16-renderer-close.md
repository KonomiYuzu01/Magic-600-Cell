---
id: owner-decisions-2026-11-16-renderer-close
type: decision
status: verified
visibility: public
summary: Owner decision of 16 November 2026 - the stage 2.4 renderer line closes in three steps - the remaining measurements are cut to the minimum a sound selection needs, Fable evaluates the renderer selection in place of packet E-2.4-05's Astra review, and the owner signs the selection ADR and the renderer freeze, after which the renderer line ends; the gate is unchanged. The owner signed the same day with W-J option A, and the renderer line is closed.
related: [renderer-candidates, day7-go-no-go-ruling, owner-decisions-2026-10-22, owner-decisions-2026-11-14, owner-decisions-2026-11-15-fable, owner-decisions-2026-11-16-time-saving]
supersedes: []
claims:
  - {id: renderer-close-steps, evidence_kind: decision, checked_at: 2026-11-16}
  - {id: renderer-signed-option-a, evidence_kind: decision, checked_at: 2026-11-16}
---

# Owner decision, 16 November 2026: close the renderer line

The owner gave this decision in chat in Chinese on 16 November 2026. This page records it in English.

## Decision

The owner's words, translated: "Handle the renderer this way: cut the remaining necessary measurements to the
minimum, then have Fable evaluate the renderer choice, then let me sign the freeze, and this line ends."

1. **Measurements cut to the minimum.** Of the stage 2.4 measurements still open, only those that a sound selection
   cannot do without are run. Every other one is deferred with a one-line reason and recorded as a risk. A GPU run
   still happens only with the owner at the machine.
2. **Fable evaluates the selection.** The `fable-solver` subagent (Fable 5.2, effort `high`, one seven-part packet)
   evaluates the choice among the candidates that passed the day-7 go/no-go, in place of the Astra review that packet
   [E-2.4-05](../../progress/1.0/packets/renderer/E-2.4-05-selection.md) names. Its points are leads that the
   integrator checks against the recorded evidence. Fable's view, like any agreement between models, accepts nothing.
3. **The owner signs the freeze.** The integrator drafts the selection ADR, the renderer freeze record and the H-10
   handoff. They stay drafts until the owner signs. Only the owner's signature accepts the ADR and records the freeze
   (`AGENTS.md` "Ask the owner": accepting ADRs).
4. **Then the renderer line ends.** After the signed freeze no further stage 2.4 renderer experiment runs. A later
   change to the frozen renderer or to its features follows chapter 12's PHS.52: its recorded revision and review
   process and the owner's acceptance.

## What stays

- The gate: all 259,800 sticker slots at full detail with the most complex animation, an average of at least 30 fps
  and a p99 frame time of at most 33.3 ms (PresentMon, native resolution, no frame generation) on the owner's RTX 4070
  Laptop GPU, with peak VRAM at most about 7 GB ([renderer-candidates](renderer-candidates.md)). No bar moves. The
  day-7 ruling of 10 October 2026 stands ([day7-go-no-go-ruling](day7-go-no-go-ruling.md)).
- E-2.4-05's acceptance, apart from who reviews: W5 when a preset has arrived, otherwise W3 with a record that W3 was
  used; W-J on the selected candidate ([owner, 22 October 2026](owner-decisions-2026-10-22.md), Q3); peak VRAM
  recorded; among passing candidates, the one the vertical slice can use with the least extra work; a feature that
  alone breaks the gate goes back to the design track with its H-06 cost; if none passes, the owner decides.
- Codex stays at its minimum ([owner, 14 November 2026](owner-decisions-2026-11-14.md)). The selection gets no Codex
  call.
- Frozen text is not edited. Frozen clauses that say no renderer is selected (RPI.60, RM.29) are read together with
  the signed freeze record; their wording follows in a later versioned change.

## Integrator readings (the owner may change them)

- **Fable's role.** `AGENTS.md` lets Fable join on escalation, for a joint attack or for the three proactive LP2
  tasks of [15 November 2026](owner-decisions-2026-11-15-fable.md), and says that other proactive use needs the
  owner. This decision is that authorization, for the renderer selection only.
- **"Necessary".** A measurement is kept only if the gate or E-2.4-05's acceptance cannot be judged without it.
  Measurements that serve only the design line, or only add headroom detail, are deferred.
- **W5.** The design line is on hold until LP2 to LP5 are done, so the presets H-01 to H-04 do not arrive. The
  selection uses W3 and records that, as E-2.4-05 provides. Charter E1, the gate in the shipped look, stays open for
  the vertical slice and for LP3's run of the unchanged gate on the selected renderer (RPI.79).
- **"This line ends".** The stage 2.4 renderer experiment line ends: packets E-2.4-01 to E-2.4-05, with E-2.4-0J's
  item 5 run on the selected candidate. The vertical slice and the living test of stage 2.4 belong to the design track
  and stay open with the design line. LP3's on-screen half may start once the freeze is signed (PHS.24).
- **Superseded, within this scope.** E-2.4-05's review command (an Astra review of the filled packet) and its sentence
  that the selection gets the senior model's review.

## The signature

The owner signed on 16 November 2026, in chat in Chinese. The owner's words, translated: "Sign; choose A." The
signature accepted [ADR-R01](../../architecture/1.0/renderer/ADR-R01-renderer-selection.md): the 1.0 renderer is
S-A2, the S-B Direct3D 12 drawing core as a shared native library hosted by Godot 4.7.2 .NET. It recorded the
[renderer freeze](renderer-freeze.md), approved Godot 4.7.2 .NET and a supported .NET release as product dependencies,
and sent [H-10](../../progress/1.0/renderer-handoff-H-10.md) to the design track. Under option A, W-J on the selected
candidate runs as LP3 acceptance runs at C9 with unchanged bars, and a failing run reopens the selection.

The renderer line is closed: no further stage 2.4 renderer experiment runs. The vertical slice and the living test
stay open with the design line, and LP3's on-screen half may start (PHS.24).

## Where it is recorded

- This page.
- The [selection record](../../progress/1.0/renderer-selection.md): the gap check, the minimal measurement set and
  Fable's evaluation (16 November 2026).
- The signed records: [ADR-R01](../../architecture/1.0/renderer/ADR-R01-renderer-selection.md) (accepted), the
  [renderer freeze record](renderer-freeze.md) and the [H-10 handoff](../../progress/1.0/renderer-handoff-H-10.md).
- The renderer experiment plan (section 6) and the protocol (section 4.6): handoff rows H-09, H-10 and H-11; packet
  E-2.4-05 notes that Fable's evaluation replaced its Astra review.
- `AGENTS.md`, `CLAUDE.md` and the briefing are unchanged by this page; the briefing's renderer lines are updated in a
  later batch.
- Progress: stage 2.4's items for the selection gate and the renderer window are ticked with the pull request that
  records the signature. The vertical slice, the living test and the level 2 identity binding stay open.
