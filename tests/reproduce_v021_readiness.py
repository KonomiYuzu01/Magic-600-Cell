"""Reproduce the old PID-equality bug using a real forwarding subprocess.
This emulates the Windows venv extra process on POSIX; it is not a Windows run.
"""
from pathlib import Path
import importlib.util, json, os, signal, subprocess, sys, tempfile, time
from urllib.request import build_opener, ProxyHandler, Request
import argparse
ap=argparse.ArgumentParser();ap.add_argument('old_package',type=Path);args=ap.parse_args()
if os.name=='nt':raise SystemExit('This historical reproduction uses POSIX process-group cleanup. Use test_engine_lifecycle.py on Windows.')
OLD=args.old_package.resolve()
spec=importlib.util.spec_from_file_location('old_bootstrap',OLD/'native/bootstrap.py')
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
report={'scope':'Real Python forwarding-process fixture on Linux; emulates Windows venv PID indirection','checks':[]}
with tempfile.TemporaryDirectory(prefix='c600-repro-') as td:
    p=Path(td);lp=p/'launch.json'
    wrapper='import subprocess,sys; p=subprocess.Popen(sys.argv[1:]); sys.exit(p.wait())'
    with (p/'engine.log').open('wb') as log:
        proc=subprocess.Popen([sys.executable,'-c',wrapper,sys.executable,str(OLD/'server.py'),'--no-browser','--port','0','--data',str(p/'session'),'--launch-info',str(lp)],cwd=OLD,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
        try:
            deadline=time.monotonic()+20
            while not lp.exists():
                if time.monotonic()>deadline or proc.poll() is not None: raise RuntimeError((p/'engine.log').read_text())
                time.sleep(.05)
            info=json.loads(lp.read_text())
            op=build_opener(ProxyHandler({}))
            with op.open(Request(info['base']+'/api/status',headers={'X-C600-Token':info['token']}),timeout=3) as r:
                status=json.load(r)
            assert info['pid']!=proc.pid
            try: m.read_launch(lp,proc,timeout=.4); raise AssertionError('Old code unexpectedly accepted forwarded server')
            except RuntimeError as e: assert str(e)=='State engine readiness timeout'
            report['checks'].append({'name':'Healthy forwarded full-state server rejected by old readiness check','passed':True,'launcher_pid':proc.pid,'engine_pid':info['pid'],'server_status_received':True,'old_result':'State engine readiness timeout','labelled_state_hash':status['state_hash']})
        finally:
            os.killpg(proc.pid,signal.SIGTERM);proc.wait(timeout=5)
try:
    (OLD/'native/NativeHost.cs').read_bytes().decode('cp936')
    raise AssertionError('CP936 unexpectedly decoded source')
except UnicodeDecodeError as e:
    report['checks'].append({'name':'Default CP936 source decode fails on native host UTF-8 bytes','passed':True,'byte_offset':e.start,'exception':str(e)})
report['passed']=True
out=Path(__file__).parent/'v022/old_fault_reproduction.json';out.parent.mkdir(parents=True,exist_ok=True);out.write_text(json.dumps(report,indent=2),encoding='utf-8')
print(json.dumps(report,indent=2))
