"""Deterministic SVG diagrams of committed tutorial evidence; no simulation or search.

Run with --out DIRECTORY to redraw, --only NAME ... to select figures, or --check
to regenerate in a temporary directory and verify the saved manifest and files.
All numerical observations are read from records, never from a copied data table.
"""

import argparse
import ast
from functools import lru_cache
import gzip
import hashlib
import json
import math
from pathlib import Path, PurePosixPath
import re
import sys
import tempfile
import textwrap
import types
import xml.etree.ElementTree as ET


# Shared provisional palette. Colour choices are not an owner design decision.
PALETTE = {
    "white": "#ffffff", "ink": "#172b3a", "muted": "#526574",
    "line": "#cbd5dd", "panel": "#f3f6f8", "grey": "#d7dde2",
    "blue": "#2469a0", "teal": "#087f83", "orange": "#b76616",
    "purple": "#8054a2", "red": "#b43c4c", "green": "#397546",
    "brown": "#7f553d", "olive": "#77721d", "cyan": "#14798a",
}
ACCENTS = tuple(PALETTE[k] for k in ("blue", "teal", "orange", "purple", "red", "green", "brown", "olive", "cyan"))
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
T = "research/jumbling/tutorial"
F = T + "/figures"
REST = "research/jumbling/restoration"
LOW = REST + "/12-low-height"
ROUND = LOW + "/runs/round22"
CLOUD = ROUND + "/cloud/height4-descent-bbef1c8e"
RUNNER = REST + "/13-tutorial-runner"
DEV = RUNNER + "/runs/dev"
API_PATH = RUNNER + "/scripts/runner_api.py"
TREES_PATH = F + "/trees.json"
SVG_NS = "http://www.w3.org/2000/svg"
NAME_RE = re.compile(r"[a-z0-9][a-z0-9-]{0,63}\Z")
FIGURES = {}
BACKTICK = chr(96)
# Unchanged SVGs retain their original production metadata byte for byte.
# The manifest separately inventories the current inputs used to verify a redraw.
ORIGINAL_RENDERER = "4999331eb3c77ec66b6af2cf989eaeaca63d2046e00340409826223b231e8008"
UPDATED_FIGURES = {"bulk-n-7101", "bulk-conj-7101-s30", "03-counts-7302",
                   "03-r22d75-prediction-cap64", "ch04-six-stops-caps", "ch06-twisted-cycle"}
# Preserve the production provenance of the already committed chapter SVGs.
# The manifest still hashes every *current* verification input. Adding appendix
# observations to a shared JSON file must not rewrite an unrelated chapter SVG.
CHAPTER_DATA_RENDERER = "908108557d04296ca9866adfa5211c9473111916958a5673c194fcc621884a26"
CHAPTER_DATA_SHA256 = "35ae0c746ed3182ca3a60625117fa5ac7067d50ce16228c647cbcba45b04258b"
CHAPTER_TREES_SHA256 = "748e36ba7ad9b6fd703d5d6ec549ae627a891e0683589812e25e64da1b8fca51"
ET.register_namespace("", SVG_NS)


def sha256(raw):
    return hashlib.sha256(raw).hexdigest()


def production_inputs(name, inputs):
    result = dict(inputs)
    if not name.startswith("appa-"):
        result[F + "/diagrams.py"] = CHAPTER_DATA_RENDERER if name in UPDATED_FIGURES else ORIGINAL_RENDERER
        if F + "/figdata.json" in result:
            result[F + "/figdata.json"] = CHAPTER_DATA_SHA256
        if TREES_PATH in result:
            result[TREES_PATH] = CHAPTER_TREES_SHA256
    return result


def repository_path(value):
    """Reject traversal and private/absolute paths before opening any input."""
    if not isinstance(value, str) or not value or "\\" in value or ":" in value:
        raise ValueError("expected a repository-relative path")
    path = PurePosixPath(value)
    if path.is_absolute() or ".." in path.parts or str(path) != value:
        raise ValueError("expected a normalized repository-relative path")
    local = ROOT.joinpath(*path.parts)
    if not local.resolve().is_relative_to(ROOT):
        raise ValueError("input leaves the repository")
    return local


class Evidence:
    """Read-only input access with a per-figure content-digest inventory."""

    def __init__(self, name, chapter=None):
        self.name = name
        self.inputs = {}
        self.notes = []
        self.raw(F + "/diagrams.py")
        self.chapter_path = None
        self.chapter = ""
        self.number = ""
        self.alt = ""
        if chapter:
            self.chapter_path = T + "/" + chapter
            self.chapter = self.text(self.chapter_path)
            pattern = r"!\[(Figure ([^:]+)): ([^\]]*)\]\(figures/svg/" + re.escape(name) + r"\.svg\)"
            match = re.search(pattern, self.chapter)
            if not match:
                raise ValueError("missing chapter specification: " + name)
            self.number, self.alt = match[1], match[3]

    def raw(self, path):
        raw = repository_path(path).read_bytes()
        self.inputs[path] = sha256(raw)
        return raw

    def text(self, path):
        return self.raw(path).decode("utf-8")

    def json(self, path):
        raw = self.raw(path)
        return json.loads(gzip.decompress(raw) if path.endswith(".gz") else raw)

    def record(self, chapter):
        return self.json(T + f"/records/ch{chapter:02}-examples.json")

    def figdata(self):
        document = self.json(F + "/figdata.json")
        if document.get("format") != "magic600-tutorial-figdata/1":
            raise ValueError("unsupported computed figure data")
        return document["figures"][self.name]

    def note(self, value):
        self.notes.append(value)


def figure(name, chapter, title):
    def register(function):
        if name in FIGURES:
            raise ValueError("duplicate figure name")
        FIGURES[name] = (function, chapter, title)
        return function
    return register


def fmt(value):
    if isinstance(value, float):
        return f"{value:.3f}".rstrip("0").rstrip(".") or "0"
    return str(value)


def count(value):
    return f"{value:,}" if isinstance(value, int) else str(value)


def clean(text):
    text = re.sub(r"\[([^\]]+)\]\([^)]+\)", r"\1", text)
    return text.replace(BACKTICK, "").replace("**", "").replace(r"\|", "|").strip()


def tables(text):
    result, rows = [], []
    for line in text.splitlines() + [""]:
        if line.startswith("|"):
            cells = [clean(x) for x in re.split(r"(?<!\\)\|", line)[1:-1]]
            if cells and not all(re.fullmatch(r"[: -]+", c) for c in cells):
                rows.append(cells)
        elif rows:
            result.append(rows)
            rows = []
    return result


def table_after(text, marker):
    return tables(text.split(marker, 1)[1])[0]


def integer(text):
    return int(text.replace(",", "").strip())


def tagged_lines(text):
    result = []
    for line in text.splitlines():
        match = re.match(r"([A-Z][A-Z0-9_]*) (\{.*\})$", line)
        if match:
            result.append((match[1], json.loads(match[2])))
    return result


class Svg:
    def __init__(self, evidence, title):
        self.evidence = evidence
        self.root = ET.Element("{" + SVG_NS + "}svg", {
            "version": "1.1", "width": "1200", "role": "img",
            "font-family": "system-ui, -apple-system, Segoe UI, sans-serif",
            "font-size": "16", "fill": PALETTE["ink"],
            "aria-labelledby": "figure-title figure-description",
        })
        self.el("title", text=evidence.number + ": " + title, id="figure-title")
        self.el("desc", text=evidence.alt or title, id="figure-description")
        defs = self.el("defs")
        for key in ("muted", "purple", "teal", "red", "blue"):
            marker = ET.SubElement(defs, "{" + SVG_NS + "}marker", {
                "id": "arrow-" + key, "viewBox": "0 0 10 10", "refX": "9", "refY": "5",
                "markerWidth": "7", "markerHeight": "7", "orient": "auto",
                "markerUnits": "strokeWidth",
            })
            ET.SubElement(marker, "{" + SVG_NS + "}path", {"d": "M 0 0 L 10 5 L 0 10 z", "fill": PALETTE[key]})
        self.background = self.el("rect", x=0, y=0, width=1200, height=1, fill=PALETTE["white"])
        self.text(40, 40, evidence.number + ": " + title, size=23, weight="600")
        self.line(40, 60, 1160, 60, colour=PALETTE["line"])

    def el(self, tag, text=None, **attrs):
        element = ET.SubElement(self.root, "{" + SVG_NS + "}" + tag, {
            k.replace("_", "-"): fmt(v) for k, v in sorted(attrs.items()) if v is not None
        })
        if text is not None:
            element.text = str(text)
        return element

    def text(self, x, y, text, size=16, colour=None, anchor="start", weight=None, **attrs):
        return self.el("text", text=str(text), x=x, y=y, font_size=size,
                       fill=colour, text_anchor=anchor, font_weight=weight, **attrs)

    @staticmethod
    def wrapped(text, width, size=15):
        chars = max(8, int(width / (size * .58)))
        return [line for part in str(text).splitlines() for line in
                (textwrap.wrap(part, width=chars, break_long_words=True, break_on_hyphens=False) or [""])]

    def paragraph(self, x, y, text, width=1120, size=15, colour=None):
        for line in self.wrapped(text, width, size):
            self.text(x, y, line, size=size, colour=colour)
            y += size * 1.4
        return y

    def rect(self, x, y, w, h, fill=None, stroke=None, **attrs):
        return self.el("rect", x=x, y=y, width=w, height=h,
                       fill=fill or PALETTE["panel"], stroke=stroke, **attrs)

    def line(self, x1, y1, x2, y2, colour=None, width=1.5, arrow=False, **attrs):
        return self.el("line", x1=x1, y1=y1, x2=x2, y2=y2,
                       stroke=colour or PALETTE["muted"], stroke_width=width,
                       marker_end="url(#arrow-muted)" if arrow else None, **attrs)

    def path(self, d, colour="muted", arrow=False, dashed=False):
        return self.el("path", d=d, fill="none", stroke=PALETTE[colour], stroke_width=1.5,
                       marker_end="url(#arrow-" + colour + ")" if arrow else None,
                       stroke_dasharray="5 4" if dashed else None)

    def box(self, x, y, w, h, text, fill=None, size=15):
        self.rect(x, y, w, h, fill=fill, stroke=PALETTE["line"], rx=6)
        lines = self.wrapped(text, w - 24, size)
        first = y + h / 2 - (len(lines) - 1) * size * .7 + size * .35
        for i, line in enumerate(lines):
            self.text(x + w / 2, first + i * size * 1.4, line, size=size, anchor="middle")

    def legend(self, items, y):
        x = 40
        for label, colour in items:
            width = 34 + len(label) * 8.8
            if x + width > 1160:
                x, y = 40, y + 28
            self.rect(x, y - 12, 14, 14, fill=colour)
            self.text(x + 22, y, label, size=14)
            x += width
        return y + 30

    def finish(self, y):
        for note in self.evidence.notes:
            lines = self.wrapped("Data note: " + note, 1080, 15)
            height = len(lines) * 21 + 20
            self.rect(40, y, 1120, height, stroke=PALETTE["orange"], rx=5)
            self.paragraph(56, y + 24, "Data note: " + note, width=1080, size=15)
            y += height + 12
        self.text(40, y + 22, "Committed research evidence · provisional colours · no Windows/DirectX or performance claim",
                  size=13, colour=PALETTE["muted"])
        height = math.ceil(y + 48)
        self.root.set("height", str(height))
        self.root.set("viewBox", f"0 0 1200 {height}")
        self.background.set("height", str(height))
        self.el("metadata", text=json.dumps({
            "figure": self.evidence.name, "inputs": production_inputs(self.evidence.name, self.evidence.inputs),
            "notes": self.evidence.notes,
            "tool": "Python " + ".".join(map(str, sys.version_info[:3])) + "; standard library SVG writer",
            "command": "python3 -I -B " + F + "/diagrams.py --only " + self.evidence.name,
        }, sort_keys=True, ensure_ascii=False, separators=(",", ":")))
        return ET.tostring(self.root, encoding="utf-8", xml_declaration=True) + b"\n"


