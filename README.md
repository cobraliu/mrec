# mrec

Cross-platform mouse & keyboard macro recorder / replayer, in Rust (one binary).

Records mouse movement, left/right/middle clicks (double-click = two fast
clicks, preserved by timing), scroll wheel, and keyboard input — then replays
it with the original timing, looped any number of times.

## Platform support

| Platform | Status |
|----------|--------|
| Windows | ✅ |
| macOS | ✅ (needs Accessibility permission, see below) |
| Linux / **X11** | ✅ |
| Linux / **Wayland** | ❌ not supported — Wayland blocks global input capture/injection by design |

Check a Linux session with `echo $XDG_SESSION_TYPE` (`x11` = works, `wayland` = no).

## Build

```bash
cargo build --release
# binary -> target/release/mrec
```

Build prerequisites:

- **Linux**: X11 dev headers — `sudo apt install libx11-dev libxtst-dev libxi-dev`
  (runtime only needs the regular `libX11`/`libXtst`, present on any X11 desktop).
- **Windows / macOS**: just the Rust toolchain.

## Usage

```bash
# record for 30s (default); operate your mouse/keyboard during this window
./target/release/mrec record macro.json 30

# replay 10 times
./target/release/mrec replay macro.json 10

# replay forever (Ctrl+C to stop)
./target/release/mrec replay macro.json 0
```

Both commands print a 3-2-1 countdown so you can switch to the target window,
plus a live progress line (seconds left + events while recording, elapsed/total
+ loop number while replaying).

## Notes

- **Coordinates are absolute screen pixels.** Replay assumes the same
  resolution / window layout as recording, or clicks land in the wrong place.
- **macOS**: grant Accessibility permission (System Settings → Privacy &
  Security → Accessibility) to the terminal/binary, required for both capturing
  and injecting events.
- Recording stores raw [`rdev::EventType`](https://docs.rs/rdev) values as JSON;
  replay re-emits them via `rdev::simulate`, so record and replay use one model.
