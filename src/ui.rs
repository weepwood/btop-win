use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols::Marker,
    text::{Line, Span, Text},
    widgets::{
        Axis, Block, BorderType, Borders, Cell, Chart, Clear, Dataset, Gauge, GraphType,
        Paragraph, Row, Table, Wrap,
    },
};

use crate::{
    app::App,
    theme::Theme,
    utils::{format_bytes, format_rate, format_uptime, percent},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProcessColumns {
    Compact,
    Io,
    Full,
}

pub fn draw(frame: &mut Frame<'_>, app: &mut App, theme: Theme) {
    let area = frame.area();
    frame.render_widget(
        Block::default().style(
            Style::default()
                .fg(theme.foreground)
                .bg(theme.background),
        ),
        area,
    );

    if area.width < 72 || area.height < 20 {
        draw_too_small(frame, area, theme);
        return;
    }

    if area.height < 30 {
        draw_compact(frame, app, area, theme);
        if app.show_help {
            draw_help(frame, area, theme);
        }
        return;
    }

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(12),
            Constraint::Length(8),
            Constraint::Min(6),
            Constraint::Length(2),
        ])
        .split(area);

    draw_header(frame, app, vertical[0], theme);

    let overview = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(vertical[1]);
    draw_cpu(frame, app, overview[0], theme);
    draw_memory(frame, app, overview[1], theme);

    let io = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(48), Constraint::Percentage(52)])
        .split(vertical[2]);
    draw_network(frame, app, io[0], theme);
    draw_disks(frame, app, io[1], theme);

    draw_processes(frame, app, vertical[3], theme);
    draw_footer(frame, app, vertical[4], theme);

    if app.show_help {
        draw_help(frame, area, theme);
    }
}

fn draw_compact(frame: &mut Frame<'_>, app: &mut App, area: Rect, theme: Theme) {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(8),
            Constraint::Length(6),
            Constraint::Min(3),
            Constraint::Length(2),
        ])
        .split(area);

    draw_header(frame, app, vertical[0], theme);
    draw_cpu(frame, app, vertical[1], theme);
    draw_memory(frame, app, vertical[2], theme);
    draw_processes(frame, app, vertical[3], theme);
    draw_footer(frame, app, vertical[4], theme);
}

fn draw_header(frame: &mut Frame<'_>, app: &App, area: Rect, theme: Theme) {
    let status = if app.paused {
        chip(" PAUSED ", theme.background, theme.warning)
    } else if app.has_sample {
        chip(" LIVE ", theme.background, theme.good)
    } else {
        chip(" WARMING UP ", theme.background, theme.warning)
    };

    let title = Line::from(vec![
        chip(" btop-win ", theme.background, theme.primary),
        Span::raw("  "),
        status,
        Span::raw("  "),
        Span::styled(
            app.snapshot.host_name.as_str(),
            Style::default()
                .fg(theme.foreground)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  •  ", Style::default().fg(theme.border)),
        Span::styled(
            app.snapshot.os_version.as_str(),
            Style::default().fg(theme.muted),
        ),
        Span::raw("  "),
        chip(
            format!(" {} ", theme.name),
            theme.background,
            theme.secondary,
        ),
    ]);

    let right = format!(
        "up {}  collect {:.1}ms  draw {:.1}ms  skip {}",
        format_uptime(app.snapshot.uptime_seconds),
        app.snapshot.diagnostics.collection_duration_ms,
        app.last_render_duration_ms,
        app.snapshot.diagnostics.skipped_samples,
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.primary))
        .style(
            Style::default()
                .fg(theme.foreground)
                .bg(theme.panel_background),
        );
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new(title).style(Style::default().bg(theme.panel_background)),
        inner,
    );

    let right_width = right.chars().count() as u16;
    // Keep a conservative left-side reserve so host, OS and theme labels cannot
    // be overwritten by diagnostics on narrow terminals.
    if inner.width > right_width.saturating_add(48) {
        let right_area = Rect::new(inner.x + inner.width - right_width, inner.y, right_width, 1);
        frame.render_widget(
            Paragraph::new(right)
                .alignment(Alignment::Right)
                .style(
                    Style::default()
                        .fg(theme.muted)
                        .bg(theme.panel_background),
                ),
            right_area,
        );
    }
}

