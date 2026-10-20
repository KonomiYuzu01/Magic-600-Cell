"""Height-field and kernel contracts of runner-run/3 steps, checker side (headless research).

Stub until packet P4 is integrated: every height and high step is reported
``unchecked_contract``, which check_run.py counts separately and fails unless the
caller allows unchecked contracts. check_run.py calls, for each height or high step:

- ``prepare(row, st)`` before the step's first letter. ``st`` is the live simulator
  state before the step; the replay changes it, so copy what you need here.
- ``on_letter(ctx, k, st)`` after letter ``k`` (0-based) is applied, with the ``ctx``
  that ``prepare`` returned;
- ``finish(ctx, row, st)`` after the last letter; it returns the step's failures, a
  list of names. It is not called when a letter was not admissible.

Like check_run.py, this module shares only the definition of the puzzle: no runner,
rule, gate or player code (plan runner-run/3, section 0, checker independence).
"""

UNCHECKED = 'unchecked_contract'


def prepare(row, st):
    return None


def on_letter(ctx, k, st):
    return None


def finish(ctx, row, st):
    return [UNCHECKED]
