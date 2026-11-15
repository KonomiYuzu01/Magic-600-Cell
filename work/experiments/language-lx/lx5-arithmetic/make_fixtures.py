"""Freeze or verify LX-5 operands, integer reference answers and source digests."""
import argparse
import gzip
import json
import math
from pathlib import Path
import random
import struct
import sys

sys.dont_write_bytecode = True
from common import BUILD, REPO, ROOT, cleanup, json_bytes, sha, write
from reference import evaluate, q5, qadd, qmul, self_test, word_summary

FIXTURES = ROOT / "fixtures"
NC = "research/jumbling/sim/acceptance.json"
RUN = "research/jumbling/restoration/18-tail-completion/results/lattice-d1/run/run.json"
SEED = 0x6005
OPS = ("add", "sub", "mul", "div", "rem", "gcd", "cmp", "shl", "shr", "bits", "dec", "hex",
       "rat_add", "rat_mul", "rat_cmp", "q_add", "q_mul", "q_conj", "q_norm", "q_sign", "sum", "word")


def matmul(a, b):
    return [[sumq(qmul(a[i][k], b[k][j]) for k in range(4)) for j in range(4)] for i in range(4)]


def sumq(items):
    answer = (0, 0, 1)
    for item in items:
        answer = qadd(answer, item)
    return answer


def power(a, n):
    out = [[(int(i == j), 0, 1) for j in range(4)] for i in range(4)]
    while n:
        if n & 1:
            out = matmul(a, out)
        n >>= 1
        if n:
            a = matmul(a, a)
    return out


def pose_inputs():
    nc = json.loads((REPO / NC).read_bytes())["negative_control"]
    h, g = [record["matrix"] for record in nc["journal"]["records"][:2]]
    assert g == nc["g"]["matrix"]
    step = matmul(g, h)  # Column-vector chronology: H then g, so g H.
    assert sumq(step[i][i] for i in range(4)) == (4, 0, 3)
    assert power(g, 2) == power(g, 0)
    for n, height in enumerate(nc["cap_0_pose_entry_heights"], 1):
        actual = max(abs(v) for row in power(step, n) for entry in row for v in entry)
        assert actual == height, (n, actual, height)
    poses = []
    for n in (44, 164, 1906, 14197):
        matrix = power(step, n)
        entry = max((e for row in matrix for e in row), key=lambda e: max(abs(v).bit_length() for v in e))
        poses.append((n, entry, max(abs(v).bit_length() for v in entry)))
    return poses, {"source": NC, "sha256": sha((REPO / NC).read_bytes()),
                   "H": h, "g": g, "recipe": "entry of largest component bit length in (g H)^n; row-major first tie",
                   "recorded_rounds_checked": 12,
                   "powers": [{"rounds": n, "component_bits": bits, "entry": list(map(str, e))} for n, e, bits in poses]}


