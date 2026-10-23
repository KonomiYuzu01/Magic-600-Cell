"""First whole-stop player, catalogue r22d25-cat/1 (not a coverage claim).

From the repository root:
  python3 -I -B <this file> run (--record FILE | --table FILE) --out DIR
      [--cache DIR] [--seconds S] [--d19 FILE] [--max-steps N]

Single-process, exact, headless research. All outputs and adapter scratch are
under the chosen output/cache directories; no simulator history guides search.
"""
import os
import sys
for _name in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ.setdefault(_name, '1')
sys.dont_write_bytecode = True

import argparse
import hashlib
import json
import math
import subprocess
import tempfile
import time
from dataclasses import dataclass
from functools import lru_cache
from itertools import product
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import r22d25_state as K
import numpy as np

CATALOGUE = 'r22d25-cat/1'
DEFAULT_LIMITS = {'direct': 244, 'S': 244, 'T': 720, 'ST': 964}
DEFAULT_D19 = HERE / 'r22d19_rule.py'
FAMILIES = ('F1', 'F2', 'F3', 'd19', 'S', 'ST', 'T1')


@dataclass
class Limits:
    direct: int = 244
    S: int = 244
    T: int = 720
    ST: int = 964
    d19: object = DEFAULT_D19
    cache: object = None

    def record(self):
        return {k: getattr(self, k) for k in DEFAULT_LIMITS}

    def validate(self):
        for k, maximum in DEFAULT_LIMITS.items():
            if type(getattr(self, k)) is not int or not 1 <= getattr(self, k) <= maximum:
                raise ValueError('invalid_limits')


class BudgetStop(Exception):
    def __init__(self, family, cursor):
        self.family, self.cursor = family, cursor


class Budget:
    def __init__(self, seconds=300):
        if not math.isfinite(seconds) or seconds < 0:
            raise ValueError('invalid_seconds')
        self.deadline = time.monotonic() + seconds

    def remaining(self):
        return max(0., self.deadline - time.monotonic())

    def check(self, family, cursor):
        if self.remaining() <= 0:
            raise BudgetStop(family, cursor)


def _limits(limits):
    result = Limits(**limits) if isinstance(limits, dict) else limits
    result.validate()
    return result


def ledger(providers=()):
    names = list(FAMILIES)
    for p in providers:
        name = getattr(p, '__name__', '')
        if not name or not name.replace('_', '').isalnum() or name in names:
            raise ValueError('invalid_provider_name')
        names.append(name)
    return {f: {'status': 'not_applicable', 'tried': 0, 'pruned': {}, 'cursor': None} for f in names}


def _prune(row, reason, count=1):
    row['pruned'][reason] = row['pruned'].get(reason, 0) + count


def _active(row, cursor):
    row['status'], row['cursor'] = 'budget', cursor


def _done(row):
    row['status'], row['cursor'] = 'exhausted', None


def _candidate(s, u, word, family, gate, params=None, high_exact=False):
    return {'status': 'candidate', 'family': family, 'kind': gate['kind'],
            'target': list(u['piece_ids']), 'params': dict(params or {}),
            'high_exact': bool(high_exact is True),
            'word': {'word': K.literal_word(word), 'input_digest': s.digest,
                     'output_digest': gate['output_digest']}, 'gate': gate}