def draw_table(svg, headers, rows, y, widths=None):
    widths = widths or [1120 / len(headers)] * len(headers)
    for row_index, row in enumerate([headers] + list(rows)):
        lines = [svg.wrapped(value, w - 20, 14) for value, w in zip(row, widths)]
        h = max(map(len, lines)) * 20 + 18
        x = 40
        for column, w in enumerate(widths):
            svg.rect(x, y, w, h, fill=PALETTE["panel"] if row_index % 2 == 0 else PALETTE["white"],
                     stroke=PALETTE["line"])
            for j, line in enumerate(lines[column]):
                svg.text(x + 10, y + 24 + j * 20, line, size=14,
                         weight="600" if row_index == 0 else None)
            x += w
        y += h
    return y + 24


def flow(svg, rows, y=100):
    """Ordered checks; the right-hand branch states its result or next action."""
    for i, (label, branch) in enumerate(rows):
        h = max(len(svg.wrapped(label, 658)), len(svg.wrapped(branch, 302))) * 21 + 26
        if i:
            svg.line(390, y - 24, 390, y - 3, arrow=True)
        svg.box(40, y, 700, h, label)
        svg.line(740, y + h / 2, 814, y + h / 2, arrow=True)
        svg.box(820, y, 340, h, branch)
        y += h + 28
    return y


def phase_strip(svg, names, y):
    width = (1120 - 32 * (len(names) - 1)) / len(names)
    for i, name in enumerate(names):
        x = 40 + i * (width + 32)
        svg.box(x, y, width, 54, name)
        if i:
            svg.line(x - 30, y + 27, x - 4, y + 27, arrow=True)
    return y + 86


def plot(svg, series, labels, y, title, log=False, bars=False, point_colours=None, height=240, label_stride=None):
    """Exact observations with linear or base-10 logarithmic axes; None is a gap."""
    svg.text(40, y + 18, title, size=18, weight="600")
    top, bottom, left, right = y + 46, y + 46 + height, 110, 1150
    observed = [v for s in series for v in s["values"] if v is not None]
    maximum = max(observed, default=1)
    if log:
        exponent = max(1, math.ceil(math.log10(max(maximum, 1))))
        ticks = [10 ** e for e in range(exponent + 1)]
        def y_of(v):
            return bottom - math.log10(max(v, 1)) / exponent * height
    else:
        raw = max(maximum, 1) / 4
        decade = 10 ** math.floor(math.log10(raw))
        step = next(n * decade for n in (1, 2, 5, 10) if n * decade >= raw)
        ceiling = math.ceil(max(maximum, 1) / step) * step
        ticks = [i * step for i in range(round(ceiling / step) + 1)]
        def y_of(v):
            return bottom - v / ceiling * height
    for tick in ticks:
        yy = y_of(tick)
        svg.line(left, yy, right, yy, colour=PALETTE["line"], width=1)
        svg.text(left - 12, yy + 5, count(round(tick)), size=13, anchor="end")
    n = len(labels)
    spacing = (right - left) / max(n - (0 if bars else 1), 1)
    def x_of(i):
        return left + (i + .5 if bars else i) * spacing
    for si, s in enumerate(series):
        colour = s.get("colour", ACCENTS[si % len(ACCENTS)])
        previous = None
        for i, value in enumerate(s["values"]):
            if value is None:
                previous = None
                continue
            x, yy = x_of(i), y_of(value)
            if bars:
                width = spacing * .76 / len(series)
                bx = x - spacing * .38 + si * width
                mark = svg.rect(bx, yy, width - 1, bottom - yy, fill=colour)
            else:
                if previous:
                    svg.line(*previous, x, yy, colour=colour, width=2)
                mark = svg.el("circle", cx=x, cy=yy, r=3.8,
                              fill=point_colours[i] if point_colours else colour)
            ET.SubElement(mark, "{" + SVG_NS + "}title").text = f"{labels[i]} · {s['label']}: {count(value)}"
            previous = (x, yy)
        if not bars:
            indexes = [i for i, value in enumerate(s["values"]) if value is not None]
            for i in dict.fromkeys([indexes[0], indexes[-1]] if indexes else []):
                svg.text(x_of(i), y_of(s["values"][i]) - 11, count(s["values"][i]),
                         size=13, anchor="middle", colour=colour)
    stride = label_stride or max(1, math.ceil(n / 38))
    for i, label in enumerate(labels):
        if i % stride == 0 or i == n - 1:
            svg.text(x_of(i), bottom + 24, label, size=13, anchor="middle")
    return bottom + 60


@lru_cache(maxsize=1)
def tree_api():
    name = "tutorial_diagrams_tree_api"
    module = types.ModuleType(name)
    sys.modules[name] = module
    # Reuse the runner's structural checker without a source loader writing a
    # bytecode cache next to the read-only runner sources (also without -B).
    exec(compile(repository_path(API_PATH).read_bytes(), API_PATH, "exec"), module.__dict__)
    return module


def folded_tree(svg, root, y, label=None, show_spans=False):
    """One row per recorded part, retaining zero spans and all reference arrows."""
    if label:
        svg.text(40, y + 18, label, size=18, weight="600")
        y += 44
    rows, positions = [], {}
    stack = [(root, (), 0)]
    while stack:
        node, path, depth = stack.pop()
        # Keep even depth-64 trees inside the page. Full index paths retain
        # their depth when the indentation reaches the available margin.
        x = 60 + min(depth, 10) * 28
        text = f"{node.get('name', node.get('kind', 'part'))} [{count(node['letters'])}]"
        if show_spans and "span" in node:
            text += f" · [{node['span'][0]}, {node['span'][1]})"
        lines = svg.wrapped(text, 640 - x, 15)
        path_lines = svg.wrapped(str(list(path)), 118, 13)
        ref_text = next((key + " " + str(node[key]) for key in ("inverse_of", "same_as") if key in node), "")
        ref_lines = svg.wrapped(ref_text, 224, 13)
        h = max(22 * len(lines), 18 * len(path_lines), 18 * len(ref_lines)) + 12
        rows.append((node, path, depth, x, y, h, lines, path_lines, ref_lines))
        positions[path] = (x, y + h / 2)
        y += h + 7
        for i in reversed(range(len(node.get("parts", [])))):
            stack.append((node["parts"][i], path + (i,), depth + 1))
    for node, path, depth, x, yy, h, lines, path_lines, ref_lines in rows:
        if path:
            px, py = positions[path[:-1]]
            svg.path(f"M {fmt(px + 8)} {fmt(py + 8)} V {fmt(yy + h / 2)} H {fmt(x - 5)}")
        svg.rect(x, yy, 650 - x, h, fill=PALETTE["panel"] if "parts" in node else PALETTE["white"],
                 stroke=PALETTE["line"], rx=4)
        for j, line in enumerate(lines):
            svg.text(x + 10, yy + 23 + j * 22, line, size=15)
        for j, line in enumerate(path_lines):
            svg.text(670, yy + 22 + j * 18, line, size=13, colour=PALETTE["muted"])
    reference_index = 0
    for node, path, depth, x, yy, h, lines, path_lines, ref_lines in rows:
        for key, colour in (("inverse_of", "purple"), ("same_as", "teal")):
            if key not in node:
                continue
            target = tuple(node[key])
            if target not in positions:
                raise ValueError("tree reference has no target")
            tx, ty = positions[target]
            lane = 1040 + (reference_index % 8) * 14
            cy = yy + h / 2
            for j, line in enumerate(ref_lines):
                svg.text(800, yy + 22 + j * 18, line, size=13, colour=PALETTE[colour])
            svg.path(f"M 1024 {fmt(cy)} H {lane} V {fmt(ty)} H 650", colour=colour,
                     arrow=True, dashed=key == "same_as")
            reference_index += 1
    return y + 22


