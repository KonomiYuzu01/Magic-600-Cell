"""Build docs/progress/1.0/command-table.json, the H-08 draft command table.

Rows are written by hand from the top-down inventory (inventory/top-down.json).
This script checks that every row names known inventory IDs, uses only the
defined vocabularies, and that every inventory row is covered or listed as
not a command.
"""
import json, sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
INV = json.loads((HERE / "inventory/top-down.json").read_text(encoding="utf-8"))
INV_IDS = {f["id"] for f in INV["functions"]}

KINDS = {"commit", "workspace", "view", "query", "file", "app"}
UNDO = {"journal", "workspace", "none"}
GATES = {"protection", "fresh_preview", "confirm", "model_match", "not_busy", "file_check"}
CONTEXTS = {"puzzle", "solve", "operation", "keyboard", "filter", "macro_base", "editor", "any"}

rows = []
def c(cid, label, kind, src, *, gates=(), preview="none", undo="none", contexts=("any",),
      repeat=False, job=False, key04=None, note=None):
    rows.append({"id": cid, "label": label, "kind": kind, "inventory": list(src),
                 "gates": list(gates), "preview": preview, "undo": undo,
                 "contexts": list(contexts), "repeat": repeat, "job": job,
                 "key_0_4": key04, "disposition": "pending-2.2", "note": note})

# session
c("session.new", "New solve", "commit", ["T-08"], gates=["confirm", "not_busy"], undo="journal")
c("session.resume", "Resume", "app", ["T-09"], gates=["not_busy"])
c("session.timer.start", "Start timer", "commit", ["T-10"], undo="none")
c("session.timer.stop", "Stop timer", "commit", ["T-10"], undo="none")
c("session.scramble", "Scramble (staged)", "workspace", ["T-11"], gates=["not_busy"], undo="workspace",
  note="stages into the operation; the commit is operation.execute")
c("session.reset", "Reset puzzle (recoverable)", "commit", ["T-12"], gates=["confirm", "not_busy"], undo="journal")
c("session.undo", "Undo", "commit", ["T-14"], gates=["not_busy"], undo="journal", key04="Commands > Undo")
c("session.redo", "Redo", "commit", ["T-14"], gates=["not_busy"], undo="journal", key04="Commands > Redo")
c("session.checkpoint.save", "Save checkpoint", "commit", ["T-15"], gates=["not_busy"])
c("session.checkpoint.restore", "Restore checkpoint", "commit", ["T-15"], gates=["confirm", "not_busy"], undo="journal")
c("session.report", "Progress report", "query", ["T-16"])
# files
c("file.log.export", "Export C600 or MPUlt log", "file", ["T-17"], job=True, key04="Ctrl+Alt+O/L (Functions)")
c("file.log.check", "Check a log file", "query", ["T-18"], gates=["model_match"], job=True)
c("file.log.import", "Import a checked log", "commit", ["T-18"], gates=["file_check", "confirm", "not_busy"],
  undo="journal", job=True, note="keeps preferences; leaves a recovery checkpoint")
c("file.keymap.export", "Export keymap", "file", ["T-19"], key04="Ctrl+Alt+E (Functions)")
c("file.keymap.check", "Check keymap file", "query", ["T-19"], gates=["model_match"])
c("file.keymap.apply", "Apply keymap file", "workspace", ["T-19"], gates=["file_check", "confirm"], undo="workspace",
  key04="Ctrl+Alt+I (Functions)")
# operation (solve)
c("solve.open", "Open Solve", "view", ["T-20"])
c("operation.phase.select", "Select phase Prepare / Macro / Cleanup", "view", ["T-21"], contexts=["solve"], key04="Ctrl+1/2/3")
c("operation.phase.edit", "Edit phase turns", "workspace", ["T-22"], gates=["not_busy"], undo="workspace",
  contexts=["solve"], key04="F4")
c("operation.cleanup.from_prepare", "Cleanup = inverse of Prepare", "workspace", ["T-23"], undo="workspace",
  contexts=["solve"], key04="Ctrl+Shift+I")
c("operation.check", "Check full effect", "query", ["T-24"], job=True, contexts=["solve", "operation"])
c("operation.preview", "Preview", "query", ["T-24"], gates=["protection"], job=True, contexts=["operation"],
  key04="Ctrl+Enter")
