use std::{cmp::Ordering, collections::VecDeque};

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
    pub paused: bool,
    pub show_help: bool,
    pub has_sample: bool,
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
            paused: false,
            show_help: false,
            has_sample: false,
        }
    }

    pub fn apply_snapshot(&mut self, snapshot: Snapshot) {
        if self.paused {
            return;
        }

        self.snapshot = snapshot;
        self.sort_processes();
        self.cpu_history.push(self.snapshot.cpu.total_usage);
        self.memory_history.push(self.snapshot.memory.used_ratio() * 100.0);
        self.network_received_history.push(self.snapshot.network.received_bytes_per_second);
        self.network_transmitted_history.push(self.snapshot.network.transmitted_bytes_per_second);
        self.has_sample = true;
        self.clamp_process_selection();
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
            return true;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return true,
            KeyCode::Char('?') => self.show_help = !self.show_help,
            KeyCode::Char('p') | KeyCode::Char(' ') => self.paused = !self.paused,
            KeyCode::Char('s') => {
                self.process_sort = self.process_sort.next();
                self.sort_processes();
                self.process_table_state.select(Some(0));
            }
            KeyCode::Char('r') => self.clear_histories(),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1),
            KeyCode::PageDown => self.move_selection(10),
            KeyCode::PageUp => self.move_selection(-10),
            KeyCode::Home => self.process_table_state.select(Some(0)),
            KeyCode::End => {
                if !self.snapshot.processes.is_empty() {
                    self.process_table_state.select(Some(self.snapshot.processes.len() - 1));
                }
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

    pub fn selected_process(&self) -> Option<&ProcessSnapshot> {
        self.process_table_state.selected().and_then(|index| self.snapshot.processes.get(index))
    }

    fn clear_histories(&mut self) {
        self.cpu_history.clear();
        self.memory_history.clear();
        self.network_received_history.clear();
        self.network_transmitted_history.clear();
    }

    fn move_selection(&mut self, delta: isize) {
        let count = self.snapshot.processes.len();
        if count == 0 {
            self.process_table_state.select(None);
            return;
        }

        let current = self.process_table_state.selected().unwrap_or_default();
        let next = current.saturating_add_signed(delta).min(count - 1);
        self.process_table_state.select(Some(next));
    }

    fn clamp_process_selection(&mut self) {
        let count = self.snapshot.processes.len();
        match (count, self.process_table_state.selected()) {
            (0, _) => self.process_table_state.select(None),
            (_, None) => self.process_table_state.select(Some(0)),
            (_, Some(index)) if index >= count => self.process_table_state.select(Some(count - 1)),
            _ => {}
        }
    }

    fn sort_processes(&mut self) {
        match self.process_sort {
            ProcessSort::Cpu => self.snapshot.processes.sort_by(|left, right| {
                descending_f64(left.cpu_usage, right.cpu_usage).then_with(|| left.pid.cmp(&right.pid))
            }),
            ProcessSort::Memory => self.snapshot.processes.sort_by(|left, right| {
                right.memory_bytes.cmp(&left.memory_bytes).then_with(|| left.pid.cmp(&right.pid))
            }),
            ProcessSort::Read => self.snapshot.processes.sort_by(|left, right| {
                descending_f64(left.read_bytes_per_second, right.read_bytes_per_second)
                    .then_with(|| left.pid.cmp(&right.pid))
            }),
            ProcessSort::Write => self.snapshot.processes.sort_by(|left, right| {
                descending_f64(left.written_bytes_per_second, right.written_bytes_per_second)
                    .then_with(|| left.pid.cmp(&right.pid))
            }),
            ProcessSort::Name => self.snapshot.processes.sort_by(|left, right| {
                left.name.to_lowercase().cmp(&right.name.to_lowercase()).then_with(|| left.pid.cmp(&right.pid))
            }),
        }
    }
}

fn descending_f64(left: f64, right: f64) -> Ordering {
    right.partial_cmp(&left).unwrap_or(Ordering::Equal)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
