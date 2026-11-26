"""Write a jumble-on session fixture at each D02 sample state (ADR-D02 build step B5).

  python renderer/shell/build_d02_fixtures.py [--state ID ...] [--out DIR] [--writer EXE]

Each state's word is rebuilt from the research records its sample was made from
(`tools/looklab/fixtures/make_d02_samples.py`), as menu letters `[cap, item]` of the pinned I_a menu
(`research/jumbling/fixtures/wj-I_a.json`): a W-J journal record gives its grip and the base index of its label, and
the research words give their letters as they are, less any letter of the menu's identity item: it moves no piece, so
the state is the same, and the command layer refuses it (`KN2:input.recipe_shape`; the stop-40 restorer has four). The
host's fixture writer (`c600-shell-session-fixture --word FILE`,
renderer/bridge/host) commits each letter as `jumble.twist.menu` through the command layer, runs the correctness
control at the close, and reopens a copy once to time it. Its record names the head's digest and the two pose-array
digests, and this script checks all three against the sample's `state.json`: a sample binds to a session by them
(`ShellJumble.Bind`). A fixture whose digests differ is removed and the run fails.

Fixtures go to `work/build/d02-fixtures/<state>/` (ignored by Git): the fixture's `session/` and `fixture.json`, and
`word.json` beside them. A state whose fixture is there and matches is kept. `index.json` lists each state's letters,
digests, write time and reopen time (risk R2: a deep fixture replays about 3,300 commits). The writer is the bridge's
release build (`cargo build --release -p c600-shell-host`, which `build_product.py` runs). The two correction states
(`nu-c<id>`) are not built here: their words come from the adapter's `correction_lookup`, and B26 builds them. Nothing
here launches a window or uses a GPU.
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BRIDGE = ROOT / "renderer" / "bridge"
WRITER = BRIDGE / "target" / "release" / "c600-shell-session-fixture.exe"
ASSETS = ROOT / "assets"
SAMPLES = ROOT / "tools" / "looklab" / "data" / "samples" / "jumble"
OUT = ROOT / "work" / "build" / "d02-fixtures"
FORMAT = "magic600-d02-fixtures/1"
WORD_FORMAT = "magic600-d02-word/1"
# The research records, as make_d02_samples.py names them.
WJ = "research/jumbling/fixtures/wj-I_a.json"
DEEP = "research/jumbling/restoration/11-scramble-census/runs/heights/ia-k3000-s7302.json"
S7001 = "research/jumbling/restoration/11-scramble-census/runs/round1/ia-k10-s7001.json"
S7001_WORDS = "research/jumbling/restoration/13-tutorial-runner/runs/depth10-seed7001/words"
STOP40 = "research/jumbling/theory/results/seams-I_a-40-stop.json"
MENU = ROOT / WJ
# Not built here, with the step that builds them.
NOT_BUILT = {"nu-c0": "B26", "nu-c35": "B26"}
# make_d02_samples.py's H4_LETTERS (copied from research/jumbling/differential/leads/results/summary.md:497).
H4_LETTERS = [[64, 15], [185, 14], [357, 12], [471, 12], [263, 14], [407, 13], [523, 12],
              [124, 2], [274, 13], [46, 1], [46, 3], [104, 14]]
# The writer's last stdout lines name the reopen time: `reopen_ms: N`.
REOPEN = "reopen_ms: "


class FixtureError(Exception):
    """A fixture that was not written or does not match its sample."""


def read_json(relative: str) -> dict | list:
    return json.loads((ROOT / relative).read_text(encoding="utf-8"))


def journal_letters() -> list[list[int]]:
    """The W-J journal's 841 records as menu letters: each record's grip and its label's index in the menu base."""
    wj = read_json(WJ)
    base = {item["label"]: index for index, item in enumerate(wj["menu"]["base"])}
    letters = []
    for index, record in enumerate(wj["journal"]["records"]):
        label = record.get("params", {}).get("label")
        if record.get("family") != "menu" or label not in base:
            raise FixtureError(f"W-J record {index} is not a letter of the I_a menu")
        letters.append([record["grip"], base[label]])
    return letters


def words() -> dict[str, list[list[int]]]:
    """Every buildable state's word, from the records make_d02_samples.py builds it from."""
    journal = journal_letters()
    deep = read_json(DEEP)
    s7001 = read_json(S7001)
    out = {"on-zero": [], "wj-ia-300": journal[:300], "wj-ia-420": journal[:420], "wj-ia-end": journal,
           "ia-40": journal[:40] + [list(m) for m in read_json(STOP40)["moves"]],
           "deep-1": [list(m) for m in deep["history"]]}
    out["deep-2"] = out["deep-1"] + [list(m) for m in deep["moves"]] + H4_LETTERS
    word = [list(m) for m in s7001["history"]]
    out["s7001-0"] = list(word)
    for k in range(9):
        word += [list(m) for m in read_json(f"{S7001_WORDS}/{k:05d}.json")["word"]]
        out[f"s7001-{k + 1}"] = list(word)
    return out


