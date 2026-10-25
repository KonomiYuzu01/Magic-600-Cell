"""Chronological certificates for exact I_a macros (headless research only).

Run from the repository root. Inputs are immutable; all output/cache paths must
be outside the repository. No change to the meaning of t3p-certificate/1.
"""
import argparse
from contextlib import contextmanager
from functools import lru_cache
import hashlib
import json
import os
from pathlib import Path
import sys
import time

sys.dont_write_bytecode = True
for _var in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ[_var] = '1'
S14 = Path(__file__).resolve().parents[2] / '14-blocking-structure' / 'scripts'
sys.path.insert(0, str(S14))
import t3p as P  # noqa: E402
import numpy as np  # noqa: E402
from exact import Q5, matmul, transpose  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

BM, J, T, C, CLS = P.BM, P.J, P.T, P.C, P.CLS
FORMAT = 'macro-certificate/1'


class Failure(ValueError):
    def __init__(self, test, detail='', event=None, macro=None):
        self.test, self.detail, self.event, self.macro = test, detail, event, macro
        super().__init__(f'{test}: {detail}')

    def record(self):
        return dict(test=self.test, detail=self.detail, event=self.event, macro=self.macro)


def require(ok, test, detail=''):
    if not ok:
        raise Failure(test, detail)


def word(raw):
    if not isinstance(raw, list):
        raise Failure('events', 'expected a list of [grip, item] pairs')
    result = []
    for pair in raw:
        if not isinstance(pair, (list, tuple)) or len(pair) != 2:
            raise Failure('events', 'expected [grip, item]')
        result.append((P.grip(pair[0]), P.item(pair[1])))
    return result


def allowance(e):
    require(type(e) is int and e >= 0 and e % 2 == 0,
            'excursion', 'allowance must be an even nonnegative integer')
    return e


def support_claim(pieces):
    return dict(size=len(pieces), sha256=hashlib.sha256(
        np.sort(pieces).astype('<i8').tobytes()).hexdigest())


@lru_cache(maxsize=7200)
def pole_action(k):
    """Exact matrix-vector products followed by lookup in the model pole table.

    Object integers prevent overflow. N=(N1+sqrt(5)N2)/2; the matrix has
    denominator d. This deliberately does not assume KP.perms is the answer.
    """
    g1, g2, d = BM.KP.int_matrix(k)
    a, b = C.data.N1.astype(object), C.data.N2.astype(object)
    g1, g2 = g1.astype(object), g2.astype(object)
    u, v = a @ g1.T + 5 * b @ g2.T, b @ g1.T + a @ g2.T
    forward = np.array([C.data.POLE[tuple(Q5(int(aa), int(bb), 2 * d)
                                           for aa, bb in zip(ur, vr))]
                        for ur, vr in zip(u, v)], np.int64)
    require(np.array_equal(np.sort(forward), np.arange(600)), 'pole permutation')
    pre = np.argsort(forward)
    sigma = np.full(6, -1, np.int8)  # the last entry preserves branch -1
    for q in range(5):
        images = np.unique(CLS[forward[CLS == q]])
        require(len(images) == 1, 'class permutation')
        sigma[q] = images[0]
    return pre, forward, sigma


@lru_cache(maxsize=6600)
def retained(c, i):
    require(0 <= i < 12, 'retained item')
    k = BM.KP.index_of_matrix(P.matrix(c, i))
    require(k >= 0, 'retained matrix')
    return k, *pole_action(k)


@lru_cache(maxsize=36000)
def inverse_item(c, i):
    inv = to_tuple(transpose(P.matrix(c, i)))
    return next(j for j in range(60) if P.matrix(c, j) == inv)


def column_read(state, c, i, x, items=P.JUMBLES):
    """S,F,D after a retained letter, without updating the label table.

    S and F use the OLD preimage column and preimage class. D uses exact
    new poses of the moved pieces, not their labels/heights/branches alone.
    The formal image is undone even on failure.
    """
    _, pre, _, sigma = retained(c, i)
    moved = state.inside(c)
    keep = np.ones(len(state.h), bool)
    keep[moved] = False
    old = state.lab[:, x]
    new = state.lab[moved, pre[x]]
    s = int((old[keep] == 2).sum() + (new == 2).sum())
    f = int(((old == 1) & keep & (state.h >= 2) & (state.b != CLS[x])).sum())
    f += int(((new == 1) & (state.h[moved] >= 2)
              & (state.b[moved] != CLS[pre[x]])).sum())
    remaining = np.flatnonzero(keep & (old == 1))
    ds = state.oracle.counts(state.st, state.h, state.b, remaining, x, items)
    digest = state.st.digest()
    try:
        with P.formal_image(state.st, c, i, moved):
            ds += state.oracle.local_counts(state.st, moved, state.h[moved],
                                            sigma[state.b[moved]], new, x, items)
    finally:
        require(state.st.digest() == digest, 'undo', 'column reading')
    return s, f, ds


