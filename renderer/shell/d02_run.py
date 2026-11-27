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
RECORD = "d02-run.json"
FORMAT = "magic600-d02-run-v1"
FOREGROUND_AFTER_S = 5.0
RUN_TIMEOUT_S = 240.0


def load(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def data() -> dict:
    """What the expectations are computed from: the draft rows, the table's rows and the sample states."""
    return {"rows": load(DESIGN / "command-rows-d02.json")["commands"], "table": load(TABLE)["commands"],
            "states": [state["state"] for state in load(SAMPLES / "states.json")["states"]]}


def palette_matches(query: str, rows: list[dict], table: list[dict]) -> list[str]:
    """The palette's rows for a query, in registry order (a fresh session has no pins or uses): every term in the
    label or the command id (PaletteModel.Rows). The labels shown are the D02 rows' own; table rows keep theirs."""
    terms = query.lower().split()
    found = []
    for row in table + rows:
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
        problems.append(f"step 2: the session refused the twist at {state}: the router turns cap 0 only (grip.select "
                        "is not built), so the bind script needs a state where cap 0 admits a twist, such as on-zero")
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
    if LOCAL_EMPTY in lines:
        problems.append(f"step {step}: Local shows its empty sentence while it draws a cap")
    if named and (dump.get("named") != cap or local.get("words") or any(line.startswith("Local shows cap") for line in lines)):
        problems.append(f"step {step}: cap {dump.get('named')} is named and Local's readout reads {local.get('words')!r}, "
                        f"expected cap {cap} named and no default")
    if not named and (dump.get("named") != -1 or LOCAL_DEFAULT not in lines):
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
    times the opening zoom; a wait changes nothing, the zoom turns no camera, and the core is sent the zoom."""
    if [dump["do"] for dump in dumps] != ZOOM_STEPS:
        return [f"the steps written were {[dump['do'] for dump in dumps]}"]
    zooms = [dump.get("zoom") for dump in dumps]
    if any(not isinstance(zoom, dict) or "sent" not in zoom for zoom in zooms[1:]):
        return ["a step's dump has no zoom sent to the core"]
    problems = []
    base = zooms[1]["sent"]
    if zooms[1]["factor"] != 1 or zooms[1]["notches"] != 0:
        problems.append(f"the view was zoomed before any wheel ({zooms[1]})")
    expected = {2: ZOOM_STEP ** 3, 3: ZOOM_STEP ** 3, 4: ZOOM_STEP ** -2, 5: ZOOM_STEP ** -2, 6: ZOOM_MAX, 7: ZOOM_MAX}
    for index, factor in expected.items():
        zoom = zooms[index]
        if abs(zoom["factor"] - factor) > 1e-6:
            problems.append(f"after '{ZOOM_STEPS[index]}' the zoom factor is {zoom['factor']:.4f}, not {factor:.4f}")
        if abs(zoom["sent"] - base * factor) > 1e-4:
            problems.append(f"after '{ZOOM_STEPS[index]}' the core was sent zoom {zoom['sent']:.4f}, not {base * factor:.4f}")
    if not zooms[2]["sent"] > base:
        problems.append("wheel up zoomed out; it must zoom in (a larger zoom draws a larger picture)")
    if not zooms[4]["sent"] < zooms[3]["sent"]:
        problems.append("wheel down zoomed in; it must zoom out")
    if zooms[3] != zooms[2] or zooms[5] != zooms[4] or zooms[7] != zooms[6]:
        problems.append("the zoom changed during a wait")
    if zooms[7]["notches"] - zooms[5]["notches"] >= 40:
        problems.append("notches past the largest zoom still changed it")
    if any(dump.get("camera_turns", 0) != 0 for dump in dumps):
        problems.append("the wheel turned the camera")
    print(f"zoom: opening {base:.3f}; +3 notches {zooms[2]['sent']:.3f}; -5 {zooms[4]['sent']:.3f}; +40 {zooms[6]['sent']:.3f}")
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


EXPECT["two-views"] = expect_two_views
ON_FIXTURE.add("two-views")
# ADR-D01.17, .39 and .51: the context menu, the explore state and the non-standard colours mark.
EXPECT["explore-menu"] = expect_explore_menu
ON_FIXTURE.add("explore-menu")
EXPECT["slice"] = expect_slice
ON_FIXTURE.add("slice")
EVENT_CHECKS = {"two-views": check_two_views_events, "slice": check_slice_events}


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


def expect_ring_sheet(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """The ring-sheet script: the same zoom as the rings', from the data folder's opening file."""
    return expect_hopf(dumps, source, state, projection="ringSheet")


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
    if final["selection"] is not None:
        problems.append("the click in the opening's corner did not clear the selection")
    print(f"pick: {len(clicks)} clicks, {final['hits']} hits, {final['misses']} misses, {final['stale']} stale")
    return problems


EXPECT["pick"] = expect_pick
ON_FIXTURE.add("pick")


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


def window_problems(dumps: list[dict]) -> list[str]:
    """The owner's window rule as the shell itself reports it in every dump."""
    problems = []
    for dump in dumps:
        window = dump.get("window", {})
        if window.get("topmost") is not False:
            problems.append(f"step {dump['step']}: the shell's window is topmost")
        if window.get("foreground") is not False:
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
        clock=time.monotonic, sleep=time.sleep) -> tuple[int, dict]:
    """Launches the run, watches the foreground, and compares the screens. Returns the exit code and the record."""
    path = SCRIPTS / f"{script}.json"
    source = data()
    if script not in EXPECT or not path.is_file():
        raise ValueError(f"no D02 script named {script} (renderer/shell/d02_scripts/)")
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
        process = subprocess.Popen(arguments, cwd=cwd, env=env, **options)
        started["process"], started["at"] = process, clock()
        return process

    written = json.loads(path.read_text(encoding="utf-8"))
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
    if "at" in started:
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
    if after is None or after != before:
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
        problems += window_problems(dumps)
        problems += EXPECT[script](dumps, source, state)
    if script in EVENT_CHECKS:
        problems += EVENT_CHECKS[script](out / "events.jsonl")
    record = {"format": FORMAT, "script": script, "state": state, "steps": len(dumps), "mismatches": problems,
              "foreground_kept": after is not None and after == before, "topmost": bool(topmost), "windows": seen,
              "gave_back": max((dump.get("window", {}).get("gave_back", 0) for dump in dumps), default=0)}
    if out.is_dir():
        (out / RECORD).write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8", newline="\n")
    return (0 if not problems else 1), record


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


if __name__ == "__main__":
    sys.exit(main())
