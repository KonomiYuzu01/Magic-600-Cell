# Closure wave handoff sheet

Status: the integrator's working sheet for the drafts that close the freeze-readiness findings. It decides nothing. Each form below
is a working form, so that seven drafts written in parallel meet in one place. Accepting an ADR, freezing a contract and answering an
LQ stay with the owner.

Read with: `findings.json` (what each packet must close), `check_closure.py` (how a packet is checked), and chapter 12's PHS.13 to
PHS.17 and PHS.48 (what the freeze input needs).

## H1. Packets and owned files

| Packet | Line | Owns | Closes |
| --- | --- | --- | --- |
| `p1` | EN3 | new `docs/progress/1.0/jumbling-interfaces.md`, new `docs/progress/1.0/command-rows-jumbling.json` | FR-K01 (EN3 half), FR-K06 rules, FR-B03's jumbling half, FR-C02's word carrier, FR-K05's jumbling rows |
| `p2a` | EN4 table | `docs/progress/1.0/command-table.md`, `command-table.json`, `build_command_table.py`, new `test_command_table.py`, new `command-aliases.json`, new `command-join.json` | the C1 join, DT-M1 fix-first items 2 to 4, FR-B04's table half, the row format |
| `p2b` | EN4 layers | new `docs/progress/1.0/layer-interfaces.md`, chapters 01 and 08 | FR-K01 (EN4 half), FR-K02, FR-K03, FR-B04's contract half, FR-C01, KN5.8, KN7.2, FR-K06's command side |
| `p3` | EN4 format | new folder `docs/architecture/1.0/language/format-packet/`, chapter 05, and in chapter 07 only the `ProvenanceBlock` definition and BND.23 | FR-C02, every PS2.13 row, FR-K06's durable side, plan finding PLAN-03 |
| `p4` | LP1 execution | chapter 09, the `LG4.*` clauses of chapter 02, RTE.40 of chapter 11, new folder `docs/architecture/1.0/language/contract-vectors/` | FR-K04, FR-B02 |
| `p5` | LP1 language | the `LG2.*` clauses of chapter 02, chapters 03 and 04, RTE.6 of chapter 11 | FR-B01, FR-B05, FR-B06, FR-B07, FR-B08 |
| `p6` | renderer | chapter 10, new `docs/architecture/1.0/language/renderer-feature-matrix.md` | FR-K05, the display side of FR-B04 and FR-B06 |

Each packet also owns `closure/manifests/<packet>.json`. No packet edits chapter 12, chapter 13, the partition, an ADR, `schemas/`,
a critical path, another packet's file or another packet's clauses. A second wave, after these land, writes the language-kernel annex
(FR-B03) and runs the reconciliation traces of H13.

New clause numbers continue the family's highest number. Chapter 11 is shared: `p4` adds from RTE.71, `p5` from RTE.80. New files
use their own family: `JI` (`jumbling-interfaces.md`), `LI` (`layer-interfaces.md`), `CM` (`command-table.md`), `FP` (format packet),
`CV` (contract vectors), `RM` (renderer feature matrix), `KI` (kernel interface annex), `CX` (claim matrix). Clause style stays `- **KN5.7 [D]** text`, with the tags **[B]**, **[D]**,
**[P LQn]** and **[O]** as defined in chapter 00.

## H2. Command row format and the jumbling delta (owner: `p2a`; input: `p1`)

`p1` delivers its rows as a delta, so one builder stays the only way into `command-table.json`:

```json
{
  "format": "c600-command-rows-delta-1",
  "add": [
    {"id": "jumble.on", "label": "Jumble on", "kind": "commit", "inventory": [], "gates": ["confirm"], "lane": "commit",
     "preview": "none", "undo": "journal", "contexts": ["puzzle"], "repeat": false, "job": false, "key_0_4": null,
     "disposition": [], "note": "new in 1.0; ENG-0J line 145", "source": "ENG-0J:145"}
  ],
  "amend": [
    {"id": "macro.add", "expect": {"kind": "workspace"}, "set": {"note": "stores a menu word as well"}, "source": "ENG-0J:146"}
  ]
}
```

