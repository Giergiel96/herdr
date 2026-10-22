use super::*;

pub(super) fn render_worktree_create_overlay(
    b: &mut Buffer,
    create: &ClientWorktreeCreateOverlay,
    p: &Palette,
) -> Option<OverlayRender> {
    let popup = popup(b.area, 68, 12)?;
    let inner = panel(b, popup, p.accent, p.panel_bg)?;
    put_text(
        b,
        inner.x,
        inner.y,
        inner.width,
        "new worktree",
        Style::default()
            .fg(p.text)
            .bg(p.panel_bg)
            .add_modifier(Modifier::BOLD),
    );
    put_text(
        b,
        inner.x,
        inner.y + 2,
        inner.width,
        " branch",
        Style::default().fg(p.overlay0).bg(p.panel_bg),
    );
    let input = Rect::new(inner.x, inner.y + 3, inner.width, 1);
    b.set_style(input, Style::default().fg(p.text).bg(p.surface0));
    let cursor = text_editor::render(
        b,
        Rect::new(input.x + 1, input.y, input.width.saturating_sub(1), 1),
        &create.branch,
        Style::default().fg(p.text).bg(p.surface0),
    );
    put_text(
        b,
        inner.x,
        inner.y + 5,
        inner.width,
        " checkout",
        Style::default().fg(p.overlay0).bg(p.panel_bg),
    );
    put_text(
        b,
        inner.x,
        inner.y + 6,
        inner.width,
        &format!(" {}", create.checkout_path),
        Style::default().fg(p.subtext0).bg(p.panel_bg),
    );
    if create.creating {
        put_text(
            b,
            inner.x,
            inner.y + 8,
            inner.width,
            " creating…",
            Style::default().fg(p.accent).bg(p.panel_bg),
        );
    } else if let Some(error) = create.error.as_deref() {
        put_text(
            b,
            inner.x,
            inner.y + 8,
            inner.width,
            &format!(" {error}"),
            Style::default().fg(p.red).bg(p.panel_bg),
        );
    }
    let buttons = row(inner, &[20, 12], 2, 9);
    let [primary, cancel] = buttons.as_slice() else {
        return None;
    };
    button(
        b,
        *primary,
        " ↵ create and open ",
        Style::default()
            .fg(contrast(p))
            .bg(p.accent)
            .add_modifier(Modifier::BOLD),
    );
    button(
        b,
        *cancel,
        " esc cancel ",
        Style::default()
            .fg(p.text)
            .bg(p.surface0)
            .add_modifier(Modifier::BOLD),
    );
    Some(OverlayRender {
        area: popup,
        primary: *primary,
        clear: Rect::default(),
        cancel: *cancel,
        navigator_popup: Rect::default(),
        navigator_search: Rect::default(),
        navigator_rows: Vec::new(),
        worktree_search: Rect::default(),
        worktree_rows: Vec::new(),
        cursor: cursor.filter(|_| !create.creating),
        ..OverlayRender::default()
    })
}

