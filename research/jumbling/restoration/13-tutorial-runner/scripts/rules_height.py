"""Height rules for the tutorial runner (exact, headless research).

Each call reads one state and asks unchanged research programs for one step.
Only the runner plays that step. ``parse_log`` returns a dictionary with start,
normalized steps, end (tag and payload), status and an optional reason.
``choose_pair`` returns result (step, exhausted, undecided or error), the step
when decided, and each output's end. Historical truncated shards can decide a
pair; the live adapter rejects every unexplained truncation.
"""
from concurrent.futures import ThreadPoolExecutor, as_completed
import json
import math
import os
from pathlib import Path
import pickle
import re
import signal
import sys
import threading

import runner_api as api

ROOT = Path(__file__).resolve().parents[5]
ROUND = Path('research/jumbling/restoration/12-low-height/scripts/round22')
H4 = ROUND / 'cloud/height4-descent-bbef1c8e'
RULE_SOURCES = {
    'r22d75': H4 / 'r22d75_from_snapshot.py',
    'r22d81': ROUND / 'r22d81_rule.py',
    'r22d81_pairs': ROUND / 'r22d81_pairs.py',
    'h4': H4 / 'h4_unlock.py',
    'two_letter_u': H4 / 'two_letter_u.py',
}
RULE_SCRIPTS = dict(RULE_SOURCES)
SCRIPTS = [Path(__file__).resolve().relative_to(ROOT), *RULE_SOURCES.values(),
           ROUND / 'r22d31_snapshots.py', ROUND.parent / 'eb6.py']
PREAMBLE = {'SNAPSHOT', 'RECORD', 'LOADED', 'U_ITEMS', 'SELFCHECK', 'LETTERS'}
ENDS = {
    'r22d75': {'HEIGHT2', 'STUCK', 'BUDGET'},
    'r22d81': {'HANDOFF', 'STUCK', 'BUDGET', 'NOT_APPLICABLE'},
    'r22d81_pairs': {'UNLOCKED', 'STUCK', 'BUDGET', 'NOT_APPLICABLE'},
    'h4': {'HANDOFF', 'STUCK', 'BUDGET'},
    'two_letter_u': {'DONE', 'BUDGET'},
}
NO_TARGET_NOTE = 'no target; L2 is not run without a target (h4_unlock.py:314)'


class OutputError(ValueError):
    """A child-output error, caught at the plugin boundary with its attempts."""
    def __init__(self, reason='malformed_output'):
        self.reason = reason


def _need(condition, reason='malformed_output'):
    if not condition:
        raise OutputError(reason)


def _integer(value, low=0, high=None):
    return type(value) is int and value >= low and (high is None or value < high)


def _digest(value):
    _need(isinstance(value, str) and re.fullmatch('[0-9a-f]{64}', value) is not None)
    return value


def _letter(value):
    _need(isinstance(value, list) and len(value) == 2 and
          _integer(value[0], high=600) and _integer(value[1], 1, 60))
    return value


def _counts(value):
    _need(isinstance(value, list) and len(value) == 3 and all(_integer(n) for n in value))
    return value


def _target_fields(payload):
    _need('tau' in payload and (payload['tau'] is None or _integer(payload['tau'], high=600)))
    _need(_integer(payload.get('u')))


def _details(payload):
    """Copy typed, small evidence fields, never arbitrary text or log paths."""
    result = {}
    for key in ('tau', 'u', 'steps', 'max_height'):
        if key in payload:
            value = payload[key]
            _need(value is None or _integer(value))
            if key == 'tau':
                _need(value is None or value < 600)
            result[key] = value
    if 'digest' in payload:
        result['digest'] = _digest(payload['digest'])
    for key in ('n2_n4_n6', 'n6_n4_n2'):
        if key in payload:
            result[key] = _counts(payload[key])
    if 'limit_cpu' in payload:
        value = payload['limit_cpu']
        _need(type(value) in (int, float) and math.isfinite(value) and value >= 0)
        result['limit_cpu'] = value
    if 'targets' in payload:
        value = payload['targets']
        _need(isinstance(value, list) and all(isinstance(p, list) and len(p) == 2 and
              _integer(p[0], high=600) and _integer(p[1], 1) for p in value))
        result['targets'] = value
    if payload.get('best_pair') is not None:
        value = payload['best_pair']
        _need(isinstance(value, dict) and _integer(value.get('u_end')) and
              _integer(value.get('u_mid')) and isinstance(value.get('letters'), list) and
              len(value['letters']) == 2)
        result['best_pair'] = {'u_end': value['u_end'], 'u_mid': value['u_mid'],
                               'letters': [_letter(p) for p in value['letters']]}
    return result


