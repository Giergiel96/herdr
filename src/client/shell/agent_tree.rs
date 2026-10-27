//! The agents panel mode of the upper sidebar: one row per agent, grouped by
//! the agent pane's working directory, with reported subagents nested under
//! their agent. Workspaces without an agent appear as dimmed shell rows so
//! they stay reachable. Pure projection of the client snapshot; nothing here
//! touches the server.

use super::render::{display_width, put_right_text, put_segment, put_text, ShellRenderState};
use super::*;
use crate::api::schema::AgentStatus;
use crate::protocol::ClientShellAgent;

/// Pane metadata token that lists running subagents as `name@unix_seconds`
/// entries separated by `;`. Reported by the agent integration hooks.
pub(super) const SUBAGENTS_TOKEN: &str = "subagents";
const SUBAGENT_COLLAPSE_PREFIX: &str = "pane:";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AgentTreeKind {
    Header,
    Agent,
    Shell,
    Subagent,
}

#[derive(Debug, Clone)]
pub(super) struct AgentTreeEntry {
    pub(super) kind: AgentTreeKind,
    /// Group key for headers and members; `pane:<id>` collapses subagents.
    pub(super) key: String,
    pub(super) workspace_index: usize,
    pub(super) pane_id: Option<String>,
    pub(super) label: String,
    pub(super) status: AgentStatus,
    pub(super) count: usize,
    pub(super) started_unix: Option<u64>,
    pub(super) collapsed: bool,
    pub(super) last: bool,
}

struct Member<'a> {
    workspace_index: usize,
    pane_id: Option<String>,
    agent: Option<&'a ClientShellAgent>,
    cwd: String,
}

fn pane_cwd<'a>(snapshot: &'a ClientShellSnapshot, pane_id: &str) -> Option<&'a str> {
    let pane = snapshot.panes.iter().find(|pane| pane.pane_id == pane_id)?;
    pane.cwd
        .as_deref()
        .or(pane.foreground_cwd.as_deref())
        .filter(|cwd| !cwd.is_empty())
}

fn members(snapshot: &ClientShellSnapshot) -> Vec<Member<'_>> {
    let mut members = Vec::new();
    for (workspace_index, workspace) in snapshot.workspaces.iter().enumerate() {
        let mut agents = snapshot
            .agents
            .iter()
            .filter(|agent| agent.workspace_id == workspace.workspace_id)
            .collect::<Vec<_>>();
        agents.sort_by(|left, right| left.pane_id.cmp(&right.pane_id));
        if agents.is_empty() {
            let pane_id = snapshot
                .panes
                .iter()
                .filter(|pane| pane.workspace_id == workspace.workspace_id)
                .min_by_key(|pane| (!pane.focused, pane.pane_id.clone()))
                .map(|pane| pane.pane_id.clone());
            members.push(Member {
                workspace_index,
                pane_id,
                agent: None,
                cwd: workspace.new_workspace_cwd.clone(),
            });
            continue;
        }
        for agent in agents {
            let cwd = pane_cwd(snapshot, &agent.pane_id)
                .map(str::to_owned)
                .unwrap_or_else(|| workspace.new_workspace_cwd.clone());
            members.push(Member {
                workspace_index,
                pane_id: Some(agent.pane_id.clone()),
                agent: Some(agent),
                cwd,
            });
        }
    }
    members
}

fn group_key(member: &Member<'_>, snapshot: &ClientShellSnapshot) -> String {
    if member.cwd.is_empty() {
        format!(
            "ws:{}",
            snapshot.workspaces[member.workspace_index].workspace_id
        )
    } else {
        member.cwd.clone()
    }
}

fn group_label(key: &str, snapshot: &ClientShellSnapshot, head: &Member<'_>) -> String {
    std::path::Path::new(key)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|label| !label.is_empty())
        .unwrap_or_else(|| snapshot.workspaces[head.workspace_index].label.clone())
}

/// Parse the `subagents` pane token into `(name, started_unix)` pairs.
pub(super) fn parse_subagents(value: &str) -> Vec<(String, Option<u64>)> {
    value
        .split(';')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| match entry.rsplit_once('@') {
            Some((name, started)) if !name.is_empty() => {
                (name.to_owned(), started.trim().parse::<u64>().ok())
            }
            _ => (entry.to_owned(), None),
        })
        .collect()
}

