"""The reduced Track J driver (ADR-D02 build step B32): J1, a model-free run in the product shell by keys alone.

  python -B renderer/shell/track_j.py --reduced --only j1 --out NEW_FOLDER [--build DIR]

J1's session is tools/looklab/data/samples/jumble/j1-session.json. Each of its parts is played in a fresh product
shell on its sample state's session fixture, in sample mode, through d02_run.py's player: keys and palette text in,
the screen's text out (section 7 of docs/progress/1.0/adr-d02-build-framework.md). A part's steps are chords, held
keys, typed text, waits and `op` steps. An `op` step is the palette's typed mode, as a researcher types it: Space
(command.find in the keymap; in sample mode F1 shows the key map), then `> row argument`, then Enter. No step clicks,
hovers, focuses a control or runs a row past the palette.

The run checks J1's four parts:
- keyboard reach: every row whose id starts with jumble., research. or evidence. (the command table and the D02
  draft rows) and every D02 draft row is typed. A row is reached when the palette lists it for the typed line and
  closes on Enter, which it does only after running the row (CommandPalette.Accept keeps an unavailable row on screen
  with its reason), and the error lamp counts no new failure. A row this build does not carry out is listed under
  not_run with the reason the palette must show for it (and the argument it is typed with, where the row's handler
  takes only a named object); J1 types it and checks that the palette refuses it with that reason, then closes the
  palette with Escape. A not_run row that runs is a failure: take it off not_run. An op may also expect a refusal that the state explains (a start
  before its budget); it is checked the same way;
- observation: one check per rule of docs/progress/1.0/jumbling-observation.md that this build can show by keys, and
  the cut slice's trace (its section 5); a rule it cannot show is listed under not_run with the reason;
- the repair list's boundary: its rows in the record's order, no row focused on open, nothing staged before a digit
  or Enter; under not_run while the list has no key route;
- the boundary count: the Jumble host's BoundaryLedger counts no unasked act in any screen.

It prints `M600_D02_J1_PASS operations=<k> reached=<k> boundary=0 observation=<n>/<n> not_run=<list>` or
`M600_D02_J1_FAIL ...` with the problems, and writes track-j.json in the out folder. Each part's run folder holds the
shell's screen dumps and logs. A run opens a window on the GPU for each part (machine mark W): never during a
quiet-machine hold, and leave the window alone while it plays. Exit 0 on a pass, 1 on a failure, 2 for a usage error.
"""
from __future__ import annotations

import sys

sys.dont_write_bytecode = True

import argparse
import hashlib
import json
import math
import random
import re
import secrets
import shutil
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import d02_run  # noqa: E402

ROOT = HERE.parents[1]
SESSION = ROOT / "tools" / "looklab" / "data" / "samples" / "jumble" / "j1-session.json"
LABELS = ROOT / "tools" / "looklab" / "data" / "design" / "command-labels.json"
FORMAT = "magic600-d02-j1-session/1"
RECORD = "track-j.json"
PALETTE = "overlay:CommandPalette"
PREFIXES = ("jumble.", "research.", "evidence.")
OPEN = "Space"
UNAVAILABLE = "Unavailable: "
PIECES = 177_120
STEP_KINDS = {"op", "chord", "text", "wait_ms", "hold", "release"}
J0_KINDS = STEP_KINDS | {"open"}
# J1's third part, the repair list's boundary, is named in not_run while it cannot run by keys.
REPAIR_PART = "repair-boundary"
OP_KEYS = {"op", "argument", "wait_ms", "refused", "close"}
# Words a view would use to choose for the solver (jumbling-observation.md rule 6); the palette's own help texts are
# not read for them.
CHOOSING = re.compile(r"\b(best|recommended|recommend|next move|most promising)\b", re.IGNORECASE)


def operations(table: list[dict], drafts: list[dict]) -> list[str]:
    """J1's rows in order: the command table's jumble., research. and evidence. rows, then every D02 draft row."""
    found = [row["id"] for row in table if row["id"].startswith(PREFIXES)]
    return found + [row["id"] for row in drafts if row["id"] not in found]


def labels(table: list[dict], drafts: list[dict], relabel: dict[str, str] | None = None) -> dict[str, str]:
    """Each row's label as the palette shows it: command-labels.json's word where it has one."""
    if relabel is None:
        relabel = json.loads(LABELS.read_text(encoding="utf-8"))["labels"]
    return {row["id"]: relabel.get(row["id"], row["label"]) for row in table + drafts}


def shown(label: str, argument: str | None) -> str:
    return label + (f": {argument}" if argument is not None else "")


def expand(steps: list[dict], not_run: dict | None = None,
           kinds: set[str] = STEP_KINDS) -> tuple[list[dict], list[dict]]:
    """A part's steps as d02 script steps, and each op with the screen dumps of its typed line, its Enter and its last
    step (step i of the script is screen dump i + 1: dump 0 is the shell's start). J0's session may also open one of
    Local's sections (kinds=J0_KINDS), since the repair list has no key route yet."""
    not_run = not_run or {}
    script, ops = [], []
    for step in steps:
        if set(step) - (OP_KEYS if "op" in step else kinds) or ("op" not in step and len(step) != 1):
            raise ValueError(f"a session step is an op (with optional argument, wait_ms, refused and close) or one of "
                             f"{sorted(kinds)}: {step}")
        if "op" not in step:
            script.append(dict(step))
            continue
        row, argument = step["op"], step.get("argument")
        refused = step.get("refused")
        if refused is None and isinstance(not_run.get(row), dict):
            refused = not_run[row].get("palette")
        script.append({"chord": OPEN})
        opened = len(script)
        script.append({"text": "> " + row + (f" {argument}" if argument is not None else "")})
        typed = len(script)
        script.append({"chord": "Enter"})
        entered = len(script)
        if step.get("wait_ms"):
            script.append({"wait_ms": step["wait_ms"]})
        # A refused row leaves the palette open, and an op that opens an overlay (a context menu) leaves it open:
        # Escape closes the open overlay (transient.close closes it first and is spent on it).
        if refused is not None or step.get("close"):
            script.append({"chord": "Escape"})
        ops.append({"row": row, "argument": argument, "refused": refused, "open": opened, "text": typed,
                    "enter": entered, "last": len(script)})
    return script, ops


def expected_do(step: dict) -> str:
    for kind in ("chord", "text", "hold", "release", "open"):
        if kind in step:
            return f"{kind} {step[kind]}"
    return f"wait {step['wait_ms']} ms"


def palette(dump: dict) -> list[str]:
    return [text for group, text in dump["lines"] if group == PALETTE]


def failures(dump: dict) -> tuple[int, int]:
    errors = dump.get("errors", {})
    return errors.get("count", 0), errors.get("failures", 0)


def reach(dumps: list[dict], op: dict, label: str, until: int) -> tuple[str, str | None]:
    """("ran", None) when the typed line ran the row (also when the op expected a refusal: j1 says so);
    ("refused", reason) when the palette refused it with the reason the op expects; ("missed", why) otherwise."""
    typed, entered = dumps[op["text"]], dumps[op["enter"]]
    listed = palette(typed)
    if not listed:
        return "missed", "the palette was not open for the typed line"
    want = shown(label, op["argument"])
    if want not in listed:
        return "missed", f"the palette does not list {want!r} for the typed line (it shows {listed[1:2]})"
    reason = next((text.split("\n")[0][len(UNAVAILABLE):] for text in listed if text.startswith(UNAVAILABLE)), None)
    before, after = failures(typed), failures(dumps[until])
    if after != before:
        return "missed", (f"the error lamp counted {after[0] - before[0]} new errors and {after[1] - before[1]} new "
                          f"failures: {dumps[until].get('errors', {}).get('last', '')}")
    if reason is None:
        if palette(entered):
            return "missed", "Enter left the palette open"
        return "ran", None
    if op["refused"] is None:
        return "missed", f"the palette refuses it: {reason}"
    if op["refused"] not in reason:
        return "missed", f"the palette refuses it with {reason!r}, not {op['refused']!r}"
    if not palette(entered):
        return "missed", "Enter closed the palette on an unavailable row"
    if palette(dumps[op["last"]]):
        return "missed", "Escape left the palette open"
    return "refused", reason


def boundary(dumps: list[dict]) -> tuple[int, int]:
    """The most unasked acts the Jumble host's BoundaryLedger counted, and how many screens carry no count: a screen
    without the ledger's count is a problem, never a zero."""
    counts = [(dump.get("stage") or {}).get("unasked") for dump in dumps]
    known = [n for n in counts if isinstance(n, int) and not isinstance(n, bool)]
    return max(known, default=0), len(counts) - len(known)


def screen(dump: dict) -> list[str]:
    """The screen's words outside the palette (whose help texts quote the rows' notes)."""
    return [text for group, text in dump["lines"] if group != PALETTE]


def find(ops: list[dict], ref: str) -> dict:
    """An op named by `row`, `row argument` or either with `#n` for its nth time (from 1)."""
    ref, _, nth = ref.partition("#")
    row, _, argument = ref.partition(" ")
    found = [op for op in ops if op["row"] == row and (not argument or op["argument"] == argument)]
    n = int(nth or 1)
    if len(found) < n:
        raise ValueError(f"the session has no op {ref} (number {n})")
    return found[n - 1]


def at(dumps: list[dict], ops: list[dict], ref: str) -> dict:
    """The screen a check reads: `start` is the one before the first op; an op's is the one after its last step;
    `step N` is the screen dump of the part's script step N (from 1)."""
    if ref == "start":
        return dumps[ops[0]["open"] - 1]
    if ref.startswith("step "):
        return dumps[int(ref[5:])]
    return dumps[find(ops, ref)["last"]]


# The observation checks: each takes the part's dumps, its ops, the session's rule entry and the part's sample state,
# and returns its problems.
CHECKS: dict = {}


def check(name: str):
    def register(function):
        CHECKS[name] = function
        return function
    return register


