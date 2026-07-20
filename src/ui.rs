use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols::Marker,
    text::{Line, Span, Text},
    widgets::{
        Axis, Block, Borders, Cell, Chart, Clear, Dataset, Gauge, GraphType, Paragraph, Row, Table,
        Wrap,
    },
};

use crate::{
    app::App,
    utils::{format_bytes, format_rate, format_uptime, percent},
};

const BORDER: Color = Color::DarkGray;
const PRIMARY: Color = Color::Cyan;
const SECONDARY: Color = Color::Magenta;
const GOOD: Color = Color::Green;
const WARN: Color = Color::Yellow;

pub fn draw(frame: &mut Frame<'_>, app: &mut App) {
    let area = frame.area();
    if area.width < 72 || area.height < 20 {
        draw_too_small(frame, area);
        return;
    }

    if area.height < 30 {
        draw_compact(frame, app, area);
        if app.show_help {
            draw_help(frame, area);
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

    draw_header(frame, app, vertical[0]);

    let overview = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(vertical[1]);
    draw_cpu(frame, app, overview[0]);
    draw_memory(frame, app, overview[1]);

    let io = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(48), Constraint::Percentage(52)])
        .split(vertical[2]);
    draw_network(frame, app, io[0]);
    draw_disks(frame, app, io[1]);

    draw_processes(frame, app, vertical[3]);
    draw_footer(frame, app, vertical[4]);

    if app.show_help {
        draw_help(frame, area);
    }
}

fn draw_compact(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
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

    draw_header(frame, app, vertical[0]);
    draw_cpu(frame, app, vertical[1]);
    draw_memory(frame, app, vertical[2]);
    draw_processes(frame, app, vertical[3]);
    draw_footer(frame, app, vertical[4]);
}

fn draw_header(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let status = if app.paused {
        Span::styled(
            " PAUSED ",
            Style::default().fg(Color::Black).bg(WARN).bold(),
        )
    } else if app.has_sample {
        Span::styled(" LIVE ", Style::default().fg(Color::Black).bg(GOOD).bold())
    } else {
        Span::styled(
            " WARMING UP ",
            Style::default().fg(Color::Black).bg(WARN).bold(),
        )
    };

    let title = Line::from(vec![
        Span::styled(
            " btop-win ",
            Style::default()
                .fg(Color::Black)
                .bg(PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        status,
        Span::raw("  "),
        Span::styled(
            app.snapshot.host_name.as_str(),
            Style::default().fg(Color::White).bold(),
        ),
        Span::raw("  "),
        Span::styled(
            app.snapshot.os_version.as_str(),
            Style::default().fg(Color::Gray),
        ),
    ]);

    let right = format!(
        "uptime {} | collect {:.1}ms draw {:.1}ms skip {}",
        format_uptime(app.snapshot.uptime_seconds),
        app.snapshot.diagnostics.collection_duration_ms,
        app.last_render_duration_ms,
        app.snapshot.diagnostics.skipped_samples,
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(title), inner);

    let right_width = right.chars().count() as u16;
    if inner.width > right_width + 2 {
        let right_area = Rect::new(inner.x + inner.width - right_width, inner.y, right_width, 1);
        frame.render_widget(
            Paragraph::new(right)
                .alignment(Alignment::Right)
                .style(Style::default().fg(Color::Gray)),
            right_area,
        );
    }
}

fn draw_cpu(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let points = app.cpu_history.points();
    let x_max = app.cpu_history.len().saturating_sub(1).max(1) as f64;
    let datasets = vec![
        Dataset::default()
            .name("total")
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(PRIMARY))
            .data(&points),
    ];

    let title = format!(
        " CPU {:>5.1}%  {} MHz  {}/{} cores ",
        app.snapshot.cpu.total_usage,
        app.snapshot.cpu.frequency_mhz,
        app.snapshot.cpu.physical_cores.unwrap_or_default(),
        app.snapshot.cpu.logical_cores
    );

    let chart = Chart::new(datasets)
        .block(panel(title))
        .x_axis(Axis::default().bounds([0.0, x_max]))
        .y_axis(
            Axis::default()
                .bounds([0.0, 100.0])
                .labels(["0%", "50%", "100%"]),
        );
    frame.render_widget(chart, area);
}

fn draw_memory(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let memory = &app.snapshot.memory;
    let block = panel(" Memory ");
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

    let ram_label = format!(
        "RAM  {} / {}  (available {})",
        format_bytes(memory.used_bytes),
        format_bytes(memory.total_bytes),
        format_bytes(memory.available_bytes)
    );
    frame.render_widget(
        Gauge::default()
            .block(Block::default().borders(Borders::BOTTOM))
            .gauge_style(Style::default().fg(PRIMARY).bg(Color::Black))
            .percent(percent(memory.used_ratio() * 100.0))
            .label(ram_label)
            .use_unicode(true),
        rows[0],
    );

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
            .block(Block::default().borders(Borders::BOTTOM))
            .gauge_style(Style::default().fg(SECONDARY).bg(Color::Black))
            .percent(percent(memory.swap_used_ratio() * 100.0))
            .label(swap_label)
            .use_unicode(true),
        rows[1],
    );

    let core_text = app
        .snapshot
        .cpu
        .per_core_usage
        .iter()
        .take(8)
        .enumerate()
        .map(|(index, usage)| format!("C{index}: {:>4.0}%", usage))
        .collect::<Vec<_>>()
        .join("  ");
    frame.render_widget(
        Paragraph::new(core_text)
            .style(Style::default().fg(Color::Gray))
            .wrap(Wrap { trim: true }),
        rows[2],
    );
}

