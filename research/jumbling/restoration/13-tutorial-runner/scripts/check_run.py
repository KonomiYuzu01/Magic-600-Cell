"""Replay runner-run/1 records without the runner or its search/step code.

Usage (from the repository root): python3 -I -B check_run.py RUN [--out FILE]
Only the existing independent lattice verifier is run as a child process. Its
report goes in a fresh check/ subdirectory of RUN (or FILE's parent). All other
inputs are read-only. Failure names in the JSON lines identify the checked field
or contract; the final ``fails`` also includes failures from individual steps.
This checks the words played, not the runner's choice of rule or its budgets.
Evidence: exact simulator replay, not Windows/DirectX or performance evidence.
"""

import argparse
import gzip
import hashlib
import json
from pathlib import Path, PurePosixPath, PureWindowsPath
import re
import subprocess
import sys
import tempfile
import zlib

sys.dont_write_bytecode = True
REPO = Path(__file__).resolve().parents[5]
J = REPO / 'research/jumbling'
sys.path[:0] = [str(J / 'theory'), str(J), str(J / 'fixtures')]

import numpy as np  # noqa: E402
import restore  # noqa: E402
import rim  # noqa: E402
import sim  # noqa: E402
import trees  # noqa: E402
from sim.kplus import matrix_from_json  # noqa: E402
from sim.state import Pose  # noqa: E402

SOLVED = 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855'
FORBIDDEN = {'segments', 'slp', 'certificate', 'node'}
PATH_KEYS = {'file', 'word', 'unit', 'table', 'library', 'out'}
PHASES = ('bulk', 'tail', 'fallback')


def integer(x, minimum=0):
    return type(x) is int and x >= minimum


def digest(x):
    return isinstance(x, str) and re.fullmatch(r'[0-9a-fA-F]{64}', x) is not None


def add(fails, name):
    if name not in fails:
        fails.append(name)


def no_duplicates(pairs):
    doc = {}
    for key, value in pairs:
        if key in doc:
            raise ValueError('duplicate JSON key')
        doc[key] = value
    return doc


def read_json(path):
    raw = Path(path).read_bytes()
    if raw.startswith(b'\x1f\x8b'):
        raw = gzip.decompress(raw)
    return json.loads(raw, object_pairs_hook=no_duplicates)


def forbidden(doc):
    if isinstance(doc, dict):
        return bool(FORBIDDEN.intersection(doc)) or any(forbidden(v) for v in doc.values())
    return isinstance(doc, list) and any(forbidden(v) for v in doc)


def inside(base, name):
    """Reject traversal and Windows paths too, and resolve any symlink escape."""
    if (not isinstance(name, str) or not name or '\\' in name or '\x00' in name
            or PurePosixPath(name).is_absolute() or PureWindowsPath(name).drive
            or '..' in name.split('/')):
        raise ValueError('unsafe path')
    path = (base / name).resolve()
    if not path.is_relative_to(base):
        raise ValueError('path escapes run')
    return path


def private_path(text):
    """A string that names an absolute, home-relative, Windows or parent path anywhere in it."""
    return ('\\' in text or re.search(r'(^|[\s:=])[/~]', text) is not None
            or re.match(r'[A-Za-z]:[\\/]', text) is not None or '..' in text.split('/'))


def record_paths(base, doc, fails):
    if isinstance(doc, str):
        if private_path(doc):
            add(fails, 'path')
    elif isinstance(doc, dict):
        for key, value in doc.items():
            if key in PATH_KEYS and value is not None:
                try:
                    inside(base, value)
                except (ValueError, OSError, RuntimeError):
                    add(fails, 'path')
            # Script provenance keys are repository-relative paths. They are
            # checked for safe spelling, but no runner/search script is loaded.
            if key == 'scripts' and isinstance(value, dict):
                for name in value:
                    try:
                        inside(base, name)
                    except (ValueError, OSError, RuntimeError):
                        add(fails, 'path')
            record_paths(base, value, fails)
    elif isinstance(doc, list):
        for value in doc:
            record_paths(base, value, fails)


