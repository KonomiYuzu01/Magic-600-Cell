"""Experimental human-directed workbench; mechanics remain in the existing Session."""
from collections import OrderedDict
from pathlib import Path
from urllib.parse import urlparse, parse_qs
import atexit
import base64
import copy
import json
import os
import secrets
import sys
import threading
import time
import numpy as np
from core import PuzzleState, Filters, ORDER, canonical, digest, invrecipe, invword
from grips import grips
from draft_inspection import inspect_phase, BOUNDARIES
from cycle_projection import inspect_cycles, validate_cycle_display, inspect_cycle_display
from filter_projection import PieceFilterProjection
from grip_frames import frames as cap_frames, resolve as resolve_frame, notation as frame_notation, rotation_table
from keymap_catalog import (commands as bank_commands, shared_banks, keymap_command_ids,
                            export_keymap, inspect_keymap, import_keymap)
from mathematical_names import MathematicalNames, NAMING_VERSION
from transported_frames import TransportedFrames, FRAME_VERSION
from local_geometry import geometry as sticker_geometry, cell_state
from work_sheets import build_sheet, inspect_sheet
from macro_library import edit_metadata, facets, export_selected, inspect_import
from macro_use import PROOF_VERSION, classify_effect, scoped_stars, infer_use, use_context
from residuals import derive_residual, derive_delta
from work_intents import normalize_goal, evaluate_goal
from current_recommendation import score_current
from position_requirements import mode as requirement_mode, prepare as prepare_requirement, holds as requirement_holds
from reference_variants import ReferenceVariants
from endgame_library import EndgameLibrary
from endgame_invariants import load_audited_invariants
from workflow_continuity import switch_context, plan_commit_continuity
from candidate_analysis import analyse_candidates
from session_workflow import SessionWorkflow, MAX_LOG_BASE64_BYTES
from enhanced import Workflow

HERE = Path(__file__).resolve().parent
BUILD = 'PostApproval-20260916'
PHASES = ['prepare', 'macro', 'cleanup']
PURPOSES = {'A': 'Buffer A preparation', 'B': 'Buffer B preparation',
            'I': 'Insertion / block extension', 'M': 'Macro composition / recording',
            'E': 'Buffer / orientation residual work'}
GRIP_KEYS = ['Digit' + str(x) for x in [1, 2, 3, 4, 5, 6, 7, 8, 9, 0]] + ['Key' + x for x in 'QWERTYUIOP']
EFFECT_KINDS = ('star', 'pure-position', 'pure-orientation', 'mixed', 'pure-piece-cycle', 'single-three-cycle', 'orientation', 'cross-orbit', 'identity', 'unchecked')
EFFECT_PROOF_VERSION = PROOF_VERSION


class _BackendTiming:
    """Opt-in, bounded worker timings; only flushes at process shutdown."""
    def __init__(self, path):
        self.path = Path(path)
        self.rows, self.sequence, self.dropped = [], 0, 0
        self.local = threading.local()
        self.flushed = False
        atexit.register(self.flush)

    def wrap(self, call, route, action):
        self.sequence += 1
        if self.sequence > 256:
            self.dropped += 1
            return call
        sequence, submitted = self.sequence, time.perf_counter_ns()
        action = action if isinstance(action, str) and len(action) <= 64 else '<invalid>'
        def worker():
            started = time.perf_counter_ns()
            row = dict(sequence=sequence, route=route, action=action,
                queue_ns=started-submitted, outcome='ok', spans=[], _start=started, _depth=0)
            self.local.row = row
            try:
                return call()
            except BaseException as error:
                row['outcome'] = type(error).__name__
                raise
            finally:
                row['worker_ns'] = time.perf_counter_ns()-started
                row.pop('_start'); row.pop('_depth'); row.pop('_locked', None)
                self.local.row = None
                self.rows.append(row)
        return worker

    def locked(self, session):
        row = getattr(self.local, 'row', None)
        if row is None:
            return
        row['_locked'] = time.perf_counter_ns()
        row['lock_wait_ns'] = row['_locked']-row['_start']
        row['before'] = dict(head=session.head, revision=session.rev)

    def unlocked(self, session):
        row = getattr(self.local, 'row', None)
        if row is None:
            return
        row['locked_ns'] = time.perf_counter_ns()-row['_locked']
        row['after'] = dict(head=session.head, revision=session.rev)

    def measure(self, name, call, *args, **kwargs):
        row = getattr(self.local, 'row', None)
        if row is None:
            return call(*args, **kwargs)
        started = time.perf_counter_ns()
        span = dict(name=name, depth=row['_depth'], start_ns=started-row['_start'], outcome='ok')
        if name in ('native_reply', 'reply.snapshot'):
            span['prediction'] = bool(kwargs.get('prediction', True))
        row['spans'].append(span)
        row['_depth'] += 1
        try:
            result = call(*args, **kwargs)
            if name == 'reply.native_snapshot' and isinstance(result, dict):
                span['mode'] = result.get('mode')
            return result
        except BaseException as error:
            span['outcome'] = type(error).__name__
            raise
        finally:
            span['ns'] = time.perf_counter_ns()-started
            row['_depth'] -= 1

    def flush(self):
        if self.flushed:
            return
        self.flushed = True
        try:
            with self.path.open('x', encoding='utf-8') as stream:
                stream.write(json.dumps(dict(format='magic600-backend-timing-v1',
                    pid=os.getpid(), requests=len(self.rows), dropped=self.dropped)) + '\n')
                for row in self.rows:
                    stream.write(json.dumps(row) + '\n')
        except OSError as error:
            print('Backend timing output unavailable: ' + type(error).__name__, file=sys.stderr)


class _NativeInspectionReuse:
    """One detached result per inspector; caller holds the Session lock."""
    def __init__(self):
        self.entries = {}

    def read(self, kind, inspect, wb, parameter, macro_id):
        wb.m.check_cancel()
        # Let the existing residual validator inspect every published bundle.
        if kind == 'cycles' and (wb.residual_cache is not None or wb.transported_frames is None):
            return inspect(wb, parameter, macro_id)
        bindings = (inspect, wb.m, wb.names, wb.transported_frames, wb.invariants)
        key = canonical(dict(guard=wb.guard(),
            workspace={k: v for k, v in wb.w.items() if k not in ('bank', 'previous_bank', 'view')},
            preferences={k: v for k, v in wb.s.prefs.items() if k != 'layout'},
            filter_context=PieceFilterProjection(wb).context_hash,
            pending_public=None if wb.s.pending is None else wb.s.pending['public'],
            parameter=parameter, macro_id=macro_id, macro=wb.library.get(macro_id)))
        previous = self.entries.get(kind)
        if (previous is not None and previous[1] == key
                and all(a is b for a, b in zip(previous[0], bindings))):
            result = copy.deepcopy(previous[2])
            wb.m.check_cancel()
            return result
        result = inspect(wb, parameter, macro_id)
        wb.m.check_cancel()
        # Unavailable invariant evidence may become available on a later read.
        if kind == 'cycles' and parameter.get('mode', 'Operation') != 'Operation' and wb.invariants is None:
            return result
        retained = copy.deepcopy(result)
        wb.m.check_cancel()
        self.entries[kind] = (bindings, key, retained)
        return result


def integer(value, limit, name):
    if type(value) is not int or not 0 <= value < limit:
        raise ValueError(name + ' is outside the valid integer range')
    return value


def clone(value):
    return json.loads(canonical(value))


def canonical_cell(value):
    if type(value) is not int or not 1 <= value <= 600:
        raise ValueError('Canonical cell must be an integer in C1..C600')
    return value


def workspace_defaults(workspace):
    """Add presentation/work fields without rewriting canonical saved identities."""
    workspace.setdefault('inspected_position', None)
    workspace.setdefault('grip_frames', {})
    workspace.setdefault('goal', 'insert')
    workspace.setdefault('target_requirement', None)
    workspace.setdefault('draft_sources', {p: [] for p in PHASES})
    if not isinstance(workspace.get('operation_generation'), str) or not workspace['operation_generation']:
        workspace['operation_generation'] = secrets.token_hex(12)
    for context in workspace.get('contexts', {}).values():
        context.setdefault('goal', 'insert')
        context.setdefault('target_requirement', None)
        context.setdefault('draft_sources', {p: [] for p in PHASES})
        if not isinstance(context.get('operation_generation'), str) or not context['operation_generation']:
            context['operation_generation'] = secrets.token_hex(12)
    return workspace


