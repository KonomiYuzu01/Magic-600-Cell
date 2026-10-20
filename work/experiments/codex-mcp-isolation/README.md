# Codex MCP isolation harness

Do wrapper calls (`tools/agents/codex_review.py`) start MCP servers that the user config does not declare? This harness answers that for the pinned codex-cli. It runs the real wrapper and the real Codex binary against a local stand-in for the ChatGPT backend.

## Question and answer

In a cloud session on 20 October 2026, every wrapper review call had a child process `node <CODEX_HOME>/plugins/cache/openai-curated-remote/codex-security/<version>/scripts/../mcp/server.mjs --stdio`. That happened although the wrapper passes `--ignore-user-config` and the user config held only a project trust level.

Codex 0.159.0 turns the features `plugins`, `remote_plugin`, `apps` and `skill_mcp_dependency_install` on by default (`codex features list`). `--ignore-user-config` skips only `$CODEX_HOME/config.toml`. With a ChatGPT sign-in, Codex works as follows:

- It asks the ChatGPT backend which plugins are installed on the account (`GET /backend-api/ps/plugins/installed`).
- It downloads their bundles into `$CODEX_HOME/plugins/cache/<marketplace>/<plugin>/<version>/`.
- It loads them as enabled, from the account state rather than from any config file.
- It starts their stdio MCP servers outside the sandbox, as direct children of `codex exec`, and passes them the variables they request. codex-security requests `CODEX_HOME`, so it can read `auth.json`.
- With `apps` on, it also connects the hosted connector endpoint `POST /backend-api/ps/mcp`.

`--disable plugins` stops the plugin path; `--disable remote_plugin` alone does not. `--disable apps` stops the connector endpoint.

## Method

- `harness/mock_backend.py` is an HTTPS stand-in for `https://chatgpt.com/backend-api/` on 127.0.0.1, served with a throwaway test CA from `harness/prepare.sh`. It reports one installed plugin, `codex-security` from the `openai-curated-remote` marketplace, serves its bundle, answers the model call with one fixed `pass` review, and logs every request (method, path, status, sizes; no bodies).
- `harness/build_bundle.py` builds the stand-in bundle. It has the real plugin's name, version and MCP layout (`.mcp.json` with `env_vars` including `CODEX_HOME`), but its server only logs that it started and answers the MCP handshake. It offers one inert tool.
- `harness/write_auth.py` writes a fake ChatGPT sign-in into a fresh `CODEX_HOME`: an unsigned token and dummy values, no real credentials.
- `harness/run_case.sh` runs one unmodified wrapper review call from a throwaway copy of the repository, inside `unshare -rn`, a network namespace with loopback only, so nothing leaves the machine. The wrapper finds Codex through its `MAGIC600_CODEX_CMD` hook. That hook points at `harness/codex_shim.py`, which runs the given Codex binary with the wrapper's exact arguments. The shim adds only the two base-URL overrides that point Codex at the mock, plus any `SHIM_EXTRA` flags, all right after `exec`. Process snapshots every 0.3 s record the children of `codex exec`.
- `harness/run_all.sh` runs the standard cases and prints the sanitized summary from `harness/summarize.py`.

Run it on Linux with user namespaces, Python 3 and OpenSSL. `CODEX_BIN` must be the native codex-cli 0.159.x binary:

```text
CODEX_BIN=<codex binary> bash work/experiments/codex-mcp-isolation/harness/run_all.sh <new work dir> <base-rev> <candidate-rev> > summary.json
```

Never point it at a real `CODEX_HOME`: each case creates its own home with the fake sign-in.

## Results, codex-cli 0.159.0

[`results/summary.json`](results/summary.json) is the run of 20 October 2026: base 7cabb88, candidate = the wrapper with the four `--disable` flags (the change below).

| Case | Flags after `exec` | Stand-in MCP server started | Plugin bundle downloaded | `POST /ps/mcp` | Children of `codex exec` | Wrapper |
|---|---|---|---|---|---|---|
| A-base | none (current wrapper) | yes, child of `codex exec`, got `initialize` and `tools/list` | yes | 2 | stand-in server | valid pass |
| B1 | `--disable remote_plugin` | yes | yes | 2 | stand-in server | valid pass |
| B2 | `--disable plugins` | no | no | 2 | none | valid pass |
| B3 | `--disable plugins --disable apps` | no | no | 0 | none | valid pass |
| C-candidate | the four flags of the candidate | no | no | 0 | none | valid pass |

In an earlier run of B1, a snapshot also caught `git ls-remote https://github.com/openai/plugins.git` as a child: with the remote catalog off, Codex syncs the local curated marketplace from GitHub. The process is short-lived, so snapshots catch it only sometimes.

## Wrapper change

The wrapper passes `--disable plugins --disable apps --disable remote_plugin --disable skill_mcp_dependency_install` to every `codex exec` call: plan, review and implement (`MCP_ISOLATION` in `tools/agents/codex_review.py`). Tests in `tests/test_codex_review.py` and `tests/test_codex_implement.py` fail if any of the four flags is missing. Case C-candidate ran this change.

The owner waived the Codex plan check and review for it (owner exception, 20 October 2026). A check before each call, that Codex reports these features off and lists no enabled MCP server, was planned but not implemented.

## Limits

- Synthetic backend and a stand-in plugin. The real codex-security server was not run; its capabilities were read from its public bundle (openai/plugins 0722921d).
- Linux only. Windows behaviour follows from the same source, but no Windows run was made.
- Only codex-cli 0.159.0 was tested. Another version needs a new run before the result applies to it.
- Other curated plugins declare remote HTTP MCP servers. For example, figma uses `https://mcp.figma.com/mcp` and gmail `https://gmailmcp.googleapis.com/mcp/v1`. When enabled on the account, they would connect from the machine, with no local process. They are covered by `--disable plugins` but were not exercised here.
- Enterprise cloud-managed configuration, which needs a business workspace sign-in, was not exercised.