def prepare_words():
    directory = BUILD / "words"
    directory.mkdir(parents=True, exist_ok=True)
    run_path = REPO / RUN
    records = json.loads(run_path.read_bytes())["steps"]
    assert len(records) == 53 and sum(r["letters"] for r in records) == 13231266
    sources = []
    with (directory / "lattice.ia16").open("wb") as whole:
        for i, record in enumerate(records):
            assert record["index"] == i
            path = run_path.parent / record["word"]
            packed = path.read_bytes()
            raw = gzip.decompress(packed)
            assert len(raw) == record["letters"] * 2 and sha(raw) == record["word_sha256"]
            sources.append({"path": path.relative_to(REPO).as_posix(), "letters": record["letters"],
                            "gzip_sha256": sha(packed), "ia16_sha256": sha(raw)})
            whole.write(raw)  # One output stream; neither counters nor hashes reset at a word boundary.
            if i == 14:
                assert record["letters"] == 2352984
                write(directory / "longest.ia16", raw)
                write(directory / "prefix672417.ia16", raw[:672417 * 2])
                write(directory / "prefix871582.ia16", raw[:871582 * 2])
    # xorshift32, modulo 36,000; byte order is explicitly little endian.
    state = SEED
    with (directory / "bound.ia16").open("wb") as stream:
        left = 10_000_000
        while left:
            count = min(left, 32768)
            block = bytearray(count * 2)
            for i in range(count):
                state ^= (state << 13) & 0xFFFFFFFF
                state ^= state >> 17
                state ^= (state << 5) & 0xFFFFFFFF
                struct.pack_into("<H", block, 2 * i, state % 36000)
            stream.write(block)
            left -= count
    write(directory / "empty.ia16", b"")
    recipes = [
        ("prefix672417.ia16", "672417-letters", "first 672417 letters of word 00014; source-derived size surrogate, not the earlier E2 witness"),
        ("prefix871582.ia16", "871582-letters", "first 871582 letters of word 00014; source-derived size surrogate, not the T6 witness"),
        ("longest.ia16", "2352984-letters", "recorded word 00014 in full"),
        ("bound.ia16", "10000000-letters", "xorshift32 seed 24581; shifts 13,17,5; mask 32 bits; letter = state % 36000"),
        ("lattice.ia16", "13231266-letters", "all 53 recorded words concatenated in step-index order, one stream"),
        ("empty.ia16", "empty-stream", "empty stream control"),
    ]
    outputs = {}
    meta = []
    for name, size, recipe in recipes:
        result = word_summary(directory / name)
        outputs[name] = result
        length, total, digest, _ = result.split("|", 3)
        meta.append(dict(file=name, size=size, recipe=recipe, letters=int(length), letter_sum=int(total),
                         ia16_sha256=digest, canonical_summary_sha256=sha(result.encode("ascii"))))
    return outputs, {"run": RUN, "run_sha256": sha(run_path.read_bytes()), "sources": sources, "streams": meta}


class Corpus:
    def __init__(self):
        self.values, self.reverse, self.cases, self.measured = {}, {}, [], set()

    def value(self, n):
        if n not in self.reverse:
            key = "v" + str(len(self.values))
            self.values[key] = n
            self.reverse[n] = key
        return self.reverse[n]

    def add(self, size, op, *args, measure=True):
        key = f"c{len(self.cases):04d}"
        measured = measure and (size, op) not in self.measured
        if measured:
            self.measured.add((size, op))
        operands = list(args) if op == "word" else [self.value(a) for a in args]
        self.cases.append(dict(id=key, size=size, op=op, measured=measured, args=operands))

    def integers(self, size, a, b, measure=True):
        assert b
        for op in ("add", "sub", "mul", "div", "rem", "gcd", "cmp"):
            self.add(size, op, a, b, measure=measure)
        for op in ("shl", "shr"):
            for shift in (65, 0, 1, 31, 32, abs(a).bit_length() + 1):
                self.add(size, op, a, shift, measure=measure)
        for op in ("bits", "dec", "hex"):
            self.add(size, op, a, measure=measure)

    def algebra(self, size, a, b, d):
        for op in ("rat_add", "rat_mul", "rat_cmp"):
            self.add(size, op, 6 * a, -6 * d, -3 * b, 3 * d)
        left, right = (a, b, d), (b + 1, -a, -d)
        for op in ("q_add", "q_mul"):
            self.add(size, op, *left, *right)
        for op in ("q_conj", "q_norm", "q_sign"):
            self.add(size, op, *left)
        for x in ((a, -b, d), (-a, b, d), (0, b, -d), (a, 0, d), (0, 0, -d)):
            self.add(size, "q_sign", *x)


