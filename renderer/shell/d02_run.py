"""Play a D02 script in the product shell and compare every screen with its expectation (ADR-D02 build step B4).

  python -B renderer/shell/d02_run.py --script NAME --out NEW_FOLDER [--build DIR] [--state ID]

It starts the newest product build (or --build) through launch.py with `--frame --puzzle jumble-on --d02` and the
script renderer/shell/d02_scripts/NAME.json. The shell plays the script inside: each step's chord goes through the
shell's own key entry, and after each step the shell writes the screen's text, pane by pane, to d02-screen.jsonl in
the run folder, then quits after the last step. This script computes each step's expectation from the design and
sample files and prints `M600_D02_RUN_PASS script=NAME steps=N mismatches=0`, or `M600_D02_RUN_FAIL` with the
mismatches, and writes d02-run.json beside the shell's logs.

The `bind` script (B5) runs on the named state's session fixture, which `build_d02_fixtures.py` writes under
work/build/d02-fixtures/: the sample binds by its pose digests after the open, a twist unbinds it, an undo binds it
again. The `host` script (B6) runs in sample mode on the state's fixture and checks the Jumble host in Solve. A state
with a written fixture also opens it in the other scripts.

The owner's window rule (26 November 2026) is checked on the way: the window that has the foreground before the
launch must still have it five seconds after it, the shell's window never carries WS_EX_TOPMOST, and no screen dump
finds the shell in the foreground. A click into the window during the run fails that check; leave it alone. A run
opens a window on the GPU for about half a minute (machine mark W): never during a quiet-machine hold. Exit 0 on a
pass, 1 on a failure, 2 for a usage error.
"""
from __future__ import annotations

import hashlib
import sys

sys.dont_write_bytecode = True

import argparse
import json
import re
import subprocess
import threading
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import launch  # noqa: E402

ROOT = HERE.parents[1]
SCRIPTS = HERE / "d02_scripts"
DESIGN = ROOT / "tools" / "looklab" / "data" / "design"
SAMPLES = ROOT / "tools" / "looklab" / "data" / "samples" / "jumble"
TABLE = ROOT / "docs" / "progress" / "1.0" / "command-table.json"
SCREEN = "d02-screen.jsonl"
# The D02 session fixtures, one folder per sample state (build_d02_fixtures.py), and the binding chip's words.
FIXTURES = ROOT / "work" / "build" / "d02-fixtures"
BOUND, UNBOUND = "bound · ", "unbound · stale"
# Local's empty sentence (ADR-D02.12; jumble-words.json local.empty), as the owner asked for it on 26 November 2026.
LOCAL_EMPTY = "Name a cap: click a numbered cap on the cap map, or F1 › jumble.cap.name <id>"
# Local is never empty (owner, 26 November 2026): with a state bound and no cap named it draws the lowest cap, cap 0,
# and its readout says so with the empty sentence as the hint (LocalCap.Words); the default is never named.
LOCAL_DEFAULT = "Local shows cap 0 by default; no cap is named. " + LOCAL_EMPTY
# B32 findings: in sample mode the key map holds F1 (ADR-D01.74), so the hint names the palette's Space there.
LOCAL_EMPTY_SAMPLE = LOCAL_EMPTY.replace("F1 ›", "Space ›")
LOCAL_DEFAULT_SAMPLE = "Local shows cap 0 by default; no cap is named. " + LOCAL_EMPTY_SAMPLE
RECORD = "d02-run.json"
FORMAT = "magic600-d02-run-v1"
FOREGROUND_AFTER_S = 5.0
RUN_TIMEOUT_S = 240.0


