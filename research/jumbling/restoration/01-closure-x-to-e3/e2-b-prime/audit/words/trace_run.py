"""Run a constructor script under a CPython audit hook and record every file-system and process event it raises.

Usage: python3 -I -B trace_run.py <trace json> <script> [script args...]

Recorded: 'open' (path, mode, flags; this also covers source and bytecode files read by the import system),
directory listings, file-system mutations, temporary files, mmap, process creation and network events. At the end
it lists every loaded module that has a file, with its path. Nothing is written except <trace json> and what the
traced script itself writes.
"""
import os
import sys

sys.dont_write_bytecode = True

WATCH = {
    'open', 'os.listdir', 'os.scandir', 'os.mkdir', 'os.remove', 'os.rename', 'os.replace', 'os.rmdir',
    'os.symlink', 'os.link', 'os.truncate', 'os.chmod', 'os.chown', 'os.utime', 'os.chdir', 'os.chflags',
    'shutil.copyfile', 'shutil.copymode', 'shutil.copystat', 'shutil.copytree', 'shutil.rmtree', 'shutil.move',
    'shutil.make_archive', 'shutil.unpack_archive', 'subprocess.Popen', 'os.system', 'os.exec', 'os.posix_spawn',
    'os.spawn', 'os.fork', 'os.forkpty', 'os.kill', 'socket.connect', 'socket.bind', 'socket.getaddrinfo',
    'socket.gethostbyname', 'urllib.Request', 'http.client.connect', 'tempfile.mkstemp', 'tempfile.mkdtemp',
    'glob.glob', 'glob.glob/2', 'mmap.__new__', 'ctypes.dlopen', 'sqlite3.connect', 'pathlib.Path.glob',
    'pathlib.Path.rglob', 'os.listxattr', 'os.getxattr', 'zipimport.zipimporter', 'import',
}
EVENTS = {}
ACTIVE = [True]


def hook(event, args):
    if not ACTIVE[0] or event not in WATCH:
        return
    if event == 'import':
        key = (event, str(args[0]), '')
    elif event == 'open':
        path, mode = args[0], args[1]
        if isinstance(path, bytes):
            path = path.decode('utf-8', 'replace')
        key = (event, str(path), str(mode))
    else:
        key = (event, repr(args)[:400], '')
    EVENTS[key] = EVENTS.get(key, 0) + 1


def main():
    out = sys.argv[1]
    script = sys.argv[2]
    sys.argv = [script] + sys.argv[3:]
    sys.addaudithook(hook)
    import runpy
    status = 'ok'
    try:
        runpy.run_path(script, run_name='__main__')
    except SystemExit as exc:
        status = f'SystemExit {exc.code}'
    except BaseException as exc:  # noqa: BLE001
        status = f'error {type(exc).__name__}: {str(exc)[:300]}'
    finally:
        ACTIVE[0] = False
        mods = {}
        for name, m in list(sys.modules.items()):
            f = getattr(m, '__file__', None)
            if f:
                mods[name] = os.path.realpath(f)
        rows = [{'event': e, 'target': t, 'mode': md, 'count': n,
                 'real': os.path.realpath(t) if e == 'open' and not t.isdigit() else None}
                for (e, t, md), n in sorted(EVENTS.items())]
        import json
        with open(out, 'w') as fh:
            json.dump({'script': os.path.realpath(script), 'argv': sys.argv, 'status': status, 'events': rows,
                       'modules': mods}, fh, indent=1)
        print(f'trace written: {status}', file=sys.stderr)
    if status != 'ok':
        sys.exit(1)


if __name__ == '__main__':
    main()
