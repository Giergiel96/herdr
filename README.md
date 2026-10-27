# herdr, personal fork

This is Giergiel96's fork of [herdrdev/herdr](https://github.com/herdrdev/herdr),
a terminal workspace manager for AI coding agents. It is not the herdr project:
not maintained, supported or released by the herdr maintainers. Install herdr
from [herdr.dev](https://herdr.dev) and report bugs or ideas upstream, not here.
Everything else about herdr itself is in the
[upstream README](https://github.com/herdrdev/herdr#readme) and
[docs](https://herdr.dev/docs/).

## What this fork changes

Client-side changes to the sidebar and the tab row, for a one-agent-per-workspace
workflow. No server or protocol changes; the client attaches to an unmodified
upstream server.

- `[ui.sidebar.spaces] mode = "agents"`: the upper sidebar lists agents instead
  of workspaces, grouped by the agent pane's working directory under collapsible
  header bands, with subagents nested underneath (from the `subagents` pane
  token, reported by a Claude Code hook) and dimmed shell rows for workspaces
  without an agent. Agent names take the color of the agent state; clicking a
  row focuses its pane.
- `[ui.sidebar.spaces] group_by = "cwd"`: workspaces that share a directory sit
  under one collapsible header; a collapsed group is its header line.
- `[ui.agent_bar]`: a one-line bar in place of the tab row, with the focused
  agent on the left and its metadata tokens (context, usage, model) on the right.
- Sidebar token `fg` accepts theme palette color names (`teal`, `peach`, ...)
  so row colors follow `[theme]`.
- `keys.focus_panes` action and F13-F20 decoding for legacy terminal sequences.
- The local server's generic "unknown keys" banner is hidden when the client
  parsed the shared config cleanly (client newer than server).
- GitHub workflows removed; Actions are disabled on this repository.

Each change, the files it touches and the rebase recipe are in
[FORK.md](FORK.md).

## Branches

- `develop`: the only long-lived branch and the default. It is rebased onto
  `upstream/master` whenever the fork catches up with a herdr release, so its
  history rewrites; features land through pull requests on this repository,
  never against `herdrdev/herdr`.

## Building

Same as upstream: Rust from `rust-toolchain.toml` and Zig 0.16.0 for the
vendored libghostty-vt, then `cargo build --release`.

## License

Apache 2.0, as upstream. See [LICENSE](LICENSE).
