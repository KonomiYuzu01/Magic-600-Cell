---
id: codex-mcp-isolation
type: evidence
status: draft
visibility: public
summary: Signed in with ChatGPT, codex-cli 0.159.0 starts the MCP servers of the plugins installed on the account and connects the hosted ChatGPT connector endpoint in every wrapper call, despite --ignore-user-config. In a local harness, --disable plugins and --disable apps stop both and the review still succeeds. The wrapper change that passes them waits for the Codex plan check and review.
related: [development-loop, owner-decisions-2026-09-30]
supersedes: []
claims:
  - {id: harness-results, evidence_kind: fixture, path: work/experiments/codex-mcp-isolation/results/summary.json, sha256: 58057406e7649bc34a4ebd3897f0a32559e2d64e3ce535cd8ce6b30d2ce6e077, checked_at: 2026-10-20}
  - {id: harness-method, evidence_kind: fixture, path: work/experiments/codex-mcp-isolation/README.md, sha256: 4b3a2203b43661300c62a80ced12cbfef5e0f73d76c4bb6551a289f0c9e2ad2f, checked_at: 2026-10-20}
  - {id: proposed-change, evidence_kind: source, path: work/experiments/codex-mcp-isolation/proposed-wrapper-change.patch, sha256: aa71ebd47872d37a0e58efaa3cd1b6104eda4e21620c5b38da279924ed3834e5, checked_at: 2026-10-20}
  - {id: wrapper-ignores-only-user-config, evidence_kind: source, path: tools/agents/codex_review.py, line: 73, sha256: 1ec3b8802fdfb8f4f16ab0bcd19409f9fed6f2eb3dea2e383f91ab7f96750af6, checked_at: 2026-10-20}
---

# Codex MCP isolation

The review wrapper (`tools/agents/codex_review.py`) passes `--ignore-user-config` to every Codex call. The aim was that MCP servers and plugins from the user config never load. In a cloud session on 20 October 2026, each wrapper review call still had a child process: the MCP server of the curated codex-security plugin.

## Why it happens

codex-cli 0.159.0 turns four features on by default: `plugins`, `remote_plugin`, `apps` and `skill_mcp_dependency_install` (see `codex features list`). `--ignore-user-config` skips only `config.toml`.

With a ChatGPT sign-in, Codex first asks the ChatGPT backend which plugins are installed on the account. That includes plugins installed in other Codex clients, such as the Codex app. Codex then:

- downloads their bundles into `CODEX_HOME/plugins/cache/`;
- loads them as enabled, from the account state rather than any config file;
- starts their MCP servers outside the sandbox, also in calls made with `--sandbox read-only`;
- connects the hosted ChatGPT connector endpoint (`/backend-api/ps/mcp`), with the `apps` feature.

The [isolation harness](../../../work/experiments/codex-mcp-isolation/README.md) reproduces both behaviours with the unmodified wrapper, the real Codex binary and a local stand-in for the ChatGPT backend. Its results are in [summary.json](../../../work/experiments/codex-mcp-isolation/results/summary.json):

- `--disable plugins` stops the plugin download and the plugin MCP server.
- `--disable apps` stops the connector endpoint.
- With both disabled, the review still succeeds.
- `--disable remote_plugin` alone does not stop the plugin server.

## What codex-security can do

This comes from a static read of its public bundle (openai/plugins 0722921d, plugin version 0.1.32); nothing was executed.

- **Tools:** it registers 47 tools, 22 of them visible to the model.
- **Rights:** most tools start Python or git with the user's full rights and environment, and write under `CODEX_HOME/state/plugins/codex-security/`.
- **Model spend:** a deep-scan tool starts its own `codex exec` workers. They do not ignore the user config, and they spend the subscription or an API key.
- **Sign-in access:** the server is passed `CODEX_HOME`, so it can read the sign-in file. It also asks for API-key and AWS variables; the wrapper already removes names containing KEY, SECRET or TOKEN.
- **Network:** the server itself uses no network.

Other curated plugins, such as figma, github, gmail and linear, declare remote HTTP MCP servers. When enabled on the account, they would connect from the machine without any local process.

## Proposed change

The [proposed patch](../../../work/experiments/codex-mcp-isolation/proposed-wrapper-change.patch) makes every `codex exec` call of the wrapper pass four flags: `--disable plugins`, `--disable apps`, `--disable remote_plugin` and `--disable skill_mcp_dependency_install`. `--disable` rejects an unknown feature name, so a renamed feature makes the call fail instead of being skipped silently. A check before each call is also planned: it confirms that Codex reports these features off and lists no enabled MCP server.

The wrapper is a critical path, so the patch waits for the Codex plan check and review. Until it lands, every wrapper call on a machine signed in with ChatGPT starts the MCP servers of the plugins enabled on that account.

## Rechecking

Run the harness again after any Codex upgrade. A new default or a new MCP source shows up in three ways:

- the stand-in plugin server starts;
- `codex exec` gets a new child process;
- the mock backend receives a new request.
