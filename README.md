# btop-win

A fast Windows-native terminal system monitor inspired by **btop**. It is written in Rust and renders a low-flicker TUI with Ratatui.

> This project is an independent implementation. It does not copy btop++ source code or claim compatibility with its configuration format.

## What btop-like monitors do

A terminal monitor is a pipeline rather than a single command:

1. **Collect cumulative counters** from the operating system: CPU time, memory totals, network bytes, disk I/O and process statistics.
2. **Take another sample after an interval.** CPU percentage and transfer speed are calculated from the difference between two adjacent samples.
3. **Normalize the data** into a platform-independent snapshot.
4. **Retain bounded history** for charts. Old points are discarded so memory usage stays stable.
5. **Render only the current frame** into an alternate terminal screen. The terminal backend updates changed cells rather than clearing and repainting the console with ordinary text.
6. **Process keyboard and resize events** in the UI thread while collection runs independently.

`btop-win` follows that architecture: a background collector owns `sysinfo` handles and sends immutable snapshots to the UI thread. The UI never blocks on Windows system queries.

## Current features

- Total CPU usage, frequency, logical/physical core counts and per-core summary
- RAM and swap usage
- Network download/upload rate and history graph
- Disk capacity, filesystem and read/write rate
- Process table with CPU, memory and I/O sorting
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
| `q`, `Esc`, `Ctrl+C` | Quit |
| `p`, `Space` | Pause/resume |
| `s` | Cycle process sorting |
| `↑` / `↓`, `j` / `k` | Select process |
| `PageUp` / `PageDown` | Move ten rows |
| `Home` / `End` | First/last process |
| `r` | Reset chart history |
| `?` | Toggle help |

## Architecture

```text
Windows counters
      │
      ▼
Collector thread ── refresh/delta/normalize ──► Snapshot channel
                                                    │
                                                    ▼
Input events ───────────────────────────────► App state
                                                    │
                                                    ▼
                                             Ratatui renderer
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

- Windows Performance Counter collector for additional metrics
- Optional LibreHardwareMonitor bridge for temperatures and fan speeds
- GPU backends for NVIDIA, AMD and Intel
- Search/filter mode and process tree view
- Theme and persistent configuration files
- Scoop and WinGet manifests
- ARM64 Windows release

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
