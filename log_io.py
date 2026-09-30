"""Bounded declarative proof-log IO; validation never mutates a live Session."""
from __future__ import annotations
import base64, binascii, gzip, io, json, math, re
import numpy as np
from core import PuzzleState, canonical, state_hash

MAX_LOG_BYTES = 16 * 1024 * 1024
MAX_JSON_BYTES = 24 * 1024 * 1024
MAX_TRANSACTIONS = 10000
MAX_PRIMITIVES = 2000000
FORMAT = 'C600-STUDIO-SESSION-v1'
HASH = re.compile(r'[0-9a-f]{64}\Z')
# Fixed original 600-cell-Full PuzzleStructure.GetRad(), measured on Windows:
# native-picking-04/diagnostics/native-camera-radius.json. The original IL takes
# its value from full sticker geometry, independent of view/camera/filter state.
NATIVE_CAMERA_RADIUS = 4.94427195193458
NATIVE_MAX_ANGLE = float(np.float32(math.pi))

def validate_camera(camera):
    """Validate an explicit current-UI camera, never preferences from a log."""
    fields={'format','matrix','radius','angle','cell','face_shrink','sticker_shrink'}
    if not isinstance(camera,dict) or set(camera)!=fields or camera.get('format')!='C600-native-camera-v1':
        raise ValueError('Invalid native camera object')
    matrix=camera['matrix']
    if not isinstance(matrix,list) or len(matrix)!=16:raise ValueError('Native camera requires a 4x4 matrix')
    values=[*matrix,camera['radius'],camera['angle'],camera['face_shrink'],camera['sticker_shrink']]
    for value in values:
        try:finite=type(value) in (int,float) and math.isfinite(value)
        except OverflowError:finite=False
        if not finite:raise ValueError('Native camera numbers must be finite')
    if any(abs(value)>1.00001 for value in matrix):raise ValueError('Native camera matrix is not orthogonal')
    mat=np.asarray(matrix,dtype=float).reshape(4,4)
    if np.any(np.abs(mat@mat.T-np.eye(4))>1e-5):raise ValueError('Native camera matrix is not orthogonal')
    if not (-.91*NATIVE_CAMERA_RADIUS<=camera['radius']<=1.01*NATIVE_CAMERA_RADIUS and
            0<camera['angle']<=NATIVE_MAX_ANGLE and 0<=camera['face_shrink']<=1 and 0<=camera['sticker_shrink']<=1):
        raise ValueError('Native camera value is outside its range')
    if type(camera['cell'])!=int or not 0<=camera['cell']<600:raise ValueError('Native camera cell must be 0..599')
    return json.loads(canonical(camera))

def _object(pairs):
    result = {}
    for key, value in pairs:
        if key in result: raise ValueError('Duplicate JSON field: ' + key)
        result[key] = value
    return result

