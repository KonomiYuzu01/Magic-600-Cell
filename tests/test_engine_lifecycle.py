"""Real full-engine readiness, redirector, and cleanup regressions.
The forwarding fixture actually starts two processes on every platform. On
Windows the normal path additionally exercises the installed venv/runtime.
No user attempt is opened. Temporary sessions are deleted only after shutdown.
"""
from pathlib import Path
import contextlib,io,json,os,platform,runpy,secrets,shutil,socket,subprocess,sys,tempfile,threading,time
from urllib.error import HTTPError
from unittest.mock import patch
ROOT=Path(__file__).resolve().parents[1];sys.path.insert(0,str(ROOT))
from engine_process import EngineProcess,local_request,python_child_command,read_launch,redact,stop_owned_process,validate_launch
from session_lock import SessionLock
from server import atomic_json
OUT=ROOT/'tests/v022';OUT.mkdir(parents=True,exist_ok=True);checks=[]
def ok(name,**details):
 checks.append(dict(name=name,passed=True,**details));print('PASS',name,flush=True)
def unlocked(data):
 with contextlib.closing(SessionLock(data)):pass
 renamed=data.with_name(data.name+'-closed-check');data.rename(renamed);renamed.rename(data)
def make_engine(tmp,name,**kwargs):
 p=tmp/name;return EngineProcess(ROOT,p/'session',p/'launch.json',p/'engine.log',timeout=15,**kwargs)
@contextlib.contextmanager
def windows_reader(path):
 # A real Windows reader that permits reads/writes but does not share deletion.
 # This deterministically reproduces the sharing window behind WinError 5.
 import ctypes
 from ctypes import wintypes
 kernel=ctypes.WinDLL('kernel32',use_last_error=True)
 kernel.CreateFileW.argtypes=[wintypes.LPCWSTR,wintypes.DWORD,wintypes.DWORD,wintypes.LPVOID,wintypes.DWORD,wintypes.DWORD,wintypes.HANDLE];kernel.CreateFileW.restype=wintypes.HANDLE
 kernel.CloseHandle.argtypes=[wintypes.HANDLE];kernel.CloseHandle.restype=wintypes.BOOL
 handle=kernel.CreateFileW(str(path),0x80000000,3,None,3,0,None)
 if handle==ctypes.c_void_p(-1).value:raise ctypes.WinError(ctypes.get_last_error())
 try:yield
 finally:kernel.CloseHandle(handle)