class Workbench:
    def __init__(self, session, lock, workflow=None, native_profile=None):
        self.s, self.m, self.lock = session, session.m, lock
        self.native_profile = native_profile if native_profile is not None else lambda: None
        self.session_workflow = SessionWorkflow(session, workflow if workflow is not None else Workflow(session))
        self.epoch = secrets.token_hex(12)
        self.save_warnings = []
        self.review = None
        self.execution = None
        self.sheet_confirmation = None
        self.residual_cache = None
        self.completed_operations = {}
        self.effects = OrderedDict()
        self.effect_facts = OrderedDict()
        self.effect_facts_bytes = 0
        self.grip_cache = {}
        self.frame_cache = {}
        self.frame_notation = frame_notation(self.m)
        self.names = MathematicalNames(self.m)
        self.transported_frames = TransportedFrames(self.m)
        self.reference_variants = ReferenceVariants(self.m)
        self.endgames = EndgameLibrary(self.m, self.transported_frames)
        self.invariants = None
        self.candidate_cache = None
        self.library = self.default_library()
        saved = session.prefs.get('layout', {}).get('magic600_experiment')
        self.w = clone(saved) if saved else self.new_workspace()
        if not saved and session.prefs['rules'] == [dict(expr='all', style='solid')]:
            self.w['filter'] = 'all'
        if self.w['model'] != self.m.model_id:
            raise ValueError('Experimental workspace belongs to a different model')
        workspace_defaults(self.w)
        self.library.update(self.w.get('personal_macros', {}))
        self.restore_completed_operation()
        self.s.save_prefs({'pin_safety': False})
        self.save()

    def new_workspace(self):
        return workspace_defaults(dict(model=self.m.model_id, orbit=33, current=None,
            target=None, next=None, inspected=None, bank='33-I', previous_bank='33-A',
            contexts={}, block=dict(name='Working block', members=[], protected=[]),
            reference=[], roles=list(map(int, self.m.trees[33]['buffers'])),
            draft={p: [] for p in PHASES}, phase='prepare', input='draft',
            prefix=False, captures={}, bank_overrides={}, templates={}, personal_macros={},
            keybinds={}, filter='active', saved_filters={}, source='New solved experiment',
            view=dict(tab='block', local=True, global_view=False, local_center=1), version=1))

    def default_library(self):
        result = {}
        for o in range(35):
            for sign, suffix in [(1, 'forward'), (-1, 'inverse')]:
                key = f'o{o}-n0-{suffix}'
                result[key] = dict(id=key, name=f'O{o:02} · retained star 0 · {suffix}',
                    recipe=[dict(kind='star', orbit=o, node=0, sign=sign)],
                    orbit=o, tags=['retained star', suffix], note='Fixed explicit library entry; never retargeted.', version=1)
        for sign, suffix in [(1, 'forward'), (-1, 'inverse')]:
            key = f'o33-n11-{suffix}'
            result[key] = dict(id=key, name=f'O33 · retained star 11 · {suffix}',
                recipe=[dict(kind='star', orbit=33, node=11, sign=sign)], orbit=33,
                tags=['retained star', suffix], note='E1 adjacent Home-block example.', version=1)
        return result

    def persist_prefs(self, changes):
        """Save preferences; a warning means they were stored but the reply failed."""
        previous_prefs = self.s.prefs
        try:
            self.s.save_prefs(changes)
        except Exception as error:
            # An acknowledgment can fail after SQLite has stored the new work.
            # Only an exact changed record proves that this save took effect.
            try:
                stored = json.loads(self.s._get('prefs') or '{}')
            except Exception:
                stored = None
            if stored != previous_prefs and stored == dict(previous_prefs, **changes):
                self.s.prefs = stored
                return 'Workspace preferences were saved; reading their result failed: ' + str(error)
            raise
        return None

    def workspace_layout(self, workspace):
        layout = dict(self.s.prefs.get('layout', {}))
        layout['magic600_experiment'] = clone(workspace)
        return layout

    def save(self):
        previous = self.s.prefs.get('layout', {}).get('magic600_experiment')
        try:
            return self.persist_prefs({'layout': self.workspace_layout(self.w)})
        except Exception:
            if previous is not None:
                self.w = clone(previous)
                workspace_defaults(self.w)
                self.library = self.default_library()
                self.library.update(self.w.get('personal_macros', {}))
            raise

    def save_window_layout(self, body):
        """Caller holds the Session lock; only detached window preferences change."""
        if not isinstance(body, dict) or set(body) != {'generation', 'windows'}:
            raise ValueError('Window layout requires only generation and windows')
        generation = body['generation']
        if type(generation) is not int or not 0 <= generation < 2**53:
            raise ValueError('Layout generation must be a nonnegative integer')
        windows = body['windows']
        known = {'local', 'global', 'keyboard', 'macro', 'operation', 'solve', 'puzzle'}
        if not isinstance(windows, dict) or not set(windows) <= known:
            raise ValueError('Window layout contains an unknown window')
        required = {'x', 'y', 'width', 'height', 'visible'}
        for name, record in windows.items():
            permitted = required | ({'detached'} if name == 'puzzle' else set())
            if not isinstance(record, dict) or not required <= set(record) <= permitted:
                raise ValueError(name + ' requires rectangle and visibility only')
            for key in ('x', 'y', 'width', 'height'):
                value = record[key]
                low, high = (-100000, 100000) if key in ('x', 'y') else (1, 32768)
                if type(value) is not int or not low <= value <= high:
                    raise ValueError(name + '.' + key + ' is outside the supported integer range')
            if type(record['visible']) is not bool or ('detached' in record and type(record['detached']) is not bool):
                raise ValueError(name + ' visibility and detached flags must be Boolean')
        # Validate the complete batch before touching the current authoritative work.
        existing = self.w['view'].get('windows', {})
        if not isinstance(existing, dict):
            raise ValueError('Saved window layout is not an object; it was not changed')
        merged = clone(existing)
        merged.update(clone(windows))
        self.w['view']['windows'] = merged
        warning = self.save()
        result = dict(saved=True, generation=generation)
        if warning:
            result['warning'] = warning
        return result

    def intent(self):
        return {k: self.w.get(k) for k in ['orbit', 'current', 'target', 'next', 'roles', 'reference', 'block', 'draft', 'draft_sources', 'prefix', 'goal', 'target_requirement']}

    def external_pending(self, draft):
        """Bind only additional pending mathematics, never the execution token."""
        pending = self.s.pending
        if pending is None:
            return None
        public = pending.get('public', {})
        result = dict(status='Unknown', model=public.get('model_id'),
            pre_state=public.get('pre_state'), head=pending.get('head'),
            revision=pending.get('rev'), recipe_hash=None)
        try:
            recipe = self.m.normalize(pending['recipe'])[0]
            normalized_draft = self.m.normalize(draft)[0] if draft else []
        except (KeyError, TypeError, ValueError):
            return result
        result['recipe_hash'] = digest(canonical(recipe).encode())
        fresh = (result['model'] == self.m.model_id and result['pre_state'] == self.s.st.hash
                 and result['head'] == self.s.head and result['revision'] == self.s.rev)
        # The same active draft already contributes its full operation support.
        # A new preview of retained executed steps is still an uncommitted action.
        if fresh and self.operation_state() == 'draft' and recipe == normalized_draft:
            return None
        result['status'] = 'Current' if fresh else 'Stale'
        return result

    def review_context(self):
        """One detached version binding; this does not certify or stage an action."""
        phases = self.w['draft']
        recipe = [step for phase in PHASES for step in phases[phase]]
        sources = {}
        for phase in PHASES:
            sources[phase] = [dict(id=item['id'], version=item['version'],
                start=item['start'], count=item['count'],
                recipe_hash=digest(canonical(item['recipe']).encode()))
                for item in self.w['draft_sources'][phase]]
        policy = dict(protected=self.s.prefs['protected'], prefix=self.w['prefix'],
                      position_locks=self.position_locks())
        result = dict(version='review-context-v1', model=self.m.model_id,
            naming_version=NAMING_VERSION, frame_version=FRAME_VERSION,
            epoch=self.epoch, head=self.s.head, revision=self.s.rev, state_hash=self.s.st.hash,
            orbit=self.w['orbit'], current=self.w['current'], locked_next=self.w['next'],
            target=self.w['target'], roles=self.w['roles'], reference=self.w['reference'],
            block=self.w['block'], goal=self.w['goal'],
            target_requirement=self.w['target_requirement'], macro_sources=sources,
            recipe_hash=digest(canonical(recipe).encode()),
            phase_hashes={phase: digest(canonical(phases[phase]).encode()) for phase in PHASES},
            operation_state=self.operation_state(), external_pending=self.external_pending(recipe),
            protection_revision=digest(canonical(policy).encode()))
        result['id'] = digest(canonical(result).encode())
        return clone(result)

    def position_locks(self):
        # Work selection does not release an explicitly captured requirement.
        # The live block supersedes its potentially stale saved context copy.
        blocks = [(self.w['orbit'], self.w['block'])]
        blocks += [(int(o), context['block']) for o, context in self.w['contexts'].items()
                   if int(o) != self.w['orbit']]
        combined = {}
        for orbit, block in blocks:
            for member in block['protected']:
                prepare_requirement(self.m, member)
                key = (member['position'], requirement_mode(member.get('mode', 'exact')), tuple(member['labels']))
                combined.setdefault(key, set()).add(orbit)
        return [dict(position=position, mode=mode, labels=list(labels), orbit=int(self.m.oid[position]),
                     work_orbits=sorted(owners))
                for (position, mode, labels), owners in sorted(combined.items())]

    def guard(self, pending=True):
        result = dict(context=self.review_context()['id'])
        if pending:
            result['pending'] = self.s.pending['token'] if self.s.pending else None
        return digest(canonical(result).encode())

    def piece(self, identity=None, position=None, state=None):
        state = state or self.s.st
        if identity is not None:
            integer(identity, self.m.np, 'Piece identity')
            position = int(state.where[identity])
        if position is None:
            return None
        p = state.piece(integer(position, self.m.np, 'Position'))
        p['home_cells'] = [c + 1 for c in p['home_cells']]
        p['current_cells'] = [c + 1 for c in p['current_cells']]
        p['cap_cells'] = [c + 1 for c in p['cap_cells']]
        p['orientation_group'] = self.m.census['orbits'][p['orbit']]['orientation_group'] if p['orbit'] >= 0 else 'fixed'
        p['names'] = self.names.piece_record(p)
        return p

    def banks(self):
        result = []
        for o in range(35):
            a, b = map(int, self.m.trees[o]['buffers'])
            for suffix, purpose in PURPOSES.items():
                key = f'{o}-{suffix}'
                positions = [b] if suffix == 'B' else [a] if suffix == 'A' else [a, b]
                cells = list(dict.fromkeys(c + 1 for p in positions for c in self.m.caps(p)))
                capture = self.w['captures'].get(key)
                slots = capture or ([] if suffix == 'I' else cells[:20])
                result.append(dict(id=key, orbit=o, purpose=purpose,
                    name=self.w['bank_overrides'].get(key, {}).get('name', purpose),
                    buffers=[a, b], caps=cells, slots=slots,
                    frame='Explicit ordered tetrahedral cap frame', needs_capture=suffix == 'I' and not capture,
                    frames={str(c): self.selected_frame(key, c) for c in slots} if key == self.w['bank'] else {},
                    frame_bases={str(c): self.frame_choices(c)['base_vertices'] for c in slots} if key == self.w['bank'] else {},
                    turns_enabled=True, commands=bank_commands(suffix),
                    macros=[f'o{o}-n0-inverse', f'o{o}-n0-forward'],
                    requirements='Explicit Current/Target capture required' if suffix == 'I' and not capture else 'Fixed cap mapping; select a macro explicitly'))
        return result + shared_banks()

    def frame_choices(self, cell):
        canonical_cell(cell)
        if cell not in self.frame_cache:
            self.frame_cache[cell] = cap_frames(self.m, cell)
        return clone(self.frame_cache[cell])

    def selected_frame(self, bank, cell):
        return clone(self.w.get('grip_frames', {}).get(bank, {}).get(str(cell),
                          self.frame_choices(cell)['base_vertices']))

    def cap_axes(self, cell):
        canonical_cell(cell)
        if cell not in self.grip_cache:
            axes = grips(self.m, cell - 1, rotations=rotation_table(self.m))
            # Keep the retained grips() legacy `cell` field zero-based.
            # The explicit names remove ambiguity at the experimental boundary.
            for record in [axes] + axes['axes'] + [axis['inverse'] for axis in axes['axes']]:
                record.update(lab_cell_index=cell - 1, canonical_cell=cell)
            self.grip_cache[cell] = axes
        return self.grip_cache[cell]

    def concrete(self):
        parts = clone(self.w['draft'])
        result = [step for phase in PHASES for step in parts[phase]]
        if not result:
            raise ValueError('The draft is empty. Select an existing macro or enter explicit turns.')
        return self.m.normalize(result)[0]

    def restore_completed_operation(self):
        # One explicit source/generation receipt, on the current journal ancestry.
        # Recipe equality alone never attributes a legacy event to a work context.
        self.completed_operations.clear()
        contexts = [(self.w['orbit'], self.w)]
        contexts += [(int(o), context) for o, context in self.w['contexts'].items()
                     if int(o) != self.w['orbit']]
        pending = {}
        for orbit, context in contexts:
            marker = context.get('completed_operation')
            if (not isinstance(marker, dict) or marker.get('generation') != context.get('operation_generation')
                    or not isinstance(marker.get('generation'), str) or not marker['generation']):
                continue
            recipe = [step for phase in PHASES for step in context['draft'][phase]]
            if recipe:
                pending[orbit] = (marker, self.m.normalize(recipe)[0])
        head, seen = self.s.head, set()
        while pending and head not in seen:
            self.m.check_cancel(); seen.add(head)
            event = self.s.event(head)
            if event is None:
                break
            if event['assistance'] in ('manual-work-sheet', 'recorded-scramble'):
                raw = self.s._get('event:' + str(head) + ':workflow')
                receipt = json.loads(raw) if raw is not None else None
                if isinstance(receipt, dict) and receipt.get('version') == 'workflow-transition-v1':
                    orbit = receipt.get('source_orbit')
                    if type(orbit) is int and orbit in pending:
                        marker, recipe = pending[orbit]
                        if (receipt.get('model') == self.m.model_id
                                and receipt.get('source_operation') == marker['generation']
                                and receipt.get('pre_state') == marker.get('pre') == event['pre']
                                and receipt.get('post_state') == marker.get('post') == event['post']
                                and marker.get('parent') == event['parent']
                                and json.loads(event['recipe']) == recipe):
                            self.completed_operations[orbit] = head
                            del pending[orbit]
            if event['parent'] is None:
                break
            head = event['parent']

    def operation_state(self):
        marker = self.w.get('completed_operation')
        return 'executed' if (self.w['orbit'] in self.completed_operations and isinstance(marker, dict)
            and marker.get('generation') == self.w.get('operation_generation')) else 'draft'

    def commit_with_result_warning(self, token, recipe, assistance, *, retained_draft=None):
        if (not self.s.pending or not secrets.compare_digest(str(token), self.s.pending['token'])
                or self.s.pending['recipe'] != self.m.normalize(recipe)[0]):
            raise ValueError('The pending operation no longer matches the explicit commit')
        before_head, before_hash = self.s.head, self.s.st.hash
        source_orbit = self.w['orbit']
        predicted = PuzzleState(self.m, self.s.pending['after'], trusted=True)
        source = 'live' if assistance == 'manual-grip-experiment' else 'work-sheet'
        context = self.review_context()
        # This exact token is being committed, not an unrelated outstanding action.
        context['external_pending'] = None
        context.pop('id', None); context['id'] = digest(canonical(context).encode())
        completion_evidence = []
        for orbit in range(35):
            slots = self.m.ids[self.m.so == orbit]
            if np.all(self.s.st.labels[slots] == slots) or not np.all(predicted.labels[slots] == slots):
                continue
            roles = self.w['roles'] if orbit == source_orbit else list(map(int, self.m.trees[orbit]['buffers']))
            scoped = clone(context); scoped.update(orbit=orbit, roles=roles)
            scoped.pop('id', None); scoped['id'] = digest(canonical(scoped).encode())
            bindings = dict(roles=roles, goal=self.w['goal'], prefix=self.w['prefix'],
                protected_requirements=dict(orbits=self.s.prefs['protected'], positions=self.position_locks()),
                operation=dict(status='Known', context_id=scoped['id'], recipe_hash=scoped['recipe_hash'],
                               affected_orbits=[r['orbit'] for r in self.s.pending['public']['support']]))
            completion_evidence.append(derive_residual(self.m, predicted, orbit, bindings,
                self.transported_frames, scoped, dict(kind='After', base_state_hash=before_hash,
                                                      recipe_hash=scoped['recipe_hash'])))
        planned = plan_commit_continuity(self.m, self.s.st, predicted, self.w, source=source,
                                         completion_evidence=completion_evidence)
        work = planned['workspace']
        # The retained source recipe is already executed, including after returning
        # from another orbit. The receipt is saved in the same journal transaction.
        if retained_draft is not None:
            work['draft'] = clone(retained_draft)
        if source == 'work-sheet':
            source_work = work if work['orbit'] == source_orbit else work['contexts'][str(source_orbit)]
            source_work['completed_operation'] = dict(generation=source_work['operation_generation'],
                parent=before_head, pre=before_hash, post=predicted.hash)
        planned['event_context']['residual_delta'] = self.compact_delta(self.s.st, predicted)
        completion = self.session_workflow.completion_plan(self.s.st, predicted, self.transported_frames)
        if completion is not None:
            planned['event_context']['session_completion'] = completion
        work['last_transition'] = planned['transition']
        work['orbit_suggestions'] = planned['suggestions']
        layout = clone(self.s.prefs.get('layout', {})); layout['magic600_experiment'] = work
        preferences = dict(layout=layout, orbit=work['orbit'],
            protected=sorted(set(self.s.prefs['protected']) | set(planned['protection_additions'])))
        expected_prefs = dict(clone(self.s.prefs), **preferences)
        warning = None
        try:
            self.s.commit(token, preference_changes=preferences, event_context=planned['event_context'])
        except Exception as error:
            # A status/read failure can follow a durable commit. Reconcile only
            # this exact next journal event; never infer success from an error.
            event = self.s.event()
            if (self.s.head == before_head or event is None or event['parent'] != before_head
                    or event['pre'] != before_hash or event['post'] != self.s.st.hash
                    or event['assistance'] != assistance
                    or json.loads(event['recipe']) != self.m.normalize(recipe)[0]
                    or self.s._get('event:' + str(self.s.head) + ':workflow') != canonical(planned['event_context'])
                    or self.s._get('prefs') != canonical(expected_prefs)
                    or canonical(self.s.prefs) != canonical(expected_prefs)):
                raise
            warning = 'Operation committed; reading its result failed: ' + str(error)
        self.w = clone(self.s.prefs['layout']['magic600_experiment'])
        if source == 'work-sheet':
            self.completed_operations[source_orbit] = self.s.head
        self.residual_cache = None
        return warning

    def compact_delta(self, before, after):
        bindings = dict(roles=self.w['roles'], goal=self.w['goal'], prefix=self.w['prefix'],
            protected_requirements=dict(orbits=self.s.prefs['protected'], positions=self.position_locks()))
        delta = derive_delta(self.m, before, after, self.w['orbit'], bindings)
        # Bounded journal metadata; exact endpoints and recipes remain the authority.
        changed = delta.pop('changed_positions')
        delta['changed_positions'] = changed[:32]; delta['changed_count'] = len(changed)
        delta['changed_truncated'] = len(changed) > 32
        delta['protected_damage'] = sum(r['newly_violated'] for r in delta.pop('protected_requirements')) + sum(
            len(r['positions']) for r in delta.pop('protected_orbit_changes'))
        return delta

    def journal_delta(self):
        event = self.s.event()
        if event is None or event['post'] != self.s.st.hash:
            return None
        raw = self.s._get('event:' + str(self.s.head) + ':workflow')
        if raw is None:
            return None
        receipt = json.loads(raw)
        if (receipt.get('model') != self.m.model_id or receipt.get('post_state') != event['post']
                or receipt.get('pre_state') != event['pre']):
            raise ValueError('Journal workflow receipt does not match its event')
        result = dict(head=self.s.head, source='Committed journal event', **receipt)
        for row in result.get('residual_delta', {}).get('changed_positions', []):
            row['position_name'] = self.names.address(row['position'])['short_name']
        return result

    def cached_candidates(self):
        result = self.candidate_cache
        if result is None or result['base_review_context_id'] != self.review_context()['id']:
            return None
        try:
            for row in result['candidates']:
                self.bound_macro(row['candidate'], True)
        except ValueError:
            return None
        return clone(result)

    def facts_key(self, normalized):
        # Exact content, not a hash or a library name, binds this local proof.
        return canonical(dict(model=self.m.model_id, proof_version=EFFECT_PROOF_VERSION,
                              frame_version=FRAME_VERSION, recipe=normalized))

    def remember_effect(self, normalized, facts):
        key = self.facts_key(normalized)
        summary = {name: clone(facts[name]) for name in ('id', 'primitives', 'slots', 'pieces', 'support',
            'cycle_count', 'fixed_orientation_count', 'star', 'scoped_stars', 'three_cycle_orbits', 'category')}
        summary.update(model=self.m.model_id, proof_version=EFFECT_PROOF_VERSION, summary=True,
            frame_version=facts['frame_version'], orientation_complete=facts['orientation_complete'],
            unchanged_orbits=clone(facts['unchanged_orbits']))
        summary['orbit_effects'] = clone(facts['orbit_effects'])
        # No full permutation, cycle list, slot pairs or duplicate recipe copy.
        size = len(key.encode('utf-8')) + len(canonical(summary).encode('utf-8'))
        if size > 4_000_000:
            return
        self.m.check_cancel()
        old = self.effect_facts.pop(key, None)
        if old is not None:
            self.effect_facts_bytes -= old[1]
        self.effect_facts[key] = (summary, size)
        self.effect_facts_bytes += size
        while len(self.effect_facts) > 256 or self.effect_facts_bytes > 4_000_000:
            _, (_, removed_size) = self.effect_facts.popitem(last=False)
            self.effect_facts_bytes -= removed_size

    def cached_effect(self, recipe):
        normalized, _ = self.m.normalize(recipe)
        cached = self.effect_facts.get(self.facts_key(normalized))
        return (clone(cached[0]) if cached and cached[0].get('model') == self.m.model_id
                and cached[0].get('proof_version') == EFFECT_PROOF_VERSION else None)

    def library_record(self, record, completed_orbits=None):
        facts = self.cached_effect(record['recipe'])
        if completed_orbits is None:
            completed_orbits = [r['orbit'] for r in self.s.st.progress() if r['solved'] == r['pieces']]
        kinds = [kind for kind in EFFECT_KINDS if facets(record, facts, dict(kind=kind))]
        scoped = []
        for row in facts['support'] if facts else []:
            orbit = row['orbit']
            scoped.append(dict(orbit=orbit, kinds=[kind for kind in EFFECT_KINDS
                if facets(record, facts, dict(kind=kind, affected_orbit=orbit))]))
        return dict(clone(record), effect=facts, classification=dict(kinds=kinds, scoped=scoped),
                    use=infer_use(facts), use_context=use_context(facts, self.w['orbit'],
                        self.s.prefs['protected'], completed_orbits),
                    tag_keys=[tag.casefold() for tag in record.get('tags', [])])

    def macro_query(self, query, text=''):
        facets({}, None, query)  # Validate even if no record could match.
        if not isinstance(text, str) or len(text) > 500:
            raise ValueError('Macro search text must contain at most 500 characters')
        text = text.casefold().strip()
        metadata = {key: value for key, value in query.items() if key in ('pinned', 'tags')}
        matching, unchecked = [], 0
        for record in self.library.values():
            words = ' '.join([record['id'], record['name'], record.get('note', '')] + record.get('tags', []))
            if text not in words.casefold():
                continue
            facts = self.cached_effect(record['recipe'])
            if (facts is None or not facts['orientation_complete']) and facets(record, None, metadata):
                unchecked += 1
            if facets(record, facts, query):
                matching.append(record['id'])
        return dict(query=clone(query), ids=matching, unchecked_count=unchecked,
                    tag_keys=[tag.casefold() for tag in query.get('tags', [])])

    def check_library(self, limit):
        if type(limit) is not int or not 1 <= limit <= 72:
            raise ValueError('Check 1..72 library entries in one cancellable batch')
        start = time.perf_counter()
        self.m.check_cancel()
        def complete(record):
            facts = self.cached_effect(record['recipe'])
            return facts is not None and facts['orientation_complete']
        missing = [r for r in self.library.values() if not complete(r)]
        checked_ids = []
        for record in missing[:limit]:
            self.m.check_cancel()
            self.effect(record['recipe'])
            self.m.check_cancel()
            checked_ids.append(record['id'])
        remaining = sum(not complete(r) for r in self.library.values())
        return dict(checked_ids=checked_ids, already_checked_count=len(self.library) - len(missing),
                    remaining_count=remaining, total_count=len(self.library),
                    seconds=time.perf_counter() - start,
                    scope='Read-only exact effects; no macro was selected or inserted')

    def bound_macro(self, value, binding_required=False):
        if isinstance(value, str) and not binding_required:
            identity = value
        elif isinstance(value, dict) and set(value) == {'id', 'version', 'recipe'}:
            identity = value['id']
            if not isinstance(identity, str) or type(value['version']) is not int or value['version'] < 1:
                raise ValueError('Macro binding needs an identity and positive content version')
        else:
            raise ValueError('Macro binding must contain its exact id, version and recipe')
        record = self.library.get(identity)
        if record is None:
            raise ValueError('Selected macro is missing: ' + identity)
        if isinstance(value, dict) and (value['version'] != record['version'] or
                canonical(value['recipe']) != canonical(record['recipe'])):
            raise ValueError('Selected macro content changed; inspect its current version and recipe again')
        return clone(record)

    def compare_macros(self, first, second):
        # Recompute complete actions. Imported provenance and hashes are not proofs.
        a = self.m.net(first['recipe']); b = self.m.net(second['recipe'])
        same = np.array_equal(a[0], b[0]) and np.array_equal(a[1], b[1])
        ia = np.argsort(a[1])
        inverse = np.array_equal(a[1][ia], b[0]) and np.array_equal(a[0][ia], b[1])
        local, inactive = [], []
        for orbit in range(35):
            am = self.m.so[a[0]] == orbit; bm = self.m.so[b[0]] == orbit
            if not np.any(am) and not np.any(bm):
                inactive.append(orbit)
            if np.array_equal(a[0][am], b[0][bm]) and np.array_equal(a[1][am], b[1][bm]):
                local.append(orbit)
        self.m.check_cancel()
        def side(record, action):
            return dict(record=clone(record), primitives=action[2],
                        affected_orbits=sorted(set(map(int, self.m.so[action[0]]))))
        return dict(model=self.m.model_id, a=side(first, a), b=side(second, b),
                    full_equal=bool(same), inverse=bool(inverse), locally_equal=local,
                    equal_active_orbits=[o for o in local if o not in inactive],
                    both_inactive_orbits=inactive, different_orbits=[o for o in range(35) if o not in local],
                    scope='Complete net action over all 259800 labelled slots', prefix_protection='not_checked',
                    note='Exact net comparison only. Original steps, costs and intermediate motion remain separate; review the complete operation for protection.')

    def create_macro_variant(self, body):
        source = self.bound_macro(body.get('source'), binding_required=True)
        kind = body.get('kind')
        record = edit_metadata(dict(orbit=source['orbit'], note='', tags=['personal'], pinned=False),
                               dict(name=body.get('name')))
        normalized, _ = self.m.normalize(source['recipe'])
        derived = dict(id=source['id'], version=source['version'], kind=kind)
        reference_proof = None
        if kind == 'geometry':
            reference_proof = self.reference_variants.compile(source, body.get('source_frame'),
                                                               body.get('destination_frame'))
            recipe = reference_proof['recipe']
            derived.update(reference=reference_proof['reference'], proof=reference_proof['proof'])
        elif kind == 'inverse':
            if 'reference' in body:
                raise ValueError('An inverse does not take a reference word')
            recipe = invrecipe(normalized)
        elif kind == 'reference':
            word = body.get('reference')
            if not isinstance(word, list) or not word:
                raise ValueError('Choose a nonempty explicit legal reference word')
            self.m.normalize([dict(kind='word', moves=word)])
            if canonical(word) != canonical(self.w['reference']):
                raise ValueError('The chosen reference word changed; inspect the current word again')
            recipe = [dict(kind='word', moves=invword(word))] + clone(source['recipe']) + [dict(kind='word', moves=clone(word))]
            derived.update(reference=clone(word), convention='R^-1 / Macro / R')
        else:
            raise ValueError('Choose inverse, explicit legal-word reference or verified geometric reference')
        self.m.normalize(recipe)  # Validate aggregate limits before publishing anything.
        key = 'user-' + secrets.token_hex(6)
        while key in self.library:
            key = 'user-' + secrets.token_hex(6)
        record.update(id=key, version=1, recipe=recipe, derived_from=derived)
        record = export_selected(self.m, {key: record}, [key])['entries'][0]
        comparison = None if kind == 'geometry' else self.compare_macros(source, record)
        self.m.check_cancel()
        previous_work, previous_library = self.w, self.library
        self.w = clone(self.w)
        self.w['personal_macros'][key] = record
        self.library = dict(self.library, **{key: record})
        try:
            warning = self.save()
        except Exception:
            self.w, self.library = previous_work, previous_library
            raise
        result = dict(created_id=key, record=clone(record), comparison=comparison)
        if reference_proof is not None:
            result['reference_proof'] = reference_proof
        if warning:
            result.update(warning=warning, saved=True)
        return result

    def endgame_choice(self, body):
        if (body.get('model', self.m.model_id) != self.m.model_id
                or body.get('frame_version', FRAME_VERSION) != FRAME_VERSION):
            raise ValueError('Endgame model or frame changed; inspect the finite parameters again')
        orbit = integer(body.get('orbit'), 35, 'Orbit')
        if orbit != self.w['orbit']:
            raise ValueError('Working orbit changed; choose this operation in its own orbit')
        result = self.endgames.compose(orbit, body.get('family'), body.get('x'), body.get('q'),
                                      body.get('y'), body.get('r'))
        result['roles_match'] = result['roles'] == self.w['roles']
        result['effect'] = self.effect(result['recipe'])[2]
        return result

    def save_endgame(self, body):
        proof = self.endgame_choice(body)
        key = 'user-' + secrets.token_hex(6)
        record = edit_metadata(dict(id=key, version=1, orbit=proof['orbit'], recipe=proof['recipe'],
            tags=['personal', 'retained endgame'], note='', pinned=False), dict(name=body.get('name')))
        record['derived_from'] = dict(kind='explicit-endgame', version=proof['version'],
            family=proof['family'], parameters=proof['parameters'], roles=proof['roles'],
            frame_version=proof['frame_version'])
        record = export_selected(self.m, {key: record}, [key])['entries'][0]
        self.m.check_cancel()
        old_work, old_library = self.w, self.library
        self.w = clone(self.w); self.library = dict(self.library)
        self.w['personal_macros'][key] = record; self.library[key] = record
        try:
            warning = self.save()
        except Exception:
            self.w, self.library = old_work, old_library
            raise
        return dict(created_id=key, record=clone(record), proof=proof, warning=warning)

    def invariant_service(self):
        if self.transported_frames is None:
            raise ValueError('Verified transported frames are unavailable')
        if self.invariants is None:
            self.invariants = load_audited_invariants(self.m, self.transported_frames)
        return self.invariants

    def effect(self, recipe):
        self.m.check_cancel()
        normalized, _ = self.m.normalize(recipe)
        key = digest(self.facts_key(normalized).encode())
        if (key in self.effects and self.effects[key][2]['recipe'] == normalized
                and self.effects[key][2]['orientation_complete']):
            self.effects.move_to_end(key)
            result = self.effects[key]
            self.remember_effect(normalized, result[2])
            return result[:3]
        src, dst, length, normalized = self.m.net(normalized)
        mapping = {int(a): int(b) for a, b in zip(self.m.sp[src], self.m.sp[dst])}
        cycles = []
        seen = set()
        for p in sorted(mapping):
            if p in seen or mapping[p] == p:
                continue
            cycle = []
            q = p
            while q not in seen:
                seen.add(q); cycle.append(q); q = mapping[q]
            cycles.append(dict(orbit=int(self.m.oid[p]), positions=cycle))
        fixed_orientation = sorted({int(p) for p in self.m.sp[src][self.m.sp[src] == self.m.sp[dst]]})
        orbit_effects = []
        for orbit in sorted(set(map(int, self.m.so[src]))):
            scoped = [c['positions'] for c in cycles if c['orbit'] == orbit]
            fixed = [p for p in fixed_orientation if int(self.m.oid[p]) == orbit]
            orbit_effects.append(dict(orbit=orbit, cycle_count=len(scoped),
                cycle_lengths=sorted(len(c) for c in scoped), fixed_orientation_count=len(fixed),
                single_three_cycle=len(scoped) == 1 and len(scoped[0]) == 3 and not fixed,
                contains_three_cycles=any(len(c) == 3 for c in scoped)))
        three_cycle_orbits = [o['orbit'] for o in orbit_effects if o['single_three_cycle']]
        classification, decomposition = classify_effect(self.transported_frames, src, dst)
        self.m.check_cancel()
        classified = {row['orbit']: row for row in classification['orbit_effects']}
        for row in orbit_effects:
            row.update(classified[row['orbit']])
        one_star = len(normalized) == 1 and normalized[0]['kind'] == 'star'
        star = None
        if one_star:
            step = normalized[0]; o = step['orbit']
            star = dict(orbit=o, node=step['node'], sign=step['sign'],
                a=int(self.m.trees[o]['buffers'][0]), b=int(self.m.trees[o]['buffers'][1]),
                target=int(self.m.trees[o]['positions'][step['node']]),
                frame=list(map(int, self.m.trees[o]['frames'][step['node']])),
                certificate=self.m.certify_seed(o))
        facts = dict(id=key, model=self.m.model_id, proof_version=EFFECT_PROOF_VERSION,
            frame_version=classification['frame_version'],
            orientation_complete=classification['orientation_complete'],
            unchanged_orbits=classification['unchanged_orbits'],
            recipe=normalized, primitives=length, slots=len(src),
            pieces=len(set(map(int, self.m.sp[src]))), support=self.m.support(src),
            cycles=cycles[:100], cycle_count=len(cycles), fixed_orientation=fixed_orientation[:100],
            fixed_orientation_count=len(fixed_orientation), star=star,
            scoped_stars=scoped_stars(self.m, src, dst, normalized),
            orbit_effects=orbit_effects, three_cycle_orbits=three_cycle_orbits,
            category='Verified retained star' if star else 'Identity' if len(src) == 0 else 'Orbit-local three-cycle' if three_cycle_orbits else 'Other exact effect',
            slot_pairs=list(zip(map(int, src[:160]), map(int, dst[:160]))),
            slot_pairs_truncated=len(src) > 160,
            action_hash=digest(src.astype('<i4').tobytes() + dst.astype('<i4').tobytes()))
        result = (src, dst, facts)
        self.m.check_cancel()
        self.remember_effect(normalized, facts)
        self.effects[key] = result + (decomposition,)
        while len(self.effects) > 40:
            self.effects.popitem(last=False)
        return result

    def snapshot(self, render=False, prediction=False):
        state = self.s.st
        progress = state.progress()
        completed_orbits = [r['orbit'] for r in progress if r['solved'] == r['pieces']]
        piece_filter = PieceFilterProjection(self)
        def piece(identity=None, position=None, state=state):
            return piece_filter.record(self.piece(identity=identity, position=position, state=state), state)
        r = self.review
        review = clone(r['public']) if r else None
        executable = bool(self.execution and self.execution['guard'] == self.guard(False)
                          and self.s.pending and self.execution['token'] == self.s.pending['token'])
        if review:
            if executable:
                review['status'] = 'Staged'
            elif r['guard'] != self.guard():
                review['status'] = 'Stale'
                if 'recommendation' in review:
                    review['recommendation'].update(status='Stale', category='NeedsVerification',
                        eligible=False, score=None, exact_score=None, reasons=[])
        current = piece(identity=self.w['current'])
        nxt = piece(identity=self.w['next']['identity']) if self.w['next'] else None
        members = []
        for member in self.w['block']['members']:
            identity, position = member['identity'], member['position']
            exact = requirement_holds(self.m, state.labels, prepare_requirement(self.m, member))
            members.append(piece_filter.record(dict(member, actual=int(state.at[position]), orbit=int(self.m.oid[position]), satisfied=exact,
                                current_cells=self.piece(position=position)['current_cells']), state))
        answer = dict(build=BUILD, model=self.m.model_id, epoch=self.epoch, head=self.s.head,
            review_context=self.review_context(),
            operation_state=self.operation_state(),
            revision=self.s.rev, hash=state.hash, guard=self.guard(), workspace=clone(self.w),
            session=self.session_workflow.report(),
            model_counts=dict(np=self.m.np, n=self.m.n, cells=600, vertices=120, moving_orbits=35),
            id_schema=dict(identity=dict(prefix='I', minimum=0, maximum=self.m.np - 1),
                position=dict(prefix='P', minimum=0, maximum=self.m.np - 1),
                cell=dict(prefix='C', minimum=1, maximum=600),
                slot=dict(prefix='S', minimum=0, maximum=self.m.n - 1),
                orbit=dict(prefix='O', minimum=0, maximum=34)),
            current=current, next=nxt, target=piece(position=self.w['target']),
            buffers=[piece(position=p) for p in self.w['roles']], piece_filter=piece_filter.metadata(state),
            block=members, protected=self.s.prefs['protected'], position_locks=self.position_locks(), progress=progress,
            residuals=self.cached_residuals(), journal_delta=self.journal_delta(), candidates=self.cached_candidates(),
            review=review, pending=self.s.pending['public'] if self.s.pending else None,
            executable=executable,
            library=[self.library_record(x, completed_orbits) for x in self.library.values()],
            banks=self.banks(), checkpoints=self.s.status()['checkpoints'],
            frame_notation=clone(self.frame_notation),
            approvals=dict(G1='Approved by user 2026-09-16', G2='Approved by user 2026-09-16'))
        answer['inspected'] = (piece(position=self.w['inspected_position'])
                               if self.w['inspected_position'] is not None else piece(identity=self.w['inspected']))
        if render:
            styles = self.s.render_styles().copy()
            # Only exact work membership is pickable. No ghost/reference pins.
            styles[self.s.interactive_styles() == 0] = 0
            answer['render'] = dict(labels=base64.b64encode(state.labels.astype('<u4').tobytes()).decode(),
                styles=base64.b64encode(styles.astype('u1').tobytes()).decode(), state_hash=state.hash)
        # Native comparison needs exact objects, not another full render payload.
        # Only a current complete review permits native prediction; stale results
        # must not keep a plausible-looking future arrangement on the canvas.
        prediction = prediction and review is not None and review['status'] in ('Ready', 'Staged')
        if (render or prediction) and self.operation_state() != 'executed' and any(self.w['draft'][p] for p in PHASES):
            try:
                src, dst, _, _ = self.m.net(self.concrete())
                labels = state.labels.copy(); labels[dst] = labels[src]
                predicted = PuzzleState(self.m, labels, trusted=True)
                answer['predicted'] = dict(current=piece(identity=self.w['current'], state=predicted),
                    next=piece(identity=self.w['next']['identity'], state=predicted) if self.w['next'] else None,
                    target=piece(position=self.w['target'], state=predicted),
                    buffers=[piece(position=p, state=predicted) for p in self.w['roles']],
                    block=[piece(position=m['position'], state=predicted) for m in self.w['block']['members']],
                    piece_filter=piece_filter.metadata(predicted),
                    state_hash=predicted.hash)
                if render:
                    predicted_styles = Filters(predicted, self.w['orbit'], self.s.prefs['selected'],
                        self.s.prefs['protected'], sets=self.s.prefs.get('named_sets', {})).styles(self.s.prefs['rules'], False)
                    answer['predicted'].update(labels=base64.b64encode(labels.astype('<u4').tobytes()).decode(),
                        styles=base64.b64encode(predicted_styles.astype('u1').tobytes()).decode())
            except Exception as error:
                if render:
                    raise
                # Native comparison is optional; keep the authoritative
                # snapshot even if forecasting stops.
                answer.pop('predicted', None)
                answer['prediction_error'] = ('optional forecast analysis cancelled.'
                                              if isinstance(error, InterruptedError) else str(error))
        return answer

    def effect_decomposition(self, recipe):
        """Internal full action coordinates for linked read-only consumers."""
        _, _, facts = self.effect(recipe)
        return self.effects[facts['id']][3]

    def cached_residuals(self):
        """Passive display lookup: no state decomposition or operation analysis."""
        context_id = self.review_context()['id']
        if self.residual_cache is not None and self.residual_cache['review_context_id'] == context_id:
            return clone(self.residual_cache)
        return dict(status='NotAnalysed', review_context_id=context_id)

    def residual_bundle(self, context, facts=None, predicted=None):
        """Cancellable analysis; caller publishes only the complete bound result."""
        self.m.check_cancel()
        recipe = [step for phase in PHASES for step in self.w['draft'][phase]]
        explicit_review = facts is not None
        active = bool(recipe) and (explicit_review or context['operation_state'] != 'executed')
        if active and facts is None:
            source, destination, facts = self.effect(recipe)
            labels = self.s.st.labels.copy(); labels[destination] = labels[source]
            predicted = PuzzleState(self.m, labels, trusted=True)
        scope = {row['orbit'] for row in facts['support']} if active else set()
        status = 'Known' if active or context['operation_state'] != 'executed' else 'Executed'
        pending = context['external_pending']
        if pending is not None:
            if pending['status'] != 'Current':
                status = 'Unknown'
            else:
                rows = self.s.pending['public'].get('support')
                if (not isinstance(rows, list) or any(not isinstance(row, dict)
                        or type(row.get('orbit')) is not int or not 0 <= row['orbit'] < 35 for row in rows)):
                    status = 'Unknown'
                else:
                    scope.update(row['orbit'] for row in rows)
                    status = 'Known'
        bindings = dict(roles=clone(self.w['roles']), goal=self.w['goal'], prefix=self.w['prefix'],
            protected_requirements=dict(orbits=clone(self.s.prefs['protected']), positions=self.position_locks()),
            operation=dict(status=status, context_id=context['id'], recipe_hash=context['recipe_hash'],
                           affected_orbits=sorted(scope)))
        try:
            invariants, invariant_error = self.invariant_service(), None
        except ValueError as error:
            invariants, invariant_error = None, str(error)
        current = derive_residual(self.m, self.s.st, self.w['orbit'], bindings,
                                  self.transported_frames, context, invariant_evidence=invariants)
        if invariant_error:
            current['invariant_status']['reason'] = invariant_error
        after, delta = None, None
        if active:
            after = derive_residual(self.m, predicted, self.w['orbit'], bindings, self.transported_frames,
                context, dict(kind='After', base_state_hash=context['state_hash'], recipe_hash=context['recipe_hash']),
                invariant_evidence=invariants)
            delta = derive_delta(self.m, self.s.st, predicted, self.w['orbit'], bindings)
            delta.update(source='Predicted', review_context_id=context['id'])
        self.m.check_cancel()
        if self.review_context()['id'] != context['id']:
            raise ValueError('Residual analysis context changed; inspect the current work again')
        return dict(status='Analysed', review_context_id=context['id'], current=current, after=after, delta=delta,
            after_reason=None if active else 'Retained steps were already executed' if context['operation_state'] == 'executed'
                         else 'No explicit operation is entered')

    def inspect_residual(self, context_id=None):
        with self.lock:
            context = self.review_context()
            if context_id is not None and context_id != context['id']:
                raise ValueError('Residual inspection context changed; refresh before inspecting')
            result = self.residual_bundle(context)
            self.m.check_cancel()
            self.residual_cache = clone(result)
            return result

    def review_draft(self, *, response_snapshot=True):
        self.m.check_cancel()
        before_guard = self.guard()
        review_context = self.review_context()
        recipe = self.concrete()
        src, dst, facts = self.effect(recipe)
        after = self.s.st.labels.copy(); after[dst] = after[src]
        predicted = PuzzleState(self.m, after, trusted=True)
        conflicts = [r for r in facts['support'] if r['orbit'] in self.s.prefs['protected']]
        block_conflicts = []
        requirements = []
        for member in self.position_locks():
            rule = prepare_requirement(self.m, member)
            requirements.append((member['position'], rule))
            if not requirement_holds(self.m, after, rule):
                block_conflicts.append(member['position'])
        block_conflicts = sorted(set(block_conflicts))
        prefix = dict(status='Unchecked', first_violation=None)
        if self.w['prefix']:
            mask = np.isin(self.m.so, self.s.prefs['protected'])
            temp = self.s.st.labels.copy()
            first = None
            prefix_positions = []
            for index, primitive in enumerate(self.m.expand(recipe)):
                self.m.check_cancel()
                ps, pd = self.m.move(primitive); temp[pd] = temp[ps]
                if (np.any(temp[mask] != self.s.st.labels[mask]) or
                        any(not requirement_holds(self.m, temp, rule) for _, rule in requirements)):
                    first = index + 1
                    prefix_positions = sorted(set(map(int, self.m.sp[mask & (temp != self.s.st.labels)])) |
                        {position for position, rule in requirements
                         if not requirement_holds(self.m, temp, rule)})
                    break
            prefix = dict(status='Violation' if first else 'Preserved', first_violation=first,
                conflicting_positions=prefix_positions[:12], conflicting_positions_total=len(prefix_positions),
                conflicting_positions_truncated=len(prefix_positions) > 12)
        target = self.w['target']; current = self.w['current']
        target_slots = None if target is None else self.m.slots(target)
        requirement = self.w.get('target_requirement')
        matching_requirement = requirement is not None and requirement['identity'] == current and requirement['position'] == target
        required_labels = requirement['labels'] if matching_requirement else target_slots if target == current else None
        target_met = bool(target is not None and current is not None and predicted.at[target] == current and
                          required_labels is not None and np.array_equal(predicted.labels[target_slots], required_labels))
        goal = self.w.get('goal', 'insert')
        block_met = all(requirement_holds(self.m, predicted.labels, prepare_requirement(self.m, m))
                        for m in self.w['block']['members'])
        residual = np.flatnonzero((self.m.oid == self.w['orbit']) & ~predicted.correct)
        role_reasons = []
        if current is None or target is None:
            role_reasons.append('Current identity and destination are not both assigned; this remains an explicit preparation operation.')
        elif goal != 'place' and target != current and required_labels is None:
            role_reasons.append('Destination occupant is checked, but no exact destination frame is declared. Capture or explicitly transport a requirement before claiming insertion.')
        star = facts['star']
        if star:
            if star['orbit'] != self.w['orbit']:
                role_reasons.append('Selected star belongs to another orbit.')
            if target is not None and star['target'] != target:
                role_reasons.append('The fixed star target differs from the assigned destination. No retargeting was performed.')
            if self.w['roles'] != [star['a'], star['b']]:
                role_reasons.append('Assigned A/B positions differ from the retained star roles.')
        blocked = bool(conflicts or block_conflicts or prefix['status'] == 'Violation')
        residuals = self.residual_bundle(review_context, facts, predicted)
        goal_result = evaluate_goal(self.m, self.s.st, predicted, self.w,
                                    residuals['current'], residuals['after'])
        # The legacy Prepare flag never meant a completed target. The explicit
        # result is NotApplicable, and only protection governs this permission.
        goal_met = goal == 'prepare' or goal_result['status'] == 'Met'
        public = dict(id=secrets.token_hex(12), status='Conflict' if blocked else 'Ready',
            review_context=review_context,
            residual_before=residuals['current'], residual_after=residuals['after'], residual_delta=residuals['delta'],
            effect=facts, conflicts=conflicts, block_conflicts=block_conflicts, prefix=prefix,
            target_met=target_met, reasons=role_reasons,
            goal=goal, goal_met=goal_met, goal_result=goal_result, block_met=block_met,
            remaining_pieces=len(residual), remaining_positions=list(map(int, residual[:100])),
            remaining_truncated=len(residual) > 100,
            target_after=self.piece(position=target, state=predicted),
            current_after=self.piece(identity=current, state=predicted), post_hash=predicted.hash,
            net_policy='Preserve exact captured state at the complete-operation boundary',
            goal_note=goal_result['reason'])
        relation = next((star for star in facts['scoped_stars']
                         if star['orbit'] == self.w['orbit']), None)
        match = (dict(status='Declared', review_context_id=review_context['id'], scope='complete',
                      scoped_star=relation) if relation else None)
        public['recommendation'] = score_current(self.m, self.s.st, predicted, self.w, public, match)
        self.m.check_cancel()
        self.review = dict(public=public, recipe=recipe, guard=before_guard, blocked=blocked)
        self.residual_cache = clone(residuals)
        return self.snapshot() if response_snapshot else None

    def sheet_inputs(self):
        return dict(clone(self.w), protected_orbits=clone(self.s.prefs['protected']),
                    position_locks=clone(self.position_locks()))

    def sheet_display(self, key, value):
        if value is None:
            return 'Not recorded', ''
        detail = canonical(value)
        if key == 'orbit':
            return self.names.orbit(value)['name'], detail
        if key in ('current', 'target'):
            return ('Home ' if key == 'current' else 'Position ') + self.names.address(value)['short_name'], detail
        if key == 'roles':
            return '\n'.join(role + ' ' + self.names.address(p)['short_name'] for role, p in zip(('A', 'B'), value)), detail
        if key == 'block':
            return str(value.get('name', 'Block')) + ' · ' + str(len(value.get('members', []))) + ' requirements', detail
        if key == 'target_requirement':
            kind = 'Home requirement' if value.get('provenance', {}).get('kind') == 'home' else 'Captured frame'
            return kind + ' · ' + self.names.address(value['position'])['short_name'], detail
        if key == 'prefix':
            return 'Every turn' if value else 'End of complete operation', detail
        if key == 'protection':
            return '\n'.join(self.names.orbit(o)['name'] for o in value) or 'No whole orbits protected', detail
        if key == 'position_locks':
            return str(len(value)) + ' captured positions across all orbit contexts', detail
        if key == 'reference':
            return 'Canonical reference' if not value else str(len(value)) + ' explicitly supplied reference turns', detail
        if key == 'goal':
            return {'prepare': 'Prepare', 'insert': 'Insert (exact)', 'place': 'Place piece',
                    'orient': 'Orient piece', 'finish-buffer': 'Finish buffer',
                    'block': 'Build block', 'endgame': 'Finish orbit'}.get(value, str(value)), detail
        return str(value), detail

    def replace_phase(self, phase, recipe, sources=None):
        if phase not in PHASES:
            raise ValueError('Unknown operation phase')
        normalized = self.m.normalize(recipe)[0] if recipe else []
        if not recipe and recipe != []:
            raise ValueError('Empty phase must be []')
        draft = clone(self.w['draft']); draft[phase] = normalized
        total = [x for p in PHASES for x in draft[p]]
        if total:
            self.m.normalize(total)
        self.w['draft'] = draft
        self.w['draft_sources'][phase] = clone(sources or [])

    def resolve_object_input(self, value, kind, label):
        """Resolve copied names once; persistent work fields remain canonical integers."""
        if isinstance(value, str):
            try:
                value = (self.names.parse_identity(value) if kind == 'Piece'
                         else self.names.parse_address(value))
            except ValueError as error:
                raise ValueError(label + ': ' + str(error)) from error
        return integer(value, self.m.np, label)

    def session_command(self, body):
        action = body['action']
        helper = self.session_workflow
        if action == 'session-report':
            return helper.report()
        if action == 'session-resume':
            return helper.resume()
        if action == 'session-timer':
            return helper.timer(body.get('command'))
        if action == 'session-save-log':
            return helper.save_log(body.get('format', 'c600'), self.native_profile())
        if action == 'session-log-export':
            payload = helper.export_log(body.get('format', 'c600'), self.native_profile())
            return dict(format=body.get('format', 'c600'), bytes=len(payload),
                        data_base64=base64.b64encode(payload).decode('ascii'), file_sha256=digest(payload))
        if action == 'session-log-inspect':
            return helper.inspect_log(body.get('data_base64'), body.get('format', 'c600'), self.native_profile())
        if action == 'session-log-apply':
            result = helper.apply_log(body.get('data_base64'), body.get('confirmation_id'),
                                     body.get('format', 'c600'), self.native_profile())
            # The retained importer preserves current preferences and creates a
            # recoverable journal branch. Keep work selections and drafts, but
            # never carry review or execution authority across imported state.
            self.s.pending = None
            self.review = self.execution = self.sheet_confirmation = self.residual_cache = self.candidate_cache = None
            self.completed_operations.clear()
            self.epoch = secrets.token_hex(12)
            return result
        if action == 'session-completion-ack':
            return helper.acknowledge(body.get('id'))
        if action == 'session-scramble':
            if (self.s.pending or any(self.w['draft'].values())) and body.get('replace') is not True:
                raise ValueError('Current steps or a preview exist. Explicitly allow replacement before staging a scramble.')
            generated = helper.scramble_recipe(body.get('count'), body.get('seed'))
            work = clone(self.w)
            work.update(draft=dict(prepare=generated['recipe'], macro=[], cleanup=[]),
                draft_sources={p: [] for p in PHASES}, phase='prepare', input='draft',
                operation_generation=secrets.token_hex(12), goal='prepare')
            work.pop('completed_operation', None)
            work['scramble_source'] = dict(generated['source'], operation_generation=work['operation_generation'])
            layout = clone(self.s.prefs.get('layout', {})); layout['magic600_experiment'] = work
            self.s.save_prefs({'layout': layout})
            self.w = work
            result = dict(source=clone(work['scramble_source']), staged=True)
        elif action == 'session-new':
            work = self.new_workspace()
            for key in ('captures', 'grip_frames', 'bank_overrides', 'templates', 'personal_macros', 'keybinds', 'saved_filters'):
                if key in self.w:
                    work[key] = clone(self.w[key])
            layout = clone(self.s.prefs.get('layout', {})); layout['magic600_experiment'] = work
            self.s.reset(preference_changes=dict(layout=layout, orbit=33, protected=[], selected=None,
                rules=[dict(expr='active', style='solid')], pin_safety=False), workflow_meta=helper.plan_new_attempt())
            self.w = work
            helper.workflow.timer('start')
            result = dict(reset='new', recovery=True)
        elif action == 'session-reset':
            scope = body.get('scope')
            if scope == 'puzzle':
                self.s.reset()
            elif scope == 'workspace':
                work = clone(self.w)
                for context in [work] + list(work.get('contexts', {}).values()):
                    context.update(current=None, target=None, inspected=None, inspected_position=None,
                        reference=[], goal='prepare', target_requirement=None,
                        draft={p: [] for p in PHASES}, draft_sources={p: [] for p in PHASES},
                        operation_generation=secrets.token_hex(12))
                    for key in ('selected_macro', 'completed_operation', 'scramble_source'):
                        context.pop(key, None)
                # Blocks and their explicit requirements are retained: resetting
                # presentation must never silently release mechanical protection.
                work.update(next=None, phase='prepare', input='draft', view=dict(tab='block', local=True, global_view=False, local_center=1))
                layout = clone(self.s.prefs.get('layout', {})); layout['magic600_experiment'] = work
                self.s.save_prefs({'layout': layout})
                self.w = work
            else:
                raise ValueError('Reset scope must be puzzle or workspace')
            result = dict(reset=scope)
        else:
            raise ValueError('Unknown Session command')
        self.s.pending = None
        self.review = self.execution = self.sheet_confirmation = self.residual_cache = self.candidate_cache = None
        self.completed_operations.clear()
        self.epoch = secrets.token_hex(12)
        return result

    def command(self, body, *, response_snapshot=True):
        if not isinstance(body, dict):
            raise ValueError('Command must be an object')
        if body.get('action') == 'inspect-residual':
            return self.inspect_residual(body.get('review_context_id'))
        if body.get('action') == 'inspect-cycles':
            return inspect_cycles(self, body)
        if body.get('action') == 'inspect-phase':
            # Native transport attaches detached phase inspection to its paired
            # reply. This command itself changes no work or execution authority.
            return None
        for flag in ['replace', 'append', 'replace_pending', 'enabled', 'preview', 'confirm_inputs']:
            if flag in body and type(body[flag]) is not bool:
                raise ValueError(flag + ' must be a Boolean')
        if str(body.get('action', '')).startswith('session-'):
            return self.session_command(body)
        if body.get('action') == 'display-settings':
            settings = body.get('settings')
            fields = {'hide_frame': 'native_hide_frame', 'adaptive_motion': 'native_adaptive_motion'}
            if (not isinstance(settings, dict) or not settings or set(settings) - fields.keys()
                    or any(type(value) is not bool for value in settings.values())):
                raise ValueError('Display settings require hide_frame and/or adaptive_motion Boolean values')
            view = clone(self.s.prefs.get('view', {}))
            view.update({fields[key]: value for key, value in settings.items()})
            self.s.save_prefs({'view': view})
            return dict(display={key: view.get(field, True) for key, field in fields.items()})
        if body.get('action') in ('keymap-export', 'keymap-import-check', 'keymap-import'):
            banks = {bank['id']: len(bank['slots']) if bank.get('turns_enabled') else 0
                     for bank in self.banks()}
            commands = keymap_command_ids()
            saved = self.w.get('keybinds', {})
            if body['action'] == 'keymap-export':
                return dict(document=export_keymap(self.m.model_id, saved, commands, banks))
            if body['action'] == 'keymap-import-check':
                return inspect_keymap(body.get('text'), self.m.model_id, saved, commands, banks)
            replacement = import_keymap(body.get('text'), body.get('basis'), self.m.model_id, saved, commands, banks)
            work = clone(self.w); work['keybinds'] = replacement
            layout = clone(self.s.prefs.get('layout', {})); layout['magic600_experiment'] = work
            self.s.save_prefs({'layout': layout})
            self.w = work
            return dict(imported=True, scope='keybindings-only')
        before_intent = canonical(dict(intent=self.intent(), protected=self.s.prefs['protected'],
                                       position_locks=self.position_locks()))
        action = body.get('action')
        committed_operation, command_warning = False, None
        self.save_warnings = []
        edits_draft = action in ('draft', 'insert-macro', 'inverse-cleanup', 'review') or (
            action in ('twist', 'native-word') and body.get('destination') == 'draft')
        if edits_draft and self.operation_state() == 'executed':
            raise ValueError('These steps were executed. Choose New operation to empty all phases, or Reuse these exact steps before editing or checking again.')
        if edits_draft and action != 'review' or action in ('operation-new', 'operation-reuse', 'template-use', 'fixture'):
            self.w.pop('scramble_source', None)
        if action == 'inspect':
            identity = self.resolve_object_input(body['identity'], 'Piece', 'Identity')
            self.w.update(inspected=identity, inspected_position=None)
        elif action == 'inspect-position':
            position = self.resolve_object_input(body['position'], 'Position', 'Position')
            self.w.update(inspected=None, inspected_position=position)
        elif action == 'focus':
            identity = self.resolve_object_input(body['identity'], 'Piece', 'Identity')
            target = self.resolve_object_input(body.get('target', identity), 'Position', 'Target')
            if self.m.oid[identity] != self.m.oid[target] or self.m.oid[identity] < 0:
                raise ValueError('Identity and target must share a moving orbit')
            o = int(self.m.oid[identity])
            self.switch_orbit(o, restore=True,
                              update=dict(current=identity, target=target, inspected=identity, inspected_position=None))
        elif action == 'next-pin':
            identity = self.resolve_object_input(body['identity'], 'Piece', 'Next identity')
            target = self.resolve_object_input(body.get('target', identity), 'Position', 'Next target')
            if self.m.oid[identity] < 0 or self.m.oid[identity] != self.m.oid[target]:
                raise ValueError('Next identity and destination must share a moving orbit')
            if self.w['next'] and not body.get('replace', False):
                raise ValueError('Next is already locked. Choose Replace Next explicitly.')
            self.w['next'] = dict(identity=identity, target=target)
        elif action == 'next-clear':
            self.w['next'] = None
        elif action == 'goal':
            self.w['goal'] = normalize_goal(body.get('goal'))
        elif action == 'target-home':
            identity, position = self.w['current'], self.w['target']
            if (identity is None or position is None or identity != position
                    or int(self.m.oid[identity]) != self.w['orbit']):
                raise ValueError('An explicit Home requirement needs Current and its Home destination in the active moving orbit')
            self.w['target_requirement'] = dict(identity=identity, position=position,
                labels=self.m.slots(position).tolist(), provenance=dict(kind='home', model=self.m.model_id))
        elif action == 'target-capture':
            identity, position = self.w['current'], self.w['target']
            if identity is None or position is None or int(self.s.st.at[position]) != identity:
                raise ValueError('Capture a destination frame only when Current actually occupies the chosen destination')
            self.w['target_requirement'] = dict(identity=identity, position=position,
                labels=self.s.st.labels[self.m.slots(position)].tolist())
        elif action == 'block-reference':
            word = body.get('word')
            if not isinstance(word, list) or not word or any(type(m) is not int or not 1 <= abs(m) <= 1200 for m in word):
                raise ValueError('Block reference needs an explicit finite legal word')
            if not self.w['block']['members']:
                raise ValueError('Define block members before transporting their requirements')
            recipe = self.m.normalize([dict(kind='word', moves=word)])[0]
            src, dst, _, _ = self.m.net(recipe)
            transport = self.m.ids.copy(); transport[src] = dst
            members = []
            for member in self.w['block']['members']:
                destinations = transport[self.m.slots(member['position'])]
                positions = self.m.sp[destinations]
                if not np.all(positions == positions[0]):
                    raise ValueError('The selected operation does not transport this whole piece to one position')
                order = np.argsort(destinations)
                members.append(dict(identity=member['identity'], mode=member.get('mode', 'exact'), position=int(positions[0]),
                                    labels=np.asarray(member['labels'])[order].tolist()))
            self.w['block']['members'] = members
            # Last applied reference change, not a cumulative transform from Home.
            # The explicit member positions/labels remain the requirement authority.
            self.w['block']['reference_word'] = list(word)
            # Existing protection remains at the explicitly protected positions.
        elif action == 'next-activate':
            if not self.w['next']:
                raise ValueError('No Next piece is locked')
            nxt = self.w['next']
            self.switch_orbit(int(self.m.oid[nxt['identity']]), restore=True,
                              update=dict(next=None, current=nxt['identity'], target=nxt['target'],
                                          inspected=nxt['identity'], inspected_position=None))
        elif action == 'bank':
            bank = next((x for x in self.banks() if x['id'] == body['id']), None)
            if not bank:
                raise ValueError('Unknown bank ID')
            if bank['id'] == self.w['bank']:
                return self.snapshot(render=True) if response_snapshot else None
            self.w.update(previous_bank=self.w['bank'], bank=bank['id'])
        elif action == 'orbit':
            orbit = integer(body.get('orbit'), 35, 'Orbit')
            if orbit != self.w['orbit']:
                self.switch_orbit(orbit, restore=True)
        elif action == 'capture':
            cells = body['cells']
            if not isinstance(cells, list) or not 1 <= len(cells) <= 20:
                raise ValueError('Capture requires 1..20 explicit canonical cells')
            for c in cells:
                canonical_cell(c)
            self.w['captures'][self.w['bank']] = list(cells)
        elif action == 'grip-frames':
            return dict(self.frame_choices(body['cell']), selected=self.selected_frame(self.w['bank'], body['cell']),
                        notation=clone(self.frame_notation))
        elif action == 'capture-frame':
            cell = canonical_cell(body['cell'])
            if body.get('bank') != self.w['bank']:
                raise ValueError('Key set changed while editing the Grip frame; no binding was changed')
            bank = next(b for b in self.banks() if b['id'] == self.w['bank'])
            if cell not in bank['slots']:
                raise ValueError('Capture this cap in the active key set before assigning its frame')
            vertices = body.get('vertices')
            choices = self.frame_choices(cell)
            if not isinstance(vertices, list) or any(type(v) is not int for v in vertices) or not any(vertices == x['vertices'] for x in choices['frames']):
                raise ValueError('Select one of the twelve proper ordered frames of this cap')
            if body.get('previous_vertices') != self.selected_frame(self.w['bank'], cell):
                raise ValueError('Grip frame changed while editing. Reopen the current frame; no binding was changed')
            self.w.setdefault('grip_frames', {}).setdefault(self.w['bank'], {})[str(cell)] = list(vertices)
        elif action == 'draft':
            self.replace_phase(body['phase'], body['recipe'])
        elif action == 'insert-macro':
            macro = self.library.get(body['id'])
            if not macro:
                raise ValueError('Macro entry is missing')
            phase = body.get('phase', 'macro')
            if phase not in PHASES:
                raise ValueError('Unknown phase')
            recipe = clone(macro['recipe'])
            before = self.w['draft'][phase] if body.get('append') else []
            sources = clone(self.w['draft_sources'][phase]) if body.get('append') else []
            if recipe:
                sources.append(dict(id=macro['id'], version=macro['version'], start=len(before),
                                    count=len(recipe), recipe=clone(recipe)))
            self.replace_phase(phase, before + recipe, sources)
        elif action == 'inverse-cleanup':
            recipe = invrecipe(self.w['draft']['prepare']) if self.w['draft']['prepare'] else []
            return self.command(dict(action='draft', phase='cleanup', recipe=recipe), response_snapshot=response_snapshot)
        elif action in ('twist', 'native-word'):
            if action == 'native-word':
                moves = body.get('moves')
                if not isinstance(moves, list) or not moves or any(type(x) is not int or not 1 <= abs(x) <= 1200 for x in moves):
                    raise ValueError('Native input requires exact legal signed generators')
                word = list(moves)
            else:
                if type(body.get('inverse')) is not bool:
                    raise ValueError('Twist inverse must be an explicit Boolean')
                active_bank = next(b for b in self.banks() if b['id'] == self.w['bank'])
                if not active_bank['turns_enabled']:
                    raise ValueError('The active key set has no physical turns. Select a Grip/Twist set explicitly')
                selected_vertices = self.selected_frame(self.w['bank'], body['cell'])
                if 'bank' in body and body['bank'] != self.w['bank']:
                    raise ValueError('Input key set changed before the turn was accepted')
                if 'ordered_vertices' in body and body['ordered_vertices'] != selected_vertices:
                    raise ValueError('Input Grip frame changed before the turn was accepted')
                if 'ordered_vertices' not in body and selected_vertices != self.frame_choices(body['cell'])['base_vertices']:
                    raise ValueError('A rebound Grip requires the explicit ordered frame in the input; legacy input uses only the retained frame')
                word = resolve_frame(self.m, body['cell'], selected_vertices, body['axis'], body['inverse'])['word']
            if body.get('destination') not in ('draft', 'live'):
                raise ValueError('Twist destination must be explicitly draft or live')
            if body.get('state_hash') != self.s.st.hash:
                raise ValueError('Input state is stale; refresh before turning')
            recipe = [dict(kind='word', moves=word)]
            if body.get('destination') == 'live':
                if self.s.pending:
                    raise ValueError('A preview is pending. Commit or cancel it explicitly before live turning.')
                old = self.w['draft']; prior_review, prior_execution = self.review, self.execution
                prior_pending = self.s.pending
                committed = False
                self.w['draft'] = dict(prepare=recipe, macro=[], cleanup=[])
                try:
                    self.review_draft(response_snapshot=response_snapshot)
                    if self.review['blocked']:
                        raise ValueError('Live turn conflicts with protection. Use a complete draft for temporary motion.')
                    preview = self.s.preview(recipe, 'Experimental explicit native input', 'manual-grip-experiment')
                    command_warning = self.commit_with_result_warning(preview['token'], recipe, 'manual-grip-experiment',
                                                                       retained_draft=old)
                    committed = committed_operation = True
                finally:
                    self.w['draft'] = old
                    self.review = None if committed else prior_review
                    self.execution = None if committed else prior_execution
                    if not committed:
                        self.s.pending = prior_pending
            else:
                phase = body.get('phase', 'prepare')
                if phase not in PHASES:
                    raise ValueError('Unknown phase')
                return self.command(dict(action='draft', phase=phase, recipe=self.w['draft'][phase] + recipe), response_snapshot=response_snapshot)
        elif action == 'review':
            return self.review_draft(response_snapshot=response_snapshot)
        elif action == 'preview':
            if not self.review or self.review['public']['id'] != body.get('review_id') or self.review['guard'] != self.guard():
                raise ValueError('Review is stale or missing. Review the complete operation again.')
            if self.review['blocked']:
                raise ValueError('Complete operation violates protection')
            if self.s.pending and not body.get('replace_pending'):
                raise ValueError('An unrelated preview exists. Choose explicit replacement or cancel it first.')
            preview_prior = (self.s.pending, self.review, self.execution, self.residual_cache)
            try:
                source = self.w.get('scramble_source', {})
                is_scramble = (source.get('operation_generation') == self.w['operation_generation']
                    and source.get('pre_state') == self.s.st.hash
                    and source.get('recipe_hash') == digest(canonical(self.review['recipe']).encode()))
                p = self.s.preview(self.review['recipe'], 'Recorded scramble' if is_scramble else 'Human-directed experimental work sheet',
                    'recorded-scramble' if is_scramble else 'manual-work-sheet')
                if self.review_context()['id'] != self.review['public']['review_context']['id']:
                    # Replacing an external pending action changes completion facts.
                    # Recompute them under the actual new pending state, not new IDs.
                    self.review_draft(response_snapshot=False)
                    if self.review['blocked']:
                        raise ValueError('Replacement preview no longer satisfies complete-operation protection')
                self.m.check_cancel()
                self.execution = dict(token=p['token'], guard=self.guard(False))
            except Exception:
                self.s.pending, self.review, self.execution, self.residual_cache = preview_prior
                raise
        elif action == 'commit':
            if not self.execution or self.execution['guard'] != self.guard(False):
                raise ValueError('Work intent, state or policy changed. A fresh review and preview are required.')
            command_warning = self.commit_with_result_warning(self.execution['token'], self.concrete(), self.s.pending['public']['assistance'])
            committed_operation = True
            self.execution = None
        elif action in ('operation-new', 'operation-reuse'):
            if self.s.pending:
                raise ValueError('Commit or cancel the pending preview before starting or reusing an operation.')
            if action == 'operation-new':
                self.w['draft'] = {p: [] for p in PHASES}
                self.w['draft_sources'] = {p: [] for p in PHASES}
        elif action == 'cancel-preview':
            self.s.pending = None; self.execution = None
        elif action in ['undo', 'redo', 'reset', 'checkpoint']:
            if action == 'checkpoint':
                self.s.checkpoint(body.get('name'))
            else:
                getattr(self.s, action)(); self.execution = None
                if action in ('undo', 'redo'):
                    self.restore_completed_operation()
                else:
                    self.completed_operations.clear()
        elif action == 'restore':
            self.s.restore(body['name']); self.execution = None
            saved = self.s.prefs.get('layout', {}).get('magic600_experiment')
            if saved:
                self.w = clone(saved)
                workspace_defaults(self.w)
                self.library = self.default_library()
                self.library.update(self.w.get('personal_macros', {}))
            self.review = None
            self.restore_completed_operation()
        elif action == 'protect':
            values = body['orbits']
            self.s.save_prefs({'protected': values})
        elif action == 'prefix':
            if type(body['strict']) is not bool:
                raise ValueError('Strict prefix must be a boolean')
            self.w['prefix'] = body['strict']
        elif action == 'block-add':
            capture_mode = requirement_mode(body.get('mode', 'exact'))
            capture_current = body.get('capture_current', False)
            if type(capture_current) is not bool:
                raise ValueError('Current-state capture must be an explicit boolean')
            if capture_current and 'position' not in body:
                raise ValueError('Current-state capture needs an explicit position')
            identity = self.resolve_object_input(body['identity'], 'Piece', 'Block identity')
            position = self.resolve_object_input(body.get('position', identity), 'Position', 'Block position')
            if capture_current or position != identity:
                if int(self.s.st.at[position]) != identity:
                    raise ValueError('Capture a relative member only at its actual current position; no frame is inferred')
                labels = self.s.st.labels[self.m.slots(position)].tolist()
            else:
                labels = self.m.slots(position).tolist()
            member = dict(identity=identity, position=position, labels=labels, mode=capture_mode)
            self.w['block']['members'] = [m for m in self.w['block']['members'] if m['identity'] != identity] + [member]
        elif action == 'block-remove':
            identity = self.resolve_object_input(body['identity'], 'Piece', 'Block identity')
            self.w['block']['members'] = [x for x in self.w['block']['members'] if x['identity'] != identity]
        elif action == 'block-protect':
            capture_mode = requirement_mode(body.get('mode', 'exact'))
            p = self.resolve_object_input(body['position'], 'Position', 'Protected position')
            def retained(x):
                return x['position'] != p or ('mode' in body and x.get('mode', 'exact') != capture_mode)
            # Adding a weaker lock never removes an existing stronger lock.
            captures = [x for x in self.w['block']['protected']
                        if x['position'] != p or x.get('mode', 'exact') != capture_mode]
            if body.get('enabled', True):
                captures.append(dict(position=p, labels=self.s.st.labels[self.m.slots(p)].tolist(), mode=capture_mode))
            else:
                captures = [x for x in self.w['block']['protected'] if retained(x)]
                for context in self.w['contexts'].values():
                    context['block']['protected'] = [x for x in context['block']['protected'] if retained(x)]
            self.w['block']['protected'] = captures
        elif action == 'roles':
            roles = body['positions']
            if not isinstance(roles, list) or len(roles) != 2:
                raise ValueError('A and B need two distinct positions')
            roles = [self.resolve_object_input(p, 'Position', 'Buffer position') for p in roles]
            if roles[0] == roles[1]:
                raise ValueError('A and B need two distinct positions; both inputs resolve to the same position')
            for p in roles:
                if self.m.oid[p] != self.w['orbit']:
                    raise ValueError('Buffer position belongs to a different orbit')
            self.w['roles'] = roles
        elif action == 'reference':
            word = body['word']
            if not isinstance(word, list):
                raise ValueError('Reference must be an explicit legal primitive word, or [] for canonical')
            if word:
                self.m.normalize([dict(kind='word', moves=word)])
            self.w['reference'] = list(word)
        elif action == 'transform-macro':
            macro = self.bound_macro(body['id'])
            result = self.create_macro_variant(dict(source={k: macro[k] for k in ('id', 'version', 'recipe')},
                kind='reference', reference=clone(self.w['reference']), name=body.get('name', (macro['name'] + ' · explicit reference')[:80])))
            if response_snapshot:
                snapshot = self.snapshot()
                snapshot['created_id'] = result['created_id']
                if 'warning' in result:
                    snapshot['command_warning'] = result['warning']
                return snapshot
            return result
        elif action == 'macro-variant':
            return self.create_macro_variant(body)
        elif action == 'reference-frames':
            cell = canonical_cell(body.get('cell'))
            return dict(self.frame_choices(cell), cell_name=self.names.cell(cell)['name'])
        elif action == 'macro-geometry-review':
            source = self.bound_macro(body.get('source'), binding_required=True)
            result = self.reference_variants.compile(source, body.get('source_frame'), body.get('destination_frame'))
            result['effect'] = self.effect(result['recipe'])[2]
            return result
        elif action == 'endgame-options':
            result = self.endgames.options(body.get('orbit'), body.get('position'))
            result['roles'] = list(map(int, self.m.trees[result['orbit']]['buffers']))
            result['roles_match'] = result['roles'] == self.w['roles']
            result['position_name'] = self.names.address(result['position'])
            result['role_names'] = [self.names.address(p) for p in result['roles']]
            result['position_record'] = self.piece(position=result['position'])
            result['buffer_records'] = [self.piece(position=p) for p in result['roles']]
            return result
        elif action == 'endgame-compose':
            return self.endgame_choice(body)
        elif action == 'endgame-save':
            return self.save_endgame(body)
        elif action == 'macro-select':
            self.w['selected_macro'] = {k: v for k, v in self.bound_macro(body.get('source'), True).items()
                                        if k in ('id', 'version', 'recipe')}
        elif action == 'macro-effect':
            macro = self.bound_macro(body.get('source', body['id']))
            facts = self.effect(macro['recipe'])[2]
            if body.get('select') is True:
                self.w['selected_macro'] = {key: clone(macro[key]) for key in ('id', 'version', 'recipe')}
                self.save()
            result = dict(effect=facts, applicability=self.applicability(facts))
            if response_snapshot:
                result['snapshot'] = self.snapshot()
            return result
        elif action == 'macro-compare':
            return self.compare_macros(self.bound_macro(body['a']), self.bound_macro(body['b']))
        elif action == 'save-macro':
            recipe = self.m.normalize(body['recipe'])[0]
            key = 'user-' + secrets.token_hex(6)
            record = dict(id=key, name=str(body['name'])[:80], note=str(body.get('note', ''))[:500],
                          recipe=recipe, version=1, orbit=self.w['orbit'], tags=['personal'])
            self.library[key] = record; self.w['personal_macros'][key] = record
        elif action == 'macro-note':
            record = edit_metadata(self.library[body['id']], {key: body[key] for key in ('name', 'note', 'tags', 'pinned') if key in body})
            self.library[record['id']] = record; self.w['personal_macros'][record['id']] = record
        elif action == 'macro-export':
            return dict(document=export_selected(self.m, self.library, body.get('ids')))
        elif action == 'macro-import-check':
            return inspect_import(self.m, self.library, body.get('document'))
        elif action == 'macro-import':
            imported = inspect_import(self.m, self.library, body.get('document'))
            if not imported['can_import']:
                raise ValueError('Macro import conflict: ' + '; '.join(row['id'] + ': ' + row['reason'] for row in imported['conflict']))
            added = set(imported['added'])
            for record in imported['records']:
                if record['id'] in added:
                    self.library[record['id']] = record
                    self.w['personal_macros'][record['id']] = record
        elif action == 'macro-query':
            return self.macro_query(body.get('query', {}), body.get('text', ''))
        elif action == 'macro-check-library':
            return self.check_library(body.get('limit', 72))
        elif action == 'macro-candidates':
            result = analyse_candidates(self, body.get('candidates'), self.candidate_cache)
            self.candidate_cache = result
            return clone(result)
        elif action == 'template-save':
            name = str(body['name']).strip()[:80]
            if not name:
                raise ValueError('Give the work sheet a name')
            self.w['templates'][name] = build_sheet(self.sheet_inputs(), self.m.model_id)
        elif action == 'template-inspect':
            name = body['name']; sheet = clone(self.w['templates'][name])
            inspection = inspect_sheet(sheet, self.sheet_inputs(), self.library, self.m.model_id)
            for row in inspection['rows']:
                for side in ('saved', 'current'):
                    row[side + '_display'], row[side + '_detail'] = self.sheet_display(row['key'], row[side])
            for source in inspection['macro_sources']:
                source['name'] = self.library.get(source['id'], {}).get('name', source['id'])
            token = secrets.token_hex(16)
            self.sheet_confirmation = dict(token=token, name=name, sheet=sheet, guard=self.guard())
            return dict(sheet=clone(sheet), inspection=inspection, confirmation=token)
        elif action == 'template-use':
            t = self.w['templates'][body['name']]
            inspection = inspect_sheet(t, self.sheet_inputs(), self.library, self.m.model_id)
            if not inspection['legacy']:
                receipt = self.sheet_confirmation
                if (body.get('confirm_inputs') is not True or receipt is None or
                        body.get('confirmation') != receipt['token'] or body['name'] != receipt['name'] or
                        self.guard() != receipt['guard'] or canonical(t) != canonical(receipt['sheet'])):
                    raise ValueError('Work sheet or current inputs changed, or were not confirmed. Inspect the sheet again before loading fixed steps.')
            draft = clone(t['draft']); total = [x for p in PHASES for x in draft[p]]
            if total:
                self.m.normalize(total)
            if t['reference']:
                self.m.normalize([dict(kind='word', moves=t['reference'])])
            self.w.update(draft=draft, reference=clone(t['reference']),
                          draft_sources=clone(t.get('macro_sources', {p: [] for p in PHASES})))
            if not inspection['legacy']:
                self.w['goal'] = t['goal']
        elif action == 'filter':
            expression = str(body['expression'])
            entered_expression = expression
            previous = ''; operation = body.get('operation', 'replace')
            if operation != 'replace':
                applied = self.s.prefs['rules']
                if len(applied) != 1 or applied[0]['style'] != 'solid':
                    raise ValueError('Composition requires a single solid rule. Choose Replace to replace styled or multiple rules explicitly.')
                previous = applied[0]['expr']
            expression = {'replace': expression, 'union': f'({previous}) | ({expression})',
                'intersect': f'({previous}) & ({expression})', 'subtract': f'({previous}) & !({expression})'}[operation]
            rules = [dict(expr=expression, style='solid')]
            result = self.s.filter_preview(dict(rules=rules))
            # Syntax is already checked by the retained parser. Validate only
            # references entered in this edit, without reinterpreting old rules.
            tokens = [match[1].lower() for match in Filters.token.finditer(entered_expression)]
            for i in range(len(tokens) - 3):
                kind = tokens[i]
                if kind in ('piece', 'position') and tokens[i + 1] == '(' and tokens[i + 3] == ')':
                    value = tokens[i + 2]
                    number = int(value[1:] if value.startswith(('c', 'p')) else value)
                    if not 0 <= number < self.m.np:
                        label = 'piece identity' if kind == 'piece' else 'position'
                        raise ValueError(f'{label} must be in 0..{self.m.np - 1}; received {number}.')
            if body.get('preview', False):
                return result
            self.s.filter_apply(rules, result['context_hash']); self.w['filter'] = expression
        elif action == 'save-filter':
            self.w['saved_filters'][str(body['name'])[:60]] = self.w['filter']
        elif action == 'settings':
            if 'phase' in body and body['phase'] not in PHASES:
                raise ValueError('Unknown operation phase')
            if 'input' in body and body['input'] not in ('draft', 'live'):
                raise ValueError('Input destination must be draft or live')
            for key in ['view', 'keybinds']:
                if key in body and not isinstance(body[key], dict):
                    raise ValueError(key + ' settings must be an object')
            if 'view' in body:
                if 'local_center' in body['view']:
                    canonical_cell(body['view']['local_center'])
                if 'grip_mode' in body['view'] and body['view']['grip_mode'] not in ('hold', 'latch'):
                    raise ValueError('Grip mode must be hold or latch')
            for key in ['phase', 'input', 'view', 'keybinds']:
                if key in body:
                    if key == 'view':
                        view = clone(body[key])
                        self.w['view'].update(view)
                    else:
                        self.w[key] = clone(body[key])
        elif action == 'bank-name':
            self.w['bank_overrides'][self.w['bank']] = dict(name=str(body['name'])[:80])
        elif action == 'fixture':
            if body['name'] != 'e1':
                raise ValueError('Unknown fixture')
            if self.s.head != 0:
                raise ValueError('Reset the isolated puzzle explicitly before loading E1')
            recipe = [dict(kind='star', orbit=33, node=11, sign=1), dict(kind='star', orbit=33, node=0, sign=1)]
            if self.s.pending:
                raise ValueError('Cancel the pending preview before loading a fixture')
            # The fixture commits directly and then replaces the live block, so it
            # may only run while no captured position requirement could be broken.
            if self.position_locks():
                raise ValueError('Release all position locks before loading E1')
            p = self.s.preview(recipe, 'E1 explicit legal synthetic fixture', 'synthetic-practice')
            if p['conflicts']:
                self.s.pending = None
                raise ValueError('E1 would move a protected orbit. Remove that protection before loading E1')
            self.s.commit(p['token'])
            workspace = clone(self.w)
            workspace.update(orbit=33, current=35778, target=35778, inspected=35778, inspected_position=None,
                next=dict(identity=26789, target=26789), roles=[2712,175618], bank='33-I',
                draft={p: [] for p in PHASES}, draft_sources={p: [] for p in PHASES}, source='E1 legal synthetic practice; not a human solve', reference=[])
            workspace['block'] = dict(name='Shared-face Home block', members=[dict(identity=x,position=x,labels=self.m.slots(x).tolist()) for x in [35778,26789]],protected=[])
            rules = [{'expr':'active','style':'solid'}]
            workspace['filter'] = rules[0]['expr']
            self.save_warnings.append(self.persist_prefs({'orbit': 33, 'rules': rules, 'layout': self.workspace_layout(workspace)}))
            self.w = workspace
        elif action == 'grips':
            return self.cap_axes(body['cell'])
        else:
            raise ValueError('Unknown experimental command')
        if action in ('operation-new', 'operation-reuse', 'template-use'):
            self.w['operation_generation'] = secrets.token_hex(12)
            self.w.pop('completed_operation', None)
        workspace_saved = False
        try:
            save_warning = '\n'.join(filter(None, self.save_warnings + [self.save()]))
            if save_warning:
                workspace_saved = True
                command_warning = command_warning + '\n' + save_warning if command_warning else save_warning
        except Exception as error:
            if not committed_operation:
                if action == 'preview':
                    self.s.pending, self.review, self.execution, self.residual_cache = preview_prior
                raise
            message = 'Operation committed; workspace preferences could not be saved: ' + str(error)
            command_warning = command_warning + '\n' + message if command_warning else message
        if action in ('operation-new', 'operation-reuse', 'template-use'):
            self.completed_operations.pop(self.w['orbit'], None)
        if action in ('operation-new', 'operation-reuse', 'template-use') or before_intent != canonical(dict(
                intent=self.intent(), protected=self.s.prefs['protected'], position_locks=self.position_locks())):
            # Returning to earlier values must not revive old review/commit authority.
            self.epoch = secrets.token_hex(12)
        if action in ('operation-new', 'operation-reuse', 'template-use'):
            self.review = None
            self.execution = None
            self.sheet_confirmation = None
        if response_snapshot:
            result = self.snapshot(render=action in ['commit','twist','draft','inverse-cleanup','template-use','undo','redo','reset','restore','filter','fixture','focus','bank','orbit'])
            if command_warning:
                result['command_warning'] = command_warning
            return result
        if command_warning:
            result = dict(warning=command_warning)
            if committed_operation:
                result['committed'] = True
            if workspace_saved:
                result['saved'] = True
            return result
        return None

    def switch_orbit(self, o, restore=False, update=None):
        integer(o, 35, 'Orbit')
        if o == self.w['orbit']:
            self.w.update(update or {})
            return
        # Orbit, workspace and the caller's follow-up fields are stored in one write,
        # so a failed save cannot split them or keep a half-finished switch.
        workspace = switch_context(self.m, self.w, o)
        workspace.update(update or {})
        self.save_warnings.append(self.persist_prefs({'orbit': o, 'layout': self.workspace_layout(workspace)}))
        self.w = workspace

    def applicability(self, facts):
        star = facts['star']; reasons = []
        role_status = 'Not declared'
        if star and star['target'] != self.w['target']:
            reasons.append('Fixed macro target differs from current destination')
        if star and star['orbit'] != self.w['orbit']:
            reasons.append('Macro acts on another primary orbit')
        if star:
            role_status = 'Matched' if self.w['roles'] == [star['a'], star['b']] else 'Mismatch'
            if role_status == 'Mismatch':
                reasons.append('Assigned A/B positions differ from the certified star buffers')
            if self.w['target'] in self.w['roles']:
                role_status = 'Mismatch'
                reasons.append('Assigned target collides with a buffer position')
        direction = None; frame_status = 'Unspecified'
        if self.w['current'] is None:
            reasons.append('Current identity is not assigned')
        else:
            identity = self.w['current']; position = int(self.s.st.where[identity])
            src, dst, _ = self.effect(facts['recipe'])
            # Complete sparse action, independent of truncated UI cycle lists.
            transport = self.m.ids.copy(); transport[src] = dst
            after = int(self.m.sp[transport[self.m.slots(position)[0]]])
            target = self.w['target']; reaches = target is not None and after == target
            direction = dict(identity=identity, before=position, after=after, reaches_target=reaches)
            if target is None:
                reasons.append('Destination is not assigned')
            elif not reaches:
                reasons.append('This macro body sends Current to another position; the complete operation may differ')
            else:
                requirement = self.w.get('target_requirement')
                required = (requirement['labels'] if requirement and requirement['identity'] == identity and requirement['position'] == target
                            else self.m.slots(target) if target == identity else None)
                if required is None:
                    reasons.append('Destination frame has no explicit exact requirement')
                else:
                    predicted = self.s.st.labels.copy(); predicted[dst] = self.s.st.labels[src]
                    frame_status = 'Matched' if np.array_equal(predicted[self.m.slots(target)], required) else 'Mismatch'
                    if frame_status == 'Mismatch':
                        reasons.append('Macro body reaches the destination but its exact orientation labels differ')
        return dict(reasons=reasons, roles=role_status, direction=direction, frame=frame_status,
                    scope='Selected macro body only; no preparation or cleanup assumed',
                    protection='Review the complete operation; body effect is not execution permission')


