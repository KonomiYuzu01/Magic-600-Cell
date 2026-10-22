"""Play the tutorial's lattice chapter literally in an isolated child.

Headless exact research code. Importing the plugin needs no geometry; only the
--_play child imports the literal player and the pose-table exporter.
"""
import argparse
import gzip
import hashlib
import json
import math
from pathlib import Path, PurePosixPath, PureWindowsPath
import re
import shutil
import sys
import time
import zlib

HERE = Path(__file__).resolve().parent
ROOT = Path(__file__).resolve().parents[5]
if str(HERE) not in sys.path:
    sys.path.insert(0, str(HERE))
import runner_api as api  # noqa: E402

G9 = Path('research/jumbling/restoration/09-general-method')
LAT = G9 / 'scripts/lattice/scripts'
SCH = G9 / 'scripts/schedule11'
S03 = Path('research/jumbling/restoration/03-retained-group-and-lattice-restorer/scripts')
RESTORERS = {'late': LAT / 'restore_lattice_late.py',
             'schedule11': SCH / 'restore_schedule11_late.py'}
REALIGN = G9 / 'data/realign.json.gz'
LIBRARY = G9 / 'data/library-merged.json.gz'
PLAY_SECONDS = None
TREE_PART_LIMIT = 10**6
SCRIPTS = [Path(__file__).resolve().relative_to(ROOT),
           HERE.relative_to(ROOT) / 'literal_lattice.py',
           HERE.relative_to(ROOT) / 'runner_api.py', *RESTORERS.values(),
           *(LAT / name for name in ('rlib.py', 'kit.py', 'nucoords.py')), SCH / 'sched11.py',
           *(S03 / name for name in ('rlib.py', 'kit.py', 'nucoords.py'))]

ROW_KEYS = frozenset(('index', 'phase', 'subphase', 'rule', 'word', 'letters', 'encoding',
    'word_sha256', 'tree', 'contract', 'gate', 'standing', 'before', 'after', 'table', 'seconds', 'attempts'))
FIELD_FORBIDDEN = ROW_KEYS | api.FORBIDDEN_KEYS | {
    'file', 'unit', 'library', 'out', 'parts', 'V_source', 'V_letters', 'V_sha256',
    'construction', 'certificate_sha256'}
TREE_KEYS = {'name', 'letters', 'parts', 'span', 'inverse_of', 'same_as'}
STEP_KEYS = {'rule', 'word_file', 'word_sha256', 'letters', 'tree_file', 'tree_sha256',
             'tree_size', 'before', 'after', 'fields', 'seconds'}
DEFAULTS = {'contract': 'lattice_literal', 'gate': 'player', 'standing': 'rule', 'encoding': 'ia16'}


class Invalid(ValueError):
    """A public check name, never a raw path or exception message."""


def require(condition, check):
    if not condition:
        raise Invalid(check)


def integer(value, minimum=0):
    return type(value) is int and value >= minimum


def digest(value):
    return isinstance(value, str) and re.fullmatch('[0-9a-f]{64}', value) is not None


def duration(value):
    try:
        return type(value) in (int, float) and math.isfinite(value) and value >= 0
    except OverflowError:
        return False


def snapshot(value):
    return (isinstance(value, dict) and set(value) == {'digest', 'off', 'heights'}
            and type(value['off']) is int and value['off'] == 0 and value['heights'] == {}
            and digest(value['digest']))


def private_path(value):
    return ('\\' in value or re.search(r'(^|[\s:=])[/~]', value) is not None
            or re.match(r'[A-Za-z]:[\\/]', value) is not None or '..' in value.split('/'))


def safe_record(value):
    if isinstance(value, str):
        return not private_path(value)
    if isinstance(value, dict):
        return all(isinstance(k, str) and k not in api.FORBIDDEN_KEYS and safe_record(k)
                   and safe_record(v) for k, v in value.items())
    if isinstance(value, list):
        return all(safe_record(v) for v in value)
    return value is None or type(value) in (bool, int) or type(value) is float and math.isfinite(value)