@check("state_unchanged")
def state_unchanged(dumps: list[dict], ops: list[dict], rule: dict, state: str) -> list[str]:
    """Rule 1: no filter, view, tint or tool changes the state. The head's state digest reads the same in every
    screen, nothing is staged but by the stage row the solver types (B32 findings: jumble.repair.stage is the solver's
    pick, not a tool), and the named cap changes only when a naming row runs."""
    problems = []
    digests = {(dump.get("tuning") or {}).get("state_digest") for dump in dumps}
    if len(digests) != 1 or not next(iter(digests)):
        problems.append(f"the head's state digest read {sorted(str(d) for d in digests)}")
    owner = {}
    for op in ops:
        for i in range(op["open"], op["last"] + 1):
            owner[i] = op["row"]
    records = [(dump.get("stage") or {}).get("records", 0) for dump in dumps]
    staged = sum(records[i] - records[i - 1] for i in range(1, len(dumps))
                 if records[i] > records[i - 1] and owner.get(i) != "jumble.repair.stage") + (records[0] if records else 0)
    if staged:
        problems.append(f"{staged} records were staged")
    for i in range(1, len(dumps)):
        if dumps[i].get("named") != dumps[i - 1].get("named") and \
                owner.get(i) not in ("jumble.cap.name", "jumble.cap.unname"):
            problems.append(f"screen {i} ({dumps[i]['do']}): the named cap became {dumps[i].get('named')} with no "
                            f"naming row")
    return problems


@check("tint_bytes")
def tint_bytes(dumps: list[dict], ops: list[dict], rule: dict, state: str) -> list[str]:
    """Rule 2: the tint is the engine's piece sets, not the view's: at each named screen the outline and fill bytes
    sent to the core equal the sample's tables piece by piece (d02_run.tint_expected)."""
    tables = d02_run.tint_tables(state)
    problems = []
    for ref, mode in rule["at"]:
        tint = at(dumps, ops, ref).get("tint")
        if not isinstance(tint, dict):
            problems.append(f"{ref}: the screen has no tint")
            continue
        want = d02_run.tint_expected(tables, mode, None, tint.get("hover"), tint.get("selection"))
        for key, value in want.items():
            if tint.get(key) != value:
                problems.append(f"{ref} ({mode}): {key} is {str(tint.get(key))[:100]!r}, not {str(value)[:100]!r}")
    return problems


@check("whole_analysis")
def whole_analysis(dumps: list[dict], ops: list[dict], rule: dict, state: str) -> list[str]:
    """Rule 4: the analysis covers all 177,120 pieces whatever the work filter: under the domain focus the contact
    classes' legend still counts every piece."""
    problems = []
    for ref in rule["at"]:
        dump = at(dumps, ops, ref)
        tint = dump.get("tint") or {}
        counts = [int(part.rsplit(" ", 1)[1].replace(",", "")) for part in tint.get("line", "").split(" · ")[1:]
                  if part.rsplit(" ", 1)[-1].replace(",", "").isdigit()]
        focus = (dump.get("overlay") or {}).get("focus") or tint.get("focus")
        if sum(counts) != PIECES:
            problems.append(f"{ref}: the legend {tint.get('line', '')!r} counts {sum(counts):,} pieces, not {PIECES:,}")
        if focus in (None, "none"):
            problems.append(f"{ref}: no work filter is on")
    return problems


@check("never_hidden")
def never_hidden(dumps: list[dict], ops: list[dict], rule: dict, state: str) -> list[str]:
    """Rule 5: the named cap's trace, and at a blocked cap its straddlers with their certificate points counted, are
    outlined and never hidden by the domain focus: the overlay's counts, its words on both borders and the bytes sent
    to the core equal the sample's piece by piece (d02_run.overlay_expected)."""
    tables = d02_run.tint_tables(state)
    problems = []
    for ref, cap, focus in rule["at"]:
        dump = at(dumps, ops, ref)
        overlay, tint = dump.get("overlay"), dump.get("tint")
        if not isinstance(overlay, dict) or not isinstance(tint, dict):
            problems.append(f"{ref}: the screen has no overlay or tint")
            continue
        want = d02_run.overlay_expected(tables, state, cap, None, focus, tint.get("hover"), tint.get("selection"))
        line = d02_run.overlay_words(want)
        for key, value in [*want.items(), ("line", line), ("global_line", line), ("local_line", line)]:
            said = overlay if key not in ("outline_sha256", "fill_sha256", "focus") else tint
            if said.get(key) != value:
                problems.append(f"{ref}: {key} is {str(said.get(key))[:100]!r}, not {str(value)[:100]!r}")
        if want["never_hidden"] and overlay.get("kept_dimmed"):
            problems.append(f"{ref}: {overlay['kept_dimmed']} never-hidden pieces carry the dim bit")
    return problems


@check("neutral_order")
def neutral_order(dumps: list[dict], ops: list[dict], rule: dict, state: str) -> list[str]:
    """Rule 6: the tools never choose. The setup search lists its rows for the named target in the neutral order
    (length, then menu order) and says so, and no screen word outside the palette ranks or recommends a move."""
    problems = []
    words = screen(at(dumps, ops, rule["at"]))
    places = [words.index(row) if row in words else -1 for row in d02_run.SETUP_ROWS]
    if -1 in places or places != sorted(places):
        problems.append(f"the setup rows are not all shown in their order: {places}")
    if not any("order: length, then menu order" in text for text in words):
        problems.append("the setup result does not name its order")
    for i, dump in enumerate(dumps):
        for text in screen(dump):
            if CHOOSING.search(text):
                problems.append(f"screen {i}: {text[:100]!r} chooses for the solver")
                break
    return problems


@check("cut_trace")
def cut_trace(dumps: list[dict], ops: list[dict], rule: dict, state: str) -> list[str]:
    """Section 5, the cut slice: at the blocked cap the held preview key draws no ghost and the slice says why; at the
    named cap the slice holds its inside set In_c (the record's mask, counted), unsplit; the held split key splits it
    and its release joins it."""
    problems = []
    blocked = at(dumps, ops, rule["blocked"]).get("slice") or {}
    if blocked.get("ghost") or blocked.get("inside") != 0 or blocked.get("named") is not None or \
            not (blocked.get("refusal") or "").startswith(f"Cap {rule['blocked_cap']} is blocked"):
        problems.append(f"{rule['blocked']}: the blocked cap's slice is {blocked}")
    cap = rule["cap"]
    inside = sum(d02_run.overlay_mask(d02_run.load(d02_run.SAMPLES / state / f"cap-{cap}.json")["masks"]["inside"],
                                      PIECES))
    named = at(dumps, ops, rule["named"]).get("slice") or {}
    if named.get("inside") != inside or named.get("split") or named.get("ghost"):
        problems.append(f"{rule['named']}: the slice at cap {cap} is {named}, not its inside set of {inside:,} unsplit")
    held, after = (at(dumps, ops, ref).get("slice") or {} for ref in (rule["held"], rule["released"]))
    if not held.get("split") or after.get("split"):
        problems.append(f"the split read {held.get('split')} while held and {after.get('split')} after")
    return problems


def play(part: dict, out: Path, build: Path | None, scripts: Path,
         not_run: dict) -> tuple[list[dict], list[dict], list[str]]:
    """Writes the part's d02 script, plays it in a fresh shell and returns its dumps, its ops and the run's problems."""
    script, ops = expand(part["steps"], not_run)
    path = scripts / f"{part['name']}.json"
    path.write_text(json.dumps({"format": "magic600-d02-script/1", "name": "j1", "resolution": part["resolution"],
                                "note": part["note"], "steps": script}, indent=2) + "\n", encoding="utf-8", newline="\n")
    code, record = d02_run.run("j1", out / part["name"], build, part["state"], path=path)
    dumps = []
    screen_file = out / part["name"] / d02_run.SCREEN
    if screen_file.is_file():
        dumps = [json.loads(line) for line in screen_file.read_text(encoding="utf-8").splitlines() if line.strip()]
    problems = list(record["mismatches"])
    wanted = ["start"] + [expected_do(step) for step in script]
    if dumps and [dump["do"] for dump in dumps] != wanted:
        problems.append(f"the shell wrote {len(dumps)} steps, not the session's {len(wanted)}")
        dumps = []
    return dumps, ops, problems


def j1(session: dict, out: Path, build: Path | None, player=play) -> tuple[int, dict]:
    source = d02_run.data()
    rows = operations(source["table"], source["rows"])
    named = labels(source["table"], source["rows"])
    not_run = dict(session.get("not_run", {}))
    rules = {rule["rule"] for rule in session["observation"]}
    unknown = sorted(set(not_run) - set(rows) - rules - {REPAIR_PART})
    if unknown:
        raise ValueError(f"not_run names what J1 does not check: {unknown}")
    scripts = out / "scripts"
    scripts.mkdir(parents=True)
    problems: list[str] = []
    reached: dict[str, str] = {}
    refused: dict[str, str] = {}
    missed: dict[str, str] = {}
    unasked = 0
    played: dict[str, tuple[list[dict], list[dict], str]] = {}
    for part in session["parts"]:
        dumps, ops, run_problems = player(part, out, build, scripts, not_run)
        problems += [f"{part['name']}: {problem}" for problem in run_problems]
        played[part["name"]] = (dumps, ops, part["state"])
        if not dumps:
            continue
        counted, unread = boundary(dumps)
        unasked = max(unasked, counted)
        if unread:
            problems.append(f"{part['name']}: boundary: {unread} screens carry no BoundaryLedger count")
        for i, op in enumerate(ops):
            until = ops[i + 1]["open"] - 1 if i + 1 < len(ops) else len(dumps) - 1
            outcome, why = reach(dumps, op, named.get(op["row"], op["row"]), until)
            if outcome == "ran":
                reached.setdefault(op["row"], part["name"])
                if op["refused"] is not None and op["row"] not in not_run:
                    problems.append(f"{part['name']}: {op['row']} {op['argument'] or ''}".rstrip() +
                                    f": the palette ran it, not the refusal {op['refused']!r}")
            elif outcome == "refused":
                refused.setdefault(op["row"], why)
            else:
                problems.append(f"{part['name']}: {op['row']} {op['argument'] or ''}".rstrip() + f": {why}")
                missed[op["row"]] = why
    wanted = [row for row in rows if row not in not_run]
    for row in wanted:
        if row not in reached and row not in missed:
            problems.append(f"reach: {row} ({named.get(row, row)}): no step runs it")
    for row in (row for row in rows if row in not_run):
        if row in reached:
            problems.append(f"reach: {row} is not_run but ran: take it off not_run")
        elif row not in refused and row not in missed:
            problems.append(f"reach: {row} is not_run but no step types it")
    if unasked:
        problems.append(f"boundary: the Jumble host's BoundaryLedger counted {unasked} unasked acts")
    results = []
    checked = [rule for rule in session["observation"] if rule["rule"] not in not_run]
    for rule in checked:
        dumps, ops, state = played.get(rule["part"], ([], [], ""))
        found = CHECKS[rule["check"]](dumps, ops, rule, state) if dumps else [f"part {rule['part']} wrote no screens"]
        results.append({"rule": rule["rule"], "part": rule["part"], "check": rule["check"], "problems": found})
        problems += [f"observation {rule['rule']} ({rule['part']}): {problem}" for problem in found]
    # A rule is observed when each of its entries passes; a rule checked on two parts counts once.
    names = list(dict.fromkeys(rule["rule"] for rule in checked))
    observed = sum(1 for name in names if not any(r["problems"] for r in results if r["rule"] == name))
    record = {"format": "magic600-d02-track-j-v1", "bar": "j1", "operations": len(wanted),
              "reached": sum(1 for row in wanted if row in reached), "boundary": unasked,
              "observation": [observed, len(names)], "not_run": not_run, "rules": results,
              "reached_in": reached, "refused": refused, "problems": problems}
    (out / RECORD).write_text(json.dumps(record, indent=2, ensure_ascii=False) + "\n", encoding="utf-8", newline="\n")
    return (0 if not problems else 1), record


