"""Replay runner-run/1, /2 and /3 records without runner/search/step code.

Usage: python3 -I -B check_run.py RUN [--out FILE] [--verify-seconds S] [--allow-unchecked-contracts]
Only the existing independent lattice verifier is run as a child process. Its
report goes in a fresh check/ subdirectory of RUN (or FILE's parent). All other
inputs are read-only. Failure names in the JSON lines identify the checked field
or contract; the final ``fails`` also includes failures from individual steps.
This checks the words played, not the runner's choice of rule or its budgets.
For runner-run/3 it also checks every step's tree of named parts, the fixed phase
order, every entry test from its own snapshots, the probe and the acceptance verdict
(plan runner-run/3, sections 0 to 2). Height and kernel contracts go to
check_contracts.py and lattice steps to check_lattice.py; a contract they leave
unchecked is counted as ``unchecked_contract`` and fails the record unless
--allow-unchecked-contracts is given.
Evidence: exact simulator replay, not Windows/DirectX or performance evidence.
"""

import argparse
import gzip
import hashlib
import json
import math
import os
from pathlib import Path, PurePosixPath, PureWindowsPath
import re
import signal
import subprocess
import sys
import tempfile
import zlib

sys.dont_write_bytecode = True
REPO = Path(__file__).resolve().parents[5]
J = REPO / 'research/jumbling'
sys.path[:0] = [str(J / 'theory'), str(J), str(J / 'fixtures'), str(Path(__file__).resolve().parent)]

import numpy as np  # noqa: E402
import restore  # noqa: E402
import rim  # noqa: E402
import sim  # noqa: E402
import trees  # noqa: E402
from sim.kplus import matrix_from_json  # noqa: E402
from sim.state import Pose  # noqa: E402

import check_contracts  # noqa: E402
import check_lattice  # noqa: E402

SOLVED = 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855'
FORBIDDEN = {'segments', 'slp', 'certificate', 'node'}
PATH_KEYS = {'file', 'word', 'unit', 'table', 'library', 'out'}
PHASES = ('bulk', 'tail', 'fallback')
VERIFY_LATE = J / 'restoration/09-general-method/scripts/lattice/checker/verify_late.py'
V_KEYS = {'V_source', 'V_letters', 'V_sha256'}
CLOSURE_KEYS = V_KEYS | {'parts'}

# runner-run/3 (plan runner-run/3, sections 0 and 1).
FORMATS = ('runner-run/1', 'runner-run/2', 'runner-run/3')
V3_PHASES = ('bulk', 'height', 'high', 'tail', 'fallback', 'lattice')
PHASE_ORDER = ('bulk', 'height', 'high', 'tail', 'lattice')
SUBPHASES = ('r22d75', 'r22d81', 'h4')
# The fixed order of positions; fallback steps sit in the tail's position.
POSITIONS = (('bulk', None), ('height', 'r22d75'), ('height', 'r22d81'), ('height', 'h4'),
             ('high', None), ('tail', None), ('lattice', None))
# reopen reads the solver's earlier twists, not the state, so v3 leaves it out.
BULK_RULES = ('seam', 'cover', 'close', 'conj', 'lock', 'improve', 'shift')
MODES = ('acceptance', 'development')
STANDINGS = ('rule', 'certificate', 'fallback')
PHASE_RESULTS = ('lattice', 'handoff', 'not_applicable', 'stuck', 'budget', 'timeout', 'error')
RUN_ENDS = ('budget', 'timeout', 'error')
# Clocks that acceptance mode lifts: null in config.budgets, the sentinel for the children.
CLOCKS = ('bulk_step_seconds', 'bulk_seconds', 'rule_seconds', 'greedy_seconds', 'cert_seconds',
          'height_seconds', 'height_cpu', 'kernel_seconds')
CLOCK_SENTINEL = 1e9
STEP_KEYS = {'index', 'phase', 'subphase', 'rule', 'word', 'letters', 'encoding', 'word_sha256', 'tree',
             'standing', 'before', 'after', 'seconds'}
PLAYED_KEYS = {'contract', 'gate', 'attempts'}          # every step outside the bulk phase
ACCEPTANCE_KEYS = {'verdict', 'certificate_steps', 'fallback_steps', 'probe', 'timed_out_attempts'}
# The committed options of every acceptance run (plan revision 3e, E3).
ACCEPTANCE_CONFIG = 'research/jumbling/restoration/13-tutorial-runner/acceptance-config.json'
ACCEPTANCE_OPTIONS = ('literal_limit', 'seeds', 'tail_rules', 'lattice_restorer', 'fallback_max_length', 'par',
                      'pair_shards', 'bulk_steps', 'tail_steps', 'high_steps', 'no_lattice')
LATTICE_ENDS = ('stuck', 'budget', 'timeout', 'error')
# The state-only D19 rule the kernel plugin passes (rules_kernel.py, D19).
D19 = 'research/jumbling/restoration/12-low-height/scripts/round22/r22d19_rule.py'
TREE_KEYS = {'name', 'letters', 'parts', 'span', 'inverse_of', 'same_as'}
TREE_MAX_DEPTH = 64
# A contract that check_contracts.py or check_lattice.py does not check yet.
UNCHECKED = 'unchecked_contract'


def integer(x, minimum=0):
    return type(x) is int and x >= minimum


def digest(x):
    return isinstance(x, str) and re.fullmatch(r'[0-9a-fA-F]{64}', x) is not None


def positive(x):
    return (type(x) is int and x > 0) or (type(x) is float and x > 0 and math.isfinite(x))


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
            if key in PATH_KEYS and value is not None and not (key == 'unit' and type(value) is int):
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
        # Counted only while this snapshot is the current state; the replay drops
        # ``st`` from an earlier snapshot, which then has no N to compare.
        if self._n is None and self.st is not None:
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


def check_v2_config(record, fails):
    config = record.get('config')
    if not isinstance(config, dict):
        add(fails, 'config')
    else:
        budgets, source = config.get('budgets'), config.get('source')
        if (not integer(config.get('literal_limit')) or config.get('seeds') not in ('atlas', 'default')
                or not isinstance(budgets, dict) or not positive(budgets.get('cert_seconds'))
                or not isinstance(source, dict)):
            add(fails, 'config')
        check_source(source, fails)
    check_error(record, fails)


def check_source(source, fails):
    if isinstance(source, dict):
        commit, sha, dirty = source.get('commit'), source.get('status_sha256'), source.get('dirty')
        if (not {'commit', 'status_sha256', 'dirty'}.issubset(source)
                or (commit is not None and (not isinstance(commit, str)
                    or re.fullmatch(r'[0-9a-fA-F]{40}', commit) is None))
                or (sha is not None and not digest(sha))
                or (dirty is not None and type(dirty) is not bool)):
            add(fails, 'config')


def check_error(record, fails):
    error = record.get('error')
    if ('error' not in record or (error is not None and (not isinstance(error, str)
            or '/' in error or private_path(error) or record.get('result') != 'error'))):
        add(fails, 'error')


