#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub host_name: String,
    pub os_version: String,
    pub uptime_seconds: u64,
    pub cpu: CpuSnapshot,
    pub memory: MemorySnapshot,
    pub network: NetworkSnapshot,
    pub disks: Vec<DiskSnapshot>,
    pub processes: Vec<ProcessSnapshot>,
    pub diagnostics: DiagnosticsSnapshot,
}

#[derive(Clone, Debug, Default)]
pub struct DiagnosticsSnapshot {
    pub sequence: u64,
    pub collection_duration_ms: f64,
    pub skipped_samples: u64,
}

#[derive(Clone, Debug, Default)]
pub struct CpuSnapshot {
    pub total_usage: f64,
    pub frequency_mhz: u64,
    pub logical_cores: usize,
    pub physical_cores: Option<usize>,
    pub per_core_usage: Vec<f64>,
}

#[derive(Clone, Debug, Default)]
pub struct MemorySnapshot {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
}

impl MemorySnapshot {
    pub fn used_ratio(&self) -> f64 {
        ratio(self.used_bytes, self.total_bytes)
    }

    pub fn swap_used_ratio(&self) -> f64 {
        ratio(self.swap_used_bytes, self.swap_total_bytes)
    }
}

#[derive(Clone, Debug, Default)]
pub struct NetworkSnapshot {
    pub received_bytes_per_second: f64,
    pub transmitted_bytes_per_second: f64,
    pub total_received_bytes: u64,
    pub total_transmitted_bytes: u64,
    pub interface_count: usize,
}

#[derive(Clone, Debug, Default)]
pub struct DiskSnapshot {
    pub name: String,
    pub mount_point: String,
    pub file_system: String,
    pub kind: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub read_bytes_per_second: f64,
    pub written_bytes_per_second: f64,
}

impl DiskSnapshot {
    pub fn used_bytes(&self) -> u64 {
        self.total_bytes.saturating_sub(self.available_bytes)
    }

    pub fn used_ratio(&self) -> f64 {
        ratio(self.used_bytes(), self.total_bytes)
    }
}

#[derive(Clone, Debug, Default)]
pub struct ProcessSnapshot {
    pub pid: u32,
    pub name: String,
    pub executable: String,
    pub cpu_usage: f64,
    pub memory_bytes: u64,
    pub read_bytes_per_second: f64,
    pub written_bytes_per_second: f64,
    pub status: String,
}

fn ratio(value: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        (value as f64 / total as f64).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratios_handle_zero_totals() {
        let memory = MemorySnapshot::default();
        assert_eq!(memory.used_ratio(), 0.0);
        assert_eq!(memory.swap_used_ratio(), 0.0);
    }

    #[test]
    fn disk_used_space_is_saturating() {
        let disk = DiskSnapshot {
            total_bytes: 100,
            available_bytes: 120,
            ..DiskSnapshot::default()
        };
        assert_eq!(disk.used_bytes(), 0);
    }
}