def apply_checked(state, c, i):
    """Commit one event and verify its complete support and retained calculus.

    Returns an undo token compatible with t3p.State.undo. On a failed check
    after simulator application, roll back before raising.
    """
    require(not (state.lab[:, c] == 2).any(), 'admissibility', f'blocked grip {c}')
    moved = state.inside(c)
    cl = state.st.classify(c, need_inside=True, stop_at_straddle=True)
    require(not cl.uncertain and cl.straddle is None, 'admissibility', f'simulator at {c}')
    require(np.array_equal(cl.inside, moved), 'support', 'simulator inside set')
    token = (moved, state.h[moved].copy(), state.b[moved].copy(), state.lab[moved].copy())
    ids, poses = state.st.pose_id.copy(), list(state.st._poses)
    old_matrices = {int(pid): state.st._pose_matrix(int(pid)) for pid in np.unique(ids[moved])}
    tw = BM.sim.Twist(C, c, P.matrix(c, i), 'menu', {'menu': P.M.name})
    outcome = state.st.apply(tw, journal=False)
    require(not outcome.uncertain and outcome.applied, 'admissibility', 'simulator application')
    try:
        require(outcome.moved == len(moved), 'support', 'simulator moved count')
        outside = np.ones(len(ids), bool)
        outside[moved] = False
        remap = np.array([state.st._lookup.get(p.key, -1) for p in poses], np.int64)
        require(np.array_equal(state.st.pose_id[outside], remap[ids[outside]]),
                'poses', 'an unmoved pose changed')
        for pid, matrix in old_matrices.items():
            new_ids = np.unique(state.st.pose_id[moved[ids[moved] == pid]])
            require(len(new_ids) == 1 and state.st._pose_matrix(int(new_ids[0]))
                    == to_tuple(matmul(P.matrix(c, i), matrix)), 'poses', 'moved pose product')
        exact = P.updated_rows(state.st, moved)
        hs, bs = J.tree_rows(state.st, moved)
        if i < 12:
            _, pre, _, sigma = retained(c, i)
            calculated = token[3][:, pre]
            require(np.array_equal(calculated, exact), 'retained labels', 'inverse-pole calculus')
            require(np.array_equal(hs, token[1]), 'retained heights')
            require(np.array_equal(bs, sigma[token[2]]), 'retained branches')
            state.lab[moved] = calculated
        else:
            state.lab[moved] = exact
        state.h[moved], state.b[moved] = hs, bs
        delta = J.phi(hs) - J.phi(token[1])
        return dict(support=support_claim(moved), delta=delta, phi=J.phi(state.h),
                    digest=state.st.digest()), token
    except BaseException:
        state.undo(c, i, token)
        raise