def check_acceptance_config(config, fails):
    """An acceptance run used the committed options of this checkout (plan revision 3e, E3)."""
    frozen = config.get('acceptance_config')
    if config.get('mode') != 'acceptance':
        if frozen is not None:
            add(fails, 'acceptance_config')
        return
    try:
        raw = (REPO / ACCEPTANCE_CONFIG).read_bytes()
        committed = json.loads(raw)['options']
    except (OSError, ValueError, KeyError, TypeError):
        add(fails, 'acceptance_config')
        return
    budgets = config.get('budgets') if isinstance(config.get('budgets'), dict) else {}
    lattice = config.get('lattice') if isinstance(config.get('lattice'), dict) else {}
    recorded = {'literal_limit': config.get('literal_limit'), 'seeds': config.get('seeds'),
                'tail_rules': config.get('tail_rules'), 'lattice_restorer': lattice.get('restorer'),
                **{name: budgets.get(name, 'missing') for name in
                   ('fallback_max_length', 'par', 'pair_shards', 'bulk_steps', 'tail_steps', 'high_steps')}}
    # config.budgets records an unlimited tail as null (runner.py, initialize).
    expected = dict(committed, tail_steps=committed.get('tail_steps') or None) if isinstance(committed, dict) else {}
    if (not isinstance(frozen, dict) or set(frozen) != {'file', 'sha256', 'options'}
            or frozen['file'] != ACCEPTANCE_CONFIG or frozen['sha256'] != hashlib.sha256(raw).hexdigest()
            or frozen['options'] != committed or not isinstance(committed, dict)
            or set(committed) != set(ACCEPTANCE_OPTIONS) or committed['no_lattice'] is not False
            or any(recorded[name] != expected[name] for name in recorded)):
        add(fails, 'acceptance_config')


def check_kernel_d19(config, fails):
    """The D19 rule a run's kernel used is this checkout's; an acceptance run records it (revision 3e, E2)."""
    kernel = config.get('kernel') if isinstance(config.get('kernel'), dict) else {}
    d19 = kernel.get('d19')
    if d19 is None and config.get('mode') != 'acceptance':
        return
    try:
        sha = hashlib.sha256((REPO / D19).read_bytes()).hexdigest()
    except OSError:
        sha = None
    if sha is None or d19 != {'file': D19, 'sha256': sha}:
        add(fails, 'kernel_d19')


def check_v3_config(record, fails):
    """Format 3 configuration: mode, component, the fixed orders and the lifted clocks."""
    config = record.get('config')
    if not isinstance(config, dict):
        add(fails, 'config')
        check_error(record, fails)
        return {}
    budgets, source, kernel = config.get('budgets'), config.get('source'), config.get('kernel')
    mode, component, lattice = config.get('mode'), config.get('component'), config.get('lattice')
    if (mode not in MODES or not integer(config.get('literal_limit')) or config.get('seeds') not in ('atlas', 'default')
            or not isinstance(budgets, dict) or not isinstance(source, dict) or not isinstance(kernel, dict)
            or kernel.get('max_steps') != 1 or not isinstance(lattice, dict)
            or lattice.get('restorer') not in ('late', 'schedule11')
            or (component is not None and (not isinstance(component, str) or not component or '/' in component))):
        add(fails, 'config')
    elif mode == 'acceptance':
        if (any(name not in budgets or budgets[name] is not None for name in CLOCKS)
                or config.get('clock_sentinel') != CLOCK_SENTINEL or kernel.get('seconds') is not None):
            add(fails, 'config')
    elif config.get('clock_sentinel') is not None or not positive(budgets.get('cert_seconds')):
        add(fails, 'config')
    if config.get('phase_order') != list(PHASE_ORDER) or config.get('subphase_order') != {'height': list(SUBPHASES)}:
        add(fails, 'phase_order')
    order = config.get('bulk_order')
    if not isinstance(order, list) or any(rule not in BULK_RULES for rule in order) or len(set(order)) != len(order):
        add(fails, 'bulk_order')
    check_acceptance_config(config, fails)
    check_kernel_d19(config, fails)
    check_source(source, fails)
    check_error(record, fails)
    return config


def check_v2_step(step, doc, menu, fails):
    """Check recorded literal pieces, without importing a closure/certificate builder."""
    if doc is not None and (not integer(step.get('letters')) or step['letters'] != len(doc['word'])):
        add(fails, 'letters')
    if step.get('phase') in ('tail', 'fallback'):
        gate = step.get('gate')
        if (gate not in ('stepchk', 'slp_apply')
                or (gate == 'slp_apply') != (step.get('V_source') == 'certificate')):
            add(fails, 'gate')
    if step.get('V_source') == 'certificate':
        if (not integer(step.get('T_size'), 1) or not digest(step.get('T_digest'))
                or step.get('N1') is not True or step.get('N2') is not True):
            add(fails, 'certificate_keys')
    closure = step.get('phase') == 'tail' and step.get('rule') in ('close', 't1prime')
    if not closure:
        if CLOSURE_KEYS.intersection(step):
            add(fails, 'closure_keys')
        return
    has_v = bool(V_KEYS.intersection(step))
    parts = step.get('parts')
    w_only = (isinstance(parts, list) and len(parts) == 1 and isinstance(parts[0], list)
              and len(parts[0]) == 2 and parts[0][0] == 'W')
    if ('parts' not in step or ((step['rule'] == 'close' or has_v) and not V_KEYS.issubset(step))
            or (step['rule'] == 't1prime' and not has_v and not w_only)
            or (has_v and step.get('V_source') not in ('tclose2', 'certificate'))):
        add(fails, 'closure_keys')
    if doc is None:
        return
    word, parts = doc['word'], step.get('parts')
    names = ['V', 'middle', 'V_inverse'] if step['rule'] == 'close' else (
        ['W', 'V', 'middle', 'V_inverse'] if has_v else ['W'])
    if (not isinstance(parts, list)
            or any(not isinstance(p, list) or len(p) != 2 or not integer(p[1]) for p in parts)
            or [p[0] for p in parts] != names
            or sum(p[1] for p in parts) != len(word)
            or not integer(step.get('letters')) or sum(p[1] for p in parts) != step['letters']):
        add(fails, 'parts')
        return
    if not has_v:
        if step['rule'] == 'close':
            add(fails, 'closure_keys')
        return
    pieces, pos = {}, 0
    for name, length in parts:
        pieces[name] = word[pos:pos + length]
        pos += length
    V = pieces['V']
    if (not integer(step.get('V_letters')) or len(V) != step['V_letters']
            or not digest(step.get('V_sha256'))
            or hashlib.sha256(json.dumps(V).encode('ascii')).hexdigest() != step['V_sha256']):
        add(fails, 'V_sha256')
    lookup = {restore.to_tuple(m): i for i, (_, m) in enumerate(menu.items)}
    inverses = [lookup[restore.to_tuple(restore.transpose(m))] for _, m in menu.items]

    def inverted(piece):
        return [[c, inverses[i]] for c, i in reversed(piece)]

    if pieces['V_inverse'] != inverted(V):
        add(fails, 'V_inverse')
    if step['rule'] == 't1prime' and pieces['middle'] != inverted(pieces['W']):
        add(fails, 'middle')