def generate():
    self_test()
    corpus = Corpus()
    rng = random.Random(SEED)
    for size, a in (("2^64", 2 ** 64), ("2^256", 2 ** 256),
                    ("22500-bit", (1 << 22499) | rng.getrandbits(22499)),
                    ("6775-digit", 10 ** 6774 + 123456789)):
        b = a // 3 + 17
        corpus.integers(size, a, b)
        corpus.algebra(size, a, b, a - 1)
        for x, y in ((-a, b), (a, -b), (-a, -b), (0, b), (a, 1), (b, a), (a, a)):
            for op in ("div", "rem", "gcd", "cmp"):
                corpus.add(size, op, x, y)
        for op in ("bits", "dec", "hex", "shr"):
            args = (-a - 1, 32) if op == "shr" else (-a - 1,)
            corpus.add(size, op, *args)
        corpus.add(size, "add", a - 1, 1)
        corpus.add(size, "sub", a + 1, 1)
        corpus.add(size, "mul", a, 0)
    poses, pose_meta = pose_inputs()
    for n, (a, b, d), bits in poses:
        size = "pose-nc-" + str(bits) + "-bit"
        corpus.integers(size, a, d)
        corpus.algebra(size, a, b, d)
    # Near cancellation with exactly known signs, including the 22,500-bit scale.
    for n in (31, 123, 1455, 10800, 10801):
        a, b, _ = qpow((2, 1, 1), n)
        size = "pell-" + str(a.bit_length()) + "-bit"
        assert a * a - 5 * b * b == (-1) ** n
        for x in ((a, -b, 1), (-a, b, 7), (a, b, -9)):
            corpus.add(size, "q_sign", *x)
        corpus.add(size, "q_norm", a, b, 1)
    # A separately identified seeded differential self-test corpus for Rust limbs.
    for bits in (1, 31, 32, 33, 63, 64, 65, 96, 256, 1024, 3036, 22500):
        for i in range(4 if bits > 1024 else 12):
            a = rng.getrandbits(bits) * (-1 if i & 1 else 1)
            width = (1, bits, max(1, bits // 2), max(1, bits - 31))[i % 4]
            b = (rng.getrandbits(width) or 1) * (-1 if i & 2 else 1)
            corpus.integers(f"random-{bits}-bit", a, b, measure=False)
    # Division normalization / quotient-estimate boundaries at radix 2^32.
    radix = 1 << 32
    for a, b in ((radix ** 3 - 1, radix ** 2 + 1),
                 (radix ** 4 + radix ** 2 - 1, radix ** 2 - 1),
                 ((radix - 1) * (radix ** 2 + radix - 1), radix ** 2 + radix - 1),
                 (radix ** 3 + radix ** 2 + radix - 1, radix ** 2 + 1)):
        corpus.integers("limb-boundaries", a, b, measure=False)
    word_answers, words = prepare_words()
    for stream in words["streams"]:
        corpus.add(stream["size"], "word", stream["file"])
    corpus.integers("length-166788405", 166788405, 10000000)
    corpus.add("recorded-length-sum", "sum", *(r["letters"] for r in words["sources"]))
    corpus.add("recorded-length-sum", "mul", 13231266, 2)
    assert {case["op"] for case in corpus.cases} == set(OPS)
    lines = [f"value\t{k}\t{v}\n" for k, v in corpus.values.items()]
    answers = []
    for c in corpus.cases:
        lines.append("\t".join(["case", c["id"], c["size"], c["op"], "m" if c["measured"] else "-", *c["args"]]) + "\n")
        result = word_answers[c["args"][0]] if c["op"] == "word" else evaluate(c, corpus.values, BUILD / "words")
        answers.append(c["id"] + "\t" + result + "\n")
    reference = "".join(answers).encode("ascii")
    files = {"operands.tsv": "".join(lines).encode("ascii"), "reference.tsv.gz": gzip.compress(reference, compresslevel=9, mtime=0),
             "pose-recipe.json": json_bytes(pose_meta), "word-recipes.json": json_bytes(words)}
    manifest = {"format": "lx5-fixtures/1", "seed": SEED, "operations": list(OPS),
                "cases": len(corpus.cases), "values": len(corpus.values),
                "reference_decoded": {"sha256": sha(reference), "bytes": len(reference)},
                "files": {name: {"sha256": sha(data), "bytes": len(data)} for name, data in files.items()}}
    files["MANIFEST.json"] = json_bytes(manifest)
    return files


def qpow(value, n):
    result = (1, 0, 1)
    while n:
        if n & 1:
            result = qmul(result, value)
        n >>= 1
        if n:
            value = qmul(value, value)
    return result


def freeze(verify=True):
    files = generate()
    if verify:
        assert {p.name for p in FIXTURES.iterdir() if p.is_file()} == set(files), "fixture file set changed"
        for name, content in files.items():
            assert (FIXTURES / name).read_bytes() == content, f"frozen fixture differs: {name}"
    else:
        for name, content in files.items():
            write(FIXTURES / name, content)
    return json.loads(files["MANIFEST.json"])


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true", help="explicitly establish a new fixture identity")
    args = parser.parse_args()
    try:
        manifest = freeze(verify=not args.write)
        print("Fixtures", "frozen" if args.write else "verified", "-", manifest["cases"], "cases")
    finally:
        cleanup()
