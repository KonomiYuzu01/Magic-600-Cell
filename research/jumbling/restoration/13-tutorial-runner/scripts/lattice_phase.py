"""Literal lattice phase of the tutorial runner (headless research).

Stub until packet P3 is integrated. Its not_applicable keeps the verify_late
path, which checks the retained part by composing its program (R22D-57); the
runner labels such a run a component (lattice_composition).
"""
from pathlib import Path

import runner_api as api

STUB = True
SCRIPTS = [Path(__file__).resolve().relative_to(Path(__file__).resolve().parents[5])]


def play(ctx):
    return api.End('not_applicable', 'stub')