def write_snapshot(path, state, digest, off):
    """Write our own state atomically in r22d31_snapshots.load's format."""
    path = Path(path)
    temporary = path.with_name(path.name + '.tmp')
    try:
        with temporary.open('wb') as f:
            pickle.dump({'record': 'runner', 'history': 0, 'moves': 0,
                         'digest': digest, 'off': off, 'pose_id': state.pose_id.copy(),
                         'poses': [(p.kidx, p.matrix, p.key) for p in state._poses]}, f, protocol=5)
            f.flush()
            os.fsync(f.fileno())
        os.replace(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)


def _events(dialect, lines):
    known = ENDS[dialect] | PREAMBLE | ({'CONTEXT', 'FIRST'} if dialect == 'two_letter_u'
                                      else {'START', 'STEP'})
    for line in lines:
        if not line.endswith('\n'):
            continue
        if line.startswith('Traceback'):
            yield 'crash', {}
            return
        tag, _, raw = line.rstrip('\r\n').partition(' ')
        if tag not in known:
            continue
        try:
            payload = json.loads(raw, parse_constant=lambda value: _need(False))
        except (ValueError, TypeError, RecursionError):
            raise OutputError() from None
        if tag not in PREAMBLE:
            _need(isinstance(payload, dict))
            yield tag, payload


def _normalized_step(dialect, previous, payload):
    _need(previous is not None)
    _digest(payload.get('digest'))
    _digest(payload.get('input_digest'))
    _need(payload['input_digest'] == previous['digest'], 'digest_mismatch')
    fields = dict(tau_before=None, tau_after=None, u_before=None, u_after=None,
                  rule_script=RULE_SOURCES[dialect].as_posix())
    if dialect == 'r22d75':
        letters = payload.get('word')
        old, new = _counts(previous.get('n6_n4_n2')), _counts(payload.get('n6_n4_n2'))
        _need(new < old)
        tier = next(i for i in range(3) if new[i] != old[i])
        rule, length = f'r22d75/T{tier}', 1
    else:
        _target_fields(payload)
        family = payload.get('family')
        families = {'r22d81': ('H6', 'U', 'H4', 'H2'),
                    'r22d81_pairs': ('U', 'U2'), 'h4': ('L4', 'U', 'L2')}
        _need(family in families[dialect])
        if dialect == 'r22d81_pairs':
            letters = payload.get('letters')
            length = 2 if family == 'U2' else 1
            _need(payload['tau'] == previous['tau'] and payload['tau'] is not None)
            if length == 2:
                _need(_integer(payload.get('u_mid')))
                fields['u_mid'] = payload['u_mid']
            rule = 'r22d88/' + family
        else:
            letters, length = [payload.get('letter')], 1
            if family not in ('H6', 'L4'):
                _need(payload['tau'] == previous['tau'])
            if dialect == 'h4' and family in ('U', 'L2'):
                _need(previous['tau'] is not None)
            rule = ('r22d81/' if dialect == 'r22d81' else 'h4/') + family
        fields.update(tau_before=previous['tau'], tau_after=payload['tau'],
                      u_before=previous['u'], u_after=payload['u'])
    _need(isinstance(letters, list) and len(letters) == length)
    letters = [_letter(p) for p in letters]
    if dialect != 'h4':
        _need(type(payload.get('length')) is int and payload['length'] == length)
    return {'rule': rule, 'letters': letters, 'input_digest': payload['input_digest'],
            'output_digest': payload['digest'], 'fields': fields}