c("operation.execute", "Execute", "commit", ["T-24"], gates=["fresh_preview", "protection", "not_busy"],
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
c("object.inspect", "Inspect identity or position", "view", ["T-32", "T-61"], repeat=True, key04="Enter; Shift+click")
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
c("worksheet.save", "Save worksheet", "workspace", ["T-45"])
c("worksheet.load", "Load worksheet", "workspace", ["T-45"], undo="workspace")
c("macro.compare", "Compare candidates", "query", ["T-45"], job=True)
c("endgame.configure", "Configure endgame family", "workspace", ["T-46"], undo="workspace")
c("macro.save", "Save as macro", "workspace", ["T-46"], gates=["confirm"], note="saving does not select, insert or execute")
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
c("twist.apply", "Twist", "commit", ["T-52", "T-60"], gates=["protection", "not_busy"], undo="journal",
  contexts=["puzzle", "keyboard"], note="live destination commits; draft destination edits the phase")
c("twist.inverse.toggle", "Inverse for clicks", "view", ["T-52"], contexts=["keyboard"])
c("input.destination.set", "Input destination live or draft", "workspace", ["T-53"])
c("bank.select", "Select bank", "workspace", ["T-54"])
c("grip.capture", "Set grips", "workspace", ["T-55"], undo="workspace")
c("bank.functions.toggle", "Functions bank", "view", ["T-56"])
c("keymap.edit", "Change key", "workspace", ["T-57"], gates=["confirm"], undo="workspace")
c("command.find", "Find command", "view", ["T-58"], key04="F1; Ctrl+F1 back")
# view
c("camera.rotate", "Rotate camera (3D/4D)", "view", ["T-60"], repeat=True, contexts=["puzzle"], key04="drag; Shift+left drag")
c("filter.open", "Open piece filter", "view", ["T-62"], key04="F5")
c("filter.preview", "Preview filter counts", "query", ["T-62"], contexts=["filter"])
c("filter.apply", "Apply filter", "workspace", ["T-62"], gates=["fresh_preview"], preview="required",
  undo="workspace", contexts=["filter"])
c("display.frame.toggle", "Show or hide 600-cell frame", "view", ["T-63"])
c("display.adaptive.toggle", "Adaptive motion detail", "view", ["T-64"])
c("display.adjust", "Size, field of view, lighting", "view", ["T-65"], repeat=True)
c("view.reset", "Reset view", "view", ["T-65"])
# structure (0.3 reference)
c("structure.find", "Find colour or cell", "query", ["T-67"])
c("structure.compare", "Compare cells", "query", ["T-67"])
c("structure.layer", "Prepare layer predicate", "workspace", ["T-68"], contexts=["filter"])
c("view.cell_views", "Auxiliary cell views", "view", ["T-69"])
c("camera.center", "Center main view on cell", "view", ["T-70"])
c("set.save", "Save frozen set", "workspace", ["T-71"])

NOT_COMMANDS = {
 "T-01": "launch argument", "T-02": "launch argument", "T-03": "launch argument",
 "T-04": "storage policy of the session store", "T-05": "documented procedure",
 "T-06": "process ownership", "T-07": "shell window behaviour",
 "T-13": "engine event after a completing commit; the shell shows it",
 "T-59": "input routing rule (R-18), not a command",
 "T-66": "renderer rule: show the committed state",
 "T-72": "separate client; disposition in 2.2",
}

errors = []
ids = [r["id"] for r in rows]
if len(ids) != len(set(ids)): errors.append("duplicate command id")
covered = set()
for r in rows:
    if r["kind"] not in KINDS: errors.append(f"{r['id']}: kind")
    if r["undo"] not in UNDO: errors.append(f"{r['id']}: undo")
    if not set(r["gates"]) <= GATES: errors.append(f"{r['id']}: gates")
    if not set(r["contexts"]) <= CONTEXTS: errors.append(f"{r['id']}: contexts")
    if r["kind"] == "commit" and r["repeat"]: errors.append(f"{r['id']}: commit must not repeat (R-18)")
    if r["kind"] == "commit" and "not_busy" not in r["gates"] and not r["id"].startswith("session.timer"):
        errors.append(f"{r['id']}: commit without not_busy")
    bad = set(r["inventory"]) - INV_IDS
    if bad: errors.append(f"{r['id']}: unknown inventory {bad}")
    covered |= set(r["inventory"])
missing = INV_IDS - covered - set(NOT_COMMANDS)
if missing: errors.append(f"inventory rows without a command: {sorted(missing)}")
if errors: sys.exit("\n".join(errors))

out = {"draft": "H-08", "source_inventory": "inventory/top-down.json", "vocabularies": {
        "kind": sorted(KINDS), "undo": sorted(UNDO), "gates": sorted(GATES), "contexts": sorted(CONTEXTS)},
       "commands": rows, "not_commands": NOT_COMMANDS}
(HERE / "command-table.json").write_text(json.dumps(out, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
from collections import Counter
print(len(rows), Counter(r["kind"] for r in rows))
