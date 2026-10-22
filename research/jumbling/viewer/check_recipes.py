"""Export every listed state recipe without a browser, using temporary data only."""
import argparse
from concurrent.futures import ProcessPoolExecutor
import importlib.util
import json
import multiprocessing
import os
from pathlib import Path
import sys
import tempfile

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('tutorial_figures', HERE / 'make_figures.py')
batch = importlib.util.module_from_spec(spec)
spec.loader.exec_module(batch)


def check_group(figures):
    exporter = batch.recipe_exporter()
    session, checked, lines, failed = exporter.ReplaySession(), {}, [], False
    for entry in figures:
        try:
            key = batch.export_key(entry)
            if key not in checked:
                with tempfile.TemporaryDirectory(prefix='magic600-recipe-') as directory:
                    header, tutorial = exporter.export_recipe(entry['state'], Path(directory) / 'data', session)
                    checked[key] = header['digest'], tutorial['moved']['count']
            digest, count = checked[key]
            lines.append(f'{entry["id"]}: {digest} emphasis={count}')
        except (OSError, ValueError, KeyError, TypeError, AssertionError, EOFError) as exc:
            lines.append(f'{entry["id"]}: FAIL ({exc})')
            failed = True
    return lines, failed


def recipe_groups(figures):
    """Keep shared replay prefixes together; independent families need no shared state."""
    groups = {}
    for entry in figures:
        if 'state' not in entry:
            continue
        recipe = entry['state']
        start = recipe['start']
        if 'record' in start:
            key = ('record', start['record'])
        elif 'table' in start:
            key = ('table', start['table'])
        else:
            first = recipe.get('play', [])[:1]
            key = (json.dumps(start, sort_keys=True), first[0]['word'] if first else None)
        groups.setdefault(key, []).append(entry)
    return list(groups.values())


def worker_environment():
    for name in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
        os.environ[name] = '1'


def report(results):
    failed = False
    for lines, bad in results:
        for line in lines:
            print(line, flush=True)
        failed |= bad
    return int(failed)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--list', type=Path, default=HERE.parent / 'tutorial/figures/figures.json')
    args = parser.parse_args(argv)
    try:
        groups = recipe_groups(batch.load_list(args.list))
        if len(groups) <= 1:
            return report(map(check_group, groups))
        else:
            # Separate kernels: mutable simulator caches never cross threads.
            with ProcessPoolExecutor(max_workers=min(4, os.cpu_count() or 1),
                                     mp_context=multiprocessing.get_context('spawn'),
                                     initializer=worker_environment) as pool:
                return report(pool.map(check_group, groups))
    except (OSError, ValueError, KeyError, TypeError, AssertionError, EOFError) as exc:
        print(f'recipes: FAIL ({exc})', file=sys.stderr)
        return 1


if __name__ == '__main__':
    sys.exit(main())