def named_tree(name, parts):
    """Build symbolic or record-derived word structure, deriving every length."""
    return {"name": name, "letters": sum(p["letters"] for p in parts), "parts": parts}


def leaf(name, letters, **refs):
    return dict(name=name, letters=letters, **refs)


def record_tree(evidence, record, step):
    evidence.raw(API_PATH)
    document = evidence.json(record + "/run.json")
    rows = [s for s in document["steps"] if s["index"] == step]
    if len(rows) != 1:
        raise ValueError("record has no unique step " + str(step))
    row = rows[0]
    path = str(PurePosixPath(record) / row["tree"]["file"])
    # A path inside a run must not escape that run, even while staying in the repo.
    if not repository_path(path).resolve().is_relative_to(repository_path(record).resolve()):
        raise ValueError("tree file leaves its run")
    raw = evidence.raw(path)
    if raw[:2] != b"\x1f\x8b" or raw[4:8] != bytes(4):
        raise ValueError("tree is not deterministic gzip")
    unpacked = gzip.decompress(raw)
    if sha256(unpacked) != row["tree"]["sha256"]:
        raise ValueError("tree digest differs from record")
    root = json.loads(unpacked)
    if tree_api().check_tree(root, row["letters"]) != row["tree"]["size"]:
        raise ValueError("tree size differs from record")
    return row, root


