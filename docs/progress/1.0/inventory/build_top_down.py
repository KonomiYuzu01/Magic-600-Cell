"""Build docs/progress/1.0/inventory/top-down.json, the stage 2.1 top-down inventory.

Every row is written by hand from user and design documents; this script only resolves
each evidence anchor to a path:line and fails when an anchor is missing.
"""
import json, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[4]
SNAP = "docs/progress/0.4/snapshot/"
P = {
 "U": "USAGE.md", "R": "docs/RELEASE_0_4.md", "S": "docs/STRUCTURE_EXPLORER.md", "RM": "README.md",
 "L": "docs/LIMITATIONS_AND_ROADMAP.md", "C": "work/experiments/magic600-04/packaging/CHANGES.md",
 "N": SNAP + "experiment/NATIVE_REVIEW_GUIDE.md",
 "F": SNAP + "experiment/evidence/functions-20260916/FRONTEND.md",
 "K": SNAP + "experiment/evidence/keymap-files-20260917/README.md",
 "SE": SNAP + "experiment/evidence/session-presentation-20260917/README.md",
 "SL": SNAP + "experiment/evidence/session-log-native-20260917/README.md",
 "D": SNAP + "experiment/docs/reviews/retained-display-controls-20260917.md",
 "B": SNAP + "design/01_PRODUCT_BRIEF_ZH.md",
 "E": SNAP + "design/08_INTEGRATION_AND_ENDGAME.md",
 "NM": SNAP + "design/07_NAMING_AND_RECOMMENDATION.md",
}
_cache = {}
def ev(key, anchor):
    path = P[key]
    lines = _cache.setdefault(path, (ROOT / path).read_text(encoding="utf-8").splitlines())
    for i, line in enumerate(lines, 1):
        if anchor in line:
            return f"{path}:{i}"
    sys.exit(f"anchor not found: {key} {anchor!r}")

rows = []
RELEASE_DOCS = {"U", "R", "RM", "C"}
HISTORICAL_DOCS = {"S", "L"}
def add(area, name, purpose, entry, reads, writes, status, evidence):
    if status == "released":
        # release-doc: a release-facing document says so; 0.4-dev-doc: only a 0.4 development guide or design contract does
        keys = {k for k, _ in evidence}
        status = ("release-doc" if keys & RELEASE_DOCS else
                  "0.4-dev-doc" if keys - HISTORICAL_DOCS else "0.3-reference")
    rows.append({"id": f"T-{len(rows)+1:02d}", "area": area, "name": name, "purpose": purpose,
                 "entry_points": entry, "reads": reads, "writes": writes, "engine_call": None,
                 "status": status, "evidence": [ev(k, a) for k, a in evidence]})

# A. Launch, data and process
add("launch", "Start the portable package in the G2 workspace",
    "Open the solving workbench with the puzzle visible", ["Magic600Cell.exe"], ["files"], ["none"], "released",
    [("U", "G2 is the default native workspace"), ("RM", "Download the Windows x64 portable ZIP")])
add("launch", "Start the alternate G1 keyboard workspace", "Physical-input workspace with the retained MPUlt viewport",
    ["Magic600Cell.exe --mode g1"], ["files"], ["none"], "released",
    [("U", "--mode g1"), ("N", "Open-Native-G1.cmd")])
add("launch", "Run against a chosen data folder", "Deliberate compatibility check on a copied session folder",
    ["Magic600Cell.exe --data <folder>"], ["files"], ["none"], "released",
    [("U", "--data \"<copied-session-folder>\"")])
add("data", "Separate versioned user profile", "Keep 0.4 data apart from older profiles; never import, rename or delete them silently",
    ["automatic at startup"], ["files"], ["files"], "released",
    [("U", "The default 0.4 profile is"), ("RM", "Personal data is stored separately")])
add("data", "Rollback to an earlier version", "Documented procedure: run the earlier application on its untouched profile",
    ["documented procedure"], ["files"], ["none"], "released", [("U", "To roll back")])