def t_family(s, u, high_exact, limits, budget, providers, rows):
    """T1, then caller-supplied generators with explicit terminal status."""
    row = rows['T1']
    U = u['piece_ids']
    if not U or not (s.heights[U] == 2).all():
        raise ValueError('T_target_height')
    branches = {K.pose_info(s.state._pose_matrix(pid))[1]
                for pid in np.unique(s.state.pose_id[U]).tolist()}
    if len(branches) != 1:
        raise ValueError('T_target_branch')
    # In step 4 the piece set comes from Z, but T is searched at Y = Z.S.
    # Refresh target metadata too; providers must not read Z's cached height,
    # branch or inside vector as a fact about their current snapshot.
    u = dict(u, height=2, branch=next(iter(branches)),
             caps_whole=np.flatnonzero((s.labels[U] == 1).all(axis=0)).tolist())
    for c in range(600):
        cursor = {'unit': u['first'], 'grip': c, 'item': 1}
        _active(row, cursor)
        budget.check('T1', cursor)
        # Cap-level necessary tests account for all 59 letters, in one range.
        reason = ('blocked' if c not in s.clean else
                  'target_not_whole' if not (s.labels[U, c] == 1).all() else
                  'lattice_inside' if s.lattice_inside[c] else None)
        if reason:
            _prune(row, reason, 59)
            continue
        for j in range(1, 60):
            cursor = {'unit': u['first'], 'grip': c, 'item': j}
            row['cursor'] = cursor
            budget.check('T1', cursor)
            reason = K.single_letter_prune(s, U, c, j, high_exact=high_exact)
            if reason:
                _prune(row, reason)
                continue
            row['tried'] += 1
            gate = K.strict_step(s, U, [[c, j]], high_exact)
            if gate['ok']:
                return _candidate(s, u, [[c, j]], 'T1', gate, high_exact=high_exact)
            if gate['reason'].startswith('uncertain'):
                raise ValueError('uncertain_prefix')
            _prune(row, gate['reason'])
    _done(row)
    for provider in providers:
        name = provider.__name__
        row = rows[name]
        _active(row, {'unit': u['first'], 'proposal': 0})
        budget.check(name, row['cursor'])
        gen = iter(provider(s, u, high_exact, limits, budget))
        proposal = 0
        while True:
            row['cursor'] = {'unit': u['first'], 'proposal': proposal}
            budget.check(name, row['cursor'])
            try:
                word = next(gen)
            except StopIteration as stop:
                if stop.value == 'budget':
                    raise BudgetStop(name, row['cursor'])
                if stop.value != 'exhausted':
                    raise ValueError('provider_missing_terminal_status')
                if s.state.digest() != s.digest:
                    raise ValueError('provider_changed_state')
                _done(row)
                break
            proposal += 1
            row['tried'] += 1
            w = K.literal_word(word)
            if len(w) > limits.T:
                _prune(row, 'length')
                continue
            gate = K.strict_step(s, U, word, high_exact)
            if gate['ok']:
                return _candidate(s, u, word, name, gate, {'proposal': proposal - 1}, high_exact)
            if gate['reason'].startswith('uncertain'):
                raise ValueError('uncertain_prefix')
            _prune(row, gate['reason'])
    return None


