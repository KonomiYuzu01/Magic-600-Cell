"""Read-only acceptance, except disposable build/. No report/digest refresh."""
import sys

sys.dont_write_bytecode = True

from hashlib import sha256
import json
from pathlib import Path
import shutil

HERE = Path(__file__).resolve().parent
BUILD = HERE / 'build'


def remove_build():
    if BUILD.exists():
        # Never recursively remove a resolved location outside this experiment.
        if BUILD.is_symlink() or BUILD.resolve().parent != HERE:
            raise RuntimeError('build path escapes the experiment')
        shutil.rmtree(BUILD)


def snapshot():
    return {p.relative_to(HERE).as_posix(): sha256(p.read_bytes()).hexdigest()
            for p in HERE.rglob('*') if p.is_file() and 'build' not in p.relative_to(HERE).parts}


def main():
    if sys.argv[1:] not in ([], ['--python-only']):
        raise ValueError('only optional argument: --python-only (partial verification)')
    python_only = bool(sys.argv[1:])
    if sys.flags.optimize:
        raise RuntimeError('acceptance requires assertions enabled')
    before = snapshot()
    try:
        remove_build()
        BUILD.mkdir()
        manifest = json.loads((HERE / 'MANIFEST.json').read_bytes())
        candidate = json.loads((HERE / 'CANDIDATE.json').read_bytes())
        for name, digest in {**manifest, **candidate}.items():
            assert Path(name).name == name, 'manifest path must be local'
            assert sha256((HERE / name).read_bytes()).hexdigest() == digest, ('frozen digest differs', name)
        from fixtures import produce
        outputs = produce()
        if not python_only:
            from grid_probe import run
            outputs.update(run(BUILD))
        omitted = {'default_grid.json', 'default_bisectors.json'} if python_only else set()
        assert set(outputs) == set(manifest) - {'CANDIDATE.json', 'INPUTS.json', 'MENU.json'} - omitted
        for name, data in outputs.items():
            (BUILD / name).write_bytes(data)
            assert data == (HERE / name).read_bytes(), ('re-derived table differs', name)
    finally:
        remove_build()
        if snapshot() != before:
            raise RuntimeError('acceptance changed a retained file')
    if python_only:
        print('PASS Python fixtures only; default-grid Rust enumeration NOT RUN; build/ removed.')
    else:
        print('PASS: all certificates and fixture tables independently reproduced; build/ removed.')


if __name__ == '__main__':
    main()