def no_constant(value):
    raise ValueError('nonfinite JSON number')


def read_json(path):
    return json.loads(Path(path).read_bytes(), parse_constant=no_constant)


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def atomic_json(path, value):
    api.atomic_bytes(Path(path), (json.dumps(value, allow_nan=False, separators=(',', ':')) + '\n').encode())


def orbit_partition(summary):
    """Validate the restorer's chronological partition without loading geometry."""
    order = summary.get('cascade_order')
    require(isinstance(order, list) and all(type(o) is int for o in order)
            and len(set(order)) == len(order), 'cascade_order')
    run = summary['runs'][0]
    count = run.get('factors')
    require(integer(count), 'orbit_partition')
    per = run.get('factors_per_orbit')
    if count == 0 and per is None:
        return [(o, 0, 0) for o in order]
    require(isinstance(per, dict) and list(per) == [str(o) for o in order]
            and all(integer(n) for n in per.values()) and sum(per.values()) == count,
            'orbit_partition')
    first, result = 0, []
    for orbit in order:
        n = per[str(orbit)]
        result.append((orbit, first, n))
        first += n
    return result


def orbit_tree(program, factors, orbit):
    """A bounded tree in played order, with backward references local to this step.

    committed includes pending parts, so expansion can never consume space
    already promised to a later sibling. Node recursion stops at depth 64.
    """
    total = sum(program.lengths[j] for j, _ in factors)
    committed = 1 + 2 * len(factors)  # root, factor groups, their node parts
    if committed > TREE_PART_LIMIT:
        return api.single(f'orbit {orbit}', total), 1, 0
    first, at, folded, folded_depth = {}, 0, 0, 0

    def name(j, sign):
        return f'{program.nodes[j]["op"]} {j}' + ('⁻¹' if sign < 0 else '')

    def span(j, sign, path, reference=None):
        nonlocal at
        result = api.span(name(j, sign), at, at + program.lengths[j])
        if reference is not None:
            key, earlier = reference
            result[key] = list(earlier)
        first.setdefault((j, sign), path)
        at += program.lengths[j]
        return result

    def node(j, sign, path, depth):
        nonlocal committed, folded, folded_depth
        previous = first.get((j, sign))
        if previous is not None:
            return span(j, sign, path, ('same_as', previous))
        nd = program.nodes[j]
        if j in program.words or program.lengths[j] == 0:
            return span(j, sign, path)
        if depth == api.TREE_MAX_DEPTH:
            folded_depth += 1
            return span(j, sign, path)
        if nd['op'] == 'conj':
            children = [(nd['u'], -nd['us']), (nd['x'], sign), (nd['u'], nd['us'])]
        else:
            children = nd['f'] if sign > 0 else [(j, -s) for j, s in reversed(nd['f'])]
        if committed + len(children) > TREE_PART_LIMIT:
            folded += 1
            return span(j, sign, path)
        committed += len(children)
        first[(j, sign)] = path
        parts = []
        for k, (child, sg) in enumerate(children):
            child_path = path + (k,)
            if nd['op'] == 'conj' and k == 2:
                parts.append(span(child, sg, child_path, ('inverse_of', path + (0,))))
            else:
                parts.append(node(child, sg, child_path, depth + 1))
        return api.group(name(j, sign), parts)

    parts = [api.group(f'factor {i}', [node(j, sign, (i, 0), 3)])
             for i, (j, sign) in enumerate(factors)]
    root = api.group(f'orbit {orbit}', parts)
    api.check_tree(root, total)
    return root, folded, folded_depth


def correction_tree(index, correction, macro):
    length = len(macro['records'])
    name = f'macro {correction["macro"]} {macro["family"]}' + ('⁻¹' if correction['inverse'] else '')
    return api.flat(f'correction {index}', [(name, length)] * correction['times'],
                    same={i: 0 for i in range(1, correction['times'])})


