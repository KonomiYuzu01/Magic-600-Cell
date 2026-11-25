"""Build the 0.4 interchange fixtures of c600-log (LP4-13) with 0.4's own code, unchanged.

Run from the repository root in the pinned engine environment (CPython 3.14.7, NumPy 2.3.5):

    tools/.venv/engine/Scripts/python.exe rust/c600-log/tests/fixtures/make_fixtures.py

It reads `core.py`, `session.py`, `enhanced.py`, `log_io.py`, `mpult_log.py`, `native_bridge.py` and
`tests/test_native_bridge.py` and changes none of them. Sessions live in a fresh temporary directory, never a personal
session. The MPUlt profile is 0.4's synthetic one: `test_native_bridge.make_fixture` builds a native-style export with a
shuffled enumeration, and `native_bridge.verify_native` turns it into a `C600-NATIVE-PROFILE-v1` profile. It is not an
export of the actual MPUlt process, so the MPUlt fixtures are source and fixture evidence only.

Outputs, in this directory:
- `c600-session.c600.json.gz`: a 0.4 `save_log` file (gzip level 5) of words and stars, with preferences;
- `c600-scramble.c600.json`: an uncompressed 0.4 record of a 30-turn scramble and a manual turn, as `decode_log` reads it;
- `mpult-profile.json.gz`: the synthetic profile's bytes, gzipped for storage;
- `mpult-negative.log.gz`, `mpult-partial-redo.log.gz`, `mpult-export.log.gz`: MPUlt v1 logs from 0.4's `serialize`
  and `export_native`, gzipped for storage (the test reads the MPUlt bytes inside);
- `expected.json`: what 0.4's `validate_record` and `import_native` return for each fixture, and the sources' digests.
"""
from __future__ import annotations

import base64
import gzip
import hashlib
import json
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / "tests"))

from core import Model, canonical  # noqa: E402
from enhanced import Workflow  # noqa: E402
from log_io import decode_log, encode_log, validate_record  # noqa: E402
from mpult_log import export_native, import_native, native_colors, profile_data, serialize  # noqa: E402
from native_bridge import verify_native  # noqa: E402
from session import Session  # noqa: E402
from test_native_bridge import make_fixture  # noqa: E402

SOURCES = ("core.py", "session.py", "enhanced.py", "log_io.py", "mpult_log.py", "native_bridge.py",
           "tests/test_native_bridge.py", "native/runtime/MPUlt_puzzles.txt")


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def b64(data: bytes) -> str:
    return base64.b64encode(data).decode("ascii")


def plan_json(plan: dict) -> dict:
    events = [dict(recipe=e["recipe"], length=e["length"], stars=e["stars"], pre=e["pre"], post=e["post"],
                   note=e["note"], assistance=e["assistance"]) for e in plan["events"]]
    out = dict(transactions=plan["transactions"], primitive_count=plan["primitive_count"], state=plan["state"].hash,
               events=events)
    if "selected_count" in plan:
        out["selected_count"] = plan["selected_count"]
    if "native_log" in plan:
        out["native_log"] = plan["native_log"]
    return out


def write(name: str, data: bytes) -> None:
    (HERE / name).write_bytes(data)