def line(record: dict, passed: bool) -> str:
    return (f"M600_D02_J1_{'PASS' if passed else 'FAIL'} operations={record['operations']} reached={record['reached']} "
            f"boundary={record['boundary']} observation={record['observation'][0]}/{record['observation'][1]} "
            f"not_run={','.join(sorted(record['not_run'])) or '-'}")


def load_session(path: Path = SESSION) -> dict:
    session = json.loads(path.read_text(encoding="utf-8"))
    if session.get("format") != FORMAT:
        raise ValueError(f"{path.name} is not a {FORMAT} file")
    for rule in session["observation"]:
        if rule["rule"] not in session.get("not_run", {}) and rule.get("check") not in CHECKS:
            raise ValueError(f"observation rule {rule['rule']} names no known check")
    return session


# J0 (B33): planted defects, the seed 7001 replay, and the packet the researcher subagent reads. Each defect is
# planted in a copy of Look Lab's samples (launch.py --d02-data) and the fixed session in j0-session.json is played on
# it; the checks read the screens against the repository's samples, never against the planted copy.
J0_SESSION = d02_run.SAMPLES / "j0-session.json"
DEFECTS = d02_run.SAMPLES / "defects.json"
DATA = d02_run.SAMPLES.parent
WORDS = d02_run.SAMPLES / "seed7001-words.json"
PANEL = DATA / "research" / "panel.json"
J0_FORMAT = "magic600-d02-j0-session/1"
DEFECTS_FORMAT = "magic600-d02-defects/1"
KEY_FORMAT = "magic600-d02-j0-key/1"
J0_RECORD = "track-j0.json"
J0_REPORT = "track-j0.md"
LEAST, SHARE = 20, 0.8
FAULTED, CLEAN_RUNS = 8, ("clean-1", "clean-2")
# Section 7's cap on the researcher subagent: scripts written, dumps read, minutes and words of output.
CAP = {"scripts": 2, "dumps": 12, "minutes": 90, "words": 2000}
DEFECT_ID = re.compile(r"^(opus|fable)-\d{2}$")
# Words that never reach the researcher's packet: they would name a planted copy, a defect or a run's label.
LEAKS = ("plant", "defect", "opus-", "fable-", *CLEAN_RUNS)
# A dump's fields that are the harness's (the window's foreground), not the screen's.
HARNESS_KEYS = ("window",)
LEGEND = (("blocked", "blocked"), ("closed", "closed"), ("at_floor", "at floor (N_c = F_c > 0)"),
          ("above_floor", "above floor"), ("pending", "pending"))
OFF_LATTICE_BAND = "ν band: undefined (pieces off the lattice)"
PAIR = re.compile(r"^-?\d+ -?\d+$")
SIGNED = re.compile(r"^[+\-−]?\d+$")
# ScriptList.StateGlyph's icons for the pending, ok, failed and stale check states.
STATE_ICONS = ("task.running", "task.ok", "severity.error", "script.rerun")
# The rules a J0 problem names, and the researcher's answers name, each with the sentence the packet gives for it.
RULES = {
    "digest": "the shell binds the state by its digests and names it in every screen",
    "state": "no view, tint or filter changes the state, and only a naming row changes the named cap",
    "boundary": "the program never acts unasked",
    "lattice": "off the lattice, the strip, the rail and the ν band read undefined",
    "cap-readouts": "the named cap's N_c, floor and readout are the engine's record of that cap",
    "cap-map": "the cap map's legend counts the caps of each class",
    "threshold": "a threshold counts members, non-members, blocked and pending caps apart",
    "blocked-undefined": "at a blocked cap, N_c, the floor, the covers and the repair rows are undefined, never a number",
    "covers": "Cap covers lists each side as the record answers it, and a NotApplicable side is never a pass",
    "repair-rows": "the repair list shows the record's rows, no more and no fewer",
    "repair-order": "the repair rows keep the record's order, and no row is focused on open",
    "delta-n": "each repair row's ΔN keeps the record's value and sign",
    "never-hidden": "the cut trace and a blocked cap's straddlers are outlined and never hidden, piece by piece",
    "style-bytes": "the tint's bytes are the engine's piece sets, piece by piece",
    "contact-classes": "the contact classes are the engine's, and no class is reused across a contact",
    "verdict": "each research claim shows its latest check's verdict, and the head counts them",
    "cut-off": "a check cut off by its budget reads 'not found within budget', never as finished",
}
# Section 7's defect classes.
CLASSES = [
    "repair rows reordered or pre-focused",
    "N_c at a blocked cap",
    "total N as a sum of caps",
    "pending shown as undefined",
    "a late result overwriting a newer one",
    "a result without coverage shown as ready",
    "a folded row with a numeral",
    "a class reused across a contact",
    "a blocked cap counted as a non-member",
    'a cut-off worded as "none"',
    "NotApplicable counted as a pass",
    "research output admitted to a solving preview",
    "a digest changed by one character",
    "a ΔN sign flipped",
    "a style byte for the wrong piece",
]
# The classes a sample file cannot plant in this build, with the reason the report gives.
NOT_PLANTABLE = {
    "total N as a sum of caps": "J0's fixture state, wj-ia-300, is off the lattice, where the strip and the rail read "
                                "undefined; adding the caps would be a change to the shell's code (ShellJumble.cs), "
                                "not to a sample",
    "pending shown as undefined": "wj-ia-300's rim is complete (pending 0), and the pending wording is the shell's "
                                  "(ShellJumble.cs)",
    "a late result overwriting a newer one": "an ordering fault between two jobs; sample mode answers from files, "
                                             "with no job that can finish late",
    "a result without coverage shown as ready": "the readiness test is the shell's: a sample with its coverage taken "
                                                "out tests the shell, not a planted fault",
    "a folded row with a numeral": "folded rows are written at run time from superseded releases (ADR-D02.35, .36); "
                                   "no sample file holds one",
    "research output admitted to a solving preview": "sample mode has no solving preview, and admission is the "
                                                     "command layer's",
}


def load_j0_session(path: Path = J0_SESSION) -> dict:
    session = json.loads(path.read_text(encoding="utf-8"))
    if session.get("format") != J0_FORMAT:
        raise ValueError(f"{path.name} is not a {J0_FORMAT} file")
    unknown = sorted({entry["rule"] for entry in session["checks"]} - set(J0_CHECKS))
    if unknown:
        raise ValueError(f"{path.name} checks rules J0 does not know: {unknown}")
    expand(session["steps"], {}, J0_KINDS)
    return session


def load_defects(paths: list[Path]) -> list[dict]:
    """Every defect of the files named, in order; each is {id, by, class, rule, change[, note]}. A rule outside the
    driver's (RULES) is allowed: the defect counts as planted and only the researcher subagent can catch it. The same
    change under two ids, and an id whose prefix is not its writer, are refused (each would inflate the planted count)."""
    defects, seen, changes = [], set(), {}
    for path in paths:
        doc = json.loads(path.read_text(encoding="utf-8"))
        if doc.get("format") != DEFECTS_FORMAT:
            raise ValueError(f"{path.name} is not a {DEFECTS_FORMAT} file")
        for defect in doc["defects"]:
            missing = sorted({"id", "by", "class", "rule", "change"} - set(defect))
            if missing:
                raise ValueError(f"{path.name}: a defect lacks {missing}")
            if not DEFECT_ID.match(defect["id"]) or defect["id"] in seen:
                raise ValueError(f"{path.name}: defect id {defect['id']!r} is not opus-NN or fable-NN, or repeats")
            writer = defect["id"].split("-")[0]
            if not isinstance(defect["by"], str) or defect["by"].strip().lower() != writer:
                raise ValueError(f"{defect['id']}: {writer}- ids are written by {writer.title()}, not {defect['by']!r}")
            if defect["class"] not in CLASSES:
                raise ValueError(f"{defect['id']}: {defect['class']!r} is not one of section 7's classes")
            if not isinstance(defect["rule"], str) or not defect["rule"].strip():
                raise ValueError(f"{defect['id']}: the defect names no rule")
            if not defect["change"]:
                raise ValueError(f"{defect['id']}: the defect changes nothing")
            change = json.dumps(defect["change"], sort_keys=True, ensure_ascii=False)
            if change in changes:
                raise ValueError(f"{defect['id']}: the same change as {changes[change]}")
            changes[change] = defect["id"]
            seen.add(defect["id"])
            defects.append(defect)
    return defects


