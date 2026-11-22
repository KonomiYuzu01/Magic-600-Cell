"""Build H-08 from hand-written rows, the signed decision join and H2's deltas.

Importing this module writes nothing. build_table accepts altered inputs for
headless checks. The CLI writes only --out, after every check has passed.
"""
import argparse
from copy import deepcopy
from collections import Counter
import hashlib
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
KINDS = {"commit", "workspace", "view", "query", "file", "app"}
UNDO = {"journal", "workspace", "layout", "none"}
GATES = {"protection", "fresh_preview", "fresh_filter_preview", "confirm", "model_match", "file_check"}
LANES = {"commit", "exclusive", "latest_wins", "program", "view", "none"}
CONTEXTS = {"puzzle", "solve", "operation", "keyboard", "filter", "macro_base", "editor", "any"}
REPEAT = {"camera.rotate", "camera.zoom", "list.move", "view.preview.scrub"}
LIST_ACTIONS = {"list.move": "navigation", "object.inspect": "inspection", "list.row.copy": "row_copy"}
USER_ENTRY_KINDS = {"key", "button", "menu", "command", "mouse", "control"}
DELTA_FORMAT = "c600-command-rows-delta-1"
# Applied in this order: the language delta may amend a jumbling row. The design delta adds ADR-D01's view and
# workspace rows, ordinary edits after the LP3 freeze (owner, 22 November 2026, item 3), and the commit delta its
# commit rows and the rows it moves into the layout history (item 4).
DELTA_FILES = ("command-rows-jumbling.json", "command-rows-language.json", "command-rows-design.json",
               "command-rows-commit.json")
ROW_FIELDS = {"id", "label", "kind", "inventory", "gates", "lane", "preview", "undo", "contexts",
              "repeat", "job", "key_0_4", "disposition", "note", "source", "arguments"}
UNDO_ROWS = {"session.undo", "session.redo"}
# ADR-D01.14's layout history: its own undo and redo rows, which only the layout undo class reaches.
LAYOUT_UNDO_ROWS = {"layout.undo", "layout.redo"}
# ADR-D01.15's partial undo: a live commit inside the head's journal entry, with the protection recheck.
PARTIAL_UNDO_ROW = "session.undo.partial"
PARTIAL_UNDO_ARGUMENTS = {"entry": "ActionNumber", "move": "Nat", "token": "UserActionToken"}
# ADR-D01.12's one-key preview-and-commit: a live commit of the solver's own macro, a record draft, with the recheck
# operation.execute makes, and no default key.
ONE_KEY_ROW = "operation.preview.commit"
ONE_KEY_ARGUMENTS = {"draft": "ResolvedDraftRef", "token": "UserActionToken"}
# ADR-D01.15's multi-step rollback: one gesture of ordinary undo or redo steps of the move history, each its own
# transaction and receipt under the gesture's command ID, with the gesture and step i of k in its arguments (the
# integrator's ruling of 22 November 2026). Only the layer names the gesture and the steps, so they are optional.
ROLLBACK_ROW = "session.rollback"
ROLLBACK_ARGUMENTS = {"hint": "ActionHint", "gesture": "Text?", "step": "Nat?", "of": "Nat?",
                      "token": "UserActionToken"}
# CM.15 and JI.32/JI.33: required names, independent of the generating delta.
# Optional parameters remain in the delta's full schemas.
DELTA_REQUIRED_ARGUMENTS = {
    "jumble.on": {"revision", "token"},
    "jumble.off": {"revision", "token"},
    "jumble.twist.menu": {"cap", "letter", "revision", "token"},
    "jumble.twist.append": {"cap", "letter", "draft", "phase"},
    "jumble.survey": {"snapshot", "max_chunk"},
    "jumble.observe": {"snapshot", "fields", "scope", "max_chunk"},
    "jumble.straddlers": {"snapshot", "cap", "max_chunk"},
    "jumble.certificate.inspect": {"snapshot", "certificate", "cap", "piece"},
    "jumble.release.check": {"snapshot", "letter"},
    "jumble.covers": {"snapshot", "cap", "max_chunk"},
    "jumble.repair.list": {"snapshot", "cap", "max_chunk"},
    "jumble.nu": {"snapshot"},
    "jumble.corrections.check": {"snapshot", "catalogue", "corrections"},
    "jumble.correction.lookup": {"catalogue", "construction"},
    "jumble.checkpoint.check": {"snapshot", "test"},
    "jumble.scramble": {"policy", "token"},
    "macro.add": {"macro", "phase"},
    "macro.save": {"word", "name"},
    "operation.check": {"draft", "snapshot", "fields", "max_chunk"},
    "operation.preview": {"draft"},
    "operation.execute": {"preview", "token"},
    "view.preview.scrub": {"preview", "prefix", "revision"},
    "view.open": {"view", "source"},
    "view.close": {"view"},
    "recording.start": {"name"},
    "recording.stop": {"recording"},
    "recording.play": {"recording"},
    "recording.convert": {"recording", "target"},
    "twist.apply": {"cap", "rotation", "token"},
    "session.undo.partial": {"entry", "move", "token"},
    "operation.preview.commit": {"draft", "token"},
    "session.rollback": {"hint", "token"},
    "workspace.set": {"record", "payload", "provenance"},
    "set.save": {"members", "kind", "name"},
    "theme.select": set(),
    "theme.tune": {"family", "knobs"},
    "token.override.set": {"name", "value"},
    "token.override.reset": {"name"},
    "density.preset.set": {"preset"},
    "type.scale.set": {"preset"},
    "motion.preset.set": {"preset"},
    "motion.reduced.set": {"on"},
    "panel.mount": {"panel", "mount"},
    "panel.unmount": {"panel"},
    "radial.edit": {"radial", "entries"},
    "palette.pin": {"command", "pinned"},
    "palette.alias": {"command", "aliases"},
    "explore.set": {"on"},
    "view.preview.open": {"view", "preview"},
    "view.pin": {"view", "pinned"},
    "view.compare.wipe": {"view"},
    "history.dock.mode": {"mode"},
    "layout.undo": {"hint", "token"},
    "layout.redo": {"hint", "token"},
    "keymap.edit": {"keymap", "candidate"},
    "keymap.defaults": {"keymap"},
    "keymap.save": {"candidate"},
    "keymap.name": {"keymap", "name"},
    "file.keymap.apply": {"checked"},
    "work.layout.set": {"layout"},
    "work.layout.reset": set(),
}
SERVICES = [
    {"id": "kernel.snapshot.open", "kind": "service", "caller": "both", "lane": "none",
     "clauses": ["KN2.2", "LI.11"], "note": "Capture one immutable input under the caller's grants and run/epoch ownership."},
    {"id": "kernel.snapshot.close", "kind": "service", "caller": "both", "lane": "none",
     "clauses": ["KN2.2", "LI.11", "KI.4"], "note": "Release the supplied snapshot owner; no durable action."},
    {"id": "kernel.stream.next", "kind": "continuation", "caller": "both", "lane": "none",
     "clauses": ["KI.4", "KI.8", "DET.63"], "note": "Continue the admitted stream in its original invocation and lane; no new admission."},
    {"id": "kernel.value.retain", "kind": "service", "caller": "both", "lane": "none",
     "clauses": ["LI.13", "DET.57", "KI.4"], "note": "Create a charged owning alias of immutable data under the caller's grants."},
    {"id": "kernel.value.release", "kind": "service", "caller": "both", "lane": "none",
     "clauses": ["DET.58", "KI.4", "KI.5"], "note": "Release the named owner; acknowledgement does not assert physical deallocation."},
    {"id": "kernel.outcome", "kind": "service", "caller": "both", "lane": "none",
     "clauses": ["KN5.23", "LI.30", "KI.3"], "note": "Look up the permitted command/request receipt without a new invocation or replay."},
    {"id": "kernel.job_result", "kind": "service", "caller": "both", "lane": "none",
     "clauses": ["KN7.3", "LI.14", "LI.29"], "note": "Read the permitted result for the original command/job binding; never rerun it."},
    {"id": "kernel.undo_hint", "kind": "service", "caller": "host", "lane": "none",
     "clauses": ["LI.9", "LI.26"], "note": "Given session/epoch and direction: undo | redo, read one ActionHint or none for that direction in the shared durable order; the hint reserves nothing and grants no permission."},
    {"id": "language.publish_view", "kind": "service", "caller": "both", "lane": "view",
     "clauses": ["RPI.10", "LI.25", "KI.5"], "note": "Publish to the resolved ViewId under the view grant. Scheduled publication finalizes its original invocation and lane generation; standalone publication uses normal admission."},
]