def decode_log(payload, format='c600'):
    if format != 'c600': raise ValueError('Unsupported log format; choose a C600 proof log')
    if not isinstance(payload, str) or len(payload) > 4 * ((MAX_LOG_BYTES + 2) // 3):
        raise ValueError('Log upload must be base64 data no larger than 16 MiB')
    try: raw = base64.b64decode(payload, validate=True)
    except (ValueError, binascii.Error) as exc: raise ValueError('Invalid log base64 data') from exc
    if not raw or len(raw) > MAX_LOG_BYTES: raise ValueError('Log file must contain 1 byte..16 MiB')
    if raw[:2] == b'\x1f\x8b':
        try:
            with gzip.GzipFile(fileobj=io.BytesIO(raw), mode='rb') as stream: raw = stream.read(MAX_JSON_BYTES + 1)
        except (OSError, EOFError) as exc: raise ValueError('Corrupt gzip log') from exc
    if len(raw) > MAX_JSON_BYTES: raise ValueError('Expanded log exceeds 24 MiB')
    try:
        return json.loads(raw.decode('utf-8-sig'), object_pairs_hook=_object,
                          parse_constant=lambda value: (_ for _ in ()).throw(ValueError('Non-finite JSON number')))
    except (UnicodeError, json.JSONDecodeError, RecursionError) as exc: raise ValueError('Log must contain valid UTF-8 JSON') from exc

TRANSACTION_CAP = 'Interactive import is capped at 10,000 transactions'
PRIMITIVE_CAP = 'Interactive import exceeds 2,000,000 primitive moves'

def check_import_budget(record):
    """Refuse to write a C600 record that validate_record would reject for its size."""
    events = record.get('events')
    if not isinstance(events, list) or len(events) > MAX_TRANSACTIONS: raise ValueError(TRANSACTION_CAP)
    total = 0
    for ev in events:
        total += _count(ev.get('primitive_count') if isinstance(ev, dict) else None, 'primitive count')
        if total > MAX_PRIMITIVES: raise ValueError(PRIMITIVE_CAP)

def _exportable(record):
    try: check_import_budget(record)
    except ValueError as exc:
        raise ValueError(str(exc) + '; this history cannot be written as an importable C600 log. Make a session backup instead.') from exc

def encode_export(record):
    """Session export download: the same import budget as proof logs."""
    _exportable(record)
    return gzip.compress(canonical(record).encode(), compresslevel=5)

def encode_log(record):
    _exportable(record)
    raw = canonical(record).encode('utf-8')
    if len(raw) > MAX_JSON_BYTES: raise ValueError('Proof log exceeds the 24 MiB interactive size limit')
    packed = gzip.compress(raw, compresslevel=5, mtime=0)
    if len(packed) > MAX_LOG_BYTES: raise ValueError('Compressed proof log exceeds 16 MiB')
    return packed

def _hash(value, name):
    if not isinstance(value, str) or HASH.fullmatch(value) is None: raise ValueError('Invalid ' + name + ' checksum')

def _count(value, name):
    if type(value) == int:
        if value < 0: raise ValueError('Invalid ' + name)
        return value
    if isinstance(value, str) and re.fullmatch(r'0|[1-9][0-9]{0,15}', value): return int(value)
    raise ValueError('Invalid ' + name)

def validate_record(model, record):
    """Return a detached, fully replayed import plan before any journal writes.

    Recipes are finite legal words/stars of the immutable current model. Replay
    checks every complete 259800-label state; file preferences are never applied.
    """
    if not isinstance(record, dict) or record.get('format') != FORMAT or record.get('model_id') != model.model_id:
        raise ValueError('Session format/model mismatch')
    if record.get('root') != 'labelled_identity': raise ValueError('Proof log root must be labelled_identity')
    if record.get('native_windows_equivalence') is not False: raise ValueError('Unsupported native equivalence claim')
    if not isinstance(record.get('prefs', {}), dict): raise ValueError('Log preferences must be an object (they are not imported)')
    try: size = len(json.dumps(record, ensure_ascii=False, allow_nan=False, separators=(',', ':')).encode('utf-8'))
    except (ValueError, TypeError, RecursionError) as exc: raise ValueError('Log contains invalid JSON values') from exc
    if size > MAX_JSON_BYTES: raise ValueError('Expanded log exceeds 24 MiB')
    events = record.get('events')
    if not isinstance(events, list) or len(events) > MAX_TRANSACTIONS:
        raise ValueError(TRANSACTION_CAP)
    _hash(record.get('final_state'), 'final-state')
    # Validate the whole structure and expansion budget before expensive replay.
    prepared = []; total = 0
    for index, ev in enumerate(events):
        model.check_cancel()
        if not isinstance(ev, dict): raise ValueError('Log transaction must be an object')
        _hash(ev.get('pre'), 'pre-state'); _hash(ev.get('post'), 'post-state')
        try: recipe, length = model.normalize(ev.get('recipe'))
        except (KeyError, IndexError, TypeError) as exc: raise ValueError('Invalid legal recipe in log') from exc
        total += length
        if total > MAX_PRIMITIVES: raise ValueError(PRIMITIVE_CAP)
        stars = sum(x['kind'] == 'star' for x in recipe)
        if _count(ev.get('primitive_count'), 'primitive count') != length or _count(ev.get('stars'), 'star count') != stars:
            raise ValueError('Log transaction count differs from its legal witness')
        note = ev.get('note', 'Imported'); assistance = ev.get('assistance', 'unknown')
        if not isinstance(note, str) or len(note) > 500 or not isinstance(assistance, str) or len(assistance) > 80:
            raise ValueError('Log note/assistance text is invalid or too long')
        prepared.append(dict(recipe=recipe, length=length, stars=stars, pre=ev['pre'], post=ev['post'], note=note, assistance=assistance))
    labels = model.ids.copy()
    for index, ev in enumerate(prepared):
        model.check_cancel()
        if state_hash(labels) != ev['pre']: raise ValueError('Import pre-state hash mismatch at transaction ' + str(index))
        src, dst, _, _ = model.net(ev['recipe']); labels[dst] = labels[src]
        if state_hash(labels) != ev['post']: raise ValueError('Import replay mismatch at transaction ' + str(index))
    if state_hash(labels) != record['final_state']: raise ValueError('Import final-state mismatch')
    state = PuzzleState(model, labels)
    model.check_cancel()
    return dict(events=prepared, state=state, primitive_count=total, transactions=len(prepared))