fn draw_network(frame: &mut Frame<'_>, app: &App, area: Rect) {
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

    let datasets = vec![
        Dataset::default()
            .name("down")
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(GOOD))
            .data(&received),
        Dataset::default()
            .name("up")
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(SECONDARY))
            .data(&transmitted),
    ];

    let title = format!(
        " Network ({})  ↓ {}  ↑ {}  total ↓ {} ↑ {} ",
        app.snapshot.network.interface_count,
        format_rate(app.snapshot.network.received_bytes_per_second),
        format_rate(app.snapshot.network.transmitted_bytes_per_second),
        format_bytes(app.snapshot.network.total_received_bytes),
        format_bytes(app.snapshot.network.total_transmitted_bytes),
    );
    let chart = Chart::new(datasets)
        .block(panel(title))
        .x_axis(Axis::default().bounds([0.0, x_max]))
        .y_axis(Axis::default().bounds([0.0, y_max * 1.1]));
    frame.render_widget(chart, area);
}

fn draw_disks(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let header = Row::new(["Mount", "FS", "Used", "Read", "Write"])
        .style(Style::default().fg(PRIMARY).bold())
        .bottom_margin(1);

    let rows = app.snapshot.disks.iter().map(|disk| {
        Row::new(vec![
            Cell::from(if disk.name.is_empty() {
                disk.mount_point.clone()
            } else {
                format!("{} {}", disk.mount_point, disk.name)
            }),
            Cell::from(format!("{}/{}", disk.file_system, disk.kind)),
            Cell::from(format!(
                "{:>3}% {}",
                percent(disk.used_ratio() * 100.0),
                format_bytes(disk.used_bytes())
            )),
            Cell::from(format_rate(disk.read_bytes_per_second)),
            Cell::from(format_rate(disk.written_bytes_per_second)),
        ])
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
    .block(panel(" Disks "));
    frame.render_widget(table, area);
}

fn draw_processes(frame: &mut Frame<'_>, app: &mut App, area: Rect) {
    let header = Row::new(["PID", "Process", "CPU", "Memory", "Read", "Write", "State"])
        .style(Style::default().fg(PRIMARY).bold())
        .bottom_margin(1);

    let rows = app
        .visible_processes()
        .map(|process| {
            Row::new(vec![
                Cell::from(process.pid.to_string()),
                Cell::from(process.name.clone()),
                Cell::from(format!("{:>6.1}%", process.cpu_usage)),
                Cell::from(format_bytes(process.memory_bytes)),
                Cell::from(format_rate(process.read_bytes_per_second)),
                Cell::from(format_rate(process.written_bytes_per_second)),
                Cell::from(process.status.clone()),
            ])
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
        format!("filter: /{}_", app.process_filter)
    } else if app.process_filter.is_empty() {
        "filter: all".to_owned()
    } else {
        format!("filter: {}", app.process_filter)
    };
    let title = format!(
        " Processes {}/{}  sort: {} {}  {}  |  {} ",
        visible_count,
        app.snapshot.processes.len(),
        app.process_sort.label(),
        app.sort_direction.label(),
        filter,
        truncate(&selected, area.width.saturating_sub(62) as usize)
    );

    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Min(20),
            Constraint::Length(8),
            Constraint::Length(11),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(11),
        ],
    )
    .header(header)
    .column_spacing(1)
    .row_highlight_style(Style::default().fg(Color::Black).bg(PRIMARY).bold())
    .highlight_symbol("▶ ")
    .block(panel(title));

    frame.render_stateful_widget(table, area, &mut app.process_table_state);
}

fn draw_footer(frame: &mut Frame<'_>, app: &App, area: Rect) {
    let text = if app.show_help {
        "? close help"
    } else if app.filter_mode {
        "type to filter  Enter keep  Esc clear  Backspace delete"
    } else {
        "q quit  / filter  c/m/n/d/w sort  o order  ↑↓/jk select  p pause  ? help"
    };
    frame.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Gray)),
        area,
    );
}