def install(handler, context):
    service = Workbench(context['session'], context['lock'], workflow=context['workflow'],
                        native_profile=lambda: context['native_profile'][0])
    old_get, old_post = handler.do_GET, handler.do_POST
    token = context['token']; jobs = context['jobs']; pool = context['pool']; cancel = context['cancel_event']
    submit_lock = __import__('threading').Lock()
    inspection_reuse = _NativeInspectionReuse()
    timing_path = os.environ.get('MAGIC600_BACKEND_PROFILE')
    timing = _BackendTiming(timing_path) if timing_path and Path(timing_path).is_absolute() else None

    def timed(name, call, *args, **kwargs):
        return timing.measure(name, call, *args, **kwargs) if timing else call(*args, **kwargs)

    def native_reply(result, since, inspection=None, inspection_error=None,
                     cycle_projection=None, cycle_projection_error=None, cycle_projection_note=None, prediction=True):
        # Caller holds the authoritative Session lock. Both views describe the
        # same committed state; native arrays never come from a second Session.
        profile = context['native_profile'][0]
        if profile is None:
            raise ValueError('Native bridge has not passed')
        work = timed('reply.snapshot', service.snapshot, prediction=prediction)
        if 'prediction_error' in work:
            message = 'Complete-operation forecast unavailable: ' + work.pop('prediction_error')
            inspection_error = inspection_error + '; ' + message if inspection_error else message
            inspection = None
        cell = service.w.get('view', {}).get('local_center', 1)
        local_state = timed('reply.local', lambda: cell_state(service, cell, service.selected_frame(service.w['bank'], cell)))
        return dict(result=result, work=work, local_cell=local_state,
                    phase_inspection=inspection, phase_inspection_error=inspection_error,
                    cycle_projection=cycle_projection, cycle_projection_error=cycle_projection_error,
                    cycle_projection_note=cycle_projection_note,
                    native_snapshot=timed('reply.native_snapshot', context['native_snapshots'].read, context['session'], profile, since, 2))

    def native_input(body):
        profile = context['native_profile'][0]
        session = context['session']; model = context['model']
        if profile is None or body.get('profile_sha256') != profile['profile_sha256']:
            raise ValueError('Native profile is missing or stale; reconnect')
        if body.get('state_hash') != session.st.hash:
            raise ValueError('Native view is stale; refresh before input')
        if body.get('action') == 'inspect':
            slot = body.get('native_sticker')
            if type(slot) is not int or not 0 <= slot < model.n:
                raise ValueError('Native sticker must be an in-range native slot')
            lab = int(profile['native_to_lab'][slot])
            if session.interactive_styles()[lab] == 0:
                raise ValueError('Hidden native sticker is not an interactive target')
            position = int(model.sp[lab])
            if body.get('gesture') == 'shift-right':
                return service.command(dict(action='inspect-position', position=position), response_snapshot=False)
            if body.get('gesture') != 'shift-left':
                raise ValueError('Inspection gesture must be shift-left or shift-right')
            return service.command(dict(action='inspect', identity=int(session.st.at[position])), response_snapshot=False)
        if body.get('action') != 'turn':
            raise ValueError('Native input supports explicit turn or inspection only')
        tokens = body.get('tokens')
        if not isinstance(tokens, list) or not 1 <= len(tokens) <= 10000:
            raise ValueError('Native turn requires 1 to 10000 explicit tokens')
        word = []
        for item in tokens:
            if not isinstance(item, str) or item not in profile['token_words']:
                raise ValueError('Native token is not in the verified generator mapping')
            word.extend(profile['token_words'][item])
        return service.command(dict(action='native-word', moves=word,
            state_hash=body['state_hash'], destination=body.get('destination'), phase=body.get('phase')), response_snapshot=False)

    def get(self):
        try:
            path = urlparse(self.path).path
            if not self.valid_host():
                return self.js({'error': 'Invalid Host'}, 403)
            if path == '/':
                if not secrets.compare_digest(parse_qs(urlparse(self.path).query).get('token', [''])[0], token):
                    return self.js({'error': 'Use the experiment launcher'}, 403)
                return self.send(200, (HERE / 'web' / 'index.html').read_bytes(), 'text/html; charset=utf-8')
            if path.startswith('/experiment/'):
                name = path.removeprefix('/experiment/')
                if name not in ['app.js', 'style.css', 'keys.js', 'graphics.js', 'ids.js']:
                    return self.js({'error': 'Not found'}, 404)
                return self.send(200, (HERE / 'web' / name).read_bytes(), 'text/css' if name.endswith('.css') else 'text/javascript')
            if path == '/api/experiment/snapshot':
                if not self.authenticated():
                    return self.js({'error':'Unauthorized'}, 403)
                with context['lock']:
                    return self.js(service.snapshot(render=True))
            if path == '/api/experiment/native-snapshot':
                if not self.authenticated():
                    return self.js({'error': 'Unauthorized'}, 403)
                since = parse_qs(urlparse(self.path).query, keep_blank_values=True).get('since', [None])
                if len(since) != 1 or (since[0] is not None and (not since[0] or len(since[0]) > 128)):
                    raise ValueError('Invalid native predecessor revision')
                with context['lock']:
                    return self.js(native_reply(None, since[0]))
            if path == '/api/structure':
                if not self.authenticated():
                    return self.js({'error': 'Unauthorized'}, 403)
                structure = context['model'].structure()
                structure['examples'] = dict(five=int(np.flatnonzero((context['model'].k == 5) & (context['model'].oid >= 0))[0]))
                structure['local_geometry'] = sticker_geometry(context['model'])
                structure['cell_names'] = [service.names.cell(c) for c in range(1, 601)]
                structure['orbit_profiles'] = [dict(orbit=r['id'], names=service.names.orbit(r['id']), hosting_count=r['colors'],
                    affecting_cap_count=len(r['mask']), orientation_group=r['orientation_group'],
                    pieces=r['pieces'], hosting_cells=[c+1 for c in r['faces']],
                    affecting_caps=[c+1 for c in r['mask']]) for r in context['model'].census['orbits']]
                return self.js(structure)
            return old_get(self)
        except (ValueError, KeyError, TypeError) as error:
            return self.js({'error': str(error)}, 400)

    def post(self):
        path = urlparse(self.path).path
        if path in ['/api/shutdown', '/api/stop-job', '/api/native/handshake']:
            return old_post(self)
        if path not in ('/api/experiment/command', '/api/experiment/native-command', '/api/experiment/native-input', '/api/experiment/window-layout', '/api/experiment/session-log'):
            return self.js({'error': 'Use the explicit experimental command boundary; automatic solver routes are unavailable.'}, 403)
        try:
            if not self.valid_host() or not self.authenticated():
                return self.js({'error': 'Unauthorized'}, 403)
            if self.headers.get('Origin') not in [None, 'http://' + self.headers.get('Host', '')]:
                return self.js({'error': 'Foreign origin rejected'}, 403)
            size = int(self.headers.get('Content-Length', '0'))
            limit = MAX_LOG_BASE64_BYTES + 65536 if path == '/api/experiment/session-log' else 2 * 1024 * 1024
            if not 0 < size <= limit or 'application/json' not in self.headers.get('Content-Type', ''):
                raise ValueError('A bounded JSON command is required')
            body = json.loads(self.rfile.read(size))
            if not isinstance(body, dict):
                raise ValueError('Command must be an object')
            if path == '/api/experiment/session-log' and body.get('action') not in ('session-log-inspect', 'session-log-apply', 'session-log-export'):
                raise ValueError('The bounded log route accepts only log inspection, confirmed import or export')
            if path == '/api/experiment/window-layout':
                # Preference traffic must neither reserve a solving job nor publish
                # a stale paired snapshot. Yield to an active operation immediately.
                if not context['lock'].acquire(blocking=False):
                    return self.js(dict(saved=False, generation=body.get('generation')))
                status = 200
                try:
                    try:
                        result = service.save_window_layout(body)
                    except (ValueError, KeyError, TypeError):
                        raise
                    except Exception as error:
                        result, status = {'error': 'Window layout was not saved: ' + str(error)}, 500
                finally:
                    context['lock'].release()
                # A slow socket must not retain authority over the solving state.
                return self.js(result, status)
            is_native = path != '/api/experiment/command'
            boundary = body.pop('inspect_boundary', None)
            macro_id = body.pop('inspect_macro', None)
            cycle_display = validate_cycle_display(context['model'], body.pop('inspect_cycle', None))
            if boundary is not None and (not isinstance(boundary, str) or boundary not in BOUNDARIES):
                raise ValueError('Inspection boundary must be actual, prepare, macro or cleanup')
            if macro_id is not None and not isinstance(macro_id, str):
                raise ValueError('Inspection macro must identify an existing fixed entry')
            since = body.pop('native_since', None)
            if since is not None and (not isinstance(since, str) or not since or len(since) > 128):
                raise ValueError('Invalid native predecessor revision')
            def work():
                with context['lock']:
                    if timing:
                        timing.locked(context['session'])
                    context['model'].cancel_event = cancel
                    try:
                        if is_native and context['native_profile'][0] is None:
                            raise ValueError('Native bridge has not passed')
                        rev_before = context['session'].rev
                        result = timed('command', lambda: native_input(body) if path.endswith('/native-input') else service.command(body, response_snapshot=not is_native))
                        if not is_native:
                            return result
                        inspection, inspection_error = None, None
                        if boundary is not None:
                            try:
                                inspection = timed('inspect_phase', inspection_reuse.read, 'phase', inspect_phase, service, boundary, macro_id)
                            except Exception as error:
                                # An independent display failure must never turn a
                                # successful commit into a reported failed action.
                                inspection_error = ('Phase inspection cancelled after the primary action completed.'
                                                    if isinstance(error, InterruptedError) else str(error))
                        cycle_projection, cycle_error, cycle_note = None, None, None
                        if cycle_display is not None:
                            try:
                                cycle_projection, cycle_note = timed('inspect_cycles', inspection_reuse.read, 'cycles', inspect_cycle_display, service, cycle_display, macro_id)
                            except Exception as error:
                                # Display analysis cannot revoke or misreport an
                                # already committed primary operation.
                                cycle_error = ('Cycle display cancelled after the primary action completed.'
                                               if isinstance(error, InterruptedError) else str(error))
                        try:
                            try:
                                return timed('native_reply', native_reply, result, since, inspection, inspection_error,
                                                    cycle_projection, cycle_error, cycle_note)
                            except InterruptedError:
                                # The primary action already completed. Stop cancels optional
                                # analysis, not its authoritative reply or durable journal result.
                                # Keep the shared Stop event set; finish only the current-state
                                # refresh under this same lock, without another forecast.
                                context['model'].cancel_event = None
                                message = 'Complete-operation forecast unavailable: optional analysis cancelled after the primary action completed.'
                                inspection_error = '; '.join(filter(None, [inspection_error, message]))
                                return timed('native_reply', native_reply, result, since, None, inspection_error,
                                                    cycle_projection, cycle_error, cycle_note, prediction=False)
                        except Exception as error:
                            imported = isinstance(result, dict) and result.get('import_applied') is True
                            if not imported and context['session'].rev == rev_before:
                                raise
                            # The journal change is already durable. Rendering a paired
                            # reply cannot turn it into a failed import or operation.
                            result = dict(result if isinstance(result, dict) else {})
                            if not imported:
                                result['committed'] = True
                            result['warning'] = '; '.join(filter(None, [result.get('warning'),
                                ('Log imported' if imported else 'Operation committed') + '; native display refresh failed: ' + str(error)]))
                            return dict(result=result, requires_refresh=True)
                    finally:
                        context['model'].cancel_event = None
                        if timing:
                            timing.unlocked(context['session'])
            with submit_lock:
                if any(not future.done() for future in jobs.values()):
                    return self.js({'error': 'An operation is busy; input was not queued.'}, 409)
                for key in list(jobs):
                    if len(jobs) > 20 and jobs[key].done():
                        del jobs[key]
                cancel.clear(); key = secrets.token_hex(10)
                jobs[key] = pool.submit(timing.wrap(work, path, body.get('action')) if timing else work)
            return self.js({'job': key})
        except (ValueError, KeyError, TypeError) as error:
            return self.js({'error': str(error)}, 400)
    handler.do_GET, handler.do_POST = get, post