def tier(s, limits, budget, rows):
    """R22D-37, by unit, then F1/F2/F3, with exact prefix exclusions."""
    units = [u for u in s.high_units if u['height'] == 6]
    for number, u in enumerate(units):
        U = u['piece_ids']
        row = rows['F1']
        for c in range(600):
            cursor = {'unit': u['first'], 'grip': c, 'item': 1}
            _active(row, cursor)
            budget.check('F1', cursor)
            reason = ('blocked' if c not in s.clean else
                      'target_not_whole' if not (s.labels[U, c] == 1).all() else
                      'lattice_inside' if s.lattice_inside[c] else None)
            if reason:
                _prune(row, reason, 59)
                continue
            for j in range(1, 60):
                cursor = {'unit': u['first'], 'grip': c, 'item': j}
                row['cursor'] = cursor
                budget.check('F1', cursor)
                reason = K.single_letter_prune(s, U, c, j, lowering=True)
                if reason:
                    _prune(row, reason)
                    continue
                row['tried'] += 1
                gate = K.protected_lowering(s, U, [[c, j]], 6)
                if gate['ok']:
                    return _candidate(s, u, [[c, j]], 'F1', gate, {'height': 6})
                if gate['reason'].startswith('uncertain'):
                    raise ValueError('uncertain_prefix')
                _prune(row, gate['reason'])
        if number == len(units) - 1:
            _done(row)
        else:
            _active(row, {'next_unit': units[number + 1]['first']})
        row = rows['F2']
        for c in range(600):
            cursor = {'unit': u['first'], 'c': c, 'x': 1, 'd': 0, 'y': 1, 'z': 1}
            _active(row, cursor)
            budget.check('F2', cursor)
            if c not in s.clean:
                _prune(row, 'blocked_first', 59 * 599 * 59 * 59)
                continue
            for x in range(1, 60):
                first = K.play(s.state, [[c, x]])
                for d in range(600):
                    if d == c:
                        continue
                    cursor = {'unit': u['first'], 'c': c, 'x': x, 'd': d, 'y': 1, 'z': 1}
                    row['cursor'] = cursor
                    budget.check('F2', cursor)
                    cl = first.classify(d)
                    if cl.uncertain:
                        raise ValueError('uncertain_prefix')
                    if cl.straddle is not None:
                        _prune(row, 'blocked_second', 59 * 59)
                        continue
                    for y in range(1, 60):
                        middle = K.play(first, [[d, y]])
                        cl = middle.classify(c)
                        if cl.uncertain:
                            raise ValueError('uncertain_prefix')
                        if cl.straddle is not None:
                            _prune(row, 'blocked_third', 59)
                            continue
                        for z in range(1, 60):
                            cursor = {'unit': u['first'], 'c': c, 'x': x, 'd': d, 'y': y, 'z': z}
                            row['cursor'] = cursor
                            budget.check('F2', cursor)
                            word = [[c, x], [d, y], [c, z]]
                            row['tried'] += 1
                            gate = K.protected_lowering(s, U, word, 6)
                            if gate['ok']:
                                return _candidate(s, u, word, 'F2', gate, {'height': 6})
                            if gate['reason'].startswith('uncertain'):
                                raise ValueError('uncertain_prefix')
                            _prune(row, gate['reason'])
        if number == len(units) - 1:
            _done(row)
        else:
            _active(row, {'next_unit': units[number + 1]['first']})
        row = rows['F3']
        inv = inverse_items()
        for c in sorted(u['caps_whole']):
            if c not in s.clean or s.cap_branches[c] != u['branch']:
                _prune(row, 'not_clean_own_branch', 59)
                continue
            for j in range(1, 60):
                cursor = {'unit': u['first'], 'a': [c, j], 'W_length': 1, 'W': []}
                _active(row, cursor)
                budget.check('F3', cursor)
                q = K.moved_pose(s.state, s.state.pose_id[U[0]], c, j)
                if s.state.ctx.kplus.index_of_matrix(q) >= 0 or K.pose_info(q)[0] != 4:
                    _prune(row, 'a_not_whole_unit_lowering')
                    continue
                # All U have one exact pose and all are inside c, not just U[0].
                reached = K.pose_info(q)[1]
                alphabet = [(d, y) for d in range(600) if s.cap_branches[d] == reached for y in range(1, 60)]
                for length in (1, 2):
                    for W in product(alphabet, repeat=length):
                        cursor = {'unit': u['first'], 'a': [c, j], 'W_length': length,
                                  'W': [list(a) for a in W]}
                        row['cursor'] = cursor
                        budget.check('F3', cursor)
                        word = [[c, j]] + [list(a) for a in W] + [[c, inv[j]]] + [[d, inv[y]] for d, y in reversed(W)]
                        row['tried'] += 1
                        gate = K.protected_lowering(s, U, word, 6)
                        if gate['ok']:
                            return _candidate(s, u, word, 'F3', gate, dict(cursor, reached_branch=reached))
                        if gate['reason'].startswith('uncertain'):
                            raise ValueError('uncertain_prefix')
                        _prune(row, gate['reason'])
        if number == len(units) - 1:
            _done(row)
        else:
            _active(row, {'next_unit': units[number + 1]['first']})
    return None