class PlayerObserver:
    """Stream only letters delivered by the literal player; hash exact boundaries."""

    def __init__(self, output, before, summary, library):
        self.output, self.before = Path(output), dict(before)
        self.summary, self.library = summary, library
        self.steps, self.current, self.final_state = [], None, None
        self.factor_index = self.orbit_index = self.correction_index = 0

    def begin(self, program, factors, corrections, order):
        self.program, self.factors, self.corrections = program, factors, corrections
        self.orbits = [entry for entry in orbit_partition(self.summary) if entry[2]]
        require(sum(n for _, _, n in self.orbits) == len(factors), 'orbit_partition')

    def _open(self, rule):
        require(self.current is None, 'step_boundary')
        k = len(self.steps)
        word = f'words/{k:05d}.ia16.gz'
        self.current = {'rule': rule, 'word_file': word, 'before': dict(self.before)}
        self.writer = api.Ia16Writer(self.output / word)
        self.started = time.monotonic()

    def _orbit(self):
        orbit, first, count = self.orbits[self.orbit_index]
        if self.current is None:
            self._open(f'lattice/orbit-{orbit}')
        return orbit, first, count

    def leaf(self, node, sign, codes):
        self._orbit()
        self.writer.write_codes(codes)

    def _close(self, state, tree, fields):
        live = state()
        require(live.is_lattice(), 'step_lattice')
        after = {'digest': live.digest(), 'off': 0, 'heights': {}}
        word = self.writer.close()
        tree_name = f'step-trees/{len(self.steps):05d}.json.gz'
        info = api.write_tree(self.output / tree_name, tree, word['letters'])
        self.steps.append({**self.current, 'word_sha256': word['sha256'], 'letters': word['letters'],
            'tree_file': tree_name, 'tree_sha256': info['sha256'], 'tree_size': info['size'],
            'after': after, 'fields': fields, 'seconds': time.monotonic() - self.started})
        self.before, self.current = after, None

    def factor_end(self, index, state):
        require(index == self.factor_index, 'factor_boundary')
        orbit, first, count = self._orbit()
        if index + 1 == first + count:
            tree, folded, depth = orbit_tree(self.program, self.factors[first:first + count], orbit)
            self._close(state, tree, {'orbit': orbit, 'factor_first': first, 'factor_count': count,
                                     'tree_folded': folded, 'tree_folded_depth': depth})
            self.orbit_index += 1
        self.factor_index += 1

    def correction_twist(self, code):
        correction = self.corrections[self.correction_index]
        if self.current is None:
            self._open(f'lattice/correction-{self.correction_index}-{correction["macro"]}')
        self.writer.write_codes([code])

    def correction_end(self, index, state):
        require(index == self.correction_index, 'correction_boundary')
        correction = self.corrections[index]
        macro = self.library['macros'][correction['macro']]
        if self.current is None:
            self._open(f'lattice/correction-{index}-{correction["macro"]}')
        self._close(state, correction_tree(index, correction, macro),
                    {'correction': index, 'macro': correction['macro'], 'family': macro['family'],
                     'times': correction['times'], 'inverse': correction['inverse'],
                     'macro_letters': len(macro['records'])})
        self.correction_index += 1

    def final(self, state):
        require(self.current is None and self.factor_index == len(self.factors)
                and self.correction_index == len(self.corrections), 'final_boundary')
        self.final_state = state

    def discard(self):
        if self.current is not None:
            self.writer.file.close()
            self.writer.raw.close()
            self.current = None
        discard_play(self.output)
        self.steps = []


def discard_play(output):
    for name in ('words', 'step-trees'):
        shutil.rmtree(Path(output) / name, ignore_errors=True)
    (Path(output) / 'end-table.json').unlink(missing_ok=True)