def word_sha256(word):
    return hashlib.sha256(json.dumps(word).encode('ascii')).hexdigest()


def inverse_items(menu):
    """The menu item of each item's inverse, through the exact matrices."""
    lookup = {restore.to_tuple(m): i for i, (_, m) in enumerate(menu.items)}
    return [lookup[restore.to_tuple(restore.transpose(m))] for _, m in menu.items]


def valid_attempts(attempts):
    return isinstance(attempts, list) and all(
        isinstance(a, dict) and type(a.get('timed_out')) is bool for a in attempts)


def check_v3_step(step, config, fails):
    """Format 3 labels: keys, phase and sub-phase, encoding, standing, contract and gate."""
    phase, gate, mode = step.get('phase'), step.get('gate'), config.get('mode')
    seconds = step.get('seconds')
    if (not STEP_KEYS.issubset(step) or (phase != 'bulk' and not PLAYED_KEYS.issubset(step))
            or type(seconds) not in (int, float) or not math.isfinite(seconds) or seconds < 0):
        add(fails, 'step_keys')
    if phase not in V3_PHASES:
        add(fails, 'phase')
    elif (step.get('subphase') not in SUBPHASES) if phase == 'height' else (step.get('subphase') is not None):
        add(fails, 'subphase')
    # A lattice word can be long; check_lattice.py streams it from an ia16 file.
    if step.get('encoding') not in ('json', 'ia16') or (phase == 'lattice' and step['encoding'] != 'ia16'):
        add(fails, 'encoding')
    if not digest(step.get('word_sha256')):
        add(fails, 'word_sha256')
    # Development mode takes a certificate step's gate table (standing certificate) and plays
    # fallback steps; acceptance mode plays the certificate step's literal word (standing
    # rule) and runs the fallback only as a probe (plan section 0, acceptance labelling).
    standing = step.get('standing')
    if (standing not in STANDINGS or (standing == 'fallback') != (phase == 'fallback')
            or (standing == 'certificate') != (gate == 'slp_apply' and mode == 'development')
            or (phase == 'fallback' and mode != 'development')):
        add(fails, 'standing')
    if ((gate == 'slp_apply') != (step.get('construction') == 'certificate')
            or ('construction' in step and not digest(step.get('certificate_sha256')))):
        add(fails, 'certificate_keys')
    if phase == 'bulk' and step.get('rule') not in (config.get('bulk_order') or ()):
        add(fails, 'bulk_order')
    if phase != 'bulk' and not valid_attempts(step.get('attempts')):
        add(fails, 'attempts')
    contract = step.get('contract')
    if phase in ('height', 'high', 'lattice') and (
            not isinstance(contract, str) or not contract or not isinstance(gate, str) or not gate
            or (phase == 'high' and (contract not in ('kernel_landing', 'kernel_lowering') or gate != 'r22d25'))
            or (phase == 'lattice' and (contract != 'lattice_literal' or gate != 'player'))):
        add(fails, 'contract')
    if 'labels_identity' in step and (phase != 'lattice' or type(step['labels_identity']) is not bool):
        add(fails, 'labels_identity')
    # Each kernel call hashes the D19 rule it ran; it must be the rule the configuration records (I1R-03).
    d19 = config['kernel'].get('d19') if isinstance(config.get('kernel'), dict) else None
    if phase == 'high' and isinstance(d19, dict) and (
            not isinstance(step.get('kernel'), dict) or step['kernel'].get('d19_sha256') != d19.get('sha256')
            or not digest(step['kernel']['d19_sha256'])):
        add(fails, 'kernel_d19')


def load_word_v3(base, step, fails):
    """A json word as in format 2, with its sha256; an ia16 word through check_lattice.read_ia16."""
    if step.get('encoding') != 'ia16':
        doc = load_word(base, step, fails)
        if doc is not None and word_sha256(doc['word']) != step.get('word_sha256'):
            add(fails, 'word_sha256')
        return doc
    try:
        path = inside(base, step.get('word'))
    except (ValueError, OSError, RuntimeError):
        add(fails, 'path')
        return None
    try:
        word = check_lattice.read_ia16(path, str(step.get('word_sha256')))
    except check_lattice.Sha256Mismatch:
        add(fails, 'word_sha256')
        return None
    except (ValueError, OSError, EOFError, zlib.error):
        add(fails, 'word_file')
        return None
    # An ia16 file holds only the codes; the replay checks the row's own digests.
    ends = [doc.get('digest') if isinstance(doc, dict) else None for doc in (step.get('before'), step.get('after'))]
    return {'word': word, 'input_digest': ends[0], 'output_digest': ends[1]}


def check_tree(base, step, word, inverses, fails):
    """The step's tree of named parts expands to exactly its word (plan section 1, step tree).

    ``word`` is the literal word, or None when the checker does not hold it (a lattice
    step, whose letter ranges check_lattice.py checks): then only the structure is checked."""
    tree = step.get('tree')
    if not isinstance(tree, dict) or set(tree) != {'file', 'sha256', 'size'}:
        add(fails, 'tree_missing')
        return
    try:
        path = inside(base, tree['file'])
    except (ValueError, OSError, RuntimeError):
        add(fails, 'path')
        return
    try:
        raw = path.read_bytes()
    except OSError:
        add(fails, 'tree_missing')
        return
    try:
        if raw[:2] != b'\x1f\x8b' or raw[4:8] != bytes(4):
            raise ValueError('a tree file is gzip with mtime 0')
        data = gzip.decompress(raw)
    except (ValueError, OSError, EOFError, zlib.error):
        add(fails, 'tree_malformed')
        return
    if not digest(tree['sha256']) or hashlib.sha256(data).hexdigest() != tree['sha256']:
        add(fails, 'tree_sha256')
        return
    try:
        root = json.loads(data, object_pairs_hook=no_duplicates)
    except (ValueError, RecursionError):
        add(fails, 'tree_malformed')
        return
    # Depth-first walk without recursion: every part with its index path and first letter.
    parts, at, stack = [], 0, [(root, (), 1)]
    while stack:
        part, path, depth = stack.pop()
        if (not isinstance(part, dict) or depth > TREE_MAX_DEPTH or not TREE_KEYS.issuperset(part)
                or not isinstance(part.get('name'), str) or not part['name'] or not integer(part.get('letters'))
                or ('span' in part) == ('parts' in part) or ('inverse_of' in part and 'same_as' in part)):
            if isinstance(part, dict) and FORBIDDEN.intersection(part):
                add(fails, 'forbidden_key')
            add(fails, 'tree_malformed')
            return
        if 'span' in part:
            span = part['span']
            if not isinstance(span, list) or len(span) != 2 or not all(integer(x) for x in span) or span[1] < span[0]:
                add(fails, 'tree_malformed')
                return
            if part['letters'] != span[1] - span[0]:
                add(fails, 'tree_length')
            if span[0] != at:
                add(fails, 'tree_expansion')            # a gap or an overlap
            parts.append((path, part, span[0]))
            at = span[1]
        else:
            children = part['parts']
            if not isinstance(children, list) or not children:
                add(fails, 'tree_malformed')
                return
            if (all(isinstance(c, dict) and integer(c.get('letters')) for c in children)
                    and part['letters'] != sum(c['letters'] for c in children)):
                add(fails, 'tree_length')
            parts.append((path, part, at))
            stack.extend((children[i], path + (i,), depth + 1) for i in reversed(range(len(children))))
    if at != root['letters']:
        add(fails, 'tree_expansion')
    if root['letters'] != step.get('letters'):
        add(fails, 'tree_length')
    if not integer(tree['size']) or tree['size'] != len(parts):
        add(fails, 'tree_malformed')
    # inverse_of and same_as name an earlier part, wholly before this one, of the same length.
    order = {path: n for n, (path, _, _) in enumerate(parts)}
    for n, (path, part, start) in enumerate(parts):
        for key in ('inverse_of', 'same_as'):
            if key not in part:
                continue
            ref = part[key]
            other = order.get(tuple(ref)) if isinstance(ref, list) and all(integer(i) for i in ref) else None
            if other is not None:
                _, earlier, first = parts[other]
            if (other is None or other >= n or first + earlier['letters'] > start
                    or earlier['letters'] != part['letters']
                    or (key == 'same_as' and earlier['name'] != part['name'])):
                add(fails, 'tree_malformed')
                continue
            if word is not None:
                mine, theirs = word[start:start + part['letters']], word[first:first + part['letters']]
                if key == 'inverse_of':
                    theirs = [[c, inverses[i]] for c, i in reversed(theirs)]
                if mine != theirs:
                    add(fails, 'tree_expansion')


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
    elif not fallback_kept(before, after, mask):
        add(fails, 'fallback_subset')