# CM.18: the rows a program reads through a host bridge, each typed by puzzle and level (KN2.4). Table contracts for
# the bridges' draft rows at LP3's freeze, not a bridge, a measured cost or a profile grant.
READ_LEVELS = {"Full": {"labels"}, "jumbling": {"labels", "poses"}, "both": {"none"}}
READ_FIELDS = ("id", "puzzle", "level", "argument", "answer", "fields", "tariff", "codes", "clauses", "note")
CODE = re.compile(r"[A-Z]{2,3}[0-9]?:[a-z_]+\.[a-z0-9_]+")
READ_PROTOCOL = {
    "form": "read_snapshot<T>(\"<id>\") or read_snapshot<T>(\"<id>\", argument)",
    "order": "The host prices the read first and refuses a malformed argument there, and on the Full bridge a declared "
             "type it does not serve, at the cost of the read's step only. The run then charges the tariff and the "
             "host answers; a refusal after the charge keeps the charge.",
    "types": {"Recipe": "Word | List<Int> | List<record { kind: Text, moves: Word, node: Nat, orbit: Nat, sign: Int }>"},
    "tariff": {"jumbling": "177,120, one unit per piece, per pass over a state; result bound 2^24 for every row"},
    "codes": {"unserved": "LG3:capability.not_granted",
              "every": ["LG2:type.mismatch", "KN5:input.argument_form", "KN7:budget.kernel_cost", "KN7:budget.memory",
                        "KN7:budget.output", "KN7:resource_limit.host_exhausted"],
              "Full": ["KN5:input.retained_tool_needs_jumble_off"],
              "jumbling": ["KN5:input.menu_word_needs_jumble_on", "KN1:input.identity_mismatch",
                           "KN5:consistency.state_hash_mismatch"],
              "both": []},
    "clauses": ["KN2.3", "KN2.4", "LG2.8"],
}
READS = [
    {"id": "positions", "puzzle": "Full", "level": "labels", "argument": "List<Nat>",
     "answer": "List<record { home: List<Nat>, orbit: Int, piece: Nat, position: Int, slots: List<Nat> }>",
     "fields": "all",
     "tariff": "259,800 + 20 per piece named; result bound: that many of the largest records",
     "codes": ["KN2:input.named_object_invalid"],
     "clauses": ["KN2.4"],
     "note": "Pieces named by home position, 0 to 177,119, with repeats; any list of integers, such as a Set<Nat>. Each record gives the piece's orbit (-1 for a fixed cell centre), its home slots, the slots that hold its stickers now and the position whose slots hold them all (-1 when they are split)."},
    {"id": "labels", "puzzle": "Full", "level": "labels", "argument": "List<Nat>",
     "answer": "List<Nat>",
     "fields": "all",
     "tariff": "1 per slot named; result bound: that many of the largest labels",
     "codes": ["KN2:input.named_object_invalid"],
     "clauses": ["KN2.4"],
     "note": "Slots named, 0 to 259,799, with repeats; the label each holds at the head."},
    {"id": "effect", "puzzle": "Full", "level": "labels", "argument": "Recipe",
     "answer": "record { destination: List<Nat>, expansion_sha256: Text, moved_pieces: Nat, moved_stickers: Nat, net_sha256: Text, primitive_count: Nat, source: List<Nat>, support: List<record { orbit: Int, pieces: Nat, stickers: Nat }> }",
     "fields": "subset",
     "tariff": "259,800 + 1 per primitive of the recipe's expansion (0 when the kernel refuses the recipe); result bound: the largest answer with the declared fields",
     "codes": ["KN2:consistency.star_certificate"],
     "clauses": ["KN2.4"],
     "note": "A recipe's net source-to-destination map, its digests and lengths and its support per orbit; it reads no state. The kernel's own refusal of a recipe passes through."},
    {"id": "protection", "puzzle": "Full", "level": "labels", "argument": "record { lock: record { kind: Text, labels: List<Nat>, slots: List<Nat> }, mode: Text, recipe: Recipe }, with limit: Nat for Strict only",
     "answer": "record { checked: Nat, conflicts: List<Nat>, expansion_sha256: Text, level: Text, primitive_count: Nat, refused_prefix: Int }",
     "fields": "all",
     "tariff": "the lock's pairs + the expansion's primitives + 259,800 x (generators + 1), with 0 generators for Net and min(primitives, 1,200) for Strict; result bound: the empty answer and one slot per pair",
     "codes": ["KN2:input.named_object_invalid", "KN2:consistency.star_certificate"],
     "clauses": ["KN2.4", "LB2.1"],
     "note": "A captured lock judged Net or Strict against a recipe at the head; mode is net or strict, and level is labels. The kernel's own refusal of a recipe passes through."},
    {"id": "replay", "puzzle": "Full", "level": "labels", "argument": "Recipe",
     "answer": "record { conflicts: List<Nat>, expansion_sha256: Text, level: Text, net_sha256: Text, primitive_count: Nat, protected: List<Nat>, refused_prefix: Int }",
     "fields": "all",
     "tariff": "2 x 259,800 + 2 per primitive of the expansion (0 when the kernel refuses the recipe); result bound: the largest replay answer",
     "codes": ["KN2:consistency.star_certificate"],
     "clauses": ["KN2.4", "KN2.7"],
     "note": "A recipe replayed prefix by prefix at the head under the session's protected orbits, keeping the head; level is labels. The kernel's own refusal of a recipe passes through."},
    {"id": "setups", "puzzle": "Full", "level": "labels", "argument": "record { orbit: Nat, position: Nat }",
     "answer": "record { buffers: List<Nat>, family: Text, orbit: Nat, position: Nat, setups: List<record { frame: List<Nat>, node: Nat, setup: Word }>, table_sha256: Text }",
     "fields": "all",
     "tariff": "259,800 + 3 x 259,800 per setup the named position has in the table; result bound: that many of the largest setups",
     "codes": ["LG3:capability.not_granted", "KN1:input.identity_mismatch", "KN2:input.named_object_invalid", "KN2:consistency.star_certificate"],
     "clauses": ["BND.1", "LB2.1"],
     "note": "The certified setups for a target the solver names, in the table's order, none ranked; only the setup-search exception reads it. Without a certified table from the host it is not granted."},
    {"id": "structure.compare", "puzzle": "Full", "level": "labels", "argument": "record { first: Nat, second: Nat }",
     "answer": "record { first: Nat, second: Nat, steps: Nat }",
     "fields": "all",
     "tariff": "1; result bound: three integers",
     "codes": ["KN2:input.named_object_invalid"],
     "clauses": ["KN2.3", "LG2.8"],
     "note": "The fewest face steps between two cells, 0 to 599; model data. The command row of the same name is its entry."},
    {"id": "filter.preview", "puzzle": "Full", "level": "labels", "argument": "record { cell: Nat, layer: Nat }",
     "answer": "record { cell: Nat, layer: Nat, pieces: Nat, slots: Nat }",
     "fields": "all",
     "tariff": "259,800; result bound: the largest answer",
     "codes": ["KN2:input.named_object_invalid"],
     "clauses": ["KN2.3", "LG2.8"],
     "note": "The counts of a layer predicate: the positions with a home slot layer face steps from cell, and their home slots; model data. The command row of the same name is its entry."},
    {"id": "structure.layer", "puzzle": "Full", "level": "labels", "argument": "record { cell: Nat, layer: Nat }",
     "answer": "record { cell: Nat, layer: Nat, pieces: List<Nat>, slots: List<Nat> }",
     "fields": "all",
     "tariff": "259,800 + 177,120 + 259,800; result bound: an answer holding every position and slot",
     "codes": ["KN2:input.named_object_invalid"],
     "clauses": ["KN2.3", "LG2.8"],
     "note": "The members that filter.preview counts, by home position and home slot, in increasing order; model data. The command row of the same name prepares its predicate."},
    {"id": "orbits", "puzzle": "Full", "level": "labels", "argument": "List<Int>",
     "answer": "List<record { orbit: Int, pieces: List<Nat> }>",
     "fields": "all",
     "tariff": "259,800 + 7,200 per orbit named; result bound: that many of the largest records",
     "codes": ["KN2:input.named_object_invalid"],
     "clauses": ["KN2.3", "LB2.1"],
     "note": "The pieces of named orbits, -1 to 34 (-1 for the fixed cell centres), with repeats; model data, the orbit that the positions row gives each piece at every state."},
    {"id": "journal", "puzzle": "both", "level": "none", "argument": "record { cursor: Text, from: Text, limit: Nat }",
     "answer": "record { entries: List<record { event: Text, exit: Text, kind: Text, letters: Nat, solved: Bool }>, next: Text }",
     "fields": "all",
     "tariff": "1 per event of the page asked (limit, at most 4,096); result bound: limit of the largest entries and the longest cursor",
     "codes": ["KN2:stale.revision_gone"],
     "clauses": ["KN4.3"],
     "note": "KN4.3's journal query: a page from from (the head when empty) toward the start on that event's line, newest first, limit 1 to 4,096. It answers with Jumble on or off, replays nothing and changes nothing."},
    {"id": "heights", "puzzle": "jumbling", "level": "poses", "argument": None,
     "answer": "record { lattice_key: Text, poses: Map<Text,record { branches: List<Int>, count: Nat, domain: Text, heights: List<Nat> }> }",
     "fields": "subset",
     "tariff": "1 pass",
     "codes": [],
     "clauses": ["KN2.4", "JI.6"],
     "note": "Per exact pose its domain, branches and heights, with the lattice key."},
    {"id": "height_counts", "puzzle": "jumbling", "level": "poses", "argument": None,
     "answer": "record { max_above_6: List<Nat>, n_0: Nat, n_2: Nat, n_4: Nat, n_6: Nat, n_above_6: Nat }",
     "fields": "subset",
     "tariff": "1 pass",
     "codes": [],
     "clauses": ["KN2.4", "JI.6"],
     "note": "The height counts n_0, n_2, n_4, n_6 and above 6, with the maximum above 6."},
    {"id": "replay", "puzzle": "jumbling", "level": "poses", "argument": "MenuWord",
     "answer": "record { complete: Bool, first_refused: List<Nat>, rows: List<record { after: Text, max_left_height: List<Nat>, phi_after: List<Nat> }>, start: record { digest: Text, max_left_height: Nat, phi: Nat } }",
     "fields": "subset",
     "tariff": "1 + 1 per letter of the word",
     "codes": ["LB1:input.incompatible_alphabet"],
     "clauses": ["KN2.4", "JI.10"],
     "note": "A menu word replayed letter by letter at the head: each prefix's pose digest, left height and Phi, or the first refused prefix."},
    {"id": "word_check", "puzzle": "jumbling", "level": "poses", "argument": "record { unit: List<Nat>, word: MenuWord }",
     "answer": "record { admissible: Bool, applied_letters: Nat, lattice_pieces_lifted_off: List<Nat>, others: Nat, others_exact: List<Nat>, others_landed: List<Nat>, unit_on_lattice: List<Nat>, unit_size: List<Nat> }",
     "fields": "subset",
     "tariff": "1 + 1 per letter of the word (the unit adds none)",
     "codes": ["LB1:input.incompatible_alphabet", "KN2:input.named_object_invalid"],
     "clauses": ["KN2.4", "JI.11"],
     "note": "A whole word's observation for a unit the solver names, not an existence test. The units entry reads it with others_changed_ids, unit and unit_landed in place of unit_on_lattice and unit_size."},
    {"id": "protection", "puzzle": "jumbling", "level": "poses", "argument": "record { mode: Text, pieces: List<Nat>, word: MenuWord }",
     "answer": "record { first_refused_prefix: List<Nat>, level: Text, mode: Text, pieces: List<Nat>, protected: List<Nat>, verdict: Text }",
     "fields": "subset",
     "tariff": "1 + 1 per letter of the word (the pieces add none)",
     "codes": ["LB1:input.incompatible_alphabet", "KN2:input.named_object_invalid"],
     "clauses": ["KN2.4", "KN1.11"],
     "note": "Net or Strict for named pieces against a menu word at level poses; mode is net or strict. The derivation's own refusal passes through."},
    {"id": "nu", "puzzle": "jumbling", "level": "labels", "argument": None,
     "answer": "record { reason: List<Text>, values: List<Int> }",
     "fields": "subset",
     "tariff": "1 pass",
     "codes": [],
     "clauses": ["KN2.4", "JI.13"],
     "note": "nu at the head in A5's fixed C5 convention, or its reason when it is undefined off the lattice; no correction is selected. nu's own refusal passes through."},
    {"id": "correction_set_check", "puzzle": "jumbling", "level": "labels", "argument": "List<record { id: Nat, m: Int }>",
     "answer": "record { reason: List<Text>, values: List<Int> }",
     "fields": "subset",
     "tariff": "1 + 1 per entry of the drafted set",
     "codes": [],
     "clauses": ["KN2.4", "JI.13"],
     "note": "nu after the solver's drafted set of correction IDs and multiplicities. The check's own refusal passes through."},
    {"id": "seam_facts", "puzzle": "jumbling", "level": "poses", "argument": "Nat | Text",
     "answer": "record { totals: List<record { aligned: Nat, off_grip_mismatches: Nat, on_grip_mismatches: Nat, total_N: Nat }> }",
     "fields": "subset",
     "tariff": "2 passes, for one cap or for \"all\"",
     "codes": ["KN2:input.named_object_invalid"],
     "clauses": ["JI.7", "JI.8"],
     "note": "The seam totals for a cap or for \"all\", including off-grip cells; LP3's library reads \"all\"."},
    {"id": "rim_summary", "puzzle": "jumbling", "level": "poses", "argument": None,
     "answer": "record { complete: Bool, digest: Text, grip_n: List<Nat>, grip_n_undefined: List<Text>, rows: List<record { A_c: List<Nat>, F_c: List<Nat>, In_c: List<Nat>, N_c: List<Nat>, Out_c: List<Nat>, at_floor: List<Bool>, blocked: List<Bool>, cap: Nat, status: Text, undefined: List<Text> }> }",
     "fields": "subset",
     "tariff": "601 passes: 1 + one per cap (600)",
     "codes": [],
     "clauses": ["KN2.4", "JI.6"],
     "note": "Per cap N_c, In_c, Out_c, A_c, F_c, blocked and floor, each with its definedness."},
    {"id": "covers", "puzzle": "jumbling", "level": "poses", "argument": "Nat",
     "answer": "record { answers: List<record { c_covers_d: record { counts: List<record { aligned_inside: Nat, aligned_outside: Nat, inside: Nat, outside: Nat }>, reason: List<Text>, straddlers: List<Nat> }, cap: Nat, d_covers_c: record { counts: List<record { aligned_inside: Nat, aligned_outside: Nat, inside: Nat, outside: Nat }>, reason: List<Text>, straddlers: List<Nat> } }>, domain: List<Nat>, input_digest: Text, message: List<Text>, parameters: record { cap: Nat, threshold: Nat }, status: Text }",
     "fields": "subset",
     "tariff": "82 passes: 1 + the named cap and its 80 meeting caps",
     "codes": ["KN2:input.named_object_invalid"],
     "clauses": ["KN2.4", "JI.12"],
     "note": "Covers at a cap the solver names, in both directions; a blocked or closed cap answers status error, not a refusal."},
    {"id": "correction_lookup", "puzzle": "jumbling", "level": "labels", "argument": "Nat",
     "answer": "record { family: Text, id: Nat, letters: List<List<Nat>>, row: List<Nat> }",
     "fields": "subset",
     "tariff": "1 pass",
     "codes": ["KN2:input.named_object_invalid", "KN2:input.menu_binding_mismatch"],
     "clauses": ["JI.13"],
     "note": "One shipped correction construction by ID, under the catalogue's menu binding."},
    {"id": "classify", "puzzle": "jumbling", "level": "poses", "argument": "MenuWord",
     "answer": "record { certificate: List<record { grip: Nat, h_above: Q5, h_below: Q5, orbit: Nat, piece: Nat, point_above: List<Q5>, point_below: List<Q5>, points_checked_against_all_constraints: Bool, transport: Nat, vertex_above: Nat, vertex_below: Nat }>, code: List<Text>, grip: Nat, moved: Nat, status: Text, uncertain: Nat }",
     "fields": "subset",
     "tariff": "2 passes",
     "codes": ["LB1:input.incompatible_alphabet"],
     "clauses": ["KN2.4", "JI.6"],
     "note": "The outcome of one named twist, a menu word of exactly one letter, with its certificates; it changes nothing."},
    {"id": "survey", "puzzle": "jumbling", "level": "poses", "argument": "Nat",
     "answer": "record { cap: Nat, status: Text }",
     "fields": "subset",
     "tariff": "2 passes",
     "codes": ["KN2:input.named_object_invalid"],
     "clauses": ["KN2.4", "LB3.5"],
     "note": "One named grip's certified status; \"all\" is not served."},
    {"id": "straddlers", "puzzle": "jumbling", "level": "poses", "argument": "Nat",
     "answer": "record { cap: Nat, certificates: List<record { grip: Nat, h_above: Q5, h_below: Q5, orbit: Nat, piece: Nat, point_above: List<Q5>, point_below: List<Q5>, points_checked_against_all_constraints: Bool, transport: Nat, vertex_above: Nat, vertex_below: Nat }>, pieces: List<Nat>, status: Text, uncertain: Nat }",
     "fields": "subset",
     "tariff": "2 passes",
     "codes": ["KN2:input.named_object_invalid"],
     "clauses": ["KN2.4", "JI.6"],
     "note": "The straddlers of a cap the solver names, each with its certificate pair."},
    {"id": "units", "puzzle": "jumbling", "level": "poses", "argument": "List<Nat> | Text",
     "answer": "record { complete: Bool, units: List<record { branches: List<Int>, domain: Text, heights: List<Nat>, members: List<Nat>, pose: Text }> }",
     "fields": "subset",
     "tariff": "2 passes",
     "codes": ["KN2:input.named_object_invalid"],
     "clauses": ["JI.9"],
     "note": "The exact-pose units of the off pieces, for \"all\" or for named pieces; LP3's library reads a named list with units only."},
    {"id": "repair_list", "puzzle": "jumbling", "level": "poses", "argument": "Nat",
     "answer": "record { input_digest: Text, message: List<Text>, parameters: record { cap: Nat, contract_revision: Text, menu_identity: List<Text>, order_digest: List<Text>, tie_rule: Text }, rows: List<record { d: Nat, delta: Int, x: Nat, y: Nat, z: Nat }>, status: Text }",
     "fields": "subset",
     "tariff": "1 + R passes, with R = 0 when the state's menu has no items and otherwise 1 + 80 + (items - 1) x (1 + 3 x 80), its worst case under the menu",
     "codes": ["KN2:input.named_object_invalid"],
     "clauses": ["KN2.4", "JI.12", "BND.2"],
     "note": "The owner's three-twist exception: the pairs at a cap the solver names, in a neutral order. A blocked or closed cap, or free mode, answers status error with no rows."},
    {"id": "sticker_test", "puzzle": "jumbling", "level": "labels", "argument": None,
     "answer": "StickerTestVerdict",
     "fields": "all",
     "tariff": "1 pass",
     "codes": [],
     "clauses": ["KN2.4", "LB3.2"],
     "note": "A5's sticker test: holds, fails or unestablished."},
    {"id": "retained_checkpoint", "puzzle": "jumbling", "level": "poses", "argument": "Word",
     "answer": "RetainedCheckpointVerdict",
     "fields": "all",
     "tariff": "1 + 1 per letter of the witness word",
     "codes": [],
     "clauses": ["KN2.4", "LB3.2"],
     "note": "The retained-state checkpoint for a witness word of 600-cell-Full's letters, signed primitive IDs 1 to 1,200."},
    {"id": "label_checkpoint", "puzzle": "jumbling", "level": "labels", "argument": None,
     "answer": "LabelCheckpointVerdict",
     "fields": "all",
     "tariff": "2 passes (the labels and nu)",
     "codes": [],
     "clauses": ["KN2.4", "LB3.2"],
     "note": "The label checkpoint, from the labels and nu."},
]