def flip_mask(mask: dict, bits: list[int]) -> dict:
    """A piece mask (bits-le-zlib-base64) with the bits named flipped, its count and digest written again."""
    import base64
    import hashlib
    import zlib
    raw = bytearray(zlib.decompress(base64.b64decode(mask["data"])))
    for bit in bits:
        if not 0 <= bit < len(raw) * 8:
            raise ValueError(f"bit {bit} is outside the mask")
        raw[bit >> 3] ^= 1 << (bit & 7)
    return {**mask, "data": base64.b64encode(zlib.compress(bytes(raw), 9)).decode("ascii"),
            "sha256": hashlib.sha256(raw).hexdigest(), "count": sum(bin(b).count("1") for b in raw)}


def plant_change(doc, change: dict) -> None:
    """Applies one change at its JSON path: set, negate (a number), swap (two items of a list), char (one character of
    a string) or mask_flip (bits of a piece mask). A change that leaves its value as it was is refused."""
    node = doc
    path = change["path"]
    for key in path[:-1]:
        node = node[key]
    key = path[-1]
    before = json.dumps(node[key], sort_keys=True)
    op = change["op"]
    if op == "set":
        node[key] = change["value"]
    elif op == "negate":
        if isinstance(node[key], bool) or not isinstance(node[key], (int, float)):
            raise ValueError(f"negate needs a number at {path}")
        node[key] = -node[key]
    elif op == "swap":
        items = node[key]
        items[change["i"]], items[change["j"]] = items[change["j"]], items[change["i"]]
    elif op == "char":
        text, place, to = node[key], change["at"], change["to"]
        if not isinstance(text, str) or len(to) != 1 or not 0 <= place < len(text):
            raise ValueError(f"char needs a string and one character at {path}")
        node[key] = text[:place] + to + text[place + 1:]
    elif op == "mask_flip":
        node[key] = flip_mask(node[key], change["bits"])
    else:
        raise ValueError(f"unknown change {op!r}")
    if json.dumps(node[key], sort_keys=True) == before:
        raise ValueError(f"the change at {path} leaves the value as it was")


def plant(defect: dict, folder: Path, source: Path = DATA) -> list[str]:
    """Copies the samples into folder/samples (without the defect lists and the sessions) and applies the defect's
    changes; answers the sample files it changed. A clean run's defect has no change."""
    samples = folder / "samples"
    shutil.copytree(source, samples, ignore=shutil.ignore_patterns("defects*.json", "j0-session.json",
                                                                   "j1-session.json"))
    docs: dict[str, object] = {}
    root = samples.resolve()
    for change in defect["change"]:
        name = change["file"]
        target = (samples / name).resolve()
        if root not in target.parents or not target.is_file() or target.suffix != ".json":
            raise ValueError(f"{defect['id']}: {name} is not a sample JSON file")
        if name not in docs:
            docs[name] = json.loads(target.read_text(encoding="utf-8"))
        try:
            plant_change(docs[name], change)
        except (KeyError, IndexError, TypeError, ValueError) as error:
            raise ValueError(f"{defect['id']}: {name}: {error}") from None
    for name, doc in docs.items():
        (samples / name).write_text(json.dumps(doc, ensure_ascii=False) + "\n", encoding="utf-8", newline="\n")
    return sorted(docs)


def replay(root: Path, writer: Path | None = None, write=None) -> dict:
    """The seed 7001 replay (section 7): the host's own engine, the fixture writer, executes the start's word and then
    each of the nine words in turn, and the state after each word must bind to the sample the word file names: the
    head's digest equals the word's output digest and all three digests equal that sample's (build_d02_fixtures.write
    refuses a fixture that differs). The words' letters must extend the start as the research word does; a write that
    fails is the word's problem."""
    import build_d02_fixtures as fixtures
    write = write or fixtures.write
    writer = writer or fixtures.WRITER

    def bind(state: str, letters: list) -> str:
        write(writer, state, fixtures.moving(letters, identity)[0], folder)
        return json.loads((folder / state / "fixture.json").read_text(encoding="utf-8"))["digest"]

    doc = json.loads(WORDS.read_text(encoding="utf-8"))
    every, identity = fixtures.words(), fixtures.identity_items()
    folder = root / "replay"
    problems: list[str] = []
    word = [list(letter) for letter in every[doc["start"]]]
    before = None
    try:
        before = bind(doc["start"], word)
    except fixtures.FixtureError as error:
        problems.append(f"the start {doc['start']} does not bind: {error}")
    rows = []
    for part in doc["parts"]:
        word += [list(letter) for letter in part["letters"]]
        row = {"word": part["name"], "after": part["after"], "letters": part["letters"], "bound": False}
        rows.append(row)
        if word != every[part["after"]]:
            row["problem"] = f"its letters do not extend {doc['start']} to {part['after']} as the research word does"
            before = None
            continue
        if before is not None and part["input_digest"] != before:
            row["problem"] = f"its input digest is not the state before it ({before[:12]})"
        try:
            digest = bind(part["after"], word)
        except fixtures.FixtureError as error:
            row["problem"] = str(error)
            before = None
            continue
        row["digest"] = digest
        if digest != part["output_digest"]:
            row["problem"] = f"the state's digest {digest[:12]} is not the word's output digest {part['output_digest'][:12]}"
        row["bound"] = "problem" not in row
        before = digest
    if doc["parts"] and doc["parts"][-1]["output_digest"] != doc["solved_digest"]:
        problems.append("the last word does not end at the solved digest")
    return {"bound": sum(1 for row in rows if row["bound"]), "words": len(rows), "start": doc["start"],
            "rows": rows, "problems": problems}


# J0's checks: each takes a run's dumps, the session's ops, the session's check entry and the sample state, and
# returns its problems; the rule a problem names is the entry's.
J0_CHECKS: dict = {}


def j0_check(rule: str):
    def register(function):
        J0_CHECKS[rule] = function
        return function
    return register


def sample(state: str, name: str):
    return d02_run.load(d02_run.SAMPLES / state / name)


def tier1(row: dict) -> str:
    """The cap map panel's line for a named cap (as expect_name_cap reads it)."""
    if row["status"] == "admissible":
        return f"cap {row['cap']} · N_c {row['N_c']} · floor {row['F_c']} · u 0 · 80 meeting caps"
    return f"cap {row['cap']} · N_c undefined · floor undefined"


def said(lines: list[str], head: str) -> list[str]:
    return [text for text in lines if text.startswith(head)][:2]


@j0_check("digest")
def j0_digest(dumps: list[dict], ops: list[dict], entry: dict, state: str) -> list[str]:
    """From the first step on, every screen's top bar reads `bound · <state>` and the tint is bound to the state."""
    want = f"bound · {state}"
    bad = [i for i in range(1, len(dumps)) if want not in d02_run.texts(dumps[i], "frame")
           or (dumps[i].get("tint") or {}).get("bound") != state]
    if not bad:
        return []
    shown = said(d02_run.texts(dumps[bad[0]], "frame"), "bound") + said(d02_run.texts(dumps[bad[0]], "frame"), "unbound")
    return [f"{len(bad)} screens from screen {bad[0]} do not read {want!r} with the tint bound (screen {bad[0]} reads "
            f"{shown})"]


@j0_check("state")
def j0_state(dumps: list[dict], ops: list[dict], entry: dict, state: str) -> list[str]:
    return state_unchanged(dumps, ops, entry, state)


@j0_check("boundary")
def j0_boundary(dumps: list[dict], ops: list[dict], entry: dict, state: str) -> list[str]:
    unasked, unread = boundary(dumps)
    return ([f"the Jumble host's BoundaryLedger counted {unasked} unasked acts"] if unasked else []) + \
        ([f"{unread} screens carry no BoundaryLedger count"] if unread else [])


@j0_check("lattice")
def j0_lattice(dumps: list[dict], ops: list[dict], entry: dict, state: str) -> list[str]:
    off = sample(state, "totals.json")["off_lattice"] > 0
    cell = "Jumble · " + ("off the lattice" if off else "on the lattice")
    band = OFF_LATTICE_BAND if off else d02_run.nu_words(state)["band"]
    problems = []
    for ref in entry["at"]:
        dump = at(dumps, ops, ref)
        frame = d02_run.texts(dump, "frame")
        if cell not in frame:
            problems.append(f"{ref}: the top bar does not read {cell!r}")
        if off and frame.count("undefined") < 2:
            problems.append(f"{ref}: the strip and the rail are not undefined off the lattice")
        if (dump.get("nu") or {}).get("band") != band:
            problems.append(f"{ref}: the ν band reads {(dump.get('nu') or {}).get('band')!r}, not {band!r}")
    return problems


@j0_check("cap-readouts")
def j0_cap_readouts(dumps: list[dict], ops: list[dict], entry: dict, state: str) -> list[str]:
    rows, problems = sample(state, "rim.json")["rows"], []
    for ref, cap in entry["at"]:
        row, dump = rows[cap], at(dumps, ops, ref)
        if row["status"] != "admissible":
            raise ValueError(f"j0-session.json reads cap {cap}'s readouts, but the sample does not admit it")
        panel, local = d02_run.texts(dump, "panel:jumble.cap_map"), d02_run.texts(dump, "view:Local")
        if not any(text.startswith(tier1(row)) for text in panel):
            problems.append(f"{ref}: the cap map's panel reads {said(panel, f'cap {cap} ·')}, not {tier1(row)!r}")
        missing = [key for key in ("In_c", "Out_c", "A_c", "N_c") if str(row[key]) not in local]
        if missing:
            problems.append(f"{ref}: Local's readout does not show cap {cap}'s {', '.join(missing)} "
                            f"({', '.join(str(row[key]) for key in missing)})")
        if "▸ Named cap · 1" not in local and "▾ Named cap · 1" not in local:
            problems.append(f"{ref}: Local's Named cap section does not count the named cap")
    return problems


@j0_check("cap-map")
def j0_cap_map(dumps: list[dict], ops: list[dict], entry: dict, state: str) -> list[str]:
    classes, problems = sample(state, "totals.json")["cap_classes"], []
    for ref in entry["at"]:
        panel = d02_run.texts(at(dumps, ops, ref), "panel:jumble.cap_map")
        for key, word in LEGEND:
            if f"{word} · {classes[key]}" not in panel:
                problems.append(f"{ref}: the legend reads {said(panel, word + ' ·')}, not {word} · {classes[key]}")
    return problems