def load_table(path, menu):
    """Current-only table loading; do not import orient's search dependencies."""
    doc = read_json(path)
    st = sim.State(restore.ctx, menu=menu)
    ids = doc['pose_id']
    if (not isinstance(ids, list) or len(ids) != len(st.pose_id)
            or not isinstance(doc['poses'], list) or not doc['poses']
            or any(not integer(i) or i >= len(doc['poses']) for i in ids)
            or not digest(doc['digest'])):
        raise ValueError('invalid pose table')
    st._poses, st._lookup = [Pose(0, None, ('K', 0))], {('K', 0): 0}
    remap = np.array([st._intern_matrix(matrix_from_json(m)) for m in doc['poses']], np.int32)
    st.pose_id = remap[np.asarray(ids, np.int64)]
    st._compact()
    st.journal, st._anchor_cache = [], {}
    if st.digest() != doc['digest']:
        raise ValueError('table digest mismatch')
    return st


class Snapshot:
    def __init__(self, st):
        self.digest = st.digest()
        self.off = np.array([p.kidx < 0 for p in st._poses], bool)[st.pose_id]
        self.count = int(self.off.sum())
        self.ids = st.pose_id.copy()
        self.matrices = {int(p): st._pose_matrix(int(p)) for p in np.unique(st.pose_id[self.off])}
        self.st = st
        self._heights = self._n = None

    def heights(self):
        if self._heights is None:
            hist = {}
            ids, counts = np.unique(self.ids[self.off], return_counts=True)
            for p, count in zip(ids.tolist(), counts.tolist()):
                height = str(trees.dist(trees.factors(self.matrices[p])[0]))
                hist[height] = hist.get(height, 0) + count
            self._heights = hist
        return self._heights

    def n(self):
        # Called only while this snapshot is the current state.
        if self._n is None:
            self._n = int(rim.Domains(self.st).total_misalignment())
        return self._n

    def check(self, doc, prefix, fails, heights=False, n=False):
        if not isinstance(doc, dict):
            add(fails, prefix)
            return
        if not digest(doc.get('digest')) or doc['digest'] != self.digest:
            add(fails, prefix + '_digest')
        if not integer(doc.get('off')) or doc['off'] != self.count:
            add(fails, prefix + '_off')
        if heights or 'heights' in doc:
            hist = doc.get('heights')
            if (not isinstance(hist, dict) or any(not integer(v, 1) for v in hist.values())
                    or hist != self.heights()):
                add(fails, prefix + '_heights')
        if n or 'N' in doc:
            if not integer(doc.get('N')) or doc['N'] != self.n():
                add(fails, prefix + '_N')


def load_word(base, step, fails):
    try:
        path = inside(base, step.get('word'))
    except (ValueError, OSError, RuntimeError):
        add(fails, 'path')
        return None
    try:
        doc = read_json(path)
    except (ValueError, OSError, EOFError, zlib.error):
        add(fails, 'word_file')
        return None
    if forbidden(doc):
        add(fails, 'forbidden_key')
    if not isinstance(doc, dict) or set(doc) != {'word', 'input_digest', 'output_digest'}:
        add(fails, 'word_keys')
        return None
    word = doc['word']
    if (not isinstance(word, list) or any(not isinstance(t, list) or len(t) != 2
            or not integer(t[0]) or t[0] >= 600 or not integer(t[1]) or t[1] >= 60 for t in word)):
        add(fails, 'word_literal')
        return None
    if not digest(doc['input_digest']) or not digest(doc['output_digest']):
        add(fails, 'word_digest')
        return None
    return doc