@figure("01-menu-items", "01-state.md", "The menu, grouped by order and linked to inverses")
def menu_items(e, s):
    data = e.record(1)["A_menu"]
    y = s.legend([("retained", PALETTE["blue"]), ("jumble", PALETTE["orange"]),
                  ("a loop marks a self-inverse item", PALETTE["muted"])], 92)
    for order, items in sorted(data["orders"].items(), key=lambda pair: int(pair[0])):
        s.text(40, y + 25, "Order " + order, weight="600")
        sequence, seen = [], set()
        for item in items:
            if item in seen:
                continue
            inverse = data["inverse"][item]
            sequence.append(item)
            seen.add(item)
            if inverse != item:
                sequence.append(inverse)
                seen.add(inverse)
        positions = {item: (145 + (i % 12) * 84, y + (i // 12) * 88 + 26)
                     for i, item in enumerate(sequence)}
        for item in sequence:
            x, yy = positions[item]
            inverse = data["inverse"][item]
            if inverse == item:
                s.path(f"M {x + 22} {yy} C {x + 4} {yy - 23}, {x + 60} {yy - 23}, {x + 44} {yy}")
            elif item < inverse:
                xx, other_y = positions[inverse]
                s.path(f"M {x + 33} {yy} V {yy - 15} H {xx + 33} V {other_y}")
        for item in sequence:
            x, yy = positions[item]
            colour = PALETTE["blue"] if item in data["retained"] else PALETTE["orange"]
            s.rect(x, yy, 66, 42, fill=PALETTE["white"], stroke=colour, rx=5)
            s.text(x + 33, yy + 28, item, anchor="middle", colour=colour, size=18)
        y += math.ceil(len(sequence) / 12) * 88 + 20
    return y


@figure("01-height-histograms", "01-state.md", "Left-height histograms at the recorded starts and stops")
def height_histograms(e, s):
    data = e.record(1)
    cases = [("seed 7001 · start", data["C_seed7001"]["heights"]),
             ("seed 7101 · start", data["F_seed7101"]["start"]["heights"])]
    census = e.text(REST + "/11-scramble-census/README.md")
    for table in tables(census):
        if table[0][-1] != "Left heights at the end":
            continue
        for row in table[1:]:
            hist = {h: integer(n) for h, n in re.findall(r"(\d+): ([\d,]+)", row[-1])}
            cases.append(("seed " + row[1] + " · census stop", hist))
    heights = sorted({int(h) for _, hist in cases for h in hist})
    maximum = max(v for _, hist in cases for v in hist.values())
    for i, (label, hist) in enumerate(cases):
        x, y = 40 + (i % 2) * 570, 90 + (i // 2) * 300
        s.rect(x, y, 550, 280, stroke=PALETTE["line"], rx=6)
        s.text(x + 20, y + 30, label, size=18, weight="600")
        baseline = y + 226
        for j, height in enumerate(heights):
            value = hist.get(str(height), 0)
            xx = x + 86 + j * 155
            bar = value / maximum * 145
            s.rect(xx, baseline - bar, 76, bar, fill=ACCENTS[j % len(ACCENTS)])
            s.text(xx + 38, baseline - bar - 9, count(value), size=14, anchor="middle")
            s.text(xx + 38, baseline + 25, "height " + str(height), size=14, anchor="middle")
        s.line(x + 40, baseline, x + 520, baseline, colour=PALETTE["line"])
    y = 90 + math.ceil(len(cases) / 2) * 300
    return s.paragraph(40, y, "Equal vertical scales in every panel; only off-lattice pieces are counted. "
                       "The two starts are computed; the census stops are finite measured records.")


@figure("01-cover-graph-7101", "01-state.md", "The cover relation before the next census step")
def cover_graph(e, s):
    data = e.record(1)["F_seed7101"]["before_step_16"]
    graph = {int(c): ds for c, ds in data["cover_graph"].items()}
    nodes = set(graph) | set(data["cover_tops"])
    if len(nodes) != data["misaligned_caps"]:
        raise ValueError("the record no longer identifies all misaligned caps")
    edges = {c: sorted(set(graph.get(c, [])) & nodes) for c in sorted(nodes)}
    # Condense strongly connected components: the float cover relation has cycles.
    reach = {c: {c} | set(edges[c]) for c in sorted(nodes)}
    for k in sorted(nodes):
        for c in sorted(nodes):
            if k in reach[c]:
                reach[c].update(reach[k])
    components, remaining = [], set(nodes)
    while remaining:
        c = min(remaining)
        component = sorted(d for d in remaining if d in reach[c] and c in reach[d])
        components.append(component)
        remaining.difference_update(component)
    component_of = {c: i for i, cs in enumerate(components) for c in cs}
    @lru_cache(maxsize=None)
    def layer(ci):
        following = {component_of[d] for c in components[ci] for d in edges[c] if component_of[d] != ci}
        return 1 + max(map(layer, following)) if following else 0
    levels = {}
    for c in sorted(nodes):
        levels.setdefault(layer(component_of[c]), []).append(c)
    y = s.legend([("cover top", PALETTE["teal"]), ("other misaligned cap", PALETTE["grey"])], 92)
    y = s.paragraph(40, y, "Arrow c → d means d covers c. Layers use the condensed cover relation; mutual covers share a layer.") + 16
    positions = {}
    for level, caps in sorted(levels.items()):
        s.text(40, y + 28, "Layer " + str(level), size=14)
        for i, c in enumerate(caps):
            positions[c] = (165 + (i % 10) * 98, y + (i // 10) * 80)
        y += math.ceil(len(caps) / 10) * 80 + 80
    for c in sorted(nodes):
        x1, y1 = positions[c]
        for d in edges[c]:
            x2, y2 = positions[d]
            if y1 == y2:
                s.path(f"M {x1 + 42} {y1} Q {fmt((x1 + x2) / 2 + 42)} {y1 - 42} {x2 + 42} {y2}", arrow=True)
            else:
                s.line(x1 + 42, y1, x2 + 42, y2 + 48, arrow=True)
    for c, (x, yy) in sorted(positions.items()):
        s.rect(x, yy, 84, 48, fill=PALETTE["teal"] if c in data["cover_tops"] else PALETTE["panel"], rx=5)
        s.text(x + 42, yy + 21, c, anchor="middle", colour=PALETTE["white"] if c in data["cover_tops"] else None)
        external = len(set(graph.get(c, [])) - nodes)
        s.text(x + 42, yy + 40, "+" + str(external) + " other", size=13, anchor="middle",
               colour=PALETTE["white"] if c in data["cover_tops"] else PALETTE["muted"])
    return s.paragraph(40, y, "Only misaligned caps are vertices. Each '+ other' count is the number of covering caps outside "
                       "that set; their individual edges are in the source record. The cover test uses float centroids.")


@figure("01-phase-transitions", "01-state.md", "Phase order and every recorded end result")
def phase_transitions(e, s):
    data = e.record(7)["A_d0"]["config"]
    y = phase_strip(s, data["phase_order"], 92)
    y = s.paragraph(40, y, "Height sub-phases: " + " → ".join(data["subphase_order"]["height"]) + ". No transition returns to an earlier phase.")
    table = table_after(e.chapter, "**Transitions.**")
    rows = [(row[0] + " · " + row[1] + "\n" + row[2], row[3]) for row in table[1:]]
    e.text(T + "/02-bulk.md")
    rows += [("bulk · child crash or failed word contract", "run ends error"),
             ("any phase · operator signal", "run ends error; lattice keeps a cancelled attempt and no phase result")]
    return flow(s, rows, y + 20)


@figure("01-step-tree-closure", "01-state.md", "A closure and a T1′ step, folded with inverse links")
def step_tree_closure(e, s):
    examples = e.record(7)["E_trees"]
    y = folded_tree(s, examples["b20"]["tree"], 90, "b20 · closure")
    return folded_tree(s, examples["b07"]["tree"], y + 20, "b07 · T1′")


@figure("bulk-n-7101", "02-bulk.md", "Census descent: N after each step, coloured by rule")
def bulk_n(e, s):
    base = REST + "/11-scramble-census/runs/"
    first = e.json(base + "round1/ia-k40-s7101.json")
    data = e.json(base + "round2/ia-k40-s7101.json")
    steps = data["steps"]
    if steps[:len(first["steps"])] != first["steps"]:
        raise ValueError("round 2 no longer extends round 1")
    rules = sorted({row["rule"] for row in steps})
    colours = dict(zip(rules, ACCENTS))
    y = s.legend([(rule, colours[rule]) for rule in rules], 92)
    values = [steps[0]["N_before"]] + [row["N_after"] for row in steps]
    point_colours = [PALETTE["muted"]] + [colours[row["rule"]] for row in steps]
    y = plot(s, [{"label": "N", "values": values}], list(map(str, range(len(values)))), y,
             "N · step 0 is the start", point_colours=point_colours, height=370, label_stride=10)
    return s.paragraph(40, y, f"Round 1 ends after step {len(first['steps'])}; round 2's cumulative record ends after step {len(steps)}. "
                       "Every point is a census step, not a runner step. The two rounds are not concatenated twice.")


@figure("bulk-conj-7101-s30", "02-bulk.md", "A three-part conjugate repair and its cap counts")
def bulk_conj(e, s):
    rows = [json.loads(line) for line in e.text("research/jumbling/manual/scripts/examples.out").splitlines()]
    row = next(r for r in rows if r.get("record") == "ia-k40-s7101.json" and r.get("step") == 30)
    word, trace = row["word"], row["trace"]
    root = named_tree(row["rule"], [leaf(label + " = " + str(tuple(letter)), len([letter]))
                                    for label, letter in zip(("x", "y", "z"), word)])
    y = folded_tree(s, root, 90)
    c, d = word[0][0], word[1][0]
    data = e.figdata()
    if data["word"] != word:
        raise ValueError("computed repair word differs from the manual")
    counts = [[label, count(point["N"]), count(point["N_c"][str(c)]), count(point["N_c"][str(d)])]
              for label, point in zip(("before", "after x", "after y", "after z"), data["states"])]
    return draw_table(s, ["State", "N", f"N_{c}", f"N_{d}"], counts, y)


def height_examples(e):
    first = tagged_lines(e.text(ROUND + "/r22d75_height_first.txt"))
    second = tagged_lines(e.text(ROUND + "/r22d81_rule.txt"))
    initial = next(row for tag, row in first if tag == "START")
    points = [{"counts": initial["n6_n4_n2"], "u": None, "rule": "start", "digest": initial["digest"]}]
    previous = initial["n6_n4_n2"]
    for tag, row in first:
        if tag == "STEP":
            now = row["n6_n4_n2"]
            tier = next(i for i, (a, b) in enumerate(zip(previous, now)) if a != b)
            points.append({"counts": now, "u": None, "rule": "r22d75/T" + str(tier), "digest": row["digest"]})
            previous = now
    for tag, row in second:
        if tag == "START":
            points[-1]["u"] = row["u"]
        if tag == "STEP":
            points.append({"counts": list(reversed(row["n2_n4_n6"])), "u": row["u"],
                           "rule": "r22d81/" + row["family"], "digest": row["digest"]})
    boundary = len(points) - 1
    # The cloud summary records individual letters. Join the two-letter U pairs
    # into the same steps that the chapter/plugin records, without losing counts.
    text = e.text(CLOUD + "/RESULT.md")
    table = table_after(text, "The 13 letters of this line, from bbef1c8e:")
    u_by_digest = {}
    for path in sorted(repository_path(CLOUD).glob("h4_unlock_run[1-4].txt")):
        for tag, row in tagged_lines(e.text(path.relative_to(ROOT).as_posix())):
            if "digest" in row and "u" in row:
                u_by_digest[row["digest"][:8]] = row["u"]
    # Each next pair's CONTEXT records the previous pair's resulting u.
    for round_no in (2, 3, 4):
        for tag, row in tagged_lines(e.text(CLOUD + f"/two_letter_u_round{round_no}_shard0.txt")):
            if tag == "CONTEXT":
                u_by_digest[row["digest"][:8]] = row["u"]
    for row in table[1:]:
        if "two-letter U, first" in row[2]:
            continue
        digest = row[-1].replace("…", "")
        # The thousands separators and tuple separators both contain commas.
        # Parse the tuple through its comma-space boundaries, not bare commas.
        counts = [integer(x) for x in row[3].strip("() ").split(", ")]
        rule = "h4/pair" if "two-letter" in row[2] else "h4/L4" if "L4" in row[2] else "h4/U" if "unlock" in row[2] else "r22d75/T2"
        points.append({"counts": list(reversed(counts)), "u": u_by_digest.get(digest), "rule": rule, "digest": digest})
    return points, boundary


@figure("03-counts-7302", "03-height.md", "Height counts and target straddlers along the recorded steps")
def height_counts(e, s):
    points, boundary = height_examples(e)
    computed = e.figdata()["points"]
    if len(computed) != len(points) or any(not c["state_digest"].startswith(p["digest"])
                                          for c, p in zip(computed, points)):
        raise ValueError("computed target states differ from the chart")
    for point, value in zip(points, computed):
        point["u"] = value["u"]
    rules = list(dict.fromkeys(p["rule"] for p in points))
    colours = {rule: (PALETTE["muted"] if rule == "start" else ACCENTS[(i - 1) % len(ACCENTS)])
               for i, rule in enumerate(rules)}
    y = s.legend([(r, colours[r]) for r in rules], 92)
    labels = list(map(str, range(len(points))))
    for i, label in enumerate(("n6", "n4", "n2", "u")):
        values = [p["counts"][i] if i < 3 else p["u"] for p in points]
        y = plot(s, [{"label": label, "values": values}], labels, y, label + " after each step",
                 point_colours=[colours[p["rule"]] for p in points], height=185)
    y = draw_table(s, ["Point", "H", "Target τ", "Straddlers u"],
                   [[str(p["index"]), str(p["H"]) if p["H"] is not None else "—",
                     str(p["tau"]) if p["tau"] is not None else "no target",
                     count(p["u"]) if p["u"] is not None else "—"] for p in computed], y)
    y = s.paragraph(40, y, "H = 6 while n6 > 0, then H = 4 while n4 > 0. "
                    "At the H6 handoff the target is recomputed for H4; the old H6 u = 0 belongs to the absent H6 target. "
                    "A gap means no target. Example (c) restarts the height phase.") + 20
    return s.paragraph(40, y, f"Example (b): step 0 to {boundary}, ending at {points[boundary]['digest'][:8]}… . "
                       f"Example (c): the remaining {len(points) - boundary - 1} steps, ending at {points[-1]['digest'][:8]}… . "
                       "Markers show the rule; each panel has its own vertical scale.")


@figure("03-r22d75-prediction-cap64", "03-height.md", "The count prediction at the first height letter")
def count_prediction(e, s):
    rows = tagged_lines(e.text(ROUND + "/r22d75_height_first.txt"))
    before = next(row for tag, row in rows if tag == "START")
    after = next(row for tag, row in rows if tag == "STEP")
    y = s.paragraph(40, 96, "Letter " + str(tuple(after["word"][0])) + ": " + before["digest"][:8] + "… → " + after["digest"][:8] + "…") + 24
    values = [[name, count(a), f"{b-a:+,}", count(b)] for name, a, b in
              zip(("n6", "n4", "n2"), before["n6_n4_n2"], after["n6_n4_n2"])]
    y = draw_table(s, ["Count", "Before", "Δ", "After"], values, y)
    data = e.figdata()
    if data["state_digest"] != before["digest"] or data["after_digest"] != after["digest"]:
        raise ValueError("computed prediction states differ from the record")
    y = draw_table(s, ["Inside the cap", "h = 6", "h = 4", "h = 2"], [
        [label] + [count(data[key][str(h)]) for h in (6, 4, 2)]
        for key, label in (("F_h", "F_h · other branch"), ("O_h", "O_h · own branch"),
                           ("r_h", "r_h · lowered own branch"))
    ], y, [340, 260, 260, 260])
    y = s.paragraph(40, y, "L₀ · lattice pieces inside cap 64: " + count(data["L0"]) + ".") + 16
    if data["proof_lattice_off"] != data["L0"]:
        y = s.paragraph(40, y, "R22D-74's “takes lattice pieces off” count is " + count(data["proof_lattice_off"]) +
                        "; L₀ counts lattice pieces inside the cap before the letter.") + 16
    y = s.paragraph(40, y + 10, "Δ(n6, n4, n2) = (F₄ − r₆, F₂ + r₆ − F₄ − r₄, L₀ + r₄ − F₂ − r₂).", size=18)
    return y + 20


@figure("03-call-flow", "03-height.md", "How next_step and _call classify a program's output")
def call_flow(e, s):
    y = flow(s, [
        ("next_step: check height invariant; write snapshot; start the rule program", "height invariant fails → error"),
        ("_call 1: operator signal?", "yes → error / cancelled"),
        ("2: parse complete lines; stop at first STEP or h4 START with null target", "unknown tags skipped; malformed payload retained as parse error"),
        ("3: verify START / CONTEXT digest and expected target, u and shard fields", "mismatch → digest_mismatch / inconsistent_output"),
        ("4: first STEP? Or h4 START with null target?", "proposal; or stuck / no_target"),
        ("5: malformed, mismatch, missing_step or crash parse status?", "error before checking the clock"),
        ("6: shard DONE: FIRST indexes exactly its share of the order?", "failure → inconsistent_output"),
        ("7: wall clock killed the child?", "timeout / wall_clock"),
        ("8: normal exit and end line: START, step count, digest, target and u checks", "HEIGHT2 / HANDOFF → handoff; BUDGET → budget; UNLOCKED / STUCK → stuck"),
        ("9: SIGXCPU at the CPU limit?", "budget / cpu_rlimit"),
        ("10: remaining nonzero exit / traceback, or incomplete output", "error / crash, or truncated"),
        ("next_step: STUCK with a target? r22d81 calls its pair program; h4 calls pair shards", "otherwise return the sub-phase end; proposal → runner plays and verifies"),
    ], 90)
    formats = e.record(3)["B_formats"]
    errors = [r for r in formats if r["reason"] == "malformed_output"]
    return draw_table(s, ["Zero-step program end at the chapter's pinned revision", "Actual result / reason"],
                      [[r["label"], r["result"] + " / " + r["reason"]] for r in errors], y, [800, 320])


@figure("ch04-family-order", "04-high.md", "The kernel's families, gates and search exits")
def family_order(e, s):
    table = table_after(e.chapter, "## 4.6 The order of families")
    y = s.paragraph(40, 96, "Before the families: invalid limits or snapshot → error; height above the supported range → "
                    "unsupported; no off piece → exhausted. F1 → F2 → F3 runs for one unit, then repeats for the next unit.") + 18
    rows = []
    for row in table[1:]:
        rows.append((row[1] + " · " + row[2] + "\nGate: " + row[5] + "\nLetters: " + row[6],
                     "gate passes → proposal; no candidate → next family / unit" if row[1] != "none" else "exhausted → kernel stuck"))
    y = flow(s, rows, y)
    return s.paragraph(40, y, "Every family checks its clock: a stop returns budget with that family's cursor. "
                       "An uncertain twist or classification, failed composition, invalid D19 answer or other exception "
                       "returns error. A missing D19 program returns unsupported / d19_missing. A gate rejection tries the next candidate.")


@figure("ch04-tier-shapes", "04-high.md", "F2 and F3 word shapes and letterwise inverse links")
def tier_shapes(e, s):
    symbols = ("(c, x)", "(d, y)", "(c, z)")
    f2 = named_tree("r22d25/F2", [leaf(name, len([name])) for name in symbols])
    y = folded_tree(s, f2, 90)
    lengths = [int(n) for n in re.search(r"k = (\d+) or (\d+)", e.chapter).groups()]
    for k in lengths:
        a = leaf("a = (c, j)", len(["a"]))
        w = named_tree("W", [leaf("w" + str(i + 1), len([i])) for i in range(k)])
        ai = leaf("a⁻¹ = (c, inverse(j))", a["letters"], inverse_of=[0])
        wi = named_tree("W⁻¹", [leaf("w" + str(i + 1) + "⁻¹", len([i]), inverse_of=[1, i])
                                for i in reversed(range(k))])
        wi["inverse_of"] = [1]
        y = folded_tree(s, named_tree("r22d25/F3", [a, w, ai, wi]), y, "F3 · |W| = " + str(k))
    return s.paragraph(40, y, "Symbolic word shapes from the rule definition. No F2 or F3 step has been recorded at a development state.")


def literal_assignment(text, name):
    for statement in ast.parse(text).body:
        if isinstance(statement, ast.Assign) and any(isinstance(t, ast.Name) and t.id == name for t in statement.targets):
            return ast.literal_eval(statement.value)
    raise ValueError("missing literal assignment: " + name)


@figure("ch04-d19-tree", "04-high.md", "The recorded D19 word as named parts and repeated commutators")
def d19_tree(e, s):
    word = literal_assignment(e.text(LOW + "/scripts/round22/r22d16_step.py"), "word")
    # The chapter defines |V| by the word's length modulo the commutator length.
    commutator_size = len(("x", "y", "x⁻¹", "y⁻¹"))
    v_size = 0 if len(word) % commutator_size == 2 else 1
    start, stop = 1 + v_size, len(word) - v_size - 1
    middle = word[start:stop]
    comm = middle[:commutator_size]
    if middle != comm * (len(middle) // commutator_size):
        raise ValueError("D19 middle is no longer a repeated commutator")
    copies = []
    for i in range(len(middle) // commutator_size):
        if i:
            copies.append(leaf("C = " + " · ".join(str(tuple(a)) for a in comm),
                               len(comm), same_as=[2, 0]))
        else:
            parts = []
            for j, (label, letter) in enumerate(zip(("x", "y", "x⁻¹", "y⁻¹"), comm)):
                refs = {"inverse_of": [2, 0, j - 2]} if j >= 2 else {}
                parts.append(leaf(label + " = " + str(tuple(letter)), len([letter]), **refs))
            copies.append(named_tree("C = " + " · ".join(str(tuple(a)) for a in comm), parts))
    root = named_tree("r22d25/d19", [
        leaf("a = " + str(tuple(word[0])), len(word[:1])),
        leaf("V = " + " · ".join(str(tuple(a)) for a in word[1:start]), v_size),
        named_tree("W = C^" + str(len(copies)), copies),
        leaf("V⁻¹ = " + " · ".join(str(tuple(a)) for a in word[stop:-1]), v_size, inverse_of=[1]),
        leaf("a⁻¹ = " + str(tuple(word[-1])), len(word[-1:]), inverse_of=[0]),
    ])
    y = folded_tree(s, root, 90)
    return s.paragraph(40, y, "The word is read as a literal from r22d16_step.py without executing it. "
                       "The runner itself records D19 as a single span; this finer structure follows the rule's definition.")


@figure("ch04-six-stops-caps", "04-high.md", "Cap-level prunes in kernel order")
def six_stops_caps(e, s):
    document = e.json(ROUND + "/kernel-d19/bbef1c8e/kernel/run.json")
    pruned = document["ledger"]["S"]["pruned"]
    caps = e.record(4)["C_tables"]["home_chambers_per_cap"]["caps"]
    items = e.record(1)["A_menu"]["items"] - 1
    pairs = {reason: letters // items for reason, letters in pruned.items()}
    if any(letters % items for letters in pruned.values()) or sum(pairs.values()) % caps:
        raise ValueError("prune counts are not whole cap/unit pairs")
    units = sum(pairs.values()) // caps
    data = e.figdata()
    if data["state_digest"] != document["start"]["digest"] or data["totals"] != pairs or len(data["units"]) != units:
        raise ValueError("computed prune matrix differs from the ledger")
    y = s.paragraph(40, 96, f"State {document['start']['digest'][:8]}… · {units} height-4 units × {caps} caps = "
                    f"{count(units * caps)} pairs. Each cell is the first cap-level prune in the kernel's order.") + 20
    reasons = ("blocked", "target_not_whole", "lattice_inside")
    colours = dict(zip(reasons, ACCENTS))
    colours[None] = PALETTE["white"]
    y = s.legend([(r, colours[r]) for r in reasons] + [("no cap-level prune", colours[None])], y)
    x = 80
    for i, reason in enumerate(reasons):
        width = 1060 * pairs[reason] / sum(pairs.values())
        element = s.rect(x, y, width, 32, fill=ACCENTS[i])
        ET.SubElement(element, "{" + SVG_NS + "}title").text = reason + ": " + count(pairs[reason]) + " pairs"
        x += width
    y = draw_table(s, ["Prune", "Cap/unit pairs", "Letters pruned"],
                   [[r, count(pairs[r]), count(pruned[r])] for r in reasons], y + 52, [500, 310, 310])
    s.text(80, y + 18, "Caps (kernel order); rows are height-4 units in kernel order", size=15)
    y += 45
    width, height = 1060, units * 6
    for i, unit in enumerate(data["units"]):
        # Adjacent equal cells share one SVG rectangle, preserving every pair.
        start = 0
        for end in range(1, caps + 1):
            reason = unit["reasons"][start]
            if end == caps or unit["reasons"][end] != reason:
                element = s.rect(80 + start * width / caps, y + i * 6, (end - start) * width / caps, 6,
                                 fill=colours[reason], data_unit=i, data_cap_start=start, data_cap_end=end,
                                 data_reason=reason or "none")
                ET.SubElement(element, "{" + SVG_NS + "}title").text = (
                    f"Unit {i + 1} (first piece {unit['first']}), caps {start}–{end - 1}: {reason or 'none'}")
                start = end
    for i in range(caps + 1):
        xx = 80 + i * width / caps
        s.line(xx, y, xx, y + height, colour=PALETTE["white"], width=.25)
    for i in range(units + 1):
        yy = y + i * height / units
        s.line(80, yy, 1140, yy, colour=PALETTE["white"], width=.6)
        if i < units and i % 10 == 0:
            s.text(70, yy + 5, i + 1, size=13, anchor="end")
    for i in range(0, caps, 100):
        s.text(80 + i * width / caps, y + height + 23, i, size=13)
    s.text(1140, y + height + 23, caps - 1, size=13, anchor="end")
    return y + height + 45


@figure("05-tail-loop", "05-tail.md", "One pass through the tail's units, rules and gates")
def tail_loop(e, s):
    options = e.record(5)["options"]
    y = phase_strip(s, options["tail_rules"], 90)
    return flow(s, [
        ("Step budget reached?", "yes → tail and run end budget"),
        ("Census units in the current state and read them in unit order", "census error → run error"),
        ("For this unit: try rules in the order above, testing each candidate's gate", "rejected candidate / failed attempt → next rule"),
        ("First unit with a candidate whose gate passed", "play, verify, record; next pass at the new state"),
        ("No candidate: if a unit exists and fallback length is positive, probe the first unit", "development: play a found word; acceptance: record probe only"),
        ("Still no candidate: save the stuck record and all attempts at this state", "timed-out attempt → tail timeout / run budget; otherwise tail stuck / run stuck"),
    ], y)


@figure("05-tree-b07-certificate", "05-tail.md", "The b07 T1′ tree and the missing certificate expansion")
def b07_certificate(e, s):
    data = e.record(7)["E_trees"]["b07"]
    y = folded_tree(s, data["tree"], 90)
    inventory = e.json(ROUND + "/line-a/literal/certificates.json")
    name = "run2/03-b07-v-cert.json.gz"
    word = e.json(ROUND + "/line-a/literal/run2/03-b07-V-word.json.gz")
    letters = len(word["word"])
    if letters != data["tree"]["parts"][1]["letters"]:
        raise ValueError("b07 flat V word and recorded tree differ")
    y = s.paragraph(40, y, "The committed flat V′ word has " + count(letters) + " letters. The certificate inventory names "
                    + name + ", sha256 " + inventory[name]["sha256"] + ".")
    e.note("The certificate's contents are not committed. Its per-orbit cascade, three-cycle steps, conjugators and "
           "seeds cannot be recovered from its checksum or flat word without new analysis. V′ stays folded at the runner's recorded boundary.")
    return y + 20


def e2_steps(e):
    base = ROUND + "/line-a/literal/"
    result = []
    for run in ("run1", "run2", "run2b", "run3/t1"):
        for line in e.text(base + run + "/seq.jsonl").splitlines():
            row = json.loads(line)
            if "letters" in row and "off_after" in row:
                result.append(row)
    for a, b in zip(result, result[1:]):
        if a["end_digest"] != b["start_digest"] or a["off_after"] != b["off_before"]:
            raise ValueError("E2 trajectory is not continuous")
    return result


@figure("05-e2-trajectory", "05-tail.md", "E2's off count and literal step lengths")
def e2_trajectory(e, s):
    rows = e2_steps(e)
    colours = [PALETTE["purple"] if "cert" in row else PALETTE["blue"] for row in rows]
    y = s.legend([("certificate V", PALETTE["purple"]), ("other literal word", PALETTE["blue"])], 92)
    y = plot(s, [{"label": "off", "values": [rows[0]["off_before"]] + [r["off_after"] for r in rows]}],
             list(map(str, range(len(rows) + 1))), y, "Off count · step 0 is the start", point_colours=[PALETTE["muted"]] + colours)
    # One series per kind keeps the original positions and marks every certificate.
    series = [{"label": label, "values": [r["letters"] if ("cert" in r) == cert else None for r in rows], "colour": colour}
              for label, cert, colour in (("certificate V", True, PALETTE["purple"]), ("literal", False, PALETTE["blue"]))]
    y = plot(s, series, [str(i + 1) for i in range(len(rows))], y, "Letters per step · log₁₀ scale", log=True, bars=True)
    y = draw_table(s, ["Step", "Unit", "Kind", "Letters", "Off after", "Certificate V"],
                   [[str(i + 1), r["unit"], r["how"], count(r["letters"]), count(r["off_after"]), "yes" if "cert" in r else "—"]
                    for i, r in enumerate(rows)], y, [80, 100, 260, 230, 230, 220])
    return s.paragraph(40, y, "These are E2 replay scripts, not a runner trajectory. Certificate V and V⁻¹ were checked "
                       "as SAFE-ALL words and applied as composed actions; this is not evidence of twist-by-twist play of every step.")


@figure("ch06-phase-flow", "06-lattice.md", "The lattice phase: children, checks and every exit")
def lattice_flow(e, s):
    y = phase_strip(s, ["entry", "solved shortcut", "restorer child", "player child", "checks", "result"], 90)
    y = flow(s, [
        ("After the tail: off = 0?", "no → lattice phase not entered"),
        ("Digest equals the solved digest?", "yes → run solved; phase not entered"),
        ("Development option --no-lattice?", "yes → run budget; phase not entered"),
        ("Run restorer, validate summary; run literal player; validate words, trees, steps and end table", "all checks pass, including identity labels → solved"),
    ], y)
    table = table_after(e.chapter, "**Exits.**")
    y = flow(s, [(row[0], row[1] + " / " + row[2]) for row in table[1:]], y)
    y = s.paragraph(40, y, "The runner also checks every step and the end table. A zero-step LatticeResult must leave the state "
                    "unplayed with identity labels. An invalid result or failed check ends error. An operator signal ends the run "
                    "error and keeps the phase result null, with the cancelled attempt.")
    return s.paragraph(40, y + 12, "Both children run with no clock in the chapter's configuration. Clock exits are part of the interface, "
                       "but are not observed clock events in these records.")


def coordinates(e):
    code = e.text(REST + "/09-general-method/scripts/lattice/scripts/nucoords.py")
    bits = literal_assignment(code, "C2F")
    fifths = literal_assignment(code, "C5F")
    orbits = e.record(6)["B_orbits"]["orbits"]
    spec = [("b" + str(o), o, 0, 2) for o in sorted(row["orbit"] for row in orbits)]
    spec += [("f" + str(o), o, 1, 2) for o in bits]
    spec += [("C5_" + str(o), o, 1, 5) for o in fifths]
    return spec


def nu_vector(nu, spec):
    return [nu.get(str(o), [0, 0])[component] % modulus for _, o, component, modulus in spec]


@figure("ch06-nu-grid", "06-lattice.md", "The ν coordinates and the chosen corrections")
def nu_grid(e, s):
    data = e.record(6)["F_depth40"]
    spec = coordinates(e)
    values = nu_vector(data["nu_input"], spec)
    fixed_text = e.chapter.split("the nine coordinates", 1)[1].split("are always 0", 1)[0]
    fixed = set(re.findall(r"[bf]\d+", fixed_text))
    y = s.legend([("always zero", PALETTE["grey"]), ("nonzero start value", PALETTE["orange"]),
                  ("zero start value", PALETTE["panel"])], 92)
    for i, ((name, orbit, component, modulus), value) in enumerate(zip(spec, values)):
        x, yy = 40 + (i % 9) * 125, y + (i // 9) * 72
        colour = PALETTE["grey"] if name in fixed else PALETTE["orange"] if value else PALETTE["panel"]
        s.rect(x, yy, 114, 60, fill=colour, stroke=PALETTE["line"], rx=5)
        text_colour = PALETTE["white"] if value and name not in fixed else PALETTE["ink"]
        s.text(x + 57, yy + 23, name, size=15, anchor="middle", colour=text_colour)
        s.text(x + 57, yy + 46, str(value) + " mod " + str(modulus), size=15, anchor="middle", colour=text_colour)
    y += math.ceil(len(spec) / 9) * 72 + 30
    library = e.json(REST + "/09-general-method/data/library-merged.json.gz")["macros"]
    current = list(values)
    rows = []
    for correction in data["corrections"]:
        macro = library[correction["macro"]]
        vector = nu_vector(macro["nu"], spec)
        direction = -1 if correction["inverse"] else 1
        delta = [(direction * correction["times"] * value) % coordinate[3] for value, coordinate in zip(vector, spec)]
        changes = [name + " += " + str(v) + " (mod " + str(modulus) + ")"
                   for (name, _, _, modulus), v in zip(spec, delta) if v]
        current = [(a + b) % coordinate[3] for a, b, coordinate in zip(current, delta, spec)]
        rows.append([str(correction["macro"]) + ("⁻¹" if correction["inverse"] else ""),
                     correction["family"], str(correction["times"]), count(correction["letters"]), "; ".join(changes)])
    if any(current):
        raise ValueError("the chosen corrections do not cancel the recorded ν")
    y = draw_table(s, ["Macro", "Family", "Times", "Letters", "Coordinates changed"], rows, y, [90, 230, 80, 120, 600])
    return s.paragraph(40, y, f"Start ν: seed 7101's depth-40 stop. The {len(rows)} corrections sum to −ν modulo each coordinate, "
                       f"leaving every coordinate zero; {count(data['correction_twists'])} correction letters. "
                       "This grid shows the invariant, not a puzzle state.")


@figure("ch06-twisted-cycle", "06-lattice.md", "One twisted cycle as a strip of played parts")
def twisted_cycle(e, s):
    data = e.record(6)
    computed = e.figdata()
    factor = data["D_factors"]["log"]["6"][0]
    orbit = next(row for row in data["B_orbits"]["orbits"] if row["orbit"] == 6)
    names = ("g", "r⁻¹", "c", "r", "g⁻¹")
    lengths = (factor["g"], factor["r"], factor["seed"], factor["r"], factor["g"])
    x, y = 40, 116
    for i, (name, length) in enumerate(zip(names, lengths)):
        width = 1120 * length / factor["length"]
        s.rect(x, y, width, 64, fill=ACCENTS[i % len(ACCENTS)])
        s.text(x + width / 2, y + 39, name + " [" + str(length) + "]", size=16, anchor="middle", colour=PALETTE["white"])
        x += width
    a, x_pos, y_pos = factor["positions"]
    b, c = orbit["B"], orbit["C"]
    y = s.paragraph(40, 218, f"A = {a}; X = {x_pos}; Y = {y_pos}; B = {b}; C = {c}. "
                    f"Total {factor['length']} letters. Strip widths are proportional to their recorded lengths.")
    stage_names = ("before",) + tuple("after " + name for name in names)
    # Follow the actual pieces initially at A, X, Y, including frame changes.
    positions = list(zip(*(track["positions"] for track in computed["tracked"])))
    lane_orders = ((a, x_pos, y_pos), (a, b, c), (a, b, c), (a, b, c), (a, b, c), (a, x_pos, y_pos))
    top = y + 100
    centers = [120 + i * 192 for i in range(len(stage_names))]
    for i, name in enumerate(stage_names):
        s.text(centers[i], top - 32, name, size=15, anchor="middle")
    for piece in range(len(factor["positions"])):
        previous = None
        for i, stage in enumerate(positions):
            yy = top + lane_orders[i].index(stage[piece]) * 112
            if previous:
                s.line(previous[0] + 57, previous[1], centers[i] - 60, yy, colour=ACCENTS[piece], arrow=True)
            previous = (centers[i], yy)
    for piece in range(len(factor["positions"])):
        for i, stage in enumerate(positions):
            yy = top + lane_orders[i].index(stage[piece]) * 112
            s.rect(centers[i] - 55, yy - 22, 110, 44, fill=ACCENTS[piece], rx=5)
            s.text(centers[i], yy + 5, stage[piece], size=15, anchor="middle", colour=PALETTE["white"])
    y = top + 280
    y = s.legend([("piece initially at " + label, ACCENTS[i]) for i, label in enumerate(("A", "X", "Y"))], y)
    y = draw_table(s, ["Played part", "Letters", "Pieces whose exact pose changes"],
                   [[name, str(length), count(part["count"])]
                    for name, length, part in zip(names, lengths, computed["parts"])], y, [300, 220, 600])
    return s.paragraph(40, y, "Whole factor: " + count(computed["whole_count"]) + " pieces change exact pose; "
                       + count(computed["collateral_count"]) + " collateral pieces besides the three tracked pieces. "
                       "Counts include frame changes at a fixed position. Piece IDs and positions before and after each part are in figdata.json.")


@figure("ch06-orbit6-tree", "06-lattice.md", "The orbit step, preserving all folded references")
def orbit6_tree(e, s):
    row, root = record_tree(e, DEV + "/d0-s7106/run", 19)
    y = s.paragraph(40, 96, row["rule"] + " · phase " + row["phase"] + " · " + count(row["letters"]) + " letters · "
                    + str(row["tree"]["size"]) + " parts; zero-length spans are retained.") + 16
    return folded_tree(s, root, y)


def cycle_diagram(s, labels, cx, top):
    points = ((cx, top), (cx + 105, top + 145), (cx - 105, top + 145))
    for p, q in zip(points, points[1:] + points[:1]):
        dx, dy = q[0] - p[0], q[1] - p[1]
        distance = math.hypot(dx, dy)
        s.line(p[0] + dx / distance * 48, p[1] + dy / distance * 48,
               q[0] - dx / distance * 50, q[1] - dy / distance * 50, arrow=True)
    for (x, y), label in zip(points, labels):
        s.box(x - 48, y - 24, 96, 48, label, size=15)


@figure("ch06-late-three-cycle", "06-lattice.md", "The late restorer's seed, star and three-cycle")
def late_three_cycle(e, s):
    data = e.record(6)
    first, second = data["E_late"]["orbit6"]["first_four_factors"][:2]
    cycle_diagram(s, ("A", "B", "C"), 220, 170)
    cycle_diagram(s, ("A", "B", "X"), 600, 170)
    cycle_diagram(s, ("A", "X", "Y"), 980, 170)
    for x, label in ((220, "seed c"), (600, "star(X) = u_X⁻¹ · c · u_X"), (980, "star(X)⁻¹ · star(Y)")):
        s.text(x, 398, label, size=16, anchor="middle")
    y = s.paragraph(40, 452, "u_X is the execution-tree path from C to X. A star moves A → B → X → A; "
                    "playing star(X)⁻¹ and then star(Y) gives A → X → Y → A.") + 20
    return draw_table(s, ["Orbit-6 example", "Recorded factor length", "Role"], [
        [first[0]["name"], count(first[0]["letters"]), "star(X)⁻¹"],
        [second[0]["name"], count(second[0]["letters"]), "star(Y)"],
    ], y, [300, 300, 520])


@figure("ch06-retained-first", "06-lattice.md", "Retained-first play and the independent junction check")
def retained_first(e, s):
    data = e.record(6)["F_depth40"]
    for x, label in ((70, "Y · starting labels"), (490, "Y · R"), (910, "identity labels")):
        s.box(x, 130, 220, 90, label, size=18)
    s.line(290, 175, 485, 175, arrow=True)
    s.text(390, 153, "R · orbit steps", anchor="middle")
    s.line(710, 175, 905, 175, arrow=True)
    s.text(810, 153, "C · corrections", anchor="middle")
    s.box(70, 390, 220, 90, "solved labels", size=18)
    s.box(490, 390, 220, 90, "T = solved · C⁻¹", size=18)
    s.line(290, 435, 485, 435, arrow=True)
    s.text(390, 413, "reverse, invert C", anchor="middle")
    s.path("M 600 390 V 225", colour="teal", arrow=True, dashed=True)
    s.paragraph(720, 312, "Junction: labels(Y · R) = labels(T); independently check T's digest.", width=410)
    y = s.paragraph(40, 545, "T is built aside for checking; its construction is not counted as played length. "
                    "The corrections are then played on Y · R.") + 20
    return s.paragraph(40, y, "Committed depth-40 example: order " + data["order"] + ", " + str(len(data["corrections"]))
                       + " corrections; T digest " + data["T_digest"] + ". The program is computed; no runner has played this phase.")


@figure("ch06-d20-steps", "06-lattice.md", "Letters per orbit step under the two restorers")
def d20_steps(e, s):
    data = e.record(6)
    run = e.json(DEV + "/d0-s7106/run/run.json")
    rows = [row for row in run["steps"] if row["phase"] == "lattice"]
    late = {row["orbit"]: row["letters"] for row in data["E_late"]["steps"]}
    labels = [str(row["orbit"]) for row in rows]
    y = s.legend([("schedule11 · committed D0 rows", PALETTE["blue"]),
                  ("late · computed program, record E", PALETTE["orange"])], 92)
    y = plot(s, [
        {"label": "schedule11", "values": [r["letters"] for r in rows], "colour": PALETTE["blue"]},
        {"label": "late", "values": [late[r["orbit"]] for r in rows], "colour": PALETTE["orange"]},
    ], labels, y, "Orbit in cascade order · letters, log₁₀ scale", log=True, bars=True, height=360)
    return s.paragraph(40, y, "schedule11 total " + count(sum(r["letters"] for r in rows)) + "; late total "
                       + count(sum(late.values())) + ". No corrections on this handoff. Exact per-step values are in each bar's SVG title. "
                       "The late series is the committed computed program, not a committed runner replay.")


@figure("07-t1prime-b04-tree", "07-records.md", "The b04 T1′ tree, to scale across its letters")
def b04_tree(e, s):
    tree = e.record(7)["E_trees"]["b04"]["tree"]
    y = s.paragraph(40, 96, "Each part occupies its exact half-open letter range. Arrows name the earlier part that it inverts.")
    s.box(40, y + 28, 1120, 54, tree["name"] + " [" + count(tree["letters"]) + "]", size=18)
    first_y = y + 126
    centers = []
    for i, part in enumerate(tree["parts"]):
        lo, hi = part["span"]
        x = 40 + 1120 * lo / tree["letters"]
        width = 1120 * (hi - lo) / tree["letters"]
        s.rect(x, first_y, width, 70, fill=ACCENTS[i], stroke=PALETTE["white"])
        s.text(x + width / 2, first_y + 42, part["name"] + " [" + count(part["letters"]) + "]",
               anchor="middle", colour=PALETTE["white"])
        s.text(x, first_y - 10, lo, size=13)
        centers.append(x + width / 2)
    s.text(1160, first_y - 10, tree["letters"], size=13, anchor="end")
    lane = first_y + 126
    for i, part in enumerate(tree["parts"]):
        if "inverse_of" in part:
            target = part["inverse_of"][0]
            s.path(f"M {fmt(centers[i])} {first_y + 70} V {lane} H {fmt(centers[target])} V {first_y + 72}",
                   colour="purple", arrow=True)
            s.text((centers[i] + centers[target]) / 2, lane - 8, "inverse_of " + str(part["inverse_of"]),
                   size=14, anchor="middle", colour=PALETTE["purple"])
            lane += 58
    return lane + 20


@figure("07-seed7001-bulk-steps", "07-records.md", "The bulk counts and digest chain of the short scramble")
def seed7001_steps(e, s):
    run = e.json(DEV + "/d0-s7001/run/run.json")
    rows = run["steps"]
    states = [rows[0]["before"]] + [row["after"] for row in rows]
    for a, b in zip(rows, rows[1:]):
        if a["after"] != b["before"]:
            raise ValueError("bulk record has a broken digest chain")
    y = s.legend([("off", PALETTE["blue"]), ("N", PALETTE["teal"])], 92)
    y = plot(s, [{"label": key, "values": [state[key] for state in states], "colour": ACCENTS[i]}
                 for i, key in enumerate(("off", "N"))], list(map(str, range(len(states)))), y,
             "After each bulk step · step 0 is the start", height=290)
    y += 25
    positions = []
    for i, state in enumerate(states):
        row, col = divmod(i, 5)
        positions.append((40 + (col if row % 2 == 0 else 4 - col) * 230, y + row * 140))
    for a, b in zip(positions, positions[1:]):
        if a[1] == b[1]:
            direction = 1 if b[0] > a[0] else -1
            s.line(a[0] + (205 if direction == 1 else 0), a[1] + 38,
                   b[0] + (0 if direction == 1 else 205), b[1] + 38, arrow=True)
        else:
            s.line(a[0] + 102, a[1] + 76, b[0] + 102, b[1], arrow=True)
    for i, (state, (x, yy)) in enumerate(zip(states, positions)):
        group = s.el("g")
        ET.SubElement(group, "{" + SVG_NS + "}title").text = "state " + str(i) + ": " + state["digest"]
        before = len(s.root)
        s.box(x, yy, 205, 76, "state " + str(i) + "\n" + state["digest"][:8] + "…", size=16)
        for element in list(s.root)[before:]:
            s.root.remove(element)
            group.append(element)
    y += math.ceil(len(states) / 5) * 140
    return s.paragraph(40, y, "The arrows follow the recorded before/after chain. Digest prefixes are shown; full digests are retained in SVG titles. "
                       "Every step is a one-letter seam step. The run ends with the solved digest.")


@figure("appa-lock-shape", "A-macros.md", "lock: abstract opening and closing parts")
def appendix_lock(e, s):
    e.text(T + "/02-bulk.md")
    y = 90
    for n in (2, 3):
        names = ("x: open cap c", "y: close cap d", "z: close cap e")[:n]
        y = folded_tree(s, named_tree("lock (abstract)", [leaf(part, 1) for part in names]), y)
    return s.paragraph(40, y, "Abstract shapes, not played words. The runner would store one span of length 2 or 3. "
                       "No committed step supplies an example or an action region (chapter 2, section 2.8).")


@figure("appa-shift-shape", "A-macros.md", "shift: abstract opening and improving parts")
def appendix_shift(e, s):
    e.text(T + "/02-bulk.md")
    y = folded_tree(s, named_tree("shift (abstract)", [leaf("x: open cap c", 1), leaf("y: improve near c", 1)]), 90)
    return s.paragraph(40, y, "Abstract shape, not a played word. The runner would store one span of length 2. "
                       "No committed step supplies an example or an action region (chapter 2, section 2.10).")


@figure("appa-f2-shape", "A-macros.md", "F2: abstract three-letter lowering")
def appendix_f2(e, s):
    e.text(T + "/04-high.md")
    y = folded_tree(s, named_tree("r22d25/F2 (abstract)", [leaf(part, 1) for part in
                    ("x = (c, x)", "y = (d, y)", "z = (c, z)")]), 90)
    return s.paragraph(40, y, "Abstract shape only. No F2 step is committed; no before/after puzzle state is claimed "
                       "(chapter 4, section 4.8). The plugin would record one span of 3 letters.")


@figure("appa-f3-shape", "A-macros.md", "F3: abstract commutator, with both allowed lengths")
def appendix_f3(e, s):
    chapter = e.text(T + "/04-high.md")
    y = 90
    for k in map(int, re.search(r"k = (\d+) or (\d+)", chapter).groups()):
        w = named_tree("W", [leaf("w" + str(i + 1), 1) for i in range(k)])
        wi = named_tree("W⁻¹", [leaf("w" + str(i + 1) + "⁻¹", 1, inverse_of=[1, i])
                                for i in reversed(range(k))])
        wi["inverse_of"] = [1]
        root = named_tree("r22d25/F3 (abstract)", [leaf("a", 1), w, leaf("a⁻¹", 1, inverse_of=[0]), wi])
        y = folded_tree(s, root, y, "|W| = " + str(k))
    return s.paragraph(40, y, "Abstract shapes only. No F3 step is committed; the plugin would store one span of 4 or 6 letters "
                       "(chapter 4, section 4.9). Inverses reverse the order as well as invert each letter.")


@figure("appa-st-shape", "A-macros.md", "S·T: abstract protected lowering, then landing")
def appendix_st(e, s):
    e.text(T + "/04-high.md")
    y = folded_tree(s, named_tree("r22d25/ST (abstract)", [leaf("S: protected height 4 → 2", 1),
                                                        leaf("T: strict landing at Z·S", 1)]), 90)
    return s.paragraph(40, y, "Abstract shape only. In the committed catalogue S and T are single letters; no T provider "
                       "is registered. No S·T step is committed (chapter 4, section 4.11).")


def appendix_tree(e, s):
    data = e.figdata()
    for path, digest in data["inputs"].items():
        if sha256(e.raw(path)) != digest:
            raise ValueError("stale appendix figure data: " + e.name + ": " + path)
    tree_api().check_tree(data["tree"], data["tree"]["letters"])
    y = folded_tree(s, data["tree"], 90, show_spans=True)
    if "word" in data and len(data["word"]) <= 40:
        y = s.paragraph(40, y, "Word: " + " · ".join(str(tuple(letter)) for letter in data["word"])) + 16
    if "note" in data:
        y = s.paragraph(40, y, data["note"]) + 16
    return s.paragraph(40, y, "Lengths and zero-based, end-exclusive spans come from figdata.json. "
                       "The original runner tree helper is applied to the committed word or factors; no search is run.")


for _name, _title in (
        ("appa-u2-tree", "The fixed-target U2 experiment: two single twists"),
        ("appa-h4-pair-tree", "The recorded h4 freeing pair: two single twists"),
        ("appa-pure-tree", "The accepted b17 pure-cell replay"),
        ("appa-correction-tree", "Correction 0: macro 7, mirror2"),
        ("appa-late-cycle-tree", "The late program's first three-cycle: factors 0 and 1")):
    figure(_name, "A-macros.md", _title)(appendix_tree)


def validate_trees(document):
    if not isinstance(document, dict) or set(document) != {"format", "entries"} or document["format"] != "magic600-tutorial-trees/1":
        raise ValueError("unsupported trees.json format; expected format and entries")
    if not isinstance(document["entries"], list):
        raise ValueError("trees entries must be a list")
    entries = {}
    for entry in document["entries"]:
        if not isinstance(entry, dict) or set(entry) != {"name", "record", "step"}:
            raise ValueError("tree entry must contain name, record and step")
        name = entry["name"]
        if not isinstance(name, str) or not NAME_RE.fullmatch(name):
            raise ValueError("invalid tree name")
        if name in entries or name in FIGURES:
            raise ValueError("duplicate tree or figure name: " + name)
        if type(entry["step"]) is not int or entry["step"] < 0:
            raise ValueError("tree step must be a nonnegative integer")
        repository_path(entry["record"])
        if not entry["record"].startswith(RUNNER + "/runs/"):
            raise ValueError("tree record must be a runner run directory")
        entries[name] = entry
    return entries


def render(name, entries):
    if name in FIGURES:
        function, chapter, title = FIGURES[name]
        evidence = Evidence(name, chapter)
        svg = Svg(evidence, title)
        end = function(evidence, svg)
    else:
        entry = entries[name]
        evidence = Evidence(name, "A-macros.md" if name.startswith("appa-") else None)
        evidence.raw(TREES_PATH)
        row, root = record_tree(evidence, entry["record"], entry["step"])
        if not name.startswith("appa-"):
            evidence.number = "Figure A." + str(list(entries).index(name) + 1)
        title = row["rule"] + " · phase " + row["phase"] + " · step " + str(row["index"]) + " · " + count(row["letters"]) + " letters"
        evidence.alt = "Folded step tree of " + entry["record"] + ", " + title
        svg = Svg(evidence, title)
        end = folded_tree(svg, root, 90, show_spans=name.startswith("appa-"))
    data = svg.finish(end + 10)
    manifest = {"inputs": dict(sorted(evidence.inputs.items())),
                "output": {"path": name + ".svg", "sha256": sha256(data)}, "notes": evidence.notes,
                "production_renderer_sha256": production_inputs(name, evidence.inputs)[F + "/diagrams.py"]}
    return data, manifest


def generate(out, only=None):
    entries = validate_trees(json.loads(repository_path(TREES_PATH).read_text(encoding="utf-8")))
    names = set(FIGURES) | set(entries)
    selected = set(only) if only is not None else names
    if not selected <= names:
        raise ValueError("unknown figure(s): " + ", ".join(sorted(selected - names)))
    # Finish all reads/renders before creating outputs. Never write into inputs.
    rendered = {name: render(name, entries) for name in sorted(selected)}
    out = Path(out)
    old = {}
    target = out / "manifest.json"
    if only is not None and target.exists():
        previous = json.loads(target.read_text(encoding="utf-8"))
        if previous.get("format") != "magic600-tutorial-diagrams/1" or not isinstance(previous.get("figures"), dict):
            raise ValueError("existing diagram manifest has an unsupported format")
        old = previous["figures"]
        for name, entry in old.items():
            if not NAME_RE.fullmatch(name) or entry["output"]["path"] != name + ".svg":
                raise ValueError("invalid path in existing diagram manifest")
            for path in entry["inputs"]:
                repository_path(path)
    figures = dict(old)
    figures.update({name: manifest for name, (_, manifest) in rendered.items()})
    document = {"format": "magic600-tutorial-diagrams/1", "figures": figures}
    # A mistaken --out must not overwrite an evidence file or one of our sources.
    targets = [out / (name + ".svg") for name in selected] + [target]
    inputs = {repository_path(path).resolve() for _, manifest in rendered.values() for path in manifest["inputs"]}
    if any(path.resolve() in inputs for path in targets):
        raise ValueError("output would overwrite an input")
    out.mkdir(parents=True, exist_ok=True)
    for name, (data, _) in rendered.items():
        (out / (name + ".svg")).write_bytes(data)
    target.write_text(json.dumps(document, indent=2, sort_keys=True, ensure_ascii=False) + "\n", encoding="utf-8")
    return document


def check(out, only=None):
    """Regenerate without modifying out, checking provenance as well as SVG bytes."""
    out = Path(out)
    with tempfile.TemporaryDirectory(prefix="magic600-diagrams-check-") as directory:
        fresh = generate(Path(directory), only)
        try:
            saved = json.loads((out / "manifest.json").read_text(encoding="utf-8"))
        except (OSError, ValueError):
            saved = {}
        old = saved.get("figures", {})
        if not isinstance(old, dict):
            old = {}
        different = set()
        for name, entry in fresh["figures"].items():
            path = out / entry["output"]["path"]
            try:
                actual = sha256(path.read_bytes())
            except OSError:
                actual = None
            if saved.get("format") != fresh["format"] or old.get(name) != entry or actual != entry["output"]["sha256"]:
                different.add(name)
        if only is None:
            different.update(set(old) - set(fresh["figures"]))
        return sorted(different)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--out", type=Path, default=HERE / "svg")
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--only", nargs="+", metavar="NAME")
    args = parser.parse_args(argv)
    try:
        if args.check:
            different = check(args.out, args.only)
            if different:
                print("Diagrams differ: " + ", ".join(different))
                return 1
            print("Diagram manifest and SVG files match.")
        else:
            document = generate(args.out, args.only)
            print("Drew " + str(len(args.only) if args.only is not None else len(document["figures"])) + " diagrams.")
        return 0
    except (OSError, ValueError, KeyError, TypeError, StopIteration, IndexError) as error:
        print("Diagram error: " + str(error), file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