def _parse_log(dialect, lines, deciding=False):
    if dialect not in ENDS or dialect == 'two_letter_u':
        raise ValueError('unknown STEP dialect')
    parsed = {'start': None, 'steps': [], 'end': None, 'status': 'truncated'}
    previous = None
    try:
        for tag, payload in _events(dialect, lines):
            if tag == 'crash':
                parsed.update(status='crash', reason='crash')
                break
            if tag == 'START':
                _need(previous is None)
                _digest(payload.get('digest'))
                if dialect == 'r22d75':
                    _counts(payload.get('n6_n4_n2'))
                else:
                    _target_fields(payload)
                previous = parsed['start'] = payload
                if deciding and dialect == 'h4' and payload['tau'] is None:
                    parsed.update(status='no_target')
                    break
            elif tag == 'STEP':
                _need(parsed['end'] is None)
                step = _normalized_step(dialect, previous, payload)
                _need(type(payload.get('step')) is int and payload['step'] == len(parsed['steps']) + 1,
                      'missing_step')
                parsed['steps'].append(step)
                previous = payload
                if deciding:
                    parsed['status'] = 'step'
                    break
            else:
                _need(parsed['end'] is None)
                if tag != 'NOT_APPLICABLE':
                    _need(_integer(payload.get('steps')))
                _details(payload)
                parsed.update(end={'tag': tag, 'payload': payload}, status='end')
    except OutputError as exc:
        parsed.update(status={'digest_mismatch': 'mismatch', 'missing_step': 'missing_step'}
                      .get(exc.reason, 'malformed'), reason=exc.reason)
    return parsed


def parse_log(dialect, lines):
    """Parse all complete steps; each reads tau, u and counts from its predecessor."""
    return _parse_log(dialect, lines)


def _parse_shard(lines):
    parsed = {'context': None, 'firsts': [], 'end': None, 'status': 'truncated'}
    try:
        for tag, payload in _events('two_letter_u', lines):
            if tag == 'crash':
                parsed.update(status='crash', reason='crash')
                break
            if tag == 'CONTEXT':
                _need(parsed['context'] is None)
                _digest(payload.get('digest'))
                _target_fields(payload)
                _need(payload['tau'] is not None)
                for key in ('first_letter_caps', 'classA'):
                    value = payload.get(key)
                    _need(isinstance(value, list) and all(_integer(c, high=600) for c in value) and
                          value == sorted(set(value)))
                _need(set(payload['classA']) <= set(payload['first_letter_caps']))
                _need(_integer(payload.get('first_letters')) and _integer(payload.get('nshards'), 1) and
                      _integer(payload.get('shard'), high=payload['nshards']))
                parsed['context'] = payload
            elif tag == 'FIRST':
                _need(parsed['end'] is None)
                parsed['firsts'].append(payload)
            else:
                _need(parsed['end'] is None)
                _need(_integer(payload.get('first_letters_done')))
                parsed.update(end={'tag': tag, 'payload': payload}, status=tag.lower())
    except OutputError as exc:
        parsed.update(status='malformed', reason=exc.reason)
    return parsed