def main() -> int:
    before = {name: sha((ROOT / name).read_bytes()) for name in SOURCES}
    model = Model()
    expected = dict(model_id=model.model_id, sources=before, c600={}, mpult={})
    with tempfile.TemporaryDirectory(prefix="c600-log-fixtures-") as work:
        work = Path(work)

        # A 0.4 session with words, stars of both signs and preferences, saved as 0.4 saves a proof log.
        s = Session(model, work / "c600")
        try:
            s.save_prefs(dict(orbit=2, pin_safety=False, protected=[]))
            for recipe in ([dict(kind="word", moves=[2, 4, 6])], [dict(kind="star", orbit=33, node=0, sign=1)],
                           [dict(kind="word", moves=[8, -8, 1200]), dict(kind="star", orbit=0, node=1, sign=-1)]):
                s.commit(s.preview(recipe)["token"])
            raw = s.export_log()
            assert raw == encode_log(s.export())
            plan = validate_record(model, decode_log(b64(raw)))
            assert plan["state"].hash == s.st.hash
            write("c600-session.c600.json.gz", raw)
            expected["c600"]["c600-session.c600.json.gz"] = dict(plan_json(plan), sha256=sha(raw), session=s.st.hash)
        finally:
            s.close()

        # A scrambled 0.4 session, written as uncompressed canonical JSON, which decode_log also reads.
        s = Session(model, work / "scramble")
        try:
            Workflow(s).scramble(30, 47, apply=True)
            s.commit(s.preview([dict(kind="word", moves=[-3, 5])])["token"])
            raw = canonical(s.export()).encode("utf-8")
            plan = validate_record(model, decode_log(b64(raw)))
            assert plan["state"].hash == s.st.hash
            write("c600-scramble.c600.json", raw)
            expected["c600"]["c600-scramble.c600.json"] = dict(plan_json(plan), sha256=sha(raw), session=s.st.hash)
        finally:
            s.close()

        # 0.4's synthetic native profile.
        export, _ = make_fixture(model)
        profile = verify_native(model, export, work / "profile")
        profile_bytes = (work / "profile" / "native_profile.json").read_bytes()
        assert json.loads(profile_bytes) == json.loads(canonical(profile))
        write("mpult-profile.json.gz", gzip.compress(profile_bytes, compresslevel=9, mtime=0))
        expected["profile_sha256"] = sha(profile_bytes)
        mapping, inverse, table, orders = profile_data(model, profile)

        def color_for(tokens):
            labels = model.ids.copy()
            for token in tokens:
                if token in ("m[", "m]"):
                    continue
                a, t, angle, mask = map(int, token.split(":"))
                for move in table[f"{a}:{t}:{angle % orders[a, t]}:{mask}"]:
                    src, dst = model.move(move)
                    labels[dst] = labels[src]
            return labels, native_colors(labels, mapping, inverse)

        def mpult(name: str, raw: bytes) -> None:
            plan = import_native(model, b64(raw), profile)
            write(name, gzip.compress(raw, compresslevel=9, mtime=0))
            expected["mpult"][name] = dict(plan_json(plan), sha256=sha(raw))

        # The three logs of tests/test_mpult_log.py: a signed inverse angle and a combined mask; macro markers with a
        # saved pointer before a redo tail; and an export of a 0.4 session's words and star.
        tokens = ["0:0:1:1", "1:1:-1:4", "2:0:1:5"]
        _, colors = color_for(tokens)
        mpult("mpult-negative.log.gz", serialize(colors, tokens, timer=12345))
        grouped = ["m[", tokens[0], tokens[1], "m]", tokens[2]]
        _, active = color_for(grouped[:2])
        mpult("mpult-partial-redo.log.gz", serialize(active, grouped, shuffle=2, ptr=2, timer=12345))
        s = Session(model, work / "mpult")
        try:
            s.commit(s.preview([dict(kind="word", moves=[1, 2, -4]), dict(kind="star", orbit=33, node=0, sign=1)])["token"])
            raw = export_native(model, s.export(), s.st.labels, profile, 12000)
            mpult("mpult-export.log.gz", raw)
            expected["mpult"]["mpult-export.log.gz"]["session"] = s.st.hash
        finally:
            s.close()

    after = {name: sha((ROOT / name).read_bytes()) for name in SOURCES}
    assert before == after, "a 0.4 source changed while the fixtures were made"
    (HERE / "expected.json").write_text(json.dumps(expected, indent=1, sort_keys=True) + "\n", encoding="utf-8",
                                        newline="\n")
    print(json.dumps({name: len((HERE / name).read_bytes()) for name in sorted(p.name for p in HERE.iterdir())}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
