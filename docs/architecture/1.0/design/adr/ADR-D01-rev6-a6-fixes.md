# ADR-D01 revision 6 draft: fixes from the A6 review

Status: **Accepted** with revision 3a (owner, 23 November 2026, at A7; [freeze record](../../../../wiki/decisions/owner-decisions-2026-11-23-a7-freeze.md), [README](README.md)). Written 23 November 2026 by the A6 session. It answers the record findings of the A6 review (call `20261123T000053Z-01a205b4`, gpt-6.2-astra). Where an amendment below and an earlier text (revision 3a, revision 4 or revision 5) differ, this file wins. As before, the earlier files are not edited in place.

## The owner's words (23 November 2026, about 00:45 UTC, translated from Chinese)

> Fix it now, then run one more round (this round reviews only what is needed, without Computer Use). The remaining quota should be enough; work as usual, and tell me if it is not. Then I freeze A7.

Reading: fix the findings now; one more Astra round, a scoped check without Computer Use; then the owner freezes A7.

## 1. Blocking record findings

### R-1. One quantity and one direction for length

- **Rev4, ADR-D01.4 item 7**, the sentence "Its one use at A7 is the rail's residual level (ADR-D01.64)" becomes:
  "At A7 the rule draws one quantity in one direction: an orbit's residual level, where a longer mark means more is left to do. It has two places: the rail cell's residual level (ADR-D01.64) and the ticks of a log's signature (ADR-D01.83). Both compute the level the same way, for the state they show. A solved orbit draws no length and shows the solved mark."
- **Rev5, ADR-D01.83**, "A log's signature" becomes:
  "35 ticks in the rail's order. Each tick is that orbit's residual level in the log's saved state, drawn exactly as the rail draws it, so a short tick means little is left. A solved orbit shows the solved mark instead of a tick. A row of log tabs reads as a shelf of what is left to do."
- Why the signature changes and not the rail: the rail's residual level is the owner's pick (22 November 2026). The signature's form was left to adr-line ("you decide").

### R-2. A log without a verified state shows no state

- **Rev5, ADR-D01.81**, "What a failed log does" becomes:
  "A log whose compile has not passed every gate (format, `model_match`, every move replayed with its legal witness, the final full-state hash) opens read-only with its diagnostics and the reason. Until a compile of its current bytes passes, it has no binding chip, no scrub, no history preview, no signature ticks and no derived quantity. Its tab shows the compile lamp in place of the signature. Nothing is drawn from a partial replay."
- **Rev5, ADR-D01.82**, "Opening" gains: "Only a log whose compile passed opens bound to its saved state."
- **Rev5, ADR-D01.83**, the compile lamp gains:
  "A compile result is bound to the digest of the document's bytes. Any edit returns the lamp to the empty ring (not compiled), and the Problems rows of the old compile dim and name the compile they came from."

### J-6. A repair commits whole

- **ADR-D01.12** gains: "A preview may be whole-only. Its commit always writes the whole word, wherever the scrub stands. The prefix action is unavailable, and the bar states the reason. The jumbled puzzle's three-twist repair (owner answer O10, 22 October 2026) is whole-only. The solver steps through it in the preview, and the one permitted repair transaction holds all three witnesses. Previews that admit a prefix keep the prefix rule above, including the separate recheck of the prefix."
- Slot name: `commit.bar.whole_only`.

## 2. Major record findings

### R-3. Recovering open documents at exit

- **Rev5, ADR-D01.80**, "The view, the layout, the workspace and the open documents" becomes:
  "The view, the layout, the workspace, and for each open document: its identity (a path, or an untitled id), the digest of its last saved bytes, and, when it is dirty, its full unsaved contents."
- **Rev5, ADR-D01.79** gains: "At the next start the open documents come back with those contents, still dirty. This includes untitled buffers that never ran. Nothing is written to a document's file until the solver saves. If a file changed on disk since, its tab says so and keeps both versions. Preferences recorded in the log are not applied (ADR-D01.80)."

### J-4. Releases are ordered by their own sequence

- **ADR-D01.9** gains: "Every preview release and every check it starts carries a release number. Release numbers increase within the session, apart from the revision. A result is current only when it matches both the current revision and the latest release. A result that arrives for an earlier release never replaces the current one, whatever order the results arrive in."
- **ADR-D01.26** gains: "Where a tool keeps the solver's own checks (the jumbled puzzle's O4), they are kept in release order, the order tried, not in the order they finish. Each keeps its ΔN in a fixed column. A commit clears them, because it changes the revision."

### J-7. Only a typed, matching job result fills a slot

- **Rev4, ADR-D01.61**, the slot states become seven: empty, pending, ready, stale, refused, budget-stopped, error. Each job result is typed as completed, cancelled, cut off or error. It carries its input digest and its parameters (for the jumbled puzzle: the cap, the order-pinned menu identity, the tie rule and the contract revision). It fills the slot only after its digest matches the shown revision and its parameters match the request.
  - A completed, matching result makes the slot **ready**. An empty list appears only here, and it names its cap and revision.
  - A cancelled result returns the slot to its previous state and shows nothing of the cancelled job.
  - A cut-off result makes the slot **budget-stopped**. It makes no negative claim, and any witness it found shows only once it has been checked.
  - An error makes the slot **error**, with the reason.
  - A result whose digest or parameters do not match is discarded. The slot stays pending or stale.
- Slot names: `slot.tool.{empty, pending, ready, stale, refused, budget_stopped, error}` (adds `error`).

### J-8. Research origin is checked at preview admission

- **ADR-D01.8**, the solving-session sentence gains: "Admission checks the word's provenance chain, not only the program that presents it. A word with any input taken from a research run record is refused at preview in a solving session, with its reason shown, even when the solver's own program wraps it (the interim reading: research output never becomes a preview in a solving session). Its evidence stays readable."