fn draw_cpu(frame: &mut Frame<'_>, app: &App, area: Rect, theme: Theme) {
    let points = app.cpu_history.points();
    let x_max = app.cpu_history.len().saturating_sub(1).max(1) as f64;
    let cpu_color = theme.usage_color(app.snapshot.cpu.total_usage as f64);
    let datasets = vec![
        Dataset::default()
            .name("total")
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(cpu_color))
            .data(&points),
    ];

    let title = format!(
        " CPU  {:>5.1}%   {} MHz   {}/{} cores ",
        app.snapshot.cpu.total_usage,
        app.snapshot.cpu.frequency_mhz,
        app.snapshot.cpu.physical_cores.unwrap_or_default(),
        app.snapshot.cpu.logical_cores
    );

    let chart = Chart::new(datasets)
        .block(panel(theme, title, cpu_color))
        .x_axis(
            Axis::default()
                .bounds([0.0, x_max])
                .style(Style::default().fg(theme.muted)),
        )
        .y_axis(
            Axis::default()
                .bounds([0.0, 100.0])
                .style(Style::default().fg(theme.muted))
                .labels(["0%", "50%", "100%"]),
        );
    frame.render_widget(chart, area);
}

fn draw_memory(frame: &mut Frame<'_>, app: &App, area: Rect, theme: Theme) {
    let memory = &app.snapshot.memory;
    let block = panel(theme, " Memory ", theme.memory);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(2),
        ])
        .split(inner);

    let ram_percentage = memory.used_ratio() * 100.0;
    let ram_label = format!(
        "RAM  {} / {}  (free {})",
        format_bytes(memory.used_bytes),
        format_bytes(memory.total_bytes),
        format_bytes(memory.available_bytes)
    );
    frame.render_widget(
        Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .border_style(Style::default().fg(theme.border)),
            )
            .gauge_style(
                Style::default()
                    .fg(theme.usage_color(ram_percentage))
                    .bg(theme.background),
            )
            .percent(percent(ram_percentage))
            .label(ram_label)
            .use_unicode(true),
        rows[0],
    );

    let swap_percentage = memory.swap_used_ratio() * 100.0;
    let swap_label = if memory.swap_total_bytes == 0 {
        "Swap unavailable".to_owned()
    } else {
        format!(
            "Swap  {} / {}",
            format_bytes(memory.swap_used_bytes),
            format_bytes(memory.swap_total_bytes)
        )
    };
    frame.render_widget(
        Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .border_style(Style::default().fg(theme.border)),
            )
            .gauge_style(
                Style::default()
                    .fg(theme.swap)
                    .bg(theme.background),
            )
            .percent(percent(swap_percentage))
            .label(swap_label)
            .use_unicode(true),
        rows[1],
    );

    let core_spans = app
        .snapshot
        .cpu
        .per_core_usage
        .iter()
        .take(8)
        .enumerate()
        .flat_map(|(index, usage)| {
            [
                Span::styled(
                    format!("C{index}"),
                    Style::default()
                        .fg(theme.muted)
                        .bg(theme.panel_background),
                ),
                Span::styled(
                    format!(" {:>3.0}%  ", usage),
                    Style::default()
                        .fg(theme.usage_color(*usage as f64))
                        .bg(theme.panel_background),
                ),
            ]
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(Line::from(core_spans))
            .style(Style::default().bg(theme.panel_background))
            .wrap(Wrap { trim: true }),
        rows[2],
    );
}

fn draw_network(frame: &mut Frame<'_>, app: &App, area: Rect, theme: Theme) {
    let received = app.network_received_history.points();
    let transmitted = app.network_transmitted_history.points();
    let x_max = app
        .network_received_history
        .len()
        .max(app.network_transmitted_history.len())
        .saturating_sub(1)
        .max(1) as f64;
    let y_max = app
        .network_received_history
        .max()
        .max(app.network_transmitted_history.max())
        .max(1024.0);
    let network = app.network_view();

    let datasets = vec![
        Dataset::default()
            .name("down")
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(theme.download))
            .data(&received),
        Dataset::default()
            .name("up")
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(theme.upload))
            .data(&transmitted),
    ];

    let title = format!(
        " Network  [{} | {}]   ↓ {}   ↑ {}   total ↓ {} ↑ {} ",
        truncate(network.name, 22),
        network.adapter_count,
        format_rate(network.received_bytes_per_second),
        format_rate(network.transmitted_bytes_per_second),
        format_bytes(network.total_received_bytes),
        format_bytes(network.total_transmitted_bytes),
    );
    let chart = Chart::new(datasets)
        .block(panel(theme, title, theme.download))
        .x_axis(
            Axis::default()
                .bounds([0.0, x_max])
                .style(Style::default().fg(theme.muted)),
        )
        .y_axis(
            Axis::default()
                .bounds([0.0, y_max * 1.1])
                .style(Style::default().fg(theme.muted)),
        );
    frame.render_widget(chart, area);
}