def fallback_kept(before, after, mask):
    """The fallback contract (Lemma U): every piece off after the word was off before and is
    outside the unit, so the unit lands and no lattice piece goes off."""
    return not np.any(after.off & ~(before.off & ~mask))


def entry(name, snap):
    """The entry test of a phase or height sub-phase (plan section 0), from a checker snapshot."""
    hist = {int(h): count for h, count in snap.heights().items()}
    top, n4, n6 = max(hist, default=0), hist.get(4, 0), hist.get(6, 0)
    return {'r22d75': top <= 6 and n4 + n6 > 0, 'r22d81': top <= 6 and n6 > 0,
            'h4': top <= 4 and n4 > 0, 'high': top <= 6 and n4 + n6 > 0,
            'tail': snap.count > 0, 'lattice': snap.count == 0}[name]


def check_plugin_phase(name, data, count, before, after, run_result, fails):
    """One height sub-phase or the high phase: its snapshots and its result against the entry test."""
    result = data.get('result')
    if not integer(data.get('steps')) or data['steps'] != count:
        add(fails, 'phase_steps')
    if result is None:
        # Not reached, or cut off by an exception that ended the run as error (after its steps, if any).
        if ((count or data.get('start') is not None) and run_result != 'error') or data.get('end') is not None:
            add(fails, 'phase_result')
        elif data.get('start') is not None:
            # Entered before the exception: its start is the state it was entered at, and
            # the runner plays a step only while the entry test holds there.
            before.check(data['start'], name + '_start', fails, heights=True)
            if count and not entry(name, before):
                add(fails, 'entry_test')
        elif count:
            add(fails, 'phase_result')
        return None
    if result not in PHASE_RESULTS:
        add(fails, 'phase_result')
        return result
    before.check(data.get('start'), name + '_start', fails, heights=True)
    after.check(data.get('end'), name + '_end', fails, heights=True)
    # The runner decides not_applicable, handoff and lattice from the state alone, so
    # each such claim is recomputed from the checker's own snapshots. Stuck is not.
    applicable = entry(name, before)
    if not count:
        consistent = (result == 'not_applicable') != applicable and result not in ('handoff', 'lattice')
    elif not after.count:
        consistent = applicable and result == 'lattice'
    elif not entry(name, after):
        consistent = applicable and result == 'handoff'
    else:
        consistent = applicable and result not in ('not_applicable', 'handoff', 'lattice')
    if not consistent:
        add(fails, 'entry_test')
    return result


