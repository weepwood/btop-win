use std::{cmp::Ordering, collections::VecDeque, time::Duration};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::widgets::TableState;

use crate::model::{ProcessSnapshot, Snapshot};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProcessSort {
    #[default]
    Cpu,
    Memory,
    Read,
    Write,
    Name,
}

impl ProcessSort {
    pub fn next(self) -> Self {
        match self {
            Self::Cpu => Self::Memory,
            Self::Memory => Self::Read,
            Self::Read => Self::Write,
            Self::Write => Self::Name,
            Self::Name => Self::Cpu,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Cpu => "CPU",
            Self::Memory => "Memory",
            Self::Read => "Read",
            Self::Write => "Write",
            Self::Name => "Name",
        }
    }

    pub fn default_direction(self) -> SortDirection {
        match self {
            Self::Name => SortDirection::Ascending,
            _ => SortDirection::Descending,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    #[default]
    Descending,
}

impl SortDirection {
    pub fn toggle(self) -> Self {
        match self {
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Ascending => "asc",
            Self::Descending => "desc",
        }
    }

    fn apply(self, ordering: Ordering) -> Ordering {
        match self {
            Self::Ascending => ordering,
            Self::Descending => ordering.reverse(),
        }
    }
}

#[derive(Debug)]
pub struct History {
    values: VecDeque<f64>,
    capacity: usize,
}

impl History {
    pub fn new(capacity: usize) -> Self {
        Self {
            values: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, value: f64) {
        if self.values.len() == self.capacity {
            self.values.pop_front();
        }
        self.values.push_back(value);
    }

    pub fn clear(&mut self) {
        self.values.clear();
    }

    pub fn points(&self) -> Vec<(f64, f64)> {
        self.values
            .iter()
            .enumerate()
            .map(|(index, value)| (index as f64, *value))
            .collect()
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn max(&self) -> f64 {
        self.values.iter().copied().fold(0.0, f64::max)
    }
}

#[derive(Debug)]
pub struct App {
    pub snapshot: Snapshot,
    pub cpu_history: History,
    pub memory_history: History,
    pub network_received_history: History,
    pub network_transmitted_history: History,
    pub process_table_state: TableState,
    pub process_sort: ProcessSort,
    pub sort_direction: SortDirection,
    pub process_filter: String,
    pub filter_mode: bool,
    pub paused: bool,
    pub show_help: bool,
    pub has_sample: bool,
    pub last_render_duration_ms: f64,
    pub render_count: u64,
}

impl App {
    pub fn new(history_capacity: usize) -> Self {
        Self {
            snapshot: Snapshot::default(),
            cpu_history: History::new(history_capacity),
            memory_history: History::new(history_capacity),
            network_received_history: History::new(history_capacity),
            network_transmitted_history: History::new(history_capacity),
            process_table_state: TableState::default().with_selected(0),
            process_sort: ProcessSort::default(),
            sort_direction: SortDirection::default(),
            process_filter: String::new(),
            filter_mode: false,
            paused: false,
            show_help: false,
            has_sample: false,
            last_render_duration_ms: 0.0,
            render_count: 0,
        }
    }

    /// Applies a new collector snapshot and returns whether the UI changed.
    pub fn apply_snapshot(&mut self, snapshot: Snapshot) -> bool {
        if self.paused {
            return false;
        }

        let selected_pid = self.selected_process().map(|process| process.pid);
        self.snapshot = snapshot;
        self.sort_processes();
        self.restore_process_selection(selected_pid);
        self.cpu_history.push(self.snapshot.cpu.total_usage);
        self.memory_history
            .push(self.snapshot.memory.used_ratio() * 100.0);
        self.network_received_history
            .push(self.snapshot.network.received_bytes_per_second);
        self.network_transmitted_history
            .push(self.snapshot.network.transmitted_bytes_per_second);
        self.has_sample = true;
        true
    }

    pub fn record_render(&mut self, duration: Duration) {
        self.last_render_duration_ms = duration.as_secs_f64() * 1_000.0;
        self.render_count = self.render_count.saturating_add(1);
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
            return true;
        }

        if self.filter_mode {
            self.handle_filter_key(key);
            return false;
        }

        match key.code {
            KeyCode::Char('q' | 'Q') => return true,
            KeyCode::Esc if self.process_filter.is_empty() => return true,
            KeyCode::Esc => self.clear_filter(),
            KeyCode::Char('?') => self.show_help = !self.show_help,
            KeyCode::Char('/') => self.filter_mode = true,
            KeyCode::Char('p' | 'P') | KeyCode::Char(' ') => self.paused = !self.paused,
            KeyCode::Char('s' | 'S') => self.cycle_sort(),
            KeyCode::Char('o' | 'O') => self.toggle_sort_direction(),
            KeyCode::Char('c' | 'C') => self.select_sort(ProcessSort::Cpu),
            KeyCode::Char('m' | 'M') => self.select_sort(ProcessSort::Memory),
            KeyCode::Char('d' | 'D') => self.select_sort(ProcessSort::Read),
            KeyCode::Char('w' | 'W') => self.select_sort(ProcessSort::Write),
            KeyCode::Char('n' | 'N') => self.select_sort(ProcessSort::Name),
            KeyCode::Char('r' | 'R') => self.clear_histories(),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1),
            KeyCode::PageDown => self.move_selection(10),
            KeyCode::PageUp => self.move_selection(-10),
            KeyCode::Home => self.process_table_state.select(Some(0)),
            KeyCode::End if self.visible_process_count() > 0 => {
                self.process_table_state
                    .select(Some(self.visible_process_count() - 1));
            }
            _ => {}
        }

        false
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent) {
        match mouse.kind {
            MouseEventKind::ScrollDown => self.move_selection(3),
            MouseEventKind::ScrollUp => self.move_selection(-3),
            _ => {}
        }
    }

    pub fn visible_processes(&self) -> impl Iterator<Item = &ProcessSnapshot> {
        let query = self.process_filter.to_lowercase();
        self.snapshot
            .processes
            .iter()
            .filter(move |process| process_matches_filter(process, &query))
    }

    pub fn visible_process_count(&self) -> usize {
        self.visible_processes().count()
    }

    pub fn selected_process(&self) -> Option<&ProcessSnapshot> {
        self.process_table_state
            .selected()
            .and_then(|index| self.visible_processes().nth(index))
    }

    fn handle_filter_key(&mut self, key: KeyEvent) {
        let selected_pid = self.selected_process().map(|process| process.pid);
        match key.code {
            KeyCode::Esc => {
                self.process_filter.clear();
                self.filter_mode = false;
            }
            KeyCode::Enter => self.filter_mode = false,
            KeyCode::Backspace => {
                self.process_filter.pop();
            }
            KeyCode::Char(value)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.process_filter.push(value);
            }
            _ => {}
        }
        self.restore_process_selection(selected_pid);
    }

    fn clear_filter(&mut self) {
        let selected_pid = self.selected_process().map(|process| process.pid);
        self.process_filter.clear();
        self.filter_mode = false;
        self.restore_process_selection(selected_pid);
    }

    fn clear_histories(&mut self) {
        self.cpu_history.clear();
        self.memory_history.clear();
        self.network_received_history.clear();
        self.network_transmitted_history.clear();
    }

    fn move_selection(&mut self, delta: isize) {
        let count = self.visible_process_count();
        if count == 0 {
            self.process_table_state.select(None);
            return;
        }

        let current = self.process_table_state.selected().unwrap_or_default();
        let next = current.saturating_add_signed(delta).min(count - 1);
        self.process_table_state.select(Some(next));
    }

    fn cycle_sort(&mut self) {
        let selected_pid = self.selected_process().map(|process| process.pid);
        self.process_sort = self.process_sort.next();
        self.sort_direction = self.process_sort.default_direction();
        self.sort_processes();
        self.restore_process_selection(selected_pid);
    }

    fn select_sort(&mut self, sort: ProcessSort) {
        let selected_pid = self.selected_process().map(|process| process.pid);
        if self.process_sort == sort {
            self.sort_direction = self.sort_direction.toggle();
        } else {
            self.process_sort = sort;
            self.sort_direction = sort.default_direction();
        }
        self.sort_processes();
        self.restore_process_selection(selected_pid);
    }

    fn toggle_sort_direction(&mut self) {
        let selected_pid = self.selected_process().map(|process| process.pid);
        self.sort_direction = self.sort_direction.toggle();
        self.sort_processes();
        self.restore_process_selection(selected_pid);
    }

    fn restore_process_selection(&mut self, selected_pid: Option<u32>) {
        let selection = selected_pid.and_then(|pid| {
            self.visible_processes()
                .position(|process| process.pid == pid)
        });

        if let Some(index) = selection {
            self.process_table_state.select(Some(index));
        } else if self.visible_process_count() == 0 {
            self.process_table_state.select(None);
        } else {
            self.process_table_state.select(Some(0));
        }
    }

    fn sort_processes(&mut self) {
        let sort = self.process_sort;
        let direction = self.sort_direction;
        match sort {
            ProcessSort::Cpu => self.snapshot.processes.sort_by(|left, right| {
                direction
                    .apply(ascending_f64(left.cpu_usage, right.cpu_usage))
                    .then_with(|| left.pid.cmp(&right.pid))
            }),
            ProcessSort::Memory => self.snapshot.processes.sort_by(|left, right| {
                direction
                    .apply(left.memory_bytes.cmp(&right.memory_bytes))
                    .then_with(|| left.pid.cmp(&right.pid))
            }),
            ProcessSort::Read => self.snapshot.processes.sort_by(|left, right| {
                direction
                    .apply(ascending_f64(
                        left.read_bytes_per_second,
                        right.read_bytes_per_second,
                    ))
                    .then_with(|| left.pid.cmp(&right.pid))
            }),
            ProcessSort::Write => self.snapshot.processes.sort_by(|left, right| {
                direction
                    .apply(ascending_f64(
                        left.written_bytes_per_second,
                        right.written_bytes_per_second,
                    ))
                    .then_with(|| left.pid.cmp(&right.pid))
            }),
            ProcessSort::Name => {
                self.snapshot
                    .processes
                    .sort_by_cached_key(|process| (process.name.to_lowercase(), process.pid));
                if direction == SortDirection::Descending {
                    self.snapshot.processes.reverse();
                }
            }
        }
    }
}

fn process_matches_filter(process: &ProcessSnapshot, query: &str) -> bool {
    query.is_empty()
        || process.pid.to_string().contains(query)
        || process.name.to_lowercase().contains(query)
        || process.executable.to_lowercase().contains(query)
}

fn ascending_f64(left: f64, right: f64) -> Ordering {
    left.partial_cmp(&right).unwrap_or(Ordering::Equal)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn process(pid: u32, name: &str, cpu_usage: f64) -> ProcessSnapshot {
        ProcessSnapshot {
            pid,
            cpu_usage,
            name: name.to_owned(),
            executable: format!("C:/Apps/{name}.exe"),
            ..ProcessSnapshot::default()
        }
    }

    fn snapshot(processes: Vec<ProcessSnapshot>) -> Snapshot {
        Snapshot {
            processes,
            ..Snapshot::default()
        }
    }

    #[test]
    fn history_keeps_capacity() {
        let mut history = History::new(2);
        history.push(1.0);
        history.push(2.0);
        history.push(3.0);
        assert_eq!(history.points(), vec![(0.0, 2.0), (1.0, 3.0)]);
    }

    #[test]
    fn sort_cycles_back_to_cpu() {
        let mut sort = ProcessSort::Cpu;
        for _ in 0..5 {
            sort = sort.next();
        }
        assert_eq!(sort, ProcessSort::Cpu);
    }

    #[test]
    fn selected_process_is_preserved_when_sort_order_changes() {
        let mut app = App::new(30);
        assert!(app.apply_snapshot(snapshot(vec![
            process(1, "one", 10.0),
            process(2, "two", 20.0),
        ])));

        let selected_index = app
            .snapshot
            .processes
            .iter()
            .position(|process| process.pid == 1)
            .unwrap();
        app.process_table_state.select(Some(selected_index));

        assert!(app.apply_snapshot(snapshot(vec![
            process(1, "one", 30.0),
            process(2, "two", 5.0),
        ])));
        assert_eq!(app.selected_process().map(|process| process.pid), Some(1));
    }

    #[test]
    fn paused_app_does_not_apply_snapshots() {
        let mut app = App::new(30);
        app.paused = true;

        assert!(!app.apply_snapshot(snapshot(vec![process(1, "one", 10.0)])));
        assert!(!app.has_sample);
        assert!(app.snapshot.processes.is_empty());
    }

    #[test]
    fn process_filter_matches_name_executable_and_pid() {
        let mut app = App::new(30);
        assert!(app.apply_snapshot(snapshot(vec![
            process(101, "AlphaWorker", 10.0),
            process(202, "BetaAgent", 20.0),
        ])));

        app.process_filter = "worker".to_owned();
        assert_eq!(app.visible_process_count(), 1);
        assert_eq!(
            app.visible_processes().next().map(|process| process.pid),
            Some(101)
        );

        app.process_filter = "202".to_owned();
        assert_eq!(
            app.visible_processes().next().map(|process| process.pid),
            Some(202)
        );
    }

    #[test]
    fn ascending_cpu_sort_places_lowest_usage_first() {
        let mut app = App::new(30);
        app.sort_direction = SortDirection::Ascending;
        assert!(app.apply_snapshot(snapshot(vec![
            process(1, "one", 30.0),
            process(2, "two", 5.0),
        ])));

        assert_eq!(
            app.snapshot.processes.first().map(|process| process.pid),
            Some(2)
        );
    }
}
