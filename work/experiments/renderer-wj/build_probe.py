"""Execute Ninja's build commands serially under the restricted Windows token."""
import subprocess
import sys
from pathlib import Path


def main():
    build, ninja = Path(sys.argv[1]), sys.argv[2]
    result = subprocess.run([ninja, '-C', str(build), '-t', 'commands', 'wj_probe'],
                            capture_output=True, text=True, check=True, timeout=30)
    commands = result.stdout.splitlines()
    if not commands:
        raise RuntimeError('Ninja emitted no build commands')
    for command in commands:
        status = subprocess.run(command.replace(' /showIncludes ', ' '), cwd=build,
                                shell=True, timeout=120).returncode
        if status:
            return status
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