rows = []
def c(cid, label, kind, src, *, gates=(), preview="none", undo="none", contexts=("any",),
      repeat=False, job=False, key04=None, note=None, lane=None, args=None, source="DT-M1 C1; signed dispositions"):
    rows.append({"id": cid, "label": label, "kind": kind, "inventory": list(src),
                 "gates": list(gates), "preview": preview, "undo": undo,
                 "contexts": list(contexts), "repeat": repeat, "job": job,
                 "key_0_4": key04, "disposition": [], "note": note,
                 "lane": lane or ("commit" if kind == "commit" else "exclusive" if job else
                                  "view" if kind == "view" else "none"),
                 "arguments": args or {}, "source": source,
                 "slot_map": "rebind" if cid == "grip.rebind" else "preserve"})

# session
c("session.new", "New solve", "commit", ["T-08"], gates=["confirm"], undo="journal")
c("session.resume", "Resume", "app", ["T-09"])
c("session.timer.start", "Start timer", "workspace", ["T-10"], undo="none")
c("session.timer.stop", "Stop timer", "workspace", ["T-10"], undo="none")
c("session.scramble", "Scramble (staged)", "workspace", ["T-11"], undo="workspace",
  note="stages into the operation; the commit is operation.execute")
c("session.reset", "Reset puzzle (recoverable)", "commit", ["T-12"], gates=["confirm"], undo="journal")
c("session.undo", "Undo", "commit", ["T-14"], undo="journal", key04="Commands > Undo")
c("session.redo", "Redo", "commit", ["T-14"], undo="journal", key04="Commands > Redo")
c("session.checkpoint.save", "Save checkpoint", "commit", ["T-15"])
c("session.checkpoint.restore", "Restore checkpoint", "commit", ["T-15"], gates=["confirm"], undo="journal")
c("session.report", "Progress report", "query", ["T-16"])
# files
c("file.log.export", "Export C600 or MPUlt log", "file", ["T-17"], job=True, key04="Ctrl+Alt+O/L (Functions)")
c("file.log.check", "Check a log file", "query", ["T-18"], gates=["model_match"], job=True)
c("file.log.import", "Import a checked log", "commit", ["T-18"], gates=["file_check", "confirm"],
  undo="journal", job=True, note="keeps preferences; leaves a recovery checkpoint")