def inverse_items():
    return _inverse_items()


@lru_cache(maxsize=1)
def _inverse_items():
    M = K.E.R.menu('I_a')
    matrices = [K.to_tuple(m) for _, m in M.items]
    return tuple(next(j for j, b in enumerate(matrices)
                      if K.to_tuple(K.matmul(a, b)) == matrices[0]) for a in matrices)


def direct(s, limits, budget, rows):
    row = rows['d19']
    cursor = {'adapter': 'd19', 'state': s.digest}
    _active(row, cursor)
    budget.check('d19', cursor)
    path = Path(limits.d19) if limits.d19 is not None else DEFAULT_D19
    if not path.is_file():
        row['status'] = 'unsupported'
        return {'status': 'unsupported', 'reason': 'd19_missing'}
    # The adapter is needed only for height-4 targets. Height 6 remains exact.
    high_exact = np.flatnonzero(s.heights == 6).tolist()
    cache = Path(limits.cache or os.environ.get('JUMBLING_HOME_CACHE', tempfile.gettempdir())).resolve()
    if cache.suffix == '.npy':
        cache = cache.parent
    cache.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='r22d25-d19-', dir=cache) as tmp:
        temp = Path(tmp)
        table, out = temp / 'table.json', temp / 'out'
        out.mkdir()
        K.B.export_pose_table(s.state, table)
        env = dict(os.environ, JUMBLING_HOME_CACHE=str(cache / 'home_centroids.npy'))
        for name in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
            env[name] = '1'
        try:
            child = subprocess.run([sys.executable, '-I', '-B', str(path.resolve()), '--table', str(table),
                                    '--out', str(out), '--cache', str(cache), '--seconds', str(budget.remaining())],
                                   env=env, cwd=K.ROOT, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                                   timeout=max(.001, budget.remaining()), check=False)
        except subprocess.TimeoutExpired:
            raise BudgetStop('d19', cursor)
        if child.returncode:
            return {'status': 'error', 'reason': 'd19_exit'}
        try:
            report = json.loads((out / 'report.json').read_text())
            status = report['result']
            if status == 'budget':
                raise BudgetStop('d19', cursor)
            if status == 'stuck':
                _done(row)
                return None
            if status != 'step':
                raise ValueError('d19_result')
            word = json.loads((out / 'word.json').read_text())
            if set(word) != {'word', 'input_digest', 'output_digest'}:
                raise ValueError('d19_word_format')
            first = report['step']['unit']
            u = next((u for u in s.high_units if u['first'] == first and u['height'] == 4), None)
            row['tried'] += 1
            if u is None:
                _prune(row, 'invalid_unit')
                raise ValueError('d19_invalid_unit')
            if len(K.literal_word(word)) > limits.direct:
                _prune(row, 'length')
                raise ValueError('d19_length')
            gate = K.strict_step(s, u['piece_ids'], word, high_exact)
            if not gate['ok']:
                _prune(row, gate['reason'])
                return {'status': 'error', 'reason': 'd19_rejected'}
            params = {'unit': first, 'high_exact_pieces': high_exact}
            return _candidate(s, u, word, 'd19', gate, params)
        except (OSError, ValueError, KeyError, TypeError) as exc:
            return {'status': 'error', 'reason': 'd19_invalid_output'}


