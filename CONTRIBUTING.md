# Contributing

## Local checks

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo build --release
```

## Design rules

- Keep native metric collection outside the UI thread.
- Send owned snapshot data across threads; do not expose native handles to widgets.
- Use elapsed wall time for rate calculations.
- Bound all time-series buffers.
- Restore terminal state on every exit path.
- Destructive process actions require an explicit confirmation design and tests.
- New Windows-only code should be isolated behind `cfg(target_os = "windows")` where practical.

## Pull requests

Explain the metric source, its units, refresh semantics and expected overhead. Include tests for pure calculations and formatting. UI changes should describe behavior in both normal and minimum terminal sizes.