pub(super) fn render_worktree_open_overlay(
    b: &mut Buffer,
    open: &ClientWorktreeOpenOverlay,
    p: &Palette,
) -> Option<OverlayRender> {
    let popup_height = (open.entries.len().saturating_mul(2) + 7).clamp(12, 26) as u16;
    let popup = popup(b.area, 96, popup_height)?;
    let inner = panel(b, popup, p.accent, p.panel_bg)?;
    put_text(
        b,
        inner.x,
        inner.y,
        inner.width,
        "open worktree",
        Style::default()
            .fg(p.text)
            .bg(p.panel_bg)
            .add_modifier(Modifier::BOLD),
    );
    let search = Rect::new(inner.x, inner.y + 1, inner.width, 1);
    let filtered = open.filtered_indices();
    put_text(
        b,
        search.x,
        search.y,
        search.width,
        &if open.search_focused {
            " / ".to_owned()
        } else if !open.query.is_empty() {
            format!(" / {}", open.query)
        } else {
            " / filter worktrees".to_owned()
        },
        Style::default()
            .fg(if open.search_focused {
                p.text
            } else {
                p.overlay0
            })
            .bg(p.panel_bg),
    );
    let count = if filtered.len() == open.entries.len() {
        format!("{} checkouts", open.entries.len())
    } else {
        format!("{}/{} checkouts", filtered.len(), open.entries.len())
    };
    let cursor = if open.search_focused {
        text_editor::render(
            b,
            Rect::new(
                search.x + 3,
                search.y,
                search.width.saturating_sub(4 + display_width(&count)),
                1,
            ),
            &open.query,
            Style::default().fg(p.text).bg(p.panel_bg),
        )
    } else {
        None
    };
    put_right_text(
        b,
        search,
        search.y,
        &count,
        Style::default().fg(p.overlay0).bg(p.panel_bg),
    );
    put_text(
        b,
        inner.x,
        inner.y + 2,
        inner.width,
        &"─".repeat(inner.width as usize),
        Style::default().fg(p.surface1).bg(p.panel_bg),
    );
    let body = Rect::new(
        inner.x,
        inner.y + 3,
        inner.width,
        inner.height.saturating_sub(6),
    );
    let visible_count = (body.height / 2).max(1) as usize;
    let selected_position = filtered
        .iter()
        .position(|index| *index == open.selected)
        .unwrap_or(0);
    let start = selected_position
        .saturating_sub(visible_count.saturating_sub(1))
        .min(filtered.len().saturating_sub(visible_count));
    let mut row_hits = Vec::new();
    for (visible, entry_index) in filtered
        .iter()
        .copied()
        .skip(start)
        .take(visible_count)
        .enumerate()
    {
        let entry = &open.entries[entry_index];
        let rect = Rect::new(body.x, body.y + visible as u16 * 2, body.width, 2);
        row_hits.push((rect, entry_index));
        let selected = entry_index == open.selected;
        let style = if selected {
            Style::default().fg(contrast(p)).bg(p.accent)
        } else {
            Style::default().fg(p.text).bg(p.panel_bg)
        };
        b.set_style(rect, style);
        put_text(
            b,
            rect.x,
            rect.y,
            rect.width,
            &format!(" {}", entry.label),
            style.add_modifier(Modifier::BOLD),
        );
        let status = entry.status_label();
        if !status.is_empty() {
            put_right_text(b, rect, rect.y, status, style);
        }
        put_text(
            b,
            rect.x,
            rect.y + 1,
            rect.width,
            &format!(" {}", entry.path),
            if selected {
                style
            } else {
                Style::default().fg(p.overlay0).bg(p.panel_bg)
            },
        );
    }
    if filtered.is_empty() {
        put_text(
            b,
            body.x,
            body.y,
            body.width,
            " no matching worktrees",
            Style::default().fg(p.overlay0).bg(p.panel_bg),
        );
    }
    if open.opening {
        put_text(
            b,
            inner.x,
            inner.bottom() - 3,
            inner.width,
            " opening…",
            Style::default().fg(p.accent).bg(p.panel_bg),
        );
    } else if let Some(error) = open.error.as_deref() {
        put_text(
            b,
            inner.x,
            inner.bottom() - 3,
            inner.width,
            &format!(" {error}"),
            Style::default().fg(p.red).bg(p.panel_bg),
        );
    }
    let buttons = row(inner, &[10, 12], 2, inner.height.saturating_sub(1));
    let [primary, cancel] = buttons.as_slice() else {
        return None;
    };
    button(
        b,
        *primary,
        " ↵ open ",
        Style::default()
            .fg(contrast(p))
            .bg(p.accent)
            .add_modifier(Modifier::BOLD),
    );
    button(
        b,
        *cancel,
        " esc cancel ",
        Style::default()
            .fg(p.text)
            .bg(p.surface0)
            .add_modifier(Modifier::BOLD),
    );
    Some(OverlayRender {
        area: popup,
        primary: *primary,
        clear: Rect::default(),
        cancel: *cancel,
        navigator_popup: Rect::default(),
        navigator_search: Rect::default(),
        navigator_rows: Vec::new(),
        worktree_search: search,
        worktree_rows: row_hits,
        cursor: cursor.filter(|_| !open.opening),
        ..OverlayRender::default()
    })
}