fn draw_disks(frame: &mut Frame<'_>, app: &App, area: Rect, theme: Theme) {
    let header = Row::new(["Mount", "FS", "Used", "Read", "Write"])
        .style(
            Style::default()
                .fg(theme.primary)
                .bg(theme.panel_background)
                .add_modifier(Modifier::BOLD),
        )
        .bottom_margin(1);

    let rows = app.snapshot.disks.iter().map(|disk| {
        let used_percentage = disk.used_ratio() * 100.0;
        Row::new(vec![
            Cell::from(if disk.name.is_empty() {
                disk.mount_point.clone()
            } else {
                format!("{} {}", disk.mount_point, disk.name)
            }),
            Cell::from(format!("{}/{}", disk.file_system, disk.kind))
                .style(Style::default().fg(theme.muted)),
            Cell::from(format!(
                "{:>3}% {}",
                percent(used_percentage),
                format_bytes(disk.used_bytes())
            ))
            .style(Style::default().fg(theme.usage_color(used_percentage))),
            Cell::from(format_rate(disk.read_bytes_per_second))
                .style(Style::default().fg(theme.download)),
            Cell::from(format_rate(disk.written_bytes_per_second))
                .style(Style::default().fg(theme.upload)),
        ])
        .style(
            Style::default()
                .fg(theme.foreground)
                .bg(theme.panel_background),
        )
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Length(7),
            Constraint::Min(15),
            Constraint::Length(12),
            Constraint::Length(12),
        ],
    )
    .header(header)
    .column_spacing(1)
    .style(
        Style::default()
            .fg(theme.foreground)
            .bg(theme.panel_background),
    )
    .block(panel(theme, " Disks ", theme.secondary));
    frame.render_widget(table, area);
}

fn draw_processes(frame: &mut Frame<'_>, app: &mut App, area: Rect, theme: Theme) {
    let columns = process_columns(area.width);
    let header = Row::new(process_header(columns))
        .style(
            Style::default()
                .fg(theme.primary)
                .bg(theme.panel_background)
                .add_modifier(Modifier::BOLD),
        )
        .bottom_margin(1);

    let rows = app
        .visible_processes()
        .map(|process| {
            let mut cells = vec![
                Cell::from(process.pid.to_string()).style(Style::default().fg(theme.muted)),
                Cell::from(process.name.clone()),
                Cell::from(format!("{:>6.1}%", process.cpu_usage))
                    .style(Style::default().fg(theme.usage_color(process.cpu_usage as f64))),
                Cell::from(format_bytes(process.memory_bytes))
                    .style(Style::default().fg(theme.memory)),
            ];
            if matches!(columns, ProcessColumns::Io | ProcessColumns::Full) {
                cells.push(
                    Cell::from(format_rate(process.read_bytes_per_second))
                        .style(Style::default().fg(theme.download)),
                );
                cells.push(
                    Cell::from(format_rate(process.written_bytes_per_second))
                        .style(Style::default().fg(theme.upload)),
                );
            }
            if columns == ProcessColumns::Full {
                cells.push(
                    Cell::from(process.status.clone()).style(Style::default().fg(theme.muted)),
                );
            }
            Row::new(cells).style(
                Style::default()
                    .fg(theme.foreground)
                    .bg(theme.panel_background),
            )
        })
        .collect::<Vec<_>>();

    let selected = app
        .selected_process()
        .map(|process| {
            if process.executable.is_empty() {
                process.name.clone()
            } else {
                process.executable.clone()
            }
        })
        .unwrap_or_else(|| "no process selected".to_owned());
    let visible_count = rows.len();
    let filter = if app.filter_mode {
        format!("filter /{}_", app.process_filter)
    } else if app.process_filter.is_empty() {
        "filter all".to_owned()
    } else {
        format!("filter {}", app.process_filter)
    };
    let title = format!(
        " Processes  {}/{}   sort {} {}   {}   •   {} ",
        visible_count,
        app.snapshot.processes.len(),
        app.process_sort.label(),
        app.sort_direction.label(),
        filter,
        truncate(&selected, area.width.saturating_sub(62) as usize)
    );

    let table = Table::new(rows, process_widths(columns))
        .header(header)
        .column_spacing(1)
        .style(
            Style::default()
                .fg(theme.foreground)
                .bg(theme.panel_background),
        )
        .row_highlight_style(
            Style::default()
                .fg(theme.selected_foreground)
                .bg(theme.selected_background)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ")
        .block(panel(theme, title, theme.secondary));

    frame.render_stateful_widget(table, area, &mut app.process_table_state);
}

fn process_columns(width: u16) -> ProcessColumns {
    if width >= 124 {
        ProcessColumns::Full
    } else if width >= 96 {
        ProcessColumns::Io
    } else {
        ProcessColumns::Compact
    }
}

fn process_header(columns: ProcessColumns) -> Vec<&'static str> {
    let mut header = vec!["PID", "Process", "CPU", "Memory"];
    if matches!(columns, ProcessColumns::Io | ProcessColumns::Full) {
        header.extend(["Read", "Write"]);
    }
    if columns == ProcessColumns::Full {
        header.push("State");
    }
    header
}

