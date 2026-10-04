"""The owner-editable seed plan: categories, search terms, fetch targets and proposal pools.

The default plan is seeds.default.yaml next to this file; the first use copies it
to <data folder>/seeds.yaml, which the owner edits. The owner can switch sources
off, change terms and targets, and add categories, but cannot add a source: only
the adapters in SOURCE_TIER passed the terms check.
"""
from __future__ import annotations

import re
import shutil
from dataclasses import dataclass
from pathlib import Path

from tastelab import common

DEFAULT = common.PACKAGE / "seeds.default.yaml"
FILE_NAME = "seeds.yaml"
KINDS = ("focus", "adjacency", "control")
# Approved sources and their tier; terms checked on 1 October 2026.
SOURCE_TIER = {"wikimedia": "A", "met": "A", "aic": "A", "nasa": "A", "openverse": "A",
               "archive": "B", "demozoo": "B", "safebooru": "B"}
NAME_RE = re.compile(r"^[a-z][a-z0-9_]{0,39}$")
TEXT_MAX = 80


class SeedError(ValueError):
    pass


@dataclass(frozen=True)
class Term:
    text: str
    queries: tuple[tuple[str, str], ...] = ()

    def query(self, source: str) -> str:
        """The search string for `source`: its override when the plan gives one, else the term itself."""
        return dict(self.queries).get(source, self.text)


@dataclass(frozen=True)
class Category:
    name: str
    kind: str
    target: int
    sources: tuple[str, ...]
    terms: tuple[Term, ...]
    adjacent: tuple[str, ...]
    tier: str | None = None     # "B" forces tier B on every image of the category


@dataclass(frozen=True)
class Seeds:
    categories: dict
    enabled: dict
    relevance_min: float

    def focus(self) -> list[str]:
        return [c.name for c in self.categories.values() if c.kind == "focus"]

    def active_sources(self, category: str) -> list[str]:
        return [s for s in self.categories[category].sources if self.enabled.get(s, False)]

    def tier_for(self, category: str, source: str) -> str:
        """Tier B wins: a category forcing B, or a tier B source, makes the image tier B."""
        forced = self.categories[category].tier if category in self.categories else None
        return "B" if "B" in (forced, SOURCE_TIER.get(source, "B")) else "A"


def _text(value, where: str) -> str:
    if not isinstance(value, str):
        raise SeedError(f"{where}: expected text")
    text = " ".join(value.split())
    if not text or len(text) > TEXT_MAX or any(ord(ch) < 32 or 127 <= ord(ch) <= 159 for ch in value):
        raise SeedError(f"{where}: text must be 1 to {TEXT_MAX} printable characters")
    return text


def _term(raw, where: str) -> Term:
    if isinstance(raw, str):
        return Term(_text(raw, where))
    if isinstance(raw, dict) and "text" in raw:
        overrides = []
        for key, value in raw.items():
            if key == "text":
                continue
            if key not in SOURCE_TIER:
                raise SeedError(f"{where}: {key!r} is not an approved source")
            overrides.append((key, _text(value, f"{where}.{key}")))
        return Term(_text(raw["text"], where), tuple(sorted(overrides)))
    raise SeedError(f"{where}: a term is text or a mapping with 'text'")


