# Optimization roadmap

This document tracks performance, stability and Windows-specific improvements for `btop-win`.

## Completed optimization batches

### Batch 1 — runtime efficiency and correctness

- **Event-driven rendering:** render only after a new accepted snapshot, keyboard/mouse input or terminal resize instead of repainting every 100 ms while idle.
- **Bounded snapshot delivery:** keep at most one complete snapshot waiting between the collector and UI, discarding new samples while the UI is behind.
- **Stable process selection:** preserve selection by PID across refreshes and sort changes.
- **Collector failure visibility:** use a named collector thread, propagate creation failures and report unexpected termination.
- **Reduced repeated work:** cache static system metadata, aggregate network counters in one pass and cache process-name sort keys.
- **Safe shutdown:** disconnect the receiver before joining the collector so a full delivery channel cannot deadlock exit.

### Batch 2 — process workflow and visible diagnostics

- **Process filtering:** `/` starts an interactive filter matching process name, executable path or PID. Enter keeps the filter and Esc clears it.
- **Filtered navigation:** table selection, Home/End, paging and PID restoration operate against visible rows rather than raw process indices.
- **Direct sorting:** `c`, `m`, `n`, `d` and `w` select CPU, memory, name, read and write sorting.
- **Sort direction:** pressing an active sort shortcut or `o` toggles ascending and descending order.
- **Sampling diagnostics:** snapshots carry a sequence number, collection duration and cumulative skipped-sample count.
- **Rendering diagnostics:** the application records previous terminal-render duration and render count.
- **Stale-snapshot rejection:** duplicate or out-of-order snapshot sequences are ignored.
- **Regression coverage:** tests cover filtering, ascending sort, pause behavior, bounded history and PID-stable selection.

## Next priorities

### P1 — network and layout workflow

- Per-network-adapter view so physical, loopback, VPN and virtual adapters can be inspected separately.
- Adapter selection and aggregate/all-adapter modes.
- Better compact layout for narrow Windows Terminal panes.
- Adaptive process columns that hide low-priority fields before truncating names.
- A diagnostics overlay with current, maximum and rolling-average collection/render durations.

### P1 — configuration and Windows metric quality

- Persistent configuration for interval, history size, visible panels, selected adapter and theme.
- Optional Windows Performance Counter backend for CPU, disk queue and network metrics.
- Process start time and parent PID to support a process tree and avoid PID-reuse ambiguity.
- Optional LibreHardwareMonitor integration for temperature, fan and sensor data.
- GPU backends with explicit capability detection rather than mandatory driver dependencies.

### P2 — operations and distribution

- Signed Windows binaries and published SHA-256 checksums.
- WinGet and Scoop manifests generated from GitHub Releases.
- Windows ARM64 build and test coverage.
- Startup and steady-state benchmarks with regression thresholds.
- Long-running soak tests for stable memory use and reliable terminal restoration.
- Confirmed process actions with permission checks and protection for critical system processes.

## Performance acceptance criteria

- No redraw occurs while paused and idle.
- At most one complete snapshot waits between collector and UI.
- UI input remains responsive while metric refresh is slow.
- Selected PID remains selected across sorting, filtering and sampling updates.
- Filtering does not clone or retain a second complete process snapshot.
- Duplicate or stale snapshot sequences are not applied.
- Collector shutdown cannot deadlock on a full delivery queue.
- Collection and render durations are visible from inside the application.
- CI passes formatting, Clippy, unit tests and the Windows Release build.