add("process", "Owned engine lifecycle", "Closing the window stops the owned engine; relaunch resumes the session",
    ["close window", "relaunch"], ["state"], ["none"], "released",
    [("N", "Closing the native window stops its owned engine")])
add("process", "Window management", "Minimized tools restore; the main window can return to the foreground",
    ["window controls"], ["preferences"], ["none"], "released", [("R", "Minimized tools restore normally")])

# B. Session
add("session", "New solve", "Start from a solved puzzle without automatic scramble; explicit scope confirmation",
    ["Session > New"], ["state"], ["state", "journal"], "released",
    [("U", "**Session** provides New, Resume"), ("SE", "New/reset require explicit scope confirmation"), ("B", "新建会话默认是完整 solved puzzle")])
add("session", "Resume", "Continue a saved long solve", ["Session > Resume", "relaunch"], ["state", "journal"], ["none"], "released",
    [("U", "**Session** provides New, Resume"), ("B", "对已保存的长期复原提供 Resume")])
add("session", "Session timer", "Explicit start/stop timer; includes thinking time while running",
    ["Session timer controls"], ["journal"], ["journal"], "released",
    [("U", "explicit timer controls"), ("L", "an explicit session timer")])
add("session", "Staged seeded scramble", "Scramble that is staged into Prepare/Review/Preview/Execute and is reproducible",
    ["Session > Scramble"], ["state"], ["state", "journal"], "released",
    [("U", "staged seeded scrambles"), ("SE", "Scramble stages into the existing")])
add("session", "Recoverable reset", "Reset the puzzle with a recovery point", ["Session > Reset"], ["state"], ["state", "journal", "checkpoints"], "released",
    [("U", "recoverable reset"), ("B", "Reset puzzle、Reset view、Reset workspace 是不同动作")])
add("session", "Completion summary", "Summary only after a real journal commit that completes the solve; never on import, undo or restore",
    ["automatic after a completing commit"], ["journal", "state"], ["none"], "released",
    [("U", "A recorded whole-solve completion requires an actual journal commit"), ("SE", "Completion notifications require")])
add("session", "Undo and redo", "Step back and forward through committed moves and operations",
    ["Commands > Undo", "Commands > Redo"], ["journal"], ["state", "journal"], "released",
    [("N", "try **Undo** and **Redo** through Commands"), ("B", "undo/redo、checkpoint")])
add("session", "Checkpoints and recovery", "Save a checkpoint and return to it; recovery after failure",
    ["Save checkpoint", "restore checkpoint"], ["checkpoints"], ["checkpoints", "state"], "released",
    [("N", "Save a checkpoint before exploring"), ("RM", "checkpoint recovery")])
add("session", "Progress reports", "Orbit progress and current-branch transaction totals",
    ["report view"], ["journal", "state"], ["none"], "unclear-in-0.4",
    [("L", "Orbit progress and current-branch transaction totals")])

# C. Files
add("files", "Export C600 and MPUlt v1 logs", "Write the session as a C600 log or an MPUltimate v1 600-cell-Full log",
    ["Session > Export", "Functions Ctrl+Alt+O/L"], ["journal", "state"], ["files"], "released",
    [("U", "C600 and MPUlt logs can be exported"), ("K", "Ctrl+Alt+O/L for Session logs")])
add("files", "Check and import C600 or MPUlt logs", "Verify model, turns, checksum and full colour state; keep preferences; leave a recovery checkpoint",
    ["Session > Check", "Session > Import"], ["files"], ["state", "journal", "checkpoints"], "released",
    [("U", "Imports preserve current workspace preferences"), ("SL", "explicit check/apply")])
add("files", "Keymap file export, check and apply", "Move key bindings between profiles; no puzzle state or commands",
    ["Functions Ctrl+Alt+E", "Functions Ctrl+Alt+I"], ["preferences", "files"], ["preferences", "files"], "released",
    [("U", "Keymap files use a separate export/check/apply flow"), ("K", "The Magic600-keymap schema 1 exports")])

# D. Solve workspace
add("solve", "Solve window", "One place to choose a macro, prepare its steps, inspect the full result and protection, preview and execute",
    ["Solve"], ["state", "protection", "workspace"], ["none"], "released",
    [("U", "Use **Solve** to choose a macro"), ("C", "a shared Solve window")])