@j0_check("threshold")
def j0_threshold(dumps: list[dict], ops: list[dict], entry: dict, state: str) -> list[str]:
    rows, k = sample(state, "rim.json")["rows"], entry["n_c"]
    admissible = [row for row in rows if row["status"] == "admissible"]
    members = sum(1 for row in admissible if row["N_c"] >= k)
    blocked = sum(1 for row in rows if row["status"] == "blocked")
    line = (f"members {members} · non-members {len(admissible) - members} · blocked {blocked} · "
            f"pending {600 - len(admissible) - blocked}")
    panel = d02_run.texts(at(dumps, ops, entry["at"]), "panel:jumble.cap_map")
    problems = []
    if line not in panel:
        problems.append(f"{entry['at']}: the threshold counts read {said(panel, 'members')}, not {line!r}")
    if f"N_c ≥ {k}" not in panel:
        problems.append(f"{entry['at']}: the threshold field does not read N_c ≥ {k}")
    return problems


@j0_check("blocked-undefined")
def j0_blocked(dumps: list[dict], ops: list[dict], entry: dict, state: str) -> list[str]:
    rows, problems = sample(state, "rim.json")["rows"], []
    for ref, cap in entry["at"]:
        row, dump = rows[cap], at(dumps, ops, ref)
        if row["status"] == "admissible":
            raise ValueError(f"j0-session.json names cap {cap} as blocked, but the sample admits it")
        panel, local = d02_run.texts(dump, "panel:jumble.cap_map"), d02_run.texts(dump, "view:Local")
        if not any(text.startswith(tier1(row)) for text in panel):
            problems.append(f"{ref}: the cap map's panel reads {said(panel, f'cap {cap} ·')}, not {tier1(row)!r}")
        if f"cap {cap} is blocked or uncertain" not in local:
            problems.append(f"{ref}: Local does not say that cap {cap} is blocked or uncertain")
        repair = dump.get("repair") or {}
        if repair.get("rows") != 0 or not any(text in local for text in ("▾ Repair list · 0", "▸ Repair list · 0")):
            problems.append(f"{ref}: the repair list counts {repair.get('rows')} rows at the blocked cap "
                            f"({said(local, '▾ Repair list') + said(local, '▸ Repair list')})")
        if dump.get("covers") or not any(text in local for text in ("▾ Cap covers · 0", "▸ Cap covers · 0")):
            problems.append(f"{ref}: Cap covers answers at the blocked cap ({said(local, '▸ Cap covers')} "
                            f"{(dump.get('covers') or [''])[0][:80]!r})")
    return problems


@j0_check("covers")
def j0_covers(dumps: list[dict], ops: list[dict], entry: dict, state: str) -> list[str]:
    problems = []
    for ref, cap in entry["at"]:
        record, dump = sample(state, f"cap-{cap}.json"), at(dumps, ops, ref)
        want, got = d02_run.covers_lines(record), dump.get("covers") or []
        for i, line in enumerate(want):
            if i >= len(got) or got[i] != line:
                problems.append(f"{ref}: Cap covers line {i + 1} reads {(got[i] if i < len(got) else None)!r:.160}, "
                                f"the record gives {line!r:.160}")
                break
        if len(got) > len(want):
            problems.append(f"{ref}: Cap covers shows {len(got) - len(want)} lines more than the record gives")
        count = len(record["covers"]["domain"])
        if f"▾ Cap covers · {count}" not in d02_run.texts(dump, "view:Local"):
            problems.append(f"{ref}: Local's open Cap covers section does not count {count} meeting caps")
    return problems


def state_glyph(text: str) -> bool:
    """ADR-D01.25's check-state glyph (ScriptList.StateGlyph) that ends the Repair list's column row: the icon font's
    private-use character, or the state icon's fallback word when the font is missing."""
    if len(text) == 1 and 0xE000 <= ord(text) <= 0xF8FF:
        return True
    icons = d02_run.load(ROOT / "tools" / "looklab" / "data" / "design" / "icons.json")["icons"]
    return text in {icons[name]["fallback"] for name in STATE_ICONS}


def repair_shown(dump: dict) -> tuple[int | None, list[tuple]]:
    """Local's open Repair list: its header's count and its rows (#, (x, d), (y, z), ΔN) as the screen lists them,
    read past the column row's check-state glyph."""
    local = d02_run.texts(dump, "view:Local")
    head = next((i for i, text in enumerate(local) if text.startswith("▾ Repair list · ")), None)
    if head is None:
        return None, []
    count = int(local[head].rsplit(" ", 1)[1].replace(",", ""))
    i, rows = head + 5, []
    if local[head + 1:head + 5] != ["#", "x d", "y z", "ΔN"]:
        return count, rows
    if i < len(local) and state_glyph(local[i]):
        i += 1
    while i + 3 < len(local) and local[i].isdigit() and PAIR.match(local[i + 1]) and PAIR.match(local[i + 2]) \
            and SIGNED.match(local[i + 3]):
        rows.append((int(local[i]), tuple(map(int, local[i + 1].split())), tuple(map(int, local[i + 2].split())),
                     int(local[i + 3].replace("−", "-"))))
        i += 4
    return count, rows


def repair_record(state: str, cap: int) -> list[tuple]:
    return [((x, d), (y, z), dn) for x, d, y, z, dn in sample(state, f"cap-{cap}.json")["repair_list"]["completed"]["rows"]]


@j0_check("repair-rows")
def j0_repair_rows(dumps: list[dict], ops: list[dict], entry: dict, state: str) -> list[str]:
    problems = []
    for ref, cap in entry["at"]:
        want, dump = repair_record(state, cap), at(dumps, ops, ref)
        count, shown = repair_shown(dump)
        if count != len(want) or (dump.get("repair") or {}).get("rows") != len(want):
            problems.append(f"{ref}: the repair list counts {count} rows ({(dump.get('repair') or {}).get('rows')} in "
                            f"the list), the record {len(want)}")
        got = sorted((xd, yz) for _, xd, yz, _ in shown)
        if got != sorted((xd, yz) for xd, yz, _ in want):
            extra = sorted(set(got) - {(xd, yz) for xd, yz, _ in want})[:3]
            lost = sorted({(xd, yz) for xd, yz, _ in want} - set(got))[:3]
            problems.append(f"{ref}: the list shows {len(got)} rows, not the record's {len(want)} (not in the record: "
                            f"{extra}; not shown: {lost})")
    return problems


@j0_check("repair-order")
def j0_repair_order(dumps: list[dict], ops: list[dict], entry: dict, state: str) -> list[str]:
    problems = []
    for ref, cap in entry["at"]:
        want, dump = repair_record(state, cap), at(dumps, ops, ref)
        _, shown = repair_shown(dump)
        # The shell refuses a list whose rows break the (x, d) order (RepairListModel.Read) and says so on its lamp.
        refused = str((dump.get("errors") or {}).get("last") or "")
        if "(x, d) order" in refused:
            problems.append(f"{ref}: the shell refused the repair list: {refused[:160]}")
        if (dump.get("repair") or {}).get("focused") != 0:
            problems.append(f"{ref}: row {(dump.get('repair') or {}).get('focused')} is focused on open")
        if [n for n, _, _, _ in shown] != list(range(1, len(shown) + 1)):
            problems.append(f"{ref}: the # column does not count 1 to {len(shown)}")
        got, order = [(xd, yz) for _, xd, yz, _ in shown], [(xd, yz) for xd, yz, _ in want]
        if sorted(got) == sorted(order) and got != order:
            first = next(i for i, (a, b) in enumerate(zip(got, order)) if a != b)
            problems.append(f"{ref}: row {first + 1} shows {got[first]}, the record's row {first + 1} is {order[first]}")
    return problems


@j0_check("delta-n")
def j0_delta_n(dumps: list[dict], ops: list[dict], entry: dict, state: str) -> list[str]:
    problems = []
    for ref, cap in entry["at"]:
        want = {(xd, yz): dn for xd, yz, dn in repair_record(state, cap)}
        _, shown = repair_shown(at(dumps, ops, ref))
        wrong = [(n, xd, yz, dn, want[(xd, yz)]) for n, xd, yz, dn in shown if (xd, yz) in want and want[(xd, yz)] != dn]
        if wrong:
            n, xd, yz, dn, value = wrong[0]
            problems.append(f"{ref}: {len(wrong)} rows show another ΔN (row {n}, x d {xd}, y z {yz}: {dn}, the record "
                            f"{value})")
    return problems


@j0_check("never-hidden")
def j0_never_hidden(dumps: list[dict], ops: list[dict], entry: dict, state: str) -> list[str]:
    """Rule 5 as J1 reads it (never_hidden), with the bytes composed for the screen's tint mode (style_expected)."""
    tables = d02_run.tint_tables(state)
    problems = []
    for ref, cap, focus in entry["at"]:
        dump = at(dumps, ops, ref)
        overlay, tint = dump.get("overlay"), dump.get("tint")
        if not isinstance(overlay, dict) or not isinstance(tint, dict):
            problems.append(f"{ref}: the screen has no overlay or tint")
            continue
        want = d02_run.overlay_expected(tables, state, cap, None, focus, tint.get("hover"), tint.get("selection"))
        line = d02_run.overlay_words(want)
        want["outline_sha256"], want["fill_sha256"] = style_expected(tables, state, tint.get("mode"), cap, focus,
                                                                     tint.get("hover"), tint.get("selection"))
        for key, value in [*want.items(), ("line", line), ("global_line", line), ("local_line", line)]:
            shown = overlay if key not in ("outline_sha256", "fill_sha256", "focus") else tint
            if shown.get(key) != value:
                problems.append(f"{ref}: {key} is {str(shown.get(key))[:100]!r}, not {str(value)[:100]!r}")
        if want["never_hidden"] and overlay.get("kept_dimmed"):
            problems.append(f"{ref}: {overlay['kept_dimmed']} never-hidden pieces carry the dim bit")
    return problems