c("file.keymap.export", "Export keymap", "file", ["T-19"], key04="Ctrl+Alt+E (Functions)")
c("file.keymap.check", "Check keymap file", "query", ["T-19"], gates=["model_match"])
c("file.keymap.apply", "Apply keymap file", "workspace", ["T-19"], gates=["file_check", "confirm"], undo="workspace",
  key04="Ctrl+Alt+I (Functions)")
# operation (solve)
c("solve.open", "Open Solve", "view", ["T-20"])
c("operation.phase.select", "Select phase Prepare / Macro / Cleanup", "view", ["T-21"], contexts=["solve"], key04="Ctrl+1/2/3")
c("operation.phase.edit", "Edit phase turns", "workspace", ["T-22"], undo="workspace",
  contexts=["solve"], key04="F4")
c("operation.cleanup.from_prepare", "Cleanup = inverse of Prepare", "workspace", ["T-23"], undo="workspace",
  contexts=["solve"], key04="Ctrl+Shift+I")
c("operation.check", "Check full effect", "query", ["T-24"], job=True, contexts=["solve", "operation"])
c("operation.preview", "Preview", "query", ["T-24"], job=True, contexts=["operation"],
  key04="Ctrl+Enter")
c("operation.execute", "Execute", "commit", ["T-24"], gates=["fresh_preview", "protection"],
  preview="required", undo="journal", contexts=["operation"], key04="Ctrl+Shift+Enter")
c("operation.preview.cancel", "Cancel preview", "workspace", ["T-24"], contexts=["operation"])
c("job.stop", "Stop running check", "app", ["T-25"], repeat=False, key04="Shift+Escape",
  note="a request; acceptance is not completion")
c("operation.new", "New operation", "workspace", ["T-26"], undo="workspace", key04="Ctrl+N")
c("operation.reuse", "Reuse steps", "workspace", ["T-26"], undo="workspace", key04="Ctrl+Alt+R")
c("view.forecast.select", "Show Actual / After Prepare / After Macro / After Cleanup", "view", ["T-27"], contexts=["solve"])
c("view.cycle.mode", "Cycle view Current / Operation / After", "view", ["T-28"], contexts=["solve"])
c("view.effect.scope", "Effect scope Macro body / All steps", "view", ["T-29"], contexts=["solve"])
c("view.local", "Local view", "view", ["T-30"])
c("view.global", "Global view", "view", ["T-30"])
c("target.next.activate", "Activate Next", "workspace", ["T-31"], undo="workspace", key04="Ctrl+Alt+N")
c("target.next.set", "Set locked Next", "workspace", ["T-31"], undo="workspace")
c("object.inspect", "Inspect identity or position", "view", ["T-32", "T-61"], key04="Enter; Shift+click")
c("object.actions", "Object action menu", "view", ["T-32"], key04="Shift+F10")
c("object.inspect.clear", "Clear inspection", "view", ["T-61"])
c("view.entry_slots", "Entry slots", "view", ["T-33"], contexts=["solve"])
c("buffer.assign", "Assign buffer role A/B", "workspace", ["T-34"], undo="workspace", contexts=["solve"])
c("buffer.analyse", "Buffer analyser", "query", ["T-34"], job=True, contexts=["solve"])
c("block.add_current", "Add current state as block requirement", "workspace", ["T-35"], undo="workspace")
c("block.add_home", "Add to Home block", "workspace", ["T-35"], undo="workspace")
c("intent.set", "Set work intent", "workspace", ["T-36"], undo="workspace", contexts=["solve"])
c("view.residuals", "Residuals and invariant diagnostics", "query", ["T-37"])
c("view.findings", "Findings", "view", ["T-38", "T-39"], key04="Shift+F12")
c("orbit.switch", "Switch work orbit", "workspace", ["T-40"], undo="workspace")
c("view.recommendations", "Recommendations", "query", ["T-41"], job=True,
  note="ranking only; never grants permission or executes")
c("view.names", "Names and addresses", "view", ["T-42"])
# macros
c("macro.base.open", "Open Macro Base", "view", ["T-43"], key04="F3")
c("macro.select", "Select macro (inspect)", "view", ["T-43", "T-44"], contexts=["macro_base", "solve"], key04="Ctrl+Alt+1/2")
c("macro.add", "Add selected macro to Macro phase", "workspace", ["T-43"], undo="workspace", key04="Ctrl+M")
c("worksheet.save", "Save worksheet", "workspace", ["T-45"], undo="workspace")
c("worksheet.load", "Load worksheet", "workspace", ["T-45"], undo="workspace")
c("macro.compare", "Compare candidates", "query", ["T-45"], job=True)
c("endgame.configure", "Configure endgame family", "workspace", ["T-46"], undo="workspace")
c("macro.save", "Save as macro", "workspace", ["T-46"], gates=["confirm"], undo="workspace",
  note="saving does not select, insert or execute")
c("macro.variant.create", "Create reference variant", "workspace", ["T-47"], undo="workspace")
# protection
c("protection.policy.set", "Set protection policy (net or strict)", "workspace", ["T-48"], undo="workspace",
  note="invalidates any open preview")
c("protection.position.protect", "Protect position", "workspace", ["T-50"], undo="workspace")
c("protection.position.release", "Release position", "workspace", ["T-50"], gates=["confirm"], undo="workspace")
c("protection.orbit.unlock", "Unlock orbit", "workspace", ["T-49"], gates=["confirm"], undo="workspace")
# input
c("keyboard.toggle", "Onscreen keyboard", "view", ["T-51"], key04="F9")
c("grip.select", "Grip (cap and frame)", "workspace", ["T-52"], contexts=["puzzle", "keyboard"])
c("twist.apply", "Twist", "commit", ["T-52", "T-60"], gates=["protection"], undo="journal",
  contexts=["puzzle", "keyboard"], note="live only; ONE.78 assumes no preview, pending owner confirmation")
c("twist.inverse.toggle", "Inverse for clicks", "view", ["T-52"], contexts=["keyboard"])
c("input.destination.set", "Input destination live or draft", "workspace", ["T-53"])
c("grip.anchor.set", "Choose grip anchor", "workspace", ["T-54"], undo="workspace",
  note="DT-M1 C7: replaces bank.select; choosing an anchor does not apply a slot map")
c("grip.rebind", "Apply reviewed grip map", "workspace", ["T-55"], undo="workspace",
  note="DT-M1 K2 and Q3(a): explicit rebind with transported slot order and frames")
c("keymap.edit", "Change key", "workspace", ["T-57"], gates=["confirm"], undo="workspace")
c("command.find", "Find command", "view", ["T-56", "T-58"], key04="F1; Ctrl+F1 back")
# view
c("camera.rotate", "Rotate camera (3D/4D)", "view", ["T-60"], repeat=True, contexts=["puzzle"], key04="drag; Shift+left drag")
c("filter.open", "Open piece filter", "view", ["T-62"], key04="F5")
c("filter.preview", "Preview filter counts", "query", ["T-62"], contexts=["filter"])
c("filter.apply", "Apply filter", "workspace", ["T-62"], gates=["fresh_filter_preview"], preview="filter",
  undo="workspace", contexts=["filter"])
c("display.frame.toggle", "Show or hide 600-cell frame", "view", ["T-63"])
c("display.adaptive.toggle", "Adaptive motion detail", "view", ["T-64"])
c("display.adjust", "Size, field of view, lighting", "view", ["T-65"])
c("view.reset", "Reset view", "view", ["T-65"])
# structure (0.3 reference)
c("structure.find", "Find colour or cell", "query", ["T-67"])
c("structure.compare", "Compare cells", "query", ["T-67"])
c("structure.layer", "Prepare layer predicate", "workspace", ["T-68"], undo="workspace", contexts=["filter"])
c("view.cell_views", "Auxiliary cell views", "view", ["T-69"])
c("camera.center", "Center main view on cell", "view", ["T-70"])
c("set.save", "Save frozen set", "workspace", ["T-71"], undo="workspace")

# Code-only purposes and the DT-M1 corrections. No jumbling rows belong here.
c("operation.phase.append_twist", "Append resolved twist to phase", "workspace", ["T-52", "T-60"],
  undo="workspace", contexts=["puzzle", "keyboard"],
  note="DT-M1 C7: edits a draft only; protection is checked on the complete operation")
c("workspace.reset", "Reset work context", "workspace", [], gates=["confirm"], undo="workspace",
  note="Preserves puzzle history, personal tools and protection")
c("session.branch.select", "Continue recorded branch", "commit", [], gates=["confirm", "model_match"], undo="journal")
c("session.history", "Inspect recorded history", "query", [])
c("session.completion", "Read recorded completion", "query", [])
c("session.practice", "Open full-model practice copy", "commit", [], gates=["model_match", "protection", "confirm"],
  undo="journal", note="Explicit fixture action in an isolated practice session; never startup")
c("file.session.backup", "Back up session", "file", [], job=True)
c("file.proof.export", "Export verified proof session", "file", [], job=True)
c("file.certificate.export", "Export macro certificate", "file", [], job=True)
c("file.macro.export", "Export selected macro records", "file", [], job=True)
c("file.macro.check", "Check macro library file", "query", [], gates=["model_match"], job=True)
c("file.macro.import", "Import checked macro records", "workspace", [], gates=["file_check", "confirm"], undo="workspace")
c("app.exit", "Save and leave application", "app", [])
c("app.reconnect", "Reconnect verified display", "app", [], lane="exclusive")
c("help.open", "Read contextual help", "view", [])
c("work.surface.show", "Reach work surface", "view", [])
c("work.surface.close", "Leave expanded work surface", "view", [],
  note="Preserves the two baseline linked views, drafts and previews")
c("work.layout.set", "Adjust workspace layout", "view", [])
c("work.layout.reset", "Recover workspace layout", "view", [])
c("work.fullscreen.toggle", "Toggle fullscreen", "view", [])
c("transient.close", "Close innermost transient", "view", [],
  note="Escape never stops a job, cancels a preview or drops a draft")