def check_unit(base, step, before, after, fails):
    try:
        unit = read_json(inside(base, step.get('unit')))['pieces']
        if (not isinstance(unit, list) or not unit or any(not integer(p) or p >= len(before.off) for p in unit)
                or len(set(unit)) != len(unit)):
            raise ValueError('invalid unit')
    except (ValueError, OSError, EOFError, zlib.error, KeyError, TypeError):
        add(fails, 'unit')
        return
    if not integer(step.get('unit_size')) or step['unit_size'] != len(unit):
        add(fails, 'unit_size')
    mask = np.zeros(len(before.off), bool)
    mask[unit] = True
    landed = before.off & ~after.off
    for key, actual in (('landed', int((landed & mask).sum())),
                        ('others_landed', int((landed & ~mask).sum()))):
        if not integer(step.get(key)) or step[key] != actual:
            add(fails, key)
    if np.any(~before.off & after.off):
        add(fails, 'collateral')
    if after.off[unit].any():
        add(fails, 'unit_landed')
    expected = 'strict_prime' if step['phase'] == 'tail' else 'fallback'
    if step.get('contract') != expected:
        add(fails, 'contract')
    if expected == 'strict_prime':
        if not before.off[unit].all():
            add(fails, 'unit_off_before')
        # Pose numbers are not stable across simulator compaction. Compare the
        # exact matrices for every other piece that remains off the lattice.
        pairs = np.unique(np.stack((before.ids, after.ids), axis=1)[before.off & after.off & ~mask], axis=0)
        if any(before.matrices[int(a)] != after.matrices[int(b)] for a, b in pairs):
            add(fails, 'strict_prime_pose')
    elif np.any(after.off & ~(before.off & ~mask)):
        add(fails, 'fallback_subset')