def check_v3_phases(record, steps, positions, start, ends, fails):
    """The phases in the fixed order, never entered twice, against the checker's snapshots.

    ``ends[k]`` is the checker's snapshot after the last step at position k or before it.
    Returns whether the phases block has the format 3 shape."""
    phases, run_result = record['phases'], record.get('result')
    if (not isinstance(phases, dict) or set(phases) != set(PHASE_ORDER)
            or any(not isinstance(phases[p], dict) for p in PHASE_ORDER)
            or not isinstance(phases['height'].get('sub'), dict) or set(phases['height']['sub']) != set(SUBPHASES)
            or any(not isinstance(phases['height']['sub'][s], dict) for s in SUBPHASES)):
        add(fails, 'phase')
        return False
    nsteps, twists = [0] * len(POSITIONS), [0] * len(POSITIONS)
    for step, k in zip(steps, positions):
        nsteps[k] += 1
        twists[k] += step['letters'] if integer(step.get('letters')) else 0

    def starts(k):
        return start if k == 0 else ends[k - 1]

    bulk = phases['bulk']
    if bulk.get('result') not in ('lattice', 'stuck', 'budget', 'timeout') or (
            bulk['result'] == 'lattice' and ends[0].count):
        add(fails, 'phase_result')
    if bulk.get('result') in ('budget', 'timeout') and record['config'].get('component') is None:
        add(fails, 'component')
    results = [bulk.get('result')]
    for k, (phase, sub) in enumerate(POSITIONS[1:5], 1):
        data = phases['height']['sub'][sub] if sub else phases[phase]
        results.append(check_plugin_phase(sub or phase, data, nsteps[k], starts(k), ends[k], run_result, fails))
    height, high, tail, lattice = (phases[p] for p in ('height', 'high', 'tail', 'lattice'))
    for data, count, total in ((bulk, nsteps[0], twists[0]), (height, sum(nsteps[1:4]), sum(twists[1:4])),
                               (high, nsteps[4], twists[4])):
        if not integer(data.get('steps')) or data['steps'] != count:
            add(fails, 'phase_steps')
        if not integer(data.get('twists')) or data['twists'] != total:
            add(fails, 'phase_twists')
    # The height phase as a whole: a budget, timeout or error of a sub-phase ends it,
    # and a sub-phase that reaches the lattice gives the lattice.
    ran = [r for r in results[1:4] if r is not None]
    outer = height.get('result')
    if outer is None:
        if ran and run_result != 'error':
            add(fails, 'phase_result')
    elif (outer not in PHASE_RESULTS or not ran or (any(r in RUN_ENDS for r in ran) and outer not in RUN_ENDS)
            or ('lattice' in ran and not any(r in RUN_ENDS for r in ran) and outer != 'lattice')):
        add(fails, 'phase_result')
    else:
        starts(1).check(height.get('start'), 'height_start', fails, heights=True)
        ends[max(k for k in (1, 2, 3) if results[k] is not None)].check(
            height.get('end'), 'height_end', fails, heights=True)
    # Tail: entered while a piece is off; stuck, a timeout or a budget leaves one off.
    result = tail.get('result')
    results.append(result)
    if not integer(tail.get('steps')) or tail['steps'] != nsteps[5]:
        add(fails, 'phase_steps')
    if result is None:
        # An exception after a tail step ends the run as error and leaves the step standing;
        # the tail plays a step only while a piece is off.
        if nsteps[5] and run_result != 'error':
            add(fails, 'phase_result')
        elif nsteps[5] and not starts(5).count:
            add(fails, 'entry_test')
    elif result not in ('not_applicable', 'lattice', 'stuck', 'timeout', 'budget'):
        add(fails, 'phase_result')
    elif result == 'not_applicable':
        if starts(5).count or nsteps[5]:
            add(fails, 'entry_test')
    elif not starts(5).count or (result == 'lattice') != (ends[5].count == 0):
        add(fails, 'entry_test')
    if 'end' in tail:
        ends[5].check(tail['end'], 'tail_end', fails, heights=True)
    # The kernel's letter bound over the whole high phase, from the checker's own start
    # snapshot, whenever the phase played a step (an error run included; P4-falsepass-02).
    if nsteps[4]:
        high_rows = [step for step, k in zip(steps, positions) if k == 4]
        try:
            named = check_contracts.check_high_phase({'off': starts(4).count, 'heights': starts(4).heights()},
                                                     high_rows)
            if not isinstance(named, list) or any(not isinstance(n, str) or not n for n in named):
                raise TypeError('failures are a list of names')
        except Exception:  # a crashed hook cannot pass a phase
            named = ['contract']
        for name in named:
            add(fails, name)
    # Lattice: entered only with every piece on the lattice. A played phase is solved (K1);
    # with no played step its result is solved (nothing scheduled) or an End (revision 3e, E2).
    played = nsteps[6] or record.get('lattice') is not None
    lat = lattice.get('result')
    results.append(lat if played or lat is not None else None)
    if not integer(lattice.get('steps')) or lattice['steps'] != nsteps[6]:
        add(fails, 'phase_steps')
    if (played or lat is not None) and starts(6).count:
        add(fails, 'entry_test')
    if ((nsteps[6] and lat != 'solved') or (lat is not None and lat not in ('solved',) + LATTICE_ENDS)
            or (lat in LATTICE_ENDS and (nsteps[6] or record.get('lattice') is not None and lat != 'error'))):
        add(fails, 'phase_result')
    # Fixed order: a position runs only after the one before it ran and did not end the run,
    # and is left out only after a result that ends the run (plan section 0, phase results):
    # an error, a budget or timeout after the bulk, or a stuck tail. An exception can end the
    # run as error before any result, and with every piece on the lattice a solved state or
    # --no-lattice ends the run solved or budget without a lattice phase result.
    ended = run_result == 'error'
    for k in range(1, len(POSITIONS)):
        previous = results[k - 1]
        ended = ended or (k > 1 and previous in RUN_ENDS) or (k == len(POSITIONS) - 1 and previous == 'stuck')
        if results[k] is None and not nsteps[k]:
            if not ended and not (k == len(POSITIONS) - 1 and run_result in ('solved', 'budget')):
                add(fails, 'phase_result')
            continue
        if (previous is None or (k > 1 and previous in RUN_ENDS)
                or (k == len(POSITIONS) - 1 and previous not in ('lattice', 'not_applicable'))):
            add(fails, 'phase_order')
    # Run results (plan section 0, phase results): a budget or timeout after the bulk ends the
    # run budget; a stuck tail ends it stuck; an error in any phase ends it error.
    # A lattice End ends the run the same way: stuck, budget (or timeout) or error.
    lat = results[6]
    if 'error' in results[1:5] or lat == 'error':
        if run_result != 'error':
            add(fails, 'result')
    elif run_result != 'error':
        if ((any(r in ('budget', 'timeout') for r in results[1:5]) or result in ('budget', 'timeout')
                or lat in ('budget', 'timeout')) and run_result != 'budget'):
            add(fails, 'result')
        if (result == 'stuck' or lat == 'stuck') != (run_result == 'stuck'):
            add(fails, 'result')
        if lat == 'solved' and run_result != 'solved':
            add(fails, 'result')
    component = record['config'].get('component')
    if ((record.get('lattice') is not None and component is None)
            or (component == 'lattice_composition' and record.get('lattice') is None)):
        add(fails, 'component')
    return True


def check_acceptance(record, steps, final, ends, fails):
    """Recount the acceptance block and recompute its verdict in the plan's order (section 0)."""
    acceptance, config, phases = record.get('acceptance'), record['config'], record['phases']
    stuck = record.get('stuck') if isinstance(record.get('stuck'), dict) else {}
    units = stuck.get('units', [])
    if not isinstance(units, list) or any(not isinstance(u, dict) or not valid_attempts(u.get('attempts'))
                                          for u in units):
        add(fails, 'attempts')
        units = []
    # Attempts at the final state: the stuck units', and those of every height sub-phase
    # or high phase that ended there. Each step's attempts were made before it.
    final_attempts = [a for unit in units for a in unit['attempts']]
    attempts = [a for s in steps if s.get('phase') != 'bulk' for a in s.get('attempts', [])] + final_attempts
    plugin = [phases['height']['sub'][sub] for sub in SUBPHASES] + [phases['high']]
    for k, data in enumerate(plugin, 1):
        if 'attempts' not in data:
            continue
        if not valid_attempts(data['attempts']):
            add(fails, 'attempts')
            continue
        attempts += data['attempts']
        if data.get('result') is not None and ends[k].digest == final.digest:
            final_attempts += data['attempts']
    lattice = phases.get('lattice') if isinstance(phases.get('lattice'), dict) else {}
    if 'attempts' in lattice:
        if not valid_attempts(lattice['attempts']):
            add(fails, 'attempts')
        else:
            attempts += lattice['attempts']
            final_attempts += lattice['attempts']
    probe = isinstance(stuck.get('probe'), dict)
    counts = {'certificate_steps': sum(s.get('standing') == 'certificate' for s in steps),
              'fallback_steps': sum(s.get('standing') == 'fallback' for s in steps),
              'timed_out_attempts': sum(a['timed_out'] for a in attempts)}
    result = record.get('result')
    if result == 'error':
        verdict = 'error'
    elif config.get('component') is not None:
        verdict = 'component'
    elif counts['certificate_steps']:
        verdict = 'stuck'
    elif counts['fallback_steps'] or probe:
        verdict = 'coverage_failure'
    elif result == 'budget' or (result == 'stuck' and any(a['timed_out'] for a in final_attempts)):
        verdict = 'budget'                      # a stop with a timed-out attempt is not stuck (R22D-55)
    else:
        verdict = result
    if (not isinstance(acceptance, dict) or set(acceptance) != ACCEPTANCE_KEYS
            or any(not integer(acceptance[key]) or acceptance[key] != value for key, value in counts.items())
            or (acceptance['probe'] is not None and type(acceptance['probe']) is not bool)
            or (acceptance['probe'] is True) != probe
            or (acceptance['probe'] is not None and config.get('mode') != 'acceptance')):
        add(fails, 'acceptance')
    if not isinstance(acceptance, dict) or acceptance.get('verdict') != verdict:
        add(fails, 'verdict')
    return verdict


