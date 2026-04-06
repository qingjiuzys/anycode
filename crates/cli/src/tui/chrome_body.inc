pub(crate) fn welcome_lines(permission_mode: &str, require_approval: bool) -> Vec<Line<'static>> {
    let sec = if require_approval {
        tr("tui-approval-on")
    } else {
        tr("tui-approval-off")
    };
    vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  anyCode", style_brand()),
            Span::styled(tr("tui-welcome-tagline"), style_dim()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  · ", style_dim()),
            Span::styled("Enter", Style::default().fg(Color::White)),
            Span::styled(tr("tui-welcome-enter-send"), style_dim()),
        ]),
        Line::from(vec![
            Span::styled("  · ", style_dim()),
            Span::styled("↑↓", Style::default().fg(Color::White)),
            Span::styled(tr("tui-welcome-history"), style_dim()),
            Span::styled("Alt+↑↓", Style::default().fg(Color::White)),
            Span::styled(tr("tui-welcome-alt-arrows"), style_dim()),
            Span::styled("^L", Style::default().fg(Color::White)),
            Span::styled(tr("tui-welcome-clear"), style_dim()),
        ]),
        Line::from(vec![
            Span::styled("  · ", style_dim()),
            Span::styled("^PgUp/Dn", Style::default().fg(Color::White)),
            Span::styled(tr("tui-welcome-scroll-ws"), style_dim()),
            Span::styled("Sh+Enter", Style::default().fg(Color::White)),
            Span::styled(tr("tui-welcome-multiline"), style_dim()),
        ]),
        Line::from(vec![
            Span::styled("  · ", style_dim()),
            Span::styled("?", Style::default().fg(Color::White)),
            Span::styled(tr("tui-welcome-shortcuts"), style_dim()),
            Span::styled("←→", Style::default().fg(Color::White)),
            Span::styled(tr("tui-welcome-arrows-move"), style_dim()),
        ]),
        Line::from(vec![
            Span::styled("  · ", style_dim()),
            Span::styled("/", Style::default().fg(Color::White)),
            Span::styled(tr("tui-welcome-slash-rest"), style_dim()),
        ]),
        Line::from(vec![
            Span::styled("  · ", style_dim()),
            Span::styled("anycode repl", Style::default().fg(Color::White)),
            Span::styled(tr("tui-welcome-repl"), style_dim()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(tr("tui-welcome-perm-label"), style_dim()),
            Span::styled(permission_mode.to_string(), style_warn()),
            Span::styled(tr("tui-welcome-perm-sep"), style_dim()),
            Span::styled(sec, style_dim()),
        ]),
        if require_approval {
            Line::from(vec![
                Span::styled(tr("tui-welcome-skip-label"), style_dim()),
                Span::styled("-I", Style::default().fg(Color::White)),
                Span::styled(" / ", style_dim()),
                Span::styled("--ignore", Style::default().fg(Color::White)),
                Span::styled(tr("tui-welcome-skip-or"), style_dim()),
                Span::styled("ANYCODE_IGNORE_APPROVAL=1", Style::default().fg(Color::Cyan)),
                Span::styled(tr("tui-welcome-skip-restart"), style_dim()),
            ])
        } else {
            Line::from("")
        },
        Line::from(""),
    ]
}

/// `?` 展开时的精简快捷键。
pub(crate) fn sidebar_help_text() -> Text<'static> {
    Text::from(vec![
        Line::from(vec![Span::styled(
            tr("tui-help-title"),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(Span::styled(tr("tui-help-line1"), style_dim())),
        Line::from(Span::styled(tr("tui-help-line2"), style_dim())),
        Line::from(Span::styled(tr("tui-help-line3"), style_dim())),
        Line::from(Span::styled(tr("tui-help-line4"), style_dim())),
        Line::from(Span::styled(tr("tui-help-line5"), style_dim())),
        Line::from(Span::styled(tr("tui-help-line6"), style_dim())),
        Line::from(Span::styled(tr("tui-help-line7"), style_dim())),
        Line::from(""),
        Line::from(Span::styled(tr("tui-help-line8"), style_dim())),
        Line::from(""),
        Line::from(Span::styled(tr("tui-help-approval-keys"), style_dim())),
        Line::from(""),
        Line::from(Span::styled(tr("tui-help-buddy"), style_dim())),
    ])
}

pub(crate) fn agents_lines() -> Vec<String> {
    vec![
        tr("tui-agent-gp"),
        tr("tui-agent-explore"),
        tr("tui-agent-plan"),
        tr("tui-agent-switch"),
    ]
}

pub(crate) fn tools_lines() -> Vec<String> {
    sidebar_tool_lines()
}
