"""Build in a disposable probe directory and run the CPU-only acceptance check."""
from pathlib import Path
import os
import shutil
import subprocess
import sys


def run(command, repository, timeout, *, shell=False, env=None):
    process = subprocess.Popen(command, shell=shell, cwd=repository, env=env)
    try:
        return process.wait(timeout=timeout)
    except subprocess.TimeoutExpired:
        # Killing only cmd.exe leaves CMake/Ninja holding the temporary directory.
        try:
            subprocess.run(['taskkill', '/PID', str(process.pid), '/T', '/F'],
                           check=False, stdout=subprocess.DEVNULL,
                           stderr=subprocess.DEVNULL, timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
        process.wait(timeout=5)
        raise


def ninja_commands(build, ninja):
    result = subprocess.run([ninja, '-C', str(build), '-t', 'commands', 'sb_probe'],
                            capture_output=True, text=True, check=True, timeout=10)
    commands = result.stdout.splitlines()
    if not commands:
        raise RuntimeError('Ninja emitted no build commands')
    for index, command in enumerate(commands, 1):
        print(f'sandbox build command {index}/{len(commands)}', flush=True)
        # Includes tracing serves Ninja's incremental dependency database, which
        # is unused in this fresh one-shot build; retain all compile/link flags.
        command = command.replace(' /showIncludes ', ' ')
        status = run(command, build, 60, shell=True)
        if status:
            return status
    return 0


def main():
    source = Path(__file__).resolve().parent
    repository = source.parents[3]
    # Application Control can refuse unsigned executables in system temp. Keep
    # this packet's build under its owned source directory, with fresh test data.
    build = source / f"build-check-{os.getpid()}"
    # Plain mkdir retains the packet directory's inherited sandbox permissions.
    build.mkdir()
    try:
        environment = os.environ.copy()
        environment['M600_SB_SERIAL_NINJA'] = '1'
        print('acceptance: serial Ninja command execution (Windows sandbox)', flush=True)
        result = run(
            f'call "{source / "build.cmd"}" "{build}"',
            repository, 300, shell=True, env=environment,
        )
        print(f"acceptance build: exit {result}", flush=True)
        if result:
            print("acceptance selftest: not run (build failed)", flush=True)
            return result
        result = run(
            [str(build / "sb_probe.exe"), "--selftest"],
            repository, 60,
        )
        print(f"acceptance selftest: exit {result}", flush=True)
        return result
    except (OSError, subprocess.TimeoutExpired) as error:
        print(f"acceptance failed: {error}", file=sys.stderr)
        return 1
    finally:
        # This exact directory was created above; no repository path is removed.
        if build.resolve().parent == source:
            shutil.rmtree(build)


if __name__ == "__main__":
    if len(sys.argv) == 4 and sys.argv[1] == '--ninja-commands':
        sys.exit(ninja_commands(Path(sys.argv[2]), sys.argv[3]))
    sys.exit(main())
