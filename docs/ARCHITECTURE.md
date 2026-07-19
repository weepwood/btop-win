# Architecture and metric semantics

## 1. Why a monitor needs repeated sampling

Most operating-system APIs expose counters, not ready-made charts. A network interface may report total bytes since boot; a process may report accumulated CPU time. A monitor therefore stores a previous sample and computes:

```text
rate = (current_counter - previous_counter) / elapsed_seconds
```

CPU percentage is conceptually similar: CPU time consumed during the interval is divided by elapsed processor time. The first observation cannot produce a reliable percentage, so the collector performs a warm-up refresh.

## 2. Runtime components

### Collector thread

`src/collector.rs` owns `sysinfo::System`, `Networks` and `Disks`. Ownership remains on one thread so no lock is required around native handles. Every cycle it:

1. refreshes CPU, memory, process, network and disk counters;
2. converts interval counters into bytes per second;
3. converts OS-specific strings and enums into display-safe owned values;
4. sends an immutable `Snapshot` through an MPSC channel;
5. sleeps only for the unspent part of the configured interval.

Sleep is split into short pieces so quitting does not wait for an entire long sampling interval.

### Application state

`src/app.rs` receives snapshots and owns interaction state: process sorting, selection, pause status, help visibility and bounded histories. History uses `VecDeque`, so memory use is proportional to the configured number of points rather than runtime.

Pausing does not stop the collector. The event loop continues draining the channel and discards samples, preventing an unbounded queue. Resuming starts from a fresh snapshot.

### Renderer

`src/ui.rs` transforms application state into Ratatui widgets. It does not call system APIs. This separation keeps rendering deterministic and prevents a slow WMI, process or disk query from freezing input.

Ratatui and Crossterm use the terminal alternate screen and update cells through terminal control sequences. This avoids the visible scrolling and flashing produced by repeatedly printing a full dashboard.

### Event loop

`src/main.rs` performs four operations:

1. drain available snapshots;
2. draw one frame;
3. poll input for at most 100 ms;
4. update application state or request shutdown.

The 100 ms input poll controls responsiveness, not metric sampling. Metric sampling uses the independent `--interval` value.

## 3. Metric details

### CPU

System and per-process CPU values need at least two refreshes. Process usage can exceed 100% on multi-core systems because one process can consume multiple logical CPUs.

### Memory

Memory values are bytes. The UI uses binary units (KiB, MiB, GiB). Available memory is displayed separately from used memory.

### Network

`sysinfo` reports bytes received/transmitted since the preceding refresh and lifetime totals. The collector divides interval bytes by real elapsed wall time. Aggregation currently includes all reported interfaces, including virtual adapters.

### Disk

Capacity comes from total and available space. Read/write values are interval counters divided by elapsed time. On Windows, process disk usage may include non-file I/O, matching the semantics exposed by the operating system.

## 4. Terminal lifecycle safety

Raw mode and the alternate screen must always be restored. Normal shutdown calls `restore_terminal`; a panic hook performs a best-effort restore before delegating to Rust's original panic handler.

## 5. Extension points

A future collector interface can support multiple backends:

```rust
trait MetricSource {
    fn sample(&mut self) -> Snapshot;
}
```

Potential Windows-specific sources include PDH/Performance Counters, DXCore or vendor GPU APIs, and an optional LibreHardwareMonitor helper. Those sources should merge into the same snapshot model so the UI remains platform-API agnostic.
