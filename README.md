# btop-win

[![CI](https://github.com/weepwood/btop-win/actions/workflows/ci.yml/badge.svg)](https://github.com/weepwood/btop-win/actions/workflows/ci.yml)

A fast Windows-native terminal system monitor inspired by **btop**. It is written in Rust and renders a low-flicker TUI with Ratatui.

> This project is an independent implementation. It does not copy btop++ source code or claim compatibility with its configuration format.

## What btop-like monitors do

A terminal monitor is a pipeline rather than a single command:

1. **Collect cumulative counters** from the operating system: CPU time, memory totals, network bytes, disk I/O and process statistics.
2. **Take another sample after an interval.** CPU percentage and transfer speed are calculated from the difference between two adjacent samples.
3. **Normalize the data** into a platform-independent snapshot.
4. **Retain bounded history** for charts. Old points are discarded so memory usage stays stable.
5. **Render only when state changes** into an alternate terminal screen. Idle periods do not redraw the complete interface.
6. **Process keyboard and resize events** in the UI thread while collection runs independently.

`btop-win` follows that architecture: a background collector owns `sysinfo` handles and sends immutable snapshots through a one-item bounded channel. The UI never blocks on Windows system queries and cannot accumulate an unbounded backlog of stale process lists.

## Current features

- Total CPU usage, frequency, logical/physical core counts and per-core summary
- RAM and swap usage
- Network download/upload rate and history graph
- Disk capacity, filesystem and read/write rate
- Process table with CPU, memory, name and I/O sorting
- Ascending/descending sort direction and direct sort shortcuts
- Process filtering by name, executable path or PID
- Stable PID-based selection across refreshes, sorting and filtering
- Collector duration, previous render duration and skipped-snapshot diagnostics
- Keyboard and mouse-wheel navigation
- Pause, chart reset, help overlay and terminal-size handling
- Panic-safe terminal restoration
- Windows CI and tagged release packaging
- Single native executable after compilation

## Install from source

Requirements:

- Windows 10/11
- Windows Terminal or another VT-compatible terminal
- Rust 1.95 or newer using the MSVC toolchain

```powershell
cargo install --git https://github.com/weepwood/btop-win
btop-win
```

Or build the repository:

```powershell
git clone https://github.com/weepwood/btop-win.git
cd btop-win
cargo build --release
.\target\release\btop-win.exe
```

## Usage

```text
btop-win [OPTIONS]

-i, --interval <MS>    Sampling interval, 250-5000 ms (default: 1000)
    --history <COUNT>  History points, 30-600 (default: 120)
-h, --help             Print help
-V, --version          Print version
```

### Keyboard

| Key | Action |
| --- | --- |
| `q`, `Esc`, `Ctrl+C` | Quit; `Esc` clears an active filter first |
| `/` | Enter process-filter editing |
| `Enter` | Keep the filter and leave edit mode |
| `Backspace`, `Esc` | Edit or clear the process filter |
| `c` / `m` / `n` / `d` / `w` | Sort CPU / memory / name / read / write |
| `o` | Toggle ascending/descending order |
| `s` | Cycle process sorting |
| `p`, `Space` | Pause/resume |
| `↑` / `↓`, `j` / `k` | Select a visible process |
| `PageUp` / `PageDown` | Move ten visible rows |
| `Home` / `End` | First/last visible process |
| `r` | Reset chart history |
| `?` | Toggle help |

## Runtime diagnostics

The header reports three lightweight diagnostics:

- **collect**: time spent refreshing and normalizing the latest system snapshot;
- **draw**: duration of the previous terminal render;
- **skip**: cumulative snapshots discarded because the UI still had an older snapshot waiting.

These metrics make performance regressions visible without requiring an external profiler.

## Architecture

```text
Windows counters
      │
      ▼
Collector thread ── refresh/delta/normalize ──► bounded snapshot channel (1)
                                                       │
                                                       ▼
Input events ───────────────────────────────► filtered/sorted app state
                                                       │
                                                       ▼
                                              event-driven renderer
                                                       │
                                                       ▼
                                              Windows Terminal
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for implementation details and metric semantics.

## Known limitations

- GPU utilization and temperature are not included in v0.1 because Windows exposes them through vendor- and driver-dependent paths.
- Process termination is deliberately excluded from the first release.
- Disk I/O on Windows depends on what the operating system and hardware driver expose.
- Process CPU can exceed 100% because it represents use across multiple logical CPUs.
- A terminal smaller than 72×20 cells shows a resize message instead of clipping panels.

## Roadmap

- Per-network-adapter view and adapter filtering
- Windows Performance Counter collector for additional metrics
- Optional LibreHardwareMonitor bridge for temperatures and fan speeds
- GPU backends for NVIDIA, AMD and Intel
- Process tree view with parent PID and start time
- Theme and persistent configuration files
- Scoop and WinGet manifests
- ARM64 Windows release
- Startup, steady-state and long-running regression benchmarks

See [docs/OPTIMIZATION_ROADMAP.md](docs/OPTIMIZATION_ROADMAP.md) for prioritized work and acceptance criteria.

## Development

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo run -- --interval 500
```

Pull requests should keep the collector independent from rendering and avoid blocking work in the event loop.

## License

MIT. See [LICENSE](LICENSE).