fn process_widths(columns: ProcessColumns) -> Vec<Constraint> {
    let mut widths = vec![
        Constraint::Length(8),
        Constraint::Min(20),
        Constraint::Length(8),
        Constraint::Length(11),
    ];
    if matches!(columns, ProcessColumns::Io | ProcessColumns::Full) {
        widths.extend([Constraint::Length(12), Constraint::Length(12)]);
    }
    if columns == ProcessColumns::Full {
        widths.push(Constraint::Length(11));
    }
    widths
}

fn draw_footer(frame: &mut Frame<'_>, app: &App, area: Rect, theme: Theme) {
    let line = if app.show_help {
        Line::from(vec![
            Span::styled(" ? ", key_style(theme)),
            Span::styled(" close help ", label_style(theme)),
        ])
    } else if app.filter_mode {
        let mut spans = Vec::new();
        push_hint(&mut spans, theme, "type", "filter");
        push_hint(&mut spans, theme, "Enter", "keep");
        push_hint(&mut spans, theme, "Esc", "clear");
        push_hint(&mut spans, theme, "⌫", "delete");
        Line::from(spans)
    } else {
        let mut spans = Vec::new();
        push_hint(&mut spans, theme, "q", "quit");
        push_hint(&mut spans, theme, "/", "filter");
        push_hint(&mut spans, theme, "[ ]", "adapter");
        push_hint(&mut spans, theme, "c/m/n", "sort");
        push_hint(&mut spans, theme, "↑↓", "select");
        push_hint(&mut spans, theme, "?", "help");
        Line::from(spans)
    };
    frame.render_widget(
        Paragraph::new(line)
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(theme.muted)
                    .bg(theme.background),
            ),
        area,
    );
}