def execute(state, events, e=0, unit_piece=None, claims=None, tokens=None):
    """Build or check ONE descending macro; claims never authorize an event."""
    events = word(events)
    require(bool(events), 'events', 'empty macro')
    require(events[-1][1] >= 12, 'lowering count', 'last event must be a jumble')
    allowance(e)
    names = ('supports', 'delta', 'phi', 'digests')
    if claims is not None:
        require(isinstance(claims, dict), 'claims', 'missing claims')
        require(set(claims) == {*names, 'excursion', 'F', 'D'}, 'claims', 'missing or unknown claim')
        for name in (*names, 'excursion', 'F', 'D'):
            require(name in claims, 'claims', f'missing {name}')
        for name in names:
            require(isinstance(claims[name], list) and len(claims[name]) == len(events),
                    'claims', f'wrong length: {name}')
        for name in ('F', 'D', 'excursion'):
            require(type(claims[name]) is int and claims[name] >= 0,
                    'claims', f'{name} must be a nonnegative integer')
        for name in ('delta', 'phi'):
            require(all(type(v) is int and v % 2 == 0 for v in claims[name]),
                    'claims', f'{name} must contain even integers')
        require(all(type(v) is str and len(v) == 64 for v in claims['digests']),
                'claims', 'digests must be SHA256 strings')
        require(all(isinstance(v, dict) and set(v) == {'size', 'sha256'}
                    and type(v['size']) is int and v['size'] >= 0
                    and type(v['sha256']) is str and len(v['sha256']) == 64
                    for v in claims['supports']), 'support', 'invalid support claim')
    start_phi = J.phi(state.h)
    result = {key: [] for key in names}
    cumulative, peak = 0, 0
    # A unit is the high pose group at the beginning of the macro; it must
    # survive the preparation and each of its members must fall at the end.
    members = None
    if unit_piece is not None:
        members = P.unit_members(state, unit_piece)
        require((state.h[members] >= 4).all(), 'unit', 'not a high starting unit')
    for j, (c, i) in enumerate(events):
        try:
            require(not (state.lab[:, c] == 2).any(), 'admissibility', f'blocked grip {c}')
            if j == len(events) - 1:
                f, ds = state.budget(c, (i,))
                d = int(ds[0])
                result.update(F=f, D=d)
                if claims is not None:
                    require(claims['F'] == f and claims['D'] == d,
                            'lowering count', f'expected F={f}, D={d}')
                if members is not None:
                    require((state.lab[members, c] == 1).all(), 'unit', 'not wholly in lowering cap')
                    require((state.h[members] >= 4).all(), 'unit', 'no longer high')
                    unit_h = state.h[members].copy()
            row, token = apply_checked(state, c, i)
            if tokens is not None:
                tokens.append((c, i, token))
            cumulative += row['delta']
            peak = max(peak, cumulative)
            require(peak <= e, 'excursion', f'observed prefix excursion {peak}, allowance {e}')
            require(row['phi'] == start_phi + cumulative, 'telescope')
            for name, value in (('supports', row['support']), ('delta', row['delta']),
                                ('phi', row['phi']), ('digests', row['digest'])):
                result[name].append(value)
                if claims is not None:
                    test = 'support' if name == 'supports' else name
                    require(claims[name][j] == value, test, f'claim differs: {name}')
            if j == len(events) - 1:
                require(row['delta'] == 2 * (f - d), 'lowering count', 'Lemma Phi')
                if members is not None:
                    require((state.h[members] == unit_h - 2).all(), 'unit', 'not every member lowered by 2')
        except Failure as exc:
            exc.event = j + 1
            raise
    result['excursion'] = peak
    require(cumulative < 0, 'descent', 'macro must strictly lower Phi')
    if claims is not None:
        require(claims['excursion'] == peak, 'excursion', 'claim differs')
    return result


@contextmanager
def evaluated(state):
    """Collect application tokens; reverse every evaluated letter on all exits."""
    digest = state.st.digest()
    tokens = []
    try:
        yield tokens
    finally:
        for c, i, token in reversed(tokens):
            state.undo(c, i, token)
        require(state.st.digest() == digest, 'undo', 'evaluation batch')


def empty_certificate(table_digest, prefix, state, e):
    return dict(format=FORMAT, table_digest=table_digest,
                prefix=[list(p) for p in prefix], start=dict(digest=state.st.digest(), phi=J.phi(state.h)),
                excursion=allowance(e), macros=[])


def append_macro(state, certificate, events, unit_piece=None):
    spec = dict(events=[list(p) for p in events])
    if unit_piece is not None:
        spec['unit_piece'] = unit_piece
    spec['claims'] = execute(state, spec['events'], certificate['excursion'], unit_piece)
    certificate['macros'].append(spec)
    return spec


def load(table, prefix=(), labels=None):
    # Read the source identity before the prefix, not the digest of its image.
    original = BM.load_table(table)
    table_digest = original.digest()
    try:
        state, setup = P.load_input(table, ','.join(f'{c}:{i}' for c, i in prefix), labels)
    except ValueError as exc:
        if str(exc).startswith('--word: inadmissible letter'):
            # Recover the exact prefix index, including repeated letters. This
            # error-only replay avoids duplicating setup work on valid input.
            for j, (c, i) in enumerate(prefix):
                if not J.twist(original, c, P.matrix(c, i)):
                    raise Failure('admissibility', f'prefix letter {c}:{i}', event=j+1) from exc
        raise
    return state, table_digest, setup