/// Subagents from the `subagents` token plus its `subagents_2`.. overflow
/// chunks, which the reporter uses because token values are capped at 80
/// characters.
fn subagents(agent: &ClientShellAgent) -> Vec<(String, Option<u64>)> {
    let mut chunks = agent
        .tokens
        .iter()
        .filter_map(|(name, value)| {
            if name == SUBAGENTS_TOKEN {
                return Some((1, value));
            }
            name.strip_prefix(SUBAGENTS_TOKEN)?
                .strip_prefix('_')?
                .parse::<usize>()
                .ok()
                .map(|index| (index, value))
        })
        .collect::<Vec<_>>();
    chunks.sort_by_key(|(index, _)| *index);
    chunks
        .into_iter()
        .flat_map(|(_, value)| parse_subagents(value))
        .collect()
}

fn member_status(member: &Member<'_>) -> AgentStatus {
    member
        .agent
        .map_or(AgentStatus::Unknown, |agent| agent.agent_status)
}

pub(super) fn entries(
    snapshot: &ClientShellSnapshot,
    collapsed_groups: &HashSet<String>,
) -> Vec<AgentTreeEntry> {
    let members = members(snapshot);
    let keys = members
        .iter()
        .map(|member| group_key(member, snapshot))
        .collect::<Vec<_>>();
    let mut group_sizes = HashMap::<&str, usize>::new();
    for key in &keys {
        *group_sizes.entry(key.as_str()).or_default() += 1;
    }
    let mut emitted = HashSet::<&str>::new();
    let mut out = Vec::new();
    for (index, member) in members.iter().enumerate() {
        let key = keys[index].as_str();
        if !emitted.insert(key) {
            continue;
        }
        let group = members
            .iter()
            .enumerate()
            .filter(|(candidate, _)| keys[*candidate] == key)
            .map(|(_, member)| member)
            .collect::<Vec<_>>();
        let grouped = group_sizes.get(key).copied().unwrap_or(1) > 1;
        if grouped {
            let collapsed = collapsed_groups.contains(key);
            out.push(AgentTreeEntry {
                kind: AgentTreeKind::Header,
                key: key.to_owned(),
                workspace_index: member.workspace_index,
                pane_id: None,
                label: group_label(key, snapshot, member),
                status: group
                    .iter()
                    .map(|member| member_status(member))
                    .max_by_key(|status| status_priority(*status))
                    .unwrap_or(AgentStatus::Unknown),
                count: group.len(),
                started_unix: None,
                collapsed,
                last: false,
            });
            if collapsed {
                continue;
            }
        }
        for member in group.iter() {
            let workspace = &snapshot.workspaces[member.workspace_index];
            let children = member.agent.map(subagents).unwrap_or_default();
            let subagent_key = member
                .pane_id
                .as_deref()
                .map(|pane_id| format!("{SUBAGENT_COLLAPSE_PREFIX}{pane_id}"))
                .unwrap_or_default();
            let children_collapsed =
                !children.is_empty() && collapsed_groups.contains(&subagent_key);
            out.push(AgentTreeEntry {
                kind: if member.agent.is_some() {
                    AgentTreeKind::Agent
                } else {
                    AgentTreeKind::Shell
                },
                key: subagent_key.clone(),
                workspace_index: member.workspace_index,
                pane_id: member.pane_id.clone(),
                label: workspace.label.clone(),
                status: member_status(member),
                count: children.len(),
                started_unix: None,
                collapsed: children_collapsed,
                last: false,
            });
            if children_collapsed {
                continue;
            }
            let total = children.len();
            for (child_index, (name, started_unix)) in children.into_iter().enumerate() {
                out.push(AgentTreeEntry {
                    kind: AgentTreeKind::Subagent,
                    key: subagent_key.clone(),
                    workspace_index: member.workspace_index,
                    pane_id: member.pane_id.clone(),
                    label: name,
                    status: AgentStatus::Working,
                    count: 0,
                    started_unix,
                    collapsed: false,
                    last: child_index + 1 == total,
                });
            }
        }
    }
    out
}

/// Keyboard navigation order: one entry per workspace, in tree order.
pub(super) fn navigation_entries(
    snapshot: &ClientShellSnapshot,
    collapsed_groups: &HashSet<String>,
) -> Vec<WorkspaceEntry> {
    let mut seen = HashSet::new();
    entries(snapshot, collapsed_groups)
        .into_iter()
        .filter(|entry| matches!(entry.kind, AgentTreeKind::Agent | AgentTreeKind::Shell))
        .filter(|entry| seen.insert(entry.workspace_index))
        .map(|entry| WorkspaceEntry::plain(entry.workspace_index))
        .collect()
}

