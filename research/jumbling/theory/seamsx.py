"""Exact cache reuse and forked trials for :mod:`seams`' bulk rules.

An apply saves the domains object and the entries that _touch invalidates. An
undo may restore them only for the matching journal record, grip and pre-apply
configuration digest. State._compact gives equal configurations equal pose
numberings, so the saved domains (including their pose-dependent memo tables),
inside sets, scores and balances equal fresh computations. Entries outside
NEAR[c], including ones first computed between apply(c, i) and its undo, remain
valid by the same locality invariant used by the parent's _touch: turning c
changes only caps that can meet c. Entries absent before apply are deleted on
restoration. Three snapshots suffice for the bulk rules' nested trials; older
committed applies age out. Undo beyond that bound uses the parent's invalidation.

Conj trials read the same survey and start and end at the same configuration.
Fork preserves that configuration and its caches without serialising geometry.
Consecutive chunks contain at most one trial per worker; results are considered
in their original order, regardless of completion order. The first success is
therefore the parent's first success. Only its three-letter word crosses back,
and the main process applies it itself, preserving journal and moves exactly.
Workers use pipes only and inherit the parent's process group (no setsid), so
the tutorial bulk child's parent-death group kill also kills its workers.
"""
from collections import deque
import multiprocessing as mp
from multiprocessing.connection import wait
import operator
import os
import traceback

import rim
import seams


def _worker(restorer, pipe):
    """Geometry and caches are inherited through fork; messages stay small."""
    try:
        while True:
            trial = pipe.recv()
            try:
                word = restorer._conj_trial(*trial)
            except BaseException as exc:
                pipe.send(('error', type(exc).__name__, str(exc)[:2048],
                           traceback.format_exc()[-8192:]))
                return
            pipe.send(('word', word))
    finally:
        pipe.close()


