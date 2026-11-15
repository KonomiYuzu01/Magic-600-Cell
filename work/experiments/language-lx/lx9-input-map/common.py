"""LX-9 fixture/result encoding. Experiment code, no product imports."""
import hashlib
import json
import math
from pathlib import Path
import struct

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
REQUEST_HEADER = "id\tax\tay\taz\tdegrees\tN\tD\n"
RESULT_HEADER = "id\tq0\tq1\tq2\tq3\tfamily\tden\tp0\tp1\tp2\n"


def digest(data):
    return hashlib.sha256(data).hexdigest()


def bits(value):
    return struct.pack(">d", value).hex()


def unbits(value):
    return struct.unpack(">d", bytes.fromhex(value))[0]


def json_bytes(value):
    return (json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n").encode()


def request_line(req):
    name, axis, angle, n, d = req
    return "\t".join([name, *(bits(x) for x in axis), bits(angle), str(n), str(d)]) + "\n"


def request_bytes(requests):
    return (REQUEST_HEADER + "".join(map(request_line, requests))).encode()


def read_requests(path):
    lines = path.read_text(encoding="ascii").splitlines()
    assert lines[0] + "\n" == REQUEST_HEADER
    rows = []
    for line in lines[1:]:
        name, *fields = line.split("\t")
        assert len(fields) == 6
        rows.append((name, tuple(map(unbits, fields[:3])), unbits(fields[3]),
                     int(fields[4]), int(fields[5])))
    assert len({r[0] for r in rows}) == len(rows)
    return rows


def result_line(name, target, choice):
    den, p, family = choice
    return "\t".join([name, *(bits(x) for x in target), family, str(den), *map(str, p)]) + "\n"


def read_results(data):
    lines = data.decode("ascii").splitlines()
    assert lines[0] + "\n" == RESULT_HEADER
    rows = {}
    for line in lines[1:]:
        name, *f = line.split("\t")
        assert len(f) == 9 and name not in rows
        assert f[4] in ("cayley", "half_turn")
        rows[name] = (tuple(f[:4]), (int(f[5]), tuple(map(int, f[6:])), f[4]))
    return rows


def rotation_key(choice):
    """Primitive projective integer quaternion: an exact SO(3) rotation key."""
    den, p, family = choice
    q = (0 if family == "half_turn" else den, *p)
    g = math.gcd(*q)
    assert g
    sign = next(x for x in q if x) > 0
    return tuple(x // g * (1 if sign else -1) for x in q)


def differences(a, b):
    assert a.keys() == b.keys(), "result request IDs differ"
    target = [k for k in a if a[k][0] != b[k][0]]
    choice = [k for k in a if rotation_key(a[k][1]) != rotation_key(b[k][1])]
    raw = [k for k in a if a[k][1] != b[k][1]]
    return {"target": target, "rotation": choice, "parameters": raw}


def verify_manifest():
    manifest = json.loads((HERE / "MANIFEST.json").read_bytes())
    for name, entry in manifest["files"].items():
        assert digest((HERE / name).read_bytes()) == entry["sha256"], name
    return manifest


def verify_candidate():
    candidate = json.loads((HERE / "CANDIDATE.json").read_bytes())
    for name, sha in candidate["files"].items():
        assert digest((ROOT / name).read_bytes()) == sha, name
    return candidate