fn elapsed_label(started_unix: Option<u64>, now_unix: u64) -> String {
    let Some(started) = started_unix else {
        return String::new();
    };
    let seconds = now_unix.saturating_sub(started);
    if seconds < 60 {
        format!("{seconds}s")
    } else if seconds < 3600 {
        format!("{}m", seconds / 60)
    } else if seconds < 86_400 {
        format!("{}h{:02}m", seconds / 3600, (seconds % 3600) / 60)
    } else {
        format!("{}d", seconds / 86_400)
    }
}

fn entry_rows(
    entry: &AgentTreeEntry,
    snapshot: &ClientShellSnapshot,
    config: &ClientShellConfig,
) -> Vec<Vec<crate::ui::ResolvedToken>> {
    match entry.kind {
        AgentTreeKind::Agent | AgentTreeKind::Shell => super::sidebar::workspace_rows(
            &snapshot.workspaces[entry.workspace_index],
            entry.status,
            false,
            &config.spaces,
        ),
        AgentTreeKind::Header | AgentTreeKind::Subagent => Vec::new(),
    }
}

fn entry_height(entry: &AgentTreeEntry, rows: usize) -> u16 {
    match entry.kind {
        AgentTreeKind::Header | AgentTreeKind::Subagent => 1,
        AgentTreeKind::Agent | AgentTreeKind::Shell => rows.max(1).min(u16::MAX as usize) as u16,
    }
}

fn gap_before(next: &AgentTreeEntry, row_gap: u16) -> u16 {
    u16::from(next.kind != AgentTreeKind::Subagent) * row_gap
}

fn render_header(
    buffer: &mut Buffer,
    line: Rect,
    entry: &AgentTreeEntry,
    highlighted: bool,
    indicators: crate::config::StatusIndicatorStyle,
    palette: &Palette,
) {
    if highlighted {
        buffer.set_style(line, Style::default().bg(palette.active_row_bg));
    }
    let accent = Style::default().fg(palette.accent);
    let right = line.right().saturating_sub(1);
    let mut x = put_segment(
        buffer,
        line.x,
        line.y,
        right,
        if entry.collapsed { "▸ " } else { "▾ " },
        accent,
    );
    if entry.collapsed {
        x = put_segment(
            buffer,
            x,
            line.y,
            right,
            status_icon(entry.status, indicators),
            Style::default().fg(status_color(entry.status, palette)),
        );
        x = x.saturating_add(1);
    }
    x = put_segment(
        buffer,
        x,
        line.y,
        right,
        &entry.label,
        accent.add_modifier(Modifier::BOLD),
    );
    let count = entry.count.to_string();
    let count_width = display_width(&count).saturating_add(1);
    let rule_right = right.saturating_sub(count_width);
    if x.saturating_add(1) < rule_right {
        x = put_segment(
            buffer,
            x.saturating_add(1),
            line.y,
            rule_right,
            &"─".repeat(usize::from(rule_right.saturating_sub(x).saturating_sub(1))),
            accent.add_modifier(Modifier::DIM),
        );
    }
    put_segment(buffer, x.saturating_add(1), line.y, right, &count, accent);
}

fn render_subagent(
    buffer: &mut Buffer,
    line: Rect,
    entry: &AgentTreeEntry,
    now_unix: u64,
    palette: &Palette,
) {
    let right = line.right().saturating_sub(1);
    let connector = if entry.last { "   └ " } else { "   ├ " };
    let mut x = put_segment(
        buffer,
        line.x,
        line.y,
        right,
        connector,
        Style::default().fg(palette.overlay0),
    );
    let elapsed = elapsed_label(entry.started_unix, now_unix);
    let label_right = right.saturating_sub(display_width(&elapsed).saturating_add(1));
    x = put_segment(
        buffer,
        x,
        line.y,
        label_right,
        &entry.label,
        Style::default().fg(palette.subtext0),
    );
    if !elapsed.is_empty() {
        put_right_text(
            buffer,
            Rect::new(x, line.y, right.saturating_sub(x), 1),
            line.y,
            &elapsed,
            Style::default().fg(palette.overlay1),
        );
    }
}