def search_step(s, limits, budget, providers=()):
    """Return a gated candidate, or a precise non-candidate terminal outcome."""
    providers = tuple(providers)
    rows = ledger(providers)
    try:
        limits = _limits(limits)
        if s.state.digest() != s.digest:
            raise ValueError('stale_snapshot')
        if s.largest_height > 6:
            rows['F1']['status'] = 'unsupported'
            return {'status': 'unsupported', 'reason': 'height_above_6', 'heights': s.heights_census, 'ledger': rows}
        if not s.off_count:
            return {'status': 'exhausted', 'ledger': rows}
        applicable = []
        if s.largest_height == 6:
            applicable.extend(('F1', 'F2', 'F3'))
        if any(u['height'] == 4 for u in s.high_units):
            applicable.extend(('d19', 'S', 'ST'))
        if s.high_units or s.height2_units:
            applicable.extend(('T1', *(p.__name__ for p in providers)))
        for name in applicable:
            _active(rows[name], {'phase': 'not_reached', 'state': s.digest})
        candidate = tier(s, limits, budget, rows)
        if candidate:
            return dict(candidate, ledger=rows)
        h4 = [u for u in s.high_units if u['height'] == 4]
        h6 = np.flatnonzero(s.heights == 6).tolist()
        if h4:
            candidate = direct(s, limits, budget, rows)
            if candidate:
                return dict(candidate, ledger=rows)
        # Step 4: the practical S family is exactly the protected single letters.
        for u in h4:
            for c in range(600):
                cursor = {'unit': u['first'], 'grip': c, 'item': 1}
                _active(rows['S'], cursor)
                budget.check('S', cursor)
                reason = ('blocked' if c not in s.clean else
                          'target_not_whole' if not (s.labels[u['piece_ids'], c] == 1).all() else
                          'lattice_inside' if s.lattice_inside[c] else None)
                if reason:
                    _prune(rows['S'], reason, 59)
                    continue
                for j in range(1, 60):
                    cursor = {'unit': u['first'], 'grip': c, 'item': j}
                    rows['S']['cursor'] = cursor
                    budget.check('S', cursor)
                    reason = K.single_letter_prune(s, u['piece_ids'], c, j, lowering=True, high_exact=h6)
                    if reason:
                        _prune(rows['S'], reason)
                        continue
                    rows['S']['tried'] += 1
                    gS = K.protected_lowering(s, u['piece_ids'], [[c, j]], 4, h6)
                    if not gS['ok']:
                        if gS['reason'].startswith('uncertain'):
                            raise ValueError('uncertain_prefix')
                        _prune(rows['S'], gS['reason'])
                        continue
                    # S is a lowering to height 2, not an immediate landing.
                    if gS['kind'] != 'lowering':
                        _prune(rows['S'], 'S_not_height_2')
                        continue
                    middle = K.snapshot(gS['state'])
                    _active(rows['ST'], dict(cursor, middle_digest=middle.digest))
                    result = t_family(middle, u, h6, limits, budget, providers, rows)
                    if result is None:
                        _prune(rows['ST'], 'T_exhausted')
                        for name in ('T1', *(p.__name__ for p in providers)):
                            _active(rows[name], {'phase': 'remaining_targets', 'S': [c, j], 'unit': u['first']})
                        continue
                    word = [[c, j]] + result['word']['word']
                    rows['ST']['tried'] += 1
                    if len(word) > limits.ST:
                        _prune(rows['ST'], 'length')
                        continue
                    gate = K.strict_step(s, u['piece_ids'], word, h6)
                    if gate['ok']:
                        return dict(_candidate(s, u, word, 'ST', gate,
                                    {'split': 1, 'S_length': 1, 'T_length': len(word) - 1,
                                     'middle_digest': middle.digest, 'T_family': result['family'],
                                     'high_exact_pieces': h6}), ledger=rows)
                    if gate['reason'].startswith('uncertain'):
                        raise ValueError('uncertain_prefix')
                    # Both component gates entail the whole gate; a failure is
                    # a kernel error, not an exclusion or proof of exhaustion.
                    raise ValueError('ST_composition_gate')
        if h4:
            _done(rows['S'])
            _done(rows['ST'])
        # Step 5 is attempted only after every preceding family is exhausted.
        _, units = K.blocker_order(s)
        for number, u in enumerate(units):
            result = t_family(s, u, True, limits, budget, providers, rows)
            if result:
                result['params'].update(blocker_cap=u['blocker_cap'], high_exact_pieces=np.flatnonzero(s.heights >= 4).tolist())
                return dict(result, ledger=rows)
            if number < len(units) - 1:
                for name in ('T1', *(p.__name__ for p in providers)):
                    _active(rows[name], {'next_unit': units[number + 1]['first']})
        # Empty universes are exhausted too; thus a stuck record has no unfinished
        # or unsupported family, including any registered external providers.
        for row in rows.values():
            _done(row)
        return {'status': 'exhausted', 'ledger': rows}
    except BudgetStop as stop:
        _active(rows[stop.family], stop.cursor)
        return {'status': 'budget', 'ledger': rows}
    except Exception as exc:
        # Exception strings from subprocesses/paths never enter public records.
        return {'status': 'error', 'reason': type(exc).__name__, 'ledger': rows}