def parse(data) -> Seeds:
    if not isinstance(data, dict) or type(data.get("version")) is not int or data.get("version") != 1:
        raise SeedError("seeds: expected a mapping with version: 1")
    relevance = data.get("relevance_min", 0.0)
    if isinstance(relevance, bool) or not isinstance(relevance, (int, float)) or not 0.0 <= relevance <= 1.0:
        raise SeedError("relevance_min: a number from 0 to 1")
    enabled = {}
    raw_sources = data.get("sources", {})
    if not isinstance(raw_sources, dict):
        raise SeedError("sources: expected a mapping of source names to booleans")
    for name, on in raw_sources.items():
        if not isinstance(name, str) or name not in SOURCE_TIER:
            raise SeedError(f"sources: {name!r} is not an approved source")
        if not isinstance(on, bool):
            raise SeedError(f"sources.{name}: true or false")
        enabled[name] = on
    raw_categories = data.get("categories")
    if not isinstance(raw_categories, dict) or not raw_categories:
        raise SeedError("categories: at least one category")
    categories, total = {}, 0
    for name, raw in raw_categories.items():
        where = f"categories.{name}"
        if not isinstance(name, str) or not NAME_RE.fullmatch(name):
            raise SeedError(f"{where}: names are lowercase letters, digits and underscores")
        if not isinstance(raw, dict):
            raise SeedError(f"{where}: expected a mapping")
        kind = raw.get("kind")
        if kind not in KINDS:
            raise SeedError(f"{where}.kind: one of {', '.join(KINDS)}")
        target = raw.get("target", 0)
        if isinstance(target, bool) or not isinstance(target, int) or not 0 <= target <= common.MAX_IMAGES:
            raise SeedError(f"{where}.target: a whole number from 0 to {common.MAX_IMAGES}")
        sources = raw.get("sources", [])
        if not isinstance(sources, list):
            raise SeedError(f"{where}.sources: approved sources only ({', '.join(SOURCE_TIER)})")
        for i, source in enumerate(sources):
            if not isinstance(source, str) or source not in SOURCE_TIER:
                raise SeedError(f"{where}.sources[{i}]: expected an approved source name")
        terms = raw.get("terms", [])
        if not isinstance(terms, list) or not terms:
            raise SeedError(f"{where}.terms: at least one term")
        adjacent = raw.get("adjacent", [])
        if not isinstance(adjacent, list):
            raise SeedError(f"{where}.adjacent: a list of terms")
        tier = raw.get("tier")
        if tier not in (None, "B"):
            raise SeedError(f"{where}.tier: only B can be forced")
        categories[name] = Category(
            name, kind, target, tuple(dict.fromkeys(sources)),
            tuple(_term(t, f"{where}.terms[{i}]") for i, t in enumerate(terms)),
            tuple(_text(t, f"{where}.adjacent[{i}]") for i, t in enumerate(adjacent)), tier)
        total += target
    if total > common.MAX_IMAGES:
        raise SeedError(f"targets add up to {total}, above the cap of {common.MAX_IMAGES}")
    return Seeds(categories, enabled, float(relevance))


def ensure(data_root: Path, name: str = FILE_NAME, default: Path = DEFAULT) -> Path:
    """The owner's copy of a default file in the data folder, created when missing and never overwritten."""
    path = common.data_root(data_root) / name
    common.check_input(path)
    if not path.exists():
        path.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(default, path)
    return path


def load(path: Path) -> Seeds:
    return parse(common.load_yaml(path))


PROBES_DEFAULT = common.PACKAGE / "probes.default.yaml"
PROBES_FILE = "probes.yaml"


@dataclass(frozen=True)
class Probes:
    template: str
    axes: tuple             # (negative pole, positive pole) pairs
    terms: tuple

    def phrase(self, text: str) -> str:
        """The probe as the embedding model reads it."""
        return self.template.replace("{}", text)

    def texts(self) -> list[str]:
        """Every probe text, poles first, without repeats."""
        return list(dict.fromkeys([t for pair in self.axes for t in pair] + list(self.terms)))


def parse_probes(data) -> Probes:
    if not isinstance(data, dict) or data.get("version") != 1:
        raise SeedError("probes: expected a mapping with version: 1")
    template = data.get("template", "{}")
    if not isinstance(template, str) or template.count("{}") != 1 or len(template) > TEXT_MAX:
        raise SeedError("probes.template: text with exactly one {}")
    raw_axes = data.get("axes", [])
    if not isinstance(raw_axes, list):
        raise SeedError("probes.axes: expected a list")
    axes = []
    for i, pair in enumerate(raw_axes):
        if not isinstance(pair, list) or len(pair) != 2:
            raise SeedError(f"probes.axes[{i}]: a pair of opposite descriptions")
        axes.append((_text(pair[0], f"probes.axes[{i}]"), _text(pair[1], f"probes.axes[{i}]")))
    terms = data.get("terms", [])
    if not isinstance(terms, list):
        raise SeedError("probes.terms: a list of descriptions")
    return Probes(template, tuple(axes), tuple(_text(t, f"probes.terms[{i}]") for i, t in enumerate(terms)))


def load_probes(path: Path) -> Probes:
    return parse_probes(common.load_yaml(path))