def _pair_choice(parsed):
    """Validate all shard evidence before applying the two ordered choice rules."""
    answer = {'result': 'undecided', 'outputs': [p['status'] for p in parsed]}
    for p in parsed:
        if p['status'] in ('malformed', 'crash'):
            raise OutputError(p.get('reason', p['status']))
    contexts = [p['context'] for p in parsed if p['context'] is not None]
    if not contexts:
        return answer
    context = contexts[0]
    keys = ('digest', 'tau', 'u', 'first_letter_caps', 'classA', 'first_letters', 'nshards')
    _need(all(all(c[k] == context[k] for k in keys) for c in contexts), 'inconsistent_output')
    caps, class_a = context['first_letter_caps'], context['classA']
    order = [[c, j] for c in class_a for j in range(1, 60)] + [
             [c, j] for c in caps if c not in class_a for j in range(1, 60)]
    _need(len(order) == context['first_letters'], 'inconsistent_output')
    indexed = {}
    for p in parsed:
        c = p['context']
        if c is None:
            continue
        if p['status'] == 'done':
            _need(p['end']['payload']['first_letters_done'] == len(p['firsts']), 'inconsistent_output')
        for row in p['firsts']:
            index = row.get('index')
            _need(_integer(index, high=len(order)))
            _letter(row.get('first'))
            _need(index % c['nshards'] == c['shard'] and row.get('first') == order[index] and
                  row.get('cls') == ('A' if order[index][0] in class_a else 'B'), 'inconsistent_output')
            _need(type(row.get('valid')) is bool)
            if row['valid']:
                _need(_integer(row.get('u_t')))
                freed, successes = row.get('freed'), row.get('successes')
                _need((isinstance(freed, list) and all(_integer(x, high=600) for x in freed))
                      if row['cls'] == 'A' else freed is None)
                _need(isinstance(successes, list))
                second = []
                for success in successes:
                    _need(isinstance(success, list) and len(success) == 3)
                    _letter(success[:2])
                    _need(_integer(success[2]) and success[2] < context['u'])
                    second.append(success[:2])
                _need(second == sorted(second) and len({tuple(x) for x in second}) == len(second))
            if index in indexed:
                _need(indexed[index] == row, 'inconsistent_output')
            indexed[index] = row
    a_indices = [i for i, letter in enumerate(order) if letter[0] in class_a]
    wins = [i for i, row in indexed.items() if row['valid'] and row['successes']]
    freeing = [i for i in wins if indexed[i]['cls'] == 'A' and indexed[i]['freed']]
    if freeing:
        chosen = min(freeing, key=lambda i: order[i])
        needed = [i for i in a_indices if order[i] < order[chosen]]
        by = 'freeing'
    elif all(i in indexed for i in a_indices):
        if not wins:
            if len(indexed) == len(order):
                answer['result'] = 'exhausted'
            return answer
        chosen = min(wins, key=lambda i: order[i])
        needed = [i for i, letter in enumerate(order) if letter < order[chosen]]
        by = 'first_success'
    else:
        return answer
    if not all(i in indexed for i in needed):
        return answer
    row = indexed[chosen]
    success = row['successes'][0]
    fields = dict(tau_before=context['tau'], tau_after=context['tau'], u_before=context['u'],
                  u_after=success[2], u_mid=row['u_t'],
                  rule_script=RULE_SOURCES['two_letter_u'].as_posix(),
                  pair_choice={'order_index': chosen, 'class': row['cls'], 'freed': row['freed'],
                               'by': by, 'shards': context['nshards'], 'first_letters': len(order)})
    return dict(answer, result='step', step={'rule': 'h4/pair', 'letters': [row['first'], success[:2]],
                'input_digest': context['digest'], 'fields': fields})


def choose_pair(outputs):
    """Choose from complete FIRST lines, retaining each file's end."""
    parsed = [_parse_shard(lines) for lines in outputs]
    try:
        return _pair_choice(parsed)
    except OutputError as exc:
        return {'result': 'error', 'reason': exc.reason, 'outputs': [p['status'] for p in parsed]}


def _stop_when(name, line):
    if line.startswith('STEP '):
        return True
    if name == 'h4' and line.startswith('START '):
        try:
            value = json.loads(line[6:])
            return isinstance(value, dict) and 'tau' in value and value['tau'] is None
        except (ValueError, TypeError, RecursionError):
            pass
    return False