try:
 with tempfile.TemporaryDirectory(prefix='c600-022-engine-') as td:
  tmp=Path(td);forward=tmp/'forward.py'
  if os.name=='nt':
   status=tmp/'reader.status.json';status.write_text('{"old":true}',encoding='utf-8');errors=[]
   with windows_reader(status):
    started=time.monotonic()
    try:atomic_json(status,{'new':True},timeout=.075);raise AssertionError('Locked status replacement unexpectedly succeeded')
    except PermissionError:assert .05<=time.monotonic()-started<1
    assert json.loads(status.read_text(encoding='utf-8'))=={'old':True}
    def replace_after_reader():
     try:atomic_json(status,{'new':True},timeout=2)
     except BaseException as e:errors.append(e)
    writer=threading.Thread(target=replace_after_reader);writer.start();time.sleep(.075)
    assert writer.is_alive() and json.loads(status.read_text(encoding='utf-8'))=={'old':True}
   writer.join(timeout=3);assert not writer.is_alive() and not errors and json.loads(status.read_text(encoding='utf-8'))=={'new':True}
   ok('Real Windows reader locks preserve complete prior JSON; atomic publication retries after release and has a bounded deadline')
  # CPython consumes __PYVENV_LAUNCHER__ when the forwarding interpreter
  # starts. A Python forwarding fixture must restore its own venv identity
  # before launching the base interpreter supplied by python_child_command.
  forward.write_text('''import os,subprocess,sys
env=os.environ.copy()
if os.name=='nt' and sys.prefix!=sys.base_prefix:
 env['__PYVENV_LAUNCHER__']=sys.executable
p=subprocess.Popen(sys.argv[1:],env=env)
sys.exit(p.wait())
''',encoding='utf-8')
  normal=make_engine(tmp,'ordinary')
  with normal:
   health=local_request(normal.info,'/api/health');assert health['ready'] and health['python_prefix']==sys.prefix
   expected=local_request(normal.info,'/api/status')['state_hash']
   ok('Ordinary full engine starts; authenticated health and virtual-environment prefix match',launcher_pid=normal.process.pid,engine_pid=normal.info['pid'],prefix_matches=True)
   try:local_request(normal.info,'/api/shutdown',{'launch_id':'x'*64});raise AssertionError('Wrong launch identity stopped engine')
   except HTTPError as e:assert e.code==403
   assert local_request(normal.info,'/api/health')['ready'];ok('Shutdown rejects a wrong launch ID without changing the state')
   with patch.dict(os.environ,{'http_proxy':'http://127.0.0.1:1','HTTP_PROXY':'http://127.0.0.1:1','no_proxy':'','NO_PROXY':''}):assert local_request(normal.info,'/api/health')['ready']
   ok('Readiness uses direct numeric loopback, ignoring inherited HTTP proxies')
   normal.launch_file.write_text('{',encoding='utf-8')
   def publish():time.sleep(.15);normal.launch_file.write_text(json.dumps(normal.info),encoding='utf-8')
   t=threading.Thread(target=publish);t.start();recovered=read_launch(normal.launch_file,normal.process,timeout=2,launch_id=normal.launch_id);t.join();assert recovered['pid']==normal.info['pid']
   ok('Incomplete metadata is retried and a completed valid launch becomes ready')
   try:read_launch(normal.launch_file,normal.process,.25,launch_id=secrets.token_hex(32));raise AssertionError('Stale launch accepted')
   except RuntimeError as e:assert 'Launch ID differs' in str(e)
   ok('A stale launch ID is rejected with the actual last-check reason')
   with patch('engine_process.local_request',return_value={'ready':True,'protocol':'wrong'}):
    try:read_launch(normal.launch_file,normal.process,.2,launch_id=normal.launch_id);raise AssertionError('Invalid authenticated health accepted')
    except RuntimeError as e:assert 'health reply' in str(e)
   ok('Health identity mismatch is rejected (injected malformed response)')
   for bad in ['http://localhost:1','http://127.0.0.1.attacker.invalid:1','http://127.0.0.1:1/path','http://user@127.0.0.1:1','https://127.0.0.1:1']:
    try:validate_launch(dict(normal.info,base=bad),normal.launch_id);raise AssertionError('Non-numeric/plain loopback URL accepted')
    except ValueError:pass
   ok('Untrusted/non-loopback metadata is rejected before credentials are sent')
   duplicate=EngineProcess(ROOT,normal.data,tmp/'duplicate/launch.json',tmp/'duplicate/engine.log',timeout=15)
   try:duplicate.start();raise AssertionError('Duplicate session engine accepted')
   except RuntimeError as e:assert 'already open in another' in str(e)
   finally:duplicate.close()
   assert duplicate.process.poll() is not None and local_request(normal.info,'/api/status')['state_hash']==expected
   ok('Existing-session conflict reports the real error; original session remains unchanged')
  assert normal.cleanup_result['graceful_request'] and not normal.cleanup_result['forced']
  assert normal.process.returncode==0 and not normal.launch_file.exists();unlocked(normal.data)
  ok('Graceful engine shutdown exits zero and releases SQLite/session handles before cleanup')
  stalled=make_engine(tmp,'stalled-clients')
  with stalled:
   address=stalled.info['base'].split('//')[1];host,port=address.split(':');port=int(port)
   idle=socket.create_connection((host,port))
   headers=socket.create_connection((host,port));headers.sendall(b'POST /api/status HTTP/1.0\r\nContent-Length: 10\r\nX-')
   body=socket.create_connection((host,port))
   body.sendall(f'POST /api/prefs HTTP/1.0\r\nHost: {address}\r\nX-C600-Token: {stalled.info["token"]}\r\nContent-Type: application/json\r\nContent-Length: 100\r\n\r\n{{"a'.encode())
   trickle_headers=socket.create_connection((host,port));trickle_headers.sendall(b'POST /api/status HTTP/1.0\r\nX-')
   trickle_body=socket.create_connection((host,port))
   trickle_body.sendall(f'POST /api/prefs HTTP/1.0\r\nHost: {address}\r\nX-C600-Token: {stalled.info["token"]}\r\nContent-Type: application/json\r\nContent-Length: 100000\r\n\r\n{{"a'.encode())
   dripping=threading.Event();dripping.set()
   def drip():
    while dripping.is_set():
     for client in (trickle_headers,trickle_body):
      with contextlib.suppress(OSError):client.sendall(b'a')
     time.sleep(.05)
   dripper=threading.Thread(target=drip,daemon=True);dripper.start()
   time.sleep(.5);started=time.monotonic()
  elapsed=time.monotonic()-started;dripping.clear();dripper.join(5)
  for client in (idle,headers,body,trickle_headers,trickle_body):client.close()
  assert stalled.cleanup_result['graceful_request'] and not stalled.cleanup_result['forced'] and stalled.process.returncode==0,stalled.cleanup_result
  assert elapsed<5,elapsed;unlocked(stalled.data)
  ok('Idle, stalled and trickling connections cannot hold graceful shutdown past the owner grace',seconds=round(elapsed,2))
  forwarded=make_engine(tmp,'forwarded',forwarder=forward)
  with forwarded:
   health=local_request(forwarded.info,'/api/health')
   assert forwarded.process.pid!=forwarded.info['pid'] and health['ready'] and health['python_prefix']==sys.prefix
   ok('Healthy engine behind a real forwarding process is accepted despite different PIDs',launcher_pid=forwarded.process.pid,engine_pid=forwarded.info['pid'])
  assert forwarded.process.returncode==0 and not forwarded.cleanup_result['forced'];unlocked(forwarded.data);shutil.rmtree(forwarded.data)
  ok('Forwarded engine and launcher both exit; scratch SQLite files can be removed immediately')
  early=make_engine(tmp,'argument-failure',extra_args=['--definitely-invalid-option'])
  try:early.start();raise AssertionError('Invalid server arguments accepted')
  except RuntimeError as e:assert 'unrecognized arguments' in str(e) and 'code 2' in str(e)
  assert early.process.poll() is not None;ok('Early server exit includes the actual error and exit code; no orphan remains')
  with socket.socket() as listener:
   listener.bind(('127.0.0.1',0));listener.listen()
   collision=make_engine(tmp,'bind-failure',extra_args=['--port',str(listener.getsockname()[1])])
   try:collision.start();raise AssertionError('Occupied-port startup accepted')
   except RuntimeError as e:
    text=str(e);assert 'server_bind' in text and 'code 1' in text and 'Fatal Python error' not in text and '_enter_buffered_busy' not in text,text
   finally:collision.close()
   assert collision.process.returncode==1 and not collision.cleanup_result['forced'];unlocked(collision.data)
   assert 'parent watcher did not stop' not in collision.log_file.read_text(encoding='utf-8')
   ok('Real bind failure preserves its original exception and exits normally with the parent stdin pipe still open')
  slow=make_engine(tmp,'deadline');slow.timeout=.001
  try:slow.start();raise AssertionError('Artificial tiny deadline unexpectedly passed')
  except RuntimeError as e:assert 'readiness timed out' in str(e)
  assert slow.process.poll() is not None
  if slow.data.exists():unlocked(slow.data)
  ok('Readiness timeout shuts down the owned engine; original failure remains available')
  pipe=make_engine(tmp,'eof')
  with pipe:pipe.process.stdin.close();pipe.process.wait(timeout=10);assert pipe.process.returncode==0
  unlocked(pipe.data);ok('Parent-control pipe EOF stops the full engine without force killing')
  abandoned=tmp/'abandoned';abandoned.mkdir();owner=tmp/'owner.py'
  owner.write_text('''import os,sys
from pathlib import Path
sys.path.insert(0,sys.argv[1])
from engine_process import EngineProcess
p=Path(sys.argv[2]);e=EngineProcess(Path(sys.argv[1]),p/'session',p/'launch.json',p/'engine.log',timeout=15)
e.start();(p/'ready').write_text('ready',encoding='utf-8');os._exit(77)
''',encoding='utf-8')
  cmd,env=python_child_command([str(owner),str(ROOT),str(abandoned)]);r=subprocess.run(cmd,env=env,timeout=30)
  assert r.returncode==77 and (abandoned/'ready').exists();deadline=time.monotonic()+10;stage=None
  while time.monotonic()<deadline:
   try:
    stage=json.loads((abandoned/'launch.status.json').read_text(encoding='utf-8'))['stage']
    if stage=='stopped':break
   except (OSError,ValueError):pass
   time.sleep(.1)
  assert stage=='stopped';unlocked(abandoned/'session');ok('Hard launcher exit triggers parent-pipe recovery and closes the engine session')
  unicode_engine=make_engine(tmp,'C600 测试 Ω')
  with unicode_engine:assert local_request(unicode_engine.info,'/api/health')['ready']
  unlocked(unicode_engine.data);ok('Unicode session/launch paths start and close successfully')
  if os.name=='nt':
   status_locked=make_engine(tmp,'held-progress-file');status_locked.timeout=30;status_locked.launch_file.parent.mkdir(parents=True,exist_ok=True)
   held=status_locked.launch_file.with_suffix('.status.json');held.write_text('{"reader_holds_old_status":true}',encoding='utf-8')
   with windows_reader(held):
    with status_locked:assert local_request(status_locked.info,'/api/health')['ready']
    assert json.loads(held.read_text(encoding='utf-8'))=={'reader_holds_old_status':True}
   assert status_locked.process.returncode==0 and not status_locked.cleanup_result['forced'];unlocked(status_locked.data)
   log=status_locked.log_file.read_text(encoding='utf-8');assert 'progress file update unavailable' in log and 'Fatal Python error' not in log
   ok('A persistently reader-locked diagnostic status file cannot abort authenticated engine startup or graceful shutdown')
  fake=tmp/'base-python.exe';fake.write_bytes(b'fixture-only')
  cmd,env=python_child_command(['server.py'],executable=str(tmp/'venv/Scripts/python.exe'),base_executable=str(fake),windows=True)
  assert cmd[0]==str(fake) and Path(env['__PYVENV_LAUNCHER__'])==tmp/'venv/Scripts/python.exe' and env['PYTHONUTF8']=='1' and '-u' in cmd
  ok('Windows venv bypass command preserves the virtual-environment launcher (construction test)')
  original=Path.read_text
  def cp936_default(self,encoding=None,errors=None,**kw):return original(self,encoding=encoding or 'cp936',errors=errors,**kw)
  with patch.object(Path,'read_text',cp936_default),contextlib.redirect_stdout(io.StringIO()):runpy.run_path(str(ROOT/'tests/test_native_startup_contract.py'),run_name='__main__')
  ok('Source contract passes with a simulated CP936 default text encoding')
  cmd,env=python_child_command(['-c','import time;time.sleep(90)'])
  hung=subprocess.Popen(cmd,env=env,start_new_session=os.name!='nt',creationflags=subprocess.CREATE_NEW_PROCESS_GROUP if os.name=='nt' else 0)
  result=stop_owned_process(hung,grace=.1,group_owned=os.name!='nt');assert result['forced'] and hung.poll() is not None
  ok('Unresponsive owned-process fallback terminates and waits; no name-wide process kill')
  secret='a'*43;text=redact(f'http://127.0.0.1:6006/?token={secret} "token": "{secret}", "launch_id":"{secret}"');assert secret not in text
  ok('Lifecycle diagnostics redact API tokens and launch IDs')
 ok('All temporary test directories removed after child shutdown');passed=True
except BaseException:
 passed=False;raise
finally:
 report=dict(passed=passed,scope='Real full-state servers and forwarding processes; Windows-specific behavior executed only if platform says Windows',platform=platform.platform(),python=sys.version,windows_executed=os.name=='nt',checks=checks)
 (OUT/'engine_lifecycle.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