add("solve", "Three-phase operation: Prepare, Macro, Cleanup", "Build an operation from explicit preparation, a macro and cleanup",
    ["Ctrl+1", "Ctrl+2", "Ctrl+3"], ["workspace"], ["workspace"], "released",
    [("R", "explicit Prepare / Macro / Cleanup operations"), ("N", "`Ctrl+1 / 2 / 3` selects Prepare / Macro / Cleanup")])
add("solve", "Phase editor", "Type finite explicit turns into a phase", ["F4", "lane double-click"], ["workspace"], ["workspace"], "released",
    [("N", "editing the phase is a separate action (`F4`)"), ("N", "The phase editor accepts finite explicit turns")])
add("solve", "Cleanup from inverse of Prepare", "Compute Cleanup as the inverse of the supplied Prepare; no search",
    ["Ctrl+Shift+I"], ["workspace"], ["workspace"], "released", [("N", "`Ctrl+Shift+I` computes Cleanup")])
add("solve", "Check, preview, execute and cancel an operation", "Full-effect check, then preview, then explicit execute; cancel keeps the draft",
    ["Ctrl+R", "Check", "Preview (Ctrl+Enter)", "Execute (Ctrl+Shift+Enter)", "Cancel preview"],
    ["state", "protection", "workspace"], ["state", "journal"], "released",
    [("N", "Open **Operation** (`Ctrl+R`), **Check**, then **Preview**"), ("N", "`Ctrl+Enter` previews and `Ctrl+Shift+Enter` executes")])
add("solve", "Stop a running check", "Request cancellation of a long check", ["Stop check", "Shift+Escape"], ["none"], ["none"], "released",
    [("N", "**Stop check** / `Shift+Escape`")])
add("solve", "New operation and reuse steps", "Empty all three phases, or keep the exact recipe for a new target with a fresh check",
    ["Ctrl+N", "Ctrl+Alt+R"], ["workspace"], ["workspace"], "released",
    [("N", "Choose **New operation** (`Ctrl+N`)"), ("N", "**Reuse steps** (`Ctrl+Alt+R`)")])
add("solve", "Forecast comparison", "Compare Actual with After Prepare, After Macro and After Cleanup without executing",
    ["Actual / After Prepare / After Macro / After Cleanup"], ["state", "workspace"], ["none"], "released",
    [("N", "Compare **Actual / After Prepare / After Macro / After Cleanup**")])
add("solve", "Cycle views Current / Operation / After", "Show the actual residual, the selected operation and its predicted residual",
    ["cycle view mode"], ["state", "workspace"], ["none"], "released",
    [("U", "**Current / Operation / After** distinguish"), ("E", "The shared graph has three distinct modes")])
add("solve", "Effect scope Macro body / All steps", "Switch the displayed effect between the macro alone and the whole operation",
    ["Macro body / All steps"], ["workspace"], ["none"], "released", [("N", "**Macro body / All steps** changes")])
add("solve", "Local and Global views", "Local: the piece and its cells; Global: the work and target regions in the whole 600-cell",
    ["Actual Local", "Actual Global"], ["state"], ["none"], "released",
    [("N", "**Actual Local / Actual Global**"), ("B", "### Local 应当让我在原地完成判断")])
add("solve", "Current and locked Next", "Keep the current target and a locked next target; activate Next explicitly",
    ["Activate Next (Ctrl+Alt+N)"], ["workspace"], ["workspace"], "released",
    [("U", "**Current**, locked **Next**"), ("N", "Explicitly **Activate Next** (`Ctrl+Alt+N`)")])
add("solve", "Token and socket inspection and actions", "Inspect identities and fixed positions; open an object's explicit actions",
    ["click token/socket", "Enter", "arrow keys", "Shift+F10"], ["state"], ["none"], "released",
    [("N", "On a token/socket, Enter inspects")])
add("solve", "Entry slots", "Read-only slot and label correspondence of the draft", ["Entry slots"], ["state", "workspace"], ["none"], "released",
    [("N", "**Entry slots** opens the exact")])