def style_expected(tables: dict, state: str, mode: str, cap: int | None, focus: str | None = None,
                   hover: int | None = None, selection: int | None = None) -> tuple[str, str]:
    """The sha256 of the outline and fill bytes sent to the core at a tint screen, tint_expected and overlay_expected
    composed: the mode's outline (the contact classes, or none for the height ramp) with the named cap's lines as entry
    15, then the hover's and the selection's outlines; the mode's fill (the height ramp, or none) with the dim bit on
    every piece outside the focused domain that is neither a line nor never hidden."""
    import hashlib
    ids, domain = tables["ids"], tables["domain"]
    pieces, rows = len(ids), range(len(domain))
    outline_row, fill_row = [0] * len(rows), [0] * len(rows)
    if mode == "contact":
        outline_row = [d02_run.class_entry(tables["classes"][domain[r]]) for r in rows]
    elif mode == "height":
        fill_row = [d02_run.height_entry(tables["height"][r]) for r in rows]
    elif mode == "off-lattice":
        outline_row = [0 if tables["lattice"][r] else 14 for r in rows]
    lines = never = None
    if cap is not None:
        masks = sample(state, f"cap-{cap}.json").get("masks", {})
        never = d02_run.overlay_mask(masks["straddlers"], pieces) if "straddlers" in masks else None
        lines = never or (d02_run.overlay_mask(masks["inside"], pieces) if "inside" in masks else None)
    outline = bytearray(outline_row[p] for p in ids)
    fill = bytearray(fill_row[p] for p in ids)
    for p in range(pieces):
        if lines and lines[p]:
            outline[p] = 15
        if focus is not None and domain[ids[p]] != d02_run.OVERLAY_DOMAIN and not (lines and lines[p])                 and not (never and never[p]):
            fill[p] |= 0x80
    if hover is not None:
        outline[hover] = 2
    if selection is not None:
        outline[selection] = 1
    return hashlib.sha256(bytes(outline)).hexdigest(), hashlib.sha256(bytes(fill)).hexdigest() if any(fill) else ""


@j0_check("style-bytes")
def j0_style_bytes(dumps: list[dict], ops: list[dict], entry: dict, state: str) -> list[str]:
    """At each tint screen, with the cap named there: the outline and fill bytes piece by piece (style_expected), and
    the mode, the legend on both borders and the focus as tint_expected gives them (the contact-classes check reads
    the classes)."""
    tables = d02_run.tint_tables(state)
    problems = []
    for ref, mode, cap in entry["at"]:
        tint = at(dumps, ops, ref).get("tint")
        if not isinstance(tint, dict):
            problems.append(f"{ref}: the screen has no tint")
            continue
        want = d02_run.tint_expected(tables, mode, None, tint.get("hover"), tint.get("selection"))
        want["outline_sha256"], want["fill_sha256"] = style_expected(tables, state, mode, cap, None, tint.get("hover"),
                                                                     tint.get("selection"))
        for key, value in want.items():
            if key != "classes" and tint.get(key) != value:
                problems.append(f"{ref} ({mode}): {key} is {str(tint.get(key))[:100]!r}, not {str(value)[:100]!r}")
    return problems


@j0_check("contact-classes")
def j0_contact_classes(dumps: list[dict], ops: list[dict], entry: dict, state: str) -> list[str]:
    contact, problems = sample(state, "contact.json"), []
    for ref in entry["at"]:
        classes = (at(dumps, ops, ref).get("tint") or {}).get("classes")
        if not isinstance(classes, list) or len(classes) != len(contact["classes"]):
            problems.append(f"{ref}: the tint shows {str(classes)[:80]!r} as the contact classes")
            continue
        reused = [(a, b) for a, b, *_ in contact["edges"] if classes[a] and classes[a] == classes[b]]
        if reused:
            a, b = reused[0]
            problems.append(f"{ref}: domains {a} and {b} touch and both show class {classes[a]} ({len(reused)} contacts)")
        differ = [d for d, (got, want) in enumerate(zip(classes, contact["classes"])) if got != want]
        if differ:
            problems.append(f"{ref}: {len(differ)} domains show another class than the engine's (domain {differ[0]}: "
                            f"{classes[differ[0]]}, the engine {contact['classes'][differ[0]]})")
    return problems


def claim_verdict(row: dict) -> tuple[str, str]:
    """A research claim's verdict words and count from its latest check, as the Claims tab writes them."""
    check = row.get("latest_check")
    if check is None:
        return "not checked", "no_check"
    if check.get("outcome") == "cut_off" or (check.get("reading") or {}).get("kind") == "cut_off":
        return "not found within budget", "no_check"
    kinds = {verdict.get("kind") for verdict in check.get("verdicts", [])}
    if "fail" in kinds:
        return f"fails within {row['scope_phrase']}", "fails"
    if kinds == {"not_applicable"}:
        return "not applicable (scope only)", "no_check"
    if kinds == {"pass"}:
        return f"holds within {row['scope_phrase']}", "holds"
    raise ValueError(f"no verdict word for {row['group']}'s latest check")


def claims_problems(dump: dict, ref: str, cut_off: bool) -> list[str]:
    """The Claims tab against the research sample: with `cut_off`, the rows whose latest check was cut off; without,
    the other rows, the counts and the head."""
    panel = d02_run.load(PANEL)
    rows = panel["answers"]["research.claims"]["answer"]["rows"]
    research = dump.get("research") or {}
    shown = {line.get("ref"): line.get("verdict") for line in research.get("lines", []) if line.get("kind") == "Row"}
    problems = []
    counts = {"groups": len(rows), "holds": 0, "fails": 0, "no_check": 0}
    for row in rows:
        words, count = claim_verdict(row)
        counts[count] += 1
        is_cut = words == "not found within budget"
        got = shown.get("research.claim/" + row["group"])
        if is_cut == cut_off and got != words:
            problems.append(f"{ref}: {row['group'].split(':')[-1]} reads {got!r}, its latest check {words!r}")
    if not cut_off:
        head = (f"{counts['groups']} claim groups: holds {counts['holds']} · fails {counts['fails']} · "
                f"no check {counts['no_check']} · evidence: {panel['evidence']} · current")
        if research.get("counts") != counts or research.get("head") != head:
            problems.append(f"{ref}: the head reads {research.get('head')!r}, not {head!r}")
    return problems


@j0_check("verdict")
def j0_verdict(dumps: list[dict], ops: list[dict], entry: dict, state: str) -> list[str]:
    return claims_problems(at(dumps, ops, entry["at"]), entry["at"], cut_off=False)


@j0_check("cut-off")
def j0_cut_off(dumps: list[dict], ops: list[dict], entry: dict, state: str) -> list[str]:
    return claims_problems(at(dumps, ops, entry["at"]), entry["at"], cut_off=True)


def j0_problems(dumps: list[dict], ops: list[dict], session: dict) -> tuple[list[tuple[str, str]], list[str]]:
    """Every check's problems on one run's screens, each with its rule, and the checks that could not read the screens
    (a field missing under a fault): those are run problems, never a catch of the check's rule."""
    found, unread = [], []
    for entry in session["checks"]:
        try:
            problems = J0_CHECKS[entry["rule"]](dumps, ops, entry, session["state"])
        except Exception as error:  # any check that cannot read the screens: a run problem, and the run goes on
            unread.append(f"[{entry['rule']}] the screens could not be read: {error!r}"[:200])
            continue
        found += [(entry["rule"], problem) for problem in problems]
    return found, unread


def stable(first: list[dict], second: list[dict]) -> set[tuple[int, str]]:
    """The (screen, field) pairs that read the same in both clean runs."""
    return {(i, key) for i, (a, b) in enumerate(zip(first, second)) for key in a.keys() & b.keys()
            if key not in HARNESS_KEYS and a[key] == b[key]}


def visible(dumps: list[dict], clean: list[dict], keys: set[tuple[int, str]]) -> list[str]:
    """The fields a faulted run shows otherwise than the clean runs, where the two clean runs agree (a diagnostic: a
    defect the screen never shows cannot be caught from it)."""
    return sorted({key for i, key in keys if i < len(dumps) and dumps[i].get(key) != clean[i].get(key)})


def play_j0(label: str, data: Path, runs: Path, build: Path | None, session: dict,
            script: Path) -> tuple[list[dict], list[str]]:
    """Plays the J0 session in a fresh product shell on its state's fixture, in sample mode, reading data/samples;
    answers its dumps and the run's problems (the window's checks, the steps written)."""
    d02_run.DATA_FOR["j0"] = lambda out, folder=data: folder
    code, record = d02_run.run("j0", runs / label, build, session["state"], path=script)
    screen_file = runs / label / d02_run.SCREEN
    dumps = [json.loads(line) for line in screen_file.read_text(encoding="utf-8").splitlines() if line.strip()] \
        if screen_file.is_file() else []
    problems = list(record["mismatches"])
    steps, _ = expand(session["steps"], {}, J0_KINDS)
    wanted = ["start"] + [expected_do(step) for step in steps]
    if dumps and [dump["do"] for dump in dumps] != wanted:
        problems.append(f"the shell wrote {len(dumps)} steps, not the session's {len(wanted)}")
        dumps = []
    if not dumps and not problems:
        problems.append("the shell wrote no screens")
    return dumps, problems


def defects_digest(defects: list[dict]) -> str:
    """The sha256 of the defects as loaded, in order (canonical JSON)."""
    return hashlib.sha256(json.dumps(defects, sort_keys=True, ensure_ascii=False).encode("utf-8")).hexdigest()


def packet_seed(digest: str, nonce: str) -> int:
    """The packet's draw seed: the defects' digest with a nonce that only the sealed key keeps, so the draw cannot be
    redone from the source."""
    return int(hashlib.sha256(f"{digest}:{nonce}".encode("utf-8")).hexdigest()[:16], 16)


def leaked(text: str, out: Path) -> list[str]:
    """What a packet file must not hold: a word naming a planted copy, a defect or a run's label, or the out folder's
    path as written, JSON-escaped or with forward slashes, in any case."""
    low = text.lower()
    paths = set()
    for place in (out, out.resolve()):
        paths |= {str(place), place.as_posix(), str(place).replace("\\", "\\\\")}
    return [word for word in LEAKS if word in low] + sorted(p for p in paths if p.lower() in low)


