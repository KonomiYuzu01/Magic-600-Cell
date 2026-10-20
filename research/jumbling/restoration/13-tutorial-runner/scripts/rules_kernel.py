"""High phase of the tutorial runner: the R22D-25 kernel with D19 (headless research).

Stub until packet P2 is integrated. The runner treats its not_applicable, when
the entry test holds, as no step and labels the run a component (high_stub).
"""
from pathlib import Path

import runner_api as api

STUB = True
SCRIPTS = [Path(__file__).resolve().relative_to(Path(__file__).resolve().parents[5])]


def next_step(ctx):
    return api.End('not_applicable', 'stub')