def play_child(restore, table, library, output, seconds=None):
    # The player sets thread limits before importing NumPy. No geometry is
    # imported when the runner merely discovers the plugin.
    import literal_lattice
    output = Path(output)
    output.mkdir(parents=True, exist_ok=True)
    before = {'digest': read_json(table)['digest'], 'off': 0, 'heights': {}}
    observer = PlayerObserver(output, before, read_json(Path(restore) / 'restore-summary.json'), read_json(library))
    report = literal_lattice.play(restore, table, 0, library, seconds, observer=observer)
    if report['result'] == 'solved':
        require(observer.final_state is not None, 'final_boundary')
        import buffer
        buffer.export_pose_table(observer.final_state, output / 'end-table.json')
        if observer.steps:
            observer.steps[-1]['fields']['labels_identity'] = report['final']['labels_identity']
    else:
        observer.discard()
    atomic_json(output / 'play-result.json', {'format': 'lattice-phase-play/1', 'report': report,
                'steps': observer.steps, 'end_table': 'end-table.json'})
    print(json.dumps({'result': report['result'], 'failure': report['failure'],
                      'steps': len(observer.steps), 'played': report['played']}), flush=True)


def inside(output, name):
    require(isinstance(name, str) and name and '\\' not in name and '\x00' not in name,
            'manifest')
    path = PurePosixPath(name)
    require(not path.is_absolute() and not PureWindowsPath(name).drive and '..' not in path.parts,
            'manifest')
    result = (output / path).resolve()
    require(result.is_relative_to(output.resolve()), 'manifest')
    return result


def manifest_schema(output):
    doc = read_json(output / 'play-result.json')
    require(isinstance(doc, dict) and set(doc) == {'format', 'report', 'steps', 'end_table'}
            and doc['format'] == 'lattice-phase-play/1' and isinstance(doc['steps'], list), 'manifest')
    inside(output, doc['end_table'])
    report = doc['report']
    report_keys = {'format', 'restore_dir', 'run', 'input', 'order', 'scheduled', 'played', 'junction',
                   'final', 'result', 'failure', 'position', 'seconds', 'model_files_checked', 'scripts'}
    require(isinstance(report, dict) and set(report) == report_keys and report.get('format') == 'literal-lattice/1'
            and report.get('result') in ('solved', 'failed', 'budget') and safe_record(report), 'manifest')
    require(isinstance(report['restore_dir'], str) and integer(report['run'])
            and report['order'] in ('retained-first', 'corrections-first')
            and isinstance(report['input'], dict) and set(report['input']) == {'table', 'digest', 'labels_sha256'}
            and report['input']['table'] == 'start-table.json'
            and all(v is None or digest(v) for k, v in report['input'].items() if k != 'table')
            and isinstance(report['junction'], dict) and set(report['junction']) == {'T_digest', 'labels_equal'}
            and (report['junction']['T_digest'] is None or digest(report['junction']['T_digest']))
            and (report['junction']['labels_equal'] is None or type(report['junction']['labels_equal']) is bool)
            and isinstance(report['seconds'], dict) and set(report['seconds']) == {'retained', 'corrections', 'total'}
            and all(duration(v) for v in report['seconds'].values())
            and type(report['model_files_checked']) is bool and isinstance(report['scripts'], dict)
            and all(digest(v) for v in report['scripts'].values()), 'manifest')
    require(isinstance(report.get('played'), dict) and set(report['played']) == {'retained', 'corrections', 'total'}
            and all(integer(n) for n in report['played'].values())
            and report['played']['total'] == report['played']['retained'] + report['played']['corrections']
            and isinstance(report.get('scheduled'), dict) and set(report['scheduled']) == {'retained', 'corrections'}
            and all(integer(n) for n in report['scheduled'].values())
            and isinstance(report.get('final'), dict) and set(report['final']) == {'digest', 'labels_identity'}
            and (report['final']['digest'] is None or digest(report['final']['digest']))
            and type(report['final']['labels_identity']) is bool
            and 'failure' in report and (report['failure'] is None or isinstance(report['failure'], str))
            and 'position' in report and (report['position'] is None or isinstance(report['position'], dict)), 'manifest')
    for step in doc['steps']:
        require(isinstance(step, dict) and STEP_KEYS <= set(step) and not set(step) - STEP_KEYS - DEFAULTS.keys(),
                'manifest')
        require(isinstance(step['rule'], str) and digest(step['word_sha256']) and integer(step['letters'])
                and digest(step['tree_sha256']) and integer(step['tree_size'], 1)
                and isinstance(step['before'], dict) and isinstance(step['after'], dict)
                and isinstance(step['fields'], dict) and type(step['seconds']) in (int, float), 'manifest')
        inside(output, step['word_file'])
        inside(output, step['tree_file'])
    return doc


