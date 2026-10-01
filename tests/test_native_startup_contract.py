"""Source/arithmetic regressions, not a substitute for real WinForms execution.
The production Windows launcher separately compiles/runs NativeHostRegression.cs.
"""
from pathlib import Path
import importlib.util, json, re, sys
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT))
host=(ROOT/'native/NativeHost.cs').read_text(encoding='utf-8-sig');layout=(ROOT/'native/NativeDockLayout.cs').read_text(encoding='utf-8-sig');boot=(ROOT/'native/bootstrap.py').read_text(encoding='utf-8-sig')
checks=[]
def ok(name):checks.append(dict(name=name,passed=True));print('PASS',name,flush=True)
# The shipped original assigned 600/305 minima to a default-width control before
# its 1400px Size assignment. No valid separator could satisfy that interval.
original_default_width=150
assert original_default_width-305<600
ok('Original panel minima form an impossible interval at default construction width (arithmetic reproduction only)')
assert not re.search(r'new\s+SplitContainer\b',host+layout)
assert '.Panel1MinSize=' not in host and '.Panel2MinSize=' not in host and '.SplitterDistance=' not in host
assert 'NativeDockLayout workspace' in host
ok('Production native construction has no SplitContainer/minimum-distance path')
constants={name:int(re.search(r'const int '+name+r' = (\d+)',layout)[1]) for name in ['MinimumViewportWidth','MinimumToolsWidth','DividerWidth']}
v,t,d=constants.values()
count=0
for width in range(0,8193):
 for preferred in [0,100,240,348,500,1600,100000]:
  for show in [False,True]:
   got=0 if not show or width<v+t+d else min(max(t,preferred),width-v-d)
   assert got==0 or (t<=got<=width-v-d and width-got-d>=v)
   count+=1
ok(f'Dock sizing arithmetic: {count} width/preference/visibility combinations')
assert 'if(busy||closing||(!connected&&!allowDisconnected))return;' in host
assert 'form.IsDisposed||!form.IsHandleCreated' in host
assert 'Application.RemoveMessageFilter(this)' in host
assert 'Interlocked.CompareExchange(ref heartbeatInFlight,1,0)' in host
assert 'Invalid native keybindings; using defaults and preserving file' in host
ok('Disconnected commands, callback disposal, heartbeat overlap and malformed key file are guarded in source')
assert 'api.Get("native/snapshot?protocol=2")' in host
assert 'api.Bytes("native/colors")' not in host and 'api.Bytes("native/styles")' not in host
assert 'NativeHostRegression.cs' in boot and 'winforms-self-test.json' in boot
assert boot.index("if args.self_test_only:") < boot.index("exe = (args.runtime")
assert boot.index('result = subprocess.run([str(regression)') < boot.index('with EngineProcess(')
ok('Real WinForms startup regression is wired before runtime loading and engine launch')
# Engine identity/health, forwarding-PID, and cleanup behavior are exercised
# by test_engine_lifecycle.py against real processes, not matching-PID mock files.
assert 'from engine_process import EngineProcess, read_launch' in boot
assert 'with EngineProcess(' in boot
ok('Bootstrap uses the shared authenticated readiness and owned cleanup lifecycle')
report=dict(passed=True,scope='Python source-contract and arithmetic tests; C# was not compiled or executed by this script',checks=checks)
out=ROOT/'tests/v022/native_startup_contract.json';out.parent.mkdir(exist_ok=True)
out.write_text(json.dumps(report,indent=2),encoding='utf-8')