c("list.move", "Move list focus", "view", [], repeat=True)
c("list.row.copy", "Copy named result row", "workspace", [], undo="workspace",
  note="O9 row copy identifies the result and absolute object; never executes")
c("camera.zoom", "Scale view", "view", [], repeat=True)
c("view.preview.scrub", "Scrub identified preview", "view", [], repeat=True)
c("view.local.configure", "Set Local origin and neighborhood", "view", [])
c("display.references.set", "Set reference visibility", "view", [])
c("display.mode.set", "Set rendering presentation", "view", [])
c("object.copy", "Copy typed object address", "app", [])
c("object.locate", "Locate named object", "view", [])
c("target.current.set", "Set current piece and destination", "workspace", [], undo="workspace")
c("target.next.clear", "Clear next-work bookmark", "workspace", [], undo="workspace")
c("target.frame.set", "Set required target frame", "workspace", [], undo="workspace")
c("block.reference.set", "Set block reference", "workspace", [], undo="workspace")
c("block.remove", "Remove named block requirement", "workspace", [], undo="workspace")
c("protection.orbit.set", "Set protected orbits", "workspace", [], gates=["confirm"], undo="workspace")
c("grip.behavior.set", "Choose held or latched grip", "workspace", [], undo="workspace")
c("grip.frames", "Inspect proper cap frames", "query", [])
c("grip.rotations", "Inspect legal cap rotations", "query", [])
c("grip.members", "Inspect complete pieces and slots in cap", "query", [])
c("keymap.defaults", "Stage default bindings", "workspace", [], undo="workspace")
c("keymap.save", "Save checked binding edits", "workspace", [], gates=["confirm"], undo="workspace")
c("keymap.name", "Name saved key map", "workspace", [], undo="workspace")
c("keyboard.disclosure.set", "Inspect or open disclosed keys", "workspace", [], undo="workspace")
c("filter.edit", "Edit work-filter predicate", "workspace", [], undo="workspace")
c("filter.preset", "Stage named work-filter preset", "workspace", [], undo="workspace")
c("filter.save", "Save named filter", "workspace", [], undo="workspace")
c("filter.load", "Load filter for review", "workspace", [], undo="workspace")
c("structure.select", "Select canonical structural cell", "workspace", [], undo="workspace")
c("structure.adjacency", "Inspect cell adjacency", "query", [])
c("structure.vertex", "Inspect incident cells at vertex", "query", [])
c("structure.filter.compose", "Compose structural filter", "workspace", [], undo="workspace")
c("macro.replace", "Replace phase with selected macro", "workspace", [], undo="workspace")
c("macro.metadata.set", "Edit macro metadata", "workspace", [], undo="workspace")
c("macro.filter", "Find macros by recorded facts", "query", [])
c("macro.library.check", "Check selected library effects", "query", [], job=True,
  note="Explicit inspection under U6-031/2; automated bookkeeping U3-085 contributes no command")
c("macro.compose", "Compose explicitly chosen macros", "workspace", [], undo="workspace")
c("macro.operand.set", "Choose composition operand", "workspace", [], undo="workspace")
c("macro.reference.check", "Check selected reference variant", "query", [], job=True)
c("endgame.choices", "Inspect chosen family's parameters", "query", [])
c("endgame.check", "Expand and check chosen construction", "query", [], job=True,
  note="Family and parameters chosen by the solver; signed 3 October ruling, no insertion planner")
c("reference.set", "Set explicit legal reference word", "workspace", [], undo="workspace")
c("worksheet.compare", "Compare saved worksheet", "query", [], job=True)
c("view.cycle.select", "Inspect named cycle", "view", [])
c("view.cycle.page", "Page cycle inspection", "view", [])
c("view.reason", "Inspect named review reason", "view", [])
c("view.journal.delta", "Inspect recorded step effect", "query", [])

# Absolute invocation arguments, not legacy keys or implicit mutable selection.
ARGUMENTS = {
    "twist.apply": {"cap": "CapId", "rotation": "ExactRotationRef"},
    "operation.phase.append_twist": {"cap": "CapId", "rotation": "ExactRotationRef", "phase": "PhaseId"},
    "operation.phase.select": {"phase": "PhaseId"},
    "operation.phase.edit": {"phase": "PhaseId", "word": "WordRef"},
    "operation.execute": {"preview": "PreviewId", "token": "UserActionToken"},
    "operation.preview": {"draft": "ResolvedDraftRef"},
    "operation.check": {"draft": "ResolvedDraftRef"},
    "input.destination.set": {"destination": "live|draft", "phase": "PhaseId?"},
    "grip.select": {"cap": "CapId", "frame": "FrameRef"},
    "grip.anchor.set": {"piece": "PieceId"},
    "grip.rebind": {"anchor": "PieceId", "map": "TransportedSlotMap", "expected_map": "Digest"},
    "grip.frames": {"cap": "CapId"}, "grip.rotations": {"cap": "CapId"}, "grip.members": {"cap": "CapId"},
    "grip.behavior.set": {"behavior": "hold|latch"},
    "buffer.assign": {"role": "A|B", "position": "PositionId"},
    "target.current.set": {"piece": "PieceId", "destination": "PositionId"},
    "target.next.set": {"piece": "PieceId", "destination": "PositionId"},
    "target.next.activate": {"bookmark": "BookmarkRef"},
    "target.next.clear": {"bookmark": "BookmarkRef"},
    "target.frame.set": {"position": "PositionId", "frame": "ExactFrameRef"},
    "object.inspect": {"object": "CanonicalObjectRef"},
    "object.locate": {"object": "CanonicalObjectRef", "view": "ViewId"},
    "object.copy": {"object": "CanonicalObjectRef", "form": "typed|canonical"},
    "object.actions": {"object": "CanonicalObjectRef"},
    "orbit.switch": {"orbit": "OrbitId"}, "intent.set": {"intent": "IntentId"},
    "macro.add": {"macro": "MacroRevisionRef", "phase": "PhaseId"},
    "macro.replace": {"macro": "MacroRevisionRef", "phase": "PhaseId"},
    "macro.select": {"macro": "MacroRevisionRef"},
    "macro.save": {"word": "WordRef", "name": "Text"},
    "macro.variant.create": {"macro": "MacroRevisionRef", "transform": "ExactTransformRef"},
    "macro.metadata.set": {"macro": "MacroRevisionRef", "metadata": "MacroMetadata"},
    "macro.compare": {"macros": "MacroRevisionRefs"},
    "macro.compose": {"operands": "WordRefs", "composition": "inverse|concat|conjugate|commutator"},
    "macro.operand.set": {"operand": "OperandId", "word": "WordRef"},
    "reference.set": {"word": "WordRef"},
    "endgame.configure": {"family": "FamilyId", "parameters": "ExactFamilyParameters"},
    "endgame.check": {"family": "FamilyId", "parameters": "ExactFamilyParameters"},
    "camera.rotate": {"view": "ViewId", "rotation": "DisplayRotation"},
    "camera.zoom": {"view": "ViewId", "scale": "DisplayScale"},
    "camera.center": {"view": "ViewId", "cell": "CellId"},
    "view.reset": {"view": "ViewId"},
    "view.forecast.select": {"phase": "actual|prepare|macro|cleanup"},
    "view.effect.scope": {"scope": "body|complete"},
    "view.cycle.mode": {"mode": "current|operation|after"},
    "view.cycle.select": {"result": "ResultRef", "cycle": "CycleId"},
    "view.cycle.page": {"result": "ResultRef", "page": "PageId"},
    "view.reason": {"result": "ResultRef", "reason": "ReasonId"},
    "view.preview.scrub": {"preview": "PreviewId", "prefix": "WordPosition"},
    "list.move": {"list": "ListId", "entry": "EntryId"},
    "list.row.copy": {"result": "ResultRef", "object": "CanonicalObjectRef"},
    "work.surface.show": {"surface": "SurfaceId"}, "work.surface.close": {"surface": "SurfaceId"},
    "structure.select": {"cell": "CellId"}, "structure.compare": {"cells": "CellIdPair"},
    "structure.adjacency": {"cell": "CellId", "radius": "Nat"},
    "structure.vertex": {"vertex": "VertexId"},
    "structure.layer": {"cell": "CellId", "layer": "Nat"},
    "protection.policy.set": {"policy": "net|strict"},
    "protection.position.protect": {"position": "PositionId", "mode": "position|exact"},
    "protection.position.release": {"position": "PositionId", "mode": "position|exact|both"},
    "protection.orbit.set": {"orbits": "OrbitIds"}, "protection.orbit.unlock": {"orbit": "OrbitId"},
    "job.stop": {"job": "JobId"},
    "session.resume": {"session": "SessionRef"},
    "session.new": {"session": "SessionRef"}, "session.reset": {"session": "SessionRef"},
    "session.scramble": {"seed": "Seed", "length": "Nat", "phase": "PhaseId"},
    "session.checkpoint.save": {"name": "Text", "revision": "RevisionRef"},
    "session.checkpoint.restore": {"checkpoint": "CheckpointRef"},
    "session.branch.select": {"head": "JournalHeadRef"},
    "session.practice": {"fixture": "FixtureRef", "session": "PracticeSessionRef"},
    "session.completion": {"completion": "CompletionRecordRef"},
    "file.log.export": {"history": "JournalRangeRef", "format": "LogFormat", "destination": "FileRef"},
    "file.log.check": {"file": "FileDigestRef"},
    "file.log.import": {"checked": "CheckedFileRef"},
    "file.keymap.export": {"keymap": "KeymapRevisionRef", "destination": "FileRef"},
    "file.keymap.check": {"file": "FileDigestRef"},
    "file.keymap.apply": {"checked": "CheckedFileRef"},
    "file.session.backup": {"session": "SessionRef", "destination": "FileRef"},
    "file.proof.export": {"history": "JournalRangeRef", "destination": "FileRef"},
    "file.certificate.export": {"macro": "MacroRevisionRef", "destination": "FileRef"},
    "file.macro.export": {"macros": "MacroRevisionRefs", "destination": "FileRef"},
    "file.macro.check": {"file": "FileDigestRef"},
    "file.macro.import": {"checked": "CheckedFileRef"},
    "operation.cleanup.from_prepare": {"draft": "ResolvedDraftRef"},
    "operation.preview.cancel": {"preview": "PreviewId"},
    "operation.reuse": {"draft": "ResolvedDraftRef"},
    "view.entry_slots": {"result": "ResultRef"},
    "buffer.analyse": {"orbit": "OrbitId", "buffers": "PositionIdPair", "target": "PositionId"},
    "block.add_current": {"positions": "PositionIds", "mode": "position|exact", "revision": "RevisionRef"},
    "block.add_home": {"positions": "PositionIds", "mode": "position|exact"},
    "block.reference.set": {"block": "BlockRef", "reference": "WordRef"},
    "block.remove": {"requirement": "BlockRequirementRef"},
    "view.residuals": {"orbit": "OrbitId"}, "view.findings": {"result": "ResultRef"},
    "view.recommendations": {"orbit": "OrbitId", "block": "BlockRef"},
    "view.names": {"object": "CanonicalObjectRef"},
    "worksheet.save": {"draft": "ResolvedDraftRef", "name": "Text"},
    "worksheet.load": {"worksheet": "WorksheetRevisionRef"},
    "worksheet.compare": {"worksheet": "WorksheetRevisionRef", "draft": "ResolvedDraftRef"},
    "keymap.edit": {"keymap": "KeymapRevisionRef", "candidate": "BindingEdits"},
    "keymap.defaults": {"keymap": "KeymapRevisionRef"},
    "keymap.save": {"candidate": "CheckedBindingsRef"},
    "keymap.name": {"keymap": "KeymapRevisionRef", "name": "Text"},
    "keyboard.disclosure.set": {"choice": "DisclosureChoice"},
    "command.find": {"query": "Text", "object": "CanonicalObjectRef?"},
    "filter.preview": {"predicate": "FilterRef", "revision": "RevisionRef"},
    "filter.apply": {"preview": "FilterPreviewRef"},
    "filter.edit": {"predicate": "FilterRef"},
    "filter.preset": {"preset": "PresetId", "object": "CanonicalObjectRef"},
    "filter.save": {"predicate": "FilterRef", "name": "Text"},
    "filter.load": {"filter": "SavedFilterRef"},
    "set.save": {"members": "CanonicalObjectRefs", "kind": "identity|position", "name": "Text"},
    "display.adjust": {"view": "ViewId", "settings": "DisplaySettings"},
    "display.references.set": {"view": "ViewId", "references": "ReferenceDisplaySettings"},
    "display.mode.set": {"view": "ViewId", "mode": "DisplayMode"},
    "structure.find": {"query": "CanonicalCellQuery"},
    "structure.filter.compose": {"predicate": "FilterRef", "composition": "FilterComposition"},
    "work.layout.set": {"layout": "LayoutRef"},
    "view.local.configure": {"origin": "CellId", "radius": "Nat", "follow": "FollowPolicy"},
    "macro.filter": {"query": "MacroQuery"},
    "macro.library.check": {"macros": "MacroRevisionRefs"},
    "macro.reference.check": {"macro": "MacroRevisionRef", "transform": "ExactTransformRef"},
    "endgame.choices": {"family": "FamilyId", "position": "PositionId"},
    "view.journal.delta": {"action": "ActionNumber"},
    "help.open": {"topic": "TermOrCommandId"},
}
for row in rows:
    row["arguments"] = ARGUMENTS.get(row["id"], {})
    if row["id"] in UNDO_ROWS:
        row["arguments"] = {"hint": "ActionHint", "token": "UserActionToken"}
        row["action_guard"] = "action_hint"
        row["slot_map"] = "by_target"
        row["note"] = "LI.26: dispatch journal or workspace action from ActionHint through its original permission path, including release confirmation. Restore a recorded slot map only when the target action changed it."
    if row["kind"] == "commit":
        row["arguments"]["token"] = "UserActionToken"
    if row["id"] == "session.resume":
        row["lane"] = "exclusive"
        row["note"] = "Resume owns recovery admission; unrelated jobs do not gate a commit or undo."
    if row["id"] in {"session.scramble", "operation.phase.edit"}:
        row["note"] = (row["note"] or "") + " H5: atomic workspace edit in lane none; no job-wide busy gate."
    if row["lane"] in {"view", "latest_wins"}:
        row["lane_target"] = {"arg": "view"} if "view" in row["arguments"] else {"control": row["id"]}
    if row["id"] in {"list.row.copy", "view.preview.scrub"}:
        row["source"] = {"list.row.copy": "DT-M1 C2; DT-J1 O9", "view.preview.scrub": "DT-M1 fix-first 2; Q10(a)"}[row["id"]]

