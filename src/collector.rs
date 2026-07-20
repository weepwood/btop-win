use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{SyncSender, TrySendError},
    },
    thread,
    time::{Duration, Instant},
};

use sysinfo::{Disks, MINIMUM_CPU_UPDATE_INTERVAL, Networks, ProcessesToUpdate, System};

use crate::model::{
    CpuSnapshot, DiagnosticsSnapshot, DiskSnapshot, MemorySnapshot, NetworkSnapshot,
    ProcessSnapshot, Snapshot,
};

pub fn spawn_collector(
    sender: SyncSender<Snapshot>,
    stop: Arc<AtomicBool>,
    interval: Duration,
) -> io::Result<thread::JoinHandle<()>> {
    thread::Builder::new()
        .name("metrics-collector".to_owned())
        .spawn(move || {
            let mut collector = Collector::new();
            let mut skipped_samples = 0_u64;

            while !stop.load(Ordering::Relaxed) {
                let cycle_started = Instant::now();
                let mut snapshot = collector.sample();
                snapshot.diagnostics.skipped_samples = skipped_samples;

                match sender.try_send(snapshot) {
                    Ok(()) => {}
                    Err(TrySendError::Full(_)) => {
                        // The UI has not consumed the previous snapshot yet.
                        // Drop this sample instead of growing a stale backlog.
                        skipped_samples = skipped_samples.saturating_add(1);
                    }
                    Err(TrySendError::Disconnected(_)) => break,
                }

                let elapsed = cycle_started.elapsed();
                if elapsed < interval {
                    sleep_interruptibly(interval - elapsed, &stop);
                }
            }
        })
}

fn sleep_interruptibly(duration: Duration, stop: &AtomicBool) {
    let deadline = Instant::now() + duration;
    while !stop.load(Ordering::Relaxed) {
        let now = Instant::now();
        if now >= deadline {
            break;
        }
        thread::sleep((deadline - now).min(Duration::from_millis(50)));
    }
}

struct Collector {
    system: System,
    networks: Networks,
    disks: Disks,
    last_sample: Instant,
    host_name: String,
    os_version: String,
    physical_cores: Option<usize>,
    sequence: u64,
}

impl Collector {
    fn new() -> Self {
        let system = System::new_all();
        let networks = Networks::new_with_refreshed_list();
        let disks = Disks::new_with_refreshed_list();
        let last_sample = Instant::now();
        let host_name = System::host_name().unwrap_or_else(|| "Windows".to_owned());
        let os_version = System::long_os_version().unwrap_or_else(|| "Windows".to_owned());
        let physical_cores = System::physical_core_count();

        // CPU and per-process percentages require two observations separated by
        // at least sysinfo's minimum update interval. Network and disk counters
        // use the same elapsed interval for their first rate calculation.
        thread::sleep(MINIMUM_CPU_UPDATE_INTERVAL);

        Self {
            system,
            networks,
            disks,
            last_sample,
            host_name,
            os_version,
            physical_cores,
            sequence: 0,
        }
    }

    fn sample(&mut self) -> Snapshot {
        let collection_started = Instant::now();
        let now = Instant::now();
        let elapsed_seconds = now
            .duration_since(self.last_sample)
            .as_secs_f64()
            .max(0.001);
        self.last_sample = now;

        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        self.system.refresh_processes(ProcessesToUpdate::All, true);
        self.networks.refresh(true);
        self.disks.refresh(true);
        self.sequence = self.sequence.saturating_add(1);

        let mut snapshot = Snapshot {
            host_name: self.host_name.clone(),
            os_version: self.os_version.clone(),
            uptime_seconds: System::uptime(),
            cpu: self.cpu_snapshot(),
            memory: self.memory_snapshot(),
            network: self.network_snapshot(elapsed_seconds),
            disks: self.disk_snapshots(elapsed_seconds),
            processes: self.process_snapshots(elapsed_seconds),
            diagnostics: DiagnosticsSnapshot {
                sequence: self.sequence,
                ..DiagnosticsSnapshot::default()
            },
        };
        snapshot.diagnostics.collection_duration_ms =
            collection_started.elapsed().as_secs_f64() * 1_000.0;
        snapshot
    }

    fn cpu_snapshot(&self) -> CpuSnapshot {
        let cpus = self.system.cpus();
        CpuSnapshot {
            total_usage: self.system.global_cpu_usage() as f64,
            frequency_mhz: cpus.first().map_or(0, |cpu| cpu.frequency()),
            logical_cores: cpus.len(),
            physical_cores: self.physical_cores,
            per_core_usage: cpus.iter().map(|cpu| cpu.cpu_usage() as f64).collect(),
        }
    }

    fn memory_snapshot(&self) -> MemorySnapshot {
        MemorySnapshot {
            total_bytes: self.system.total_memory(),
            used_bytes: self.system.used_memory(),
            available_bytes: self.system.available_memory(),
            swap_total_bytes: self.system.total_swap(),
            swap_used_bytes: self.system.used_swap(),
        }
    }

    fn network_snapshot(&self, elapsed_seconds: f64) -> NetworkSnapshot {
        let (received, transmitted, total_received, total_transmitted) = self
            .networks
            .list()
            .values()
            .fold((0_u64, 0_u64, 0_u64, 0_u64), |totals, data| {
                (
                    totals.0.saturating_add(data.received()),
                    totals.1.saturating_add(data.transmitted()),
                    totals.2.saturating_add(data.total_received()),
                    totals.3.saturating_add(data.total_transmitted()),
                )
            });

        NetworkSnapshot {
            received_bytes_per_second: received as f64 / elapsed_seconds,
            transmitted_bytes_per_second: transmitted as f64 / elapsed_seconds,
            total_received_bytes: total_received,
            total_transmitted_bytes: total_transmitted,
            interface_count: self.networks.list().len(),
        }
    }

    fn disk_snapshots(&self, elapsed_seconds: f64) -> Vec<DiskSnapshot> {
        self.disks
            .list()
            .iter()
            .map(|disk| {
                let usage = disk.usage();
                DiskSnapshot {
                    name: disk.name().to_string_lossy().into_owned(),
                    mount_point: disk.mount_point().to_string_lossy().into_owned(),
                    file_system: disk.file_system().to_string_lossy().into_owned(),
                    kind: disk.kind().to_string(),
                    total_bytes: disk.total_space(),
                    available_bytes: disk.available_space(),
                    read_bytes_per_second: usage.read_bytes as f64 / elapsed_seconds,
                    written_bytes_per_second: usage.written_bytes as f64 / elapsed_seconds,
                }
            })
            .collect()
    }

    fn process_snapshots(&self, elapsed_seconds: f64) -> Vec<ProcessSnapshot> {
        self.system
            .processes()
            .iter()
            .map(|(pid, process)| {
                let usage = process.disk_usage();
                ProcessSnapshot {
                    pid: pid.as_u32(),
                    name: process.name().to_string_lossy().into_owned(),
                    executable: process
                        .exe()
                        .map(|path| path.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    cpu_usage: process.cpu_usage() as f64,
                    memory_bytes: process.memory(),
                    read_bytes_per_second: usage.read_bytes as f64 / elapsed_seconds,
                    written_bytes_per_second: usage.written_bytes as f64 / elapsed_seconds,
                    status: format!("{:?}", process.status()),
                }
            })
            .collect()
    }
}