class SeamRestorer(seams.SeamRestorer):
    DOMAINS = rim.Domains
    _CACHE_DEPTH = 3

    def __init__(self, st, M, order=seams.SeamRestorer.ORDER, workers=None):
        if workers is None:
            try:
                workers = int(os.environ.get('JUMBLING_BULK_WORKERS', '1'))
            except ValueError as exc:
                raise ValueError('JUMBLING_BULK_WORKERS must be an integer >= 1') from exc
        try:
            count = operator.index(workers)
        except TypeError as exc:
            raise ValueError('workers must be an integer >= 1') from exc
        if isinstance(workers, bool) or count < 1:
            raise ValueError('workers must be an integer >= 1')
        super().__init__(st, M, order=order)
        self.workers = count
        self._undo_cache = deque(maxlen=self._CACHE_DEPTH)

    def domains(self):
        if self._dom is None:
            self._dom = self.DOMAINS(self.st)
        return self._dom

    def apply(self, c, i):
        near = [int(f) for f in seams.NEAR[int(c)]]
        saved = tuple({f: cache[f] for f in near if f in cache}
                      for cache in (self._cls, self._sv, self._bal))
        digest, domains = self.st.digest(), self._dom
        # Append only after a successful apply; a rejected/raising apply cannot
        # evict an older snapshot, even when the deque is full.
        super().apply(c, i)
        self._undo_cache.append((int(c), digest, self.st.journal[-1], domains, saved))

    def undo(self):
        record = self.st.journal[-1]
        c = record['grip']
        self.st.undo()
        top = self._undo_cache[-1] if self._undo_cache else None
        if (top is not None and top[0] == c and top[2] is record
                and top[1] == self.st.digest()):
            self._undo_cache.pop()
            self._dom = top[3]
            for cache, saved in zip((self._cls, self._sv, self._bal), top[4]):
                for f in seams.NEAR[int(c)]:
                    f = int(f)
                    if f in saved:
                        cache[f] = saved[f]
                    else:
                        cache.pop(f, None)
        else:
            # An external journal operation or an aged-out apply is not a
            # restoration witness. Discard the remaining witnesses as well.
            self._undo_cache.clear()
            self._touch(c)
        self.moves.pop()

    def _conj_trials(self, sv):
        mis = sorted(sv, key=lambda c: (-sv[c][0], c))
        mset = set(mis)
        trials = []
        for c in mis:
            nb = sorted(({int(x) for x in seams.NEAR[c]} & mset) - {c})
            if not nb:
                continue
            now_c, after_c = sv[c]
            # survey includes only classified, admissible caps. State.apply's
            # other possible rejection is menu membership (normally M itself).
            for i in range(self.n_items):
                g = self.g(c, i)
                if g == seams.I4:
                    continue
                if self.st.menu is not None and self.st.menu is not self.M:
                    twist = seams.sim.Twist(seams.ctx, c, g, 'menu', {'menu': self.M.name})
                    if not self.st.menu.contains(twist):
                        continue
                trials.append((c, i, nb, after_c[i] - now_c))
        return trials

    def _conj_trial(self, c, i, nb, d1):
        """One unchanged parent trial, unwound even when it finds a word."""
        first = len(self.moves)
        try:
            self.apply(c, i)
        except RuntimeError:
            return None
        try:
            for d in nb:
                r = self.scores(d)
                if r is None or r[0] == 0:
                    continue
                n_d, a_d = r
                j = min(a_d, key=lambda k: (a_d[k], k))
                if a_d[j] >= n_d:
                    continue
                self.apply(d, j)
                rc = self.scores(c)
                if rc is not None and rc[1]:
                    n_c2, a_c2 = rc
                    k = min(a_c2, key=lambda k: (a_c2[k], k))
                    if d1 + (a_d[j] - n_d) + (a_c2[k] - n_c2) < 0:
                        self.apply(c, k)
                        return [(c, i), (d, j), (c, k)]
                self.undo()
            return None
        finally:
            while len(self.moves) > first:
                self.undo()

    def _parallel_conj(self, trials):
        ctx = mp.get_context('fork')
        processes, pipes = [], []
        try:
            for _ in range(min(self.workers, len(trials))):
                parent, child = ctx.Pipe()
                pipes.append(parent)
                process = ctx.Process(target=_worker, args=(self, child), daemon=True)
                process.start()
                processes.append(process)
                child.close()
            width = len(processes)
            for start in range(0, len(trials), width):
                stop = min(start + width, len(trials))
                pending = {}
                for index in range(start, stop):
                    slot = index - start
                    pipes[slot].send(trials[index])
                    pending[pipes[slot]] = (processes[slot], index)
                results = {}
                while pending:
                    ready = wait(list(pending) + [p.sentinel for p, _ in pending.values()])
                    for pipe in list(pending):
                        if pipe not in ready:
                            continue
                        process, index = pending.pop(pipe)
                        try:
                            message = pipe.recv()
                        except EOFError as exc:
                            raise RuntimeError('conj worker exited without a result') from exc
                        if message[0] == 'error':
                            raise RuntimeError(f'conj worker {message[1]}: {message[2]}\n{message[3]}')
                        results[index] = message[1]
                    for process, _ in pending.values():
                        if process.sentinel in ready:
                            raise RuntimeError(f'conj worker exited without a result ({process.exitcode})')
                    for index in range(start, stop):
                        if index not in results:
                            break
                        if results[index] is not None:
                            return results[index]
            return None
        finally:
            # Terminate speculative trials as soon as the first ordered success
            # is known, and on errors. Join every child; leave no background work.
            for process in processes:
                if process.is_alive():
                    process.terminate()
            for process in processes:
                process.join(timeout=1)
                if process.is_alive():
                    process.kill()
                    process.join()
                process.close()
            for pipe in pipes:
                pipe.close()

    def conj(self):
        if self.workers == 1 or 'fork' not in mp.get_all_start_methods():
            return super().conj()
        sv = self.survey()
        trials = self._conj_trials(sv)
        if not trials:
            return None
        word = self._parallel_conj(trials)
        if word is not None:
            for c, i in word:
                self.apply(c, i)
        return word