- `add` carries a whole row. `amend` names an existing ID, the fields it expects, and the fields it sets.
- The builder refuses: an `add` of an existing ID; an `amend` of a missing ID; an `amend` whose `expect` differs from the current row;
  two amendments of one field of one ID; a value outside a vocabulary; a missing delta file unless `--draft-without-jumbling` is given.
- A table built with that flag says `"final": false` and carries no jumbling row. The final table, built with `p1`'s real file at
  integration, says `"final": true`. Exactly one definition per ID results.
- `p2a` runs in parallel with `p1`, so its tests use `closure/fixtures/jumbling-rows-example.json`, and its stored `command-table.json`
  is the draft form. The integrator builds the final table after both land.
- `p1` needs only the fields above. A field `p2a` adds later gets a default in the builder, or the integrator fills it.

## H3. Freshness (owner: `p2b`; closes FR-K02)

ONE.39 keeps a preview across an unrelated workspace edit, and commit step 6 compares the full `RevisionRef`. Working form:

```text
PreviewDeps = { epoch: EngineEpoch, identity: IdentityBlock, mode: jumble_off | jumble_on, state: Nat, protection: Nat, draft: Digest<resolved draft> }
fresh(p) iff p.deps == the same fields read now
```

- `draft` covers every workspace record the preview read, after expansion. An edit to a record the preview did not read changes nothing.
- `RevisionRef.workspace` is no dependency. Neither is `head` unless `p2b` shows that a head can move only with a state or protection
  revision, or names which row moves it.
- Commit step 6 compares `PreviewDeps`, and step 7 still compares the full-state hash. Undo back to the same digest moved the `state`
  revision, so the preview is stale although the digest is equal.
- This is an engineering contract under V03.3 ("exact match to engine-validated dependencies only"), and it needs no owner answer.

## H4. Terminal outcomes (owner: `p2b`; closes FR-K03; consumers: `p6`, `p1`)

`Receipt` today has no success terminal outside `committed(CommitRecord)`. Working form:

```text
Status = accepted | running(JobId)                  (not terminal)
       | committed(CommitRecord)                    (commit rows only)
       | completed(Completion)                      (every other kind)
       | refused(Error)                             (before acceptance; nothing happened)
       | failed(Error)                              (after acceptance; nothing was committed)
       | cancelled(CancelReason)
       | unknown_outcome
Completion   = { kind: query | preview_ready | workspace | view_published | file | app, revision: RevisionRef, result: ResultRef? }
ResultRef    = { job: JobId?, digest: Digest<level>, covered: typed description? }
CancelReason = by_user | superseded(by: CommandId) | replaced_revision | budget
```

- One accepted invocation has exactly one terminal status. KN5.22 stands: a committed command is never reported failed.
- `outcome(command, request) -> Receipt` answers after a lost notification. `p2b` states which kinds keep a durable terminal and what a
  lookup returns for the others once the engine epoch has ended. It never guesses a success.
- `publish_view` ends in `completed(view_published)` or `cancelled(superseded(...))`. Job results stay `JobResult` (KN7.3), tied to the
  command and job identity.

## H5. Lanes and availability (owner: `p2b` for the contract, `p2a` for the rows)

- Each row gets `lane: commit | exclusive | latest_wins | program | view | none`.
- `not_busy` leaves the gate vocabulary. It meant two things: a commit in flight, which the commit lane covers, and a running job, which
  no longer blocks a commit or an undo (DT-M1 Q5 (a), Q6 (a); KN5.20). The builder rule "a commit row has `not_busy`" becomes "a commit row
  has lane `commit`". A workspace or file row that carried `not_busy` gets a lane or a stated reason.
- `session.undo` runs in the commit lane, and a commit or undo cancels the jobs that read the replaced revision.
- Availability (KN5.8): `availability(command, args, revision) -> available | unavailable(code)` is a hint for the shell. It grants nothing,
  and the command is checked again at `invoke`. An unavailable reason is a registry code, never prose.

## H6. Approximate control (owner: `p5`; consumers: `p4`, `p6`; independent of LQ21)

