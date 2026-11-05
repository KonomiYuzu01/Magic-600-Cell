"""Integrator follow-up to LX-9: which float operation differs between Windows and Linux?

Run once on each platform, then compare:

    python -B trig_control.py <output.tsv>
    python -B trig_control.py --compare <windows.tsv> <linux.tsv>

It reads the frozen requests and writes only the output path it is given. It pins nothing and
changes no LX-9 record. Standard library only.
"""
import sys

sys.dont_write_bytecode = True

import math
import pathlib
import platform
from decimal import Decimal, getcontext

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
from common import bits, read_requests, unbits  # noqa: E402
from pure_input_map import input_quaternion  # noqa: E402

HEADER = "id\tunit_axis\thalf\tsin\tcos\ttarget\n"
ROTATION_DIFFERS = ("b1205", "b1207", "b1209", "b1211", "b1213", "b1215")


def requests():
    return read_requests(HERE.parent / "step1.tsv") + read_requests(HERE.parent / "bisectors.tsv")


def produce(out):
    lines = [HEADER]
    rows = requests()
    for name, axis, degrees, _, _ in rows:
        unit, target = input_quaternion(axis, degrees)
        half = math.radians(math.remainder(degrees, 360.0)) / 2
        lines.append("\t".join([name, ",".join(map(bits, unit)), bits(half), bits(math.sin(half)),
                                bits(math.cos(half)), ",".join(map(bits, target))]) + "\n")
    pathlib.Path(out).write_bytes("".join(lines).encode("ascii"))
    print(platform.system(), platform.python_version(), len(rows), "rows")


def load(path):
    lines = pathlib.Path(path).read_text(encoding="ascii").splitlines()
    assert lines[0] + "\n" == HEADER
    rows = {}
    for line in lines[1:]:
        name, *fields = line.split("\t")
        assert len(fields) == 5 and name not in rows
        rows[name] = fields
    return rows


def sin_cos(x):
    """Sine and cosine of the Decimal x by Taylor series, far past double precision."""
    s = c = Decimal(0)
    term, n = Decimal(1), 0
    limit = Decimal(10) ** -(getcontext().prec - 5)
    while abs(term) > limit or n < 2:
        if n % 2 == 0:
            c += term if n % 4 == 0 else -term
        else:
            s += term if n % 4 == 1 else -term
        n += 1
        term = term * x / n
    return s, c


def atan_inverse(k):
    total, term, n, sign = Decimal(0), Decimal(1) / k, 1, 1
    limit = Decimal(10) ** -(getcontext().prec - 5)
    while term > limit:
        total += sign * term / n
        term /= k * k
        n += 2
        sign = -sign
    return total


def tie_margin(c, s):
    """Positive picks the 90 degree rotation (1, e), negative the (2, e) rotation."""
    return 5 * (c + s) ** 2 - 2 * (2 * c + s) ** 2


def compare(windows_path, linux_path):
    getcontext().prec = 110
    windows, linux = load(windows_path), load(linux_path)
    assert windows.keys() == linux.keys()
    print("rows", len(windows))
    fields = {"unit_axis": 0, "half": 1, "sin": 2, "cos": 3, "target": 4}
    differ = {k: {n for n in windows if windows[n][i] != linux[n][i]} for k, i in fields.items()}
    for key, value in differ.items():
        print(f"{key} differs on {len(value)} requests")
    trig = differ["sin"] | differ["cos"]
    print("target differs with equal sin and cos:", len(differ["target"] - trig))
    print("sin or cos differs with equal target:", len(trig - differ["target"]))

    rounded = {"windows": {"sin": 0, "cos": 0}, "linux": {"sin": 0, "cos": 0}}
    for name, row in windows.items():
        s, c = sin_cos(Decimal(unbits(row[1])))
        for side, table in (("windows", windows), ("linux", linux)):
            for key, value, column in (("sin", s, 2), ("cos", c, 3)):
                if bits(float(value)) != table[name][column]:
                    rounded[side][key] += 1
    print("results not correctly rounded", rounded)

    pi = 16 * atan_inverse(5) - 4 * atan_inverse(239)
    degrees = {r[0]: r[2] for r in requests()}
    for name in ROTATION_DIFFERS:
        half = Decimal(unbits(windows[name][1]))
        exact_s, exact_c = sin_cos(half)
        real_s, real_c = sin_cos(Decimal(degrees[name]) * pi / 360)
        win = [Decimal(unbits(x)) for x in windows[name][4].split(",")]
        lin = [Decimal(unbits(x)) for x in linux[name][4].split(",")]
        print(name, "degrees", repr(degrees[name]), "half", repr(float(half)))
        print("  sin windows", windows[name][2], "linux", linux[name][2], "correctly rounded", bits(float(exact_s)))
        print("  cos windows", windows[name][3], "linux", linux[name][3], "correctly rounded", bits(float(exact_c)))
        print("  margin at the Windows target %.3e" % tie_margin(win[0], max(map(abs, win[1:]))))
        print("  margin at the Linux target   %.3e" % tie_margin(lin[0], max(map(abs, lin[1:]))))
        print("  margin at exact sin and cos of the float half angle %.3e" % tie_margin(exact_c, abs(exact_s)))
        print("  margin at exact sin and cos of the real half angle  %.3e" % tie_margin(real_c, abs(real_s)))


def main():
    if len(sys.argv) == 4 and sys.argv[1] == "--compare":
        compare(sys.argv[2], sys.argv[3])
    elif len(sys.argv) == 2:
        produce(sys.argv[1])
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main()