def _call(ctx, snap, path, name, extra=(), shard=None, expected=None, *, stop_when=None):
    cpu = float(ctx.child_clock('height_cpu'))
    tag = f'{ctx.work_dir.name}-{name}' + (f'-shard{shard}' if shard is not None else '')
    argv = [sys.executable, '-I', '-B', '-u', str(ROOT / RULE_SCRIPTS[name]),
            '--snapshot', str(path), '--expect', snap['digest'], '--cpu', str(cpu), *map(str, extra)]
    child = ctx.execute(argv, tag, ctx.wall('height_seconds'),
                        stop_when=stop_when if shard is not None else lambda line: _stop_when(name, line))
    attempt = dict(script=name, tag=tag, shard=shard, outcome=None, reason=None, rc=child.rc,
                   seconds=float(child.seconds), stopped=bool(child.stopped), timed_out=False)
    call = {'parsed': None, 'attempt': attempt, 'result': None, 'reason': None, 'details': {}}

    def finish(outcome, reason, result=None, details=None):
        attempt.update(outcome=outcome, reason=reason,
                       timed_out=bool(child.timeout or outcome in ('budget', 'timeout')))
        call.update(result=result, reason=reason, details=details or {})
        return call

    if child.cancelled:
        return finish('cancelled', 'cancelled', 'error')
    try:
        lines = child.log.read_text(encoding='utf-8', errors='replace').splitlines(keepends=True)
    except OSError:
        lines = []
    parsed = call['parsed'] = (_parse_shard(lines) if shard is not None else
                               _parse_log(name, lines, deciding=True))
    status = parsed['status']
    try:
        start = parsed.get('context') if shard is not None else parsed['start']
        if start is not None:
            _need(start['digest'] == snap['digest'],
                  'inconsistent_output' if shard is not None else 'digest_mismatch')
            if expected:
                _need(all(start.get(k) == v for k, v in expected.items()), 'inconsistent_output')
        if shard is None and status in ('step', 'no_target'):
            return finish('step' if status == 'step' else 'stuck',
                          'step' if status == 'step' else 'no_target')
        if status in ('malformed', 'mismatch', 'missing_step', 'crash'):
            return finish(status, parsed['reason'], 'error')
        if shard is not None and status == 'done' and start is not None:
            indices = [row.get('index') for row in parsed['firsts']]
            _need(all(_integer(index, high=start['first_letters']) for index in indices))
            _need(parsed['end']['payload']['first_letters_done'] == len(indices) and
                  set(indices) == set(range(shard, start['first_letters'], start['nshards'])),
                  'inconsistent_output')
        if child.timeout:
            return finish('timeout', 'wall_clock', 'timeout')
        if shard is not None:
            if status in ('done', 'budget') and child.rc == 0 and start is not None:
                return finish(status, 'cpu_budget' if status == 'budget' else 'done',
                              'budget' if status == 'budget' else None)
        elif status == 'end' and child.rc == 0:
            end = parsed['end']
            payload, end_tag = end['payload'], end['tag']
            if end_tag == 'NOT_APPLICABLE':
                _need(_integer(payload.get('max_height')) and 'digest' in payload)
                return finish('not_applicable', 'invariant_height_above_6', 'error',
                              {k: payload[k] for k in ('max_height', 'digest')})
            steps = payload['steps']
            pending = payload.get('played_unreported')
            exception = end_tag == 'BUDGET' and steps == 1 and (
                (name == 'r22d81' and isinstance(pending, dict) and pending.get('step') == 1 and
                 type(pending['step']) is int and pending.get('family') == 'H6') or
                (name == 'h4' and payload.get('families') == {'L4': 1} and
                 type(payload['families']['L4']) is int))
            if end_tag != 'BUDGET' or exception:
                _need(start is not None, 'truncated')
            if steps != 0 and not exception:
                return finish('missing_step', 'missing_step', 'error')
            if steps == 0 and 'digest' in payload:
                _need(payload['digest'] == snap['digest'], 'digest_mismatch')
            if steps == 0 and start is not None and name != 'r22d75':
                _target_fields(payload)
                _need(payload['tau'] == start['tau'] and payload['u'] == start['u'],
                      'inconsistent_output')
                if name == 'r22d81_pairs':
                    _need(payload['tau'] == expected['tau'], 'inconsistent_output')
            details = _details(payload)
            if end_tag in ('HEIGHT2', 'HANDOFF'):
                return finish('handoff', 'handoff', 'handoff', details)
            if end_tag == 'BUDGET':
                return finish('budget', 'cpu_budget', 'budget', details)
            if end_tag == 'UNLOCKED':
                return finish('unlocked', 'unlocked_without_step', 'stuck', details)
            return finish('stuck', 'no_step', 'stuck', details)
        if child.rc == -signal.SIGXCPU and child.seconds >= cpu:
            return finish('budget', 'cpu_rlimit', 'budget')
        if status == 'crash' or child.rc != 0:
            return finish('crash', 'crash', 'error')
        return finish('truncated', 'truncated', 'error')
    except OutputError as exc:
        return finish('mismatch' if exc.reason in ('digest_mismatch', 'inconsistent_output') else 'malformed',
                      exc.reason, 'error')


def _proposal(ctx, snap, step, name, attempts):
    letters, rule = step['letters'], step['rule']
    tree = api.flat(rule, [('first letter', 1), ('second letter', 1)]) if len(letters) == 2 else None
    return api.Proposal('height', ctx.subphase, rule,
                        {'word': letters, 'input_digest': snap['digest'],
                         'output_digest': step['output_digest']}, rule, RULE_SOURCES[name].stem,
                        tree=tree, fields=step['fields'], attempts=attempts)


def pair_end_digest(ctx, letters):
    """Replay only on a fresh table copy; never use the live state as scratch."""
    m = ctx.mechanics
    copy = m.load(ctx.table)
    _need(copy.digest() == ctx.state.digest(), 'digest_mismatch')
    for c, i in letters:
        outcome = copy.apply(m.sim.Twist(m.restore.ctx, c, m.restore.element(m.menu, c, i),
                                        'menu', {'menu': m.menu.name}))
        _need(outcome.applied and not outcome.uncertain)
    return _digest(copy.digest())