def load(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def data() -> dict:
    """What the expectations are computed from: the draft rows, the table's rows and the sample states."""
    drafts = load(DESIGN / "command-rows-d02.json")
    return {"rows": drafts["commands"] + drafts.get("grip", []), "table": load(TABLE)["commands"],
            "states": [state["state"] for state in load(SAMPLES / "states.json")["states"]]}


def palette_matches(query: str, rows: list[dict], table: list[dict]) -> list[str]:
    """The palette's rows for a query, in registry order (a fresh session has no pins or uses): every term in the
    label or the command id (PaletteModel.Rows). The labels shown are the D02 rows' own; table rows keep theirs."""
    terms = query.lower().split()
    found = []
    seen = set()
    for row in table + rows:
        if row["id"] in seen:
            continue
        seen.add(row["id"])
        if all(term in row["label"].lower() or term in row["id"].lower() for term in terms):
            found.append(row["label"])
    return found


def overlay(dump: dict) -> list[str]:
    return [text for group, text in dump["lines"] if group.startswith("overlay:")]


def expect_smoke(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """B4's smoke script: D02 is on; the palette lists the CR-16 draft rows (B30 added tuning.open), each marked; Escape closes it."""
    problems = []
    if [dump["do"] for dump in dumps] != ["start", "chord F1", "text tuning.", "chord Escape", "wait 6000 ms"]:
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    chip = "D02 · " + (state or "no state")
    if (("frame", chip) not in {(group, text) for group, text in map(tuple, dumps[0]["lines"])}):
        problems.append(f"step 0: the frame does not show {chip!r}")
    if overlay(dumps[0]):
        problems.append("step 0: an overlay is open at the start")
    if not overlay(dumps[1]):
        problems.append("step 1: F1 did not open the palette")
    drafts = {row["label"] for row in source["rows"]}
    wanted = palette_matches("tuning.", source["rows"], source["table"])[:12]
    if len(wanted) != 9 or not set(wanted) <= drafts:
        problems.append(f"the data: tuning. matches {wanted}, not the nine CR-16 draft rows")
    shown = overlay(dumps[2])
    labels = [text for text in shown if text in drafts]
    if labels != wanted:
        problems.append(f"step 2: the palette lists {labels}, expected {wanted}")
    if shown.count("draft row") != len(wanted):
        problems.append(f"step 2: {shown.count('draft row')} draft-row marks for {len(wanted)} draft rows")
    if "tuning." not in shown:
        problems.append("step 2: the query field does not show tuning.")
    for step in (3, 4):
        if overlay(dumps[step]):
            problems.append(f"step {step}: the palette is still open")
    return problems


def expect_help_card(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    steps = ["start", "wait 2000 ms", "hover CapMapCells", "chord F1", "chord Escape",
             "command help.open cell", "click HelpCardLearn", "wait 6000 ms"]
    if [dump["do"] for dump in dumps] != steps:
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    problems = []
    cap = overlay(dumps[3])
    if "Cap" not in cap or "Open in Learn" not in cap:
        problems.append("step 3: F1 over the cap map did not open Cap help with its Learn link")
    words = sum(len(line.split()) for line in cap)
    if words > 50:
        problems.append(f"step 3: the Cap card shows {words} words")
    if overlay(dumps[4]):
        problems.append("step 4: Escape left an overlay open")
    caption = "One of the 600 cells (bold) and its four neighbours."
    cell = overlay(dumps[5])
    if "Cell" not in cell or caption not in cell:
        problems.append("step 5: help.open cell did not show Cell and its figure caption")
    learn = texts(dumps[6], "frame")
    if "Cell" not in learn or caption not in learn or overlay(dumps[6]):
        problems.append("step 6: the link did not show Cell and its figure in Learn")
    return problems


def bind_chip(dump: dict) -> str | None:
    """The binding chip's words in a dump (B5): `bound · <state>` or `unbound · stale`."""
    for group, text in map(tuple, dump["lines"]):
        if group == "frame" and (text.startswith(BOUND) or text == UNBOUND):
            return text
    return None


def expect_bind(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """B5's bind script on the state's session fixture: the sample binds after the open, one twist unbinds it, and an
    undo binds it again. The dumps right after a key are not judged: the shown revision may still lag the head."""
    steps = ["start", "wait 3000 ms", "chord J", "wait 1500 ms", "chord Ctrl+Z", "wait 3000 ms"]
    if [dump["do"] for dump in dumps] != steps:
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    if state is None:
        return ["the bind script needs a sample state"]
    problems = []
    if any("the session refused this twist" in text for group, text in map(tuple, dumps[3]["lines"])):
        problems.append(f"step 2: the session refused the twist at {state}: the default grip is cap 0, so "
                        "the bind script needs a state where cap 0 admits a twist, such as on-zero")
    for index, wanted in ((1, BOUND + state), (3, UNBOUND), (5, BOUND + state)):
        shown = bind_chip(dumps[index])
        if shown != wanted:
            problems.append(f"step {index}: the binding chip shows {shown!r}, expected {wanted!r}")
    return problems


def texts(dump: dict, prefix: str = "") -> list[str]:
    return [text for group, text in map(tuple, dump["lines"]) if group.startswith(prefix)]


def expect_pinned_cards(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """ADR-D01.24 on wj-ia-300's session fixture: Follow piece 4021 shows its Tracked piece card with its pose row's
    values; Pin card 17 shows cap 17's card; no reference column before Set reference state; after it the header names
    the reference and the later cap card (18) shows a reference column; no row is refused."""
    steps = ["start", "wait 2500 ms", "command card.track 4021", "command card.pin 17", "command card.reference.set",
             "command card.pin 18", "wait 6000 ms"]
    if [dump["do"] for dump in dumps] != steps:
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    problems = []
    tracked = texts(dumps[2], "panel:cards")
    if "Tracked piece 4021" not in tracked or "Domain" not in tracked or "On lattice" not in tracked:
        problems.append("step 2: the Cards dock did not show Tracked piece 4021 with its pose row's fields")
    if "Cap 17" not in texts(dumps[3], "panel:cards"):
        problems.append("step 3: the Cards dock did not show cap 17's card")
    if any(line.startswith("Reference:") for i in (2, 3) for line in texts(dumps[i], "panel:cards")):
        problems.append("a reference header showed before setting a reference state")
    for i in (4, 5):
        if not any(line.startswith("Reference: ") for line in texts(dumps[i], "panel:cards")):
            problems.append(f"step {i}: the Cards dock did not show the reference header")
    if "Cap 18" not in texts(dumps[5], "panel:cards"):
        problems.append("step 5: the Cards dock did not show cap 18's card")
    for i, label in ((2, "Follow piece"), (3, "Pin card"), (4, "Set reference state"), (5, "Pin card")):
        problems.extend(f"step {i}: {line}" for line in texts(dumps[i], "frame") if line.startswith(label + ": "))
    print(f"pinned-cards: {sum(1 for line in texts(dumps[5], 'panel:cards') if line.startswith(('Cap ', 'Tracked piece ')))} cards at the end")
    return problems


def expect_explore_menu(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """The Global menu closes on Escape, explore pauses a commit, and both marks follow their state on both views."""
    steps = ["start", "wait 2500 ms", "command menu.context.open global", "wait 600 ms", "chord Escape",
             "command explore.set on", "wait 400 ms", "command operation.preview.commit",
             "command token.override.set state.committed=0.6 0.15 30", "wait 600 ms",
             "command token.override.reset state.committed=reset", "command explore.set off", "wait 6000 ms"]
    if [dump["do"] for dump in dumps] != steps:
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    problems = []
    labels = {row["id"]: row["label"] for row in source["table"] + source["rows"]}
    if labels["explore.set"] not in texts(dumps[3], "overlay:ContextMenu"):
        problems.append("step 3: the Global context menu did not show its explore entry")
    if overlay(dumps[4]):
        problems.append("step 4: Escape left an overlay open")

    for i, on in ((0, False), (5, True), (6, True), (10, True), (11, False), (12, False)):
        if dumps[i].get("explore") != {"on": on, "global": int(on), "local": int(on)}:
            problems.append(f"step {i}: both view frames should carry {int(on)} explore marks")
    refusal = labels["operation.preview.commit"] + ": explore state: commits are paused"
    if refusal not in texts(dumps[7], "frame"):
        problems.append("step 7: the commit was not refused by the explore state")

    opening = dumps[0].get("colours", {}).get("digest")
    if not re.fullmatch(r"[0-9a-f]{12}", str(opening)):
        problems.append("step 0: the opening token digest is missing or malformed")
    for i, marked in ((0, False), (8, True), (9, True), (10, False), (11, False), (12, False)):
        colours = dumps[i].get("colours", {})
        if (colours.get("nonstandard"), colours.get("global"), colours.get("local")) != (marked, int(marked), int(marked)):
            problems.append(f"step {i}: both view frames should carry {int(marked)} colour marks")
        digest = colours.get("digest")
        if not re.fullmatch(r"[0-9a-f]{12}", str(digest)) or (digest != opening) != marked:
            problems.append(f"step {i}: the token digest does not match the colour override state")

    for i, command in ((2, "menu.context.open"), (5, "explore.set"), (8, "token.override.set"),
                       (10, "token.override.reset"), (11, "explore.set")):
        problems.extend(f"step {i}: {line}" for line in texts(dumps[i], "frame") if line.startswith(labels[command] + ": "))
    for i in (8, 10):
        if status := dumps[i].get("tuning", {}).get("status"):
            problems.append(f"step {i}: the tuning panel refused the command: {status}")
    return problems


def local_problems(dump: dict, step: int, cap: int, named: bool, framed: bool = True) -> list[str]:
    """Local is never empty (owner, 26 November 2026): Local draws `cap`; its empty sentence never shows while it
    does; the default's readout (LOCAL_DEFAULT) shows while no cap is named, and the dump names no cap then; a named
    cap is named in the dump and its readout says nothing of a default. With `framed`, the shell frames the same cap in
    Local and shows Local's opening."""
    local, lines, problems = dump.get("local") or {}, texts(dump, "view:Local"), []
    if local.get("cap") != cap:
        problems.append(f"step {step}: Local draws cap {local.get('cap')}, not cap {cap}")
    if framed and (local.get("framed") != cap or local.get("opening") is not True):
        problems.append(f"step {step}: the shell frames cap {local.get('framed')} in Local with its opening "
                        f"{'shown' if local.get('opening') else 'hidden'}, not cap {cap} with it shown")
    default = LOCAL_DEFAULT_SAMPLE if dump.get("sample") else LOCAL_DEFAULT
    if LOCAL_EMPTY in lines or LOCAL_EMPTY_SAMPLE in lines:
        problems.append(f"step {step}: Local shows its empty sentence while it draws a cap")
    if named and (dump.get("named") != cap or local.get("words") or any(line.startswith("Local shows cap") for line in lines)):
        problems.append(f"step {step}: cap {dump.get('named')} is named and Local's readout reads {local.get('words')!r}, "
                        f"expected cap {cap} named and no default")
    if not named and (dump.get("named") != -1 or default not in lines):
        problems.append(f"step {step}: cap {dump.get('named')} is named or Local's readout does not say that it shows the "
                        "default cap")
    return problems


def expect_host(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """B6's host script, in sample mode on the state's session fixture: the Jumble cell, the mark on both view borders,
    Local on the default cap 0 with its readout saying so (no cap is named: the cap map's panel says so) and four closed
    sections, the cap map drawn (B8) with its class counts and its legend armed,
    and the strip and the rail undefined exactly when the sample is off the lattice (its totals' exact off-lattice
    count)."""
    if [dump["do"] for dump in dumps] != ["start", "wait 3000 ms", "wait 3000 ms"]:
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    if state is None:
        return ["the host script needs a sample state"]
    off = load(SAMPLES / state / "totals.json")["off_lattice"] > 0
    shown, problems = dumps[1], []
    every = texts(shown)
    cell = "Jumble · " + ("off the lattice" if off else "on the lattice")
    if cell not in texts(shown, "frame"):
        problems.append(f"step 1: the top bar does not show {cell!r}")
    marks = shown.get("marks", {})
    for view in ("Global", "Local"):
        # With labels on the mark's word is on screen; with labels off it is in the mark's tooltip.
        if "jumble" not in texts(shown, "view:" + view) and not any(tip.startswith("jumble") for tip in marks.get(view, [])):
            problems.append(f"step 1: view:{view} carries no Jumble mark")
    # Local is never empty: the bound state's Local draws the default cap, which counts as no named cap. The shell's
    # framing is asked by the runs on a fixture outside sample mode (name-cap, two-views).
    problems += local_problems(shown, 1, 0, named=False, framed=False)
    if "no cap named" not in texts(shown, "panel:jumble.cap_map"):
        problems.append("step 1: the cap map's panel does not say that no cap is named")
    for header in ("▸ Named cap · 0", "▸ Twist check · 0", "▸ Cap covers · 0", "▸ Repair list · 0"):
        if header not in every:
            problems.append(f"step 1: Local lacks the closed header {header!r}")
    if "Cap map · 20 rings of 30 caps" not in every:
        problems.append("step 1: the cap map is not drawn in the bottom dock")
    # B8: the map's legend counts the bound state's cap classes, as its totals record them; a sample written without
    # its rim rows (rim.json) leaves every cap pending, since the map draws certified rows only.
    classes = load(SAMPLES / state / "totals.json")["cap_classes"]
    if not (SAMPLES / state / "rim.json").is_file():
        classes = {"blocked": 0, "closed": 0, "at_floor": 0, "above_floor": 0, "pending": 600}
    for key, word in (("blocked", "blocked"), ("closed", "closed"), ("at_floor", "at floor (N_c = F_c > 0)"),
                      ("above_floor", "above floor"), ("pending", "pending")):
        if f"{word} · {classes[key]}" not in every:
            problems.append(f"step 1: the cap map's legend does not count {classes[key]} caps {word}")
    if not any("contact classes · armed" in text and "never hidden · armed" in text for text in every):
        problems.append("step 1: the cap map's legend does not show the entry's armed modes")
    # The strip and the rail are in the frame; the totals on the view borders have their own undefined slots (B7).
    undefined = texts(shown, "frame").count("undefined")
    if off and undefined < 2:
        problems.append(f"step 1: the strip and the rail are not undefined off the lattice ({undefined} undefined numerals)")
    if not off and undefined:
        problems.append(f"step 1: {undefined} undefined numerals on the lattice")
    return problems


# The drag's sense (owner, 26 November 2026): the near side follows the pointer, as in Look Lab. A point at the 4D
# near side (w toward d4, which the core's projection magnifies) must move right on screen after a rightward drag and
# down after a downward one. The core applies x' = Q x with Q column-major and draws clip y up.
NEAR_POINT = (0.0, 0.0, 0.0, 0.9)


def near_side(camera: list[float]) -> tuple[float, float]:
    """The screen x and y (y up) of the near point under a column-major camera, before the perspective factor, which
    is positive and changes neither sign."""
    q = [[camera[column * 4 + row] for column in range(4)] for row in range(4)]
    world = [sum(q[row][k] * NEAR_POINT[k] for k in range(4)) for row in range(4)]
    return world[0], world[1]


def expect_rotate(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """The rotate script: each drag in the Global opening turns the camera (its drag steps counted), a wait turns
    nothing, and the near side follows the pointer: right after the rightward drag, down after the downward one."""
    steps = ["start", "wait 2000 ms", "drag 200 0", "wait 1000 ms", "drag 0 120", "wait 1000 ms"]
    if [dump["do"] for dump in dumps] != steps:
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    turns = [dump.get("camera_turns", 0) for dump in dumps]
    problems = []
    if turns[1] != 0:
        problems.append(f"the camera turned before any drag ({turns[1]} steps)")
    if not turns[2] > turns[1]:
        problems.append("the first drag did not turn the camera")
    if not turns[4] > turns[3]:
        problems.append("the second drag did not turn the camera")
    if turns[3] != turns[2] or turns[5] != turns[4]:
        problems.append("the camera turned during a wait")
    cameras = [dump.get("camera") for dump in dumps]
    if any(camera is None or len(camera) != 16 for camera in cameras):
        return problems + ["a step's dump has no camera"]
    x1, y1 = near_side(cameras[1])
    x2, y2 = near_side(cameras[2])
    x4, y4 = near_side(cameras[4])
    if not x2 > x1 + 1e-3:
        problems.append(f"the rightward drag moved the near side left (x {x1:.3f} to {x2:.3f}); Look Lab moves it right")
    if abs(y2 - y1) > 1e-3:
        problems.append("the rightward drag moved the near side vertically")
    if not y4 < y2 - 1e-3:
        problems.append(f"the downward drag moved the near side up (y {y2:.3f} to {y4:.3f}); it must follow the pointer down")
    print(f"near side: right drag x {x1:+.3f} to {x2:+.3f}; down drag y {y2:+.3f} to {y4:+.3f}")
    return problems


def pose_count(state: str, keep) -> tuple[int, int]:
    """The pieces of a sample state's pose rows that `keep(domain, height, lattice)` holds, and all of them."""
    style = load(SAMPLES / state / "style.json")
    rows = [(row["domain"], row["heights"][0], row["count"]) for row in style["rows"]]
    lattice = style["lattice_domain"]
    return sum(n for d, h, n in rows if keep(d, h, lattice)), sum(n for _, _, n in rows)


def expect_focus_layer(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """The Focus layer script (owner, 26 November 2026): the entry reads "Focus layer =" on Global's border; a preview
    shows its exact count of the bound state's pieces before anything applies; the apply adds the layer to Global's
    focus-layer keys with that count; an intersect composes with it in place; the palette finds the rows by the alias
    "piece filter" under their Focus layer labels."""
    if state is None or not (SAMPLES / state / "style.json").is_file():
        return ["the focus-layer script needs a sample state with pose rows"]
    by_step = {dump["do"]: dump for dump in dumps}
    marks = ["wait 801 ms", "wait 802 ms", "wait 503 ms", "wait 804 ms", "wait 805 ms", "wait 806 ms"]
    if any(mark not in by_step for mark in marks):
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    layers = [by_step[mark].get("focus_layer") for mark in marks]
    if any(not isinstance(layer, dict) for layer in layers):
        return ["a step's dump has no focus layer"]
    off, total = pose_count(state, lambda d, h, lattice: d != lattice)
    both, _ = pose_count(state, lambda d, h, lattice: d != lattice and h >= 4)
    problems = []
    first, applied, _, second, again, _ = layers
    if first["entry"] != "Focus layer =":
        problems.append(f"Global's border has no Focus layer entry ({first['entry']!r})")
    if first["dock"] != "focus_layer.editor":
        problems.append(f"the editor is not open in the right dock ({first['dock']!r})")
    if not first["line"].startswith(f"{off:,} of {total:,} pieces · replace · live query · rev "):
        problems.append(f"the preview reads {first['line']!r}, not {off:,} of {total:,} pieces")
    if any(key.startswith("= ") for key in first["keys"]):
        problems.append("the layer applied at its preview")
    if f"= not lattice · {off:,}" not in applied["keys"]:
        problems.append(f"the applied layer is not on Global's keys: {applied['keys']}")
    if not second["line"].startswith(f"{both:,} of {total:,} pieces · intersect · live query"):
        problems.append(f"the intersect preview reads {second['line']!r}, not {both:,}")
    expected = f"= not lattice and height >= 4 · {both:,}"
    if expected not in again["keys"] or sum(key.startswith("= ") for key in again["keys"]) != 1:
        problems.append(f"the composed layer did not replace the first in place: {again['keys']}")
    palette = texts(by_step["wait 806 ms"], "overlay:CommandPalette")
    for label in ("Open focus layer", "Preview focus layer counts", "Apply focus layer"):
        if label not in palette:
            problems.append(f"the palette did not find {label!r} by the alias 'piece filter'")
    print(f"focus layer: not lattice {off:,} of {total:,}; and height >= 4 {both:,}; keys {again['keys']}")
    return problems


def expect_seed7001(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """B25's seed7001 script: the Word panel plays the nine words by part and, in an opened part, by twist; after each
    played part the state digest is that word file's output digest and N is the sample's; the commit bar's line gives
    the exact length and alphabet; the signature is undefined (the word starts off the lattice); at solved the log tab
    shows N = 0 and the crossing sentence shows with its reachability."""
    words = load(SAMPLES / "seed7001-words.json")
    parts = words["parts"]
    played = [dump for dump in dumps if dump["do"] == "wait 300 ms" and isinstance(dump.get("word"), dict)]
    if len(played) != 10:
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    problems = []
    first = played[0]["word"]
    if first["length"] != f"{len(parts)} twists · alphabet {words['alphabet']}":
        problems.append(f"the commit bar's line reads {first['length']!r}")
    if not first["signature"].startswith("35 slots undefined"):
        problems.append(f"the off-lattice word's signature reads {first['signature']!r}")
    counters = [dump["word"]["counter"] for dump in played]
    wanted = ["part 1 of 9", "part 2 of 9", "part 3 of 9, twist 0 of 1", "part 3 of 9"] + [f"part {k} of 9" for k in range(4, 10)]
    if counters != wanted[:len(counters)]:
        problems.append(f"the counters read {counters}")
    done = [1, 2, 2, 3, 4, 5, 6, 7, 8, 9]
    for dump, k in zip(played, done):
        if dump["word"]["digest"] != parts[k - 1]["output_digest"]:
            problems.append(f"after {dump['word']['counter']} the digest is not {parts[k - 1]['file']}'s output")
        n = load(SAMPLES / parts[k - 1]["after"] / "totals.json")["N"]
        if not dump["word"]["state"].startswith(f"N {n:,} "):
            problems.append(f"after {dump['word']['counter']} the state reads {dump['word']['state']!r}, not N {n:,}")
    last = played[-1]["word"]
    if last["log"] != "Log tab: N = 0 · no residual ticks":
        problems.append(f"the solved log tab reads {last['log']!r}")
    if not last["crossing"].startswith("On the lattice, ν = 0: labels pass to 600-cell-Full"):
        problems.append(f"the crossing sentence at solved reads {last['crossing']!r}")
    if any(dump["word"]["crossing"] for dump in played[:-1]):
        problems.append("the crossing sentence showed before solved")
    print(f"seed7001: {' -> '.join(c for c in counters)}; end {last['state']}")
    return problems


def nu_words(state: str) -> dict:
    """B26: the ν lines a state and the catalogue give: the band's line, and the check and preview of the script's set."""
    doc = load(SAMPLES / "corrections.json")
    k = doc["nu_constants"]
    names, moduli = k["names"], k["moduli"]
    rows = {row["id"]: row for row in doc["catalogue"]["corrections"]}
    nu = load(SAMPLES / state / "totals.json")["nu"]["nu"]

    def nonzero(row):
        parts = [names[i] if moduli[i] == 2 else f"{names[i]}={row[i]}" for i in range(len(row)) if row[i]]
        return " ".join(parts) or "zero"

    def plus(*adds):
        return [(nu[i] + sum(r["row"][i] for r in adds)) % moduli[i] for i in range(len(nu))]

    dials = " · ".join(f"O{o:02d} dial {nu[names.index(f'C5_{o}')]} of 5" for o in k["c5_orbits"])
    twists = lambda n: f"{n:,} twist" + ("" if n == 1 else "s")
    after = plus(rows[0], rows[35])
    return {"band": f"ν band: {nonzero(nu)} · {dials} · {len(k['i_a_fixed'])} fixed dots", "after": after,
            "check": f"ν after this set: {nonzero(after)} · c00×+1 c35×+1, "
                     f"{twists(len(rows[0]['letters']) + len(rows[35]['letters']))} · labels not predicted",
            "preview": f"Preview c35: {twists(len(rows[35]['letters']))} · ν after: {nonzero(plus(rows[35]))} · labels not predicted"}


def expect_nu_band(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """B26's nu-band script on nu-c0: the band shows under the rail, 35 cells wide, with the state's marks and dials;
    two marked rows, the last previewed, and one chord's check give nu-c35's ν and say the labels are not predicted."""
    if state != "nu-c0":
        return [f"the nu-band script runs on nu-c0, not {state}"]
    last = dumps[-1].get("nu") if dumps else None
    if not isinstance(last, dict):
        return ["the run wrote no ν lines"]
    want = nu_words(state)
    problems = []
    if want["after"] != load(SAMPLES / "nu-c35" / "totals.json")["nu"]["nu"]:
        problems.append("the set {c00, c35} on nu-c0 is not nu-c35's ν in the samples")
    if last["band"] != want["band"]:
        problems.append(f"the band reads {last['band']!r}, not {want['band']!r}")
    if not last["band_visible"] or not last["band_width"] > 0:
        problems.append("the band is not on screen under the rail")
    if len(last["cells"]) != 35 or sorted(last["cells"]) != list(range(35)):
        problems.append(f"the band has {len(last['cells'])} cells, not the rail's 35")
    if last["rows"] != "36" or not last["selected"].startswith("c35 · "):
        problems.append(f"the panel shows {last['rows']} rows with {last['selected']!r} selected")
    if last["set"] != "0:1 35:1":
        problems.append(f"the set is {last['set']!r}, not c00 and c35 at +1")
    if last["preview"] != want["preview"]:
        problems.append(f"the preview reads {last['preview']!r}, not {want['preview']!r}")
    if last["check"] != want["check"]:
        problems.append(f"the check reads {last['check']!r}, not {want['check']!r}")
    print(f"nu-band: {last['band']}; {last['check']}")
    return problems


CORE_MIN_TEXTURE = 128  # SA2_MIN_TEXTURE_PX, renderer/core/include/sa2_interop.h


def expect_fit(dumps: list[dict], source: dict, state: str | None, vertical: bool = True) -> list[str]:
    """The frame fits the window (owner, 26 November 2026: "看不到"): at the script's resolution the top bar, the docks,
    Global and Local lie wholly inside the window, and Local is at least 0.2 of the pair (ADR-D01's minimum). In a
    small window (fit-1280) only the width is checked: nothing lies past the right edge; the frame may scroll down."""
    frame = dumps[-1].get("frame") if dumps else None
    if not isinstance(frame, dict) or "window" not in frame:
        return ["the run wrote no frame geometry"]
    width, height = frame["window"]
    problems = []
    for key in ("top_bar", "global", "local"):
        if key not in frame:
            problems.append(f"the {key.replace('_', ' ')} is not on screen")
    for key, (x, y, w, h) in ((k, v) for k, v in frame.items() if k not in ("window", "min")):
        if x < -0.5 or x + w > width + 0.5 or (vertical and (y < -0.5 or y + h > height + 0.5)):
            problems.append(f"the {key.replace('_', ' ')} lies outside the {width:.0f}x{height:.0f} window: "
                            f"x {x:.0f} to {x + w:.0f}, y {y:.0f} to {y + h:.0f}")
    # A view under the core's smallest surface (SA2_MIN_TEXTURE_PX) is a frame that never laid out (26 November
    # 2026: a rebuild loop left Global at 86 px while the right edge still fitted).
    for key in ("global", "local"):
        if key in frame and (frame[key][2] < CORE_MIN_TEXTURE or frame[key][3] < CORE_MIN_TEXTURE):
            problems.append(f"{key.title()} is {frame[key][2]:.0f}x{frame[key][3]:.0f} px, under the core's "
                            f"{CORE_MIN_TEXTURE} px surface: the frame did not lay out")
    if "global" in frame and "local" in frame:
        share = frame["local"][2] / (frame["global"][2] + frame["local"][2])
        if share < 0.2:
            problems.append(f"Local is {share:.2f} of the pair's width, under ADR-D01's 0.2")
        print(f"fit: window {width:.0f}x{height:.0f}; Global {frame['global'][2]:.0f} px, Local {frame['local'][2]:.0f} px "
              f"({share:.2f} of the pair)")
    return problems


# The wheel zoom (owner, 26 November 2026: "我现在没法zoom in"): wheel up zooms in, one notch multiplies the zoom by
# 1.1, and the zoom stays within 0.5 to 4 times the opening's. The core multiplies the projected picture by the zoom
# (geometry.hlsl), so a larger zoom sent is a larger picture.
ZOOM_STEP, ZOOM_MIN, ZOOM_MAX = 1.1, 0.5, 4.0
ZOOM_STEPS = ["start", "wait 1500 ms", "wheel 3", "wait 800 ms", "wheel -5", "wait 800 ms", "wheel 40", "wait 800 ms"]


def expect_zoom(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """The zoom script: three notches up zoom in by 1.1 cubed, five down zoom out below that, forty up stop at four
    times the opening zoom. The drawn zoom eases to each target and is sent to the core; no wheel turns the camera."""
    if [dump["do"] for dump in dumps] != ZOOM_STEPS:
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    zooms = [dump.get("zoom") for dump in dumps]
    if any(not isinstance(zoom, dict) or "sent" not in zoom for zoom in zooms[1:]):
        return ["a step's dump has no zoom sent to the core"]
    problems = []
    base = zooms[1]["sent"]
    if zooms[1]["factor"] != 1 or zooms[1]["notches"] != 0:
        problems.append(f"the view was zoomed before any wheel ({zooms[1]})")
    for index, target in ((2, ZOOM_STEP ** 3), (4, ZOOM_STEP ** -2), (6, ZOOM_MAX)):
        previous, wheel, settled = zooms[index - 1:index + 2]
        asked = dumps[index].get("motion", {}).get("zoom", {}).get("target")
        if asked is None or abs(asked - target) > 1e-6:
            problems.append(f"after '{ZOOM_STEPS[index]}' the zoom target is {asked!r}, not {target:.4f}")
        lo, hi = sorted((previous["factor"], target))
        if not lo - 1e-6 <= wheel["factor"] <= hi + 1e-6:
            problems.append(f"after '{ZOOM_STEPS[index]}' the drawn zoom overshot its target")
        if abs(settled["factor"] - target) > 1e-6:
            problems.append(f"after '{ZOOM_STEPS[index + 1]}' the zoom factor is {settled['factor']:.4f}, not {target:.4f}")
        for zoom in (wheel, settled):
            if abs(zoom["sent"] - base * zoom["factor"]) > 1e-4:
                problems.append(f"after '{ZOOM_STEPS[index]}' the core was not sent the drawn zoom")
        if settled["notches"] != wheel["notches"]:
            problems.append("the wait added wheel notches")
    if not zooms[3]["sent"] > base:
        problems.append("wheel up zoomed out; it must zoom in (a larger zoom draws a larger picture)")
    if not zooms[5]["sent"] < zooms[3]["sent"]:
        problems.append("wheel down zoomed in; it must zoom out")
    if zooms[7]["notches"] - zooms[5]["notches"] >= 40:
        problems.append("notches past the largest zoom still changed it")
    if any(dump.get("camera_turns", 0) != 0 for dump in dumps):
        problems.append("the wheel turned the camera")
    print(f"zoom: opening {base:.3f}; +3 notches {zooms[3]['sent']:.3f}; -5 {zooms[5]['sent']:.3f}; +40 {zooms[7]['sent']:.3f}")
    return problems


def expect_name_cap(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """B9's name-cap script, by keys only: before naming, no cap is named and Local draws the default cap 0, saying so
    (owner, 26 November 2026: Local is never empty); after
    `> jumble.cap.name 30`, the panel's Tier 1 line, the Named cap count and Local's readout follow cap 30's rim row;
    after `> jumble.capmap.threshold.set N_c >= 50`, the field reads the formula and the four counts are the rim's."""
    steps = ["start", "wait 2000 ms", "chord F1", "text > jumble.cap.name 30", "chord Enter", "wait 1500 ms", "chord F1",
             "text > jumble.capmap.threshold.set N_c >= 50", "chord Enter", "wait 3000 ms"]
    if [dump["do"] for dump in dumps] != steps:
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    if state is None or not (SAMPLES / state / "rim.json").is_file():
        return ["the name-cap script needs a sample state with rim rows"]
    rows = load(SAMPLES / state / "rim.json")["rows"]
    row, problems = rows[30], []
    before, named, done = dumps[1], dumps[5], dumps[9]
    if "no cap named" not in texts(before) or "▸ Named cap · 0" not in texts(before):
        problems.append("step 1: a cap is named before any key")
    problems += local_problems(before, 1, 0, named=False)
    if row["status"] == "admissible":
        tier1 = f"cap 30 · N_c {row['N_c']} · floor {row['F_c']} · u 0 · 80 meeting caps"
    else:
        tier1 = "cap 30 · N_c undefined · floor undefined"
    if not any(text.startswith(tier1) for text in texts(named, "panel:jumble.cap_map")):
        problems.append(f"step 5: the cap map's panel does not read {tier1!r}")
    if "▸ Named cap · 1" not in texts(named):
        problems.append("step 5: Local's Named cap section does not count the named cap")
    problems += local_problems(named, 5, 30, named=True) + local_problems(done, 9, 30, named=True)
    local = texts(named, "view:Local")
    if row["status"] == "admissible" and not all(str(row[key]) in local for key in ("In_c", "Out_c", "A_c", "N_c")):
        problems.append("step 5: Local's readout does not show cap 30's rim row")
    admissible = [r for r in rows if r["status"] == "admissible"]
    members = sum(1 for r in admissible if r["N_c"] >= 50)
    blocked = sum(1 for r in rows if r["status"] == "blocked")
    pending = 600 - len(admissible) - blocked
    line = f"members {members} · non-members {len(admissible) - members} · blocked {blocked} · pending {pending}"
    if line not in texts(done, "panel:jumble.cap_map"):
        problems.append(f"step 9: the threshold counts are not {line!r}")
    if "N_c ≥ 50" not in texts(done, "panel:jumble.cap_map"):
        problems.append("step 9: the threshold field does not read N_c ≥ 50")
    if not any(text.startswith("cap 30 ·") for text in texts(done, "panel:jumble.cap_map")):
        problems.append("step 9: setting the threshold unnamed the cap")
    if overlay(done):
        problems.append("step 9: an overlay is still open")
    return problems


# B10: a seat's twelve twists at the Q3.b rhythm (JumbleHost.SeatRhythm), each check three ticks long.
SEAT_RHYTHM = [0, 1, 2, 6, 7, 12, 13, 14, 15, 20, 21, 22]
CHECK_TICKS = 3


def q3b_kept(record: dict) -> list[str]:
    """The kept list the rhythm must give at the record's cap, line for line as KeptChecks.Screen() writes it: only the
    latest release's check completes; a release folds the pending one, and a run of consecutive superseded releases
    is one row "not checked:" with the twists' glyphs."""
    checks = record["release_checks"]
    rows = dict(zip(checks["items"], checks["rows"]))
    cap = record["cap"]
    at = {tick: item for item, tick in enumerate(SEAT_RHYTHM, start=1)}
    entries, queue, pending, release = [], [], None, 0

    def signed(value):
        return "undefined" if value is None else (f"+{value}" if value > 0 else str(value))

    for tick in range(SEAT_RHYTHM[-1] + CHECK_TICKS + 2):
        for done in sorted(q for q in queue if q[2] <= tick):
            if pending and pending[0] == done[0]:
                entries.append(("row", pending))
                pending = None
        queue = [q for q in queue if q[2] > tick]
        if tick in at:
            release += 1
            if pending:
                if entries and entries[-1][0] == "fold" and entries[-1][1][-1][0] + 1 == pending[0]:
                    entries[-1][1].append(pending)
                else:
                    entries.append(("fold", [pending]))
            pending = (release, release, at[tick])
            queue.append((release, at[tick], tick + CHECK_TICKS))
    lines = []
    for kind, value in entries:
        if kind == "fold":
            lines.append("not checked: " + " ".join(f"{cap}·{item}" for _, _, item in value))
            continue
        _, tried, item = value
        row = rows[item]
        heights = [(k.replace("n_above_6", "above 6").replace("_", ""), row["delta_n_h"].get(k, 0))
                   for k in ("n_0", "n_2", "n_4", "n_6", "n_above_6")]
        changed = " ".join(f"{k} {signed(v)}" for k, v in heights if v)
        n_c = (row.get("rim_after") or {}).get("N_c")
        lines.append(" | ".join([str(tried), f"{row['letter_cap']}·{item}", "undefined" if n_c is None else str(n_c),
                                 "undefined" if row["floor"] is None else str(row["floor"]), signed(row["delta_N"]),
                                 changed or "0", signed(row["delta_u"])]))
    return lines


def expect_q3b(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """B10's q3b script: the kept list equals the rhythm's own result (completed rows in release order, each run of
    superseded releases folded as "not checked:" with its glyphs only), Local's Twist check section shows it with its
    count, and no cancel of an older release changed the screen (printed as screen_changes_on_cancel).

    C2-1: each cancel step's dump holds the whole screen read before the host's cancel (`lines_before`, with the jobs'
    clock held) and after frames passed (`lines`); any difference in any region counts. The last step is a negative
    control: the same two reads around a deliberate change (Local's open section closes), which must count."""
    if state is None or not (SAMPLES / state / "cap-150.json").is_file():
        return ["the q3b script needs a sample state with a record of cap 150"]
    if len(dumps) < 4 or dumps[-2]["do"] != "wait 1000 ms" or dumps[-1]["do"] != "screen control":
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    last, control, problems = dumps[-2], dumps[-1], []
    cancel_steps = [dump for dump in dumps if dump["do"] == "cancel older"]
    if any("lines_before" not in dump for dump in cancel_steps + [control]):
        return ["a cancel or control step's dump has no screen read before it"]
    changed = [index for index, dump in enumerate(cancel_steps) if dump["lines_before"] != dump["lines"]]
    cancels = last.get("cancels", 0)
    print(f"screen_changes_on_cancel={len(changed)} cancels={cancels} cancel_steps={len(cancel_steps)}")
    for index in changed:
        dump = cancel_steps[index]
        before, after = dump["lines_before"], dump["lines"]
        gone = [line for line in before if line not in after][:2]
        new = [line for line in after if line not in before][:2]
        problems.append(f"cancel step {index + 1} changed the screen: {gone} became {new}")
    if control["lines_before"] == control["lines"]:
        problems.append("the negative control's screen change was not counted; the two reads do not see the screen")
    if cancels < 10:
        problems.append(f"only {cancels} older releases were cancelled")
    wanted = q3b_kept(load(SAMPLES / state / "cap-150.json"))
    if last.get("kept") != wanted:
        problems.append(f"the kept list is {last.get('kept')}, the rhythm gives {wanted}")
    folded = [line for line in wanted if line.startswith("not checked:")]
    if not folded or len(folded) == len(wanted):
        problems.append("the rhythm did not give both completed and folded rows")
    if f"▾ Twist check · {len(wanted)}" not in texts(last):
        problems.append(f"Local's open Twist check section does not count {len(wanted)} rows")
    shown = texts(last, "view:Local")
    for line in folded:
        if line not in shown:
            problems.append(f"Local does not show the folded row {line!r}")
    return problems


# B11: the covers script's steps, and its lines as CapCovers.Lines() writes them from a cap record.
COVERS_STEPS = ["start", "wait 2000 ms", "chord F1", "text > jumble.cap.name 150", "chord Enter", "wait 1500 ms",
                "open cap_covers", "wait 1000 ms", "hover cover 8", "wait 1000 ms",
                "chord F1", "text > jumble.focus.height 4", "chord Enter", "wait 1000 ms",
                "chord F1", "text > jumble.focus.domain 22", "chord Enter", "wait 1000 ms",
                "chord F1", "text > jumble.focus.straddlers", "chord Enter", "wait 2500 ms"]


def covers_lines(record: dict) -> list[str]:
    """Cap covers' four lines for a cap record: the domain line, then "covered by", "covers" and "undefined" in cap-id
    order, each with how many of the 80 it answered."""
    covers = record["covers"]
    cap, threshold = covers["parameters"]["cap"], covers["parameters"]["threshold"]

    def answer(side: dict) -> str:
        return "undefined" if side.get("status") == "not_applicable" else ("yes" if side["covers"] else "no")
    rows = [(a["cap"], answer(a["d_covers_c"]), answer(a["c_covers_d"]),
             a["c_covers_d"].get("reason") or a["d_covers_c"].get("reason")) for a in covers["answers"]]

    def line(head: str, caps: list[int]) -> str:
        return head + (": " + " ".join(map(str, caps)) if caps else "")
    by = [c for c, b, _, _ in rows if b == "yes"]
    of = [c for c, _, o, _ in rows if o == "yes"]
    undefined = [(c, r) for c, b, o, r in rows if "undefined" in (b, o)]
    reasons = sorted({r or "undefined" for _, r in undefined})
    return [f"The {len(covers['domain'])} caps that meet cap {cap} · at least {threshold} aligned face pieces a side",
            line(f"covered by · {len(by)} of {sum(1 for _, b, _, _ in rows if b != 'undefined')} answered", by),
            line(f"covers · {len(of)} of {sum(1 for _, _, o, _ in rows if o != 'undefined')} answered", of),
            line(f"undefined · {len(undefined)}" + (f" ({', '.join(reasons)})" if reasons else ""), [c for c, _ in undefined])]


def expect_covers(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """B11's covers script: after naming cap 150 and opening Cap covers, the slot's lines equal the record's (domain
    line, then covered by, covers and undefined in cap-id order) and the section counts the 80 meeting caps; hovering
    cap 8 inspects it and leaves the cap map's marks unchanged; each focus row adds its predicate with the sample's
    count and focuses it, in both views."""
    if [dump["do"] for dump in dumps] != COVERS_STEPS:
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    if state is None or not (SAMPLES / state / "cap-150.json").is_file():
        return ["the covers script needs a sample state with a record of cap 150"]
    record = load(SAMPLES / state / "cap-150.json")
    style = load(SAMPLES / state / "style.json")
    problems = []
    opened, hovered = dumps[7], dumps[9]
    wanted = covers_lines(record)
    if opened.get("covers") != wanted:
        problems.append(f"step 7: Cap covers reads {opened.get('covers')}, the record gives {wanted}")
    if "▾ Cap covers · 80" not in texts(opened, "view:Local"):
        problems.append("step 7: Local's open Cap covers section does not count 80 meeting caps")
    if not any(text.startswith("The 80 caps that meet cap 150") for text in texts(opened, "view:Local")):
        problems.append("step 7: Local does not show the domain line")
    if not str(hovered.get("inspect", "")).startswith("cap 8 · "):
        problems.append(f"step 9: hovering 8 inspects {hovered.get('inspect')!r}")
    if hovered.get("map_marks") != opened.get("map_marks"):
        problems.append(f"step 9: hovering in Cap covers changed the cap map's marks ({opened.get('map_marks')} to {hovered.get('map_marks')})")
    if "|150|" not in str(opened.get("map_marks")):
        problems.append(f"step 7: the cap map does not mark cap 150 as named ({opened.get('map_marks')})")

    def at_least(k: int) -> int:
        return sum(r["count"] for r in style["rows"] if r["heights"][0] >= k)

    def in_domain(d: int) -> int:
        return sum(r["count"] for r in style["rows"] if r["domain"] == d)
    lattice = style["lattice_domain"]
    keys = [(13, f"Height ≥ 4 · {at_least(4):,}", "jumble.height"),
            (17, f"Domain {lattice} (lattice) · {in_domain(lattice):,}", "jumble.domain"),
            (21, "Straddlers · 0", "jumble.straddlers")]
    for step, key, focused in keys:
        focus = dumps[step].get("focus") or []
        if len(focus) != 2:
            problems.append(f"step {step}: {len(focus)} views take focus layers, not Global and Local")
        for view in focus:
            if key not in view or f"| focused {focused} |" not in view:
                problems.append(f"step {step}: a view's focus keys read {view!r}, not {key!r} focused")
    print(f"covers: {wanted[1].split(':')[0]}; {wanted[3].split(':')[0]}; focus {(dumps[21].get('focus') or ['none'])[0]}")
    return problems


# B12: the repair script's steps.
REPAIR_STEPS = ["start", "wait 2000 ms", "chord F1", "text > jumble.cap.name 150", "chord Enter", "wait 1500 ms",
                "open repair_list", "wait 1000 ms", "focus LocalBody_repair_list", "wait 500 ms", "chord Down", "chord Down",
                "wait 500 ms", "chord Right", "wait 800 ms", "text 2", "wait 2000 ms"]


def expect_repair(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """B12's repair script: Local's Repair list counts the record's rows and shows them in the record's order with no
    row focused on open; two arrows down focus row 2; the preview's forward step key shows "Repair 2 of 3" with the
    step's moving-set count; typing 2 in the list's context picks row 2, and only that pick is recorded."""
    if [dump["do"] for dump in dumps] != REPAIR_STEPS:
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    if state is None or not (SAMPLES / state / "cap-150.json").is_file():
        return ["the repair script needs a sample state with a record of cap 150"]
    record = load(SAMPLES / state / "cap-150.json")
    rows = record["repair_list"]["completed"]["rows"]
    problems = []
    opened, focused, stepped, picked = dumps[7], dumps[12], dumps[14], dumps[16]
    local = texts(opened, "view:Local")
    if f"▾ Repair list · {len(rows)}" not in local:
        problems.append(f"step 7: Local's open Repair list does not count {len(rows)} rows")
    first = rows[0]
    for cell in ("#", "x d", "y z", "ΔN", f"{first[0]} {first[1]}", f"{first[2]} {first[3]}", str(first[4])):
        if cell not in local:
            problems.append(f"step 7: Local does not show {cell!r}")
    repair = opened.get("repair") or {}
    if repair.get("rows") != len(rows) or repair.get("focused") != 0:
        problems.append(f"step 7: the list opened with {repair}")
    if (focused.get("repair") or {}).get("focused") != 2:
        problems.append(f"step 12: two arrows down focused {(focused.get('repair') or {}).get('focused')}")
    steps = {s["row"]: s for s in record["repair_steps"]}
    moving = steps[2]["moving_sets"][1]["count"]
    label = f"Repair 2 of 3 · moving {moving:,}"
    if (stepped.get("repair") or {}).get("step") != "Repair 2 of 3" or label not in texts(stepped, "view:Local"):
        problems.append(f"step 14: the step label is {(stepped.get('repair') or {}).get('step')!r}, not {label!r} on screen")
    if (picked.get("repair") or {}).get("picks") != [2]:
        problems.append(f"step 16: the picks are {(picked.get('repair') or {}).get('picks')}, not [2]")
    # B13: the pick staged row 2 as one sealed span with the whole-only bar, wrote one repair-pair record, and every
    # staging and focusing carried its key.
    letters = " ".join(f"{cap}·{item}" for cap, item in steps[2]["letters"])
    stage = picked.get("stage") or {}
    if stage.get("word") != f"[{letters}]" or stage.get("spans") != 1 or stage.get("records") != 1:
        problems.append(f"step 16: the stage is {stage}, not one sealed span [{letters}]")
    if stage.get("bar") != "Whole repair only" or "Whole repair only" not in texts(picked, "view:Local"):
        problems.append("step 16: the bar does not read Whole repair only")
    if f"Staged: [{letters}]" not in texts(picked, "view:Local"):
        problems.append("step 16: Local does not show the staged span")
    if stage.get("unasked") != 0 or stage.get("asked") != 3:
        problems.append(f"step 16: the boundary ledger counted {stage.get('unasked')} unasked and {stage.get('asked')} asked acts, not 0 and 3")
    if (opened.get("stage") or {}).get("spans") != 0:
        problems.append("step 7: a span was staged before any pick")
    print(f"repair: {len(rows)} rows; {label}; picks {(picked.get('repair') or {}).get('picks')}; staged {stage.get('word')} "
          f"{stage.get('bar')!r}; unasked {stage.get('unasked')}")
    return problems


TWIST_ROW_150_PLANE = "c7b19b2e60a26fcdc234491d6f51139751e06e49ddb66fdb8d0528602e445f70"


def expect_twist_row(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """B18's part for B20: the default cap 0 (blocked in wj-ia-300) has no twist row; after `> jumble.cap.name 150`
    the row of item 1 has a unit orthogonal plane, an angle in (0, pi] and moves exactly cap 150's inside set."""
    import hashlib
    import math
    steps = ["start", "wait 3000 ms", "chord F1", "text > jumble.cap.name 150", "chord Enter", "wait 2500 ms"]
    if [dump["do"] for dump in dumps] != steps:
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    if state is None or not (SAMPLES / state / "cap-150.json").is_file():
        return ["the twist-row script needs a sample state with cap 150's record"]
    rows = load(SAMPLES / state / "rim.json")["rows"]
    problems = []
    before, named = dumps[1].get("turn"), dumps[5].get("turn")
    if before is None or named is None:
        return ["a dump has no turn row"]
    if before.get("cap") != 0 or before.get("code") != 0:
        problems.append(f"step 1: the turn row reads cap {before.get('cap')} with code {before.get('code')}, not cap 0")
    # B18-08 (review call 20261126T225307Z-e6ce1514): the blocked cap's status is exactly 0, and the head is the same
    # before and after the query (the row commits nothing).
    if rows[0]["status"] != "blocked" or before.get("status") != 0:
        problems.append(f"step 1: cap 0 is {rows[0]['status']} and the turn row's status is {before.get('status')}, not blocked and 0")
    heads = [dumps[k].get("word", {}).get("digest") for k in (1, 5, len(dumps) - 1)]
    if heads[0] is None or len(set(heads)) != 1 or before.get("read_at") != named.get("read_at"):
        problems.append(f"the head moved across the query: digests {heads}, read at {before.get('read_at')} and {named.get('read_at')}")
    if named.get("cap") != 150 or named.get("code") != 0 or named.get("status") != 1:
        return problems + [f"step 5: the turn row is {named}"]
    if named.get("revision") != named.get("read_at"):
        problems.append(f"step 5: the row's revision {named.get('revision')} is not the head's {named.get('read_at')}")
    # Cap 150's twist in wj-ia-300 is a half turn in the engine's plane u = (-cos 36, -sin 18, 0, 1/2), v = (0, -1/2,
    # -cos 36, -sin 18) (single floats, little-endian, as the host writes them).
    if abs(named["angle"] - math.pi) > 1e-6 or named.get("plane_sha256") != TWIST_ROW_150_PLANE:
        problems.append(f"step 5: the angle {named['angle']} or the plane digest {named.get('plane_sha256')} is not cap 150's")
    inside = overlay_mask(load(SAMPLES / state / "cap-150.json")["masks"]["inside"], 177_120)
    packed = bytearray((len(inside) + 7) // 8)
    for piece, bit in enumerate(inside):
        if bit:
            packed[piece >> 3] |= 1 << (piece & 7)
    if named["moving"] != sum(inside) or named["moving_sha256"] != hashlib.sha256(bytes(packed)).hexdigest():
        problems.append(f"step 5: the row moves {named['moving']} pieces, not cap 150's inside set of {sum(inside)}")
    if not 0 < named["angle"] <= math.pi + 1e-6:
        problems.append(f"step 5: the angle {named['angle']} is outside (0, pi]")
    u, v = named["plane"][:4], named["plane"][4:]
    dot = lambda a, b: sum(x * y for x, y in zip(a, b))
    if abs(dot(u, u) - 1) > 1e-3 or abs(dot(v, v) - 1) > 1e-3 or abs(dot(u, v)) > 1e-3:
        problems.append(f"step 5: the plane {named['plane']} is not unit and orthogonal")
    return problems


EXPECT = {"twist-row": expect_twist_row, "smoke": expect_smoke, "help-card": expect_help_card, "bind": expect_bind, "host": expect_host, "rotate": expect_rotate, "name-cap": expect_name_cap,
          "pinned-cards": expect_pinned_cards,
          "q3b": expect_q3b, "covers": expect_covers, "repair": expect_repair, "zoom": expect_zoom,
          "focus-layer": expect_focus_layer, "seed7001": expect_seed7001, "nu-band": expect_nu_band,
          "fit-2000": expect_fit, "fit-1280": lambda dumps, source, state: expect_fit(dumps, source, state, vertical=False)}
# Scripts that run on the named state's session fixture, which build_d02_fixtures.py writes, and scripts that run in
# sample mode (ADR-D01's frame on sample data), where the strip and the rail are. The twist scripts (bind and
# two-views) run in sample mode as well, so they show that J reaches the session's router there too.
ON_FIXTURE = {"twist-row", "help-card", "pinned-cards", "bind", "host", "name-cap", "q3b", "covers", "repair", "focus-layer", "nu-band", "fit-2000", "fit-1280"}
IN_SAMPLE_MODE = {"host", "nu-band", "fit-2000", "fit-1280", "bind", "two-views"}


# The drawing track (ADR-D02 B18c onward): its scripts, and the event-log checks that read the run's frame rows.
TWO_VIEWS_STEPS = ["start", "wait 4000 ms", "chord Space", "text > jumble.cap.name 150", "chord Enter", "wait 4000 ms",
                   "chord J", "wait 700 ms", "chord J", "wait 700 ms", "chord Ctrl+Z", "wait 4000 ms"]


def expect_two_views(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """B18c: the script ran to its end, and Local is never empty (owner, 26 November 2026): before the naming the shell
    frames the default cap 0 in Local with its opening shown, and from the naming on cap 150, through the twists and
    the undo; the frame rows decide the rest (check_two_views_events). ADR-D01.41: the drags after those steps
    decide the linked rotation (check_local_drag)."""
    steps = [dump["do"] for dump in dumps]
    if steps[:len(TWO_VIEWS_STEPS)] != TWO_VIEWS_STEPS:
        return [f"the steps written were {steps}"]
    problems = local_problems(dumps[1], 1, 0, named=False)
    for index in range(5, len(TWO_VIEWS_STEPS)):
        problems += local_problems(dumps[index], index, 150, named=True)
    # The script's drags follow its base steps (the player checks that every step ran); a dump list of only the base
    # steps is the Local check alone.
    return problems + (check_local_drag(dumps) if len(dumps) > len(TWO_VIEWS_STEPS) else [])


def same(a, b) -> bool:
    return a is not None and b is not None and len(a) == len(b) and all(abs(x - y) < 1e-6 for x, y in zip(a, b))


def check_local_drag(dumps: list[dict]) -> list[str]:
    """ADR-D01.40/.41: a drag that starts in Local turns the linked rotation, so both views are sent a turned camera;
    so does a drag in Global. Unlinked, a drag in Local turns only Local and one in Global only Global; linking again
    sends Local the shared rotation. Each mark is the wait after its step, when the shell has sent the cameras."""
    steps = [dump["do"] for dump in dumps]
    marks = ["drag 160 0 in local", "wait 701 ms", "wait 702 ms", "wait 703 ms", "wait 704 ms", "wait 705 ms"]
    if any(mark not in steps for mark in marks):
        return [f"the steps written were {steps}"]
    before = dumps[steps.index(marks[0]) - 1]
    rows = [before] + [dumps[steps.index(mark)] for mark in marks[1:]]
    if any(not isinstance(row.get("rotation"), dict) for row in rows):
        return ["a step's dump has no rotation"]
    start, local, glob, unlinked_local, unlinked_global, relinked = (row["rotation"] for row in rows)
    problems = []
    if not start["linked"] or not local["linked"]:
        problems.append("Local's rotation was not linked by default")
    if not rows[1]["camera_turns"] > rows[0]["camera_turns"]:
        problems.append("the drag in Local did not turn the linked camera")
    for name, before_row, after_row in (("in Local", start, local), ("in Global", local, glob)):
        if same(before_row["sent_global"], after_row["sent_global"]):
            problems.append(f"the linked drag {name} did not turn Global")
        if same(before_row["sent_local"], after_row["sent_local"]):
            problems.append(f"the linked drag {name} did not turn Local")
        if not same(after_row["sent_local"], after_row["global"]["spin"]):
            problems.append(f"after the linked drag {name} Local was not sent the shared spin")
    if unlinked_local["linked"]:
        problems.append("the link step did not unlink Local")
    if not same(unlinked_local["sent_global"], glob["sent_global"]):
        problems.append("the unlinked drag in Local turned Global")
    if same(unlinked_local["sent_local"], glob["sent_local"]):
        problems.append("the unlinked drag in Local did not turn Local")
    if not same(unlinked_global["sent_local"], unlinked_local["sent_local"]):
        problems.append("the unlinked drag in Global turned Local")
    if same(unlinked_global["sent_global"], unlinked_local["sent_global"]):
        problems.append("the unlinked drag in Global did not turn Global")
    if not relinked["linked"] or not same(relinked["sent_local"], relinked["global"]["spin"]):
        problems.append("linking again did not return Local to the shared rotation")
    print(f"local drag: linked turns {rows[0]['camera_turns']} to {rows[1]['camera_turns']}; unlinked Local turns "
          f"{unlinked_local['local']['turns']}; {len(problems)} problems")
    return problems


def frame_rows(events: Path) -> list[dict]:
    if not events.is_file():
        return []
    rows = (json.loads(line) for line in events.read_text(encoding="utf-8").splitlines() if line.strip())
    return [row for row in rows if row.get("t") == "frame"]


def one_view_after_local(frames: list[dict]) -> list[int]:
    """Local is never empty (owner, 26 November 2026): the ids of the one-view frames after the first composite one."""
    first = next((row["frame_id"] for row in frames if row.get("panes")), None)
    return [] if first is None else [row["frame_id"] for row in frames if row["frame_id"] > first and not row.get("panes")]


def check_two_views_events(events: Path) -> list[str]:
    """B18c: composite frames of two panes that are drawn, not held, at every engine revision the run reached once
    Local was drawn (twists by the router's KeyJ and an undo), so both views show one revision; at least one revision
    change happens while Local is drawn. Local is never empty (owner, 26 November 2026): it draws the default cap before
    the naming, so no one-view frame follows the first composite one."""
    frames = frame_rows(events)
    if not frames:
        return ["the run wrote no frame rows"]
    problems = []
    if gaps := one_view_after_local(frames):
        problems.append(f"{len(gaps)} one-view frames after Local was drawn (the first: frame {gaps[0]})")
    paned = [row for row in frames if row.get("panes")]
    drawn = [row for row in paned if row.get("held") == 0]
    if not drawn or any(row.get("panes") != 2 for row in drawn):
        problems.append(f"no composite frame of two drawn panes ({len(paned)} composite rows)")
    if drawn:
        after = {row["engine_revision"] for row in frames if row["frame_id"] >= drawn[0]["frame_id"]}
        shown = {row["engine_revision"] for row in drawn}
        if after - shown:
            problems.append(f"revisions {sorted(after - shown)} were never drawn in both views")
        if len(after) < 2:
            problems.append(f"no revision changed while Local was drawn ({sorted(after)})")
    held = sum(1 for row in paned if row.get("held") == 1)
    print(f"two-views: {len(frames)} frames, {len(drawn)} drawn with two panes, {held} held, revisions "
          f"{sorted({row['engine_revision'] for row in drawn})}")
    return problems


def expect_slice(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """B20 (ADR-D02.17, CR-14): at the default cap 0 (blocked in wj-ia-300) the held preview key draws no ghost and
    names nothing; at cap 150 the held split key splits Local and its release joins it, and the held preview key draws
    the ghost of the inside layer (cap 150's inside set) and names item 1's twist only on release."""
    steps = ["start", "wait 3000 ms", "hold Down", "wait 800 ms", "release Down", "chord F1", "text > jumble.cap.name 150",
             "chord Enter", "wait 2500 ms", "hold Backslash", "wait 1500 ms", "release Backslash", "wait 1500 ms",
             "hold Down", "wait 1500 ms", "release Down", "wait 1500 ms"]
    if [dump["do"] for dump in dumps] != steps:
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    if state is None or not (SAMPLES / state / "cap-150.json").is_file():
        return ["the slice script needs a sample state with cap 150's record"]
    inside = sum(overlay_mask(load(SAMPLES / state / "cap-150.json")["masks"]["inside"], 177_120))
    slices = [dump.get("slice") for dump in dumps]
    if any(row is None for row in slices):
        return ["a dump has no slice row"]
    problems = []
    blocked = slices[3]
    if blocked["ghost"] or blocked["inside"] != 0 or not (blocked["refusal"] or "").startswith("Cap 0 is blocked"):
        problems.append(f"step 3: the blocked cap's slice is {blocked}")
    if slices[4]["named"] is not None or slices[8]["named"] is not None:
        problems.append("a twist was named at the blocked cap")
    if slices[8]["inside"] != inside or slices[8]["split"] or slices[8]["ghost"]:
        problems.append(f"step 8: the slice at cap 150 is {slices[8]}, not its inside set of {inside} unsplit")
    if not slices[10]["split"] or slices[12]["split"]:
        problems.append(f"the split read {slices[10]['split']} while held and {slices[12]['split']} after")
    held, after = slices[14], slices[16]
    if not held["ghost"] or held["named"] is not None:
        problems.append(f"step 14: while the preview key is held the slice is {held}")
    named = after["named"] or {}
    if after["ghost"] or named.get("cap") != 150 or named.get("item") != 1:
        problems.append(f"step 16: after the release the slice is {after}")
    if after["shell_min"] != 0 or not 0 < after["shell_max"] <= 3.1416:
        problems.append(f"the shell {after['shell_min']}..{after['shell_max']} is not about the pole")
    return problems


def check_slice_events(events: Path) -> list[str]:
    """B20: three drawn panes (Global and Local's two copies) while the split key is held, two after its release."""
    frames = frame_rows(events)
    drawn = [row for row in frames if row.get("panes") and row.get("held") == 0]
    three = [row["frame_id"] for row in drawn if row["panes"] == 3]
    problems = []
    if not three:
        problems.append("no drawn frame of three panes while the split key was held")
    elif not any(row["panes"] == 2 for row in drawn if row["frame_id"] > three[-1]):
        problems.append("no drawn frame of two panes after the split key's release")
    if [row["panes"] for row in drawn if row["panes"] not in (2, 3)]:
        problems.append("a drawn composite frame had neither two nor three panes")
    print(f"slice: {len(drawn)} drawn composite frames, {len(three)} with three panes")
    return problems


def expect_raw(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """B21 (ADR-D02.18, CR-14): with cap 150 named, Home turns Local's raw view on at the cut's clip (one chart), End adds
    the antipode chart labelled "antipode", PageDown widens the clip one step, the palette's clip row sets the meeting
    poles' tick, End removes the antipode chart and Home turns the raw view off; the caption is shown whenever the raw
    view is on, and nothing says "mirror"."""
    steps = ["start", "wait 3000 ms", "chord F1", "text > jumble.cap.name 150", "chord Enter", "wait 2500 ms", "chord Home",
             "wait 2000 ms", "chord End", "wait 2000 ms", "chord PageDown", "wait 1000 ms", "chord F1",
             "text > view.mode.raw.clip tick", "chord Enter", "wait 1500 ms", "chord End", "wait 1500 ms", "chord Home",
             "wait 1500 ms"]
    if [dump["do"] for dump in dumps] != steps:
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    raws = [dump.get("raw") for dump in dumps]
    if any(row is None for row in raws):
        return ["a dump has no raw row"]
    caption = "No raw view certifies no holes; N = 0 does."
    cut, tick = round(121 / 125, 6), round((3 + 5 ** 0.5) / 8, 6)
    problems = []
    want = {5: (False, False, cut), 7: (True, False, cut), 9: (True, True, cut), 11: (True, True, round(cut - 0.02, 6)),
            15: (True, True, tick), 17: (True, False, tick), 19: (False, False, tick)}
    for step, (on, antipode, h) in want.items():
        row = raws[step]
        if (row["on"], row["antipode"]) != (on, antipode) or abs(row["h"] - h) > 1e-6:
            problems.append(f"step {step}: the raw view is {row}, not on {on}, antipode {antipode}, h {h}")
        if row["charts"] != (2 if antipode else 1 if on else 0) or row["label"] != ("antipode" if antipode else ""):
            problems.append(f"step {step}: Local's charts are {row['charts']} labelled {row['label']!r}")
        if (row["caption"] == caption) != on or on and not row["line"].endswith(caption):
            problems.append(f"step {step}: the caption is {row['caption']!r} and Local's line {row['line']!r}")
        # C3/C4 finding F6: the caption as drawn on Local's border, not the model's constant.
        marks = dumps[step].get("local_marks")
        if marks is None or any(caption in mark for mark in marks) != on:
            problems.append(f"step {step}: Local's border marks are {marks!r}; the caption shows exactly while raw is on")
    if not raws[7]["line"].startswith("Raw view at cap 150"):
        problems.append(f"Local's line at step 7 reads {raws[7]['line']!r}")
    if "all 80 meeting poles inside" not in raws[15]["line"]:
        problems.append(f"at the tick Local's line reads {raws[15]['line']!r}")
    if any("mirror" in (row["line"] + row["label"]).lower() for row in raws):
        problems.append("the raw view said mirror")
    return problems


def check_raw_events(events: Path) -> list[str]:
    """B21: three drawn panes (Global, Local's chart and its antipode chart) while the antipode is on, two before and
    after."""
    frames = frame_rows(events)
    drawn = [row for row in frames if row.get("panes") and row.get("held") == 0]
    three = [row["frame_id"] for row in drawn if row["panes"] == 3]
    problems = []
    if not three:
        problems.append("no drawn frame of three panes with the antipode chart on")
    elif not any(row["panes"] == 2 for row in drawn if row["frame_id"] > three[-1]):
        problems.append("no drawn frame of two panes after the antipode chart was removed")
    if [row["panes"] for row in drawn if row["panes"] not in (2, 3)]:
        problems.append("a drawn composite frame had neither two nor three panes")
    print(f"raw: {len(drawn)} drawn composite frames, {len(three)} with three panes")
    return problems


EXPECT["two-views"] = expect_two_views


def expect_repair_keys(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """B32 findings: the repair list's key route on wj-ia-300's fixture. `jumble.repair.focus` opens Local's Repair list
    and gives it the keys with the record's rows in its order, no row focused and nothing staged; two arrows down focus
    row 2 and still stage nothing; the digit 2 stages row 2; `jumble.repair.stage 1` in the palette stages row 1. Every
    act carries its key (no unasked act)."""
    if len(dumps) < max(REPAIR_KEYS_AT.values()) + 1:
        return [f"the shell wrote {len(dumps)} screens"]
    if state is None or not (SAMPLES / state / "cap-150.json").is_file():
        return ["the repair-keys script needs a sample state with a record of cap 150"]
    rows = load(SAMPLES / state / "cap-150.json")["repair_list"]["completed"]["rows"]
    problems = []
    opened, arrowed, digit, staged = (dumps[REPAIR_KEYS_AT[k]] for k in ("opened", "arrowed", "digit", "staged"))

    def repair(dump):
        return dump.get("repair") or {}

    def stage(dump):
        return dump.get("stage") or {}

    local = texts(opened, "view:Local")
    if f"▾ Repair list · {len(rows)}" not in local:
        problems.append(f"the key route did not open Local's Repair list with {len(rows)} rows")
    at = [next((i for i, line in enumerate(local) if line == f"{row[0]} {row[1]}"), -1) for row in rows]
    if -1 in at or at != sorted(at):
        problems.append(f"Local does not show the record's rows in its order (x d at lines {at})")
    if repair(opened).get("rows") != len(rows) or repair(opened).get("focused") != 0:
        problems.append(f"the list opened with {repair(opened)}, not {len(rows)} rows and no row focused")
    if repair(opened).get("key_focus") != "LocalBody_repair_list":
        problems.append(f"the key route left the keys with {repair(opened).get('key_focus')!r}, not the list")
    if stage(opened).get("spans") or repair(opened).get("picks"):
        problems.append("something was staged when the list opened")
    if repair(arrowed).get("focused") != 2 or stage(arrowed).get("spans") or repair(arrowed).get("picks"):
        problems.append(f"two arrows down gave {repair(arrowed)} and {stage(arrowed)}, not row 2 focused and nothing staged")
    if repair(digit).get("picks") != [2] or stage(digit).get("spans") != 1:
        problems.append(f"the digit 2 gave picks {repair(digit).get('picks')} and {stage(digit).get('spans')} spans")
    if repair(staged).get("picks") != [2, 1] or stage(staged).get("spans") != 2:
        problems.append(f"jumble.repair.stage 1 gave picks {repair(staged).get('picks')} and {stage(staged).get('spans')} spans")
    if max((stage(dump).get("unasked", 0) for dump in dumps), default=0):
        problems.append("the boundary ledger counted an unasked act")
    return problems


# The screens repair-keys reads (dump i follows script step i - 1): after the focus row, the arrows, the digit, the stage row.
REPAIR_KEYS_AT = {"opened": 9, "arrowed": 12, "digit": 14, "staged": 18}
EXPECT["repair-keys"] = expect_repair_keys
ON_FIXTURE.add("repair-keys")
ON_FIXTURE.add("two-views")
# ADR-D01.17, .39 and .51: the context menu, the explore state and the non-standard colours mark.
EXPECT["explore-menu"] = expect_explore_menu
ON_FIXTURE.add("explore-menu")
EXPECT["slice"] = expect_slice
ON_FIXTURE.add("slice")
EVENT_CHECKS = {"two-views": check_two_views_events, "slice": check_slice_events}
# ADR-D02.18 (B21): the raw view and its antipode chart.
EXPECT["raw"] = expect_raw
ON_FIXTURE.add("raw")
EVENT_CHECKS["raw"] = check_raw_events


# Step C-.40 (ADR-D01.40/.41): Local on the base puzzle (600-cell-Full, Jumble off), with no D02 state. The script
# names its puzzle; the run uses the build's own base-puzzle session fixture.
BASE_LOCAL_STEPS = ["start", "wait 2500 ms", "click 0.02 0.02", "wait 800 ms", "click 0.42 0.38", "wait 800 ms",
                    "click 0.58 0.62", "wait 800 ms", "click 0.5 0.5", "wait 800 ms", "chord J", "wait 700 ms",
                    "chord Ctrl+Z", "wait 3000 ms"]
BASE_LOCAL_DEFAULT = "Local shows cell 0's turning region by default. Select a sticker to frame its cell."
STICKERS_PER_CELL = 433  # 259,800 sticker slots over 600 cells (ShellPick.Picked.Cell)


def base_local_words(cell: int, by_selection: bool) -> str:
    return (f"Local frames cell {cell}, the selected sticker's cell. Clear the selection to return to cell 0."
            if by_selection else BASE_LOCAL_DEFAULT)


def base_local_problems(dumps: list[dict]) -> list[str]:
    """Local is drawn on the base puzzle from the opening state on: its plan is Jumble off, its opening is shown and
    it frames a cell at every step. With nothing selected it frames cell 0 and its readout says that this is the
    default; at the wait after each click it frames the selected sticker's cell (ADR-D01.41), or cell 0 after a miss;
    the last click, at the opening's centre where the base view draws no sticker, clears the selection; the twist and
    the undo keep Local on cell 0. Since B17 the host picks on the base puzzle (its retained view stamps each slot's
    sticker): the corner click lands on a sticker, and the count of hits is printed."""
    steps = [dump["do"] for dump in dumps]
    if steps[:len(BASE_LOCAL_STEPS)] != BASE_LOCAL_STEPS:
        return [f"the steps written were {steps}"]
    problems, hits = [], 0
    for index in range(1, len(BASE_LOCAL_STEPS)):
        dump = dumps[index]
        plan, local = dump.get("local_plan") or {}, dump.get("local") or {}
        if plan.get("jumble") is not False:
            problems.append(f"step {index}: Local's plan is not the base puzzle's (Jumble on or missing)")
        if local.get("opening") is not True:
            problems.append(f"step {index}: Local's opening is hidden")
        if plan.get("framed", -1) < 0:
            problems.append(f"step {index}: Local frames no cell")
        if not steps[index].startswith("wait "):
            continue
        selection = (dump.get("pick") or {}).get("selection")
        slot = selection.get("slot") if selection else None
        cell = slot // STICKERS_PER_CELL if slot is not None else 0
        hits += slot is not None and steps[index - 1].startswith("click ")
        if plan.get("framed") != cell or plan.get("by_selection") is not (slot is not None):
            problems.append(f"step {index}: Local frames cell {plan.get('framed')} (by selection: "
                            f"{plan.get('by_selection')}), not the selected sticker's cell {cell}" if slot is not None
                            else f"step {index}: Local frames cell {plan.get('framed')} with nothing selected, not cell 0")
        words = base_local_words(cell, slot is not None)
        if plan.get("words") != words or words not in texts(dump, "view:Local"):
            problems.append(f"step {index}: Local's readout reads {plan.get('words')!r}, expected {words!r} on its frame")
    if (dumps[9].get("pick") or {}).get("selection") is not None:
        problems.append("the click at the opening's centre did not clear the selection")
    print(f"base-local: {hits} of 3 clicks hit; Local framed cells "
          f"{sorted({(dump.get('local_plan') or {}).get('framed', -1) for dump in dumps[1:len(BASE_LOCAL_STEPS)]})}")
    return problems


def expect_base_local(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """Step C-.40: Local on the base puzzle (base_local_problems), then the linked rotation of ADR-D01.41 by the same
    drags as two-views (check_local_drag). The frame rows decide that both views show every revision."""
    return base_local_problems(dumps) + (check_local_drag(dumps) if len(dumps) > len(BASE_LOCAL_STEPS) else [])


EXPECT["base-local"] = expect_base_local
EVENT_CHECKS["base-local"] = check_two_views_events

# B23, bare: the Hopf-ring opening. The shell sets the layout only when the opening file names the rings, and the same
# file's field of view gives the zoom, so the zoom sent is the run's sign that Global opened on the rings (a refused
# layout stops the shell before its first frame).
OPENING = ROOT / "tools" / "looklab" / "data" / "opening.json"
HOPF_STEPS = ["start", "wait 2500 ms", "wheel 2", "wait 1000 ms"]
HOPF_FRAMES = 30


def expect_hopf(dumps: list[dict], source: dict, state: str | None, projection: str = "hopfRings") -> list[str]:
    """The hopf script: Global is sent the opening file's Hopf zoom, 1/tan(fov/2), and two wheel notches scale it."""
    import math
    if [dump["do"] for dump in dumps] != HOPF_STEPS:
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    params = json.loads(OPENING.read_text(encoding="utf-8"))["params"]
    if projection == "hopfRings" and params.get("projection") != projection:
        return ["the opening file does not name the hopfRings projection"]
    zooms = [dump.get("zoom") for dump in dumps]
    if any(not isinstance(zoom, dict) or "sent" not in zoom for zoom in zooms[1:]):
        return ["a step's dump has no zoom sent to the core"]
    opening = 1 / math.tan(params["fieldOfView"] / 2)
    problems = []
    if abs(zooms[1]["sent"] - opening) > 1e-4:
        problems.append(f"Global opened at zoom {zooms[1]['sent']:.4f}, not the Hopf rings' {opening:.4f}")
    if abs(zooms[3]["sent"] - opening * ZOOM_STEP ** 2) > 1e-4:
        problems.append(f"two notches sent zoom {zooms[3]['sent']:.4f}, not {opening * ZOOM_STEP ** 2:.4f}")
    print(f"{projection}: opening zoom {zooms[1]['sent']:.4f} (1/tan(fov/2) = {opening:.4f}); +2 notches {zooms[3]['sent']:.4f}")
    return problems


def check_hopf_events(events: Path) -> list[str]:
    """B23: the shell drew at least HOPF_FRAMES frames. No cap is named, and Local is never empty (owner, 26 November
    2026): it draws the default cap, so there are composite frames and no one-view frame follows the first of them."""
    frames = frame_rows(events)
    problems = []
    if len(frames) < HOPF_FRAMES:
        problems.append(f"the run wrote {len(frames)} frame rows, fewer than {HOPF_FRAMES}")
    paned = [row for row in frames if row.get("panes")]
    if not paned:
        problems.append("no composite frame: Local never drew the default cap")
    if gaps := one_view_after_local(frames):
        problems.append(f"{len(gaps)} one-view frames after Local was drawn (the first: frame {gaps[0]})")
    print(f"hopf: {len(frames)} frames, {len(paned)} composite, revisions "
          f"{sorted({row.get('engine_revision') for row in frames})}")
    return problems


EXPECT["hopf"] = expect_hopf
ON_FIXTURE.add("hopf")
EVENT_CHECKS["hopf"] = check_hopf_events


# B22, bare: the ring sheet. The owner's opening file names the Hopf rings, so the run reads a data folder of its own
# (launch.py's --d02-data): a copy of Look Lab's samples beside an opening file that names the ring sheet. It was red
# while the shell held the sheet back: the core built no Hopf table for a sheet named first (DXGI_ERROR_DEVICE_HUNG).
def ring_sheet_data(out: Path) -> Path:
    """Writes the ring-sheet run's data folder next to the run folder and answers it."""
    import shutil
    folder = out.parent / (out.name + "-data")
    if folder.exists():
        raise ValueError(f"the run's data folder {folder} exists already")
    shutil.copytree(ROOT / "tools" / "looklab" / "data" / "samples", folder / "samples")
    opening = json.loads(OPENING.read_text(encoding="utf-8"))
    opening["params"]["projection"] = "ringSheet"
    (folder / "opening.json").write_text(json.dumps(opening, indent=2) + "\n", encoding="utf-8", newline="\n")
    return folder


SHEET_MARKS = ["ring sheet", "columns wrap", "only row neighbours are face neighbours"]
CHAIN_MARK = "projection chain (600-cell-Full)"


def chart_marks(dump: dict) -> list[str]:
    """The chart words Global's border shows in a dump, in SHEET_MARKS order, then the chain's."""
    shown = dump.get("global_marks") or []
    return [words for words in SHEET_MARKS + [CHAIN_MARK] if any(words in line for line in shown)]


def expect_sheet_chart(dumps: list[dict], pieces: list[int]) -> list[str]:
    """B22 rest, after the opening steps: the sheet's marks (its name, its wrap mark and its caption) show; a click in a
    chart selects the piece its chart shows; the chain row returns Global to the chain, the chart row applies the sheet."""
    problems = []
    if chart_marks(dumps[0]) != SHEET_MARKS:
        problems.append(f"Global's border showed {chart_marks(dumps[0])}, not the ring sheet's {SHEET_MARKS}")
    clicks = [i for i, dump in enumerate(dumps) if dump["do"].startswith("click ")]
    hits = 0
    for i in clicks:
        selection = (dumps[i + 1].get("pick") or {}).get("selection") if i + 1 < len(dumps) else None
        if selection is None:
            continue
        hits += 1
        if pieces[selection["sticker"]] != selection["piece"]:
            problems.append(f"after '{dumps[i]['do']}' sticker {selection['sticker']} names piece {selection['piece']}, "
                            f"not the model's {pieces[selection['sticker']]}")
    if not clicks or hits == 0:
        problems.append(f"no click of {len(clicks)} in a chart selected a piece")
    for row, wanted in (("view.preset.global.chain", [CHAIN_MARK]), ("view.preset.global.chart", SHEET_MARKS)):
        at = [i for i, dump in enumerate(dumps) if dump["do"] == "command " + row]
        if not at or at[0] + 1 >= len(dumps):
            problems.append(f"the script ran no '{row}' row with a step after it")
        elif chart_marks(dumps[at[0] + 1]) != wanted:
            problems.append(f"after '{row}' Global's border showed {chart_marks(dumps[at[0] + 1])}, not {wanted}")
    print(f"ring sheet: marks {chart_marks(dumps[0])}; {len(clicks)} clicks, {hits} selected a piece")
    return problems


def expect_ring_sheet(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """The ring-sheet script: the same zoom as the rings', from the data folder's opening file; then B22 rest's marks,
    chart clicks and the palette's chart rows."""
    opening = len(HOPF_STEPS)
    return (expect_hopf(dumps[:opening], source, state, projection="ringSheet")
            + expect_sheet_chart(dumps[opening - 1:], slot_pieces()))


EXPECT["ring-sheet"] = expect_ring_sheet
ON_FIXTURE.add("ring-sheet")
EVENT_CHECKS["ring-sheet"] = check_hopf_events
DATA_FOR = {"ring-sheet": ring_sheet_data}


# B17: picks. Each click's answer is in the dump after its wait: a hit selects the piece of the sticker under the point,
# which must be the model's slot_piece of the sticker's home slot; a miss clears the selection.
def slot_pieces() -> list[int]:
    """The model's slot_piece (assets/model.npz), the piece of each home slot."""
    import struct
    import zipfile
    with zipfile.ZipFile(ROOT / "assets" / "model.npz") as archive:
        data = archive.read("slot_piece.npy")
    header = struct.unpack("<H", data[8:10])[0]
    kind = data[10:10 + header].decode()
    width = 8 if "'<i8'" in kind else 4
    count = (len(data) - 10 - header) // width
    return list(struct.unpack(f"<{count}{'q' if width == 8 else 'i'}", data[10 + header:]))


def expect_pick(dumps: list[dict], source: dict, state: str | None, pieces: list[int] | None = None) -> list[str]:
    """B17: clicks answer current stickers and inspect their pieces, E follows the selected cell, and a miss clears."""
    clicks = [i for i, dump in enumerate(dumps) if dump["do"].startswith("click ")]
    if not clicks or any("pick" not in dump for dump in dumps):
        return ["the run's dumps carry no click step or no pick context"]
    pieces = pieces if pieces is not None else slot_pieces()
    problems = []
    final = dumps[-1]["pick"]
    if final["asked"] != len(clicks) or final["hits"] + final["misses"] != len(clicks) or final["stale"] != 0:
        problems.append(f"asked {final['asked']}, hits {final['hits']}, misses {final['misses']}, stale {final['stale']} "
                        f"for {len(clicks)} clicks")
    for i in clicks:
        if i + 1 >= len(dumps):
            break
        shown = dumps[i + 1]
        selection = shown["pick"]["selection"]
        if selection is not None and pieces[selection["sticker"]] != selection["piece"]:
            problems.append(f"after '{dumps[i]['do']}' sticker {selection['sticker']} names piece {selection['piece']}, "
                            f"not the model's {pieces[selection['sticker']]}")
        if state is not None or "piece_inspect" in shown:
            line = shown.get("piece_inspect", "")
            if selection is None and line:
                problems.append("the inspector kept a piece after a miss")
            if selection is not None and (not line.startswith(f"piece {selection['piece']} · ")
                                          or "domain " not in line or "height " not in line):
                problems.append(f"the inspector did not name the selected piece with its domain and height: {line!r}")
    for i, shown in enumerate(dumps[1:], 1):
        if shown["do"] != "chord E":
            continue
        previous = dumps[i - 1]
        selection = previous["pick"]["selection"]
        slot = selection["slot"] if selection is not None else None
        if slot is not None:
            cell = slot // STICKERS_PER_CELL
            if shown["named"] != cell:
                problems.append(f"E named {shown['named']}, expected the selected cell {cell}")
        else:
            reason = ("The picked piece is off the lattice and has no cell." if selection is not None
                      else "Pick a sticker at the current revision to name the Target.")
            if shown["named"] != previous["named"] or reason not in texts(shown, "frame"):
                problems.append(f"E without a selected cell must keep the Target and say {reason!r}")
    if final["hits"] == 0:
        problems.append("no click hit a sticker")
    # Checkpoint C3.9 (F2): a click selects only in the view it was made in (Global is view 1, Local view 2), and no
    # reply is dropped in a still layout.
    local_hits = 0
    for i in clicks:
        if i + 1 >= len(dumps):
            break
        selection = dumps[i + 1]["pick"]["selection"]
        want = 2 if dumps[i]["do"].endswith(" in local") else 1
        if selection is not None and selection.get("view") != want:
            problems.append(f"after '{dumps[i]['do']}' the selection names view {selection.get('view')}, not {want}")
        local_hits += want == 2 and selection is not None
    if any(dumps[i]["do"].endswith(" in local") for i in clicks) and local_hits == 0:
        problems.append("no click in Local selected a sticker in Local")
    if final.get("dropped", 0) != 0:
        problems.append(f"{final['dropped']} pick replies named another view than the clicked one")
    if final["selection"] is not None:
        problems.append("the click in the opening's corner did not clear the selection")
    print(f"pick: {len(clicks)} clicks, {final['hits']} hits, {final['misses']} misses, {final['stale']} stale")
    return problems


EXPECT["pick"] = expect_pick
ON_FIXTURE.add("pick")


def expect_sweep(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """B18 rest. B18-05: after naming cap 1 and a click that selects a sticker in another cell, Local frames the
    selection's cell (auto-framing on) and the turn row reads that cell. Without a state the grip cap's twists commit:
    J (a half-turn) flashes its moved set for jumble.half_turn's time, K sweeps with phases from 0 to 1 over
    jumble.sweep's time (its own release waits in the queue and never shortens it) and never pops to its end pose
    first. Under reduced
    motion both are static. On wj-ia-300,
    whose cap 0 is blocked, the four presses are refused and nothing sweeps or flashes."""
    import math
    dos = [dump.get("do", "") for dump in dumps]
    if (len(dumps) != 24 or dos[12] != "chord J" or dos[14] != "chord K"
            or not all(dos[k].startswith("click ") for k in (6, 8, 10))):
        return [f"the steps written were {dos}"]
    problems = []
    named = dumps[5]
    if (named.get("local_plan") or {}).get("framed") != 1 or (named.get("turn") or {}).get("cap") != 1:
        problems.append(f"step 5: Local frames {(named.get('local_plan') or {}).get('framed')} and the turn reads "
                        f"{(named.get('turn') or {}).get('cap')}, not the named cap 1")
    hits = 0
    for k in (7, 9, 11):
        selection = (dumps[k].get("pick") or {}).get("selection")
        plan, turn = dumps[k].get("local_plan") or {}, dumps[k].get("turn") or {}
        if selection is None or selection.get("slot") is None:
            # Nothing selected, or an off-lattice piece, which has no cell: Local keeps the named cap.
            if plan.get("framed") != 1 or plan.get("by_selection") is not False or turn.get("cap") != 1:
                problems.append(f"step {k}: with no selected cell Local frames {plan.get('framed')} and the turn "
                                f"reads {turn.get('cap')}, not the named cap 1")
            continue
        hits += 1
        cell = selection["slot"] // STICKERS_PER_CELL
        if cell == 1:
            problems.append(f"step {k}: the selected sticker lies in the named cell 1, so the check shows nothing")
        if plan.get("framed") != cell or plan.get("by_selection") is not True or turn.get("cap") != cell:
            problems.append(f"step {k}: Local frames {plan.get('framed')} (by selection: {plan.get('by_selection')}) and "
                            f"the turn reads {turn.get('cap')}, not the selected sticker's cell {cell}")
    if hits == 0 and state is not None:
        problems.append("no click selected a sticker of a lattice piece")
    # SF5 (27 Nov 2026): from the last click on, through the twists, the Global opening keeps its size (a readout that
    # changes at a twist must not change the border's height, which resized the core's surface at each twist).
    openings = [tuple(dump.get("global_opening") or ()) for dump in dumps[11:]]
    if any(len(size) != 2 or min(size) <= 0 for size in openings) or len(set(openings)) != 1:
        problems.append(f"the Global opening was {openings} from step 11 on, not one size")
    sweeps = [dump.get("sweep") for dump in dumps[1:]]
    if any(sweep is None for sweep in sweeps):
        return problems + ["a dump has no sweep row"]
    if [sweep["reduced"] for sweep in sweeps[:16]] != [False] * 16 or sweeps[-1]["reduced"] is not True:
        problems.append(f"reduced motion read {[sweep['reduced'] for sweep in sweeps]}, not off then on")
    final = sweeps[-1]
    motions = final["motions"]
    if state is not None:
        if final["refused"] != 4 or motions or final["running"] is not None:
            problems.append(f"on {state} the presses were refused {final['refused']} times and the motions were "
                            f"{[m['kind'] for m in motions]}, not 4 refusals and none")
        return problems
    # B22 rest (ADR-D02.10): with a chart in Global (the owner's opening names the Hopf rings) and the slice in Local at
    # another cap than the grip cap, K is swept by no view: it is static, and Global outlines its moved set once, for
    # jumble.half_turn's time. On the chain K sweeps.
    chart = final.get("chart") is True
    kinds = ["flash", "static" if chart else "sweep", "static", "static"]
    if final["refused"] != 0 or [m["kind"] for m in motions] != kinds:
        return problems + [f"refused {final['refused']}, motions {[m['kind'] for m in motions]}, not {', '.join(kinds)}"]
    flash, sweep = motions[0], motions[1]
    if abs(flash["angle"] - math.pi) > 1e-5 or flash["moving"] <= 0 or flash["phases"]:
        problems.append(f"the flash turns {flash['angle']} with {flash['moving']} moving and phases {flash['phases']}")
    if flash["duration_ms"] != 320 or flash["ended"] != "done" or not 320 <= flash["shown_ms"] < 900:
        problems.append(f"the flash lasts {flash['duration_ms']} ms, ended {flash['ended']!r} after {flash['shown_ms']} ms")
    if chart:
        if (final.get("outlines_shown"), final.get("outlines_ended")) != (1, 1) or not 320 <= final.get("outline_ms", 0) < 900:
            problems.append(f"under the chart Global outlined {final.get('outlines_shown')} moved sets ({final.get('outlines_ended')} "
                            f"ended, the last after {final.get('outline_ms')} ms), not K's once for 320 ms")
        for static in motions[1:]:
            if static["phases"] or static["shown_ms"] != 0 or static["ended"] != "static":
                problems.append(f"a twist swept by no view moved: {static}")
        return problems
    phases, ms = sweep["phases"], sweep["ms"]
    sweep_ms = load(DESIGN / "system.json")["motion"]["durations"]["twist"]
    if abs(sweep["angle"] - math.pi) < 1e-3 or sweep["moving"] <= 0 or sweep["duration_ms"] != sweep_ms:
        problems.append(f"the sweep turns {sweep['angle']} with {sweep['moving']} moving over {sweep['duration_ms']} ms")
    if (sweep["ended"] != "done" or len(phases) < 3 or phases[0] > 0.05 or phases[-1] != 1
            or any(b < a for a, b in zip(phases, phases[1:])) or any(not 0 <= p <= 1 for p in phases)
            or not 0 < sum(0 < p < 1 for p in phases) or ms[-1] < sweep_ms or ms[0] != 0):
        problems.append(f"the sweep ended {sweep['ended']!r} with phases {phases} at {ms} ms")
    if sweep["popped"]:
        problems.append("a frame drew the sweep's revision at its end pose before phase 0 was set")
    for static in motions[2:]:
        if static["phases"] or static["shown_ms"] != 0 or static["ended"] != "static":
            problems.append(f"a twist under reduced motion moved: {static}")
    revisions = [m["revision"] for m in motions]
    if revisions != sorted(revisions) or len(set(revisions)) != 4:
        problems.append(f"the motions' revisions {revisions} are not four increasing ones")
    # SF (27 Nov 2026): a twist, and the change of binding it brings (the sample state unbinds), rebuilds no chrome
    # frame: from the dump before the first twist to the reduced-motion command, the frame's rebuild count stays.
    dos = [dump["do"] for dump in dumps]
    if "chord J" in dos and "chord Space" in dos[dos.index("chord J"):]:
        first = dos.index("chord J")
        rebuilds = [dump.get("rebuilds") for dump in dumps[first - 1:first + dos[first:].index("chord Space")]]
        if None in rebuilds or len(set(rebuilds)) != 1:
            problems.append(f"the chrome frame rebuilt across the twists: rebuild counts {rebuilds}")
    else:
        problems.append("the script has no twist before its reduced-motion command")
    return problems


EXPECT["sweep"] = expect_sweep


MOTION_STEPS = ["start", "wait 3000 ms", "command jumble.cap.name 0", "drag 4.25 0", "chord K", "chord L", "chord K",
                "wait 1500 ms", "wheel 1", "wheel 1", "wait 800 ms", "chord Shift+K", "chord Shift+L", "chord Shift+K",
                "wait 1500 ms", "command motion.reduced.set on", "chord K", "chord L", "chord K", "wait 800 ms",
                "command motion.reduced.set off"]


def expect_motion(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """Three FIFO turns on the solved session, an additive eased zoom and equal states with or without motion."""
    import math
    if state is not None:
        return ["the motion script uses the solved Jumble session without --state"]
    steps = [d.get("do") for d in dumps]
    if steps != MOTION_STEPS:
        return [f"the steps written were {steps}"]
    motions = [d.get("motion") for d in dumps]
    if any(not isinstance(m, dict) for m in motions):
        return ["a dump has no motion block"]
    base, after_drag, animated, rewind, reduced, still = (motions[i] for i in (2, 3, 7, 14, 15, 19))
    revision, digest = base.get("state_revision", -1), base.get("state_digest")
    if revision < 0 or not digest:
        return ["the solved session's head was not drawn"]
    problems = []
    if (after_drag.get("state_revision"), after_drag.get("state_digest")) != (revision, digest):
        problems.append("the camera drag changed the session")
    camera = [dumps[i].get("camera", []) for i in (2, 3)]
    if any(len(matrix) != 16 for matrix in camera) or not math.isclose(
            math.atan2(camera[1][12], camera[1][0]) - math.atan2(camera[0][12], camera[0][0]), 4.25 * .004, abs_tol=1e-7):
        problems.append("the camera drag did not include all pointer increments")
    if any(m.get("reduced") is not (15 <= i < 20) for i, m in enumerate(motions)):
        problems.append("reduced motion did not start off, turn on, and finish off")
    if [m.get("queued_events") for m in motions[4:7]] != [1, 3, 5] or motions[5].get("queue_length", 0) < 2:
        problems.append("the quick presses and releases did not enter the twist queue")
    if animated.get("queue_length") != 0 or animated.get("state_revision") != revision + 3:
        problems.append("the burst did not finish at three new revisions with an empty queue")
    sweeps = animated.get("turns", [])
    expected = list(range(revision + 1, revision + 4))
    if [m.get("revision") for m in sweeps] != expected:
        problems.append("the burst did not record all three committed revisions in order")
    else:
        durations = [m.get("duration_ms", 0) for m in sweeps]
        if (any(m.get("kind") != "sweep" or m.get("row_id") != "twist.queued" or m.get("ended") != "done" for m in sweeps)
                or not durations[0] == 110 or not durations[0] > durations[1] > durations[2] >= 1000 / 60):
            problems.append(f"the queued sweeps did not speed up within the burst ({durations} ms)")
        if any(m.get("armed_revision") != r - 1 or m.get("first_frame_revision") != r or m.get("popped") is not False
               for m, r in zip(sweeps, expected)):
            problems.append("a sweep's first frame did not draw phase zero of its committed revision")

    zooms = [motions[i].get("zoom", {}) for i in (8, 9, 10)]
    first, second, settled = zooms
    if (any(z.get("row_id") != "view.transition" or z.get("duration_ms") != 260 for z in zooms)
            or first.get("target") != ZOOM_STEP or second.get("target") != ZOOM_STEP ** 2
            or first.get("from") != 1 or not 1 < first.get("drawn", 0) < ZOOM_STEP
            or second.get("from") != first.get("drawn")
            or not first["drawn"] <= second.get("drawn", 0) <= second["target"]
            or settled.get("drawn") != second["target"]):
        problems.append("the second zoom notch did not continue from the drawn zoom to the combined target")

    if rewind.get("state_revision") != revision + 6 or rewind.get("state_digest") != digest:
        problems.append("the inverse keys did not return to the solved start")
    queue_at_toggle = reduced.get("queued_events")
    if any(m.get("queue_length") != 0 or m.get("queued_events") != queue_at_toggle for m in motions[15:]):
        problems.append("a key queued under reduced motion")
    static = still.get("turns", [])[-3:]
    if ([m.get("revision") for m in static] != list(range(revision + 7, revision + 10))
            or any(m.get("kind") != "static" or m.get("duration_ms") != 0 or m.get("first_frame_revision") is not None
                   or m.get("chart_outline") is not None for m in static)
            or any(len(m.get("turns", [])) != 7 + i for i, m in enumerate(motions[16:19]))):
        problems.append("the reduced-motion keys did not commit immediately as three static turns")
    if (still.get("state_revision") != revision + 9 or not animated.get("state_digest")
            or animated.get("state_digest") == digest or still.get("state_digest") != animated.get("state_digest")):
        problems.append("the full and reduced motion runs ended at different states")
    return problems


EXPECT["motion"] = expect_motion


# The owner's bug report of 26 November 2026: jumble.cap.name without a number opens the cap prompt, every cap in
# cap-id order with nothing highlighted (the human-solve boundary: the program never ranks or pre-selects a cap).
CAP_PROMPT_STEPS = ["start", "wait 2000 ms", "chord F1", "text > jumble.cap.name", "chord Enter", "text 15", "chord Down",
                    "chord Down", "chord Enter", "wait 1500 ms", "chord F1", "text > jumble.cap.name", "chord Enter",
                    "click CapPromptRow_7", "wait 1000 ms", "chord F1", "text > jumble.cap.name", "chord Enter",
                    "chord Escape", "wait 2000 ms"]


def prompt_label(row: dict) -> str:
    """A cap prompt row's words for a rim row, as CapPrompt.Label writes them from the cap map's classes."""
    if row["status"] == "blocked":
        return f"cap {row['cap']} · blocked · N_c undefined"
    word = "closed" if row["N_c"] == 0 else "at floor" if row.get("at_floor") else "above floor"
    return f"cap {row['cap']} · {word} · N_c {row['N_c']}"


def expect_cap_prompt(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """The cap prompt: opened by the row without a number, caps 0 to 11 shown first in id order with their words and
    nothing highlighted; 15 narrows to 15 and 150 to 159; two Downs highlight 150 and Enter names it; a click on cap 7's
    row names cap 7; Escape closes it with cap 7 still named. No step refuses the row."""
    if [dump["do"] for dump in dumps] != CAP_PROMPT_STEPS:
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    if state is None or not (SAMPLES / state / "rim.json").is_file():
        return ["the cap-prompt script needs a sample state with rim rows"]
    rows = {row["cap"]: row for row in load(SAMPLES / state / "rim.json")["rows"]}
    problems = []

    def prompt(index: int) -> dict:
        return dumps[index].get("cap_prompt", {})
    opened = prompt(4)
    if not opened.get("open"):
        problems.append("step 4: the row without a number did not open the cap prompt")
    if opened.get("rows") != 600 or opened.get("window") != list(range(12)):
        problems.append(f"step 4: the prompt shows {opened.get('rows')} caps, first {opened.get('window')}, not 600 from cap 0 in id order")
    if opened.get("highlighted") != -1:
        problems.append(f"step 4: cap {opened.get('highlighted')} is highlighted when the prompt opens")
    shown = [text for text in overlay(dumps[4]) if text.startswith("cap ")]
    wanted = [prompt_label(rows[cap]) for cap in range(12)]
    if shown != wanted:
        problems.append(f"step 4: the prompt's rows read {shown}, expected {wanted}")
    if any("it needs a cap number" in text for text in texts(dumps[4])):
        problems.append("step 4: the row is still refused without a number")
    narrowed = prompt(5)
    if narrowed.get("window") != [15] + list(range(150, 160)) or narrowed.get("highlighted") != -1:
        problems.append(f"step 5: 15 shows {narrowed.get('window')} with {narrowed.get('highlighted')} highlighted")
    for index, cap in ((6, 15), (7, 150)):
        if prompt(index).get("highlighted") != cap:
            problems.append(f"step {index}: Down highlights {prompt(index).get('highlighted')}, expected cap {cap}")
    for index, cap in ((8, 150), (13, 7), (18, 7)):
        if prompt(index).get("open") or dumps[index].get("named") != cap:
            problems.append(f"step {index}: the prompt is {'open' if prompt(index).get('open') else 'closed'} with cap "
                            f"{dumps[index].get('named')} named, expected it closed with cap {cap} named")
    if not any(text.startswith("cap 150 ·") for text in texts(dumps[8], "panel:jumble.cap_map")):
        problems.append("step 8: the cap map's panel does not read cap 150's Tier 1 line")
    again = prompt(12)
    if not again.get("open") or again.get("highlighted") != -1 or "Name a cap · cap 150 is named now" not in overlay(dumps[12]):
        problems.append("step 12: the prompt did not open again with nothing highlighted and cap 150 named")
    if not prompt(17).get("open"):
        problems.append("step 17: the prompt did not open the third time")
    if overlay(dumps[19]):
        problems.append("step 19: an overlay is still open")
    return problems


EXPECT["cap-prompt"] = expect_cap_prompt
ON_FIXTURE.add("cap-prompt")

# The owner's bug report of 26 November 2026: a cap map cell says on hover that a click names it, and a click that
# names nothing says why in the status line.
CAP_HOVER_STEPS = ["start", "wait 2000 ms", "cap hover 1", "cap click 1", "cap click 1", "cap hover 0", "cap click 0",
                   "wait 2000 ms"]
HINT = "click to name this cap"


def expect_cap_hover(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """Hovering cap 1 raises it with a pointing hand and the hint over its facts, naming nothing; a click names it; a
    second click says in the status line that it is already named; the blocked cap 0 is hovered with the hint and named
    by a click."""
    if [dump["do"] for dump in dumps] != CAP_HOVER_STEPS:
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    if state is None or not (SAMPLES / state / "rim.json").is_file():
        return ["the cap-hover script needs a sample state with rim rows"]
    problems = []

    def cell(index: int) -> dict:
        return dumps[index].get("cap_map", {})
    for index, cap in ((2, 1), (5, 0)):
        shown = cell(index)
        if shown.get("hovered") != cap or shown.get("cursor") != "PointingHand":
            problems.append(f"step {index}: cap {shown.get('hovered')} is hovered with the {shown.get('cursor')} pointer, "
                            f"expected cap {cap} with a pointing hand")
        if not shown.get("tooltip", "").startswith(f"{HINT}\ncap {cap} · "):
            problems.append(f"step {index}: cap {cap}'s tooltip reads {shown.get('tooltip')!r}")
    if dumps[2].get("named") != -1 or "no cap named" not in texts(dumps[2], "panel:jumble.cap_map"):
        problems.append("step 2: hovering named a cap")
    for index, cap in ((3, 1), (4, 1), (6, 0)):
        if dumps[index].get("named") != cap:
            problems.append(f"step {index}: cap {dumps[index].get('named')} is named, expected cap {cap}")
    refused = "cap 1 is already named; F1 › jumble.cap.unname un-names it"
    if refused not in texts(dumps[4], "frame"):
        problems.append(f"step 4: the status line does not say {refused!r}")
    if not cell(4).get("tooltip", "").startswith(refused + "\ncap 1 · "):
        problems.append(f"step 4: the named cap's tooltip reads {cell(4).get('tooltip')!r}")
    if refused in texts(dumps[3], "frame"):
        problems.append("step 3: the first click was refused")
    return problems


EXPECT["cap-hover"] = expect_cap_hover
ON_FIXTURE.add("cap-hover")

# The script player's command step runs a row as the palette does (ShellChrome.Run). In sample mode,
# library.branch.toggle with the argument "Built-in" opens that branch of the Library: before the fix it threw inside
# SampleMount and nothing opened. No error reaches the error lamp.
LIBRARY_BRANCH_STEPS = ["start", "wait 2000 ms", "command macro.base.open", "command library.branch.toggle Built-in",
                        "wait 3000 ms"]
# The Library's shipped tools, the records of its Built-in branch (Chrome/Model/Library.cs).
BUILT_IN = ["Residuals", "Protection check", "Setup search", "Macro facts", "Compare macros", "Cycles", "Structure"]


def expect_library_branch(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """macro.base.open shows the Library in the left dock with Built-in folded; library.branch.toggle Built-in opens
    that branch and the Library lists the shipped tools; the frame carries the error lamp, which counts no failure at
    any step, and it rebuilds at most once a command (a frame that rebuilds on every error never lays its views out)."""
    if [dump["do"] for dump in dumps] != LIBRARY_BRANCH_STEPS:
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    problems = []
    for index, opened in ((2, False), (3, True), (4, True)):
        library = dumps[index].get("library", {})
        if library.get("left_dock") != "library" or library.get("panel") is not True:
            problems.append(f"step {index}: the left dock shows {library.get('left_dock')!r}, not the Library panel")
        if ("Built-in" in library.get("open", [])) != opened:
            problems.append(f"step {index}: Built-in is {'folded' if opened else 'open'}, expected it "
                            f"{'open' if opened else 'folded'}")
        listed = [name for name in BUILT_IN if name in library.get("rows", [])]
        if listed != (BUILT_IN if opened else []):
            problems.append(f"step {index}: the Library lists the shipped tools {listed}")
    for dump in dumps:
        errors = dump.get("errors", {})
        if errors.get("lamp") is not True:
            problems.append(f"step {dump['step']}: the frame carries no error lamp")
        if errors.get("count") != 0 or errors.get("failures") != 0:
            problems.append(f"step {dump['step']}: the error lamp counts {errors.get('count')} "
                            f"({errors.get('failures')} caught), the last: {errors.get('last')!r}")
    rebuilds = [dump.get("rebuilds") for dump in dumps]
    if None in rebuilds or rebuilds[-1] - rebuilds[1] > 2:
        problems.append(f"the frame rebuilt {rebuilds} times by step")
    return problems


EXPECT["library-branch"] = expect_library_branch
IN_SAMPLE_MODE.add("library-branch")


# B28: the research pane in Analyse on the synthetic sample. The first failing claim group opens at load, so one
# workspace tab away the owner sees the Fail row expanded to its first failing claim; object.inspect on that group folds
# it and opens it again through the palette's path. No error reaches the error lamp.
RESEARCH_FAIL = "research.claim/16/macro:sample/stage-16/macro-i7303.json#"
RESEARCH_PANE_STEPS = ["start", "wait 2000 ms", "command workspace.set layout:analyse", "wait 1500 ms",
                       f"command object.inspect {RESEARCH_FAIL}", f"command object.inspect {RESEARCH_FAIL}",
                       "wait 3000 ms"]
RESEARCH_FAIL_LINES = ["first claim that does not hold: cycle.closes", "code: KN6:evidence.certificate_rejected"]


def research_chip(index: int, research: dict) -> list[str]:
    """C5-9: the sample chip's words name Research among the parts that hold sample data."""
    chip = research.get("chip", "")
    if not chip.startswith("Sample mode:") or "Research hold sample data" not in chip:
        return [f"step {index}: the sample chip says {chip!r}"]
    return []


def expect_research_pane(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """Solve shows no research pane; Analyse shows it with the Fail row open on its first failing claim and the code;
    the first object.inspect folds the row and the second opens it again; the head says the evidence is synthetic."""
    if [dump["do"] for dump in dumps] != RESEARCH_PANE_STEPS:
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    problems = []
    for index, (shown, opened) in enumerate([(None, True), (False, True), (None, True), (True, True), (True, False),
                                             (True, True), (True, True)]):
        research = dumps[index].get("research", {})
        if research.get("evidence") != "synthetic" or "evidence: synthetic" not in research.get("head", ""):
            problems.append(f"step {index}: the pane's evidence is {research.get('evidence')!r}, head "
                            f"{research.get('head')!r}")
        problems += research_chip(index, research)
        if shown is not None and research.get("pane") is not shown:
            problems.append(f"step {index}: the research pane is {'hidden' if shown else 'shown'}")
        if (research.get("open") == RESEARCH_FAIL) != opened:
            problems.append(f"step {index}: the open object is {research.get('open')!r}")
        lines = [line for line in research.get("lines", []) if line.get("ref") == RESEARCH_FAIL]
        rows = [line for line in lines if line.get("kind") == "Row"]
        if len(rows) != 1 or rows[0].get("verdict") != "fails within on one instance":
            problems.append(f"step {index}: the Fail row reads {[row.get('verdict') for row in rows]}")
        details = [line["text"] for line in lines if line.get("kind") == "Detail"]
        if (details[:2] == RESEARCH_FAIL_LINES) != opened:
            problems.append(f"step {index}: the Fail row's lines are {details[:2]}")
    for dump in dumps:
        errors = dump.get("errors", {})
        if errors.get("lamp") is not True:
            problems.append(f"step {dump['step']}: the frame carries no error lamp")
        if errors.get("count") != 0 or errors.get("failures") != 0:
            problems.append(f"step {dump['step']}: the error lamp counts {errors.get('count')} "
                            f"({errors.get('failures')} caught), the last: {errors.get('last')!r}")
    return problems


EXPECT["research-pane"] = expect_research_pane
IN_SAMPLE_MODE.add("research-pane")


DOCUMENT_STEPS = ["start", "wait 2000 ms", "command file.document.open census.6cs", "command file.document.compile",
                  "command file.document.new script", "click SessionDocumentTab", "command file.document.close",
                  "wait 3000 ms"]


def expect_documents(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    if [dump["do"] for dump in dumps] != DOCUMENT_STEPS:
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    problems = []
    initial = dumps[0].get("documents", {})
    tabs = initial.get("tabs", [])
    signatures = [(tab.get("id"), tab.get("kind"), tab.get("ticks"), tab.get("lamp"), tab.get("binding"),
                   tab.get("readOnly"), tab.get("recording")) for tab in tabs]
    if signatures != [
        ("session", "Session", 35, None, None, False, True),
        ("mid.sample-log", "Log", 35, None, "saved", False, False),
        ("endgame.sample-log", "Log", 35, None, "saved", False, False),
        ("bad-hash.sample-log", "Log", 0, "Problems", None, True, False),
        ("census.6cs", "Script", 0, "NotCompiled", None, False, False),
        ("sweep.6cs", "Script", 0, "NotCompiled", None, False, False),
    ]:
        problems.append(f"opening document signatures are {signatures!r}")
    if "final hash does not match" not in initial.get("problems", []):
        problems.append(f"opening document problems are {initial.get('problems')!r}")
    # The session is never in the overflow list; at 1280x720 the six sample tabs may all fit.
    if not isinstance(initial.get("overflow"), int) or not 0 <= initial["overflow"] < len(tabs):
        problems.append(f"at 1280x720, document overflow is {initial.get('overflow')!r}")

    opened_ids = [tab.get("id") for tab in tabs]
    for i, active in enumerate(("session", "session", "census.6cs", "census.6cs", "untitled-1", "session", "session", "session")):
        docs = dumps[i].get("documents", {})
        if docs.get("row") is not True or docs.get("active") != active:
            problems.append(f"step {i}: row is {docs.get('row')!r} and active document is {docs.get('active')!r}")
        current = {tab.get("id"): tab for tab in docs.get("tabs", [])}
        expected_ids = opened_ids + (["untitled-1"] if i >= 4 else [])
        if list(current) != expected_ids:
            problems.append(f"step {i}: open documents are {list(current)!r}")
        census_lamp = current.get("census.6cs", {}).get("lamp")
        if census_lamp not in (("NotCompiled",) if i < 3 else ("Clean", "Problems")):
            problems.append(f"step {i}: census.6cs lamp is {census_lamp!r}")
        if i >= 4 and current.get("untitled-1", {}).get("kind") != "Script":
            problems.append(f"step {i}: the untitled document's kind is {current.get('untitled-1', {}).get('kind')!r}")
        errors = dumps[i].get("errors", {})
        if errors.get("lamp") is not True or errors.get("count") != 0 or errors.get("failures") != 0:
            problems.append(f"step {i}: error lamp is {errors!r}")
    if ["frame", "Close document: the session stays open"] not in dumps[6].get("lines", []):
        problems.append("the frame did not give the session close refusal")
    return problems


EXPECT["documents"] = expect_documents
IN_SAMPLE_MODE.add("documents")


TIMER_STEPS = ["start", "wait 2000 ms", "click TimerKey", "wait 2500 ms", "click TimerKey", "command session.practice",
               "click SessionCell", "command session.completion O06", "wait 2000 ms"]


def expect_timer(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    steps = [dump["do"] for dump in dumps]
    if steps != TIMER_STEPS:
        return [f"the steps written were {steps}"]
    problems = []
    sessions = [dump.get("session", {}) for dump in dumps]
    opening = sessions[0]
    timer = opening.get("timer")
    if timer != {"cell": True, "running": False, "text": "0:00", "row": "session.timer.start"}:
        problems.append(f"opening timer is {timer!r}")
    cell = opening.get("cell", {})
    if cell.get("present") is not True or cell.get("expanded") is not False or len(cell.get("lamps", [])) != 3:
        problems.append(f"opening session cell is {cell!r}")
    completed = opening.get("completions", [])
    if (len(completed) != 11 or completed[0].get("name") != "O06"
            or any(set(entry) != {"name", "revision", "moment", "sample"} or entry["sample"] is not True
                   or entry["moment"] is not (i == 0) for i, entry in enumerate(completed))):
        problems.append(f"opening completions are {completed!r}")
    if opening.get("historyRows") != 11 or opening.get("momentSlot") is not True:
        problems.append(f"opening history has {opening.get('historyRows')!r} completion rows "
                        f"and moment slot {opening.get('momentSlot')!r}")
    if opening.get("practice") != {"on": False, "global": 0, "local": 0}:
        problems.append(f"opening practice is {opening.get('practice')!r}")

    for i in (2, 3):
        running = sessions[i].get("timer", {})
        if running.get("running") is not True or running.get("row") != "session.timer.stop":
            problems.append(f"step {i}: running timer is {running!r}")
    paused = sessions[4].get("timer", {})
    elapsed = paused.get("text")
    if (paused.get("running") is not False or paused.get("row") != "session.timer.start"
            or not isinstance(elapsed, str) or not re.fullmatch(r"(?:\d+:)?\d+:[0-5]\d", elapsed)
            or elapsed in ("0:00", "0:01")):
        problems.append(f"the paused timer should show at least 0:02, got {paused!r}")
    if sessions[5].get("practice") != {"on": True, "global": 1, "local": 1}:
        problems.append(f"after session.practice, practice is {sessions[5].get('practice')!r}")
    cell = sessions[6].get("cell", {})
    facts = cell.get("facts", [])
    names = ["entries", "moves", "completions", "log head", "revision", "state hash", "session", "model", "build"]
    if cell.get("expanded") is not True or [fact.get("name") for fact in facts] != names or facts[5].get("waiting") is not True:
        problems.append(f"the expanded session cell is {cell!r}")
    if completed:
        first = completed[0]
        line = f"{first.get('name')} complete · exact check of every label · revision {first.get('revision')}"
        lines = dumps[7].get("lines", [])
        if ["frame", line] not in lines or ["panel:history", line + " · sample"] not in lines:
            problems.append("reading the first completion did not give its line and show its History row")
    for i, dump in enumerate(dumps):
        errors = dump.get("errors", {})
        if errors.get("lamp") is not True or errors.get("count") != 0 or errors.get("failures") != 0:
            problems.append(f"step {i}: error lamp is {errors!r}")
    return problems


EXPECT["timer"] = expect_timer
IN_SAMPLE_MODE.add("timer")
ON_FIXTURE.add("timer")


# ADR-D01.70: the effect signature in the preview view and on History's entries, drawn with the Library rows' marks.
# ScreenText reads a signature by its tooltip, so the screen's words carry it.
def expect_signature(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """ADR-D01.70 in sample mode: once the sample preview is staged, the preview view (in the frame) shows that
    preview's signature (its sample effects on their rail orbits, unverified) and the sample History shows one on its
    entry rows; no signature reads as certified; the error lamp counts no failure."""
    problems = []
    lines = dumps[-1].get("lines", []) if dumps else []
    signatures = [(group, text) for group, text in lines if text.startswith("Effect signature")]
    preview = [text for group, text in signatures if group == "frame"]
    history = [text for group, text in signatures if group.startswith("panel:history")]
    if not any(text.startswith("Effect signature, unverified: moves ") for text in preview):
        problems.append(f"the preview view shows no measured signature: {preview!r}")
    if not history:
        problems.append("History's entries show no signature")
    if any("certified" in text and "unverified" not in text for _, text in signatures):
        problems.append("a sample signature reads as certified")
    for i, dump in enumerate(dumps):
        errors = dump.get("errors", {})
        if errors.get("lamp") is not True or errors.get("count") != 0 or errors.get("failures") != 0:
            problems.append(f"step {i}: error lamp is {errors!r}")
    return problems


EXPECT["signature"] = expect_signature
IN_SAMPLE_MODE.add("signature")


# ADR-D01.68: the setup slot under the Target cell. The sample's setup search is demo data (SetupSearch.Sample), so its
# rows for the mid demo state are pinned here; the console tests (D02PortSetupTests.cs) check the rule itself.
SETUP_STEPS = ["start", "wait 2000 ms", "command setup.search", "wait 1000 ms", "command operation.phase.append_twist",
               "command setup.row.use 1", "wait 1000 ms", "command operation.preview setup:1", "wait 2500 ms"]
SETUP_DECLARATION = "Boundary class: setup_search_named_target · Principle record: sample (demo data)"
SETUP_ROWS = ["1  len 2  c517.3+ c292.0+", "2  len 3  c142.4- c517.3+ c292.0+", "3  len 3  c292.0+ c142.4+ c292.0+",
              "4  len 3  c517.3+ c292.0- c517.3+"]
SETUP_RESULT = ["for p0334, the Target", "Length 2 is minimal within the guarded alphabet.",
                "Guarded alphabet: 3 of 6 caps; excluded c217 (moves Buffer A), c442 (moves Buffer A), c367 (moves "
                "Buffer B). Length bound 3; 126 of 126 words tried; order: length, then menu order (demo data)."]


def expect_setup(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """ADR-D01.68 in sample mode: before a run the slot says not run and shows the tool's BoundaryClass and
    PrincipleRecord; setup.search for the named Target lists the rows in their neutral order with their lengths, the
    minimal line and the coverage; row 1 fills Prepare and Cleanup around the twist in Core; row 1 alone is staged as
    the preview; the error lamp counts no failure."""
    if [dump["do"] for dump in dumps] != SETUP_STEPS:
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    problems = []
    words = [[text for _, text in dump.get("lines", [])] for dump in dumps]
    for i in (0, 1):
        if "not run" not in words[i] or SETUP_DECLARATION not in words[i]:
            problems.append(f"step {i}: the setup slot does not say not run with its declaration")
        if any(text in words[i] for text in SETUP_ROWS + SETUP_RESULT):
            problems.append(f"step {i}: the setup slot shows a result before any run")
    for i in range(2, len(dumps)):
        missing = [text for text in ["4 rows", SETUP_DECLARATION] + SETUP_ROWS + SETUP_RESULT if text not in words[i]]
        if missing:
            problems.append(f"step {i}: the setup slot lacks {missing}")
        if any(text.startswith("No setup found") for text in words[i]):
            problems.append(f"step {i}: the setup slot says no setup was found")
    filled = "Setup row 1: Prepare and Cleanup set around Core (5 twists in the draft)"
    if not any(filled in text for text in words[5]):
        problems.append("step 5: row 1 did not fill Prepare and Cleanup around the twist in Core")
    if not any(re.search(r"Setup: row 1 alone is preview #\d+", text) for text in words[7] + words[8]):
        problems.append("row 1 alone was not staged as the preview")
    for i, dump in enumerate(dumps):
        errors = dump.get("errors", {})
        if errors.get("lamp") is not True or errors.get("count") != 0 or errors.get("failures") != 0:
            problems.append(f"step {i}: error lamp is {errors!r}")
    return problems


EXPECT["setup"] = expect_setup
IN_SAMPLE_MODE.add("setup")


# B29 (ADR-D02.4, .45, .46, .50, .52): the research pane's Runs tab. research.runs brings the pane from Solve; a start
# before any declaration is refused; budget, seed and depth are declared; the sample's run stops at its budget as cut
# off and reproduces with bytes equal; both CR-17 rows are refused with their reason; History gains no entry.
RESEARCH_RUN = "research.run/" + hashlib.sha256(b"magic600-d02-research-sample-run/1 seed=7430 depth=40 budget=4096").hexdigest()
RESEARCH_STEPS = ["start", "wait 2000 ms", "command research.runs", "wait 1500 ms", "command research.run.start",
                  "command research.run.declare budget=4096", "command research.run.declare seed=7430",
                  "command research.run.declare depth=40", "command research.run.start", "wait 1500 ms",
                  f"command research.run.reproduce {RESEARCH_RUN}", f"command research.run.view {RESEARCH_RUN}",
                  f"command research.check.trace {RESEARCH_RUN}", "command research.obligations", "wait 1000 ms",
                  "command research.runs", f"command object.inspect {RESEARCH_RUN}", "wait 3000 ms"]
CR17 = "row not in the command table yet (CR-17)"


def expect_research(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """Runs one palette row from Solve; a declared budget before the start; the run cut off at its budget, never a
    verdict; reproduced with bytes equal; the CR-17 rows refused with their reason; History unchanged."""
    if [dump["do"] for dump in dumps] != RESEARCH_STEPS:
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    problems = []
    history = dumps[0].get("research", {}).get("history_entries")
    for index, dump in enumerate(dumps):
        research = dump.get("research", {})
        if research.get("evidence") != "synthetic" or "evidence: synthetic" not in research.get("head", ""):
            problems.append(f"step {index}: the pane's evidence is {research.get('evidence')!r}, head "
                            f"{research.get('head')!r}")
        problems += research_chip(index, research)
        if research.get("pane") is not (index >= 2):
            problems.append(f"step {index}: the research pane is {'shown' if research.get('pane') else 'hidden'}")
        tab = "Obligations" if index in (13, 14) else "Runs" if index >= 2 else "Claims"
        if research.get("tab") != tab:
            problems.append(f"step {index}: the tab is {research.get('tab')!r}, not {tab}")
        if research.get("history_entries") != history:
            problems.append(f"step {index}: History holds {research.get('history_entries')} entries, not {history}")
        started = research.get("started", [])
        if len(started) != (1 if index >= 8 else 0):
            problems.append(f"step {index}: {len(started)} runs started")
        if index >= 8 and started:
            run = started[0]
            got = ("research.run/" + run.get("run", ""), run.get("state"), run.get("line"), run.get("seed"), run.get("depth"),
                   run.get("budget"), run.get("covered"))
            want = (RESEARCH_RUN, "BudgetStopped", "not found within budget", 7430, 40, 4096, "4096")
            if got != want:
                problems.append(f"step {index}: the run is {got}")
            again = "reproduced: bytes equal" if index >= 10 else ""
            if run.get("reproduction") != again:
                problems.append(f"step {index}: the reproduction reads {run.get('reproduction')!r}")
        if index in (8, 9, 10, 11, 12, 16):
            if research.get("open") != RESEARCH_RUN:
                problems.append(f"step {index}: the open object is {research.get('open')!r}")
            lines = [line for line in research.get("lines", []) if line.get("ref") == RESEARCH_RUN]
            rows = [line for line in lines if line.get("kind") == "Row"]
            if len(rows) != 1 or rows[0].get("verdict") != "not found within budget":
                problems.append(f"step {index}: the run's row reads {[row.get('verdict') for row in rows]}")
            details = [line["text"] for line in lines if line.get("kind") == "Detail"]
            if "the run stopped at its budget: no verdict (covered 4096)" not in details:
                problems.append(f"step {index}: the run's lines are {details}")
            if research.get("cr17") != {"research.run.view": CR17, "research.check.trace": CR17}:
                problems.append(f"step {index}: the CR-17 reasons are {research.get('cr17')}")
    if "declare a budget before the run starts (ADR-D02.4)" not in dumps[4].get("research", {}).get("notice", ""):
        problems.append(f"step 4: the refused start said {dumps[4].get('research', {}).get('notice')!r}")
    declared = dumps[7].get("research", {})
    if (declared.get("declared") != "seed 7430: free exploration (7422 to 7499), not scored · depth 40 · budget 4,096 steps"
            or declared.get("start") != "ready"):
        problems.append(f"step 7: declared {declared.get('declared')!r}, start {declared.get('start')!r}")
    for index in (11, 12):
        if CR17 not in dumps[index].get("research", {}).get("notice", ""):
            problems.append(f"step {index}: the CR-17 row said {dumps[index].get('research', {}).get('notice')!r}")
    obligations = [line for line in dumps[13].get("research", {}).get("lines", []) if line.get("kind") == "Row"]
    if len(obligations) != 4:
        problems.append(f"step 13: the Obligations tab shows {len(obligations)} rows")
    for dump in dumps:
        errors = dump.get("errors", {})
        if errors.get("lamp") is not True:
            problems.append(f"step {dump['step']}: the frame carries no error lamp")
        if errors.get("count") != 0 or errors.get("failures") != 0:
            problems.append(f"step {dump['step']}: the error lamp counts {errors.get('count')} "
                            f"({errors.get('failures')} caught), the last: {errors.get('last')!r}")
    return problems


EXPECT["research"] = expect_research
IN_SAMPLE_MODE.add("research")


# B15 (ADR-D02.20, .21, .23, .24, .30): the tint channel on a state's session fixture. The entry slot's contact classes
# are on at open; the palette row switches to the height ramp, the off-lattice outline and back to the classes; a
# domain focus dims the other domains; the height ramp then draws under that dim, and none leaves the dim alone. After
# each step the dump names the bytes last sent to the core on each channel by their sha256; the expectation recomputes
# them piece by piece from the sample's pose rows, pose ids and contact classes, with the dump's hover and selection
# outlines on top.
TINT_FOCUS_DOMAIN = 5
TINT_STEPS = [("wait 5001 ms", "contact", None), ("wait 811 ms", "height", None), ("wait 812 ms", "off-lattice", None),
              ("wait 813 ms", "contact", None), ("wait 814 ms", "contact", TINT_FOCUS_DOMAIN),
              ("wait 815 ms", "height", TINT_FOCUS_DOMAIN), ("wait 816 ms", "none", TINT_FOCUS_DOMAIN)]


def tint_tables(state: str) -> dict:
    """A sample state's pose ids (each piece's pose row), its rows' domain, height, lattice flag and count, and its
    first contact classes (contact.json: the classes an empty class memory gives)."""
    import array
    import base64
    import hashlib
    import zlib
    poses = load(SAMPLES / state / "poses.json")
    raw = zlib.decompress(base64.b64decode(poses["data"]))
    if hashlib.sha256(raw).hexdigest() != poses["sha256"]:
        raise ValueError(f"{state}: poses.json's pose ids differ from their digest")
    ids = array.array("I")
    ids.frombytes(raw)
    if sys.byteorder != "little":
        ids.byteswap()
    style = load(SAMPLES / state / "style.json")
    contact = load(SAMPLES / state / "contact.json")
    rows = style["rows"]
    return {"state": state, "ids": ids, "domain": [row["domain"] for row in rows],
            "height": [row["heights"][0] for row in rows], "count": [row["count"] for row in rows],
            "lattice": [isinstance(row.get("k"), int) for row in rows], "lattice_domain": style["lattice_domain"],
            "classes": contact["classes"]}


def class_entry(c: int) -> int:
    """JumbleTint.ClassEntry: classes 1 to 6 are palette entries 3 to 8, the overflow state 9, the lattice none."""
    return 2 + c if 1 <= c <= 6 else 9 if c == 7 else 0


def height_entry(h: int) -> int:
    """JumbleTint.HeightEntry: the ramp's steps 2, 4, 6 and above 6 are entries 10 to 13; height 0 is untinted."""
    return 0 if h <= 0 else 10 if h <= 2 else 11 if h <= 4 else 12 if h <= 6 else 13


def tint_line(tables: dict, mode: str) -> str:
    """The legend's line for a mode with its counts of pieces (JumbleTint.Line)."""
    rows = range(len(tables["domain"]))
    classes, domain = tables["classes"], tables["domain"]

    def count(keep) -> str:
        return f"{sum(tables['count'][r] for r in rows if keep(r)):,}"
    if mode == "contact":
        parts = [f"{c} {count(lambda r, c=c: classes[domain[r]] == c)}" for c in range(1, 7) if c in classes]
        over = classes.count(7)
        if over:
            parts.append(f"overflow {over:,} {'domain' if over == 1 else 'domains'} {count(lambda r: classes[domain[r]] == 7)}")
        if tables["lattice_domain"] >= 0:
            parts.append(f"lattice neutral {count(lambda r: domain[r] == tables['lattice_domain'])}")
        return "Contact classes · " + " · ".join(parts)
    if mode == "height":
        largest = max(tables["height"], default=0)
        labels = ["2", "4", "6", "above 6" + (f" (largest {largest})" if largest > 6 else "")]
        return "Height · " + " · ".join(f"{label} {count(lambda r, e=10 + s: height_entry(tables['height'][r]) == e)}"
                                        for s, label in enumerate(labels))
    if mode == "off-lattice":
        return f"Off-lattice outline · {count(lambda r: not tables['lattice'][r])} pieces"
    return ""


def tint_expected(tables: dict, mode: str, focused: int | None, hover: int | None, selection: int | None) -> dict:
    """What a step's dump must say: the mode, the classes, the sha256 of the outline bytes (the mode's under the
    hover's and the selection's outlines) and of the fill bytes (the height ramp with the dim bit on every piece
    outside the focused domain; empty when none), the focus dim in both views, and the legend on both borders."""
    import hashlib
    rows = range(len(tables["domain"]))
    outline_row, fill_row = [0] * len(rows), [0] * len(rows)
    if mode == "contact":
        outline_row = [class_entry(tables["classes"][tables["domain"][r]]) for r in rows]
    elif mode == "height":
        fill_row = [height_entry(tables["height"][r]) for r in rows]
    elif mode == "off-lattice":
        outline_row = [0 if tables["lattice"][r] else 14 for r in rows]
    if focused is not None:
        fill_row = [f | (0 if tables["domain"][r] == focused else 0x80) for r, f in zip(rows, fill_row)]
    outline = bytearray(outline_row[p] for p in tables["ids"])
    if hover is not None:
        outline[hover] = 2
    if selection is not None:
        outline[selection] = 1
    fill = bytes(fill_row[p] for p in tables["ids"]) if any(fill_row) else None
    line = tint_line(tables, mode)
    return {"mode": mode, "bound": tables["state"], "classes": tables["classes"], "moved": [], "hover": hover,
            "selection": selection,
            "outline_sha256": hashlib.sha256(bytes(outline)).hexdigest(),
            "fill_sha256": hashlib.sha256(fill).hexdigest() if fill is not None else "",
            "focus": "none" if focused is None else "dimmed", "dims": [focused is not None] * 2,
            "line": line, "global_legend": line, "local_legend": line}


def expect_tint(dumps: list[dict], source: dict, state: str | None, tables: dict | None = None) -> list[str]:
    """The tint script: after each step the bytes sent to the core equal the sample's, piece by piece."""
    if state is None or not (SAMPLES / state / "contact.json").is_file():
        return ["the tint script needs a sample state with a contact graph"]
    by_step = {dump["do"]: dump for dump in dumps}
    if any(mark not in by_step for mark, _, _ in TINT_STEPS):
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    tables = tables if tables is not None else tint_tables(state)
    problems = []
    for mark, mode, focused in TINT_STEPS:
        tint = by_step[mark].get("tint")
        if not isinstance(tint, dict):
            problems.append(f"after '{mark}' the dump has no tint")
            continue
        want = tint_expected(tables, mode, focused, tint.get("hover"), tint.get("selection"))
        for key, value in want.items():
            if tint.get(key) != value:
                said = f"{mode}" + ("" if focused is None else f", domain {focused} focused")
                problems.append(f"after '{mark}' ({said}) {key} is {str(tint.get(key))[:120]!r}, not {str(value)[:120]!r}")
    if not problems:
        print(f"tint: {len(TINT_STEPS)} steps, the bytes sent equal {state}'s piece by piece; "
              f"classes {sorted(set(tables['classes']) - {0})}; " + tint_line(tables, "contact"))
    return problems


EXPECT["tint"] = expect_tint
ON_FIXTURE.add("tint")


# B16 (ADR-D02.26, .34; jumbling-observation.md rule 5): the overlays as lines on the outline channel. No overlay while
# no cap is named; cap 150's trace (its inside set); a domain focus dimmed, then hidden by F10, with every overlay piece
# kept out of the dim set and counted; a focused repair row's step 2 outlines its moving set in place of the trace; the
# blocked cap 0 outlines its straddlers with the certificate points counted. After each step the expectation recomputes
# the outline and fill bytes sent to the core from the sample's masks, pose ids and contact classes, piece by piece.
OVERLAY_DOMAIN = 5
OVERLAY_STEPS = [("wait 5001 ms", None, None, None), ("wait 1501 ms", 150, None, None),
                 ("wait 1502 ms", 150, None, "dimmed"), ("wait 1503 ms", 150, None, "hidden"),
                 ("wait 1504 ms", 150, 2, "hidden"), ("wait 1505 ms", 0, None, "hidden")]


def overlay_mask(mask: dict, pieces: int) -> list[bool]:
    """A sample piece mask (bits-le-zlib-base64), checked against its digest and count."""
    import base64
    import hashlib
    import zlib
    raw = zlib.decompress(base64.b64decode(mask["data"]))
    if hashlib.sha256(raw).hexdigest() != mask["sha256"]:
        raise ValueError("a piece mask differs from its digest")
    bits = [bool(raw[p >> 3] & (1 << (p & 7))) for p in range(pieces)]
    if sum(bits) != mask["count"]:
        raise ValueError("a piece mask differs from its count")
    return bits


def overlay_expected(tables: dict, state: str, cap: int | None, step: int | None, focus: str | None,
                     hover: int | None, selection: int | None) -> dict:
    """What a step's dump must say: the layer and its pieces, the never-hidden count, the exact hidden count under the
    domain focus, and the sha256 of the outline bytes (the contact classes, the overlay's entry 15 on its pieces, then
    the hover's and the selection's outlines) and of the fill bytes (the dim bit on every piece outside the focused
    domain that is neither an overlay piece nor never-hidden)."""
    import hashlib
    ids, domain = tables["ids"], tables["domain"]
    pieces = len(ids)
    lines, never, points, label = None, None, None, ""
    if cap is not None:
        record = load(SAMPLES / state / f"cap-{cap}.json")
        masks = record.get("masks", {})
        straddlers = overlay_mask(masks["straddlers"], pieces) if "straddlers" in masks else None
        inside = overlay_mask(masks["inside"], pieces) if "inside" in masks else None
        moving = None
        if step is not None:
            row = next(r for r in record["repair_steps"] if r["row"] == 1)
            moving = overlay_mask(row["moving_sets"][step - 1], pieces)
            label = f"Repair {step} of {len(row['letters'])}"
        points = record.get("straddlers", {}).get("certificate_points")
        lines = moving or straddlers or inside
        never = moving or straddlers
    keep = [bool((lines and lines[p]) or (never and never[p])) for p in range(pieces)]
    outline = bytearray(class_entry(tables["classes"][domain[ids[p]]]) for p in range(pieces))
    for p in range(pieces):
        if lines and lines[p]:
            outline[p] = 15
    if hover is not None:
        outline[hover] = 2
    if selection is not None:
        outline[selection] = 1
    fill = None
    if focus is not None:
        fill = bytes(0x80 if domain[ids[p]] != OVERLAY_DOMAIN and not keep[p] else 0 for p in range(pieces))
    count = sum(lines) if lines else 0
    hidden = sum(1 for p in range(pieces) if lines and lines[p] and domain[ids[p]] != OVERLAY_DOMAIN) if focus else 0
    layer = "None" if not lines else "MovingSet" if step is not None else "CutTrace"
    return {"layer": layer, "cap": cap, "step": label, "pieces": count, "never_hidden": sum(never) if never else 0,
            "hidden": hidden, "certificate_points": points, "outline_entries": sum(1 for b in outline if b == 15),
            "kept": sum(keep), "kept_dimmed": 0,
            "outline_sha256": hashlib.sha256(bytes(outline)).hexdigest(),
            "fill_sha256": hashlib.sha256(fill).hexdigest() if fill is not None else "", "focus": focus or "none"}


def overlay_words(want: dict) -> str:
    """JumbleOverlay.Words for an expectation."""
    if want["cap"] is None:
        return ""
    glyphs = f" · {want['certificate_points']:,} certificate points (glyphs wait for their gate row)" if want["certificate_points"] is not None else ""
    kept = f" · {want['hidden']:,} kept from the filter, outlined" if want["hidden"] else ""
    if want["layer"] == "MovingSet":
        head = f"{want['step']} · moving set outlined, never hidden"
    elif want["certificate_points"] is not None:
        head = f"Cut of cap {want['cap']:,} · straddlers outlined, never hidden"
    else:
        head = f"Cut of cap {want['cap']:,} · inside set outlined"
    return f"{head} · {want['pieces']:,} pieces{kept}{glyphs}"


def expect_overlays(dumps: list[dict], source: dict, state: str | None, tables: dict | None = None) -> list[str]:
    """The overlays script: after each step the overlay's counts, its line on both borders and the bytes sent to the
    core equal the sample's, piece by piece; no never-hidden or overlay piece carries the dim bit."""
    if state is None or not (SAMPLES / state / "cap-150.json").is_file() or not (SAMPLES / state / "cap-0.json").is_file():
        return ["the overlays script needs a sample state with records of caps 150 and 0"]
    by_step = {dump["do"]: dump for dump in dumps}
    if any(mark not in by_step for mark, _, _, _ in OVERLAY_STEPS):
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    tables = tables if tables is not None else tint_tables(state)
    problems = []
    for mark, cap, step, focus in OVERLAY_STEPS:
        overlay, tint = by_step[mark].get("overlay"), by_step[mark].get("tint")
        if not isinstance(overlay, dict) or not isinstance(tint, dict):
            problems.append(f"after '{mark}' the dump has no overlay or tint")
            continue
        want = overlay_expected(tables, state, cap, step, focus, tint.get("hover"), tint.get("selection"))
        line = overlay_words(want)
        for key, value in [*want.items(), ("line", line), ("global_line", line), ("local_line", line)]:
            said = overlay if key not in ("outline_sha256", "fill_sha256", "focus") else tint
            if said.get(key) != value:
                problems.append(f"after '{mark}' {key} is {str(said.get(key))[:120]!r}, not {str(value)[:120]!r}")
    if not problems:
        named = by_step["wait 1501 ms"]["overlay"]
        stepped = by_step["wait 1504 ms"]["overlay"]
        print(f"overlays: {len(OVERLAY_STEPS)} steps, the bytes sent equal {state}'s piece by piece; "
              f"{named['line']}; under the hidden focus: {stepped['line']}")
    return problems


EXPECT["overlays"] = expect_overlays
ON_FIXTURE.add("overlays")


def expect_error_overlay(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """The named cap's straddlers use the reserved outline bit in the shared style, and clearing preserves the head."""
    import hashlib
    import struct

    if state != "wj-ia-300":
        return ["the error-overlay script needs the wj-ia-300 fixture"]
    record = load(SAMPLES / state / "cap-0.json")
    pieces = [p for p, marked in enumerate(overlay_mask(record["masks"]["straddlers"], 177_120)) if marked]
    sha = hashlib.sha256(struct.pack(f"<{len(pieces)}i", *pieces)).hexdigest()
    by_step = {dump["do"]: dump for dump in dumps}
    steps = {"wait 5001 ms": 0, "wait 1501 ms": len(pieces), "wait 1502 ms": 0}
    if not by_step.keys() >= steps.keys() or not pieces:
        return [f"missing fixture pieces or steps: {list(by_step)}"]
    problems = []
    for dump in dumps:
        if dump.get("tuning", {}).get("state_digest") != record["digest"]:
            problems.append(f"after '{dump['do']}' the session digest differed from the fixture")
    for mark, count in steps.items():
        dump = by_step[mark]
        overlay = dump.get("error_overlay", {})
        if overlay.get("pieces") != count or overlay.get("outline_bits") != count:
            problems.append(f"after '{mark}' expected {count} pieces and sent bits, got "
                            f"{overlay.get('pieces')} and {overlay.get('outline_bits')}")
        if mark == "wait 1502 ms" and any(
                d.get("source") in ("jumble.cap.certificate", "jumble.repair.list", "jumble.cap.covers")
                for d in overlay.get("diagnostics", [])):
            problems.append("unnaming the cap kept its diagnostics")
        if count == 0:
            continue
        palette = overlay.get("palette", {})
        global_rgba = palette.get("1", [])
        if len(global_rgba) != 4 or global_rgba[3] <= 0 or any(palette.get(slot) != global_rgba for slot in ("2", "3")):
            problems.append(f"after '{mark}' the slots did not share an active error palette: {palette}")
        if overlay.get("sha256") != sha or overlay.get("first_ids") != pieces[:16]:
            problems.append(f"after '{mark}' the error piece ids differed from cap 0's straddlers")
        failed = {d["source"]: d for d in overlay.get("diagnostics", []) if d.get("state") == "failed"}
        for diagnostic in ("jumble.cap.certificate", "jumble.repair.list", "jumble.cap.covers"):
            item = failed.get(diagnostic, {})
            if item.get("caps") != [0] or item.get("pieces") != count:
                problems.append(f"after '{mark}' {diagnostic} did not name cap 0 and its {count} pieces")
    if not problems:
        print(f"error overlay: cap 0 named {len(pieces)} pieces in Global and Local; cleared with the same session digest")
    return problems


EXPECT["error-overlay"] = expect_error_overlay
ON_FIXTURE.add("error-overlay")


def expect_grip_frame(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """The picked cap, its shared frame, the exact routed twists and both panes' modifier ghost."""
    steps = ["wait 2500 ms", "wait 600 ms", "command grip.select", "command grip.frame.next", "wait 1600 ms",
             "wait 601 ms", "wait 602 ms", "wait 603 ms", "wait 604 ms", "wait 1601 ms"]
    by_step = {dump["do"]: dump for dump in dumps}
    if any(not by_step.get(step, {}).get("grip") for step in steps):
        return ["the grip script is missing a grip checkpoint"]
    start, plain, selected, framed, turned, held, away, again, released, clicked = (
        by_step[step]["grip"] for step in steps)
    problems = []

    def require(condition, message):
        if not condition:
            problems.append(message)

    def head(g):
        return tuple(g.get(field) for field in ("state_digest", "committed", "refused"))

    def keys(g):
        return {row["chord"]: row["item"] for row in g.get("keys", [])}

    require((start.get("cap"), start.get("frame")) == (0, 0), "the opening grip is not cap 0, frame 0")
    require(keys(start) == {"KeyJ": 1, "Shift+KeyJ": 1, "KeyK": 2, "Shift+KeyK": 5, "KeyL": 3, "Shift+KeyL": 9},
            "the default grip lost the pinned key items")
    pick = by_step[steps[1]].get("pick", {}).get("selection") or {}
    slot = pick.get("slot")
    cap = selected.get("cap")
    frame = framed.get("frame")
    require(isinstance(slot, int) and slot // 433 == cap == 150, "grip.select did not take the picked cap 150")
    require(pick.get("revision") == selected.get("host", {}).get("revision"), "grip.select used a stale pick")
    require(all(head(g) == head(start) for g in (plain, selected, framed)), "picking or changing the grip changed the head")
    require(selected.get("frame") == 0 and frame != selected.get("frame") and frame in framed.get("admissible_frames", []),
            "grip.frame.next did not choose a new admissible frame")
    old_marks = [mark["corner"] for mark in selected.get("marks", [])]
    marks = {mark["index"]: mark["corner"] for mark in framed.get("marks", [])}
    require(set(marks) == {1, 2, 3, 4} and len(set(marks.values())) == 4, "the frame needs four ordered marks")
    require(list(marks.values()) != old_marks and set(marks.values()) == set(old_marks),
            "the next frame did not reorder the same corners")
    require(keys(selected) != keys(framed) and set(keys(framed)) == set(keys(start)), "the twist keys did not follow the frame")
    host = framed.get("host", {})
    require(host.get("cap") == cap and host.get("frame") == frame, "the shell and host grips disagree")
    require(any(row.get("frame") == frame and row.get("keys") == framed.get("keys")
                for row in host.get("frames", [])), "the visible key table differs from the host's exact map")

    def panes(grip, expect_direction):
        mark_by_index = {mark["index"]: mark["corner"] for mark in grip.get("marks", [])}
        shown = {pane["pane"]: pane for pane in grip.get("drawn", [])}
        for name in ("Global", "Local"):
            pane = shown.get(name, {})
            rect, drawn = pane.get("rect"), pane.get("marks", [])
            if rect is None or not 1 <= len(drawn) <= 4:
                problems.append(f"{name} drew no set of grip marks")
                continue
            x, y, w, h = rect
            def inside(px, py):
                return x <= px < x + w and y <= py < y + h
            require(all(mark_by_index.get(mark["index"]) == mark["corner"] and inside(mark["x"], mark["y"])
                        for mark in drawn), f"{name}'s grip marks differ from the frame or lie outside the pane")
            direction = pane.get("direction")
            require(inside(*direction) if direction and expect_direction else direction is None and not expect_direction,
                    f"{name}'s direction glyph did not follow the ghost")

    for grip in (start, selected, framed):
        panes(grip, False)
    item = keys(framed).get("KeyJ")
    for before, after, action in ((framed, turned, "KeyJ"), (turned, clicked, "Ctrl+click")):
        last = after.get("host", {}).get("last", {})
        require(after.get("committed") == before.get("committed", -1) + 1 and after.get("refused") == before.get("refused"),
                f"{action} did not commit exactly one twist")
        require(last == {"cap": cap, "item": item}, f"{action} committed the wrong cap or item")
        require(after.get("state_digest") != before.get("state_digest"), f"{action} did not move the head")
    require(not turned.get("ghost", {}).get("on"), "hover without Ctrl showed a ghost")
    for grip, on in ((held, True), (away, False), (again, True), (released, False)):
        ghost = grip.get("ghost", {})
        require(head(grip) == head(turned), "hover or modifier release changed the head")
        require(ghost.get("on") == on and ghost.get("modifier") == "Ctrl", "the ghost did not follow Ctrl and the hover")
        if on:
            require(ghost.get("cap") == cap and ghost.get("item") == item and ghost.get("moving", 0) > 0
                    and len(ghost.get("sha256") or "") == 64, "the ghost differs from the click twist")
            require(ghost.get("palette") == 2 and ghost.get("outlined", 0) > 0, "the moving pieces lack the hover outline")
        else:
            require(ghost.get("moving") == 0, "the released ghost kept a moving set")
        panes(grip, on)
    return problems


EXPECT["grip-frame"] = expect_grip_frame
ON_FIXTURE.add("grip-frame")


def expect_grip_keycaps(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """ADR-D01.74 on ADR-D01.75's grip: with the key map pinned over Local, J, K and L carry the grip's twists (each
    chord's item and its cycle over the marks, stamped inside the cap) and the header names the grip's cap and frame;
    the next frame re-binds keys, and the map marks exactly the plain keys whose item changed until it has been closed."""
    steps = ["wait 700 ms", "wait 701 ms", "wait 702 ms"]
    by_step = {dump["do"]: dump for dump in dumps}
    if any(not by_step.get(step, {}).get("grip") or not by_step[step].get("keycaps") for step in steps):
        return ["the grip keycaps script is missing a grip or keycaps checkpoint"]
    first, framed, again = (by_step[step] for step in steps)
    problems = []

    def require(condition, message):
        if not condition:
            problems.append(message)

    def caps(dump):
        return {cap["key"]: cap for cap in dump["keycaps"].get("grip", [])}

    def keys(dump):
        return {row["chord"]: row for row in dump["grip"].get("keys", [])}

    for name, dump in (("the pinned map", first), ("the map after the next frame", framed), ("the map shown again", again)):
        shown, grip, want = dump["keycaps"], dump["grip"], keys(dump)
        require(shown.get("shown") is True, f"{name} is not shown")
        require(grip.get("cap") == 150, f"{name}: the grip is not cap 150")
        require(shown.get("header", "").endswith(f" · grip: cap 150, frame {grip.get('frame')}"),
                f"{name}'s header reads {shown.get('header')!r}")
        carried = caps(dump)
        require(sorted(carried) == ["J", "K", "L"], f"{name}: the caps that carry the grip are {sorted(carried)}")
        for key, cap in carried.items():
            row = want.get("Key" + key, {})
            require(cap.get("chord") == "Key" + key and cap.get("item") == row.get("item")
                    and cap.get("meaning") == cap.get("label") == row.get("meaning") and cap.get("context") == "grip",
                    f"{name}: {key}'s cap differs from the grip's key")
        stamps = {stamp["key"]: stamp for stamp in shown.get("stamps", [])}
        require(sorted(stamps) == ["J", "K", "L"], f"{name}: the stamped caps are {sorted(stamps)}")
        for key, stamp in stamps.items():
            x, y, w, h = stamp["box"]
            require(stamp.get("label") == carried.get(key, {}).get("meaning")
                    and x <= stamp["x"] < x + w and y <= stamp["y"] < y + h, f"{name}: {key}'s stamp differs or lies outside its cap")
    require(first["grip"].get("frame") == 0 and framed["grip"].get("frame") not in (None, 0),
            "grip.frame.next did not change the frame")
    def twist(dump, key):
        row = keys(dump).get("Key" + key, {})
        return row.get("item"), row.get("meaning")

    moved = sorted(key for key in "JKL" if twist(first, key) != twist(framed, key))
    require(bool(moved), "the next frame re-bound no plain twist key")
    require(not any(cap.get("changed") for cap in caps(first).values()), "the first pinned map marks a key")
    marked = sorted(key for key, cap in caps(framed).items() if cap.get("changed"))
    require(marked == moved, f"after the next frame the map marks {marked}, not the re-bound keys {moved}")
    require(all(caps(framed).get(key, {}).get("meaning") == cap.get("meaning") for key, cap in caps(first).items()),
            "a key's cycle over the marks changed with the frame")
    mark = "corner mark: changed meaning since the grip frame changed"
    require(mark in framed["keycaps"].get("legend", ""), "the legend does not name the grip frame change's mark")
    require(by_step.get("chord Escape", {}).get("keycaps", {}).get("shown") is False, "Escape left the key map shown")
    require(not any(cap.get("changed") for cap in caps(again).values()) and mark not in again["keycaps"].get("legend", ""),
            "the map shown again still marks keys")
    print(f"grip-keycaps: frame {framed['grip'].get('frame')} re-bound {moved}")
    return problems


EXPECT["grip-keycaps"] = expect_grip_keycaps
ON_FIXTURE.add("grip-keycaps")


TUNING_KNOBS = [("wait 702 ms", "tone", 0.05, True), ("wait 703 ms", "contrast", 1.15, True),
                ("wait 704 ms", "blackLevel", 0.08, True), ("wait 705 ms", "accentHue", 40.0, False),
                ("wait 706 ms", "gridIntensity", 0.9, False)]


def tuning_knob(live: str, knob: str) -> float | None:
    """A knob's value in the dump's live tuning ("graphite · solving · accentHue=195 blackLevel=0 ...")."""
    for word in live.split():
        if word.startswith(knob + "="):
            return float(word.split("=", 1)[1])
    return None


def expect_tuning(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """B30's tuning script: the palette's row opens the panel in the right dock; each knob step shows its value, the
    knobs that move the tint's ramps send the core a new palette and the frame's window colour moves with tone; a
    click on the panel's tone knob moves it; two swaps come back; the held compare draws slot B and the release
    puts slot A back. After every step the head's state digest and the tint bytes are the open's."""
    marks = ["wait 701 ms"] + [m for m, _, _, _ in TUNING_KNOBS] + [f"wait {n} ms" for n in range(707, 712)]
    by_step = {dump["do"]: dump for dump in dumps}
    if any(mark not in by_step for mark in marks):
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    problems = []
    tuning = {mark: by_step[mark].get("tuning") for mark in marks}
    tint = {mark: by_step[mark].get("tint") or {} for mark in marks}
    if any(not isinstance(t, dict) for t in tuning.values()):
        return ["a step's dump has no tuning (is D02 on and the panel built?)"]
    first = tuning["wait 701 ms"]
    # Below 1600 px the right dock's panel folds to its rail (owner, 26 November 2026); an explicit open shows what
    # it opens, so the row reveals the panel over the fold (B31).
    folded = first.get("folded") is True
    if first.get("right_dock") != "tuning" or first.get("shown") is not True or first.get("revealed") is not folded:
        problems.append(f"after tuning.open the right dock shows {first.get('right_dock')!r}, shown {first.get('shown')}, "
                        f"folded {first.get('folded')}, revealed {first.get('revealed')} at {first.get('window_width')} px")
    if not first.get("state_digest"):
        problems.append("the head's state digest is not read")
    if not tint["wait 701 ms"].get("palette_sha256"):
        problems.append("no tint palette was sent to the core at the open")
    for mark in marks:
        t = tuning[mark]
        if t.get("state_digest") != first.get("state_digest"):
            problems.append(f"after '{mark}' the state digest is {t.get('state_digest')!r}, not the open's")
        if t.get("status"):
            problems.append(f"after '{mark}' the panel refused: {t.get('status')!r}")
        same_pointer = (tint[mark].get("hover"), tint[mark].get("selection")) == \
            (tint["wait 701 ms"].get("hover"), tint["wait 701 ms"].get("selection"))
        for key in ("outline_sha256", "fill_sha256"):
            if same_pointer and tint[mark].get(key) != tint["wait 701 ms"].get(key):
                problems.append(f"after '{mark}' the tint's {key} moved")
    before = "wait 701 ms"
    for mark, knob, value, moves in TUNING_KNOBS:
        shown = tuning_knob(tuning[mark].get("live", ""), knob)
        if shown is None or abs(shown - value) > 1e-6:
            problems.append(f"after '{mark}' {knob} is {shown}, not {value}")
        if moves and tint[mark].get("palette_sha256") == tint[before].get("palette_sha256"):
            problems.append(f"after '{mark}' {knob} sent the core no new palette")
        before = mark
    if tuning["wait 702 ms"].get("window") == first.get("window"):
        problems.append("tone did not move the frame's window colour")
    tone = tuning_knob(tuning["wait 707 ms"].get("live", ""), "tone")
    if tone is None or abs(tone) > 0.01 or tint["wait 707 ms"].get("palette_sha256") == tint["wait 706 ms"].get("palette_sha256"):
        problems.append(f"the click on the panel's tone knob left tone at {tone} or sent no new palette")
    tuned, opening = tuning["wait 707 ms"].get("live"), first.get("live")
    for mark, slot, live, holding in (("wait 708 ms", "B", opening, False), ("wait 709 ms", "A", tuned, False),
                                      ("wait 710 ms", "A", opening, True), ("wait 711 ms", "A", tuned, False)):
        t = tuning[mark]
        if (t.get("slot"), t.get("live"), t.get("holding")) != (slot, live, holding):
            problems.append(f"after '{mark}' slot {t.get('slot')}, holding {t.get('holding')}, live {t.get('live')!r}; "
                            f"expected slot {slot}, holding {holding}, live {live!r}")
    if not problems:
        print(f"tuning: {len(marks)} steps; the panel opened in the right dock "
              f"({'revealed over the fold' if folded else 'shown'} at {first.get('window_width')} px); the state digest "
              f"{str(first.get('state_digest'))[:16]} held; tone, contrast and black level sent new palettes; "
              "two swaps and the held compare came back")
    return problems


EXPECT["tuning"] = expect_tuning
EXPECT["tuning-wide"] = expect_tuning
ON_FIXTURE.add("tuning")
ON_FIXTURE.add("tuning-wide")


def expect_tuning_save(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """B31's tuning-save script: the open reveals the panel over the fold; the shipped Graphite preset is applied; a
    knob forks it and Save fills the fork; the held simulation draws over the chrome (the core gets the identity with
    the G8 switch off) and goes on release; an Advanced override that breaks the height ramp recolours the pieces and
    Save refuses it in place naming the pair; after the reset Save passes; the shipped preset comes back as shipped.
    The state digest and the tint bytes never move."""
    marks = [f"wait {n} ms" for n in range(701, 712)]
    by_step = {dump["do"]: dump for dump in dumps}
    if any(mark not in by_step for mark in marks):
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    t = {mark: by_step[mark].get("tuning") for mark in marks}
    tint = {mark: by_step[mark].get("tint") or {} for mark in marks}
    if any(not isinstance(v, dict) for v in t.values()):
        return ["a step's dump has no tuning (is D02 on and the panel built?)"]
    problems = []
    first, identity = t["wait 701 ms"], [1, 0, 0, 0, 1, 0, 0, 0, 1]

    def want(mark, ok, text):
        if not ok:
            problems.append(f"after '{mark}': {text}")

    for mark in marks:
        want(mark, t[mark].get("state_digest") == first.get("state_digest"), "the state digest moved")
        want(mark, t[mark].get("shown") is True, "the panel is not shown")
        for key in ("outline_sha256", "fill_sha256"):
            want(mark, tint[mark].get(key) == tint["wait 701 ms"].get(key), f"the tint's {key} moved")
        if mark != "wait 708 ms":
            want(mark, not t[mark].get("status"), f"the panel refused: {t[mark].get('status')!r}")
    want("wait 701 ms", first.get("revealed") is (first.get("folded") is True) and first.get("preset_shipped") is True,
         f"open: revealed {first.get('revealed')}, preset {first.get('preset')!r}")
    shipped = t["wait 702 ms"]
    want("wait 702 ms", shipped.get("preset") == "graphite" and shipped.get("preset_shipped") is True,
         f"the shipped preset: {shipped.get('preset')!r}")
    fork = t["wait 703 ms"]
    want("wait 703 ms", fork.get("preset") not in ("", "graphite") and fork.get("preset_shipped") is False
         and fork.get("presets") == shipped.get("presets", 0) + 1 and "edited" in fork.get("preset_line", ""),
         f"the knob did not fork the shipped preset: {fork.get('preset')!r}, {fork.get('preset_line')!r}")
    saved = t["wait 704 ms"]
    want("wait 704 ms", saved.get("preset") == fork.get("preset") and not saved.get("issues")
         and "edited" not in saved.get("preset_line", ""),
         f"Save did not fill the fork: {saved.get('preset_line')!r} {saved.get('issues')}")
    held, gone = t["wait 705 ms"], t["wait 706 ms"]
    want("wait 705 ms", held.get("simulating") == "deutan" and held.get("filter") is True
         and held.get("class1_simulated") not in ("", held.get("class1")) and "chrome only" in held.get("simulation_line", "")
         and held.get("core_matrix") == identity and held.get("core_simulation") is False,
         f"the held simulation: {held.get('simulating')!r}, filter {held.get('filter')}, class 1 {held.get('class1')} -> "
         f"{held.get('class1_simulated')}, core {held.get('core_matrix')}, {held.get('simulation_line')!r}")
    want("wait 706 ms", gone.get("simulating") == "" and gone.get("filter") is False and gone.get("live") == held.get("live"),
         "the release left the simulation or changed the tuning")
    bad, refused = t["wait 707 ms"], t["wait 708 ms"]
    want("wait 707 ms", "tint.jumble.height.6" in bad.get("overrides", "")
         and tint["wait 707 ms"].get("palette_sha256") != tint["wait 706 ms"].get("palette_sha256"),
         "the override did not reach the pieces' tint palette")
    status = refused.get("status", "")
    want("wait 708 ms", status.startswith("not saved: height ramp tint.jumble.height.") and "tint.jumble.height.6" in status
         and "not monotone" in status and bool(refused.get("issues_shown")) and refused.get("preset") == bad.get("preset")
         and refused.get("presets") == bad.get("presets"), f"Save was not refused in place with its pair: {status!r}")
    after = t["wait 710 ms"]
    want("wait 710 ms", not after.get("issues") and "edited" not in after.get("preset_line", ""),
         f"Save after the reset: {after.get('issues')}")
    back = t["wait 711 ms"]
    want("wait 711 ms", back.get("preset") == "graphite" and back.get("live") == shipped.get("live"),
         f"the shipped preset did not come back as shipped: {back.get('live')!r}")
    if not problems:
        print(f"tuning-save: {len(marks)} steps at {first.get('window_width')} px; the panel "
              f"{'revealed over the fold' if first.get('revealed') else 'shown'}; Graphite forked to {fork.get('preset')} "
              f"and saved; deutan held over the chrome (class 1 {held.get('class1')} -> {held.get('class1_simulated')}), "
              f"core identity (G8 off); refused: {status}; the state digest {str(first.get('state_digest'))[:16]} held")
    return problems


EXPECT["tuning-save"] = expect_tuning_save
ON_FIXTURE.add("tuning-save")

TUNING_PENDING = "Live change: Esc reverts it"
TUNING_EXPORTED = "Exported tuning-diff.txt to the run folder"
TUNING_NOTHING = "nothing to revert: no live change is pending"


def expect_tuning_revert(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """CR-16's tuning-revert script: the open reveals the panel over the fold with nothing pending; a preset and a
    theme.tune make a live change and the panel says Esc reverts it; the real Escape key reverts both to the open's
    tuning and tint palette and the line goes; the revert row with nothing pending is refused, naming why; a new knob
    change is exported, the panel saying so, and Escape reverts it too. The state digest and the tint bytes never
    move."""
    marks = [f"wait {n} ms" for n in range(701, 709)]
    by_step = {dump["do"]: dump for dump in dumps}
    if any(mark not in by_step for mark in marks):
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    t = {mark: by_step[mark].get("tuning") for mark in marks}
    tint = {mark: by_step[mark].get("tint") or {} for mark in marks}
    if any(not isinstance(v, dict) for v in t.values()):
        return ["a step's dump has no tuning (is D02 on and the panel built?)"]
    texts = {mark: [text for _, text in map(tuple, by_step[mark].get("lines", []))] for mark in marks}
    problems = []
    first = t["wait 701 ms"]

    def want(mark, ok, text):
        if not ok:
            problems.append(f"after '{mark}': {text}")

    def pending(mark):
        return any(TUNING_PENDING in text for text in texts[mark])

    for mark in marks:
        want(mark, t[mark].get("state_digest") == first.get("state_digest"), "the state digest moved")
        want(mark, t[mark].get("shown") is True, "the panel is not shown")
        want(mark, not t[mark].get("status"), f"the panel refused: {t[mark].get('status')!r}")
        for key in ("outline_sha256", "fill_sha256"):
            want(mark, tint[mark].get(key) == tint["wait 701 ms"].get(key), f"the tint's {key} moved")
    want("wait 701 ms", first.get("revealed") is (first.get("folded") is True) and not pending("wait 701 ms"),
         f"open: revealed {first.get('revealed')}, a pending line {pending('wait 701 ms')}")
    preset, tuned = t["wait 702 ms"], t["wait 703 ms"]
    want("wait 702 ms", preset.get("preset") == "graphite.inspecting" and preset.get("live") != first.get("live")
         and pending("wait 702 ms"), f"the preset: {preset.get('preset')!r}, a pending line {pending('wait 702 ms')}")
    tone, contrast = tuning_knob(tuned.get("live", ""), "tone"), tuning_knob(tuned.get("live", ""), "contrast")
    want("wait 703 ms", tone is not None and abs(tone - 0.03) < 1e-6 and contrast is not None and abs(contrast - 1.1) < 1e-6
         and pending("wait 703 ms"), f"theme.tune: tone {tone}, contrast {contrast}, a pending line {pending('wait 703 ms')}")
    for mark in ("wait 704 ms", "wait 708 ms"):
        back = t[mark]
        want(mark, back.get("live") == first.get("live") and back.get("preset") == first.get("preset")
             and back.get("presets") == first.get("presets") and back.get("slot") == first.get("slot")
             and tint[mark].get("palette_sha256") == tint["wait 701 ms"].get("palette_sha256") and not pending(mark),
             f"Escape did not revert to the open's tuning: {back.get('live')!r} (preset {back.get('preset')!r}), "
             f"the palette {'back' if tint[mark].get('palette_sha256') == tint['wait 701 ms'].get('palette_sha256') else 'moved'}, "
             f"a pending line {pending(mark)}")
    refused = t["wait 705 ms"]
    want("wait 705 ms", refused.get("live") == first.get("live") and any(TUNING_NOTHING in text for text in texts["wait 705 ms"]),
         f"the revert with nothing pending was not refused with its reason: {texts['wait 705 ms'][-3:]}")
    moved, exported = t["wait 706 ms"], t["wait 707 ms"]
    tone = tuning_knob(moved.get("live", ""), "tone")
    want("wait 706 ms", tone is not None and abs(tone - 0.05) < 1e-6 and pending("wait 706 ms"),
         f"the second change: tone {tone}, a pending line {pending('wait 706 ms')}")
    want("wait 707 ms", exported.get("live") == moved.get("live") and pending("wait 707 ms")
         and any(text.startswith(TUNING_EXPORTED) for text in texts["wait 707 ms"]),
         f"the export: {[text for text in texts['wait 707 ms'] if 'xport' in text]}, a pending line {pending('wait 707 ms')}")
    if not problems:
        print(f"tuning-revert: {len(marks)} steps at {first.get('window_width')} px; a preset and theme.tune reverted by "
              f"Escape to {str(first.get('live'))[:40]}...; the empty revert refused; the difference exported; the state "
              f"digest {str(first.get('state_digest'))[:16]} held")
    return problems


def check_tuning_diff(out: Path) -> list[str]:
    """The difference file tuning.diff.export wrote into the run folder: the format line, the preset line, theme=
    and scene=, and the exported tone of the family in force."""
    path = out / "tuning-diff.txt"
    if not path.is_file():
        return ["tuning.diff.export wrote no tuning-diff.txt into the run folder"]
    raw = path.read_bytes()
    lines = raw.decode("utf-8").split("\n")
    problems = [] if raw.endswith(b"\n") and b"\r" not in raw and not raw.startswith(b"\xef\xbb\xbf") else \
        ["tuning-diff.txt is not LF text without a byte order mark"]
    if len(lines) < 5 or lines[0] != "format=magic600-tuning-diff/1" or not lines[1].startswith("preset=") \
            or not lines[2].startswith("theme=") or not lines[3].startswith("scene="):
        return problems + [f"tuning-diff.txt starts {lines[:4]}"]
    family = lines[2].split("=", 1)[1]
    if f"knob.{family}.tone=0.05" not in lines:
        problems.append(f"tuning-diff.txt has no knob.{family}.tone=0.05: {lines}")
    return problems


EXPECT["tuning-revert"] = expect_tuning_revert
ON_FIXTURE.add("tuning-revert")
FOLDER_CHECKS = {"tuning-revert": check_tuning_diff}


SCENE_LOOK_STEPS = [
    ("start", "solving", "graphite.a"),
    ("wait 2500 ms", "solving", "graphite.a"),
    ("chord Space", "solving", "graphite.a"),
    ("text > look.sticker_palette.set graphite.b", "solving", "graphite.a"),
    ("chord Enter", "solving", "graphite.b"),
    ("wait 650 ms", "solving", "graphite.b"),
    ("command look.scene.set celebrating", "celebrating", "graphite.b"),
    ("wait 651 ms", "celebrating", "graphite.b"),
    ("command look.scene.set inspecting", "inspecting", "graphite.b"),
    ("wait 652 ms", "inspecting", "graphite.b"),
    ("command look.sticker_palette.set nosuch", "inspecting", "graphite.b"),
    ("wait 653 ms", "inspecting", "graphite.b"),
    ("command look.scene.set solving", "solving", "graphite.b"),
    ("wait 654 ms", "solving", "graphite.b"),
]


def expect_scene_look(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """Check the exports, their notices and the state and style digests on every step."""
    written = [dump.get("do") for dump in dumps]
    if written != [step for step, _, _ in SCENE_LOOK_STEPS]:
        return [f"the steps written were {written}"]
    theme = dumps[0].get("scene_look", {}).get("theme")
    if theme not in ("graphite", "paper"):
        return [f"unknown opening scene theme {theme!r}"]
    looks = load(DESIGN / "themes" / f"{theme}.json")["sceneLooks"]
    scene_export, palette_export = "m6h_set_scene_look", "m6h_set_sticker_colours"
    head_keys = ("state_digest", "revision")
    style_keys = ("outline_sha256", "fill_sha256", "palette_sha256")
    head = tuple(dumps[0].get("tuning", {}).get(key) for key in head_keys)
    style = tuple(dumps[0].get("tint", {}).get(key) for key in style_keys)
    problems = []
    if not head[0] or type(head[1]) is not int or head[1] < 0:
        problems.append("the opening state digest or revision is missing")
    if not style[0] or style[1] is None or not style[2]:
        problems.append("the opening tint style digests are missing")

    backgrounds = {}
    table_digests = {}
    for dump, (step, look, palette_id) in zip(dumps, SCENE_LOOK_STEPS):
        scene, palette = dump.get("scene_look", {}), dump.get("sticker_palette", {})
        if (scene.get("theme"), scene.get("look")) != (theme, look):
            problems.append(f"after {step!r} the scene should show {theme} {look}")
        if palette.get("id") != palette_id or palette.get("passes") is not True:
            problems.append(f"after {step!r} the passing palette should be {palette_id}")
        for key in ("fog", "glow", "edgeBrightness", "edgeWeight"):
            if scene.get(key) != looks[look][key]:
                problems.append(f"after {step!r} scene {key} differs from {theme} {look}")
        background = scene.get("background")
        if not re.fullmatch(r"#[0-9a-f]{6}", str(background)) or backgrounds.setdefault(look, background) != background:
            problems.append(f"after {step!r} the {look} background is invalid or changed")
        for values, export in ((scene, scene_export), (palette, palette_export)):
            if values.get("sent") is not True or values.get("export") != export or type(values.get("status")) is not int or values["status"] != 0:
                problems.append(f"after {step!r} the dump should report a successful {export}")
        if scene.get("drawn") != ["background"] or scene.get("held") != ["fog", "glow", "edgeBrightness", "edgeWeight"]:
            problems.append(f"after {step!r} the dump should show the background drawn and the other scene values held")
        table = "core default" if palette_id == "graphite.a" else palette_id
        digest = palette.get("table_sha256")
        if palette.get("table") != table:
            problems.append(f"after {step!r} the exported sticker table should be {table}")
        if table == "core default":
            if "table_sha256" in palette:
                problems.append(f"after {step!r} the default sticker colours should have no table digest")
        elif not re.fullmatch(r"[0-9a-f]{64}", str(digest)) or table_digests.setdefault(table, digest) != digest:
            problems.append(f"after {step!r} the sticker table digest is invalid or changed")
        if tuple(dump.get("tuning", {}).get(key) for key in head_keys) != head:
            problems.append(f"after {step!r} the state digest or revision changed")
        if tuple(dump.get("tint", {}).get(key) for key in style_keys) != style:
            problems.append(f"after {step!r} the tint style bytes changed")

        if step == "command look.sticker_palette.set nosuch":
            if not any(line.startswith("Unknown sticker palette 'nosuch'.") for line in texts(dump, "frame")):
                problems.append(f"after {step!r} the status line should explain the refusal")
        elif step == "chord Enter" or step.startswith("command look.scene.set "):
            what, export = ("Graphite B sticker palette", palette_export) if step == "chord Enter" else ("Scene look " + look, scene_export)
            notice = f"{what} sent through {export}"
            if notice not in texts(dump, "frame"):
                problems.append(f"after {step!r} the status line should show the completed export")
    if len(set(backgrounds.values())) != 3:
        problems.append("the three scene looks should have distinct backgrounds")
    return problems


EXPECT["scene-look"] = expect_scene_look
ON_FIXTURE.add("scene-look")


def key_set_changes(preset: str) -> int:
    """How many keys a keymap preset gives another meaning: a preset binding replaces the default's first binding on
    its key and context (Keymap.Parse), and the same command with the same argument is the same meaning."""
    first: dict[tuple[str, str], dict] = {}
    for binding in json.loads((DESIGN / "keymap.json").read_text(encoding="utf-8"))["bindings"]:
        first.setdefault((binding["key"], binding["context"]), binding)
    changed = 0
    for binding in json.loads((DESIGN / "keymaps" / f"{preset}.json").read_text(encoding="utf-8"))["bindings"]:
        was = first.get((binding["key"], binding["context"]))
        now = None if binding.get("command") == "" else binding
        meaning = [None if b is None else (b["command"], b.get("argument")) for b in (was, now)]
        changed += meaning[0] != meaning[1]
    return changed


def expect_key_map(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """ADR-D01.74 outside sample mode: F1 keeps ADR-D02.12's route (the palette) and leaves the key map hidden; Shift+/
    over Local pins the key map, whose header names the key set and the place; the Key set row says how many keys it
    gave another meaning, as many as the keymap files say, and the map marks them until it has been closed; an unknown
    set is refused with every set's name; no name takes the next set; Escape lets the map go."""
    steps = ["start", "wait 2500 ms", "chord F1", "chord Escape", "hover LocalFrame", "chord Shift+Slash",
             "command keymap.preset.set left-hand", "chord Escape", "chord Shift+Slash", "command keymap.preset.set default",
             "command keymap.preset.set right-hand", "command keymap.preset.set", "chord Escape", "wait 6000 ms"]
    if [dump["do"] for dump in dumps] != steps:
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    problems = []
    presets = sorted(path.stem for path in (DESIGN / "keymaps").glob("*.json"))
    changed = key_set_changes("left-hand")
    label = {row["id"]: row["label"] for row in source["table"] + source["rows"]}["keymap.preset.set"]
    mark = "corner mark: changed meaning with the last key set change"

    def shown(i: int) -> list[str]:
        return texts(dumps[i], "keymap")

    def header(i: int) -> str:
        return next((line for line in shown(i) if line.startswith("Key set: ")), "")

    for i in (0, 1, 2, 3, 4, 7, 12):
        if shown(i):
            problems.append(f"step {i}: the key map shows")
    if not overlay(dumps[2]):
        problems.append("step 2: F1 did not open the palette")
    if overlay(dumps[3]):
        problems.append("step 3: Escape left an overlay open")
    if not header(5).startswith("Key set: default · ") or "pointer over Local" not in header(5):
        problems.append(f"step 5: the pinned key map's header reads {header(5)!r}")
    for i, key_set in ((6, "left-hand"), (9, "default"), (11, "left-hand")):
        said = f"Key set: {key_set} · {changed} {'key changed' if changed == 1 else 'keys changed'} meaning: "
        if not any(line.startswith(said) for line in texts(dumps[i], "frame")):
            problems.append(f"step {i}: the status line does not begin {said!r}")
        if not header(i).startswith(f"Key set: {key_set} · "):
            problems.append(f"step {i}: the key map's header reads {header(i)!r}")
        if not any(mark in line for line in shown(i)):
            problems.append(f"step {i}: the key map's legend does not name the changed keys' mark")
    if not header(8).startswith("Key set: left-hand · ") or any(mark in line for line in shown(8)):
        problems.append("step 8: the key map shown again should name left-hand and mark no key")
    refusal = f"{label}: no key set is named right-hand; the key sets are {', '.join(['default'] + presets)}"
    if refusal not in texts(dumps[10], "frame"):
        problems.append(f"step 10: the status line does not read {refusal!r}")
    print(f"key-map: left-hand changes {changed} keys; sets {['default'] + presets}")
    return problems


EXPECT["key-map"] = expect_key_map


def expect_radial_hold(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """ADR-D01.20 outside sample mode: W held opens the workspace menu, whose hub says release runs nothing; pointed at
    its third entry the hub names it, and release runs it (the layout changes); held again and released in the hub,
    nothing runs and the status line says so; a tap of W leaves the menu open and 2 runs the second entry; V held over
    Local opens the view menu, and release on its first entry runs that row as the palette would, refusal included."""
    steps = ["start", "wait 2500 ms", "hold W", "wait 500 ms", "hover RadialEntry3", "release W", "wait 800 ms",
             "hold W", "wait 500 ms", "hover RadialHub", "release W", "chord W", "chord 2", "wait 800 ms",
             "hover LocalFrame", "hold V", "wait 500 ms", "hover RadialEntry1", "release V", "wait 6000 ms"]
    if [dump["do"] for dump in dumps] != steps:
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    problems = []
    keymap = json.loads((DESIGN / "keymap.json").read_text(encoding="utf-8"))
    labels = {row["id"]: row["label"] for row in source["table"] + source["rows"]}
    workspaces = keymap["radials"]["radial.workspace"]["entries"]
    views = keymap["radials"]["radial.view"]["entries"]

    def entry_label(entry: dict) -> str:
        return entry.get("label") or labels[entry["command"]]

    def radial(i: int) -> list[str]:
        return texts(dumps[i], "overlay:RadialOverlay")

    def hint(i: int) -> str:
        return next((line for line in radial(i) if line.startswith("Release ")), "")

    def layout(i: int) -> str:
        return json.dumps({key: value for key, value in dumps[i]["frame"].items() if key != "min"}, sort_keys=True)

    for i in (2, 3, 7, 8):
        if hint(i) != "Release here: nothing runs":
            problems.append(f"step {i}: the held menu's hub reads {hint(i)!r}")
    if keymap["radials"]["radial.workspace"]["label"] not in radial(2):
        problems.append(f"step 2: the workspace menu is not open: {radial(2)}")
    third = entry_label(workspaces[2])
    if hint(4) != f"Release runs {third}":
        problems.append(f"step 4: the hub over the third entry reads {hint(4)!r}")
    for i in (5, 10, 12, 18):
        if radial(i):
            problems.append(f"step {i}: the radial menu is still open")
    if layout(5) == layout(1):
        problems.append("step 5: releasing on the third entry left the layout as it was")
    if hint(9) != "Release here: nothing runs":
        problems.append(f"step 9: the hub under the pointer reads {hint(9)!r}")
    let = f"{keymap['radials']['radial.workspace']['label']}: released in the middle, nothing ran"
    if let not in texts(dumps[10], "frame"):
        problems.append(f"step 10: the status line does not read {let!r}")
    if layout(10) != layout(5):
        problems.append("step 10: a release in the hub changed the layout")
    if not radial(11) or hint(11):
        problems.append(f"step 11: a tap should leave the menu open with no release line: {radial(11)}")
    if layout(12) == layout(10):
        problems.append("step 12: 2 on the open menu left the layout as it was")
    first = entry_label(views[0])
    said = hint(17)
    if not said.startswith(f"Release runs {first}"):
        problems.append(f"step 17: the view menu's hub over its first entry reads {said!r}")
    if " (unavailable: " in said:
        reason = said.split(" (unavailable: ", 1)[1].rstrip(")")
        refusal = f"{first}: {reason}"
        if refusal not in texts(dumps[18], "frame"):
            problems.append(f"step 18: the status line does not read {refusal!r}, as the palette row would")
    print(f"radial-hold: third workspace {third!r}; view entry {said!r}")
    return problems


EXPECT["radial-hold"] = expect_radial_hold


BINDING_STEPS = ["start", "wait 2000 ms", "command file.document.open mid.sample-log",
                 "command file.document.open bad-hash.sample-log", "command file.document.open census.6cs",
                 "command workspace.set layout:analyse", "wait 3000 ms"]


def expect_binding_chip(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """ADR-D01.42 in sample mode: Solve shows no free view, so no chip; the verified sample log opens as a free view
    whose head names its source, saved, and its body no longer repeats it; the log with a bad hash and the census script
    carry no chip; Analyse shows the preview view, whose head names its source, preview."""
    if [dump["do"] for dump in dumps] != BINDING_STEPS:
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    problems = []

    def chips(i: int) -> list[tuple[str, str]]:
        return [(group, text) for group, text in map(tuple, dumps[i]["lines"]) if group.startswith("binding:")]

    wanted = {0: [], 1: [], 2: [("binding:document:mid.sample-log", "saved")], 3: [], 4: [],
              5: [("binding:preview", "preview")], 6: [("binding:preview", "preview")]}
    for i, chip in wanted.items():
        if chips(i) != chip:
            problems.append(f"step {i}: the binding chips read {chips(i)!r}, not {chip!r}")
    for i, active in ((2, "mid.sample-log"), (3, "bad-hash.sample-log"), (4, "census.6cs")):
        if dumps[i].get("documents", {}).get("active") != active:
            problems.append(f"step {i}: the active document is {dumps[i].get('documents', {}).get('active')!r}")
    if "saved" in texts(dumps[2], "frame"):
        problems.append("step 2: the log's body still repeats its binding")
    print(f"binding-chip: {[chips(i) for i in range(len(dumps))]}")
    return problems


EXPECT["binding-chip"] = expect_binding_chip
IN_SAMPLE_MODE.add("binding-chip")


RAIL_LOCK_STEPS = ["start", "wait 2000 ms", "click CheckChip", "wait 800 ms", "click CheckChip", "wait 3000 ms"]


def expect_rail_locks(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """ADR-D01.65 in sample mode: the rail carries the locks, and each lock names its kind (whole orbit, positions,
    identities, a block), never partial or safe; the check-mode chip shows Net or Strict with passed, failed or
    unchecked, and a click on it changes the mode, a second click back."""
    if [dump["do"] for dump in dumps] != RAIL_LOCK_STEPS:
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    problems = []
    lock_line = re.compile(r"^O\d\d lock: (whole orbit|(positions|identities|a block)(, (identities|a block))*)$")
    for i in (1, 5):
        locks = [line for text in texts(dumps[i], "rail") for line in text.splitlines()]
        if not locks:
            problems.append(f"step {i}: the rail spells no lock")
        bad = [line for line in locks if not lock_line.match(line)]
        if bad:
            problems.append(f"step {i}: lock lines that name no kind: {bad[:3]}")
        words = " ".join(locks)
        for kind in ("whole orbit", "positions", "identities", "a block"):
            if kind not in words:
                problems.append(f"step {i}: no lock names {kind!r}")
        if "partial" in words or "safe" in words:
            problems.append(f"step {i}: a lock reads partial or safe")
    dot = " · "
    chip = re.compile("^(Net|Strict)" + dot + "(passed|failed|unchecked)$")

    def mode(i: int) -> str | None:
        found = [text for text in texts(dumps[i], "frame") if chip.match(text)]
        return found[0].split(" ")[0] if len(found) == 1 else None

    modes = [mode(i) for i in (1, 3, 5)]
    if None in modes:
        shown = [[t for t in texts(dumps[i], "frame") if dot in t] for i in (1, 3, 5)]
        problems.append(f"the check-mode chip reads {shown}")
    elif not (modes[0] != modes[1] and modes[2] == modes[0]):
        problems.append(f"the chip's mode went {modes}, not there and back")
    print(f"rail-locks: modes {modes}; locks {[line for text in texts(dumps[1], 'rail') for line in text.splitlines()][:6]}")
    return problems


EXPECT["rail-locks"] = expect_rail_locks
IN_SAMPLE_MODE.add("rail-locks")


def expect_status_patterns(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """Check the status channel's bytes, the table and the rasterized piece masks from the two panes."""
    import base64
    import math
    import zlib

    script = load(SCRIPTS / "status-patterns.json")
    design = load(DESIGN / "status-patterns.json")
    fixture = load(SAMPLES / "status-fixture.json")
    if state != fixture["state"]:
        return [f"status-patterns needs the {fixture['state']} session fixture"]
    if len(dumps) != len(script["steps"]) + 1 or dumps[0].get("do") != "start":
        return ["status-patterns did not write one dump per step"]

    piece_count = 177_120
    expected = {
        "statuses": (0, ["protected", "conflict", "unchecked_prefix", "preview", "stale"], "patterns"),
        "kinds": (3, ["orbit", "position", "identity", "block"], "marks"),
    }
    by_step = {}
    problems = []
    digest = dumps[0].get("tuning", {}).get("state_digest")
    for dump in dumps:
        step = dump.get("do")
        status = dump.get("status_patterns", {})
        by_step[step] = (status, dump)
        if not digest or status.get("state_digest") != digest or dump.get("tuning", {}).get("state_digest") != digest:
            problems.append(f"{step}: state digest missing or changed")
        if (not status.get("sent") or status.get("channel") != 3 or status.get("panes") != 2
                or not dump.get("local", {}).get("opening")):
            problems.append(f"{step}: status table/bytes or the two panes were not submitted")
        if status.get("draw_code") != 0:
            problems.append(f"{step}: no raster coverage readback")
        try:
            sent = zlib.decompress(base64.b64decode(status["bytes_zlib"], validate=True))
        except (KeyError, ValueError, zlib.error):
            problems.append(f"{step}: the sent bytes are missing or invalid")
            continue
        if len(sent) != piece_count or hashlib.sha256(sent).hexdigest() != status.get("bytes_sha256"):
            problems.append(f"{step}: the sent byte length or digest differs")
        if any(b & 0xc0 or (b & 7) > 5 or ((b >> 3) & 7) > 4 for b in sent):
            problems.append(f"{step}: the status channel has reserved bits or codes")
        for group, (shift, names, table_name) in expected.items():
            ids_by_code = [[] for _ in range(8)]
            for piece, byte in enumerate(sent):
                if byte:
                    ids_by_code[(byte >> shift) & 7].append(piece)
            for code, name in enumerate(names, 1):
                entry = status.get(group, {}).get(name, {})
                ids = ids_by_code[code]
                if (entry.get("code"), entry.get("count"), entry.get("pieces")) != (code, len(ids), ids[:32]):
                    problems.append(f"{step}: {name} piece IDs do not match the bytes sent")
                if entry.get("source") not in ("fed", "fixture"):
                    problems.append(f"{step}: {name} has no source")
                for pane in ("global", "local"):
                    count = entry.get(pane)
                    if type(count) is not int or not 0 <= count <= len(ids):
                        problems.append(f"{step}: invalid {name} coverage in {pane}")
                sent_pattern = entry.get("pattern", {})
                wanted = design[table_name][name]
                if (sent_pattern.get("shape"), sent_pattern.get("token")) != (wanted["shape"], wanted["colour"]):
                    problems.append(f"{step}: {name} shape or token differs from the design")
                elif any(not isinstance(sent_pattern.get(k), (int, float)) or
                         not math.isclose(sent_pattern[k], wanted[k], rel_tol=1e-6, abs_tol=1e-7)
                         for k in ("spacing", "width", "angle")):
                    problems.append(f"{step}: {name} geometry differs from the design")
                rgba = sent_pattern.get("rgba", [])
                if len(rgba) != 4 or any(not 0 <= c <= 1 for c in rgba):
                    problems.append(f"{step}: {name} table has no resolved RGBA")

    def both(row):
        return all(row.get(key, 0) > 0 for key in ("count", "global", "local"))

    for index, kind in enumerate(("orbit", "block", "position", "identity")):
        step = f"wait {701 + index} ms"
        status, dump = by_step.get(step, ({}, {}))
        mark = status.get("kinds", {}).get(kind, {})
        protected = status.get("statuses", {}).get("protected", {})
        selection = dump.get("pick", {}).get("selection") or {}
        if (status.get("command"), status.get("command_code")) != ("protection.pick.set " + kind, 0) or not selection:
            problems.append(f"{step}: {kind} was not set from a pick")
        if mark.get("source") != "fed" or not both(mark) or protected.get("source") != "fed" or not both(protected):
            problems.append(f"{step}: {kind} and its protected pattern need pieces drawn in both panes")
        if kind in ("position", "identity") and selection.get("piece") not in mark.get("pieces", []):
            problems.append(f"{step}: {kind} did not mark the picked occupant")

    for elapsed, command, enabled in [(705, "status.sample.show on", True),
                                      (706, "protection.pick.set clear", True),
                                      (707, "status.sample.show off", False)]:
        step = f"wait {elapsed} ms"
        status, dump = by_step.get(step, ({}, {}))
        if (status.get("command"), status.get("command_code"), status.get("sample")) != (command, 0, enabled):
            problems.append(f"{step}: {command} did not complete")
        if enabled and (status.get("fixture_state") != state or dump.get("local", {}).get("framed") != fixture["cap"]):
            problems.append(f"{step}: the sample is not in Local's default cap")
        if elapsed >= 706 and any(mark.get("count") for mark in status.get("kinds", {}).values()):
            problems.append(f"{step}: a protection mark survived clear")
        if elapsed == 706:
            for name, ids in fixture["statuses"].items():
                row = status.get("statuses", {}).get(name, {})
                if row.get("source") != "fixture" or row.get("pieces") != sorted(ids) or not both(row):
                    problems.append(f"{step}: {name} needs its fixture pieces drawn in both panes")
        if elapsed == 707 and status.get("bytes_sha256") != hashlib.sha256(bytes(piece_count)).hexdigest():
            problems.append(f"{step}: a status byte survived clear and off")
    return problems


EXPECT["status-patterns"] = expect_status_patterns
ON_FIXTURE.add("status-patterns")


FRAME_LAYER_HEADINGS = ("Surfaces · free to repaint", "Instruments · meaning fixed by name; a change marks the views")


def expect_frame_layers(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """ADR-D01.77 on wj-ia-300's session fixture: the frame is a canvas in two layers. Recolouring an instrument (the
    rail's whole-orbit lock) marks both views and lists the role under the Instruments heading; its reset clears the
    mark. Recolouring two surfaces (the shell and the completion moment) repaints the window colour, lists both under
    the Surfaces heading and marks no view. The resets bring the opening token digest back; the state digest holds."""
    steps = ["start", "wait 5001 ms", "command tuning.open", "wait 701 ms",
             "command token.override.set rail.lock.whole=0.62 0.16 40", "wait 702 ms",
             "command token.override.reset rail.lock.whole=reset", "wait 703 ms",
             "command token.override.set surface.window=0.2 0.01 250", "wait 704 ms",
             "command token.override.set rail.moment.complete=0.6 0.15 140", "wait 705 ms",
             "command token.override.reset surface.window=reset", "command token.override.reset rail.moment.complete=reset",
             "wait 3000 ms"]
    if [dump["do"] for dump in dumps] != steps:
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    problems = []
    labels = {row["id"]: row["label"] for row in source["table"] + source["rows"]}
    surfaces, instruments = FRAME_LAYER_HEADINGS

    opening = dumps[0].get("colours", {}).get("digest")
    if not re.fullmatch(r"[0-9a-f]{12}", str(opening)):
        problems.append("step 0: the opening token digest is missing or malformed")
    # (step, the views marked, the digest moved): only the instrument's override marks the views.
    for i, marked, moved in ((3, False, False), (5, True, True), (7, False, False), (9, False, True), (11, False, True),
                             (14, False, False)):
        colours = dumps[i].get("colours", {})
        if (colours.get("nonstandard"), colours.get("global"), colours.get("local")) != (marked, int(marked), int(marked)):
            problems.append(f"step {i}: both view frames should carry {int(marked)} colour marks, not "
                            f"{colours.get('global')} and {colours.get('local')} (non-standard {colours.get('nonstandard')})")
        if (colours.get("digest") != opening) != moved:
            problems.append(f"step {i}: the token digest {'did not move' if moved else 'moved'}")

    def rows(i: int) -> list[str]:
        return texts(dumps[i])

    def under(lines: list[str], text: str, heading: str, before: str | None = None) -> bool:
        if heading not in lines or text not in lines:
            return False
        at, start = lines.index(text), lines.index(heading)
        return at > start and (before is None or before not in lines or at < lines.index(before))

    if not under(rows(5), "rail.lock.whole *", instruments):
        problems.append("step 5: the recoloured lock is not listed under the Instruments heading")
    if not all(heading in rows(5) for heading in FRAME_LAYER_HEADINGS):
        problems.append("step 5: the Advanced list does not show both layers' headings")
    if "rail.lock.whole *" in rows(7):
        problems.append("step 7: the lock's reset left it marked as changed")
    for text in ("rail.moment.complete *", "surface.window *"):
        if not under(rows(11), text, surfaces, instruments):
            problems.append(f"step 11: '{text}' is not listed under the Surfaces heading, before the Instruments one")
    if any(text in rows(14) for text in ("rail.moment.complete *", "surface.window *")):
        problems.append("step 14: a surface's reset left it marked as changed")

    tuning = {i: dumps[i].get("tuning") or {} for i in range(3, 15)}
    if not tuning[3].get("window") or tuning[9].get("window") == tuning[7].get("window"):
        problems.append("step 9: recolouring the shell did not repaint the frame's window colour")
    if tuning[14].get("window") != tuning[7].get("window"):
        problems.append("step 14: the shell's reset did not bring the window colour back")
    digest = tuning[3].get("state_digest")
    if not digest:
        problems.append("the head's state digest is not read")
    for i, t in tuning.items():
        if t.get("state_digest") != digest:
            problems.append(f"step {i}: the state digest moved")
        if t.get("status"):
            problems.append(f"step {i}: the tuning panel refused: {t.get('status')!r}")
    for i in (4, 6, 8, 10, 12, 13):
        command = dumps[i]["do"].split(" ")[1]
        problems.extend(f"step {i}: {line}" for line in texts(dumps[i], "frame") if line.startswith(labels[command] + ": "))
    if not problems:
        print(f"frame-layers: the lock marked both views and sat under Instruments; the shell and the completion moment "
              f"sat under Surfaces and marked nothing; the token digest {opening} came back; the state digest "
              f"{str(digest)[:16]} held")
    return problems


EXPECT["frame-layers"] = expect_frame_layers
ON_FIXTURE.add("frame-layers")


def window_problems(dumps: list[dict], foreground: bool = False) -> list[str]:
    """The owner's window rule as the shell itself reports it in every dump (a --foreground run: never topmost)."""
    problems = []
    for dump in dumps:
        window = dump.get("window", {})
        if window.get("topmost") is not False:
            problems.append(f"step {dump['step']}: the shell's window is topmost")
        if not foreground and window.get("foreground") is not False:
            problems.append(f"step {dump['step']}: the shell's window has the foreground")
    return problems


class Desktop:
    """The foreground window and a process's windows, through user32 (Windows only)."""

    def __init__(self):
        import ctypes
        from ctypes import wintypes
        self.ctypes, self.wintypes = ctypes, wintypes
        self.user32 = ctypes.WinDLL("user32")
        self.callback = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
        self.user32.GetForegroundWindow.restype = wintypes.HWND
        self.user32.EnumWindows.argtypes = (self.callback, wintypes.LPARAM)
        self.user32.GetWindowThreadProcessId.argtypes = (wintypes.HWND, ctypes.POINTER(wintypes.DWORD))
        self.user32.GetWindowLongPtrW.argtypes = (wintypes.HWND, ctypes.c_int)
        self.user32.GetWindowLongPtrW.restype = ctypes.c_ssize_t
        self.user32.IsWindowVisible.argtypes = (wintypes.HWND,)
        self.user32.GetClassNameW.argtypes = (wintypes.HWND, wintypes.LPWSTR, ctypes.c_int)

    def foreground(self) -> int:
        return int(self.user32.GetForegroundWindow() or 0)

    def windows_of(self, pid: int) -> list[int]:
        found = []

        def visit(window, _):
            owner = self.wintypes.DWORD()
            self.user32.GetWindowThreadProcessId(window, self.ctypes.byref(owner))
            if owner.value == pid and self.user32.IsWindowVisible(window):
                found.append(int(window))
            return True

        self.user32.EnumWindows(self.callback(visit), 0)
        return found

    def topmost(self, window: int) -> bool:
        return bool(self.user32.GetWindowLongPtrW(window, -20) & 0x8)  # GWL_EXSTYLE, WS_EX_TOPMOST

    def describe(self, window: int, pid: int | None) -> dict:
        """A window's class, whether the shell's process owns it, and whether it is topmost; never its title."""
        if not window:
            return {}
        name = self.ctypes.create_unicode_buffer(256)
        self.user32.GetClassNameW(window, name, 256)
        owner = self.wintypes.DWORD()
        self.user32.GetWindowThreadProcessId(window, self.ctypes.byref(owner))
        return {"class": name.value, "shell": pid is not None and owner.value == pid, "topmost": self.topmost(window)}


def run(script: str, out: Path, build: Path | None, state: str | None, desktop=None, launcher=launch.main,
        clock=time.monotonic, sleep=time.sleep, path: Path | None = None, foreground=None) -> tuple[int, dict]:
    """Launches the run, watches the foreground, and compares the screens. Returns the exit code and the record.
    A driver that writes its own script (track_j.py, B32) passes its path; a script of d02_scripts/ is found by name.
    `foreground` (gate_d02.py's --foreground, the owner's sitting only) is the gate's own activation of a window
    (tools/perf/d02/gate_row.py): the shell is not told to hand the foreground back, its window is activated once at
    the start, and the foreground checks give way to that; the topmost checks stay. This file activates nothing."""
    path = path or SCRIPTS / f"{script}.json"
    source = data()
    if script not in EXPECT or not path.is_file():
        raise ValueError(f"no D02 script named {script} (renderer/shell/d02_scripts/)")
    written = json.loads(path.read_text(encoding="utf-8"))
    if state is None:
        state = written.get("state")
    if state is not None and state not in source["states"]:
        raise ValueError(f"no sample state named {state}")
    fixture = FIXTURES / state if state else None
    if script in ON_FIXTURE and (fixture is None or not (fixture / "fixture.json").is_file()):
        raise ValueError(f"the {script} script needs a written session fixture of its state "
                         "(renderer/shell/build_d02_fixtures.py --state ID)")
    desktop = desktop or Desktop()
    started: dict = {}
    lines: list[str] = []

    def popen(arguments, cwd, env, **options):
        if foreground:
            env = {key: value for key, value in env.items() if key != "M600_D02_FOREGROUND"}
        process = subprocess.Popen(arguments, cwd=cwd, env=env, **options)
        started["process"], started["at"] = process, clock()
        return process

    # A script may name its puzzle (step C-.40: base-local runs the base puzzle, Jumble off); the default is Jumble on.
    arguments = ["--out", str(out), "--frame", "--puzzle", written.get("puzzle", "jumble-on"), "--d02", "--d02-script", str(path)]
    arguments += ["--sample"] if script in IN_SAMPLE_MODE else []
    # A script may name the window's size (the fit scripts: the owner's 2000x1200 and a smaller 1280x720).
    resolution = written.get("resolution")
    arguments += ["--resolution", resolution] if resolution else []
    arguments += ["--build", str(build)] if build else []
    arguments += ["--d02-state", state] if state else []
    arguments += ["--d02-fixture", str(fixture)] if fixture is not None and (fixture / "fixture.json").is_file() else []
    arguments += ["--d02-data", str(DATA_FOR[script](out))] if script in DATA_FOR else []  # the drawing track's (B22)
    result: dict = {}
    before = desktop.foreground()
    thread = threading.Thread(target=lambda: result.update(code=launcher(arguments, popen=popen, out_stream=lines.append)),
                              daemon=True)
    thread.start()
    while "at" not in started and thread.is_alive():
        sleep(0.1)
    after, topmost, seen = None, None, {}
    describe = getattr(desktop, "describe", lambda window, pid: {})
    took = None
    if "at" in started:
        while foreground and took is None and clock() - started["at"] < FOREGROUND_AFTER_S and thread.is_alive():
            shown = desktop.windows_of(started["process"].pid)
            if shown:
                took = bool(foreground(shown[0]))
            else:
                sleep(0.1)
        while clock() - started["at"] < FOREGROUND_AFTER_S and thread.is_alive():
            sleep(0.1)
        after = desktop.foreground()
        pid = started["process"].pid
        windows = desktop.windows_of(pid)
        topmost = any(desktop.topmost(window) for window in windows)
        seen = {"before": describe(before, pid), "after": describe(after, pid),
                "shell_windows": [describe(window, pid) for window in windows]}
    thread.join(RUN_TIMEOUT_S)
    problems = []
    if thread.is_alive():
        subprocess.run(["taskkill", "/T", "/F", "/PID", str(started["process"].pid)], capture_output=True)
        problems.append(f"the shell ran longer than {RUN_TIMEOUT_S:.0f} s and was stopped")
        thread.join(30)
    if result.get("code") != 0:
        problems.append(f"the launch ended with {result.get('code')}: {' | '.join(lines[-3:])}")
    if foreground:
        if not took:
            problems.append("the shell's window did not take the foreground at the start (--foreground): start the "
                            "run from a PowerShell in front on the desktop")
    elif after is None or after != before:
        problems.append("the foreground changed during the first five seconds" if after is not None
                        else "the shell did not start")
    if topmost:
        problems.append("the shell's window carries WS_EX_TOPMOST")
    dumps = []
    screen = out / SCREEN
    if screen.is_file():
        dumps = [json.loads(line) for line in screen.read_text(encoding="utf-8").splitlines() if line.strip()]
    else:
        problems.append("the shell wrote no d02-screen.jsonl")
    if dumps:
        problems += window_problems(dumps, foreground is not None)
        problems += EXPECT[script](dumps, source, state)
    if script in EVENT_CHECKS:
        problems += EVENT_CHECKS[script](out / "events.jsonl")
    if script in FOLDER_CHECKS:
        problems += FOLDER_CHECKS[script](out)
    record = {"format": FORMAT, "script": script, "state": state, "steps": len(dumps), "mismatches": problems,
              "foreground_kept": after is not None and after == before, "topmost": bool(topmost), "windows": seen,
              "foreground_taken": took if foreground is not None else None,
              "gave_back": max((dump.get("window", {}).get("gave_back", 0) for dump in dumps), default=0)}
    if out.is_dir():
        (out / RECORD).write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8", newline="\n")
    return (0 if not problems else 1), record


def steady_state(dumps: list[dict], name: str) -> list[str]:
    """C3/C4 finding F5 (B23's check): a view-only run changes the session at no step: the bound state's digest and
    revision, and the Word panel's digest line, are the same in every dump that carries them."""
    bound = [dump["bound"] for dump in dumps if isinstance(dump.get("bound"), dict)]
    if len(bound) != len(dumps):
        return [f"{name}: {len(dumps) - len(bound)} of {len(dumps)} dumps carry no bound state"]
    problems = []
    pairs = {(row["digest"], row["revision"]) for row in bound}
    if len(pairs) != 1 or any(revision < 0 or not digest for digest, revision in pairs):
        problems.append(f"{name}: the bound state changed or was never bound: {sorted(pairs)}")
    words = {dump["word"]["digest"] for dump in dumps if isinstance(dump.get("word"), dict) and "digest" in dump["word"]}
    if len(words) > 1:
        problems.append(f"{name}: the Word panel's digest changed: {sorted(words)}")
    print(f"{name}: steady state {sorted(pairs)}, Word digests {len(words)}")
    return problems


def with_steady_state(name: str, expect):
    def run(dumps: list[dict], source: dict, state: str | None) -> list[str]:
        return steady_state(dumps, name) + expect(dumps, source, state)
    run.__wrapped__ = expect
    return run


for _name in ("hopf", "ring-sheet", "slice", "raw"):
    EXPECT[_name] = with_steady_state(_name, EXPECT[_name])


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--script", required=True, help="a script of renderer/shell/d02_scripts/, without .json")
    parser.add_argument("--out", type=Path, required=True, help="the run folder, which must not exist yet")
    parser.add_argument("--build", type=Path, help="the product build (default: the newest)")
    parser.add_argument("--state", help="the sample state the D02 surfaces read")
    args = parser.parse_args(argv)
    try:
        code, record = run(args.script, args.out.resolve(), args.build, args.state)
    except (ValueError, OSError) as error:
        print(f"d02_run: {error}")
        return 2
    if code == 0:
        print(f"M600_D02_RUN_PASS script={args.script} steps={record['steps']} mismatches=0")
    else:
        print(f"M600_D02_RUN_FAIL script={args.script} steps={record['steps']} mismatches={len(record['mismatches'])}")
        for problem in record["mismatches"]:
            print("  " + problem)
    return code



# ---- ADR-D02 B19 and B24: the gate rows' scripts (renderer/shell/gate_d02.py) ------------------------------------------
# A gate script holds its views and repeats its motion for the row's time. The player (ShellJumble.cs) plays a flat list
# of steps, so the one step {"repeat": {"steps": [...]}} of a gate script is expanded here before the launch: its steps,
# copied until their waits add up to the asked seconds, each marked "quiet" (the player writes no screen dump after a
# quiet step, so the timed frames carry no dump's work); the last copy is not quiet, so the run still ends on a dump.
# ADR-D01's Local row (view.local): the Local hide that the gate's G1 runs with (B19).
LOCAL_HIDE_STEPS = ["start", "wait 4000 ms", "command view.local", "wait 1500 ms", "drag 160 0", "wait 701 ms",
                    "command view.local", "wait 1502 ms"]


def expect_local_hide(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """The row hides Local (its opening closed, the cap it frames kept), a drag in Global turns the camera while
    Local is hidden, and the row shows Local again on the same cap, linked and sent the shared spin."""
    steps = [dump["do"] for dump in dumps]
    if steps != LOCAL_HIDE_STEPS:
        return [f"the steps written were {steps}"]
    before, hidden, dragged, shown = dumps[1], dumps[3], dumps[5], dumps[7]
    local = [row.get("local") or {} for row in (before, hidden, shown)]
    problems = []
    if local[0].get("hidden") is not False or local[0].get("opening") is not True:
        problems.append(f"Local was not shown with its opening before the row: {local[0]}")
    if local[1].get("hidden") is not True:
        problems.append(f"the row did not hide Local: {local[1]}")
    if local[2].get("hidden") is not False or local[2].get("framed") != local[0].get("framed"):
        problems.append(f"the row did not show Local again on its cap: {local[0]} then {local[2]}")
    rotations = [row.get("rotation") for row in (hidden, dragged, shown)]
    if any(not isinstance(r, dict) for r in rotations):
        return problems + ["a step's dump has no rotation"]
    if same(rotations[0]["sent_global"], rotations[1]["sent_global"]):
        problems.append("the drag in Global while Local was hidden did not turn Global")
    if not rotations[2]["linked"] or not same(rotations[2]["sent_local"], rotations[2]["global"]["spin"]):
        problems.append("Local came back unlinked or without the shared spin")
    print(f"local-hide: hidden {local[1].get('hidden')}, back on cap {local[2].get('framed')}, linked "
          f"{rotations[2]['linked']}; {len(problems)} problems")
    return problems


def check_local_hide_events(events: Path) -> list[str]:
    """Two drawn panes, then one-view frames while Local is hidden, then two drawn panes again once it is shown."""
    frames = frame_rows(events)
    kinds = [2 if row.get("panes") == 2 and row.get("held") == 0 else 1 if not row.get("panes") else 0 for row in frames]
    runs = [kind for i, kind in enumerate(kinds) if kind and (i == 0 or kind != kinds[i - 1])]
    joined = "".join(str(kind) for kind in runs)
    print(f"local-hide: {len(frames)} frames, pane runs {joined[:40]}")
    return [] if "212" in joined else [f"no two-pane, one-view, two-pane sequence in the frame rows ({joined[:40]})"]


EXPECT["local-hide"] = expect_local_hide
ON_FIXTURE.add("local-hide")
IN_SAMPLE_MODE.add("local-hide")
EVENT_CHECKS["local-hide"] = check_local_hide_events


# A hold-for is a repeat of one wait (a held key: "hold" before the block, "release" after it).
GATE_PREFIX = "gate-"
GATE_OCCLUSION = "gate-occlusion"


def expand_repeat(script: dict, seconds: float) -> dict:
    """The script with its one repeat block unrolled for `seconds` of written waits (at least one copy)."""
    import math
    if seconds <= 0:
        raise ValueError("a repeat block needs a positive number of seconds")
    steps: list[dict] = []
    blocks = 0
    for step in script["steps"]:
        if "repeat" not in step:
            steps.append(step)
            continue
        blocks += 1
        body = step["repeat"].get("steps") or []
        if any("repeat" in inner for inner in body):
            raise ValueError("a repeat block cannot hold another")
        period = sum(int(inner.get("wait_ms", 0)) for inner in body)
        if period <= 0:
            raise ValueError("a repeat block needs a wait_ms step")
        for _ in range(max(1, math.ceil(seconds * 1000.0 / period))):
            steps += [{**inner, "quiet": True} for inner in body]
        steps[-1] = {key: value for key, value in steps[-1].items() if key != "quiet"}
    if blocks != 1:
        raise ValueError(f"a gate script holds exactly one repeat block, not {blocks}")
    return {**script, "steps": steps, "expanded_seconds": seconds}


def expect_gate(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """A gate row compares no screen text: its numbers come from the event log (tools/perf/d02/gate_row.py), and the
    owner's window rule is checked for every run. It asks only that the run reached a dump after its first step."""
    return [] if any(dump.get("step", 0) > 0 for dump in dumps) else ["the gate run wrote no dump after its first step"]


for _gate in sorted(SCRIPTS.glob(GATE_PREFIX + "*.json")):
    EXPECT[_gate.stem] = expect_gate
    ON_FIXTURE.add(_gate.stem)
EXPECT[GATE_OCCLUSION] = expect_gate
ON_FIXTURE.add(GATE_OCCLUSION)


# The restage writes no screen ("quiet"), so the Ctrl+Enter pressed one frame later lands inside the settle interval;
# its screen is written at once ("quick").
PREVIEW_AT = {"base": 9, "staged": 11, "back": 13, "forward": 15, "overlay": 19, "overlay_off": 23,
              "start": 27, "cancel": 29, "refused": 34, "again": 35, "partial": 37, "commit": 39}
SETTLE_REFUSAL = "the preview has just changed"


def expect_preview(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """A stage and a scrub draw a separate pose set. Cancelling keeps the head, and committing takes the whole word.
    Ctrl+Enter on a fresh preview is refused inside the settle interval (ADR-D01.9) with its reason said, while the
    commit bar reads settling with its lamp lit and Commit off (ADR-D01.66, rev4's ADR-D01.11); after the interval the
    bar reads ready again."""
    if len(dumps) <= max(PREVIEW_AT.values()):
        return [f"the preview script wrote {len(dumps)} screens"]
    if state != "wj-ia-300":
        return ["the preview script needs wj-ia-300"]
    screens = {name: dumps[index] for name, index in PREVIEW_AT.items()}
    previews = {name: dump.get("preview", {}) for name, dump in screens.items()}
    problems = []

    def check(condition, problem):
        if not condition:
            problems.append(problem)

    def drawing(preview):
        return (preview.get("host") or {}).get("drawn") or {}

    def panes(preview):
        return {pane["view"]: pane["vertex_evals"] for pane in drawing(preview).get("panes", [])}

    base = previews["base"]
    head = base.get("head", {})
    revision = drawing(base).get("revision")
    check(isinstance(revision, int) and bool(head.get("digest") and head.get("pose_ids_sha256")
            and head.get("pose_lattice_sha256")), "the main pair has no committed state evidence")
    check(not base.get("pane") and not screens["base"].get("stage", {}).get("spans"), "opening the list staged a repair")
    for name, preview in previews.items():
        if name == "commit":
            continue
        check(preview.get("head") == head and drawing(preview).get("revision") == revision
              and preview.get("host", {}).get("head_revision") == revision, f"{name}: the committed revision or pieces changed")
        check(all(panes(preview).get(view, 0) > 0 for view in ("Global", "Local")),
              f"{name}: the host did not draw both main views")
        check(bool(preview.get("overlay")) == (name == "overlay"), f"{name}: the script overlay did not follow its toggle")
        for view in ("Global", "Local"):
            words = texts(screens[name], f"view:{view}")
            check(("Preview pending" in words) == bool(preview.get("id")), f"{name}: the pending chip is wrong in {view}")
            check(("script" in words) == (name == "overlay"), f"{name}: the overlay's script frame is wrong in {view}")
    staged, again = previews["staged"], previews["again"]
    letters = staged.get("letters") or 0
    check(letters == 3 and staged.get("whole_only") and screens["staged"].get("stage", {}).get("spans") == 1,
          "digit 2 did not stage one whole repair with three letters")
    check(bool(staged.get("after")) and staged.get("after") != head.get("digest")
          and len(staged.get("prefix_digests", [])) == letters
          and (staged.get("prefix_digests") or [None])[-1] == staged.get("after"), "the preview has no distinct after-state")
    check(bool(staged.get("id")) and bool(again.get("id")) and again.get("id") != staged.get("id")
          and again.get("after") == staged.get("after"), "restaging did not build a fresh preview of the same repair")
    refused = previews["refused"]
    check(refused.get("id") == again.get("id") and refused.get("status") == "ready" and refused.get("settling") is True
          and refused.get("reason") == SETTLE_REFUSAL, f"Ctrl+Enter on the fresh preview did not meet the settle interval: "
          f"id {refused.get('id')}, settling {refused.get('settling')}, reason {refused.get('reason')!r}")
    check(any(line == SETTLE_REFUSAL or line.endswith(": " + SETTLE_REFUSAL) for line in texts(screens["refused"], "frame")),
          "the refused Ctrl+Enter did not say its reason")
    bar = refused.get("bar") or {}
    check(bar.get("status") == "settling" and bar.get("lamp") is True and bar.get("commit") is False
          and bar.get("commit_tip") == "Commit is off: " + SETTLE_REFUSAL,
          f"the commit bar did not show the settle interval: {bar}")
    for name in ("staged", "again", "partial"):
        bar = previews[name].get("bar") or {}
        check(previews[name].get("settling") is False and previews[name].get("reason") is None
              and bar.get("status") == "ready" and bar.get("lamp") is False and bar.get("commit") is True,
              f"{name}: the commit bar did not read ready after the settle interval: {bar}")
    for name, step in (("staged", letters), ("back", letters - 1), ("forward", letters), ("start", 0),
                       ("again", letters), ("partial", letters - 1)):
        preview = previews[name]
        check(preview.get("status") == "ready" and preview.get("pane")
              and panes(preview).get("Preview after", 0) > 0
              and drawing(preview).get("preview_id") == preview.get("host_id"), f"{name}: the preview pane was not drawn")
        check(preview.get("scrub") == step and drawing(preview).get("prefix") == step,
              f"{name}: the scrub did not draw prefix {step}")
    for name in ("cancel", "commit"):
        preview = previews[name]
        check(not preview.get("pane") and not preview.get("id") and not screens[name].get("stage", {}).get("spans")
              and not any(view.startswith("Preview ") for view in panes(preview)), f"{name}: a preview or a stage was left")
    committed = previews["commit"]
    check(isinstance(revision, int) and committed.get("host", {}).get("head_revision") == revision + letters
          and drawing(committed).get("revision") == revision + letters, "commit did not advance one revision per letter")
    record = committed.get("commit", {})
    check(record.get("letters") == letters and record.get("error") is None
          and committed.get("head", {}).get("digest") == record.get("actual") == record.get("expected") == again.get("after"),
          "the commit's digest differs from the preview's after digest")
    check(not any(dump.get("stage", {}).get("unasked") for dump in dumps), "the boundary ledger counted an unasked act")
    return problems


EXPECT["preview"] = expect_preview
ON_FIXTURE.add("preview")


if __name__ == "__main__":
    sys.exit(main())