def lattice_check(base, lattice, current, menu, bulk_table, chain, report_parent, fails, require_solved):
    """Bind the independent report to this replay, not just to its PASS flag."""
    if not isinstance(lattice, dict):
        add(fails, 'lattice')
        return False, None
    if current.count:
        add(fails, 'lattice_off')
    if lattice.get('input_digest') != current.digest:
        add(fails, 'lattice_input_digest')
    if type(lattice.get('chain')) is not bool or lattice['chain'] != bool(chain):
        add(fails, 'lattice_chain')
    try:
        lib = inside(base, lattice.get('library'))
        out = inside(base, lattice.get('out'))
        sha = hashlib.sha256(lib.read_bytes()).hexdigest()
        if not digest(lattice.get('library_sha256')) or sha != lattice['library_sha256']:
            add(fails, 'library_sha256')
        summary_file = out / 'restore-summary.json'
        # An interrupted lattice attempt is not a claim of a solution. Its
        # input and library still have to match; an unproduced length is null.
        if not summary_file.exists() and not require_solved and lattice.get('twists') is None:
            return False, None
        summary = read_json(summary_file)
        if summary['library']['sha256'] != sha:
            add(fails, 'library_sha256')
        runs = summary['runs']
        if not isinstance(runs, list):
            raise ValueError('invalid summary')
        matches = [r for r in runs if r.get('input_digest') == current.digest]
        if len(matches) != 1:
            add(fails, 'lattice_summary_run')
            return False, None
        run = matches[0]
        if run.get('status') != 'restored':
            if not require_solved and lattice.get('twists') is None:
                return False, None
            add(fails, 'lattice_status')
        retained, corrections = run.get('retained_length'), run.get('correction_twists', 0)
        twists = retained + corrections if integer(retained) and integer(corrections) else None
        if twists is None or not integer(lattice.get('twists')) or lattice['twists'] != twists:
            add(fails, 'lattice_twists')
        # This restorer is invoked on a pose table. The verifier's report uses
        # "input" rather than "input_digest", so bind that spec to the summary
        # and check its table's digest independently as well.
        spec = run.get('input')
        if not isinstance(spec, str) or not spec.startswith('table:'):
            add(fails, 'lattice_input_table')
        elif not chain:
            table = Path(spec[6:])
            if not table.is_absolute():
                table = REPO / table
            if load_table(table, menu).digest() != current.digest:
                add(fails, 'lattice_input_table')
    except (OSError, ValueError, EOFError, zlib.error, KeyError, TypeError, IndexError, AttributeError):
        add(fails, 'lattice_summary')
        return False, None
    if fails or not require_solved:
        return False, twists
    report_root = inside(report_parent, 'check')
    report_root.mkdir(parents=True, exist_ok=True)
    report_dir = Path(tempfile.mkdtemp(prefix='run-', dir=report_root))
    report = report_dir / 'report.json'
    verify = J / 'restoration/09-general-method/scripts/lattice/checker/verify_late.py'
    argv = [sys.executable, '-I', '-B', str(verify), str(REPO), str(J), str(lib), str(out), str(report)]
    # argv[6] *inside verify_late.py* must be --chain, and it must have a word.
    if chain:
        argv += ['--chain', str(bulk_table)] + [str(p) for p in chain]
    try:
        proc = subprocess.run(argv, cwd=REPO, capture_output=True, text=True, check=False)
        lines = proc.stdout.splitlines()
        if proc.returncode != 0 or not lines or not re.fullmatch(r'PASS\s+\S+', lines[-1]):
            add(fails, 'lattice_verify')
        checked = read_json(report)
        rows = checked.get('runs')
        if checked.get('pass') is not True:
            add(fails, 'lattice_verify')
        if not isinstance(rows, list) or len(rows) != 1:
            add(fails, 'lattice_checked_run')
        else:
            row = rows[0]
            if (row.get('input') != spec or row.get('status_claimed') != 'restored'
                    or row.get('restored_to_identity') is not True
                    or ('input_digest' in row and row['input_digest'] != current.digest)):
                add(fails, 'lattice_checked_run')
            if row.get('retained_length') != retained:
                add(fails, 'lattice_twists')
            if run.get('order') == 'retained-first':
                if row.get('final_digest_is_solved') is not True or row.get('final_labels_identity') is not True:
                    add(fails, 'lattice_final_identity')
                if row.get('correction_twists') != corrections:
                    add(fails, 'lattice_twists')
            if chain and checked.get('chain_end_digest') != current.digest:
                add(fails, 'lattice_input_digest')
    except (OSError, ValueError, EOFError, zlib.error, KeyError, TypeError, AttributeError):
        add(fails, 'lattice_verify')
    return not fails, twists