fn draw_help(frame: &mut Frame<'_>, area: Rect) {
    let popup = centered_rect(72, 82, area);
    frame.render_widget(Clear, popup);
    let text = Text::from(vec![
        Line::from(Span::styled(
            "Keyboard",
            Style::default().fg(PRIMARY).bold(),
        )),
        Line::from(""),
        Line::from("q / Esc / Ctrl+C    quit; Esc clears an active filter first"),
        Line::from("/                    edit process filter"),
        Line::from("Enter                keep filter and leave edit mode"),
        Line::from("Backspace / Esc      edit or clear the filter"),
        Line::from("c/m/n/d/w            sort CPU/memory/name/read/write"),
        Line::from("o                    toggle ascending/descending order"),
        Line::from("s                    cycle process sort column"),
        Line::from("p / Space            pause or resume sampling"),
        Line::from("Up/Down or j/k       select process"),
        Line::from("PageUp/PageDown      move ten rows"),
        Line::from("Home/End             first or last visible process"),
        Line::from("r                    reset history charts"),
        Line::from("?                    close this help"),
        Line::from(""),
        Line::from(Span::styled(
            "Header diagnostics show collector duration, previous render duration and the cumulative number of snapshots dropped while the UI was busy.",
            Style::default().fg(Color::Gray),
        )),
    ]);
    frame.render_widget(
        Paragraph::new(text)
            .block(panel(" Help "))
            .wrap(Wrap { trim: false }),
        popup,
    );
}

fn draw_too_small(frame: &mut Frame<'_>, area: Rect) {
    frame.render_widget(
        Paragraph::new(format!(
            "btop-win needs at least 72×20 cells.\nCurrent terminal: {}×{}\n\nResize the terminal or press q to quit.",
            area.width, area.height
        ))
        .alignment(Alignment::Center)
        .block(panel(" Terminal too small ")),
        centered_rect(60, 40, area),
    );
}

fn panel<'a>(title: impl Into<Line<'a>>) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(border_style())
        .title(title)
        .title_style(Style::default().fg(Color::White).bold())
}

fn border_style() -> Style {
    Style::default().fg(BORDER)
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
}
