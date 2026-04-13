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
        Line::from(Span::styled(tr("tui-help-line7b"), style_dim())),
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
