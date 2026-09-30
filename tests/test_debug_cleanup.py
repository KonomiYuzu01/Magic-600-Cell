"""Fault-injection tests for bounded cleanup; no files are deleted by mocks."""
from pathlib import Path
import importlib.util,json
from unittest.mock import patch
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('debug_runner',ROOT/'tests/run_debug.py');r=importlib.util.module_from_spec(spec);spec.loader.exec_module(r)
checks=[]
with patch.object(r.shutil,'rmtree',side_effect=[PermissionError('SQLite SHM still open'),None]),patch.object(r.time,'sleep'):
 out=r.remove_scratch(Path('fixture-only'),attempts=3);assert out['removed'] and out['attempts']==2
 checks.append('Transient cleanup denial retries after engine shutdown')
with patch.object(r.shutil,'rmtree',side_effect=PermissionError('SQLite SHM still open')),patch.object(r.time,'sleep'):
 out=r.remove_scratch(Path('fixture-only'),attempts=3);assert not out['removed'] and out['attempts']==3 and 'retained_directory' in out
 checks.append('Persistent cleanup denial is recorded without throwing over the original failure')
with patch.object(r.shutil,'rmtree',side_effect=FileNotFoundError):
 assert r.remove_scratch(Path('fixture-only'))['removed'];checks.append('Already-removed scratch directory is harmless')
(ROOT/'tests/v022').mkdir(parents=True,exist_ok=True)
(ROOT/'tests/v022/debug_cleanup.json').write_text(json.dumps(dict(passed=True,scope='Injected filesystem exceptions; real process/SQLite cleanup is tested separately',checks=checks),indent=2),encoding='utf-8')
for c in checks:print('PASS',c)