LG2.5 needs one form before the checker is built. `p5` drafts a recommendation and keeps the other on record. Working assumption for
the consumers, which is the integrator's leaning and form 1 of LG2.5:

- An approximate comparison yields `ApproxBool`. No ordinary `if` accepts it.
- `display_if(c, a, b)` is the only branch on it. It appears only in display expressions, its branches are pure, and its result has a display
  type. No operation takes a display type back to an exact type, whether through a call, generic instantiation, a closure, storage or an export.
- Charges (`p4`): one step for `display_if`, plus the branch taken. Output (`p6`): display values reach the renderer only through
  `publish_view`.
- Ruling after reconciliation audit 2: terminality removes the later exact computation and the reclaimed-budget channel. It does not remove
  the recorded exact payload: DET.6 covers granted and used deterministic budgets, outcome, code and coverage, DET.58 makes used memory the
  peak, and EV2.4, EV2.7 and RTE.46 compare them. So DET.64's predicate and size agreement stays. A branch that differed could change a
  recorded counter or a deterministic stop even with equal final display values. LG2.20 forbids any later exact step from reading those
  observations or reclaiming the remainder. Stated once: display accounting is host-observable and unavailable to exact continuations, and its
  deterministic fields still take part in DET.6 reproduction. LG2.21 keeps the same isolation. Removing the agreement would need an explicit
  versioned split of the exact payload, which does not exist and must not be inferred.

## H7. Source coordinates (owner: `p5`; consumer: `p6` for `SourceMap`, `p4` for stop positions)

```text
SourceRange = { source: Digest<bytes>, start: Nat, end: Nat }
```

- `start` and `end` are UTF-8 byte offsets on scalar boundaries, half-open. An empty range is an insertion point.
- A host declares its unit (`utf8`, `utf16` or `scalar`), and the conversion happens at the EV1 boundary. A range against another source digest is refused.
- Word positions are zero-based and half-open. A compressed part cites its expansion by part path and offset (EV2.11).

## H8. Text and output accounting (owner: `p4`; consumers: `p3`, `p6`)

- `Text` is a sequence of Unicode scalars. Equality and order are by scalar, with no normalization.
- Output is charged in canonical uncompressed bytes under the data core's canonical profile. Framing, transport and compression are not charged. A terminal
  status has its own fixed reserve, which cannot carry user output.
- Logical memory is charged in versioned logical units, kept apart from output and from stored size (KN7.9).

## H9. Provenance without a run (owner: `p3`; closes PLAN-03)

`ProvenanceBlock.profile` is required, and a hand-typed word or an imported 0.4 macro has no run. Working form: `profile: Profile | none`, with `none` allowed
only when `producer` is `manual` or `import(...)`. It means "no program run produced this", and it grants nothing.

- Hand-typed word: `producer: manual`, `author: own`, `profile: none`, `depends` empty (`read_state(no)` under capability separation).
- Imported 0.4 macro: `producer: import(...)`, `author: own`, `profile: none`, `depends: not_recorded`, source digest in `legacy`.
- BND.21's reading of a missing field stays `unknown`. This is a storage proposal, it decides no boundary policy, and BND.74 stays open.

## H10. Word carrier (owner: `p1`; consumer: `p3`)

PS1.20 leaves flat storage or referenced parts to EN3. Working assumption until `p1` lands: flat `ia16` chunks of expanded menu letters, with optional named
annotations over spans, because KN4.7's journal row cites the literal letters by digest and the admission bound applies to expanded letters. If `p1` chooses
otherwise it says so in `handoff_changes`, and the second wave reconciles `p3`.

## H11. Exit record transitions (rules: `p1`; command side: `p2b`; durable side: `p3`)

Final working rule after the reconciliation audits. A5's refusals stay unless the owner changes them.

1. Validity is a function of the selected journal path and durable void facts, never of labels.
2. An exit record is active when its crossing is on the selected parent path, the path from the crossing to the head holds only retained moves, and no
   void fact names it. Otherwise it is inactive or void. Inactive is not deleted.