add("solve", "Buffer roles and buffer analyser", "Fixed buffer positions, their occupants, candidate frames, setup depth and primitive cost",
    ["buffer analyser", "A/B role assignment"], ["state"], ["workspace"], "released",
    [("S", "the buffer analyser can show"), ("RM", "orientation and buffer tools")])
add("solve", "Block requirements", "Record exact target positions as a block; add current state or Home",
    ["Add inspected current state as a block requirement", "Add to Home block"], ["state"], ["workspace"], "released",
    [("N", "**Add inspected current state as a block requirement**")])
add("solve", "Work intents", "Declare place, orient, finish-buffer or finish-orbit goals; goal outcome separate from permission",
    ["intent picker"], ["state", "workspace"], ["workspace"], "released",
    [("C", "Added explicit place/orient/buffer/orbit completion intentions"), ("E", "## Explicit finite work intents")])
add("solve", "Residuals, stages and invariant diagnostics", "Show remaining position and orientation residuals per orbit and certified invariant checks",
    ["residual display"], ["state"], ["none"], "released",
    [("C", "source-bound"), ("E", "## Exact residuals and reversible stage interpretation")])
add("solve", "Findings panel", "Separate declared boundary, final preservation and intermediate motion",
    ["Findings (Shift+F12)"], ["state", "protection", "workspace"], ["none"], "released", [("N", "**Findings** (`Shift+F12`)")])
add("solve", "Conflict location on Prepare page", "Show a located conflict's identity and current position", ["Solve > Prepare"],
    ["state", "protection"], ["none"], "released", [("C", "Use Solve's Prepare page")])
add("solve", "Cross-orbit work", "Keep per-orbit drafts and selected macro revisions when switching work orbit",
    ["work-orbit switch"], ["workspace"], ["workspace"], "released",
    [("C", "Preserved per-orbit drafts"), ("N", "Work-orbit switching retains declared position protection")])
add("solve", "Recommendations and scores", "Rank existing macros by explained scores; never execute or grant permission",
    ["candidate list"], ["state", "workspace", "protection"], ["none"], "released",
    [("C", "Scores remain"), ("NM", "## Two separate ranking layers")])
add("solve", "Mathematical names and typed addresses", "Structure names for orbits and addresses for positions alongside canonical IDs",
    ["names in all views"], ["state"], ["none"], "released",
    [("C", "Added mathematical structure names"), ("NM", "PositionAddress = CellAddress")])

# E. Macros
add("macros", "Macro Base", "Search and select a stored macro; selection inspects, Add inserts",
    ["Macro Base (F3)", "Add (Ctrl+M)"], ["workspace"], ["workspace"], "released",
    [("N", "Open Macro Base (`F3`)"), ("C", "Connected Macro Base selection")])
add("macros", "Bank fixed macros", "Select the current bank's first or second fixed macro without appending",
    ["Ctrl+Alt+1", "Ctrl+Alt+2"], ["workspace"], ["workspace"], "released", [("N", "`Ctrl+Alt+1 / 2` selects")])
add("macros", "Candidate comparison and saved worksheets", "Compare candidate macros; save and reload worksheets",
    ["worksheet save/load"], ["workspace"], ["workspace"], "released", [("C", "candidate comparison, saved worksheets")])
add("macros", "Endgame families", "Choose a family, auxiliary positions and finite parameters; inspect collateral; save as macro",
    ["Solve > Endgame"], ["state", "workspace"], ["workspace"], "released",
    [("U", "Endgame families are available inside Solve")])
add("macros", "Reference variants", "Reuse a macro under explicit source and destination ordered frames",
    ["reference variant editor"], ["workspace"], ["workspace"], "released",
    [("U", "Reference variants require explicit source and destination ordered frames")])

# F. Protection
add("protection", "Protection policies", "Net or strict protection; a change invalidates the preview",
    ["policy picker"], ["protection"], ["protection"], "released",
    [("U", "Changing a protection policy invalidates"), ("E", "Net protects requirements")])