def write_packet(root: Path, runs: dict[str, list[dict]], defects: list[dict], script: list[dict], state: str,
                 nonce: str | None = None) -> dict:
    """The researcher's packet (section 7): the screens of 10 runs of the fixed session, 8 faulted and 2 clean, chosen
    and ordered by a seeded draw and named run-01 to run-10, with the session's steps and the rules an answer names;
    the key, with the draw's nonce, goes to sealed/key.json. A packet file that would name a planted copy, a defect, a
    run's label or the out folder is not written, and the packet fails."""
    nonce = secrets.token_hex(16) if nonce is None else nonce
    digest = defects_digest(defects)
    rng = random.Random(packet_seed(digest, nonce))
    by_id = {defect["id"]: defect for defect in defects}
    ready = [defect["id"] for defect in defects if runs.get(defect["id"])]
    chosen = rng.sample(ready, min(FAULTED, len(ready))) + [label for label in CLEAN_RUNS if runs.get(label)]
    rng.shuffle(chosen)
    packet, problems, key = root / "packet", [], {}
    files = {"steps.json": json.dumps({"state": state, "steps": script}, indent=1, ensure_ascii=False) + "\n",
             "rules.json": json.dumps(RULES, indent=1, ensure_ascii=False) + "\n"}
    for n, label in enumerate(chosen, 1):
        name = f"run-{n:02d}"
        files[f"{name}.jsonl"] = "".join(json.dumps({k: v for k, v in dump.items() if k not in HARNESS_KEYS},
                                                    ensure_ascii=False) + "\n" for dump in runs[label])
        key[name] = {"label": label, "broken": label in by_id, "rule": by_id[label]["rule"] if label in by_id else None}
    for name, text in files.items():
        leaks = leaked(text, root.parent)
        if leaks:
            problems.append(f"packet {name} would leak {leaks}; not written")
            continue
        (packet / name).write_text(text, encoding="utf-8", newline="\n")
    (root / "sealed" / "key.json").write_text(json.dumps({"format": KEY_FORMAT, "nonce": nonce, "defects_sha256": digest,
                                                          "runs": key}, indent=2) + "\n", encoding="utf-8", newline="\n")
    return {"runs": len(chosen), "faulted": sum(1 for v in key.values() if v["broken"]),
            "clean": sum(1 for v in key.values() if not v["broken"]), "defects_sha256": digest, "problems": problems}


def j0_passed(record: dict) -> bool:
    """J0's bar: enough planted and caught, the replay bound, and no false alarm (the driver's or the subagent's), run
    problem, packet problem or subagent over section 7's cap."""
    replayed, sub = record["replay"], record.get("subagent") or {}
    return (record["planted"] >= LEAST and record["caught_any"] >= SHARE * record["planted"]
            and replayed["words"] == 9 and replayed["bound"] == 9 and not replayed["problems"]
            and not record["false_alarms"] and not record["run_problems"] and not record["packet"]["problems"]
            and not sub.get("false_alarms") and not sub.get("over_cap"))


def j0(session: dict, defects: list[dict], out: Path, build: Path | None, player=play_j0, replayer=replay,
       writer: Path | None = None, nonce: str | None = None) -> tuple[int, dict]:
    """J0's run: the seed 7001 replay, then the session once on a clean copy, once per defect and once more clean;
    the checks on every run, each defect caught by the driver when a problem names its rule (never a rule outside the
    driver's); a check that cannot read a run's screens is a run problem; the packet; the record (j0/track-j0.json)
    and the report (j0/track-j0.md)."""
    root = out / "j0"
    for name in ("plant", "runs", "scripts", "packet", "sealed"):
        (root / name).mkdir(parents=True)
    replayed = replayer(root, writer)
    script, ops = expand(session["steps"], {}, J0_KINDS)
    path = root / "scripts" / "j0.json"
    path.write_text(json.dumps({"format": "magic600-d02-script/1", "name": "j0", "resolution": session["resolution"],
                                "note": "J0's fixed session", "steps": script}, indent=2) + "\n",
                    encoding="utf-8", newline="\n")
    by_id = {defect["id"]: defect for defect in defects}
    runs: dict[str, list[dict]] = {}
    run_problems: dict[str, list[str]] = {}
    changed: dict[str, list[str]] = {}
    for label in [CLEAN_RUNS[0], *by_id, CLEAN_RUNS[1]]:
        folder = root / "plant" / label
        changed[label] = plant(by_id.get(label, {"id": label, "change": []}), folder)
        runs[label], problems = player(label, folder, root / "runs", build, session, path)
        if problems:
            run_problems[label] = problems
    found: dict[str, list[tuple[str, str]]] = {}
    for label, dumps in runs.items():
        found[label], unread = j0_problems(dumps, ops, session) if dumps else ([], [])
        if unread:
            run_problems.setdefault(label, []).extend(unread)
    false_alarms = [f"{label}: [{rule}] {problem}" for label in CLEAN_RUNS for rule, problem in found[label]]
    clean = runs[CLEAN_RUNS[0]]
    keys = stable(clean, runs[CLEAN_RUNS[1]]) if clean and runs[CLEAN_RUNS[1]] else set()
    rows = []
    for defect in defects:
        dumps = runs[defect["id"]]
        rules = sorted({rule for rule, _ in found[defect["id"]]})
        rows.append({"id": defect["id"], "by": defect["by"], "class": defect["class"], "rule": defect["rule"],
                     "files": changed[defect["id"]], "note": defect.get("note", ""), "screens": bool(dumps),
                     "outside": defect["rule"] not in RULES, "caught_subagent": None,
                     "caught_driver": defect["rule"] in RULES and defect["rule"] in rules,
                     "flagged": rules, "problems": [f"[{rule}] {problem}" for rule, problem in found[defect["id"]]][:8],
                     "visible": visible(dumps, clean, keys) if dumps and keys else None})
    packet = write_packet(root, runs, defects, script, session["state"], nonce)
    caught = sum(1 for row in rows if row["caught_driver"])
    record = {"format": "magic600-d02-track-j0-v1", "bar": "j0", "planted": len(rows), "caught_driver": caught,
              "caught_subagent": None, "caught_any": caught, "least": LEAST, "share": SHARE,
              "by": {who: sum(1 for row in rows if row["by"] == who) for who in sorted({row["by"] for row in rows})},
              "replay": replayed, "defects": rows,
              "classes": {name: sum(1 for row in rows if row["class"] == name) for name in CLASSES},
              "not_plantable": NOT_PLANTABLE, "false_alarms": false_alarms, "run_problems": run_problems,
              "packet": packet, "subagent": None,
              # The subagent's reach half (one key script over at most 30 display names) has no driver function yet.
              "reach": None}
    record["not_planted"] = [name for name, count in record["classes"].items() if not count]
    record["passed"] = j0_passed(record)
    write_j0(root, record)
    return (0 if record["passed"] else 1), record


def load_effort(path: Path) -> dict:
    """The subagent's effort record, {"scripts", "dumps", "minutes", "words"}, each a number: the scripts it wrote, the
    dumps it read, the minutes and the words of its output, as Opus counted them while running its pass."""
    effort = json.loads(path.read_text(encoding="utf-8"))
    bad = [name for name in CAP if not isinstance(effort, dict) or isinstance(effort.get(name), bool)
           or not isinstance(effort.get(name), (int, float)) or not math.isfinite(effort[name]) or effort[name] < 0]
    if bad:
        raise ValueError(f"{path.name}: the effort record needs a number for {bad}")
    return {name: effort[name] for name in CAP}


def load_answers(path: Path, runs: dict) -> tuple[dict, str]:
    """The subagent's answers, one per packet run, each {"broken": true or false, "rules": [rule names]}, and the
    file's sha256. Anything else is refused, never read as broken."""
    raw = path.read_bytes()
    answers = json.loads(raw.decode("utf-8"))
    if not isinstance(answers, dict) or set(answers) != set(runs):
        named = sorted(answers) if isinstance(answers, dict) else type(answers).__name__
        raise ValueError(f"the answers name {named}, the packet {sorted(runs)}")
    for run, answer in answers.items():
        if not isinstance(answer, dict) or not isinstance(answer.get("broken"), bool):
            raise ValueError(f"{run}: broken must be true or false")
        rules = answer.get("rules", [])
        if not isinstance(rules, list) or not all(isinstance(rule, str) for rule in rules):
            raise ValueError(f"{run}: rules must be a list of rule names")
    return answers, hashlib.sha256(raw).hexdigest()


def score(out: Path, answers_path: Path, effort_path: Path) -> tuple[int, dict]:
    """Scores the researcher subagent's answers against the sealed key and its effort against section 7's cap, and
    writes J0's record and report again. A faulted run is caught when the answer says broken and names the defect's
    rule; for a rule outside the driver's, a rule in the subagent's own words, which the record keeps for the
    integrator to read. A clean run said broken is a false alarm and fails J0, as does going over the cap. Each
    earlier scoring stays in the record's history."""
    root = out / "j0"
    record = json.loads((root / J0_RECORD).read_text(encoding="utf-8"))
    key = json.loads((root / "sealed" / "key.json").read_text(encoding="utf-8"))
    if key.get("format") != KEY_FORMAT:
        raise ValueError(f"{root / 'sealed' / 'key.json'} is not a {KEY_FORMAT} file")
    answers, digest = load_answers(answers_path, key["runs"])
    effort = load_effort(effort_path)
    judged, false_alarms = {}, []
    for run, truth in key["runs"].items():
        broken, rules = answers[run]["broken"], answers[run].get("rules", [])
        if truth["broken"]:
            own = [rule for rule in rules if rule not in RULES]
            named = truth["rule"] in rules if truth["rule"] in RULES else bool(own)
            judged[truth["label"]] = {"run": run, "broken": broken, "caught": broken and named, "own_words": own}
        elif broken:
            false_alarms.append(run)
    if record.get("subagent"):
        record.setdefault("history", []).append(record["subagent"])
    for row in record["defects"]:
        row["caught_subagent"] = judged[row["id"]]["caught"] if row["id"] in judged else None
    record["caught_subagent"] = sum(1 for row in record["defects"] if row["caught_subagent"])
    record["caught_any"] = sum(1 for row in record["defects"] if row["caught_driver"] or row["caught_subagent"])
    record["subagent"] = {"answers": answers_path.name, "answers_sha256": digest, "effort": effort, "cap": CAP,
                          "over_cap": [f"{name} {effort[name]} over {CAP[name]}" for name in CAP
                                       if effort[name] > CAP[name]],
                          "faulted": len(judged), "caught": record["caught_subagent"],
                          "wrong_rule": sum(1 for v in judged.values() if v["broken"] and not v["caught"]),
                          "missed": sum(1 for v in judged.values() if not v["broken"]),
                          "own_words": {v["run"]: v["own_words"] for v in judged.values() if v["own_words"]},
                          "false_alarms": false_alarms}
    record["passed"] = j0_passed(record)
    write_j0(root, record)
    return (0 if record["passed"] else 1), record