def expected_steps(summary, library):
    orbits = [(f'lattice/orbit-{o}', {'orbit': o, 'factor_first': first, 'factor_count': n})
              for o, first, n in orbit_partition(summary) if n]
    run = summary['runs'][0]
    corrections = []
    require(isinstance(run.get('corrections'), list), 'fields')
    for k, c in enumerate(run['corrections']):
        require(isinstance(c, dict) and integer(c.get('macro')) and c['macro'] < len(library['macros'])
                and integer(c.get('times'), 1) and type(c.get('inverse')) is bool, 'fields')
        macro = library['macros'][c['macro']]
        corrections.append((f'lattice/correction-{k}-{c["macro"]}',
            {'correction': k, 'macro': c['macro'], 'family': macro['family'], 'times': c['times'],
             'inverse': c['inverse'], 'macro_letters': len(macro['records'])}))
    order = run.get('order', 'corrections-first')
    require(order in ('corrections-first', 'retained-first'), 'fields')
    return orbits + corrections if order == 'retained-first' else corrections + orbits


def checked_steps(doc, output, before, summary, library):
    report, steps, expected = doc['report'], [], before
    require(report['result'] == 'solved' and report['failure'] is None
            and report['position'] is None and report['model_files_checked'] is True
            and report['input']['digest'] == before['digest']
            and report['order'] == summary['runs'][0].get('order', 'corrections-first')
            and report['played']['retained'] == report['scheduled']['retained']
            and report['played']['corrections'] == report['scheduled']['corrections']
            and report['final']['labels_identity'] is True, 'report')
    plan = expected_steps(summary, library)
    require(len(doc['steps']) == len(plan), 'steps')
    for k, (s, (rule, metadata)) in enumerate(zip(doc['steps'], plan)):
        require(s['rule'] == rule and s['rule'].startswith('lattice/') and
                all(s.get(key, value) == value for key, value in DEFAULTS.items()), 'step')
        require(snapshot(s['before']) and snapshot(s['after']) and s['before'] == expected, 'snapshots')
        require(duration(s['seconds']), 'seconds')
        fields = s['fields']
        extra = {'tree_folded', 'tree_folded_depth'} if 'orbit' in metadata else set()
        last = {'labels_identity'} if k + 1 == len(plan) else set()
        require(set(fields) == set(metadata) | extra | last and not FIELD_FORBIDDEN.intersection(fields)
                and safe_record(fields) and all(fields[key] == value and type(fields[key]) is type(value)
                                               for key, value in metadata.items())
                and all(integer(fields[key]) for key in extra)
                and (not last or fields['labels_identity'] is True), 'fields')
        word, tree = inside(output, s['word_file']), inside(output, s['tree_file'])
        try:
            require(api.ia16_digest(word) == {'sha256': s['word_sha256'], 'letters': s['letters']}, 'word_sha256')
        except (OSError, EOFError, ValueError, zlib.error):
            raise Invalid('word_sha256') from None
        try:
            root, tree_sha = api.read_tree(tree)
            require(tree_sha == s['tree_sha256'] and api.check_tree(root, s['letters']) == s['tree_size'], 'tree')
            for part, _, _ in api.walk(root):
                require(not set(part) - TREE_KEYS and integer(part.get('letters'))
                        and isinstance(part.get('name'), str) and not private_path(part['name']), 'tree')
                if 'parts' in part:
                    require(isinstance(part['parts'], list) and bool(part['parts']), 'tree')
                for key in ('span', 'inverse_of', 'same_as'):
                    if key in part:
                        require(isinstance(part[key], list) and all(integer(n) for n in part[key]), 'tree')
        except (OSError, EOFError, ValueError, KeyError, TypeError, IndexError, AttributeError, RecursionError, zlib.error):
            raise Invalid('tree') from None
        steps.append(api.LatticeStep(rule, word, s['word_sha256'], s['letters'], tree,
            s['tree_sha256'], s['tree_size'], s['before'], s['after'], fields, s['seconds']))
        expected = s['after']
    require(sum(s.letters for s in steps) == report['played']['total'], 'letters')
    try:
        end_table = inside(output, doc['end_table'])
        end = read_json(end_table)
        require(isinstance(end, dict) and end.get('digest') == report['final']['digest'] == expected['digest'],
                'end_table')
    except (OSError, EOFError, ValueError):
        raise Invalid('end_table') from None
    return steps, end_table


