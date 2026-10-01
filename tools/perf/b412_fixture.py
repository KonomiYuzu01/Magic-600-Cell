"""Build and copy isolated, scripted session fixtures for the B4-12 harness."""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import sqlite3
import sys

ROOT = Path(__file__).resolve().parents[2]
WORD = [1, 601]
FORMAT = 'magic600-b412-fixture-v1'
ENGINE_SOURCES = ('core.py', 'session.py', 'session_lock.py', 'assets/manifest.json')
FIELDS = {
    'format', 'profile', 'word', 'checkpoints', 'checkpoint_names', 'state_hash',
    'head', 'journal_depth', 'database_sha256', 'database_bytes', 'builder_sha256',
    'engine_sources', 'created_utc',
}


class FixtureRefusal(ValueError):
    """Invalid input; messages contain only short, public reasons."""


class FixtureFailure(RuntimeError):
    """A fixture could not satisfy its scripted expectation."""


def parse_profile(profile: str) -> int:
    if profile == 'basic':
        return 0
    match = re.fullmatch(r'history-([1-9][0-9]{0,3})', profile) if isinstance(profile, str) else None
    if match and 1 <= int(match[1]) <= 2000:
        return int(match[1])
    raise FixtureRefusal('invalid-profile')


# Default data roots: C600 Studio profiles and the 0.4 release (%LOCALAPPDATA%/Magic600Cell/0.4).
PROTECTED_ROOTS = ('C600Studio', 'Magic600Cell')


def guard_path(path: Path) -> Path:
    path = Path(path).resolve()
    local = Path(os.getenv('LOCALAPPDATA', str(Path.home())))
    candidate = Path(os.path.normcase(str(path)))
    for name in PROTECTED_ROOTS:
        protected = Path(os.path.normcase(str((local / name).resolve())))
        if candidate.is_relative_to(protected):
            raise FixtureRefusal('default-data-directory')
    return path


def _reject_absolute_paths(value):
    if isinstance(value, str) and re.match(r'^(?:[A-Za-z]:[\\/]|/|\\\\)', value):
        raise FixtureRefusal('absolute-path-value')
    if isinstance(value, dict):
        for key, item in value.items():
            _reject_absolute_paths(key)
            _reject_absolute_paths(item)
    elif isinstance(value, list):
        for item in value:
            _reject_absolute_paths(item)


def _validate_hash(value):
    if not isinstance(value, str) or not re.fullmatch(r'[0-9a-fA-F]{64}', value):
        raise FixtureRefusal('invalid-sha256')


def validate_fixture(record: dict) -> None:
    if not isinstance(record, dict) or set(record) != FIELDS:
        raise FixtureRefusal('invalid-fixture-fields')
    _reject_absolute_paths(record)
    if record['format'] != FORMAT:
        raise FixtureRefusal('invalid-fixture-format')
    count = parse_profile(record['profile'])
    if type(record['checkpoints']) is not int or record['checkpoints'] != count:
        raise FixtureRefusal('invalid-checkpoints')
    word = record['word']
    if not isinstance(word, list) or any(type(move) is not int for move in word) or word != WORD:
        raise FixtureRefusal('invalid-word')
    names = record['checkpoint_names']
    if not isinstance(names, list) or len(names) != count or any(not isinstance(name, str) for name in names):
        raise FixtureRefusal('invalid-checkpoint-names')
    for key in ('head', 'journal_depth', 'database_bytes'):
        if type(record[key]) is not int or record[key] < (1 if key == 'database_bytes' else 0):
            raise FixtureRefusal('invalid-' + key.replace('_', '-'))
    for key in ('state_hash', 'database_sha256', 'builder_sha256'):
        _validate_hash(record[key])
    sources = record['engine_sources']
    if not isinstance(sources, dict) or set(sources) != set(ENGINE_SOURCES):
        raise FixtureRefusal('invalid-engine-sources')
    for value in sources.values():
        _validate_hash(value)
    created = record['created_utc']
    if not isinstance(created, str) or not re.fullmatch(r'[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(?:\.[0-9]+)?Z', created):
        raise FixtureRefusal('invalid-created-utc')
    try:
        datetime.fromisoformat(created)
    except ValueError:
        raise FixtureRefusal('invalid-created-utc') from None


def _sha256(path: Path) -> str:
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def _load_fixture(fixture: Path) -> dict:
    path = guard_path(fixture / 'fixture.json')
    try:
        record = json.loads(path.read_text(encoding='utf-8'))
    except (OSError, UnicodeError, ValueError):
        raise FixtureRefusal('invalid-fixture-json') from None
    validate_fixture(record)
    return record


def _template_database(fixture: Path) -> Path:
    template = guard_path(fixture / 'template')
    database = guard_path(template / 'session.sqlite3')
    if not template.is_dir() or sorted(item.name for item in template.iterdir()) != ['session.sqlite3'] or not database.is_file():
        raise FixtureRefusal('invalid-template-files')
    return database