NOT_COMMANDS = {
 "T-01": "launch argument", "T-02": "launch argument", "T-03": "launch argument",
 "T-04": "storage policy of the session store", "T-05": "documented procedure",
 "T-06": "process ownership", "T-07": "shell window behaviour",
 "T-13": "engine event after a completing commit; the shell shows it",
 "T-59": "input routing rule (R-18), not a command",
 "T-66": "renderer rule CM.8 and ONE.62: session state, identified preview or derived display",
 "T-72": "separate browser infrastructure deleted by signed U1-007 and U3-002; surviving purposes join individually",
}


class TableError(ValueError):
    """A source, join, binding or delta cannot form a command table."""


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def digest(value):
    return hashlib.sha256(canonical(value)).hexdigest()


def load(path):
    return json.loads(Path(path).read_bytes())


def decision_sources():
    """Read each split part's own inventory rows, never the aggregate entry list."""
    units = {u["id"]: u for u in load(HERE / "dispositions/units.json")["units"]}
    inventory = {}
    for n in range(1, 7):
        for author, filename in (("claude", f"claude-source-s{n}.json"), ("codex", f"bottom-up-s{n}.json")):
            inventory.update({f"{author}:{r['id']}": r for r in load(HERE / "inventory" / filename)["functions"]})
    result = {}
    for n in range(1, 7):
        for row in load(HERE / "dispositions" / f"dispositions-s{n}.json")["rows"]:
            parts = [(f"{row['unit']}/{i}", p) for i, p in enumerate(row["parts"], 1)] if row["disposition"] == "split" else [(row["unit"], row)]
            for did, part in parts:
                source_rows = part.get("rows", units[row["unit"]]["rows"])
                entries = sorted({e for rid in source_rows for e in inventory[rid]["entry_points"]})
                result[did] = {"decision": did, "disposition": part["disposition"], "purpose": part["purpose"],
                               "source_rows": source_rows,
                               "source_sha256": digest({"decision": did, "record": part, "rows": source_rows}),
                               "user_entry": any(e.split(" ")[0] in USER_ENTRY_KINDS for e in entries),
                               "legacy_ids": sorted({e.removeprefix("command ") for e in entries if e.startswith("command ")})}
    return result


