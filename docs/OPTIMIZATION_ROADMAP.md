# Optimization roadmap

This document tracks performance, stability and Windows-specific improvements for `btop-win`.

## Current review findings

### P0 — runtime efficiency and correctness

- **Event-driven rendering:** the UI previously rendered every 100 ms even when the sampling interval was 1 second and no input occurred. Render only after a new accepted snapshot, keyboard/mouse input or terminal resize.
- **Bounded snapshot delivery:** the collector previously used an unbounded channel. If rendering stalled, complete process snapshots could accumulate in memory and then be displayed late. Keep one pending snapshot and discard newer samples while the UI is behind.
- **Stable process selection:** selection previously followed the table row index. Re-sorting on each sample could silently move the highlight to another process. Preserve selection by PID and reset to the first row only when the selected process exits.
- **Collector failure visibility:** create a named collector thread through `thread::Builder`, propagate thread creation failures, and report unexpected collector termination.
- **Reduce repeated work:** cache host name, OS version and physical-core count, combine four network aggregation passes into one, and cache lowercase process-name keys during name sorting.

These items are implemented by the first optimization pull request.

## Next priorities

### P1 — user-facing monitoring workflow

- Process search/filter mode with `/`, Backspace and Esc.
- Ascending/descending sort direction and direct sort-column shortcuts.
- Per-network-adapter view so loopback, VPN and virtual adapters can be separated.
- Sampling diagnostics: collection duration, skipped samples and UI render duration.
- Persistent configuration for interval, history size, visible panels and theme.
- Better compact layout for narrow Windows Terminal panes.

### P1 — Windows metric quality

- Optional Windows Performance Counter backend for CPU, disk queue and network metrics.
- Optional LibreHardwareMonitor integration for temperature, fan and sensor data.
- GPU backends with clear capability detection rather than mandatory driver dependencies.
- Process start time and parent PID to support a process tree and avoid PID-reuse ambiguity.

### P2 — operations and distribution

- Signed Windows binaries and published SHA-256 checksums.
- WinGet and Scoop manifests generated from GitHub Releases.
- Windows ARM64 build and test coverage.
- Startup and steady-state benchmarks with regression thresholds.
- Long-running soak test to verify stable memory use and terminal restoration.
- Confirmed process actions with permission checks and protection for critical system processes.

## Performance acceptance criteria

- No redraw occurs while paused and idle.
- At most one complete snapshot waits between collector and UI.
- UI input remains responsive while a metric refresh is slow.
- Selected PID remains selected across sorting and sampling updates.
- Collector shutdown cannot deadlock on a full delivery queue.
- CI passes formatting, Clippy, unit tests and the Windows Release build.