def identity_items() -> set[int]:
    """The menu base's items whose matrix is the identity (exact entries `[a, b, c]` for (a + b*sqrt(5)) / c)."""
    base = read_json(WJ)["menu"]["base"]
    one, zero = [1, 0, 1], [0, 0, 1]
    return {index for index, item in enumerate(base)
            if all(item["matrix"][r][c] == (one if r == c else zero) for r in range(4) for c in range(4))}


def moving(letters: list[list[int]], identity: set[int]) -> tuple[list[list[int]], int]:
    """The letters without the identity item's, and how many were left out."""
    kept = [letter for letter in letters if letter[1] not in identity]
    return kept, len(letters) - len(kept)


def sample_digests(state: str) -> dict:
    return json.loads((SAMPLES / state / "state.json").read_text(encoding="utf-8"))["digests"]


def matches(record: dict, sample: dict) -> list[str]:
    """The digests in which a fixture's record differs from its sample's, by name."""
    got = {"adapter": record.get("digest"), **record.get("state_digests", {})}
    return [name for name in ("adapter", "pose_ids_sha256", "pose_lattice_sha256") if got.get(name) != sample[name]]


def write(writer: Path, state: str, letters: list[list[int]], out: Path, run=subprocess.run) -> dict:
    """Writes one fixture and answers its index row; FixtureError when it fails or does not match."""
    folder = out / state
    record_path = folder / "fixture.json"
    sample = sample_digests(state)
    if record_path.is_file():
        record = json.loads(record_path.read_text(encoding="utf-8"))
        if not matches(record, sample) and record.get("word") == letters:
            return {"state": state, "letters": len(letters), "kept": True}
        shutil.rmtree(folder)
    folder.parent.mkdir(parents=True, exist_ok=True)
    word_path = out / f"{state}.word.json"
    word_path.write_text(json.dumps({"format": WORD_FORMAT, "state": state, "letters": letters}) + "\n",
                         encoding="utf-8")
    started = time.perf_counter()
    done = run([str(writer), "--assets", str(ASSETS), "--menu", str(MENU), "--out", str(folder),
                "--word", str(word_path)], capture_output=True, text=True, timeout=3600)
    seconds = time.perf_counter() - started
    if done.returncode != 0:
        shutil.rmtree(folder, ignore_errors=True)
        raise FixtureError(f"{state}: the writer exited with {done.returncode}: {done.stderr.strip()[-600:]}")
    shutil.move(str(word_path), str(folder / "word.json"))
    record = json.loads(record_path.read_text(encoding="utf-8"))
    differ = matches(record, sample)
    if differ:
        shutil.rmtree(folder)
        raise FixtureError(f"{state}: the fixture's {', '.join(differ)} differ from the sample's")
    reopen = [line[len(REOPEN):] for line in done.stdout.splitlines() if line.startswith(REOPEN)]
    return {"state": state, "letters": len(letters), "kept": False, "write_seconds": round(seconds, 1),
            "reopen_ms": int(reopen[-1]) if reopen else None, "digest": record["digest"]}


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--state", action="append", help="a sample state to write (default: every buildable one)")
    parser.add_argument("--out", type=Path, default=OUT, help="the fixtures folder (default: work/build/d02-fixtures)")
    parser.add_argument("--writer", type=Path, default=WRITER, help="the host's c600-shell-session-fixture")
    args = parser.parse_args(argv)
    identity = identity_items()
    every = words()
    states = [s["state"] for s in json.loads((SAMPLES / "states.json").read_text(encoding="utf-8"))["states"]]
    chosen = args.state or [s for s in states if s in every]
    unknown = [s for s in chosen if s not in every]
    if unknown:
        print(f"d02 fixtures: no word for {', '.join(unknown)}"
              + "".join(f" ({s}: built at {NOT_BUILT[s]})" for s in unknown if s in NOT_BUILT))
        return 2
    if not args.writer.is_file():
        print(f"d02 fixtures: no writer at {args.writer}; build it with cargo build --release -p c600-shell-host")
        return 2
    out = args.out.resolve()
    index_path = out / "index.json"
    rows = {row["state"]: row for row in (json.loads(index_path.read_text(encoding="utf-8"))["states"]
                                          if index_path.is_file() else [])}
    failed = 0
    for state in chosen:
        letters, dropped = moving(every[state], identity)
        try:
            row = write(args.writer, state, letters, out)
        except FixtureError as error:
            print(f"FAIL {error}", flush=True)
            rows.pop(state, None)
            failed += 1
            continue
        if row["kept"] and state in rows:
            row = rows[state]
        row["identity_letters_left_out"] = dropped
        rows[state] = row
        print(f"PASS {state}: {row['letters']} letters" + ("" if row.get("write_seconds") is None else
              f", written in {row['write_seconds']} s, reopened in {row['reopen_ms']} ms"), flush=True)
        out.mkdir(parents=True, exist_ok=True)
        index_path.write_text(json.dumps({"format": FORMAT, "not_built": NOT_BUILT,
                                          "states": [rows[s] for s in states if s in rows]}, indent=1) + "\n",
                              encoding="utf-8")
    print(f"d02 fixtures: {len(chosen) - failed} of {len(chosen)} match their samples")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