def line_j0(record: dict) -> str:
    sub, reach = record["caught_subagent"], record.get("reach")
    return (f"M600_D02_J0_{'PASS' if record['passed'] else 'FAIL'} planted={record['planted']} "
            f"caught_driver={record['caught_driver']} caught_subagent={'not_run' if sub is None else sub} "
            f"caught_any={record['caught_any']} replay={record['replay']['bound']}/{record['replay']['words']} "
            f"not_planted={len(record['not_planted'])} reach={'not_run' if reach is None else reach}")


def cell(text) -> str:
    return str(text).replace("|", "\\|").replace("\n", " ")


def write_j0(root: Path, record: dict) -> None:
    """J0's record and its report: the result line, the replay, each defect, the classes, the false alarms, the run
    problems and the packet. Paths in the report are relative to the out folder."""
    (root / J0_RECORD).write_text(json.dumps(record, indent=2, ensure_ascii=False) + "\n", encoding="utf-8",
                                  newline="\n")
    replayed, sub = record["replay"], record["subagent"]
    lines = ["# Track J0, reduced run", "", "```", line_j0(record), "```", "",
             f"Planted {record['planted']} defects ({', '.join(f'{who} {n}' for who, n in record['by'].items()) or 'none'}). "
             f"J0 passes with at least {record['least']} planted, caught_any at least {record['share']:.0%} of them, "
             "all nine seed 7001 words bound, no problem on the two clean runs and no run problem. A defect is caught "
             "by the driver when one of its run's problems names the defect's rule; a defect whose rule is outside the "
             "driver's counts as planted, and only the researcher subagent can catch it. The subagent catches a "
             "faulted run when it says broken and names the defect's rule; its false alarms on the clean runs, and "
             "going over section 7's cap, fail J0 too. caught_any counts a defect the driver or the subagent caught.",
             "",
             "## Seed 7001 replay", "",
             f"The host's fixture writer executes {replayed['start']}'s word and each word after it; the state after "
             "each word binds to the sample the word file names (all three digests).", "",
             "| Word | State after | Letters | Bound | Digest | Problem |",
             "| --- | --- | --- | --- | --- | --- |"]
    lines += [f"| {r['word']} | {r['after']} | {len(r['letters'])} | {'yes' if r['bound'] else 'no'} | "
              f"{r.get('digest', '')[:12]} | {cell(r.get('problem', ''))} |"
              for r in replayed["rows"]]
    lines += [f"- {cell(problem)}" for problem in replayed["problems"]]
    lines += ["", "## Defects", "",
              "| Id | By | Class | Rule | Driver | Other rules flagged | Subagent | Shown on screen |",
              "| --- | --- | --- | --- | --- | --- | --- | --- |"]
    for row in record["defects"]:
        others = [rule for rule in row["flagged"] if rule != row["rule"]]
        subagent = "not in packet" if row["caught_subagent"] is None else "yes" if row["caught_subagent"] else "no"
        if sub is None:
            subagent = "not run"
        shown = "no screens" if not row["screens"] else "unknown" if row["visible"] is None else \
            (", ".join(row["visible"][:4]) or "no change")
        driver = "caught" if row["caught_driver"] else "outside its rules" if row.get("outside") else "missed"
        lines.append(f"| {row['id']} | {row['by']} | {cell(row['class'])} | {row['rule']} | "
                     f"{driver} | {', '.join(others) or '-'} | {subagent} | "
                     f"{cell(shown)} |")
    lines += ["", "First problems of each defect's run:", ""]
    for row in record["defects"]:
        lines.append(f"- {row['id']} ({', '.join(row['files'])}): " +
                     ("; ".join(cell(p) for p in row["problems"][:3]) or "no problem"))
    lines += ["", "## Classes", "", "| Class | Planted | Not planted because |", "| --- | --- | --- |"]
    lines += [f"| {cell(name)} | {count} | {cell(record['not_plantable'].get(name, '') if not count else '')} |"
              for name, count in record["classes"].items()]
    lines += ["", f"{len(record['not_planted'])} of {len(record['classes'])} classes are not planted. No fault switch "
              "was built: each defect is data in its own copy of the samples, so a class that only the shell's code "
              "could show is not planted; the table gives each reason J0 knows."]
    lines += ["", "## False alarms on the clean runs", ""]
    lines += [f"- {cell(alarm)}" for alarm in record["false_alarms"]] or ["None."]
    lines += ["", "## Run problems", ""]
    lines += [f"- {label}: {cell(problem)}" for label, problems in record["run_problems"].items()
              for problem in problems] or ["None."]
    packet = record["packet"]
    lines += ["", "## The researcher's packet", "",
              f"`j0/packet/` holds {packet['runs']} runs ({packet['faulted']} faulted, {packet['clean']} clean, drawn "
              f"and ordered by a seed from the defects' digest, {packet.get('defects_sha256', '')[:12]}, and a nonce "
              "that only the key keeps), named run-01 onwards, with the session's steps and the rules an answer names; "
              "the key is in `j0/sealed/key.json`. The subagent reads only the packet, the palette's rows, the key map "
              "sheet and jumbling-observation.md, never the samples, the planted copies, the defect lists, the run "
              "folders or the key. Its answers ({\"run-01\": {\"broken\": true, \"rules\": [...]}, ...}) and Opus's "
              "count of its effort ({\"scripts\": n, \"dumps\": n, \"minutes\": n, \"words\": n}) are scored with "
              "`python renderer/shell/track_j.py --score OUT --subagent ANSWERS --effort EFFORT`."]
    lines += [f"- {cell(problem)}" for problem in packet["problems"]]
    if record.get("reach") is None:
        lines += ["", "The subagent's reach half (one key script over at most 30 display names, one revision) has no "
                      "driver function in this build: reach=not_run."]
    if sub is not None:
        effort = sub["effort"]
        cap = f"over section 7's cap ({'; '.join(sub['over_cap'])})" if sub["over_cap"] else "within section 7's cap"
        lines += ["", f"Subagent: caught {sub['caught']} of {sub['faulted']} faulted runs; said broken but named another "
                      f"rule for {sub['wrong_rule']}; missed {sub['missed']}; said a clean run was broken for "
                      f"{', '.join(sub['false_alarms']) or 'none'}. Effort: {effort['scripts']} scripts, "
                      f"{effort['dumps']} dumps, {effort['minutes']} minutes, {effort['words']} words, {cap}. "
                      f"Answers sha256 {sub['answers_sha256'][:12]}."]
        lines += [f"- {run}, in its own words: {cell('; '.join(words))}" for run, words in sub["own_words"].items()]
        if record.get("history"):
            lines += ["", f"Earlier scorings: {len(record['history'])}, kept in the record's history."]
    (root / J0_REPORT).write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--reduced", action="store_true", help="the reduced Track J run (section 7)")
    parser.add_argument("--only", choices=["j0", "j1"], help="one bar (default: J1, then J0)")
    parser.add_argument("--out", type=Path, help="the out folder, which must not exist yet")
    parser.add_argument("--build", type=Path, help="the product build (default: the newest)")
    parser.add_argument("--defects", type=Path, action="append", default=[],
                        help="a further defects file, such as Fable's sealed ten (defects.json is always read)")
    parser.add_argument("--writer", type=Path, help="the host's fixture writer for the seed 7001 replay")
    parser.add_argument("--score", type=Path, help="a J0 out folder to score with the researcher subagent's answers")
    parser.add_argument("--subagent", type=Path, help="the researcher subagent's answers (with --score)")
    parser.add_argument("--effort", type=Path, help="Opus's count of the subagent's scripts, dumps, minutes and words "
                                                     "(with --score)")
    args = parser.parse_args(argv)
    try:
        if args.score is not None:
            if args.subagent is None or args.effort is None:
                print("track_j: --score needs --subagent and --effort")
                return 2
            code, record = score(args.score.resolve(), args.subagent, args.effort)
            print(line_j0(record))
            return code
        if not args.reduced or args.out is None:
            print("track_j: name --reduced and --out (or --score OUT --subagent ANSWERS --effort EFFORT)")
            return 2
        out = args.out.resolve()
        if out.exists():
            print(f"track_j: {out} exists; name a new folder")
            return 2
        bars = [args.only] if args.only else ["j1", "j0"]
        for bar in bars:  # J0 and J1 read the screens themselves, below
            d02_run.EXPECT[bar] = lambda dumps, source, state: []
            d02_run.ON_FIXTURE.add(bar)
            d02_run.IN_SAMPLE_MODE.add(bar)
        session = load_session() if "j1" in bars else None
        j0_session = load_j0_session() if "j0" in bars else None
        defects = load_defects([DEFECTS, *args.defects]) if "j0" in bars else []
        codes = []
        if "j1" in bars:
            code, record = j1(session, out, args.build)
            codes.append(code)
            print(line(record, code == 0))
            for problem in record["problems"]:
                print("  " + problem)
        if "j0" in bars:
            code, record = j0(j0_session, defects, out, args.build, writer=args.writer)
            codes.append(code)
            print(line_j0(record))
            problems = record["false_alarms"] + record["replay"]["problems"]
            problems += [f"replay {row['word']}: {row['problem']}" for row in record["replay"]["rows"] if "problem" in row]
            problems += record["packet"]["problems"]
            problems += [f"{label}: {p}" for label, found in record["run_problems"].items() for p in found]
            for problem in problems:
                print("  " + problem[:300])
    except (ValueError, OSError) as error:
        print(f"track_j: {error}")
        return 2
    return max(codes)


if __name__ == "__main__":
    sys.exit(main())