def build_fixture(profile: str, output: Path) -> dict:
    count = parse_profile(profile)
    output = guard_path(output)
    if output.exists():
        raise FixtureRefusal('output-exists')
    output.mkdir(parents=True)
    template = output / 'template'
    template.mkdir()

    sys.path.insert(0, str(ROOT))
    from core import Model, state_hash
    from session import Session

    model = Model()
    recipe = [{'kind': 'word', 'moves': WORD}]
    source, destination, _, _ = model.net(recipe)
    labels = model.ids.copy()
    labels[destination] = labels[source]
    expected = state_hash(labels)
    names = ['B412 history %04d' % i for i in range(1, count + 1)]
    session = Session(model, template)
    try:
        preview = session.preview(recipe, note='B4-12 fixture')
        session.commit(preview['token'])
        for name in names:
            session.checkpoint(name)
        status = session.status()
        depth = session.event()['depth']
    finally:
        session.close()
    if status['state_hash'] != expected or preview['post_state'] != expected:
        raise FixtureFailure('state-mismatch')

    database = template / 'session.sqlite3'
    connection = sqlite3.connect(database)
    try:
        connection.execute('PRAGMA wal_checkpoint(TRUNCATE)')
        connection.execute('PRAGMA journal_mode=DELETE')
    finally:
        connection.close()
    (template / 'engine.lock').unlink()
    if sorted(item.name for item in template.iterdir()) != ['session.sqlite3']:
        raise FixtureFailure('template-not-clean')
    record = {
        'format': FORMAT, 'profile': profile, 'word': WORD, 'checkpoints': count,
        'checkpoint_names': names, 'state_hash': expected, 'head': status['head'],
        'journal_depth': depth, 'database_sha256': _sha256(database),
        'database_bytes': database.stat().st_size,
        'builder_sha256': _sha256(ROOT / 'tools/perf/b412_fixture.py'),
        'engine_sources': {path: _sha256(ROOT / path) for path in ENGINE_SOURCES},
        'created_utc': datetime.now(timezone.utc).isoformat().replace('+00:00', 'Z'),
    }
    validate_fixture(record)
    (output / 'fixture.json').write_text(json.dumps(record, indent=2, sort_keys=True), encoding='utf-8')
    return record


def copy_fixture(fixture: Path, data: Path) -> dict:
    fixture = guard_path(fixture)
    data = guard_path(data)
    record = _load_fixture(fixture)
    if data.exists():
        raise FixtureRefusal('data-exists')
    database = _template_database(fixture)
    if _sha256(database) != record['database_sha256']:
        raise FixtureRefusal('database-sha256-mismatch')
    data.mkdir(parents=True)
    copied = data / 'session.sqlite3'
    shutil.copyfile(database, copied)
    if _sha256(copied) != record['database_sha256']:
        raise FixtureRefusal('database-sha256-mismatch')
    return {
        'format': 'magic600-b412-fixture-copy-v1', 'profile': record['profile'],
        'database_sha256': record['database_sha256'], 'state_hash': record['state_hash'],
    }


def verify_data(data: Path, fixture: Path) -> dict:
    data = guard_path(data)
    fixture = guard_path(fixture)
    record = _load_fixture(fixture)
    if data == (fixture / 'template').resolve():
        raise FixtureRefusal('template-is-not-data')
    if not data.is_dir() or not guard_path(data / 'session.sqlite3').is_file():
        raise FixtureRefusal('missing-data-database')

    sys.path.insert(0, str(ROOT))
    from core import Model
    from session import Session

    session = Session(Model(), data)
    try:
        status = session.status()
        names = [row[0] for row in session.db.execute(
            "SELECT name FROM snapshots WHERE name LIKE 'B412 history %' ORDER BY name")]
        actual = {'state_hash': status['state_hash'], 'head': status['head'], 'checkpoint_names': names}
        mismatches = [key for key, value in actual.items() if value != record[key]]
    finally:
        session.close()
    return {'format': 'magic600-b412-fixture-verify-v1', 'ok': not mismatches, 'mismatches': mismatches}


class _Parser(argparse.ArgumentParser):
    def error(self, message):
        raise FixtureRefusal('invalid-arguments')


def main(argv=None) -> int:
    parser = _Parser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    build = commands.add_parser('build')
    build.add_argument('--profile', required=True)
    build.add_argument('--output', type=Path, required=True)
    copy = commands.add_parser('copy')
    copy.add_argument('--fixture', type=Path, required=True)
    copy.add_argument('--data', type=Path, required=True)
    verify = commands.add_parser('verify')
    verify.add_argument('--data', type=Path, required=True)
    verify.add_argument('--fixture', type=Path, required=True)
    try:
        args = parser.parse_args(argv)
        if args.command == 'build':
            record = build_fixture(args.profile, args.output)
        elif args.command == 'copy':
            record = copy_fixture(args.fixture, args.data)
        else:
            record = verify_data(args.data, args.fixture)
    except FixtureRefusal as error:
        print('b412_fixture: ' + str(error), file=sys.stderr)
        return 2
    except FixtureFailure as error:
        print('b412_fixture: ' + str(error), file=sys.stderr)
        return 1
    except ImportError:
        print('b412_fixture: engine-unavailable', file=sys.stderr)
        return 1
    except Exception:
        print('b412_fixture: operation-failed', file=sys.stderr)
        return 1
    print(json.dumps(record, sort_keys=True))
    return 1 if args.command == 'verify' and not record['ok'] else 0


if __name__ == '__main__':
    raise SystemExit(main())