add("protection", "Protected-orbit checks and automatic orbit protection", "Reject operations that break protected orbits; protect an orbit when it becomes solved",
    ["automatic on commit"], ["protection", "state"], ["protection"], "released",
    [("RM", "protected-orbit checks"), ("E", "Automatic Orbit Protection")])
add("protection", "Position protection", "Protect or release a position; persistent protection strip",
    ["object menu > Protect position", "release"], ["protection"], ["protection"], "released",
    [("N", "choose **Protect position**"), ("N", "Explicitly release a position")])

# G. Input
add("input", "Onscreen keyboard", "Shows the active set and its functions; operable, not decorative",
    ["Keyboard (F9)"], ["preferences"], ["none"], "released",
    [("U", "The onscreen keyboard shows the active set"), ("N", "Open **Keyboard** (`F9`)")])
add("input", "Grip and Twist", "Select a cap and frame, then a legal twist; inverse by click or Shift; hold or latch",
    ["Grip keys", "Twist keys H1-H3/T1-T4", "Click inverse", "Shift"], ["preferences", "state"], ["state", "journal", "workspace"], "released",
    [("N", "The keyboard's two Grip rows"), ("B", "Grip 与 Twist 两种动作")])
add("input", "Live or draft input destination", "Send input to the live puzzle or into the draft phase",
    ["keyboard input menu"], ["workspace"], ["workspace"], "released", [("N", "choose live/draft through its input menu")])
add("input", "Keymap banks", "Orbit-specific key sets, five banks per orbit; switching keeps orbit and draft",
    ["bank picker", "type bank ID"], ["preferences"], ["preferences"], "released",
    [("N", "The bank picker shows five real banks"), ("C", "orbit-specific editable keymap sets")])
add("input", "Set grips", "Capture caps for an insertion bank explicitly", ["Set grips"], ["state"], ["preferences"], "released",
    [("N", "use **Set grips** explicitly")])
add("input", "Functions bank", "Utility actions on their own bank, separate from solving keys",
    ["Functions toggle"], ["preferences"], ["preferences"], "released",
    [("U", "**Functions** groups utility actions"), ("F", "registers `functions-toggle`")])
add("input", "Key binding editor", "Change a key inline with scope and conflict display; apply or cancel",
    ["Change key", "right-click action"], ["preferences"], ["preferences"], "released",
    [("N", "Choose **Change key**"), ("U", "Use its set picker to inspect or edit bindings")])
add("input", "Command index", "Find any command by search; one registry for graph actions, picker and keyboard",
    ["Find command (F1)", "All commands", "Ctrl+F1"], ["none"], ["none"], "released",
    [("N", "**Find command** (`F1`)")])
add("input", "Physical keyboard input with text and IME isolation", "Keys never twist while typing; repeat and lost key-up handled",
    ["physical keys"], ["preferences"], ["state"], "released",
    [("R", "physical keyboard input"), ("B", "必须处理文本输入、IME")])

# H. View
add("view", "Camera rotation and mouse twists", "3D/4D drag, Ctrl gestures, multi-click twists, exact picking",
    ["Shift+left drag", "drag", "multi-click"], ["preferences"], ["state", "journal"], "released",
    [("S", "Shift+left **drag** remains a 4D camera rotation"), ("B", "上次已有的 3D / 4D 拖动")])
add("view", "Piece inspection by click", "Shift+left: occupant and its home; Shift+right: required identity for a destination",
    ["Shift+left click", "Shift+right click", "Clear inspection"], ["state"], ["none"], "released",
    [("S", "Shift+left **click** on a pickable piece")])
add("view", "Piece filter", "Expression filter with presets, compose (replace/union/intersect/subtract), preview counts, explicit apply",
    ["Piece Filter (F5)", "Apply filter", "Cancel"], ["state", "preferences"], ["preferences"], "released",
    [("N", "Open **Piece Filter** with `F5`"), ("S", "## Exact filter semantics")])
add("view", "Framework visibility", "Show or hide the 600-cell frame", ["display controls"], ["preferences"], ["preferences"], "released",
    [("C", "Restored explicit controls for framework visibility"), ("S", "**Hide 600-cell frame**")])