3. Undo or redo of a retained move keeps the exit poses as the replay root. Replay runs over the moves in effect at the head.
4. Undo of a crossing restores the recorded puzzle-lineage tuple in one atomic step, under a new state revision and a durable inverse-action receipt. Redo
   validates and keeps the exit identity.
5. A reset, an import or a checkpoint restore voids every earlier exit of the session lineage, active or not, in the same transaction. The void is monotone
   and survives undo of the break and branch selection. This is a conservative refinement of A5.
6. Recovery recomputes from the durable journal and void facts. A half-crossed state never exists.
7. Equal labels never restore validity: two states with equal labels and different pose lineage are different for this rule.
8. Branch navigation is a journal operation, not a checkpoint assignment.
9. `exit` and `entry` are separate transition actions. An entry records its entrance binding, `exit_root(exit, crossing, suffix digest)` or
   `solved_witness(word)`, and both sides. Undo and redo of either use that transition's own before and after, never another record's.
10. Puzzle lineage and context are separate: `Position = {head, mode, episode, state}`, `Context = {protection, preferences}`. Crossing, branch navigation and
    undo or redo of a journal action restore Position fields only. Current preferences and current protection stay (U1-015; solving-workflow 4.2; dispositions
    S1). Protection changes only through a workspace action, or where a crossing itself changed the protection required at its level (JI.27), which that
    transition records and which is reversed with it.
11. Workspace actions join the same chain with their own action tag, inverse and changed-record references. Position before and after are equal, and the state
    revision does not move. The tag classifies the effect, not the command row kind: the non-undoable `session.checkpoint.save` commit row uses `action=workspace`
    with a null inverse (LI.9, JI.27, FP.31).
12. Every durable transition carries `command: CommandId`, `request: Digest`, `admission: Ref`, `receipt: Ref` and the new `RevisionRef`. Admission and terminal
    are durable records for commit and workspace invocations (KN5.21, KN5.23, LI.30), and stay queryable after undo and epoch loss.
13. Mechanical word commits use FP.31's mode-specific action tags and JI.27's move/undo/redo rows. A `jumble_move` joins this chain,
    including across checkpoint save and recovery, but cannot occur in a retained-only exit suffix.

Transition shape for the format packet:

```text
Transition = { id, sequence, previous, action: retained_move | jumble_move | undo | redo | exit | entry | branch | reset | import | restore | workspace,
               before: Position, after: Position, context_before: Context, context_after: Context,
               event, exit, entrance, voids, command, request, admission, receipt, revision }
```

Continuity holds on both Position and Context: each record's `before` and `context_before` equal the preceding record's `after` and `context_after`.

## H12. Alias table (owner: `p2a`; consumer: `p3`)

`docs/progress/1.0/command-aliases.json` holds DT-M1's reviewed alias rows (about 205, over the 268 decisions that carry a 0.4 command ID). It has a table version and
a content digest. `p3` cites the version and digest by reference (PS3.3 to PS3.8), and until `p2a` lands it writes the rule and leaves the value as `to be pinned`.

## H13. Second-wave traces

After the drafts land, a reconciliation run walks these through the combined text. Every step must be determined without inventing a rule, so write with them in mind:

1. An approximate comparison, through a helper and a closure, into a kernel batch and a view publication: type admission, charges, stop position, source range, terminal receipt.
2. The same publication superseded, and the same publication with output exhausted.
3. Exit, retained move, undo, redo, branch change, restore, import and recovery, with equal-label states of different pose lineage.
4. Provenance of a hand-typed word and an imported 0.4 macro under both forms of LQ21.

## H14. Manifest

Each packet writes `closure/manifests/<packet>.json`. `check_closure.py --packet <packet>` is its acceptance check.

```json
{
  "packet": "p2b",
  "assumes": [{"item": "LQ4", "assumed": "what this draft takes as given", "if_other": "what changes under the alternative"}],
  "owner_items": [{"item": "ONE.78", "why": "only the owner can settle it"}],
  "handoff_changes": [{"section": "H3", "change": "what differs from this sheet", "why": "the reason"}],
  "new_error_codes": [{"code": "KN5:stale.example", "clause": "KN5.99", "meaning": "for chapter 13"}],
  "new_requirements": [{"id": "CT-KN5-99", "phase": "LP2", "class": "source"}],
  "findings": {
    "FR-K02": {
      "status": "closed",
      "clauses": ["ONE.39"],
      "why_open": "",
      "who": "",
      "vectors": {
        "unrelated-workspace-edit": {"given": "inputs and state", "expected": "the determined outcome", "clause": "ONE.39"}
      }
    }
  }
}
```

