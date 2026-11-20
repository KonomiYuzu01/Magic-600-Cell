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
UNDO = {"journal", "workspace", "none"}
GATES = {"protection", "fresh_preview", "fresh_filter_preview", "confirm", "model_match", "file_check"}
LANES = {"commit", "exclusive", "latest_wins", "program", "view", "none"}
CONTEXTS = {"puzzle", "solve", "operation", "keyboard", "filter", "macro_base", "editor", "any"}
REPEAT = {"camera.rotate", "camera.zoom", "list.move", "view.preview.scrub"}
LIST_ACTIONS = {"list.move": "navigation", "object.inspect": "inspection", "list.row.copy": "row_copy"}
USER_ENTRY_KINDS = {"key", "button", "menu", "command", "mouse", "control"}
DELTA_FORMAT = "c600-command-rows-delta-1"
# Applied in this order: the language delta may amend a jumbling row.
DELTA_FILES = ("command-rows-jumbling.json", "command-rows-language.json")
ROW_FIELDS = {"id", "label", "kind", "inventory", "gates", "lane", "preview", "undo", "contexts",
              "repeat", "job", "key_0_4", "disposition", "note", "source", "arguments"}
UNDO_ROWS = {"session.undo", "session.redo"}
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
    "workspace.set": {"record", "payload", "provenance"},
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
        if row.get("slot_map") == "by_target" and name not in UNDO_ROWS:
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


def default_deltas():
    """The stored table's deltas, in their order."""
    return [load(HERE / name) for name in DELTA_FILES]


def build_table(*, base_rows=None, join=None, aliases=None, sources=None,
                delta=None, draft_without_jumbling=False, bindings=(), services=None):
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
    return {"draft": "H-08 EN4", "final": not draft_without_jumbling,
            "source_inventory": "inventory/top-down.json", "source_join": "command-join.json",
            "aliases": {"file": "command-aliases.json", "version": aliases["version"], "content_digest": aliases["content_digest"]},
            "vocabularies": {"kind": sorted(KINDS), "undo": sorted(UNDO), "gates": sorted(GATES),
                             "contexts": sorted(CONTEXTS), "lane": sorted(LANES), "preview": ["none", "required", "filter"]},
            "undo_dispatch": {"entry": {"undo": "session.undo", "redo": "session.redo"}, "classes": ["journal", "workspace"], "guard": "action_hint",
                              "stale": "KN5:stale.undo_superseded", "protection": "original_permission_path_including_release_confirmation"},
            "commands": candidate, "services": services, "not_commands": NOT_COMMANDS}


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
