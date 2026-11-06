# SlowSpot — Find What Is Slowing Down Your Windows PC

[![Rust](https://img.shields.io/badge/rust-stable-brightgreen.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%20%2F%2011%20(64--bit)-blue.svg)](https://microsoft.com)
[![CRT](https://img.shields.io/badge/CRT-statically%20linked-success.svg)](#features)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**SlowSpot** is a tiny, standalone, portable Windows desktop app written in Rust. It answers one question when your PC gets sluggish: **"Which process is actually slowing my computer down, and what should I do about it?"**

Instead of showing hundreds of raw, constantly reshuffling numbers like Task Manager, SlowSpot combines CPU, RAM and disk I/O into a single score using a **dominant-resource** model with damping, points at the culprit, and offers safe actions to deal with it.

> The application UI is in Vietnamese. Project documents under `docs/` are also in Vietnamese; code, comments, commit messages and logs are in English.

---

## Features

1. **Standalone and dependency-free**
   - A single `slowspot.exe` (about 5.7 MB; the portable zip is about 2.5 MB).
   - The C runtime is statically linked (`+crt-static`): no Visual C++ Redistributable (`vcruntime140.dll` / `msvcp140.dll`) is required.
   - A Vietnamese-capable font (Noto Sans) is embedded with `include_bytes!`.
2. **Lightweight**
   - Monitoring uses well under 1% CPU thanks to an event-driven repaint loop.
   - About 50 MB of private memory.
   - No log files, temp files or Registry keys are written while running.
3. **Dominant-resource scoring with damping**
   - Detects CPU saturation, disk I/O saturation and RAM pressure (> 80%).
   - A 3-tick damping window prevents the list from flickering.
   - Levels: **Suspicious** (score 40–69) and **Culprit** (score 70 and above).
4. **Overload tracking**
   - A process that becomes overloaded is tracked from the moment the overload began: **Overloaded → Cooling (N s left) → Recovered / Exited**. The retention window defaults to 120 seconds (`watch_retention_secs`).
   - When the system is overloaded but no single process is to blame, usage is aggregated by image name to name the heaviest group (for example several `rustc.exe` processes).
5. **Safety first**
   - A system-critical whitelist (`csrss.exe`, `lsass.exe`, `smss.exe`, `services.exe`, `winlogon.exe`, `dwm.exe`, `explorer.exe`, …), PID 0/4 and SlowSpot itself can never be closed.
   - Two-tier actions: **Graceful Close** (`WM_CLOSE`) first; **Force Kill** (`TerminateProcess`) becomes available only if the app does not close within 5 seconds.
   - The target image name is re-checked before acting, to defend against PID reuse.
   - If OpenGL cannot be initialised (no GPU driver), a native message box explains why and the app exits cleanly instead of crashing silently.
6. **Clear UI**
   - The header badge and the culprit card are colored by level (green / amber / red) and always paired with an icon and text.
   - 60-second CPU, RAM and disk charts, a searchable Top 20 process table, and an "Ignore this culprit" action for work you started on purpose (video renders, big builds, game downloads).
   - A stable layout: sections keep fixed slots so nothing jumps when the state changes.

---

## Quick start (portable)

1. Download `slowspot-portable.zip` or `slowspot.exe`.
2. Extract anywhere (or copy the exe to a USB drive) and double-click `slowspot.exe`.
   - It starts in under a second.
   - Administrator rights are not required for normal use.

### Demo mode

```powershell
.\slowspot.exe --demo
```

Demo mode uses a simulated data source and never touches real processes. The scenario loops through **Calm → Suspicious → Culprit → Cooling → Recovered**, and the status bar shows the current phase and the seconds left.

### Automated UI screenshots

```powershell
.\scripts\shots.ps1                                  # builds release and captures 4 phases x 2 sizes
.\slowspot.exe --demo --screenshot <dir> --size 1000x680
```

Screenshots are read from the frame rendered by the app itself, so they do not depend on the desktop session. Layout assertions (no clipped or overlapping text, consistent section gaps, minimum visible table rows) run as part of `cargo test`.

---

## Build from source

Requirements:
- Windows 10 / 11 (64-bit)
- Rust toolchain `stable-x86_64-pc-windows-msvc`
- Visual Studio Build Tools (C++ workload)

```powershell
cargo build              # debug build
cargo build --release    # release build -> target\release\slowspot.exe
```

### Packaging

```powershell
powershell -ExecutionPolicy Bypass -File scripts/package.ps1
```

This builds the release binary and creates `dist/slowspot.exe` and `dist/slowspot-portable.zip`.

---

## Testing and verification

```powershell
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

`scripts/verify.ps1` checks the portable binary independently:
1. Only Windows system DLLs are imported (no `vcruntime140*`, `msvcp140*` or `api-ms-win-crt-*`).
2. The exe starts in an empty sandbox folder.
3. The process stays alive after 8 seconds and the window title is `SlowSpot`.
4. CPU and private memory are measured over about 10 seconds.
5. No unexpected files are created.
6. The process is cleaned up.

```powershell
powershell -ExecutionPolicy Bypass -File scripts/verify.ps1
```

---

## Project layout

```
SlowSpot/
├── .cargo/config.toml       # rustflags: static CRT
├── assets/fonts/            # embedded Noto Sans + OFL.txt
├── docs/                    # design documents (Vietnamese)
├── scripts/                 # package.ps1, verify.ps1, shots.ps1
└── src/
    ├── analyzer/            # scoring, damping, safety filter, overload tracker, pressure breakdown
    ├── collector/           # sampling thread, sysinfo source, mock source
    ├── platform/            # Win32 actions (graceful close, force kill, message box)
    ├── ui/                  # eframe/egui UI: app, state, palette, charts, table, widgets, layout assertions
    ├── config.rs            # runtime configuration
    ├── model.rs             # shared data types
    ├── lib.rs
    └── main.rs              # entry point, CLI flags, startup error handling
```

---

## Known limitations

1. **OpenGL 2.0+ is required.** The UI uses the `glow` backend. On machines with only the Windows software OpenGL 1.1 driver, SlowSpot shows a message box and exits instead of crashing.
2. **Administrator rights for protected processes.** Without elevation SlowSpot still shows system-wide usage, but closing processes owned by `SYSTEM` or `LOCAL SERVICE` is denied by Windows. Run SlowSpot as Administrator to act on them.
3. **Platform.** Only Windows 10 / 11 on 64-bit `x86_64` is supported.

---

## License

SlowSpot is released under the [MIT License](LICENSE).

The embedded **Noto Sans** font is distributed under the [SIL Open Font License 1.1](assets/fonts/OFL.txt).
