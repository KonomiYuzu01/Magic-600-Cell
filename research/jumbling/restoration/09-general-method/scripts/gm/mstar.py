"""The outer loop of the general method M* (SENIOR-REVIEW.md section 9, Theorem M*), apart from the driver so that its
control flow can be tested without the simulator (test_mstar.py).

At the current state: list the units; if no piece is off the lattice, stop for the lattice phase; otherwise try the
heuristic steps in order and take the first verified clean step; if none returns one, take the clean step of the
exhaustive fallback. A step budget, when given, is an experiment bound outside M*. It is checked after the off count,
so a run whose last permitted step reaches the lattice still goes on to the lattice phase.
Research only: headless Linux; no Windows, DirectX, input or performance claim.
"""


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
