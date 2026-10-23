"""Independent literal checker for r22d25-run/1, not a coverage proof.

From the repository root:
  python3 -I -B <this file> RUN_DIR [--cache DIR] [--record-dir CENSUS_DIR]

Reads records, word files and the repository model/simulator only. Every
accepted word is played twist by twist. Large negative searches remain claims.
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
import re
import tempfile
from collections import Counter, deque
from pathlib import Path, PureWindowsPath

ROOT = Path(__file__).resolve().parents[5]
J = ROOT / 'research/jumbling'
sys.path[:0] = [str(J / 'theory'), str(J), str(J / 'fixtures')]
import numpy as np
import sim
import restore
import trees
import orient
import rim
import seams
from exact import matmul
from sim.kernel import Evaluator, pullback_form
from sim.kplus import to_tuple

LIMITS = {'direct': 244, 'S': 244, 'T': 720, 'ST': 964}
FAMILIES = {'F1', 'F2', 'F3', 'd19', 'S', 'ST', 'T1'}
SOURCES = {'research/jumbling/restoration/12-low-height/scripts/round22/r22d25_state.py',
           'research/jumbling/restoration/12-low-height/scripts/round22/r22d25_search.py', 'assets/manifest.json'}
HEX = re.compile(r'^[0-9a-f]{64}$')


class CheckFailure(Exception):
    pass


def need(condition, name):
    if not condition:
        raise CheckFailure(name)


def relative_paths(value):
    """Reject absolute paths anywhere, including parameters and dictionary keys."""
    if isinstance(value, dict):
        return all(relative_paths(k) and relative_paths(v) for k, v in value.items())
    if isinstance(value, (list, tuple)):
        return all(relative_paths(v) for v in value)
    if isinstance(value, str):
        return not (value.startswith(('/', '\\')) or PureWindowsPath(value).is_absolute()
                    or re.match(r'^[A-Za-z]:', value))
    return True


def contained(root, name, check):
    need(isinstance(name, str) and name and relative_paths(name), check)
    path = root / name
    need('..' not in Path(name).parts and path.resolve().is_relative_to(root.resolve()), check)
    return path


def nonnegative(value):
    return type(value) is int and value >= 0


def duration(value):
    return type(value) in (int, float) and math.isfinite(value) and value >= 0


def is_digest(value):
    return isinstance(value, str) and HEX.fullmatch(value) is not None


def valid_census(value):
    return isinstance(value, dict) and all(isinstance(k, str) and k.isdigit() and int(k) > 0
                                          and nonnegative(v) for k, v in value.items())


def validate_format(doc):
    need(isinstance(doc, dict) and doc.get('format') == 'r22d25-run/1', 'format')
    need(relative_paths(doc), 'paths.relative')
    need(doc.get('catalogue') == 'r22d25-cat/1', 'catalogue')
    limits = doc.get('limits', {})
    need(set(limits) == set(LIMITS) and all(type(limits[k]) is int and 1 <= limits[k] <= v
                                          for k, v in LIMITS.items()), 'limits')
    start = doc.get('start', {})
    need(start.get('kind') in ('record', 'table') and isinstance(start.get('file'), str)
         and Path(start['file']).name == start['file'], 'start.file')
    for label, row in (('start', start), ('last', doc.get('last', {}))):
        need(is_digest(row.get('digest')) and nonnegative(row.get('off'))
             and valid_census(row.get('heights')) and sum(row['heights'].values()) == row['off'], label + '.format')
    need(isinstance(doc.get('steps'), list) and duration(doc.get('seconds')), 'run.format')
    need(doc.get('result') in ('lattice', 'stuck', 'budget', 'unsupported', 'error'), 'result')
    for i, step in enumerate(doc['steps']):
        prefix = f'step.{i}.'
        need(step.get('index') == i and step.get('kind') in ('landing', 'lowering'), prefix + 'format')
        need(isinstance(step.get('family'), str) and step['family'].replace('_', '').isalnum(), prefix + 'family')
        target = step.get('target')
        need(isinstance(target, list) and len(target) > 0 and all(nonnegative(p) for p in target)
             and target == sorted(set(target)), prefix + 'target')
        need(isinstance(step.get('params'), dict) and type(step.get('high_exact')) is bool, prefix + 'gate.format')
        need(step.get('word') == f'words/{i:05d}.json', prefix + 'word.path')
        need(nonnegative(step.get('letters')) and step['letters'] > 0 and duration(step.get('seconds')), prefix + 'letters.format')
        need(is_digest(step.get('input_digest')) and is_digest(step.get('output_digest')), prefix + 'digest.format')
        need(nonnegative(step.get('off_before')) and nonnegative(step.get('off_after'))
             and valid_census(step.get('heights_after')), prefix + 'counts.format')
    rows = doc.get('ledger')
    need(isinstance(rows, dict) and FAMILIES <= set(rows), 'ledger.families')
    for family, row in rows.items():
        need(isinstance(row, dict) and row.get('status') in ('exhausted', 'budget', 'unsupported', 'not_applicable')
             and nonnegative(row.get('tried')) and isinstance(row.get('pruned'), dict)
             and all(nonnegative(n) for n in row['pruned'].values()) and 'cursor' in row, 'ledger.format.' + family)
        need(row['status'] != 'budget' or isinstance(row['cursor'], dict), 'ledger.cursor.' + family)
    scripts = doc.get('scripts')
    need(isinstance(scripts, dict) and SOURCES == set(scripts)
         and all(is_digest(v) for v in scripts.values()), 'scripts.format')
    need(all(step['family'] in rows and step['family'] not in ('S',) for step in doc['steps']), 'steps.families')


def copy_state(st):
    other = sim.State(st.ctx, filtered=False, menu=restore.menu('I_a'))
    other.pose_id, other._poses, other._lookup = st.pose_id.copy(), list(st._poses), dict(st._lookup)
    return other


def piece_heights(st):
    by_pose = []
    for p in st._poses:
        if p.kidx >= 0:
            by_pose.append(0)
        else:
            l, r = trees.factors(p.matrix)
            need(trees.dist(r) == 0, 'model.right_height')
            by_pose.append(int(trees.dist(l)))
    return np.asarray(by_pose, np.int32)[st.pose_id]


def summary(st):
    h = piece_heights(st)
    return {'digest': st.digest(), 'off': int((h > 0).sum()),
            'heights': {str(k): int(v) for k, v in sorted(Counter(h[h > 0].tolist()).items())}}


def literal(word, name):
    need(isinstance(word, list) and len(word) > 0, name)
    for pair in word:
        need(isinstance(pair, list) and len(pair) == 2 and type(pair[0]) is int and type(pair[1]) is int
             and 0 <= pair[0] < 600 and 0 <= pair[1] < 60, name)
    return word


def replay(st, word, name):
    end = copy_state(st)
    M = restore.menu('I_a')
    for i, (c, j) in enumerate(word):
        o = end.apply(sim.Twist(end.ctx, c, restore.element(M, c, j), 'menu', {'menu': M.name}), journal=False)
        need(o.applied and not o.uncertain, name + f'.prefix.{i}')
    return end


def gate(before, after, target, lowering=False, high_exact=False, extra_exact=()):
    """Independent exact-pose gate. A boolean supplied by the record proves nothing."""
    n = len(before.pose_id)
    if not target or any(type(p) is not int or p < 0 or p >= n for p in target):
        return False
    lat0, lat1 = before.lattice_flags(), after.lattice_flags()
    if lat0[target].any() or (lat0 & ~lat1).any():
        return False
    hs = piece_heights(before)
    targets = set(target)
    for p in np.flatnonzero(~lat0).tolist():
        if p in targets:
            continue
        exact = before._poses[int(before.pose_id[p])].key == after._poses[int(after.pose_id[p])].key
        if (high_exact and hs[p] >= 4) or p in extra_exact:
            if not exact:
                return False
        elif not lat1[p] and not exact:
            return False
    if lowering:
        h = int(hs[target[0]])
        return (h >= 4 and (hs[target] == h).all()
                and (lat1[target].all() or (piece_heights(after)[target] == h - 2).all()))
    return bool(lat1[target].all())


def start_state(doc, directory, record_dir=None):
    src = doc['start']
    path = contained(directory, src['file'], 'start.file')
    if not path.is_file() and src['kind'] == 'record':
        base = Path(record_dir) if record_dir else J / 'restoration/11-scramble-census/runs'
        matches = list(base.rglob(src['file']))
        need(len(matches) == 1, 'start.file.locate')
        path = matches[0]
    need(path.is_file(), 'start.file.missing')
    source = json.loads(path.read_text())
    st = sim.State(sim.get_context(), filtered=False, menu=restore.menu('I_a'))
    if src['kind'] == 'table':
        orient.set_pose_table(st, source)
    else:
        M = restore.menu('I_a')
        for i, (c, j) in enumerate(source['history'] + source['moves']):
            need(type(c) is int and type(j) is int and 0 <= c < 600 and 0 <= j < 60, 'start.word')
            o = st.apply(sim.Twist(st.ctx, c, restore.element(M, c, j), 'menu', {'menu': M.name}), journal=False)
            need(o.applied and not o.uncertain, f'start.prefix.{i}')
    need(len(st.pose_id) == len(st.ctx.data.sigbits), 'start.full_model')
    return st


def family_gate(st, end, step, word, limits, prefix):
    family, target, params = step['family'], step['target'], step['params']
    h = piece_heights(st)
    need(max(target) < len(h), prefix + 'gate.target')
    if family in ('F1', 'F2', 'F3', 'd19', 'ST'):
        # Reconstruct the complete pose/inside-vector unit, including pieces
        # the record might have omitted. A target's representative is no proof.
        high = np.flatnonzero(h >= 4)
        need(set(target) <= set(high.tolist()), prefix + 'family.unit')
        lab = labels(st, high) == 1
        first = int(np.searchsorted(high, target[0]))
        key = st._poses[int(st.pose_id[target[0]])].key
        same_pose = np.array([st._poses[int(st.pose_id[p])].key == key for p in high], bool)
        whole_unit = high[same_pose & (lab == lab[first]).all(axis=1)].tolist()
        need(target == whole_unit, prefix + 'family.unit')
    else:
        # The tail's units are rigid components split by frame domain, with no
        # optional free-set relaxation.
        _, low = units(st, labels(st), h)
        need(target in low, prefix + 'family.unit')
    extra = set(np.flatnonzero(h == 6).tolist()) if family not in ('F1', 'F2', 'F3') else set()
    if 'high_exact_pieces' in params:
        expected = np.flatnonzero(h >= 4).tolist() if step['high_exact'] else sorted(extra)
        need(params['high_exact_pieces'] == expected, prefix + 'gate.high_exact_pieces')
    if family in ('F1', 'F2', 'F3'):
        need((h[target] == 6).all(), prefix + 'gate.height')
        need(all(j != 0 for _, j in word), prefix + 'family.nonidentity')
        if family == 'F1':
            need(len(word) == 1, prefix + 'family.F1')
        elif family == 'F2':
            need(len(word) == 3 and word[0][0] == word[2][0] and word[0][0] != word[1][0], prefix + 'family.F2')
        else:
            a, W = params.get('a'), params.get('W')
            need(a == word[0] and isinstance(W, list) and len(W) in (1, 2), prefix + 'family.F3')
            M = restore.menu('I_a')
            def inverse(pair):
                c, j = pair
                g = restore.element(M, c, j)
                return [c, next(k for k in range(1, 60)
                                if to_tuple(matmul(g, restore.element(M, c, k))) == restore.I4)]
            need(word == [a] + W + [inverse(a)] + [inverse(p) for p in reversed(W)], prefix + 'family.F3.inverse')
            c, j = a
            lab = labels(st, target)
            need((lab[:, c] == 1).all(), prefix + 'family.F3.whole')
            b = trees.pose_tree(st.pose(target[0]))[2]
            need(trees.branch(tuple(st.ctx.data.N[c])) == b, prefix + 'family.F3.branch')
            middle = replay(st, [a], prefix + 'family.F3.a')
            mh = piece_heights(middle)
            need((mh[target] == 4).all(), prefix + 'family.F3.lowering')
            b2 = trees.pose_tree(middle.pose(target[0]))[2]
            need(all(trees.branch(tuple(st.ctx.data.N[d])) == b2 for d, _ in W), prefix + 'family.F3.Wbranch')
        need(gate(st, end, target, lowering=True, high_exact=step['high_exact']), prefix + 'gate.lowering')
        expected_kind = 'landing' if end.lattice_flags()[target].all() else 'lowering'
        need(step['kind'] == expected_kind, prefix + 'gate.kind')
    else:
        need(step['kind'] == 'landing', prefix + 'gate.kind')
        if family == 'd19':
            need((h[target] == 4).all() and len(word) <= limits['direct'], prefix + 'family.d19')
        elif family == 'ST':
            split = params.get('split')
            need((h[target] == 4).all() and split == 1 and params.get('S_length') == 1
                 and params.get('T_length') == len(word) - 1 and 1 <= len(word) - 1 <= limits['T']
                 and len(word) <= limits['ST'] and split <= limits['S'], prefix + 'family.ST.split')
            middle = replay(st, word[:split], prefix + 'family.ST.S')
            need(middle.digest() == params.get('middle_digest'), prefix + 'family.ST.middle_digest')
            need((piece_heights(middle)[target] == 2).all()
                 and gate(st, middle, target, lowering=True, extra_exact=extra), prefix + 'gate.S')
            need(gate(middle, end, target, extra_exact=extra), prefix + 'gate.T')
            need(params.get('T_family') != 'T1' or len(word) - split == 1, prefix + 'family.ST.T1')
        else:
            need((h[target] == 2).all() and len(word) <= limits['T'] and step['high_exact'], prefix + 'family.T')
            if family == 'T1':
                need(len(word) == 1, prefix + 'family.T1')
        need(gate(st, end, target, high_exact=step['high_exact'], extra_exact=extra), prefix + 'gate.strict')


def labels(st, pieces=None):
    """Complete classification, implemented here without a kernel dependency."""
    C, n = st.ctx, len(st.pose_id)
    pick = np.arange(n) if pieces is None else np.asarray(pieces, np.int64)
    out = np.zeros((len(pick), 600), np.uint8)
    pos = np.full(n, -1, np.int32)
    pos[pick] = np.arange(len(pick))
    k = st._kpose()[st.pose_id]
    lp, op = pick[k[pick] >= 0], pick[k[pick] < 0]
    groups = [(pid, st._anchors(pid, mem)) for pid, mem in st._groups(op)] if len(op) else []
    for c in range(600):
        out[pos[lp], c] = C.data.in_sig(lp, C.kplus.perms[C.kplus.inv[k[lp]], c])
        for pid, anchors in groups:
            ev = Evaluator(pullback_form(st._pose_matrix(pid), C.data.N[c]))
            for a, sub in anchors:
                lab = ev.classify(C.regions.cap_vform(a))[0]
                need(lab != 'uncertain', 'singles.classification')
                if lab != 'straddle':
                    if lab == 'in':
                        out[pos[sub], c] = 1
                    continue
                for p in sub.tolist():
                    lab = ev.classify(C.regions.vform(p))[0]
                    need(lab != 'uncertain', 'singles.classification')
                    out[pos[p], c] = {'in': 1, 'out': 0, 'straddle': 2}[lab]
    return out


def units(st, lab, h):
    high = {}
    for p in np.flatnonzero(h >= 4).tolist():
        key = (st._poses[int(st.pose_id[p])].key, (lab[p] == 1).tobytes())
        high.setdefault(key, []).append(p)
    off = np.flatnonzero(h > 0)
    if not len(off):
        return list(high.values()), []
    bits = np.concatenate((np.packbits(lab[off] == 1, axis=1), np.packbits(lab[off] == 0, axis=1)), axis=1)
    _, first, inverse = np.unique(bits, axis=0, return_index=True, return_inverse=True)
    I, O = bits[first, :75], bits[first, 75:]
    remaining = set(range(len(first)))
    D = rim.Domains(st)
    low = []
    while remaining:
        seed = min(remaining)
        remaining.remove(seed)
        queue, component = deque([seed]), []
        while queue:
            p = queue.popleft()
            component.append(p)
            separated = ((I[p] & O) | (O[p] & I)).any(axis=1)
            near = [q for q in sorted(remaining) if not separated[q]]
            remaining.difference_update(near)
            queue.extend(near)
        ps = off[np.isin(inverse, component)]
        for d in sorted(set(D.dom[ps].tolist())):
            U = ps[D.dom[ps] == d]
            if (h[U] == 2).all():
                low.append(U.tolist())
    return sorted(high.values(), key=lambda u: u[0]), sorted(low, key=lambda u: u[0])


def single_words(st, lab, h, target, lowering=False, high_exact=False, extra_exact=()):
    """Independent necessary tests followed by a fresh literal replay and gate."""
    lat = st.lattice_flags()
    M = restore.menu('I_a')
    retained = {j for j, (_, m) in enumerate(M.items) if st.ctx.kplus.index_of_matrix(to_tuple(m)) >= 0}
    targets = set(target)
    for c in range(600):
        if (lab[:, c] == 2).any() or not (lab[target, c] == 1).all() or ((lab[:, c] == 1) & lat).any():
            continue
        inside = np.flatnonzero(lab[:, c] == 1)
        for j in range(1, 60):
            if j in retained:
                continue
            good = True
            g = restore.element(M, c, j)
            for pid, ps in st._groups(inside):
                q = to_tuple(matmul(g, st._pose_matrix(pid)))
                landed = st.ctx.kplus.index_of_matrix(q) >= 0
                itarget = targets.intersection(ps.tolist())
                other = set(ps.tolist()) - targets
                if (not landed and not lowering) or (other and not landed):
                    good = False
                    break
                if itarget and lowering and not landed and trees.pose_tree(q)[0] != int(h[target[0]]) - 2:
                    good = False
                    break
                if (high_exact and any(h[p] >= 4 for p in other)) or other.intersection(extra_exact):
                    good = False
                    break
            if good:
                end = replay(st, [[c, j]], 'singles.replay')
                if gate(st, end, target, lowering, high_exact, extra_exact):
                    yield [[c, j]]


def exhaustion(st, doc, emit):
    need(all(row['status'] == 'exhausted' for row in doc['ledger'].values()), 'stuck.ledger')
    h = piece_heights(st)
    need(h.max(initial=0) <= 6, 'stuck.height')
    lab = labels(st)
    high, low = units(st, lab, h)
    for U in high:
        if h[U[0]] == 6:
            need(next(single_words(st, lab, h, U, lowering=True), None) is None, 'stuck.F1')
    emit({'check': 'stuck.F1', 'ok': True, 'verification': 're-derived exactly'})
    h6 = set(np.flatnonzero(h == 6).tolist())
    S_count = 0
    for U in high:
        if h[U[0]] == 4:
            S_count += sum(1 for _ in single_words(st, lab, h, U, lowering=True, extra_exact=h6))
    need(doc['ledger']['S']['tried'] >= S_count, 'stuck.S.count')
    emit({'check': 'stuck.S', 'ok': True, 'protected_single_letters': S_count,
          'verification': 're-derived exactly; subsequent T exhaustion is claimed'})
    for U in low:
        need(next(single_words(st, lab, h, U, high_exact=True), None) is None, 'stuck.T1')
    emit({'check': 'stuck.T1', 'ok': True, 'verification': 're-derived exactly at the last state'})
    emit({'check': 'stuck.other_families', 'ok': True, 'verification': 'claimed, not re-verified',
          'families': sorted(set(doc['ledger']) - {'F1', 'S', 'T1'})})


def verify(directory, cache, record_dir=None, emit=print):
    """Return zero only after checking the complete record, emitting JSON lines."""
    def checked(row):
        emit(json.dumps(row, sort_keys=True))
    try:
        directory = Path(directory)
        Path(cache).mkdir(parents=True, exist_ok=True)
        seams.CACHE = Path(cache) / 'home_centroids.npy'
        os.environ['JUMBLING_HOME_CACHE'] = str(seams.CACHE.resolve())
        doc = json.loads((directory / 'run.json').read_text())
        validate_format(doc)
        checked({'check': 'format', 'ok': True})
        # The model file is the model identity and must match. The kernel files
        # are provenance only: the replay below never uses them, so a later
        # kernel edit is reported and does not fail an older record.
        differs = []
        for name, digest in doc['scripts'].items():
            path = contained(ROOT, name, 'scripts.path')
            same = path.is_file() and hashlib.sha256(path.read_bytes()).hexdigest() == digest
            if name == 'assets/manifest.json':
                need(same, 'scripts.digest')
            elif not same:
                differs.append(name)
        checked({'check': 'scripts', 'ok': True, 'kernel_differs': sorted(differs)})
        st = start_state(doc, directory, record_dir)
        need(summary(st) == {k: doc['start'][k] for k in ('digest', 'off', 'heights')}, 'start.state')
        checked({'check': 'start.state', 'ok': True})
        initial = piece_heights(st)
        bound = 6 * int((initial == 6).sum()) + LIMITS['ST'] * int((initial > 0).sum())
        total = 0
        for i, step in enumerate(doc['steps']):
            prefix = f'step.{i}.'
            wf = contained(directory, step['word'], prefix + 'word.path')
            word_doc = json.loads(wf.read_text())
            need(relative_paths(word_doc) and set(word_doc) == {'word', 'input_digest', 'output_digest'}, prefix + 'word.format')
            word = literal(word_doc['word'], prefix + 'word.literal')
            need(st.digest() == step['input_digest'] == word_doc['input_digest'], prefix + 'input_digest')
            need(len(word) == step['letters'], prefix + 'played_length')
            need(st.off_lattice_count() == step['off_before'], prefix + 'off_before')
            end = replay(st, word, prefix + 'word')
            need(end.digest() == step['output_digest'] == word_doc['output_digest'], prefix + 'output_digest')
            need(end.off_lattice_count() == step['off_after'], prefix + 'off_after')
            need(summary(end)['heights'] == step['heights_after'], prefix + 'heights_after')
            family_gate(st, end, step, word, doc['limits'], prefix)
            if step['kind'] == 'landing':
                need(step['off_after'] < step['off_before'], prefix + 'landing_progress')
            else:
                need((piece_heights(end) == 6).sum() < (piece_heights(st) == 6).sum(), prefix + 'lowering_progress')
            total += len(word)
            need(total <= bound, 'run.termination_bound')
            checked({'check': prefix + 'literal_gate', 'ok': True, 'letters': len(word)})
            st = end
        need(summary(st) == doc['last'], 'last.state')
        checked({'check': 'last.state', 'ok': True})
        if doc['result'] == 'lattice':
            need(st.is_lattice(), 'result.lattice')
        elif doc['result'] == 'stuck':
            need(not st.is_lattice(), 'stuck.off')
            exhaustion(st, doc, checked)
        elif doc['result'] == 'budget':
            need(any(r['status'] == 'budget' and isinstance(r['cursor'], dict) for r in doc['ledger'].values()), 'budget.cursor')
        elif doc['result'] == 'unsupported':
            need(any(r['status'] == 'unsupported' for r in doc['ledger'].values()), 'unsupported.ledger')
            # Either a piece above height 6, or the d19 program was unavailable.
            need(piece_heights(st).max(initial=0) > 6 or doc['ledger'].get('d19', {}).get('status') == 'unsupported',
                 'unsupported.precondition')
        checked({'check': 'result.' + doc['result'], 'ok': True})
        emit('PASS')
        return 0
    except CheckFailure as exc:
        checked({'check': str(exc), 'ok': False})
        emit('FAIL ' + str(exc))
    except Exception as exc:
        name = 'record.' + type(exc).__name__
        checked({'check': name, 'ok': False})
        emit('FAIL ' + name)
    return 1


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('directory', type=Path)
    ap.add_argument('--cache', type=Path)
    ap.add_argument('--record-dir', type=Path)
    args = ap.parse_args(argv)
    if args.cache is None:
        with tempfile.TemporaryDirectory(prefix='r22d25-check-') as cache:
            return verify(args.directory, cache, args.record_dir)
    return verify(args.directory, args.cache, args.record_dir)


if __name__ == '__main__':
    sys.exit(main())