def certify(table, prefix, macros, e=0, units=(), labels=None):
    require(len(units) <= len(macros), 'unit', 'more units than macros')
    state, digest, _ = load(table, prefix, labels)
    cert = empty_certificate(digest, prefix, state, e)
    for j, events in enumerate(macros):
        try:
            append_macro(state, cert, events, units[j] if j < len(units) else None)
        except Failure as exc:
            exc.macro = j + 1
            raise
    return cert


def independent_replay(table, cert):
    """replay_word.py's independent path: no cached labels or incremental trees."""
    st = BM.load_table(table)
    count = 0
    for j, (c, i) in enumerate(word(cert['prefix'])):
        require(J.twist(st, c, P.matrix(c, i)), 'replay admissibility', f'prefix {j + 1}')
        h, _ = BM.tree_info(st)  # deliberately after EVERY prefix letter
        count += 1
    h, _ = BM.tree_info(st)
    require(dict(digest=st.digest(), phi=J.phi(h)) == cert['start'], 'replay start')
    for m, spec in enumerate(cert['macros']):
        for j, (c, i) in enumerate(word(spec['events'])):
            require(J.twist(st, c, P.matrix(c, i)), 'replay admissibility', f'macro {m + 1}, event {j + 1}')
            h, _ = BM.tree_info(st)
            require(J.phi(h) == spec['claims']['phi'][j], 'replay phi', f'macro {m + 1}, event {j + 1}')
            require(st.digest() == spec['claims']['digests'][j], 'replay digest', f'macro {m + 1}, event {j + 1}')
            count += 1
    return dict(passed=True, events=count, digest=st.digest(), phi=J.phi(h))


def check(table, cert, labels=None, replay=False):
    started = time.process_time()
    result = dict(format='macro-check/1', passed=False, failures=[], macros=[])
    try:
        require(isinstance(cert, dict) and cert.get('format') == FORMAT, 'format')
        require(cert.get('table_digest') == BM.load_table(table).digest(), 'table_digest')
        allowance(cert['excursion'])
        require('prefix' in cert and 'start' in cert and isinstance(cert.get('macros'), list), 'claims')
        state, _, setup = load(table, word(cert['prefix']), labels)
        result['setup'] = setup
        require(cert['start'] == dict(digest=state.st.digest(), phi=J.phi(state.h)), 'start')
        for m, spec in enumerate(cert['macros']):
            try:
                require(isinstance(spec.get('claims'), dict), 'claims', 'missing claims')
                claims = execute(state, spec['events'], cert['excursion'], spec.get('unit_piece'), spec['claims'])
                result['macros'].append(dict(passed=True, claims=claims))
            except Failure as exc:
                exc.macro = m + 1
                raise
        result.update(passed=True, digest=state.st.digest(), phi=J.phi(state.h))
        if replay:
            result['replay'] = independent_replay(table, cert)
    except Failure as exc:
        result['passed'] = False
        result['failures'].append(exc.record())
    except (ValueError, KeyError, TypeError, IndexError, AssertionError) as exc:
        result['passed'] = False
        result['failures'].append(dict(test='certificate', detail=str(exc), event=None, macro=None))
    result['cpu_s'] = round(time.process_time() - started, 6)
    return result


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__)
    sub = ap.add_subparsers(dest='command', required=True)
    for command in ('cert', 'check'):
        p = sub.add_parser(command)
        p.add_argument('table')
        if command == 'check':
            p.add_argument('certificate')
        p.add_argument('out')
        p.add_argument('--labels')
        if command == 'cert':
            p.add_argument('--prefix', default='')
            p.add_argument('--macro', action='append', required=True)
            p.add_argument('--unit-piece', action='append', type=int, default=[])
            p.add_argument('--excursion', type=int, default=0)
        else:
            p.add_argument('--replay', action='store_true')
    a = ap.parse_args(argv)
    P.external_path(a.out)
    if a.labels:
        P.external_path(a.labels)
    try:
        if a.command == 'cert':
            result = certify(a.table, P.parse_word(a.prefix), [P.parse_word(w) for w in a.macro],
                             a.excursion, a.unit_piece, a.labels)
        else:
            result = check(a.table, json.loads(Path(a.certificate).read_text()), a.labels, a.replay)
    except (Failure, ValueError, KeyError, TypeError, AssertionError) as exc:
        result = dict(passed=False, failures=[exc.record() if isinstance(exc, Failure)
                                            else dict(test='certificate', detail=str(exc))])
    P.write_json(a.out, result)
    return 0 if result.get('passed', True) else 1


if __name__ == '__main__':
    sys.exit(main())