- `status` is `closed`, `partial` or `open`. `closed` means the draft contains what the finding asks for, subject to review. Anything else needs `why_open` and `who`.
- Every vector tag `findings.json` names for the packet is required, with a concrete `given` and `expected`. "See text" is not an expected result.
- `new_error_codes` go to chapter 13 and `new_requirements` to chapter 12 at integration, by the integrator.

## H15. Reconciled forms (input to the P7 packets; the audits' findings are in `reconciliation-findings.json`)

The three reconciliation audits found real conflicts between the drafts. Each form below is the one the packets write to, so that five packets
running in parallel meet in one place. A packet that finds a form wrong says so in `handoff_changes`, with the reason and a vector.

### H15.1 Undo entry (RA1-01)

`session.undo` and `session.redo` are the only undo rows (DT-M1 C1, Q4 (a)). `workspace.undo` and `workspace.redo` leave the table: dispatch is internal and
needs no row. Their inventory, aliases and join mappings move to `session.undo` and `session.redo`. `undo_dispatch` becomes the entry plus the two classes it
reaches. Both rows carry `slot_map: by_target`: a journal action preserves the map, and a workspace action that recorded one restores it. A workspace inverse
returns `committed`, with equal puzzle hashes and advanced workspace versions.

### H15.2 Action hint (RA1-08)

```text
ActionHint = { order: ActionNumber, target: ActionNumber }
```

`order` is the current durable action-order number and `target` the action to invert. Both rows take `{"hint": "ActionHint", "token": "UserActionToken"}`, and
both fields enter the request and token binding. `action_guard` becomes `action_hint`. A hint whose order or target no longer matches gets
`KN5:stale.undo_superseded`.

### H15.3 Preview admission (RA1-02)

`operation.preview` has no `protection` gate. The computed `protection` field of `Preview` stays, and `operation.execute` keeps its gate. A word refused at a
prefix still builds no preview (ONE.33).

### H15.4 Filter freshness (RA1-10)

New gate `fresh_filter_preview`, and new `preview` value `filter`, for `filter.apply` only. The builder rule becomes: `preview` is `required` exactly when
`fresh_preview` is a gate, and `filter` exactly when `fresh_filter_preview` is a gate.

```text
FilterPreviewDeps = { epoch: EngineEpoch, identity: IdentityBlock, mode: jumble_off | jumble_on, state: Nat, filter: Digest<resolved filter> }
filter_fresh(f) iff f.deps == the same fields read now
```

The predicate is bound to the submitted `FilterPreviewRef`. It uses no pending preview slot and needs no presentation. A stale reference gets
`KN5:stale.filter_preview`. KN0.8 stays the word predicate.

### H15.5 Lane targets (RA1-12)

Every row in lane `view` or `latest_wins` declares `lane_target`, one of `{"arg": "<argument naming the ViewId>"}`, `{"view": "<fixed ViewId>"}` or
`{"control": "<session control target>"}`. The command layer resolves it before admission, and it is part of the request binding. Two requests share a lane
exactly when session and resolved target are equal.

### H15.6 Overlay dependencies (RA1-11)

```text
requires: [ { row: RowId, args: { static: Digest } | computed } ]
```

A static entry uses that (row, digest) availability answer. A computed entry uses an argument-independent hint from the row-level gates only, and any
argument-dependent refusal comes at `invoke`.

### H15.7 Cut-off (RA1-03)

LI.28 is the only mapping from job outcome to receipt. `completed(query)` may carry a partial value only when the result type declares partial coverage as a
normal outcome within the grant. An invocation that exhausted its budget is `cancelled(budget)`. JI.15 cites LI.28 and keeps no mapping of its own.