def validate_rows(candidate, inventory_ids):
    by_id = {}
    for row in candidate:
        if not isinstance(row, dict) or not ROW_FIELDS <= row.keys():
            raise TableError("row missing required H2 fields")
        name = row["id"]
        if not isinstance(name, str) or not re.fullmatch(r"[a-z][a-z0-9_]*(?:\.[a-z0-9_]+)+", name):
            raise TableError("invalid command id")
        if name in by_id:
            raise TableError(f"duplicate command id: {name}")
        by_id[name] = row
        for field, vocabulary in (("kind", KINDS), ("undo", UNDO), ("lane", LANES), ("preview", {"none", "required", "filter"})):
            if not isinstance(row[field], str) or row[field] not in vocabulary:
                raise TableError(f"{name}: invalid {field}")
        for field, vocabulary in (("gates", GATES), ("contexts", CONTEXTS), ("inventory", inventory_ids),
                                  ("disposition", {"keep", "redesign"})):
            values = row[field]
            if not isinstance(values, list) or any(not isinstance(v, str) or v not in vocabulary for v in values) or len(values) != len(set(values)):
                raise TableError(f"{name}: invalid {field}")
        if not row["contexts"] or not row["label"] or not row["source"]:
            raise TableError(f"{name}: empty context, label or source")
        if type(row["repeat"]) is not bool or type(row["job"]) is not bool:
            raise TableError(f"{name}: repeat and job must be booleans")
        if row["repeat"] and (name not in REPEAT or row["kind"] != "view"):
            raise TableError(f"{name}: forbidden repeat (DT-M1 fix-first 2)")
        if row["kind"] == "commit" and row["lane"] != "commit":
            raise TableError(f"{name}: commit requires commit lane")
        if (row["preview"] == "required") != ("fresh_preview" in row["gates"]):
            raise TableError(f"{name}: preview and fresh_preview disagree")
        if (row["preview"] == "filter") != ("fresh_filter_preview" in row["gates"]):
            raise TableError(f"{name}: preview and fresh_filter_preview disagree")
        if row["preview"] == "filter" and name != "filter.apply":
            raise TableError(f"{name}: only filter.apply uses filter freshness")
        if name == "filter.apply" and row["preview"] != "filter":
            raise TableError("filter.apply: requires filter freshness")
        if name == "operation.preview" and "protection" in row["gates"]:
            raise TableError("operation.preview: protection is a verdict, not an admission gate")
        if name == "operation.execute" and "protection" not in row["gates"]:
            raise TableError("operation.execute: protection gate required")
        if row.get("slot_map", "preserve") not in {"preserve", "rebind", "by_target"}:
            raise TableError(f"{name}: invalid slot_map")
        if row.get("slot_map") == "rebind" and name != "grip.rebind":
            raise TableError(f"{name}: only grip.rebind replaces the slot map")
        if row.get("slot_map") == "by_target" and name not in UNDO_ROWS | {ROLLBACK_ROW}:
            raise TableError(f"{name}: only session undo/redo restores a target's recorded map")
        arguments = row["arguments"]
        if not isinstance(arguments, dict) or any(not isinstance(k, str) or not k or not isinstance(v, str) or not v for k, v in arguments.items()):
            raise TableError(f"{name}: arguments must be an object of named types")
        if row["lane"] in {"view", "latest_wins"}:
            target = row.get("lane_target")
            if not isinstance(target, dict) or len(target) != 1 or not set(target) <= {"arg", "view", "control"}:
                raise TableError(f"{name}: invalid lane_target")
            if not all(isinstance(v, str) and v for v in target.values()):
                raise TableError(f"{name}: empty lane_target")
            if "arg" in target and arguments.get(target["arg"]) != "ViewId":
                raise TableError(f"{name}: lane_target arg must name a ViewId argument")
        if row["kind"] == "commit" and arguments.get("token") != "UserActionToken":
            raise TableError(f"{name}: commit needs user action token (ONE.4)")
        if name in UNDO_ROWS and (row["kind"] != "commit" or row["gates"] or row.get("action_guard") != "action_hint"
                                 or arguments != {"hint": "ActionHint", "token": "UserActionToken"} or row.get("slot_map") != "by_target"):
            raise TableError(f"{name}: undo/redo must be ungated with ActionHint and by_target slot map")
        if name in LAYOUT_UNDO_ROWS and (row["kind"] != "commit" or row["gates"] or row.get("action_guard") != "action_hint"
                                        or arguments != {"hint": "ActionHint", "token": "UserActionToken"}
                                        or row.get("slot_map", "preserve") != "preserve" or row["undo"] != "layout"):
            raise TableError(f"{name}: layout undo/redo must be ungated with ActionHint and the layout undo class")
        if row["undo"] == "layout" and row["kind"] != "workspace" and name not in LAYOUT_UNDO_ROWS:
            raise TableError(f"{name}: only a workspace row is a layout-history step (ADR-D01.14)")
        if name == PARTIAL_UNDO_ROW and (row["kind"] != "commit" or row["lane"] != "commit" or row["preview"] != "none"
                                         or row["undo"] != "journal" or row["gates"] != ["protection"]
                                         or arguments != PARTIAL_UNDO_ARGUMENTS or "action_guard" in row
                                         or row.get("slot_map", "preserve") != "preserve"):
            raise TableError(f"{name}: partial undo must be a live journal commit with the protection gate (ADR-D01.15)")
        if name == ONE_KEY_ROW and (row["kind"] != "commit" or row["lane"] != "commit" or row["preview"] != "none"
                                    or row["undo"] != "journal" or row["gates"] != ["protection"]
                                    or arguments != ONE_KEY_ARGUMENTS or "action_guard" in row
                                    or row["key_0_4"] is not None or row.get("slot_map", "preserve") != "preserve"):
            raise TableError(f"{name}: one-key preview-and-commit must be a live journal commit of a record draft with "
                             "the protection gate and no default key (ADR-D01.12)")
        if name == ROLLBACK_ROW and (row["kind"] != "commit" or row["lane"] != "commit" or row["preview"] != "none"
                                     or row["undo"] != "journal" or row["gates"] or row.get("action_guard") != "action_hint"
                                     or arguments != ROLLBACK_ARGUMENTS or row["key_0_4"] is not None
                                     or row.get("slot_map") != "by_target"):
            raise TableError(f"{name}: multi-step rollback must be an ungated journal commit with ActionHint, the "
                             "by_target slot map and no default key (ADR-D01.15)")
        if name in {"workspace.undo", "workspace.redo", "jumble.preview.scrub"}:
            raise TableError(f"{name}: duplicate purpose; use the shared session or view entry")
        if name == "twist.apply" and (row["preview"] != "none" or row["undo"] != "journal" or "protection" not in row["gates"]):
            raise TableError("twist.apply: preserve ONE.78 (owner, 6 November 2026, decision 10)")
        if name == "operation.phase.append_twist" and (row["kind"] != "workspace" or row["undo"] != "workspace" or row["gates"]):
            raise TableError("draft twist must have workspace undo and no live gates")
        if name.startswith("session.timer.") and (row["kind"] != "workspace" or row["undo"] != "none"):
            raise TableError("timer must be non-journal with no undo")
    covered = {t for row in candidate for t in row["inventory"]}
    missing = inventory_ids - covered - NOT_COMMANDS.keys()
    if missing:
        raise TableError(f"inventory rows without a command or reason: {sorted(missing)}")
    return by_id


def validate_delta_arguments(name, arguments):
    required = DELTA_REQUIRED_ARGUMENTS.get(name, set())
    optional = {key for key, kind in arguments.items()
                if isinstance(kind, str) and kind.endswith("?")}
    optional_required = required & optional
    if optional_required:
        raise TableError(f"{name}: delta required arguments cannot be optional: {sorted(optional_required)}")
    if arguments.keys() - optional == {"token"}:
        raise TableError(f"{name}: delta argument schema cannot be token-only")
    missing = required - arguments.keys()
    if missing:
        raise TableError(f"{name}: delta missing required arguments: {sorted(missing)}")


def apply_delta(candidate, delta, changed=None):
    if not isinstance(delta, dict) or delta.get("format") != DELTA_FORMAT:
        raise TableError("invalid delta format")
    if set(delta) != {"format", "add", "amend"} or not all(isinstance(delta[k], list) for k in ("add", "amend")):
        raise TableError("delta requires add and amend arrays")
    by_id = {row["id"]: row for row in candidate}
    for original in delta["add"]:
        if not isinstance(original, dict) or not ROW_FIELDS <= original.keys():
            raise TableError("delta add missing required H2 fields")
        row = deepcopy(original)
        if row["id"] in by_id:
            raise TableError(f"delta add existing id: {row['id']}")
        if not isinstance(row["arguments"], dict) or not row["arguments"]:
            raise TableError(f"{row['id']}: delta add needs an explicit nonempty argument schema")
        validate_delta_arguments(row["id"], row["arguments"])
        row.setdefault("slot_map", "preserve")
        by_id[row["id"]] = row
        candidate.append(row)
    changed = set() if changed is None else changed
    for amendment in delta["amend"]:
        if not isinstance(amendment, dict) or set(amendment) != {"id", "expect", "set", "source"}:
            raise TableError("invalid delta amendment")
        name = amendment["id"]
        if name not in by_id:
            raise TableError(f"delta amend missing id: {name}")
        row = by_id[name]
        if not isinstance(amendment["expect"], dict) or not amendment["expect"] or not isinstance(amendment["set"], dict) or not amendment["set"] or not amendment["source"]:
            raise TableError(f"{name}: amendment needs expect, set and source")
        if not isinstance(amendment["set"].get("arguments"), dict) or not amendment["set"]["arguments"]:
            raise TableError(f"{name}: amendment needs an explicit nonempty argument schema")
        validate_delta_arguments(name, amendment["set"]["arguments"])
        for key, value in amendment["expect"].items():
            if key not in row or row[key] != value:
                raise TableError(f"{name}: incompatible amendment: {key}")
        for key, value in amendment["set"].items():
            if key == "id" or key not in row:
                raise TableError(f"{name}: amendment changes unknown or identity field: {key}")
            if (name, key) in changed:
                raise TableError(f"{name}: field amended twice: {key}")
            changed.add((name, key))
            row[key] = deepcopy(value)
        row.setdefault("amendment_sources", []).append(amendment["source"])


def validate_join(join, sources, by_id):
    if join.get("format") != "c600-command-join-1" or not isinstance(join.get("decisions"), list):
        raise TableError("invalid command join")
    seen = set()
    contributors = {name: [] for name in by_id}
    for item in join["decisions"]:
        did = item.get("decision")
        if did in seen or did not in sources:
            raise TableError(f"duplicate or unknown decision: {did}")
        seen.add(did)
        source = sources[did]
        for field in ("disposition", "purpose", "source_rows", "source_sha256", "user_entry", "legacy_ids"):
            if item.get(field) != source[field]:
                raise TableError(f"{did}: join differs from signed source: {field}")
        commands = item.get("commands")
        if not isinstance(commands, list) or any(not isinstance(c, str) or c not in by_id for c in commands) or len(commands) != len(set(commands)):
            raise TableError(f"{did}: unknown or duplicate mapped command")
        if item["disposition"] in {"delete", "automate"} and commands:
            raise TableError(f"{did}: {item['disposition']} gets no command")
        if not commands and (item.get("outcome") != "none" or not isinstance(item.get("reason"), str) or not item["reason"].strip()):
            raise TableError(f"{did}: none needs a reason")
        if commands and item.get("outcome") != "mapped":
            raise TableError(f"{did}: commands need mapped outcome")
        for name in commands:
            contributors[name].append({"decision": did, "disposition": item["disposition"]})
    missing = sources.keys() - seen
    if missing:
        raise TableError(f"missing decisions: {sorted(missing)}")
    for name, row in by_id.items():
        row["decisions"] = sorted(contributors[name], key=lambda d: d["decision"])
        dispositions = sorted({d["disposition"] for d in contributors[name]})
        if row["disposition"] and row["disposition"] != dispositions:
            raise TableError(f"{name}: disposition differs from join")
        row["disposition"] = dispositions


