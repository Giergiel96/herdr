# Fork ledger

This is Giergiel96/herdr, a personal fork of [herdrdev/herdr](https://github.com/herdrdev/herdr).
Branch `feat/spaces-group-by-repo` carries every local change on top of upstream `master`.
Keep this file current: one row per change, so a rebase shows at a glance what has to survive.

Upstream base: `2713feb5` (fix: install opencode and kilo integrations under XDG_CONFIG_HOME (#55, herdr 0.9.5).

## Changes

| Change | Files | Notes |
| --- | --- | --- |
| `[ui.sidebar.spaces] group_by = "worktree" \| "cwd"` config option | `src/config/sidebar.rs`, `src/config.rs` | `SpaceGroupBy` enum, default `Worktree` keeps upstream behaviour. |
| cwd grouping in the spaces panel | `src/client/shell/sidebar.rs` | `cwd_group_keys`, `cwd_entries`, `cwd_group_header`, `render_group_header`, `entry_height`, `entry_gap`, `workspace_rows_rect`; `workspace_entries`, `parent_group_key`, `render_parent_group_toggle`, `displayed_workspace_status` take `group_by`. Upstream worktree logic moved unchanged into `worktree_entries`. |
| `WorkspaceEntry` gains `group_header` / `continues_group` / `collapsed_group` and `plain()` | `src/client/shell/state.rs`, `workspace_navigation.rs` | Header line is counted in row heights; members pack without `row_gap`; a collapsed group is its header line only (accent color, left `▾`/`▸` glyph, rolled-up state icon). |
| `group_by` threaded through every `workspace_entries` caller | `endpoint_sidebar.rs`, `endpoint_navigation.rs`, `mobile.rs`, `mouse.rs`, `state.rs` | Multi-machine sidebar renders the same header. Mobile list stays flat. |
| Tests | `src/client/shell/tests/agents_worktrees_notifications.rs`, `src/config/sidebar.rs` | `cwd_grouping_renders_a_header_over_equal_members_and_collapses_them`, `parses_space_group_by_and_rejects_unknown_values`. |
| `[ui.sidebar.spaces] mode = "spaces" \| "agents"`: agent tree in the upper panel | `src/config/sidebar.rs`, `src/client/shell/agent_tree.rs` (new), `sidebar.rs` (`render_space_list` split out, mode branch), `state.rs` (`WorkspaceHit.pane_id`, `ClientWorkspacePress.pane_id`, navigation branch), `mouse.rs` (press carries pane, no drop targets), `endpoint_navigation.rs` (pane focus on click, no drag) | Rows per agent grouped by pane cwd; subagents from the `subagents` pane token (`name@unix_s;...`), collapse key `pane:<id>`; shell rows for agentless workspaces. Multi-machine sidebar and mobile keep the spaces list. |
| Drop the local server's generic "unknown keys" banner when the client parsed the same config cleanly (version skew, not a config error) | `src/client/shell/config.rs`, `src/client/shell/state.rs` | `merged_config_diagnostic` takes `shared_config_file`; only the exact unknown-keys text is filtered, only for the local endpoint. |
| Docs | `docs/next/website/src/data/config-reference.json`, `docs/next/website/src/content/docs/configuration.mdx` | `scripts/config_reference_check.py` must stay green. |

Design constraints worth remembering:

- No wire or protocol change. `ClientShellWorkspace` is frozen in the stable endpoint codec
  (`tests/fixtures/endpoint-snapshot-v1.json`), so the client groups by the existing
  `new_workspace_cwd` field (plus `worktree.key` when provenance exists) instead of a new repo key.
- Group keys are recomputed per entry during render. Fine for tens of workspaces; compute once per
  frame before proposing this upstream.

Outside the repo: `~/.claude/hooks/herdr-subagents.sh` (registered in `~/.claude/settings.json`
for `SubagentStart`, `SubagentStop` and `SessionStart`) writes the `subagents` pane token that the
agents mode renders. It is a Claude Code hook, not part of herdr; keep it when reinstalling the
herdr Claude integration, which only overwrites `herdr-agent-state.sh`.

GitHub Actions are disabled on the fork (repo settings) and `.github/workflows` is deleted in
its own commit, so nothing publishes releases, previews or the website from here. On a rebase,
conflicts in those files resolve with `git rm`. Never open a pull request against
`herdrdev/herdr` from this branch: the upstream gate closes PRs from accounts outside
`.github/APPROVED_CONTRIBUTORS` (see #5605).

## Branches

- `master`: untouched mirror of `upstream/master`. Only ever fast-forward it.
- `develop`: the fork's line, default branch. Feature branches merge here through pull requests on
  the fork (never against `herdrdev/herdr`).

## Rebasing on upstream

```sh
git fetch upstream
git checkout master && git merge --ff-only upstream/master && git push origin master
git checkout develop && git rebase master      # or `git merge master` to keep history
```

Expected conflict spots: `src/client/shell/sidebar.rs` (grouping and render loop),
`endpoint_sidebar.rs` (mirror of the render loop), `state.rs` (`WorkspaceEntry`),
`config-reference.json`. After resolving:

```sh
export ZIG=~/.local/zig-dl/zig-aarch64-macos-0.16.0/zig
env -u HERDR_SOCKET_PATH -u HERDR_CLIENT_SOCKET_PATH cargo clippy --all-targets
env -u HERDR_SOCKET_PATH -u HERDR_CLIENT_SOCKET_PATH cargo test --bin herdr -- cwd_grouping group_by grouped_worktrees duplicate_repo_parents
python3 -I scripts/config_reference_check.py
env -u HERDR_SOCKET_PATH -u HERDR_CLIENT_SOCKET_PATH cargo build --release
```

Then update the upstream base line above and this table. Vendored libghostty-vt pins the Zig
version (`crates/ghostty-vt/build.rs`); when upstream bumps it, fetch the matching Zig into
`~/.local/zig-dl`.

## Install

```sh
cp target/release/herdr ~/.local/bin/herdr
herdr server stop
```

Config that uses the fork:

```toml
[ui.sidebar.spaces]
group_by = "cwd"
```
