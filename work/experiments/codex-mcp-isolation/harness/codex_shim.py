"""MAGIC600_CODEX_CMD shim: run the pinned Codex with the wrapper's exact arguments, adding only the two
base-URL overrides (right after `exec`) that point the ChatGPT backend and the model API at the local mock.
SHIM_EXTRA (space-separated) adds experiment flags after `exec`; the shim logs the final argv."""
import json, os, sys
codex, args = os.environ["SHIM_CODEX"], sys.argv[1:]
if args and args[0] == "exec":
    port = os.environ.get("SHIM_PORT", "8443")
    args = ["exec", "-c", f'chatgpt_base_url="https://127.0.0.1:{port}/backend-api/"',
            "-c", f'openai_base_url="https://127.0.0.1:{port}/backend-api/codex"',
            *os.environ.get("SHIM_EXTRA", "").split(), *args[1:]]
with open(os.environ["SHIM_LOG"], "a", encoding="utf-8") as f:
    f.write(json.dumps([codex, *args]) + "\n")
os.execv(codex, [codex, *args])
