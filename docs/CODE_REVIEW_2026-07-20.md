# Code review — theme and rendering pass

Date: 2026-07-20

Scope: `src/ui.rs`, `src/config.rs`, `src/main.rs`, rendering-related application state, and collector/UI boundaries after `v0.2.0`.

## Fixed in this change

### 1. UI colors were hard-coded inside rendering functions

**Severity:** High for maintainability

The renderer directly referenced a small set of global `Color` constants and several additional literal colors. Adding another palette required editing many unrelated widgets, and semantic roles such as warning, upload, selected row and panel background were not represented.

**Resolution:** Introduce `src/theme.rs` with semantic color roles and four validated built-in palettes. Rendering functions now receive one immutable `Theme` value.

### 2. Panel styling was inconsistent

**Severity:** Medium

Panels, gauges, tables, overlays and the selected process row used unrelated foreground/background assumptions. Some widgets forced black backgrounds while others inherited the terminal background.

**Resolution:** Apply a consistent application background, panel background, rounded borders, title styles, metric accents and selected-row contrast.

### 3. Header diagnostics could overwrite primary identity text

**Severity:** Medium

The previous header rendered right-aligned diagnostics whenever the diagnostics string itself fit. It did not reserve enough room for the left-side application, status, host and OS labels.

**Resolution:** Add a conservative left-side reserve. Diagnostics are omitted before they can overlap primary header information.

### 4. Theme selection had no validation boundary

**Severity:** Medium

Without a typed theme identifier, future string-based selection would spread parsing and fallback logic across the UI.

**Resolution:** Add `ThemeName`, centralized aliases, explicit error messages, CLI parsing and unit tests.

### 5. Theme behavior was undocumented and untested

**Severity:** Low

**Resolution:** Document `--theme`, available palettes and the semantic-role architecture. Add parsing and utilization-threshold tests.

## Follow-up findings

### P1 — stale snapshot validation belongs in `App`

The event loop currently rejects duplicate or out-of-order snapshot sequences before calling `App::apply_snapshot`. The application state method itself does not enforce that invariant, so tests or future callers can still apply stale snapshots directly.

**Recommendation:** Move sequence validation into `App::apply_snapshot` and keep the event loop as a transport concern. Add a regression test that submits a lower sequence after a newer sample.

### P1 — ignored input events still request a redraw

`main.rs` marks the UI dirty after every non-release key event and every mouse event, even when `App` does not change. Unknown keys and mouse movement therefore cause unnecessary terminal draws.

**Recommendation:** Return an input outcome such as `Ignored`, `Changed` or `Quit` from key and mouse handlers and redraw only for `Changed`.

### P2 — process filtering repeats lowercase allocations

`visible_processes()` lowercases the query and each process name/executable on every call. A single render calls the filtered view multiple times for rows, counts and selected-process restoration.

**Recommendation:** Store normalized searchable text in `ProcessSnapshot`, or maintain a visible-index cache invalidated only when the snapshot or query changes.

### P2 — layout measures characters, not terminal cell width

Truncation and header width checks use `.chars().count()`. East Asian wide characters and some symbols occupy two terminal cells, so Chinese host names or process paths can still misalign.

**Recommendation:** Use terminal display-width measurement, for example through `unicode-width`, and test ASCII, CJK and combining characters.

### P2 — hot-plug behavior needs a Windows integration test

The collector refreshes disk and network values continuously, but physical adapter and removable-disk hot-plug behavior should be verified against current `sysinfo` list-refresh semantics on Windows.

**Recommendation:** Add an integration checklist and, where supported, explicitly refresh device lists at a lower cadence than value sampling.

## Review conclusion

The theme refactor improves visual consistency without coupling the collector to Ratatui. The new palette API is small, typed and testable. The remaining issues are independent performance/correctness tasks and should be addressed in focused follow-up pull requests.