def copy_state(st):
    other = sim.State(restore.ctx, menu=st.menu)
    other._poses, other._lookup, other.pose_id = list(st._poses), dict(st._lookup), st.pose_id.copy()
    return other


def check_probe(base, probe, end, st, fails):
    """Replay a probe word on a copy of the end state under the fallback contract."""
    pf = []
    doc = load_word(base, probe, pf)
    try:
        unit = read_json(inside(base, probe.get('unit')))['pieces']
        if (not isinstance(unit, list) or not unit or any(not integer(p) or p >= len(end.off) for p in unit)
                or len(set(unit)) != len(unit)):
            raise ValueError('invalid unit')
    except (ValueError, OSError, EOFError, zlib.error, KeyError, TypeError, RuntimeError):
        unit = None
    # A step the rules missed: a non-empty word for a unit that is off at the end state.
    if (doc is None or unit is None or pf or not doc['word'] or not end.off[unit].all()
            or doc['input_digest'] != end.digest
            or probe.get('input_digest') != end.digest or probe.get('output_digest') != doc['output_digest']
            or not integer(probe.get('letters')) or probe['letters'] != len(doc['word'])):
        add(fails, 'probe')
        return
    copy = copy_state(st)
    for cap, item in doc['word']:
        try:
            outcome = copy.apply(sim.Twist(restore.ctx, cap, restore.element(st.menu, cap, item),
                                           'menu', {'menu': st.menu.name}), journal=False)
            admitted = outcome.applied and not outcome.uncertain
        except Exception:  # a simulator error cannot certify a letter
            admitted = False
        if not admitted:
            add(fails, 'probe')
            return
    after = Snapshot(copy)
    mask = np.zeros(len(end.off), bool)
    mask[unit] = True
    # Lemma U: the unit lands and no lattice piece goes off; with the unit off before, the
    # off set strictly shrinks.
    if after.digest != doc['output_digest'] or not fallback_kept(end, after, mask):
        add(fails, 'probe')


def labels_identity(st):
    """Restored means identity sticker labels (owner decision, 15 October 2026), from the simulator."""
    if st.off_lattice_count():
        return False
    stickers = st.lattice_stickers()
    labels = np.asarray(stickers['labels'])
    return bool(np.all(stickers['frame_agrees'])) and bool(np.array_equal(labels, np.arange(len(labels))))


def check_lattice_report(base, data, twists, restorer, start, end, fails):
    """The plugin's report the runner kept: the file the runner writes, lattice/report.json, unchanged, and
    agreeing with the replay (I1R-01): the restorer the configuration names, the digests the checker replayed at
    the phase's start and end, a solved player with no failure, and the replayed length, all of it scheduled."""
    report = data.get('report')
    try:
        raw = inside(base, report['file']).read_bytes()
        doc = json.loads(raw)
        player = doc['player']
        played = player['played']
        ok = (set(report) == {'file', 'sha256'} and report['file'] == 'lattice/report.json'
              and hashlib.sha256(raw).hexdigest() == report['sha256']
              and doc['restorer'] == restorer and player['input']['digest'] == start
              and player['final']['digest'] == end and player['final']['labels_identity'] is True
              and player['result'] == 'solved' and player['failure'] is None
              and integer(played['retained']) and integer(played['corrections'])
              and played['retained'] + played['corrections'] == played['total'] == twists
              and player['scheduled'] == {'retained': played['retained'], 'corrections': played['corrections']})
    except (OSError, ValueError, KeyError, TypeError):
        ok = False
    if not ok:
        add(fails, 'lattice_report')


def check_step_table(base, step, current, menu, fails):
    """A row's table is the runner's checkpoint after that step, tables/<index>.json, and it loads to the
    state the checker replayed (I1R-02)."""
    try:
        ok = (step['table'] == 'tables/%05d.json' % step['index']
              and load_table(inside(base, step['table']), menu).digest() == current.digest)
    except (OSError, ValueError, EOFError, zlib.error, KeyError, TypeError, IndexError):
        ok = False
    if not ok:
        add(fails, 'table')