fn draw_help(frame: &mut Frame<'_>, area: Rect, theme: Theme) {
    let popup = centered_rect(76, 88, area);
    frame.render_widget(Clear, popup);
    let text = Text::from(vec![
        Line::from(Span::styled(
            "Keyboard",
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        help_line(theme, "q / Esc / Ctrl+C", "quit; Esc clears an active filter first"),
        help_line(theme, "/", "edit process filter"),
        help_line(theme, "Enter", "keep filter and leave edit mode"),
        help_line(theme, "Backspace / Esc", "edit or clear the filter"),
        help_line(theme, "[ / ]", "previous or next network adapter"),
        help_line(theme, "a", "return to aggregate all-adapter network view"),
        help_line(theme, "c/m/n/d/w", "sort CPU/memory/name/read/write"),
        help_line(theme, "o", "toggle ascending/descending order"),
        help_line(theme, "s", "cycle process sort column"),
        help_line(theme, "p / Space", "pause or resume sampling"),
        help_line(theme, "Up/Down or j/k", "select process"),
        help_line(theme, "PageUp/PageDown", "move ten rows"),
        help_line(theme, "Home/End", "first or last visible process"),
        help_line(theme, "r", "reset history charts"),
        help_line(theme, "?", "close this help"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Theme  ", Style::default().fg(theme.secondary)),
            Span::styled(
                format!("{}  (start with --theme btop|dracula|nord|mono)", theme.name),
                Style::default().fg(theme.muted),
            ),
        ]),
        Line::from(Span::styled(
            "Process columns adapt to terminal width. Network histories reset when the selected adapter changes.",
            Style::default().fg(theme.muted),
        )),
    ]);
    frame.render_widget(
        Paragraph::new(text)
            .block(panel(theme, " Help ", theme.primary))
            .style(
                Style::default()
                    .fg(theme.foreground)
                    .bg(theme.panel_background),
            )
            .wrap(Wrap { trim: false }),
        popup,
    );
}

fn draw_too_small(frame: &mut Frame<'_>, area: Rect, theme: Theme) {
    frame.render_widget(
        Paragraph::new(format!(
            "btop-win needs at least 72×20 cells.\nCurrent terminal: {}×{}\n\nResize the terminal or press q to quit.",
            area.width, area.height
        ))
        .alignment(Alignment::Center)
        .style(
            Style::default()
                .fg(theme.foreground)
                .bg(theme.panel_background),
        )
        .block(panel(
            theme,
            " Terminal too small ",
            theme.warning,
        )),
        centered_rect(60, 40, area),
    );
}

fn panel<'a>(theme: Theme, title: impl Into<Line<'a>>, accent: Color) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(accent))
        .title(title)
        .title_style(
            Style::default()
                .fg(theme.title)
                .bg(theme.panel_background)
                .add_modifier(Modifier::BOLD),
        )
        .style(
            Style::default()
                .fg(theme.foreground)
                .bg(theme.panel_background),
        )
}

fn chip(text: impl Into<String>, foreground: Color, background: Color) -> Span<'static> {
    Span::styled(
        text.into(),
        Style::default()
            .fg(foreground)
            .bg(background)
            .add_modifier(Modifier::BOLD),
    )
}

fn push_hint(spans: &mut Vec<Span<'static>>, theme: Theme, key: &str, label: &str) {
    if !spans.is_empty() {
        spans.push(Span::raw("  "));
    }
    spans.push(Span::styled(format!(" {key} "), key_style(theme)));
    spans.push(Span::styled(format!(" {label} "), label_style(theme)));
}

fn key_style(theme: Theme) -> Style {
    Style::default()
        .fg(theme.background)
        .bg(theme.primary)
        .add_modifier(Modifier::BOLD)
}

fn label_style(theme: Theme) -> Style {
    Style::default().fg(theme.muted).bg(theme.background)
}

fn help_line(theme: Theme, key: &str, description: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{key:<21}"),
            Style::default()
                .fg(theme.secondary)
                .bg(theme.panel_background),
        ),
        Span::styled(
            description.to_owned(),
            Style::default()
                .fg(theme.foreground)
                .bg(theme.panel_background),
        ),
    ])
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

fn truncate(value: &str, max_chars: usize) -> String {
    if max_chars < 2 {
        return String::new();
    }
    let count = value.chars().count();
    if count <= max_chars {
        value.to_owned()
    } else {
        let mut output = value.chars().take(max_chars - 1).collect::<String>();
        output.push('…');
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_unicode_by_characters() {
        assert_eq!(truncate("abcdef", 4), "abc…");
        assert_eq!(truncate("中文测试", 3), "中文…");
    }

    #[test]
    fn process_columns_adapt_to_terminal_width() {
        assert_eq!(process_columns(72), ProcessColumns::Compact);
        assert_eq!(process_columns(96), ProcessColumns::Io);
        assert_eq!(process_columns(124), ProcessColumns::Full);
    }

    #[test]
    fn configured_theme_is_visible_in_header_palette() {
        assert_eq!(crate::theme::ThemeName::Nord.palette().name, "nord");
    }
}