/// Accent bar along the left edge of the focused agent's rows.
fn render_focus_bar(buffer: &mut Buffer, rect: Rect, palette: &Palette) {
    for y in rect.y..rect.bottom() {
        put_text(
            buffer,
            rect.x,
            y,
            1,
            "▎",
            Style::default().fg(palette.accent),
        );
    }
}

fn render_subagent_toggle(
    buffer: &mut Buffer,
    rect: Rect,
    entry: &AgentTreeEntry,
    palette: &Palette,
) -> Option<(Rect, String)> {
    if entry.count == 0 || entry.key.is_empty() {
        return None;
    }
    let toggle = Rect::new(rect.right().saturating_sub(1), rect.y, 1, 1);
    put_text(
        buffer,
        toggle.x,
        toggle.y,
        toggle.width,
        if entry.collapsed { "▸" } else { "▾" },
        Style::default().fg(palette.accent),
    );
    Some((toggle, entry.key.clone()))
}

pub(super) fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default()
}

/// Render the agent tree into the upper sidebar body. Mirrors the spaces list
/// contract: fills `hits.workspaces`, the scroll metrics and the body rect.
pub(super) fn render_agent_tree(
    buffer: &mut Buffer,
    workspace_area: Rect,
    snapshot: &ClientShellSnapshot,
    config: &ClientShellConfig,
    state: &mut ShellRenderState<'_>,
    hits: &mut ShellHitMap,
) {
    let palette = &config.palette;
    let now = now_unix();
    let entries = entries(snapshot, state.collapsed_groups);
    let body = Rect::new(
        workspace_area.x,
        workspace_area.y.saturating_add(WORKSPACE_HEADER_ROWS),
        workspace_area.width,
        workspace_area
            .height
            .saturating_sub(WORKSPACE_HEADER_ROWS + 1),
    );
    hits.workspace_body = body;
    let rows = entries
        .iter()
        .map(|entry| entry_rows(entry, snapshot, config))
        .collect::<Vec<_>>();
    let row_heights = entries
        .iter()
        .zip(&rows)
        .map(|(entry, rows)| entry_height(entry, rows.len()))
        .collect::<Vec<_>>();
    let gaps = entries
        .iter()
        .enumerate()
        .map(|(index, _)| {
            entries
                .get(index + 1)
                .map_or(0, |next| gap_before(next, config.spaces.row_gap))
        })
        .collect::<Vec<_>>();
    let mut metrics = super::scroll::list_scroll_metrics(
        &row_heights,
        &gaps,
        body.height,
        *state.workspace_scroll,
    );
    if !body.is_empty() && std::mem::take(state.reveal_focused_workspace) {
        if let Some(target) = entries.iter().position(|entry| {
            entry.kind != AgentTreeKind::Subagent
                && snapshot.workspaces[entry.workspace_index].focused
        }) {
            *state.workspace_scroll = super::scroll::list_scroll_start_to_reveal(
                &row_heights,
                &gaps,
                body.height,
                *state.workspace_scroll,
                target,
            );
            metrics = super::scroll::list_scroll_metrics(
                &row_heights,
                &gaps,
                body.height,
                *state.workspace_scroll,
            );
        }
    }
    hits.workspace_max_scroll = metrics.max_offset_from_bottom;
    hits.workspace_scroll_metrics = Some(metrics);
    *state.workspace_scroll = metrics
        .max_offset_from_bottom
        .saturating_sub(metrics.offset_from_bottom);
    let show_scrollbar = metrics.max_offset_from_bottom > 0 && body.width > 1;
    let content_width = body.width.saturating_sub(u16::from(show_scrollbar));
    let navigating = state.selected_workspace_id.is_some();
    let mut y = body.y;
    for (position, entry) in entries.iter().enumerate().skip(*state.workspace_scroll) {
        let height = row_heights[position].min(body.height);
        if y.saturating_add(height) > body.bottom() {
            break;
        }
        let rect = Rect::new(body.x, y, content_width, height);
        let workspace = &snapshot.workspaces[entry.workspace_index];
        let selected = state.selected_workspace_id.is_some_and(|target| {
            target.matches(state.active_endpoint_id, &workspace.workspace_id)
        });
        let group_toggle = match entry.kind {
            AgentTreeKind::Header => {
                let highlighted = entry.collapsed
                    && entries_in_group(&entries, &entry.key)
                        .any(|member| snapshot.workspaces[member.workspace_index].focused);
                render_header(
                    buffer,
                    rect,
                    entry,
                    highlighted || selected,
                    config.status_indicators,
                    palette,
                );
                Some((rect, entry.key.clone()))
            }
            AgentTreeKind::Agent | AgentTreeKind::Shell => {
                let focused = workspace.focused
                    && entry
                        .pane_id
                        .as_deref()
                        .is_none_or(|pane_id| pane_is_focused_or_only(snapshot, pane_id));
                if selected {
                    buffer.set_style(rect, Style::default().bg(palette.selection_bg));
                } else if focused {
                    buffer.set_style(rect, Style::default().bg(palette.active_row_bg));
                }
                super::sidebar::render_workspace_rows(
                    buffer,
                    rect,
                    entry.status,
                    config.status_indicators,
                    &WorkspaceEntry::plain(entry.workspace_index),
                    rows[position].clone(),
                    focused,
                    selected,
                    navigating,
                    false,
                    palette,
                );
                if entry.kind == AgentTreeKind::Shell {
                    buffer.set_style(
                        rect,
                        Style::default()
                            .fg(palette.overlay0)
                            .add_modifier(Modifier::DIM),
                    );
                }
                if focused || selected {
                    render_focus_bar(buffer, rect, palette);
                }
                render_subagent_toggle(buffer, rect, entry, palette)
            }
            AgentTreeKind::Subagent => {
                render_subagent(buffer, rect, entry, now, palette);
                None
            }
        };
        hits.workspaces.push(WorkspaceHit {
            rect,
            endpoint_id: ClientEndpointId::Local,
            workspace_id: workspace.workspace_id.clone(),
            pane_id: entry.pane_id.clone(),
            indented: entry.kind == AgentTreeKind::Subagent,
            group_toggle,
        });
        let gap = entries
            .get(position + 1)
            .map_or(0, |next| gap_before(next, config.spaces.row_gap));
        y = y.saturating_add(height + gap);
    }
    if show_scrollbar {
        let track = Rect::new(body.right().saturating_sub(1), body.y, 1, body.height);
        hits.workspace_scrollbar = track;
        super::scroll::render_list_scrollbar(buffer, track, metrics, palette);
    }
}

