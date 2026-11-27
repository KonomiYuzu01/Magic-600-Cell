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
import json
import re
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


def expand(steps: list[dict], not_run: dict | None = None) -> tuple[list[dict], list[dict]]:
    """A part's steps as d02 script steps, and each op with the screen dumps of its typed line, its Enter and its last
    step (step i of the script is screen dump i + 1: dump 0 is the shell's start)."""
    not_run = not_run or {}
    script, ops = [], []
    for step in steps:
        if set(step) - (OP_KEYS if "op" in step else STEP_KINDS) or ("op" not in step and len(step) != 1):
            raise ValueError(f"a J1 step is an op (with optional argument, wait_ms, refused and close), a chord, a "
                             f"hold, a release, a text or a wait_ms: {step}")
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
    for kind in ("chord", "text", "hold", "release"):
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


def boundary(dumps: list[dict]) -> int:
    return max(((dump.get("stage") or {}).get("unasked", 0) for dump in dumps), default=0)


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
        unasked = max(unasked, boundary(dumps))
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
    observed, results = 0, []
    checked = [rule for rule in session["observation"] if rule["rule"] not in not_run]
    for rule in checked:
        dumps, ops, state = played.get(rule["part"], ([], [], ""))
        found = CHECKS[rule["check"]](dumps, ops, rule, state) if dumps else [f"part {rule['part']} wrote no screens"]
        results.append({"rule": rule["rule"], "part": rule["part"], "check": rule["check"], "problems": found})
        if found:
            problems += [f"observation {rule['rule']} ({rule['part']}): {problem}" for problem in found]
        else:
            observed += 1
    record = {"format": "magic600-d02-track-j-v1", "bar": "j1", "operations": len(wanted),
              "reached": sum(1 for row in wanted if row in reached), "boundary": unasked,
              "observation": [observed, len(checked)], "not_run": not_run, "rules": results,
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


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--reduced", action="store_true", required=True, help="the reduced Track J run (section 7)")
    parser.add_argument("--only", choices=["j1"], required=True, help="the bar to run; J0 comes with B33")
    parser.add_argument("--out", type=Path, required=True, help="the out folder, which must not exist yet")
    parser.add_argument("--build", type=Path, help="the product build (default: the newest)")
    args = parser.parse_args(argv)
    out = args.out.resolve()
    if out.exists():
        print(f"track_j: {out} exists; name a new folder")
        return 2
    try:
        d02_run.EXPECT["j1"] = lambda dumps, source, state: []  # J1 reads the screens itself, below
        d02_run.ON_FIXTURE.add("j1")
        d02_run.IN_SAMPLE_MODE.add("j1")
        code, record = j1(load_session(), out, args.build)
    except (ValueError, OSError) as error:
        print(f"track_j: {error}")
        return 2
    print(line(record, code == 0))
    for problem in record["problems"]:
        print("  " + problem)
    return code


if __name__ == "__main__":
    sys.exit(main())