def load_start(kind, path):
    M = K.E.R.menu('I_a')
    state = K.E.sim.State(K.E.sim.get_context(), filtered=False, menu=M)
    doc = json.loads(Path(path).read_text())
    if kind == 'table':
        K.orient.set_pose_table(state, doc)
    else:
        for c, j in doc['history'] + doc['moves']:
            o = state.apply(K.E.sim.Twist(state.ctx, c, K.element(c, j), 'menu', {'menu': M.name}), journal=False)
            if not o.applied or o.uncertain:
                raise ValueError('invalid_record_prefix')
    # A digest alone cannot detect missing identity pieces.
    if len(state.pose_id) != len(state.ctx.data.sigbits):
        raise ValueError('full_model')
    state.journal = []
    return state


def atomic_json(path, doc):
    path = Path(path)
    temporary = path.with_suffix(path.suffix + '.tmp')
    temporary.write_text(json.dumps(doc, indent=2, allow_nan=False) + '\n')
    os.replace(temporary, path)


def state_record(s):
    return {'digest': s.digest, 'off': s.off_count, 'heights': s.heights_census}


def script_digests():
    return {p.relative_to(K.ROOT).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in (HERE / 'r22d25_state.py', HERE / 'r22d25_search.py', K.ROOT / 'assets/manifest.json')}