def validate_aliases(aliases, sources, by_id):
    if aliases.get("format") != "c600-command-aliases-1" or not aliases.get("version"):
        raise TableError("invalid aliases format or version")
    payload = {k: v for k, v in aliases.items() if k != "content_digest"}
    if aliases.get("content_digest") != digest(payload):
        raise TableError("alias content digest mismatch")
    expected = {legacy for s in sources.values() for legacy in s["legacy_ids"]}
    seen, patterns = set(), set()
    for group in aliases["rows"]:
        if group["pattern"] in patterns:
            raise TableError("duplicate alias pattern")
        patterns.add(group["pattern"])
        for case in group["cases"]:
            legacy = case["legacy_id"]
            if legacy in seen or legacy not in expected:
                raise TableError(f"duplicate or unknown legacy id: {legacy}")
            seen.add(legacy)
            source_decisions = [s["decision"] for s in sources.values() if legacy in s["legacy_ids"]]
            if case.get("source_decisions") != source_decisions:
                raise TableError(f"{legacy}: alias source decisions differ")
            target = case["target"]
            if target is None:
                if not case.get("reason"):
                    raise TableError(f"{legacy}: unmapped alias needs reason")
                continue
            name = target["command"]
            if name not in by_id or not isinstance(target["arguments"], dict) or not isinstance(target["requires"], list):
                raise TableError(f"{legacy}: invalid alias target")
            if target["arguments"].keys() - by_id[name].get("arguments", {}).keys():
                raise TableError(f"{legacy}: unknown target argument")
            if name in UNDO_ROWS and (set(target.get("invoke_requires", {})) != {"hint", "token"}
                                      or "dispatch_by_action_class" in target or target["arguments"]):
                raise TableError(f"{legacy}: undo alias requires fresh hint and token at the shared entry")
            # CM.3: bindings may be literal, resolved from captured data/choice,
            # or acquired at invocation. General prose is not an argument binding.
            resolved = {match.group(1) for requirement in target["requires"]
                        if isinstance(requirement, str)
                        and (match := re.fullmatch(r"([a-z][a-z0-9_]*):\s*\S.*", requirement))}
            invoked = target.get("invoke_requires", {})
            if not isinstance(invoked, dict) or any(not isinstance(v, str) or not v.strip() for v in invoked.values()):
                raise TableError(f"{legacy}: invalid invocation requirements")
            bound = target["arguments"].keys() | resolved | invoked.keys()
            required = {key for key, kind in by_id[name]["arguments"].items() if not kind.endswith("?")}
            if missing := required - bound:
                raise TableError(f"{legacy}: missing required alias arguments: {sorted(missing)}")
            # Split inventories can mention unrelated IDs. Only a surviving
            # contributor to this purpose can support an executable alias.
            eligible = {s["decision"] for s in sources.values() if legacy in s["legacy_ids"] and s["disposition"] in {"keep", "redesign"}}
            contributors = {d["decision"] for d in by_id[name]["decisions"]}
            if not eligible & contributors:
                raise TableError(f"{legacy}: alias has no surviving decision for {name}")
    if seen != expected:
        raise TableError(f"missing legacy aliases: {sorted(expected - seen)}")


def validate_bindings(bindings, by_id):
    """Check supplied G6 bindings; no production key letters are selected here."""
    classes = {"capture", "text", "list", "global"}
    for binding in bindings:
        routing = binding.get("routing")
        name = binding.get("command")
        if routing not in classes:
            raise TableError("invalid routing class")
        if routing in {"capture", "text"} and name is not None:
            raise TableError("captured/text key reaches a command")
        if name is None:
            continue
        if name not in by_id:
            raise TableError("binding names unknown command")
        if routing == "list" and name not in LIST_ACTIONS:
            raise TableError("list key reaches non-navigation command")
        if binding.get("repeat", False) and not by_id[name]["repeat"]:
            raise TableError("binding enables forbidden repeat")


def validate_services(services, by_id):
    seen = set()
    for service in services:
        if not isinstance(service, dict) or set(service) != {"id", "kind", "caller", "lane", "clauses", "note"}:
            raise TableError("service requires only id, kind, caller, lane, clauses and note")
        name = service["id"]
        if not isinstance(name, str) or not re.fullmatch(r"[a-z][a-z0-9_]*(?:\.[a-z0-9_]+)+", name):
            raise TableError("invalid service id")
        if name in by_id or name in seen:
            raise TableError(f"service ID collision: {name}")
        seen.add(name)
        if service["kind"] not in {"service", "continuation"} or service["caller"] not in {"runtime", "host", "both"} or service["lane"] not in LANES:
            raise TableError(f"{name}: invalid service kind, caller or lane")
        if not isinstance(service["clauses"], list) or not service["clauses"] or any(not isinstance(c, str) or not c for c in service["clauses"]) or not isinstance(service["note"], str) or not service["note"]:
            raise TableError(f"{name}: service needs clauses and note")


def validate_reads(reads, services):
    """CM.18: each bridge read is typed by puzzle and level; a code shared by every read of a puzzle is listed once."""
    if not isinstance(reads, dict) or set(reads) != set(READ_PROTOCOL) | {"rows"}:
        raise TableError("reads require only form, order, types, tariff, codes, clauses and rows")
    codes = reads["codes"]
    lists = ("every", *READ_LEVELS)
    if not isinstance(codes, dict) or set(codes) != {"unserved", *lists} or any(not isinstance(codes[k], list) for k in lists):
        raise TableError("read codes require unserved, every and one list per puzzle")
    shared = [codes["unserved"]] + [code for key in lists for code in codes[key]]
    if any(not isinstance(code, str) or not CODE.fullmatch(code) for code in shared):
        raise TableError("invalid shared read code")
    if not isinstance(reads["rows"], list) or not reads["rows"]:
        raise TableError("reads require a nonempty row list")
    services = {service["id"] for service in services}
    puzzles = {}
    for row in reads["rows"]:
        if not isinstance(row, dict) or set(row) != set(READ_FIELDS):
            raise TableError("read requires only " + ", ".join(READ_FIELDS))
        name, puzzle = row["id"], row["puzzle"]
        if not isinstance(name, str) or not re.fullmatch(r"[a-z][a-z0-9_]*(?:\.[a-z0-9_]+)*", name):
            raise TableError("invalid read id")
        if name in services:
            raise TableError(f"read ID collision with a service: {name}")
        if not isinstance(puzzle, str) or puzzle not in READ_LEVELS or not isinstance(row["level"], str) \
                or row["level"] not in READ_LEVELS[puzzle]:
            raise TableError(f"{name}: invalid read puzzle or level")
        if puzzle in puzzles.setdefault(name, set()):
            raise TableError(f"read {name} repeated for {puzzle}")
        puzzles[name].add(puzzle)
        if "both" in puzzles[name] and len(puzzles[name]) > 1:
            raise TableError(f"read {name} for both puzzles cannot also be one puzzle's")
        if row["argument"] is not None and (not isinstance(row["argument"], str) or not row["argument"]):
            raise TableError(f"{name}: read argument must be a type or None")
        if any(not isinstance(row[key], str) or not row[key] for key in ("answer", "tariff", "note")):
            raise TableError(f"{name}: read needs answer, tariff and note")
        if row["fields"] not in ("all", "subset"):
            raise TableError(f"{name}: read fields must be all or subset")
        if not isinstance(row["clauses"], list) or not row["clauses"] or any(not isinstance(c, str) or not c for c in row["clauses"]):
            raise TableError(f"{name}: read needs clauses")
        own = row["codes"]
        if not isinstance(own, list) or any(not isinstance(code, str) or not CODE.fullmatch(code) for code in own) or len(set(own)) != len(own):
            raise TableError(f"{name}: invalid read code")
        if set(own) & set(codes["every"] + codes[puzzle]):
            raise TableError(f"{name}: a shared read code is listed again")


def default_deltas():
    """The stored table's deltas, in their order."""
    return [load(HERE / name) for name in DELTA_FILES]


def build_table(*, base_rows=None, join=None, aliases=None, sources=None,
                delta=None, draft_without_jumbling=False, bindings=(), services=None, reads=None):
    """Build and validate in memory. None means a missing delta, not an empty one.

    delta is one H2 delta or a list applied in order.
    """
    if draft_without_jumbling and delta is not None:
        raise TableError("draft-without-jumbling cannot include a delta")
    if delta is None and not draft_without_jumbling:
        raise TableError("missing jumbling delta; use --draft-without-jumbling only for a draft")
    candidate = deepcopy(rows if base_rows is None else base_rows)
    inventory_ids = {r["id"] for r in load(HERE / "inventory/top-down.json")["functions"]}
    by_id = validate_rows(candidate, inventory_ids)
    sources = decision_sources() if sources is None else sources
    join = load(HERE / "command-join.json") if join is None else join
    aliases = load(HERE / "command-aliases.json") if aliases is None else aliases
    # H2's expect sees the joined current row, including its dispositions.
    validate_join(join, sources, by_id)
    if delta is not None:
        # One set across the deltas: H2 refuses a field amended twice, whichever file does it.
        changed = set()
        for part in delta if isinstance(delta, list) else [delta]:
            apply_delta(candidate, part, changed)
    by_id = validate_rows(candidate, inventory_ids)
    validate_join(join, sources, by_id)
    validate_aliases(aliases, sources, by_id)
    validate_bindings(bindings, by_id)
    services = deepcopy(SERVICES if services is None else services)
    validate_services(services, by_id)
    reads = deepcopy({**READ_PROTOCOL, "rows": READS} if reads is None else reads)
    validate_reads(reads, services)
    return {"draft": "H-08 EN4", "final": not draft_without_jumbling,
            "source_inventory": "inventory/top-down.json", "source_join": "command-join.json",
            "aliases": {"file": "command-aliases.json", "version": aliases["version"], "content_digest": aliases["content_digest"]},
            "vocabularies": {"kind": sorted(KINDS), "undo": sorted(UNDO), "gates": sorted(GATES),
                             "contexts": sorted(CONTEXTS), "lane": sorted(LANES), "preview": ["none", "required", "filter"]},
            "undo_dispatch": {"entry": {"undo": "session.undo", "redo": "session.redo"}, "classes": ["journal", "workspace"], "guard": "action_hint",
                              "stale": "KN5:stale.undo_superseded", "protection": "original_permission_path_including_release_confirmation",
                              "layout": {"entry": {"undo": "layout.undo", "redo": "layout.redo"}, "classes": ["layout"]}},
            "commands": candidate, "services": services, "reads": reads, "not_commands": NOT_COMMANDS}


def encoded(table):
    return (json.dumps(table, indent=1, ensure_ascii=False) + "\n").encode("utf-8")


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--delta", type=Path, action="append")
    parser.add_argument("--out", type=Path, default=HERE / "command-table.json")
    parser.add_argument("--draft-without-jumbling", action="store_true")
    args = parser.parse_args(argv)
    try:
        if args.draft_without_jumbling and args.delta is not None:
            raise TableError("--delta and --draft-without-jumbling are incompatible")
        delta = None
        if not args.draft_without_jumbling:
            paths = args.delta or [HERE / name for name in DELTA_FILES]
            for path in paths:
                if not path.is_file():
                    raise TableError(f"missing delta {path.name}; use --draft-without-jumbling only for a draft")
            delta = [load(path) for path in paths]
        table = build_table(delta=delta, draft_without_jumbling=args.draft_without_jumbling)
        args.out.write_bytes(encoded(table))
    except (TableError, OSError, ValueError) as error:
        parser.exit(1, f"command table: {error}\n")
    print(f"{len(table['commands'])} commands; final={table['final']}; {dict(Counter(r['kind'] for r in table['commands']))}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