def _shards(ctx, snap, path, stuck, attempts):
    n = ctx.budgets['pair_shards']
    event = threading.Event()
    calls, first_error = [None] * n, None
    with ThreadPoolExecutor(max_workers=n) as pool:
        try:
            futures = {pool.submit(_call, ctx, snap, path, 'two_letter_u',
                                  ('--shard', i, '--nshards', n), i,
                                  {'tau': stuck['tau'], 'u': stuck['u'], 'shard': i, 'nshards': n},
                                  stop_when=lambda line: event.is_set()): i for i in range(n)}
            for future in as_completed(futures):
                call = calls[futures[future]] = future.result()
                if call['result'] == 'error' and first_error is None:
                    first_error = call
                    event.set()
        except BaseException:
            event.set()
            raise
        finally:
            event.set()
    attempts.extend(call['attempt'] for call in calls)
    if first_error is not None:
        return api.End('error', first_error['reason'], first_error['details'], attempts)
    try:
        parsed = [call['parsed'] for call in calls]
        chosen = _pair_choice(parsed)
        if chosen['result'] == 'step':
            step = chosen['step']
            _need(ctx.state.digest() == snap['digest'], 'digest_mismatch')
            step['output_digest'] = pair_end_digest(ctx, step['letters'])
            return _proposal(ctx, snap, step, 'two_letter_u', attempts)
        if chosen['result'] == 'exhausted' and all(c['attempt']['outcome'] == 'done' for c in calls):
            return api.End('stuck', 'pair_exhausted', attempts=attempts)
        if any(call['result'] == 'timeout' for call in calls):
            return api.End('timeout', 'wall_clock', attempts=attempts)
        if any(call['attempt']['outcome'] == 'budget' for call in calls):
            reason = 'cpu_rlimit' if any(c['reason'] == 'cpu_rlimit' for c in calls) else 'cpu_budget'
            return api.End('budget', reason, attempts=attempts)
        return api.End('error', 'inconsistent_output', attempts=attempts)
    except (OutputError, ValueError) as exc:
        return api.End('error', exc.reason if isinstance(exc, OutputError) else 'malformed_output',
                       attempts=attempts)


def next_step(ctx):
    """Return one Proposal or End, preserving every child attempt on errors."""
    sub = ctx.subphase
    if sub not in ('r22d75', 'r22d81', 'h4'):
        raise ValueError('unknown height subphase')
    snap = ctx.snapshot()
    limit = 4 if sub == 'h4' else 6
    if max((int(h) for h, count in snap['heights'].items() if count), default=0) > limit:
        return api.End('error', f'invariant_height_above_{limit}')
    path, attempts = ctx.work_dir / 'state.pkl', []
    try:
        write_snapshot(path, ctx.state, snap['digest'], snap['off'])
        call = _call(ctx, snap, path, sub)
        attempts.append(call['attempt'])
        parsed = call['parsed']
        if call['attempt']['outcome'] == 'step':
            return _proposal(ctx, snap, parsed['steps'][0], sub, attempts)
        if parsed is not None and parsed['status'] == 'no_target' and call['result'] is None:
            return api.End('stuck', 'no_target', {'note': NO_TARGET_NOTE}, attempts)
        if call['result'] != 'stuck' or call['attempt']['outcome'] != 'stuck' or sub == 'r22d75':
            return api.End(call['result'], call['reason'], call['details'], attempts)
        stuck = parsed['end']['payload']
        try:
            _target_fields(stuck)
            if stuck['tau'] is None or (sub == 'h4' and not stuck.get('targets')):
                return api.End('stuck', 'no_target', call['details'], attempts)
            if sub == 'h4':
                _details({'targets': stuck['targets']})
                return _shards(ctx, snap, path, stuck, attempts)
            call = _call(ctx, snap, path, 'r22d81_pairs', ('--target', stuck['tau']),
                         expected={'tau': stuck['tau'], 'u': stuck['u']})
            attempts.append(call['attempt'])
            if call['attempt']['outcome'] == 'step':
                return _proposal(ctx, snap, call['parsed']['steps'][0], 'r22d81_pairs', attempts)
            return api.End(call['result'], call['reason'], call['details'], attempts)
        except OutputError as exc:
            return api.End('error', exc.reason, attempts=attempts)
    finally:
        path.unlink(missing_ok=True)