def run(state, start, out, limits, budget, max_steps=None, providers=()):
    """Whole-stop loop; the accepted word is independently replayed once more."""
    started = time.monotonic()
    out = Path(out)
    out.mkdir(parents=True, exist_ok=True)
    (out / 'words').mkdir(exist_ok=True)
    s = K.snapshot(state)
    doc = {'format': 'r22d25-run/1', 'catalogue': CATALOGUE, 'limits': limits.record(),
           'start': dict(start, **state_record(s)), 'steps': [], 'result': 'budget',
           'ledger': ledger(providers), 'last': state_record(s), 'scripts': script_digests(), 'seconds': 0.}
    if s.off_count:
        _active(doc['ledger']['F1' if s.largest_height >= 6 else 'd19' if s.high_units else 'T1'],
                {'step': 0, 'phase': 'start'})
    initial_n0 = s.off_count
    initial_n6 = int((s.heights == 6).sum())
    played = 0

    def save():
        doc['seconds'] = time.monotonic() - started
        atomic_json(out / 'run.json', doc)

    save()
    while True:
        if not s.off_count:
            doc['result'], doc['ledger'] = 'lattice', ledger(providers)
            break
        if max_steps is not None and len(doc['steps']) >= max_steps:
            doc['result'], doc['ledger'] = 'budget', ledger(providers)
            family = 'F1' if s.largest_height == 6 else 'd19' if s.high_units else 'T1'
            _active(doc['ledger'][family], {'max_steps': max_steps, 'step': len(doc['steps'])})
            break
        step_time = time.monotonic()
        result = search_step(s, limits, budget, providers)
        doc['ledger'] = result['ledger']
        if result['status'] != 'candidate':
            doc['result'] = 'stuck' if result['status'] == 'exhausted' else result['status']
            if 'reason' in result:
                doc['reason'] = result['reason']
            break
        try:
            before_digest = s.digest
            end = K.play(s.state, result['word'])
            if s.state.digest() != before_digest or end.digest() != result['word']['output_digest']:
                raise ValueError('replay_digest')
            refreshed = K.refresh_after_accept(end, result['word']['output_digest'])
            index = len(doc['steps'])
            word_file = f'words/{index:05d}.json'
            row = {k: result[k] for k in ('family', 'kind', 'target', 'params', 'high_exact')}
            row.update(index=index, word=word_file, letters=len(result['word']['word']),
                       input_digest=s.digest, output_digest=refreshed.digest, off_before=s.off_count,
                       off_after=refreshed.off_count, heights_after=refreshed.heights_census,
                       seconds=time.monotonic() - step_time)
            played += row['letters']
            if played > 6 * initial_n6 + DEFAULT_LIMITS['ST'] * initial_n0:
                raise ValueError('termination_bound')
            if row['kind'] == 'landing' and refreshed.off_count >= s.off_count:
                raise ValueError('landing_progress')
            if row['kind'] == 'lowering' and int((refreshed.heights == 6).sum()) >= int((s.heights == 6).sum()):
                raise ValueError('lowering_progress')
            atomic_json(out / word_file, result['word'])
            doc['steps'].append(row)
            s = refreshed
            doc['last'] = state_record(s)
            # The previous state's coverage ledger must never certify this one.
            doc['ledger'] = ledger(providers)
            if s.off_count:
                _active(doc['ledger']['F1' if s.largest_height >= 6 else 'd19' if s.high_units else 'T1'],
                        {'step': len(doc['steps']), 'phase': 'refresh'})
            save()
        except Exception:
            doc['result'], doc['reason'] = 'error', 'accepted_replay_failed'
            break
    save()
    return doc


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__)
    sub = ap.add_subparsers(dest='command', required=True)
    parser = sub.add_parser('run')
    start = parser.add_mutually_exclusive_group(required=True)
    start.add_argument('--record', type=Path)
    start.add_argument('--table', type=Path)
    parser.add_argument('--out', required=True, type=Path)
    parser.add_argument('--cache', type=Path)
    parser.add_argument('--seconds', type=float, default=300.)
    parser.add_argument('--d19', type=Path, default=DEFAULT_D19)
    parser.add_argument('--max-steps', type=int)
    args = ap.parse_args(argv)
    if args.max_steps is not None and args.max_steps < 0:
        ap.error('--max-steps must be nonnegative')
    if not math.isfinite(args.seconds) or args.seconds < 0:
        ap.error('--seconds must be finite and nonnegative')
    out = args.out
    out.mkdir(parents=True, exist_ok=True)
    cache = args.cache or out / 'cache'
    K.set_cache(cache)
    kind, source = ('record', args.record) if args.record else ('table', args.table)
    state = load_start(kind, source)
    # A self-contained capture; records and tables contain no private source path.
    filename = 'start-record.json' if kind == 'record' else 'start-table.json'
    if kind == 'record':
        original = json.loads(source.read_text())
        atomic_json(out / filename, {k: original[k] for k in ('history', 'moves')})
    else:
        K.B.export_pose_table(state, out / filename)
    limits = Limits(d19=args.d19, cache=cache)
    doc = run(state, {'kind': kind, 'file': filename}, out, limits, Budget(args.seconds), args.max_steps)
    print(json.dumps({'result': doc['result'], 'steps': len(doc['steps']), 'last': doc['last'], 'ledger': doc['ledger']}), flush=True)
    return int(doc['result'] == 'error')


if __name__ == '__main__':
    sys.exit(main())