add("view", "Adaptive motion detail", "Lower detail while the camera moves; full detail returns at rest",
    ["display controls"], ["preferences"], ["preferences"], "released", [("C", "adaptive motion and the")])
add("view", "Retained MPUlt display sliders and view reset", "Cell and sticker size, field of view, lighting; Reset view",
    ["display dialog", "Reset view"], ["preferences"], ["preferences"], "released",
    [("C", "retained MPUlt size, field-of-view and lighting settings"), ("D", "Reset fields")])
add("view", "Instant turn", "A twist appears only after its durable commit; no turn animation",
    ["automatic"], ["state"], ["none"], "released",
    [("C", "preserving Instant"), ("S", "Native twists appear instantly")])

# I. Structure explorer and older tools (0.3 reference; presence in 0.4 unclear)
add("structure", "Structure explorer", "Find a colour, explore 1- or 2-hop neighbours, compare cells, list vertices",
    ["Structure tab", "Find color", "Compare"], ["none"], ["none"], "0.3-reference",
    [("S", "The **Structure** tab helps locate cells")])
add("structure", "Cell layers", "Exact shared-face BFS shells L0 to L15 as filter predicates",
    ["Layers", "Use current layer", "Use home layer"], ["none"], ["none"], "0.3-reference", [("S", "## Cell-centered layers")])
add("structure", "Auxiliary cell views", "Global overview and Focused neighbourhood windows with Follow/Pin and Center main",
    ["Cell views", "View menu"], ["state"], ["none"], "0.3-reference", [("S", "## Auxiliary cell views")])
add("structure", "Center viewport", "Explicit camera move to a selected cell", ["Center viewport", "Center main"], ["none"], ["none"], "0.3-reference",
    [("S", "Use **Center viewport** for an explicit camera change")])
add("structure", "Saved sets", "Frozen identity or position sets up to 10,000 members", ["saved set"], ["state"], ["preferences"], "0.3-reference",
    [("L", "| Saved sets |")])
add("structure", "Browser client", "Older web client with its own control and preset set; not the 0.4 entry point",
    ["web/ launch scripts"], ["state"], ["preferences"], "0.3-reference",
    [("L", "| Client parity |"), ("RM", "Root launch scripts, `web/` and `packaging/`")])

unclear = [
 {"item": "Whether the Structure tab, auxiliary cell views, cell layers and saved sets are reachable in the 0.4 G2 workspace", "evidence": [ev("S", "Shared-model and historical 0.3 interface reference")]},
 {"item": "G1 versus G2 feature parity; G1 ordered-frame capture was missing in the development sample", "evidence": [ev("N", "Ordered-frame capture is not yet implemented")]},
 {"item": "Scramble kinds (short, chosen length, full) and their definitions in 0.4", "evidence": [ev("B", "Scramble 放在熟悉且可键盘访问的位置")]},
 {"item": "Whether Reset view and Reset workspace exist as separate 0.4 actions", "evidence": [ev("B", "Reset puzzle、Reset view、Reset workspace 是不同动作")]},
 {"item": "Macro recording (the brief asks for record, input or select); release docs name only select and fixed-step editing", "evidence": [ev("B", "用户从macro base选择、输入或录制macro"), ev("C", "fixed-step editing")]},
 {"item": "Development fixtures reachable from All commands (for example fixture-e1) in the released package", "evidence": [ev("N", "search `fixture-e1`")]},
 {"item": "Progress reports and branch totals as a 0.4 view", "evidence": [ev("L", "Orbit progress and current-branch transaction totals")]},
]

out = {"inventory": "top-down", "method": "user and design documents only; no source code read",
       "sources": sorted(set(P.values())), "functions": rows, "unclear": unclear}
dst = ROOT / "docs/progress/1.0/inventory/top-down.json"
dst.parent.mkdir(parents=True, exist_ok=True)
dst.write_text(json.dumps(out, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
from collections import Counter
print(len(rows), len(unclear), Counter(r["status"] for r in rows), Counter(r["area"] for r in rows))