### J-9. Outcome-neutral motion and drags

- **ADR-D01.55** gains: "Motion, like sound, never differs with whether a check or repair result is good or bad. A drag has no snap, detent or attraction toward alignment, and no sound that favours one. Exact states that depend on a drag, such as the jumbled puzzle's cut-cell states, update only on release and are never updated live from float positions."

### Q5. Replay and comparison side are registry rows

- **ADR-D01.16** gains: "Bringing a branch's later moves back is the row `session.branch.replay`. It prepares a preview and never commits; the commit goes through the bar."
- **ADR-D01.10** gains: "What the preview view compares (both, before, after) is set by the rows `preview.side.{both, before, after}` and cycled by `preview.side.next`."
- **ADR-D01.17** gains: "Every action a surface offers resolves through the registry. A button, a chip or a menu entry is never a widget-only action. The check-mode chip's Net and Strict use the command table's protection policy row."
- These rows join the command table at its freeze. Their default keys are drafted with the default keymap (ADR-D01.21).

## 3. Minor record findings fixed here

- **R-4.** The motion and cue list of rev4 section 4 freezes as this versioned table (event-to-ID table, version 1). It fixes the events and IDs only; sounds, curves and durations come in later rounds.

  | Event | Motion row (H-03) | Cue id |
  | --- | --- | --- |
  | Strip change | `strip.change` | `cue.strip` |
  | Orbit completion | `orbit.complete` | `cue.orbit` |
  | Next activation | `next.activate` | `cue.next` |
  | Lock set | `lock.set` | `cue.lock` (shared) |
  | Lock released | `lock.release` | `cue.lock` (shared) |
  | Draft/Direct switch | `route.switch` | `cue.route` |

- **J-3.** The instruments of ADR-D01.77 gain an **undefined** value pattern. It is distinct from zero, from stale, from pending and from "unknown". An exact numeral or mark set whose input is not defined shows it, for example the jumbled puzzle's ν off the lattice. It never shows zeros, stale values or partial values. Slot name: `numeral.undefined`.
- **Q3.** **ADR-D01.22**, the View section's scope has three values: Both (the default), Global and Local. Each eligible field (projection, shrink, frame visibility) is edited for the chosen scope, and under Both an edit writes both views.

## 4. Not changed in the record

- **B-1 to B-13** are defects of the A5 Look Lab build against a record that already states the rule. Examples: ADR-D01.4 items 1 and 2 (a failed check stops at the preview), ADR-D01.58 (Enter in the command bar never commits), ADR-D01.12 (a prefix is rechecked as its own preview), ADR-D01.9 (a replaced or overtaken preview cannot be committed), ADR-D01.69 and rev4 ADR-D01.36 (the Library dock in Solve). They are fixed in the Look Lab build, not here.
- **Q1, Q2, B-11, C-1 and C-2** are minor build, measurement or confirmation items. They go to the build follow-up and to round 3 (values), as the review rules defer minor findings.

## 5. Fixes from the scoped re-check (23 November 2026)

The scoped re-check (call `20261123T024528Z-01a2064b`, gpt-6.2-astra) found four record items still open: J-6, R-3, J-7 and a new N-1. The amendments below replace the named text of sections 1 and 2. Everything else in this file stands.

### J-6. Every repair commits whole (replaces the J-6 amendment of section 1)

- **ADR-D01.12** gains: "A preview may be whole-only. Its commit always writes the whole word, wherever the scrub stands. The prefix action is unavailable, and the bar states the reason. Every repair of the jumbled puzzle's descent is whole-only: the three-twist repair and the two-twist repair, whose z is the identity (owner answer O10, 22 October 2026; the identity-twist rule of the jumble topic). The solver steps through it in the preview. The one permitted repair transaction holds exactly the repair's actual witnesses, two or three, and never an identity twist. Previews that admit a prefix keep the prefix rule above, including the separate recheck of the prefix. Whole-only belongs to a repair taken from the repair list; the same twists typed by the solver as an ordinary word are an ordinary preview."
- Slot name: `commit.bar.whole_only` (unchanged).

### R-3. What the exit inventory stores (replaces the R-3 amendment of section 2)

- **Rev5, ADR-D01.80**, "The view, the layout, the workspace and the open documents" becomes:
  "The view, the layout, the workspace, and for each open document: its identity (a path, or an untitled id with the mark never saved), the digest of its last saved bytes when it has been saved, and its full contents at exit, clean or dirty."
- **Rev5, ADR-D01.79** gains: "At the next start each open document comes back with its contents at exit, dirty if it was dirty, including untitled buffers that never ran. Nothing is written to a document's file until the solver saves. If a file's bytes on disk no longer match the stored saved digest, its tab says so and keeps both versions: the contents at exit and the file on disk. Preferences recorded in the log are not applied (ADR-D01.80)."

### J-7. A result states its coverage (adds to the J-7 amendment of section 2)

- A completed result also carries its coverage: the domain it searched in full. For the repair list this is the named cap and the pin, menu and tie rule it ran under. For a setup search it is the setup family examined, its guarded alphabet and its length bound (ADR-D01.68).
- A completed result without coverage is treated as an error.
- An empty list names its coverage, and it claims only that coverage.

### N-1. A late result never undoes a newer one

- **ADR-D01.9** (with the J-4 amendment) gains: "Every tool job request also takes a release number from the same session sequence."
- **Rev4, ADR-D01.61** gains: "Every job carries the release number of the request that started it (ADR-D01.9). A job result of any type, cancelled included, changes a slot only when its release number is the slot's latest request. A result from an older request is discarded, whatever order the results arrive in. Cancelling a job that is no longer the latest request changes nothing on screen."