def check_run(run_dir, emit, report_parent=None):
    """Emit step rows and a final row, and return the final row (also used by tests)."""
    base = Path(run_dir).resolve()
    fails, step_fails, words = [], [], []
    counts = {phase: 0 for phase in PHASES}
    lattice_checked, lattice_twists, end_digest = False, None, None
    record, steps = {}, []
    emitted = 0
    try:
        record = read_json(base / 'run.json')
        if not isinstance(record, dict):
            record = {}
            raise ValueError('record must be an object')
        if record.get('format') != 'runner-run/1':
            add(fails, 'format')
        if record.get('menu') != 'I_a':
            add(fails, 'menu')
        if record.get('result') not in ('solved', 'stuck', 'budget', 'error'):
            add(fails, 'result')
        if forbidden(record):
            add(fails, 'forbidden_key')
        record_paths(base, record, fails)
        steps = record.get('steps')
        if not isinstance(steps, list) or any(not isinstance(s, dict) for s in steps):
            steps = []
            raise ValueError('invalid steps')
        if any(not isinstance(record.get(key), dict) for key in ('start', 'phases', 'totals')):
            add(fails, 'record')
        for step in steps:
            sf = []
            words.append(load_word(base, step, sf))
            # Strict-prime's unit is entirely off before and entirely on after:
            # these two counts cannot disagree even before replay begins.
            if (step.get('phase') == 'tail' and step.get('contract') == 'strict_prime'
                    and (not integer(step.get('landed')) or step.get('landed') != step.get('unit_size'))):
                add(sf, 'landed')
            step_fails.append(sf)
        # Invalid literal data is never handed to the simulator or the external
        # verifier, even if later digest fields could disguise it.
        if fails or any(step_fails):
            for step, sf in zip(steps, step_fails):
                row_fails = sf or fails or ['record']
                emit({'index': step.get('index'), 'phase': step.get('phase'), 'rule': step.get('rule'),
                      'pass': False, 'fails': list(row_fails)})
            for sf in step_fails:
                for name in sf:
                    add(fails, name)
            return finish(record, steps, counts, end_digest, False, lattice_twists, fails, emit)
        menu = restore.menu('I_a')
        start = record['start']
        if start['kind'] == 'scramble':
            if not integer(start['k']) or type(start['seed']) is not int:
                raise ValueError('invalid scramble')
            st, _ = restore.scramble(menu, start['k'], start['seed'])
        elif start['kind'] == 'table':
            st = load_table(inside(base, start['file']), menu)
        else:
            raise ValueError('invalid start kind')
        current = Snapshot(st)
        current.check(start, 'start', fails, heights=True)
        bulk = record['phases']['bulk']
        if not isinstance(bulk, dict):
            raise ValueError('invalid bulk phase')
        bulk_count = 0
        for step in steps:
            if step.get('phase') != 'bulk':
                break
            bulk_count += 1
        bulk_table = inside(base, bulk['end']['table'])

        def check_bulk_end():
            current.check(bulk['end'], 'bulk_end', fails, heights=True)
            if bulk['end']['table'] != 'tables/bulk-end.json':
                add(fails, 'bulk_table')
            try:
                if load_table(bulk_table, menu).digest() != current.digest:
                    add(fails, 'bulk_table_digest')
            except (OSError, ValueError, EOFError, zlib.error, KeyError, TypeError, IndexError):
                add(fails, 'bulk_table')

        if not bulk_count:
            check_bulk_end()
        chain = []
        for index, (step, word) in enumerate(zip(steps, words)):
            sf = step_fails[index]
            phase = step.get('phase')
            if not isinstance(step.get('rule'), str) or not step['rule']:
                add(sf, 'rule')
            if not integer(step.get('index')) or step['index'] != index:
                add(sf, 'index')
            if phase not in PHASES:
                add(sf, 'phase')
            if phase == 'bulk' and index >= bulk_count:
                add(sf, 'phase_order')
            before = current
            if phase == 'bulk':
                before.n()
            before.check(step.get('before'), 'before', sf, heights=phase in ('tail', 'fallback'), n=phase == 'bulk')
            if word['input_digest'] != before.digest:
                add(sf, 'input_digest')
            if not integer(step.get('letters')) or step['letters'] != len(word['word']):
                add(sf, 'letters')
            parts = step.get('parts')
            if 'parts' in step and (not isinstance(parts, list)
                    or any(not isinstance(p, list) or len(p) != 2 or not isinstance(p[0], str)
                           or not integer(p[1]) for p in parts)
                    or sum(p[1] for p in parts) != len(word['word'])):
                add(sf, 'parts')
            for cap, item in word['word']:
                try:
                    outcome = st.apply(sim.Twist(restore.ctx, cap, restore.element(menu, cap, item),
                                                 'menu', {'menu': menu.name}), journal=False)
                    admitted = outcome.applied and not outcome.uncertain
                except Exception:  # a simulator error cannot certify a letter
                    admitted = False
                if not admitted:
                    add(sf, 'admissible')
                    break
            current = Snapshot(st)
            current.check(step.get('after'), 'after', sf, heights=phase in ('tail', 'fallback'), n=phase == 'bulk')
            if word['output_digest'] != current.digest:
                add(sf, 'output_digest')
            if phase in PHASES:
                counts[phase] += len(word['word'])
            if phase == 'bulk':
                recorded_before, recorded_after = step.get('before', {}).get('N'), step.get('after', {}).get('N')
                if (not integer(recorded_before) or not integer(recorded_after)
                        or recorded_after >= recorded_before or current.n() >= before.n()):
                    add(sf, 'N_decrease')
            elif phase in ('tail', 'fallback'):
                check_unit(base, step, before, current, sf)
                chain.append(inside(base, step['word']))
            emit({'index': step.get('index'), 'phase': phase, 'rule': step.get('rule'),
                  'pass': not sf, 'fails': list(sf)})
            emitted += 1
            for name in sf:
                add(fails, name)
            if index + 1 == bulk_count:
                check_bulk_end()
        for phase, data in record['phases'].items():
            if phase not in PHASES or not isinstance(data, dict):
                add(fails, 'phase')
                continue
            # The tail loop includes fallback steps unless a separate fallback
            # phase is explicitly recorded. Totals always separate the two.
            group = ('tail', 'fallback') if phase == 'tail' and 'fallback' not in record['phases'] else (phase,)
            nsteps = sum(s.get('phase') in group for s in steps)
            if not integer(data.get('steps')) or data['steps'] != nsteps:
                add(fails, 'phase_steps')
            if 'twists' in data and (not integer(data['twists']) or data['twists'] != sum(counts[p] for p in group)):
                add(fails, 'phase_twists')
            if phase != 'bulk' and 'end' in data:
                current.check(data['end'], phase + '_end', fails, heights=True)
        if record.get('result') == 'stuck' or record.get('stuck') is not None:
            current.check(record.get('stuck'), 'stuck', fails, heights=True)
        lattice = record.get('lattice')
        if lattice is not None:
            lattice_checked, lattice_twists = lattice_check(
                base, lattice, current, menu, bulk_table, chain,
                Path(report_parent).resolve() if report_parent is not None else base, fails,
                require_solved=record.get('result') == 'solved')
        elif record.get('result') == 'solved' and current.digest != SOLVED:
            add(fails, 'solved_digest')
        end_digest = SOLVED if lattice_checked else current.digest
        totals = record['totals']
        for phase in PHASES:
            name = phase + '_twists'
            if not integer(totals.get(name)) or totals[name] != counts[phase]:
                add(fails, name)
        if (totals.get('lattice_twists') != lattice_twists or 'lattice_twists' not in totals
                or (totals['lattice_twists'] is not None and not integer(totals['lattice_twists']))):
            add(fails, 'lattice_twists')
    except Exception:  # malformed input must still get a failing JSON final row
        add(fails, 'record')
        for step in steps[emitted:]:
            emit({'index': step.get('index'), 'phase': step.get('phase'), 'rule': step.get('rule'),
                  'pass': False, 'fails': ['record']})
    return finish(record, steps, counts, end_digest, lattice_checked, lattice_twists, fails, emit)


def finish(record, steps, counts, end_digest, lattice_checked, lattice_twists, fails, emit):
    final = {'pass': not fails, 'result': record.get('result'), 'steps': len(steps),
             **{p + '_twists': counts[p] for p in PHASES}, 'lattice_twists': lattice_twists,
             'end_digest': end_digest, 'lattice_checked': lattice_checked, 'fails': list(fails)}
    emit(final)
    return final


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('run_dir', type=Path)
    parser.add_argument('--out', type=Path)
    args = parser.parse_args()
    output = None
    try:
        if args.out is not None:
            output = args.out.open('w')

        def emit(row):
            line = json.dumps(row, sort_keys=True)
            print(line, flush=True)
            if output is not None:
                output.write(line + '\n')
                output.flush()

        final = check_run(args.run_dir, emit, args.out.parent if args.out is not None else None)
    finally:
        if output is not None:
            output.close()
    return 0 if final['pass'] else 1


if __name__ == '__main__':
    sys.exit(main())