def script_name(path):
    # Test stubs may live in a temporary directory; never publish that directory.
    path = Path(path)
    if path.is_absolute():
        return path.relative_to(ROOT).as_posix() if path.is_relative_to(ROOT) else path.name
    return path.as_posix()


def play(ctx):
    output = ctx.work_dir / 'play'
    restorer, elapsed = None, 0.0

    def end(result, reason, details=None, attempt=False):
        discard_play(output)
        attempts = []
        if attempt:
            attempts.append({'rule': 'lattice/' + restorer, 'outcome': result, 'reason': reason,
                             'timed_out': result in ('budget', 'timeout'), 'seconds': elapsed})
        return api.End(result, reason, details or {}, attempts)

    def child_end(child, stage):
        if child.cancelled:
            return end('error', 'cancelled')
        if child.timeout:
            return end('timeout', 'lattice_wall_clock', attempt=True)
        if child.rc != 0:
            return end('error', 'restorer_exit' if stage == 'restore' else 'player_exit')
        return None

    before = ctx.snapshot()
    if before.get('off') != 0:
        return end('error', 'not_on_lattice')
    try:
        table_bytes = Path(ctx.table).read_bytes()
        if json.loads(table_bytes).get('digest') != before.get('digest'):
            return end('error', 'table_digest')
    except (OSError, ValueError, AttributeError):
        return end('error', 'table_digest')
    restorer = ctx.budgets.get('lattice_restorer')
    if not isinstance(restorer, str) or restorer not in RESTORERS:
        return end('error', 'restorer_unknown')
    try:
        ctx.work_dir.mkdir(parents=True, exist_ok=True)
        (ctx.work_dir / 'start-table.json').write_bytes(table_bytes)
        raw = (ROOT / LIBRARY).read_bytes()
        library_bytes = gzip.decompress(raw) if raw[:2] == b'\x1f\x8b' else raw
        (ctx.work_dir / 'library-merged.json').write_bytes(library_bytes)
        library = json.loads(library_bytes)
        (ctx.work_dir / 'restore').mkdir()
        output.mkdir()
    except (OSError, EOFError, ValueError, zlib.error):
        return end('error', 'restorer_output_missing')
    library_sha = hashlib.sha256(library_bytes).hexdigest()
    script = ROOT / RESTORERS[restorer]
    argv = [sys.executable, '-I', '-B', script]
    if restorer == 'late':
        argv += [script.parent, ROOT, ROOT / 'research/jumbling']
    else:
        argv += [ROOT, ROOT / 'research/jumbling', ROOT / REALIGN]
    argv += ['library-merged.json', 'restore', 'table:start-table.json']
    try:
        child = ctx.execute(argv, 'lattice-restore-' + ctx.work_dir.name, None, cwd=ctx.work_dir)
    except OSError:
        return end('error', 'restorer_exit')
    elapsed = restore_seconds = child.seconds
    failure = child_end(child, 'restore')
    if failure is not None:
        return failure
    summary_path, solutions = ctx.work_dir / 'restore/restore-summary.json', ctx.work_dir / 'restore/solutions.json'
    try:
        summary = read_json(summary_path)
        read_json(solutions)
    except (OSError, EOFError, ValueError):
        return end('error', 'restorer_output_missing')
    try:
        require(isinstance(summary, dict) and isinstance(summary.get('runs'), list)
                and len(summary['runs']) == 1 and isinstance(summary['runs'][0], dict), 'runs')
        run = summary['runs'][0]
        status = run.get('status')
        if isinstance(status, str) and status.startswith('failure certificate:'):
            require(safe_record(status), 'status')
            return end('stuck', 'restorer_no_solution', {'status': status}, attempt=True)
        if status != 'restored':
            return end('error', 'restorer_status', {'status': status if isinstance(status, str)
                                                 and safe_record(status) else 'invalid'})
        require(run.get('input') == 'table:start-table.json', 'input')
        require(run.get('input_digest') == before['digest'], 'input_digest')
        require(isinstance(summary.get('library'), dict) and summary['library'].get('sha256') == library_sha,
                'library_sha256')
        partition = orbit_partition(summary)
        if restorer == 'schedule11':
            require(summary.get('realign_sha256') == sha(ROOT / REALIGN), 'realign_sha256')
    except (Invalid, KeyError, TypeError, OSError) as exc:
        return end('error', 'restorer_output_invalid', {'check': str(exc) if isinstance(exc, Invalid) else 'summary'})
    argv = [sys.executable, '-I', '-B', Path(__file__).resolve(), '--_play',
            'restore', 'start-table.json', 'library-merged.json', 'play']
    if PLAY_SECONDS is not None:
        argv += ['--seconds', str(PLAY_SECONDS)]
    try:
        child = ctx.execute(argv, 'lattice-play-' + ctx.work_dir.name, None, cwd=ctx.work_dir)
    except OSError:
        return end('error', 'player_exit')
    elapsed += child.seconds
    failure = child_end(child, 'play')
    if failure is not None:
        return failure
    if not (output / 'play-result.json').is_file():
        return end('error', 'player_exit')
    try:
        doc = manifest_schema(output)
    except (OSError, EOFError, ValueError, TypeError, KeyError, RecursionError):
        return end('error', 'play_output_invalid', {'check': 'manifest'})
    player = doc['report']
    if player['result'] == 'failed':
        return end('error', 'player_failed', {'failure': player['failure']})
    if player['result'] == 'budget':
        return end('budget', 'player_clock', {'position': player['position'], 'played': player['played']}, attempt=True)
    try:
        steps, end_table = checked_steps(doc, output, before, summary, library)
    except (Invalid, OSError, EOFError, ValueError, TypeError, KeyError, IndexError) as exc:
        return end('error', 'play_output_invalid', {'check': str(exc) if isinstance(exc, Invalid) else 'manifest'})
    report = {'restorer': restorer, 'restorer_script': script_name(RESTORERS[restorer]),
        'restore_seconds': restore_seconds, 'play_seconds': child.seconds,
        'restore_summary_sha256': sha(summary_path), 'solutions_sha256': sha(solutions),
        'library_sha256': library_sha, 'realign_sha256': summary.get('realign_sha256') if restorer == 'schedule11' else None,
        'cascade_order': summary['cascade_order'], 'skipped_orbits': [o for o, _, n in partition if not n],
        'step_counts': {'orbit': sum(n > 0 for _, _, n in partition), 'correction': len(run['corrections'])},
        'tree_folded': sum(s.fields.get('tree_folded', 0) for s in steps),
        'tree_folded_depth': sum(s.fields.get('tree_folded_depth', 0) for s in steps), 'player': player}
    if not safe_record(report):
        return end('error', 'play_output_invalid', {'check': 'report'})
    return api.LatticeResult(steps, end_table, report)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--_play', action='store_true', required=True)
    parser.add_argument('restore', type=Path)
    parser.add_argument('table', type=Path)
    parser.add_argument('library', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--seconds', type=float)
    args = parser.parse_args(argv)
    play_child(args.restore, args.table, args.library, args.output, args.seconds)
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
