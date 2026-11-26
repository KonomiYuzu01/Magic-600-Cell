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

import sys

sys.dont_write_bytecode = True

import argparse
import json
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
    """B4's smoke script: D02 is on; the palette lists the CR-16 draft rows, each marked; Escape closes it."""
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
    if len(wanted) != 8 or not set(wanted) <= drafts:
        problems.append(f"the data: tuning. matches {wanted}, not the eight CR-16 draft rows")
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


def expect_host(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """B6's host script, in sample mode on the state's session fixture: the Jumble cell, the mark on both view borders,
    Local's empty sentence and four closed sections, the cap map drawn (B8) with its class counts and its legend armed,
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
    if "Name a cap: click its cell, or type its number." not in every:
        problems.append("step 1: Local does not show its empty sentence")
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
    """B9's name-cap script, by keys only: before naming, no cap is named and Local shows its empty sentence; after
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
    if "Name a cap: click its cell, or type its number." not in texts(before):
        problems.append("step 1: Local does not show its empty sentence")
    if row["status"] == "admissible":
        tier1 = f"cap 30 · N_c {row['N_c']} · floor {row['F_c']} · u 0 · 80 meeting caps"
    else:
        tier1 = "cap 30 · N_c undefined · floor undefined"
    if not any(text.startswith(tier1) for text in texts(named, "panel:jumble.cap_map")):
        problems.append(f"step 5: the cap map's panel does not read {tier1!r}")
    if "▸ Named cap · 1" not in texts(named):
        problems.append("step 5: Local's Named cap section does not count the named cap")
    if "Name a cap: click its cell, or type its number." in texts(named):
        problems.append("step 5: Local still shows its empty sentence with a cap named")
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


EXPECT = {"smoke": expect_smoke, "bind": expect_bind, "host": expect_host, "rotate": expect_rotate, "name-cap": expect_name_cap,
          "q3b": expect_q3b, "covers": expect_covers, "repair": expect_repair, "zoom": expect_zoom,
          "focus-layer": expect_focus_layer, "seed7001": expect_seed7001, "nu-band": expect_nu_band}
# Scripts that run on the named state's session fixture, which build_d02_fixtures.py writes, and scripts that run in
# sample mode (ADR-D01's frame on sample data), where the strip and the rail are.
ON_FIXTURE = {"bind", "host", "name-cap", "q3b", "covers", "repair", "focus-layer", "nu-band"}
IN_SAMPLE_MODE = {"host", "nu-band"}


# The drawing track (ADR-D02 B18c onward): its scripts, and the event-log checks that read the run's frame rows.
def expect_two_views(dumps: list[dict], source: dict, state: str | None) -> list[str]:
    """B18c: the script ran to its end; the frame rows decide (check_two_views_events)."""
    return [] if dumps else ["the two-views run wrote no screen"]


def frame_rows(events: Path) -> list[dict]:
    if not events.is_file():
        return []
    rows = (json.loads(line) for line in events.read_text(encoding="utf-8").splitlines() if line.strip())
    return [row for row in rows if row.get("t") == "frame"]


def check_two_views_events(events: Path) -> list[str]:
    """B18c: one-view frames until a cap is named, then composite frames of two panes that are drawn, not held, at
    every engine revision the run reached after that (twists by the router's KeyJ and an undo), so both views show one
    revision; at least one revision change happens while Local is drawn."""
    frames = frame_rows(events)
    if not frames:
        return ["the run wrote no frame rows"]
    problems = []
    if "panes" in frames[0]:
        problems.append("the first frame is a composite frame before any cap was named")
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


EXPECT["two-views"] = expect_two_views
ON_FIXTURE.add("two-views")
EVENT_CHECKS = {"two-views": check_two_views_events}


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

    arguments = ["--out", str(out), "--frame", "--puzzle", "jumble-on", "--d02", "--d02-script", str(path)]
    arguments += ["--sample"] if script in IN_SAMPLE_MODE else []
    arguments += ["--build", str(build)] if build else []
    arguments += ["--d02-state", state] if state else []
    arguments += ["--d02-fixture", str(fixture)] if fixture is not None and (fixture / "fixture.json").is_file() else []
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