### H15.8 Publication (S2-01)

Pre-acceptance validation applies to a standalone publication whose payload exists at admission. A scheduled publication keeps its original invocation:
a final validation error becomes `failed(original Error)` on that invocation, with no other lane admission and no resurrection.

### H15.9 Contextual code forms (S2-05)

A registry row's `form` is its primary form, and a row may list contextual forms. `EV3:busy.renderer_unavailable` is a refusal before acceptance and a `stop`
inside an error outcome after acceptance (`Receipt.status = failed`). Completed receipts stay after installation.

### H15.10 Diagnostic and pick precedence (S2-02, S2-03)

- `LG2:type.exactness_violation` is the code for `ApproxBool` and control-dependence misuse. `LG2:type.exact_required` keeps a delimited meaning. EV1 forwards
  the code unchanged.
- A pick checks its bindings first, with `EV3:stale.display_pick`. `EV3:stale.source_link` is for a later source-digest mismatch.

### H15.11 Form 2 lowering (S2-04)

Form 2 of LG2.21 stays an alternative. It lowers to named entry, branch and exit operations with an execution identity, so its charges are determined. No owner
selection is needed.

### H15.12 Evidence corrections (RS3-01 to RS3-06)

Draft corrections to chapter 06, with no threshold chosen and LQ3 open:

- EVD.32 row 5's guard gains `job != not_run`.
- Failure of EVD.42's form-specific refutation conditions is an admission inconsistency at row 0, before support and Ref propagation. For a refutation the order is I5, EVD.41, then I6 on live leaves only.
- Support attributed to its own not-run job is inert before `stands` is evaluated. EVD.33 and EVD.47's row-7 statement is qualified as the fallback.
- I5 extends to any offered job-produced negative whose own job did not complete. Checked positive witnesses stay.
- FP.19 `Grade` adds `human_review` and `formal_proof`, with their conditional admission unchanged. LQ15 stays open.
- A non-search leaf can bind its producing job through a source-bound fact independent of `SearchRecord`, with explicit absence.

### H15.13 Word relation targets (RS3-07)

`relation.word` is a tagged target: `{"local": <annotation id>}`, `{"part": <part id>}` or `{"ref": Ref}`, with ordering and closure rules. JI.21 and FP.13
state the same form, and `examples.json` encodes JI.23.

### H15.14 Word carrier wording (RS3-08)

PS1.18 to PS1.20 and CT-PS1-09/10: flat bytes are conditional on the flat draft, literal chunks are mandatory for stored menu words, and annotations never
generate replay letters. Compressed graph retention stays separate from admission of its expanded word. LQ10 stays open.

### H15.15 Alias pin (RS3-09)

FP.15 uses the alias table's own declared identity profile (`content_digest` excludes itself and the final LF), never `DigestRef(content)`. `p7-table` bumps the
table version because its rows change. `p7-data` writes the rule and pins the current value, and the integrator refreshes that pin after `p7-table` lands.

### H15.16 Error code proposals (RS3-11)

ERR.2 makes the prefix the raising component, so the `JI` prefix goes. JI clauses stay the rule sources.

| Draft code | Final code | Why |
| --- | --- | --- |
| `JI:input.unsupported_dimension` | `KN1:input.unsupported_dimension` | The model registry refuses the header or request |
| `JI:input.named_object_invalid` | `KN2:input.named_object_invalid` | The engine validates the named object |
| `JI:input.render_payload` | existing `EV3:input.display_bundle_mismatch` | Same meaning, so no new code |
| `JI:input.projection_chain` | existing `EV3:input.display_unsupported` | Same meaning, so no new code |
| `JI:input.scramble_policy_unset` | `KN5:input.scramble_policy_unset` | Command-layer availability |
| `JI:unverified_reference.branch_path` | `KN4:input.branch_path_unproved` | The store finds no proved navigation path; ERR.14's `unverified_reference` needs a missing frame |
| `JI:unverified_reference.exit_record` | `KN5:input.exit_record_voided` | The command layer refuses reactivation |
| `JI:unverified_reference.exposed_geometry` | `KN2:input.exposed_geometry_unavailable` | The engine producer has no certified data |