fn entries_in_group<'a>(
    entries: &'a [AgentTreeEntry],
    key: &'a str,
) -> impl Iterator<Item = &'a AgentTreeEntry> + 'a {
    entries
        .iter()
        .filter(move |entry| entry.kind == AgentTreeKind::Header && entry.key == key)
}

fn pane_is_focused_or_only(snapshot: &ClientShellSnapshot, pane_id: &str) -> bool {
    let Some(pane) = snapshot.panes.iter().find(|pane| pane.pane_id == pane_id) else {
        return true;
    };
    pane.focused
        || snapshot
            .agents
            .iter()
            .filter(|agent| agent.workspace_id == pane.workspace_id)
            .count()
            <= 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_subagent_token_entries() {
        assert_eq!(
            parse_subagents("coder@1700000000;reviewer@abc; lone ;"),
            vec![
                ("coder".to_owned(), Some(1_700_000_000)),
                ("reviewer".to_owned(), None),
                ("lone".to_owned(), None),
            ]
        );
        assert!(parse_subagents("").is_empty());
    }

    #[test]
    fn subagent_chunks_are_joined_in_order() {
        let agent = ClientShellAgent {
            pane_id: "p".into(),
            workspace_id: "w".into(),
            tab_id: "t".into(),
            name: None,
            display_agent: None,
            agent: None,
            title: None,
            terminal_title: None,
            terminal_title_stripped: None,
            agent_status: AgentStatus::Working,
            state_change_seq: 0,
            state_labels: Vec::new(),
            tokens: vec![
                ("subagents_2".into(), "c@3".into()),
                ("subagents".into(), "a@1;b@2".into()),
                ("other".into(), "x".into()),
            ],
            focused: false,
        };
        assert_eq!(
            subagents(&agent)
                .into_iter()
                .map(|(name, _)| name)
                .collect::<Vec<_>>(),
            ["a", "b", "c"]
        );
    }

    #[test]
    fn elapsed_labels_round_down() {
        assert_eq!(elapsed_label(Some(100), 130), "30s");
        assert_eq!(elapsed_label(Some(100), 100 + 14 * 60 + 59), "14m");
        assert_eq!(elapsed_label(Some(0), 3600 * 2 + 5 * 60), "2h05m");
        assert_eq!(elapsed_label(Some(0), 86_400 * 3 + 7), "3d");
        assert_eq!(elapsed_label(None, 5), "");
    }
}
