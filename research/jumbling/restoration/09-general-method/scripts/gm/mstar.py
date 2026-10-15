"""The outer loop of the general method M* (SENIOR-REVIEW.md section 9, Theorem M*) and its exhaustive word search,
apart from the driver and the simulator so that both can be tested on synthetic states (test_mstar.py).

At the current state: list the units; if no piece is off the lattice, stop for the lattice phase; otherwise try the
heuristic steps in order and take the first verified clean step; if none returns one, take the clean step of the
exhaustive fallback. A step budget, when given, is an experiment bound outside M*. It is checked after the off count,
so a run whose last permitted step reaches the lattice still goes on to the lattice phase.
Research only: headless Linux; no Windows, DirectX, input or performance claim.
"""
import json
from pathlib import Path


def drive(table, units_of, heuristics, fallback, budget=0, log=None):
    """Run the off-lattice phase of M* from `table`.

    units_of(table, k) -> (info, units), where info['off'] is the number of off pieces.
    heuristics: functions (table, k, info, units) -> the table after a verified clean step, or None. Each must return
    (the driver bounds its subprocesses), so the fallback is reached whenever they all miss. A heuristic that raises
    an exception counts as a miss; an exception from the fallback ends the run.
    fallback(table, k, info, units) -> the table after the clean step of the exhaustive search, or None only when an
    experiment bound on the search length ran out.
    budget: 0 for M* (no step limit); n > 0 stops after n steps unless the lattice is reached.
    Returns (table, steps taken, reason) with reason 'lattice', 'budget' or 'fallback bound'.
    """
    log = log or (lambda **kw: None)
    k = 0
    while True:
        info, units = units_of(table, k)
        if info['off'] == 0:
            return table, k, 'lattice'
        if budget and k >= budget:
            log(event='stop', step=k, off=info['off'], result='step budget (outside M*)')
            return table, k, 'budget'
        nxt = None
        for h in heuristics:
            try:
                nxt = h(table, k, info, units)
            except Exception as e:  # a failed heuristic (a crash, a partial output after a kill) is a miss
                log(event='heuristic_error', step=k, heuristic=getattr(h, '__name__', repr(h)), error=repr(e)[:300])
                nxt = None
            if nxt is not None:
                break
        if nxt is None:
            nxt = fallback(table, k, info, units)
            if nxt is None:
                log(event='stop', step=k, off=info['off'], result='fallback length bound (outside M*)')
                return table, k, 'fallback bound'
        table = nxt
        k += 1


def closure_ok(d):
    """Whether the closure directory d holds a whole step record: word-full.json and pre.json parse, and end-table.json
    parses and carries the word's output digest. A process killed while it writes these files leaves a record that
    fails this, so the driver takes a heuristic step only with a whole record."""
    d = Path(d)
    try:
        w = json.loads((d / 'word-full.json').read_text())
        json.loads((d / 'pre.json').read_text())
        return json.loads((d / 'end-table.json').read_text())['digest'] == w['output_digest']
    except (OSError, ValueError, KeyError, TypeError):
        return False


def word_search(apply, undo, accept, caps, items, max_length=0, log=None):
    """The exhaustive search of M*: the words over the letters (c, i), cap c in range(caps) and item i in
    range(1, items) (item 0 is the identity and is left out), with no two consecutive letters at one cap, in order of
    length (1, 2, ...) and lexicographically within a length. Returns (the first word that accept() takes at its end
    state, letters tried), or (None, letters tried) when max_length > 0 runs out.

    apply(c, i) applies the letter and returns True, or returns False and changes nothing when cap c is blocked;
    admissibility depends on the cap only, so the other items at c are skipped. undo() reverts the last applied
    letter, and accept() judges the current end state. The traversal keeps its own stack, so the word length is
    bounded by memory only. After a length without an accepted word every applied letter has been undone.
    log(length, tried, found) is called after each length.
    """
    assert caps >= 1 and items >= 2

    def after(c, i, last):
        # the letter after (c, i) at a depth whose previous letter is at cap `last`; after(-1, items - 1, last) is
        # the first letter at that depth
        if i + 1 < items:
            return c, i + 1
        c += 1
        if c == last:
            c += 1
        return (c, 1) if c < caps else None

    tried = 0
    length = 0
    while not max_length or length < max_length:
        length += 1
        word = []
        cur = after(-1, items - 1, -1)
        found = False
        while True:
            last = word[-1][0] if word else -1
            if cur is None:  # this depth is exhausted: back up one letter
                if not word:
                    break
                c, i = word.pop()
                undo()
                cur = after(c, i, word[-1][0] if word else -1)
                continue
            c, i = cur
            tried += 1
            if not apply(c, i):
                cur = after(c, items - 1, last)
                continue
            word.append(cur)
            if len(word) < length:
                cur = after(-1, items - 1, c)
                continue
            if accept():
                found = True
                break
            word.pop()
            undo()
            cur = after(c, i, last)
        if log:
            log(length, tried, found)
        if found:
            return word, tried
    return None, tried