One more new code: `KN5:stale.filter_preview` (H15.4). `p7-reg` registers these with the 16 other draft codes of the manifests: 23 in all. Any other new code goes
in a packet's `new_error_codes`, for the integrator.

### H15.17 Service registrations (the annex's `requires`)

`command-table.json` gains a top-level `services` list: `{id, kind: service | continuation, caller: runtime | host | both, lane, clauses, note}`. A service is no
command: it has no key, alias, availability or undo entry, it grants nothing, and it is no part of C1's count. Its IDs must not collide with a command ID. It holds
`kernel.snapshot.open`, `kernel.snapshot.close`, `kernel.stream.next`, `kernel.value.retain`, `kernel.value.release`, `kernel.outcome`, `kernel.job_result`,
`kernel.undo_hint` and `language.publish_view`. `jumble.observe` is a command and enters through the delta.

### H15.18 Claim matrix (CX.18, CX.19)

- Chapter 06's worked example E1 must not grade `replay_word.py` as an independent checker: the pinned source walk shows imports into construction code. E1 keeps
  both paths unqualified until KN6.2 and KN6.5 are met.
- LB5.17 and `manifests/p5.json` stop saying the per-leaf inventory is owed. It now exists as a draft plan. Checkers, typed bindings and the mutation gate stay owed,
  and FR-B08 stays partial.

### H15.19 Scrub row

`jumble.preview.scrub` leaves the delta. DT-M1 C1 gives one command per purpose, so `view.preview.scrub` carries the jumbling preview by its arguments, and JI.33
and CT-JI-07 cite it.

## H16. P7 packets (the reconciliation wave)

Each packet writes `closure/manifests/<packet>.json` in H14's form, with `findings` keyed by the IDs of `reconciliation-findings.json`, and passes
`python docs/architecture/1.0/language/closure/check_p7.py --packet <packet>`. New clauses continue the family's highest number. No packet edits another packet's
file. A form another packet owns is written as H15 states it, and a disagreement goes in `handoff_changes`.

| Packet | Owns | Findings |
| --- | --- | --- |
| `p7-table` | `docs/progress/1.0/`: `build_command_table.py`, `command-table.json`, `command-table.md`, `command-join.json`, `command-aliases.json`, `command-rows-jumbling.json`, `test_command_table.py`; `kernel-interface/*`; manifests `p2a.json` and `p7-table.json` | RA1-01, 02, 08, 09, 10, 12, 13 (rows), H15.17, H15.19, the final table |
| `p7-layers` | `docs/progress/1.0/`: `jumbling-interfaces.md`, `layer-interfaces.md`; chapters 01 and 08; manifests `p1.json`, `p2b.json` and `p7-layers.json` | RA1-01 to 12 (clauses), S2-01 (LI), S2-05 (LI), RS3-07 (JI), RS3-11 (JI) |
| `p7-data` | chapters 05 and 06; `format-packet/*`; manifests `p3.json` and `p7-data.json` | RS3-01 to 09, RA1-04 to 07 (format), H15.18 (E1), the moved chapter pin |
| `p7-lang` | chapters 02, 03, 04, 09 and 11; `contract-vectors/*`; manifests `p4.json`, `p5.json` and `p7-lang.json` | S2-02, S2-03 (EV3), S2-04, S2-06, RS3-12 (chapter 04 and p5), H6's ruling, H15.18 (LB5.17), the pending-EN4 wording |
| `p7-render` | chapter 10; `renderer-feature-matrix.md`; manifests `p6.json` and `p7-render.json` | S2-01, S2-03 (RM), S2-05 (RM), RS3-10 |
| `p7-reg` | chapters 12 and 13; the ADR files; manifest `p7-reg.json` | H15.16's registry, S2-05 and RS3-11 (chapter 13), RS3-12 (chapter 12), the 38 requirements and counts, ADR descriptions, ERR.12 |

LX-9's effect on KN3.4, LG2.4, RTE.64 to RTE.66 and DET.33 to DET.34 is not part of this wave: it waits for the exact-request experiment and the owner.