def run_verifier(argv, cwd, report, seconds):
    """Keep both streams and terminate the verifier's process group on timeout."""
    timed_out = False
    with report.with_suffix('.stdout.log').open('wb') as stdout, \
            report.with_suffix('.stderr.log').open('wb') as stderr:
        proc = subprocess.Popen(argv, cwd=cwd, stdout=stdout, stderr=stderr, start_new_session=True)
        try:
            try:
                proc.wait(timeout=seconds)
            except subprocess.TimeoutExpired:
                timed_out = True
                try:
                    os.killpg(proc.pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass
                try:
                    proc.wait(timeout=1)
                except subprocess.TimeoutExpired:
                    pass
        finally:
            # Also kill descendants if the parent exits before they do.
            try:
                os.killpg(proc.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            proc.wait()
    return proc.returncode, timed_out


def lattice_check(base, lattice, current, menu, bulk_table, chain, report_parent, fails, require_solved,
                  v2=False, verify_seconds=7200):
    """Bind the independent report to this replay, not just to its PASS flag."""
    if not isinstance(lattice, dict):
        add(fails, 'lattice')
        return False, None
    if current.count:
        add(fails, 'lattice_off')
    if lattice.get('input_digest') != current.digest:
        add(fails, 'lattice_input_digest')
    if type(lattice.get('chain')) is not bool or lattice['chain'] != (False if v2 else bool(chain)):
        add(fails, 'lattice_chain')
    if v2:
        chain = []
        spec = lattice.get('input')
        try:
            if not isinstance(spec, str) or not spec.startswith('table:'):
                raise ValueError('invalid input spec')
            table = inside(base, spec[6:])
        except (ValueError, OSError, RuntimeError):
            add(fails, 'path')
            return False, None
        try:
            if not table.exists():
                add(fails, 'lattice_input_missing')
            elif load_table(table, menu).digest() != current.digest:
                add(fails, 'lattice_input_table')
        except (OSError, ValueError, EOFError, zlib.error, KeyError, TypeError, IndexError):
            add(fails, 'lattice_input_table')
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
        if v2 and run.get('input') != spec:
            add(fails, 'lattice_summary_run')
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
        if not v2:
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
    try:
        cwd = base if v2 else REPO
        paths = [os.path.relpath(p, base) if v2 else str(p) for p in (lib, out, report)]
        argv = [sys.executable, '-I', '-B', str(VERIFY_LATE), str(REPO), str(J)] + paths
        # The independent verifier reads plain JSON chain words. Materialize
        # gzipped words in the fresh report directory, never over the inputs.
        if chain:
            plain_chain = []
            for index, path in enumerate(chain):
                if path.read_bytes().startswith(b'\x1f\x8b'):
                    plain = report_dir / ('chain-%05d.json' % index)
                    plain.write_text(json.dumps(read_json(path)))
                    path = plain
                plain_chain.append(str(path))
            # argv[6] inside verify_late.py is --chain, followed by a table and words.
            argv += ['--chain', str(bulk_table)] + plain_chain
        returncode, timed_out = run_verifier(argv, cwd, report, verify_seconds)
        if timed_out:
            add(fails, 'lattice_verify_timeout')
            return False, twists
        lines = report.with_suffix('.stdout.log').read_text(errors='replace').splitlines()
        if returncode != 0 or not lines or not re.fullmatch(r'PASS\s+\S+', lines[-1]):
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


def check_run(run_dir, emit, report_parent=None, verify_seconds=7200, allow_unchecked=False):
    """Emit step rows and a final row, and return the final row (also used by tests).

    With ``allow_unchecked`` a runner-run/3 record may pass with contracts that
    check_contracts.py or check_lattice.py leave unchecked; they are still counted."""
    base = Path(run_dir).resolve()
    fails, step_fails, words = [], [], []
    counts = {phase: 0 for phase in PHASES}
    lattice_checked, lattice_twists, end_digest = False, None, None
    record, steps = {}, []
    emitted = 0
    v3, extra = False, None

    def step_row(step, sf):
        row = {'index': step.get('index'), 'phase': step.get('phase'), 'rule': step.get('rule')}
        if not v3:
            return {**row, 'pass': not sf, 'fails': list(sf)}
        shown = [name for name in sf if name != UNCHECKED or not allow_unchecked]
        return {**row, 'subphase': step.get('subphase'), 'pass': not shown, 'fails': shown,
                'unchecked': UNCHECKED in sf}

    def merge(sf):
        for name in sf:
            if name != UNCHECKED or not allow_unchecked:
                add(fails, name)
        if UNCHECKED in sf:
            extra['unchecked_contracts'] += 1

    try:
        record = read_json(base / 'run.json')
        if not isinstance(record, dict):
            record = {}
            raise ValueError('record must be an object')
        v2 = record.get('format') == 'runner-run/2'
        v3 = record.get('format') == 'runner-run/3'
        if record.get('format') not in FORMATS:
            add(fails, 'format')
        if v2:
            check_v2_config(record, fails)
        config = {}
        if v3:
            extra = {'verdict': None, 'unchecked_contracts': 0}
            counts = {phase: 0 for phase in V3_PHASES}
            config = check_v3_config(record, fails)
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
        menu = restore.menu('I_a')
        inverses = inverse_items(menu) if v3 else None
        for step in steps:
            sf = []
            if v3 and step.get('phase') == 'lattice':
                # A lattice word can be long: check_lattice.py reads it during the replay.
                words.append(None)
                try:
                    inside(base, step.get('word'))
                except (ValueError, OSError, RuntimeError):
                    add(sf, 'path')
            else:
                words.append(load_word_v3(base, step, sf) if v3 else load_word(base, step, sf))
            if v2 or v3:
                check_v2_step(step, words[-1], menu, sf)
            if v3:
                check_v3_step(step, config, sf)
                check_tree(base, step, words[-1]['word'] if words[-1] is not None else None, inverses, sf)
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
                emit(step_row(step, sf or fails or ['record']))
            for sf in step_fails:
                if v3:
                    merge(sf)
                else:
                    for name in sf:
                        add(fails, name)
            return finish(record, steps, counts, end_digest, False, lattice_twists, fails, emit, extra)
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
        # Format 3: each step's position in the fixed order, and ends[k], the checker's
        # snapshot after the last step at position k or, without one, where k was passed.
        first, ends, positions, at = current, [None] * len(POSITIONS), [], 0
        lattice_start = None                        # the replayed digest where the lattice phase starts
        for index, (step, word) in enumerate(zip(steps, words)):
            sf = step_fails[index]
            phase = step.get('phase')
            if not isinstance(step.get('rule'), str) or not step['rule']:
                add(sf, 'rule')
            if not integer(step.get('index')) or step['index'] != index:
                add(sf, 'index')
            if phase not in (V3_PHASES if v3 else PHASES):
                add(sf, 'phase')
            if phase == 'bulk' and index >= bulk_count:
                add(sf, 'phase_order')
            if v3:
                k = POSITIONS.index(('tail' if phase == 'fallback' else phase,
                                     step.get('subphase') if phase == 'height' else None))
                if k < at:
                    add(sf, 'phase_order')          # a phase or sub-phase entered again
                while at < k:
                    ends[at] = current
                    at += 1
                positions.append(k)
            before = current
            if phase == 'bulk':
                before.n()
            heights = phase != 'bulk' if v3 else phase in ('tail', 'fallback')
            before.check(step.get('before'), 'before', sf, heights=heights, n=phase == 'bulk')
            if word is not None:
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
            if word is None:
                # A lattice step (format 3): check_lattice.py plays its ia16 word on st.
                try:
                    named = check_lattice.replay_lattice_step(st, step, inside(base, step['word']), menu)
                    if not isinstance(named, list) or any(not isinstance(n, str) or not n for n in named):
                        raise TypeError('failures are a list of names')
                except Exception:  # a crashed hook cannot pass a step
                    named = ['lattice_step']
                for name in named:
                    add(sf, name)
            else:
                # Height and kernel contracts (plan section 2, hooks M6): prepare sees the
                # state before the replay, on_letter the state after each letter.
                hooked, ctx = v3 and phase in ('height', 'high'), None
                if hooked:
                    try:
                        ctx = check_contracts.prepare(step, st)
                    except Exception:
                        add(sf, 'contract')
                        hooked = False
                admitted = True
                for letter, (cap, item) in enumerate(word['word']):
                    try:
                        outcome = st.apply(sim.Twist(restore.ctx, cap, restore.element(menu, cap, item),
                                                     'menu', {'menu': menu.name}), journal=False)
                        admitted = outcome.applied and not outcome.uncertain
                    except Exception:  # a simulator error cannot certify a letter
                        admitted = False
                    if not admitted:
                        add(sf, 'admissible')
                        break
                    if hooked:
                        try:
                            check_contracts.on_letter(ctx, letter, st)
                        except Exception:
                            add(sf, 'contract')
                            hooked = False
                if hooked and admitted:
                    try:
                        named = check_contracts.finish(ctx, step, st)
                        if not isinstance(named, list) or any(not isinstance(n, str) or not n for n in named):
                            raise TypeError('failures are a list of names')
                    except Exception:  # a crashed hook cannot pass a step
                        named = ['contract']
                    for name in named:
                        add(sf, name)
            before.st = None
            current = Snapshot(st)
            current.check(step.get('after'), 'after', sf, heights=heights, n=phase == 'bulk')
            if v3 and 'table' in step:
                check_step_table(base, step, current, menu, sf)
            if phase == 'lattice' and lattice_start is None:
                lattice_start = before.digest
            if word is not None and word['output_digest'] != current.digest:
                add(sf, 'output_digest')
            if phase in counts:
                if word is not None:
                    counts[phase] += len(word['word'])
                elif integer(step.get('letters')):
                    counts[phase] += step['letters']
            if phase == 'bulk':
                recorded_before, recorded_after = step.get('before', {}).get('N'), step.get('after', {}).get('N')
                if (not integer(recorded_before) or not integer(recorded_after)
                        or recorded_after >= recorded_before or current.n() >= before.n()):
                    add(sf, 'N_decrease')
            elif phase in ('tail', 'fallback'):
                check_unit(base, step, before, current, sf)
                chain.append(inside(base, step['word']))
            emit(step_row(step, sf))
            emitted += 1
            if v3:
                merge(sf)
            else:
                for name in sf:
                    add(fails, name)
            if index + 1 == bulk_count:
                check_bulk_end()
        shaped = False
        if v3:
            while at < len(POSITIONS):
                ends[at] = current
                at += 1
            shaped = check_v3_phases(record, steps, positions, first, ends, fails)
        else:
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
        stuck = record.get('stuck')
        if record.get('result') == 'stuck' or stuck is not None:
            current.check(stuck, 'stuck', fails, heights=True)
            if v3 and isinstance(stuck, dict):
                if (stuck.get('phase') not in V3_PHASES or not isinstance(stuck.get('reason'), str)
                        or not stuck['reason'] or not isinstance(stuck.get('units'), list)):
                    add(fails, 'stuck')
                # A probe is a fallback word found but not played: acceptance mode only.
                if 'probe' in stuck:
                    if not isinstance(stuck['probe'], dict) or config.get('mode') != 'acceptance':
                        add(fails, 'probe')
                    else:
                        check_probe(base, stuck['probe'], current, st, fails)
        lattice = record.get('lattice')
        played = v3 and any(s.get('phase') == 'lattice' for s in steps)
        restorer = config['lattice'].get('restorer') if isinstance(config.get('lattice'), dict) else None
        if played:
            # Plan section 2, end digest (R3-05): a played lattice is solved by identity
            # labels; the replayed digest may differ from SOLVED by rotations they do not see.
            lattice_twists = counts['lattice']
            if lattice is not None:
                add(fails, 'lattice')
            lattice_rows = [s for s in steps if s.get('phase') == 'lattice']
            last = lattice_rows[-1]
            if (type(last.get('labels_identity')) is not bool
                    or (record.get('result') == 'solved' and (last['labels_identity'] is not True or current.count))):
                add(fails, 'labels_identity')
            # Only the phase's end table exists (runner.py, played_lattice); the step loop compared it with
            # the replay (I1R-02).
            if 'table' not in last or any('table' in s for s in lattice_rows[:-1]):
                add(fails, 'table')
            end_digest = current.digest
            if shaped:
                check_lattice_report(base, record['phases']['lattice'], lattice_twists, restorer, lattice_start,
                                     current.digest, fails)
        elif v3 and shaped and lattice is None and record['phases']['lattice'].get('result') == 'solved':
            # The restorer scheduled nothing (revision 3e, E2): the checker tests the labels itself.
            lattice_twists = 0
            if current.count or not labels_identity(st):
                add(fails, 'labels_identity')
            check_lattice_report(base, record['phases']['lattice'], 0, restorer, current.digest, current.digest,
                                 fails)
            end_digest = current.digest
        else:
            if lattice is not None:
                lattice_checked, lattice_twists = lattice_check(
                    base, lattice, current, menu, bulk_table, chain,
                    Path(report_parent).resolve() if report_parent is not None else base, fails,
                    require_solved=record.get('result') == 'solved', v2=v2 or v3, verify_seconds=verify_seconds)
            elif record.get('result') == 'solved' and current.digest != SOLVED:
                add(fails, 'solved_digest')
            end_digest = SOLVED if lattice_checked else current.digest
            if v3 and lattice is not None:
                end_digest = None                   # lattice by composition until I1
        if v3 and shaped:
            data = record['phases']['lattice']
            if data.get('twists') != lattice_twists or (data['twists'] is not None and not integer(data['twists'])):
                add(fails, 'phase_twists')
            # The composition path records the run's result; an error can cut it short. Without it, a
            # phase with no played step has no result, solved (nothing scheduled) or an End, which
            # check_v3_phases holds against the run's result.
            if not played and (
                    (lattice is not None and data.get('result') != record.get('result')
                     and not (data.get('result') is None and record.get('result') == 'error'))
                    or (lattice is None and data.get('result') not in (None, 'solved') + LATTICE_ENDS)):
                add(fails, 'phase_result')
        totals = record['totals']
        for phase in counts:
            name = phase + '_twists'
            if phase != 'lattice' and (not integer(totals.get(name)) or totals[name] != counts[phase]):
                add(fails, name)
        if (totals.get('lattice_twists') != lattice_twists or 'lattice_twists' not in totals
                or (totals['lattice_twists'] is not None and not integer(totals['lattice_twists']))):
            add(fails, 'lattice_twists')
        if v3 and shaped:
            extra['verdict'] = check_acceptance(record, steps, current, ends, fails)
    except Exception:  # malformed input must still get a failing JSON final row
        add(fails, 'record')
        for step in steps[emitted:]:
            emit(step_row(step, ['record']))
    return finish(record, steps, counts, end_digest, lattice_checked, lattice_twists, fails, emit, extra)


def finish(record, steps, counts, end_digest, lattice_checked, lattice_twists, fails, emit, extra=None):
    final = {'pass': not fails, 'result': record.get('result'), 'steps': len(steps),
             **{p + '_twists': n for p, n in counts.items() if p != 'lattice'}, 'lattice_twists': lattice_twists,
             'end_digest': end_digest, 'lattice_checked': lattice_checked, 'fails': list(fails)}
    if extra is not None:
        final.update(extra)                     # format 3: the verdict and the unchecked contracts
    emit(final)
    return final


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('run_dir', type=Path)
    parser.add_argument('--out', type=Path)
    parser.add_argument('--verify-seconds', type=float, default=7200,
                        help='lattice verifier time limit in seconds (default: 7200)')
    parser.add_argument('--allow-unchecked-contracts', action='store_true',
                        help='runner-run/3: pass with contracts that check_contracts.py or check_lattice.py '
                             'do not check yet; they are still counted')
    args = parser.parse_args()
    if not positive(args.verify_seconds):
        parser.error('verify-seconds must be finite and positive')
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

        final = check_run(args.run_dir, emit, args.out.parent if args.out is not None else None,
                          verify_seconds=args.verify_seconds, allow_unchecked=args.allow_unchecked_contracts)
    finally:
        if output is not None:
            output.close()
    return 0 if final['pass'] else 1


if __name__ == '__main__':
    sys.exit(main())