pub(super) fn render_worktree_remove_overlay(
    b: &mut Buffer,
    remove: &ClientWorktreeRemoveOverlay,
    p: &Palette,
) -> Option<OverlayRender> {
    let popup = popup(b.area, 72, 10)?;
    let inner = panel(b, popup, p.red, p.panel_bg)?;
    put_text(
        b,
        inner.x,
        inner.y,
        inner.width,
        " delete worktree checkout?",
        Style::default()
            .fg(p.red)
            .bg(p.panel_bg)
            .add_modifier(Modifier::BOLD),
    );
    put_text(
        b,
        inner.x,
        inner.y + 1,
        inner.width,
        " This removes the checkout folder:",
        Style::default().fg(p.text).bg(p.panel_bg),
    );
    put_text(
        b,
        inner.x,
        inner.y + 2,
        inner.width,
        &format!(" {}", remove.path),
        Style::default().fg(p.subtext0).bg(p.panel_bg),
    );
    let force = remove.force_confirmation;
    // Force confirmation needs two warning lines at normal heights. Short
    // terminals get one line so the status row and buttons keep separate rows.
    let compact_warning = force && inner.height < 7;
    if !compact_warning {
        put_text(
            b,
            inner.x,
            inner.y + 3,
            inner.width,
            " The branch is not deleted. The Herdr workspace will close.",
            Style::default().fg(p.text).bg(p.panel_bg),
        );
    }
    let warning_y = if compact_warning {
        inner.y + 3
    } else {
        inner.y + 4
    };
    let warning_lines: &[&str] = if !force {
        &[]
    } else if inner.height >= 8 {
        &[
            " Uncommitted or unpushed work in this checkout",
            " or its submodules will be permanently deleted.",
        ]
    } else {
        &[" Uncommitted or unpushed work will be deleted."]
    };
    for (offset, line) in warning_lines.iter().enumerate() {
        put_text(
            b,
            inner.x,
            warning_y + offset as u16,
            inner.width,
            line,
            Style::default().fg(p.red).bg(p.panel_bg),
        );
    }
    let status_y = if force {
        warning_y + warning_lines.len() as u16
    } else {
        inner.y + 5
    };
    if remove.removing {
        put_text(
            b,
            inner.x,
            status_y,
            inner.width,
            " removing…",
            Style::default().fg(p.accent).bg(p.panel_bg),
        );
    } else if let Some(error) = remove.error.as_deref() {
        put_text(
            b,
            inner.x,
            status_y,
            inner.width,
            &format!(" {error}"),
            Style::default().fg(p.red).bg(p.panel_bg),
        );
    }
    let buttons_off = if force { status_y - inner.y + 1 } else { 7 };
    let buttons = row(inner, &[18, 12], 2, buttons_off);
    let [primary, cancel] = buttons.as_slice() else {
        return None;
    };
    button(
        b,
        *primary,
        if remove.force_confirmation {
            " ↵ delete anyway "
        } else {
            " ↵ remove "
        },
        Style::default()
            .fg(contrast(p))
            .bg(p.red)
            .add_modifier(Modifier::BOLD),
    );
    button(
        b,
        *cancel,
        " esc cancel ",
        Style::default()
            .fg(p.text)
            .bg(p.surface0)
            .add_modifier(Modifier::BOLD),
    );
    Some(OverlayRender {
        area: popup,
        primary: *primary,
        clear: Rect::default(),
        cancel: *cancel,
        navigator_popup: Rect::default(),
        navigator_search: Rect::default(),
        navigator_rows: Vec::new(),
        worktree_search: Rect::default(),
        worktree_rows: Vec::new(),
        cursor: None,
        ..OverlayRender::default()
    })
}
