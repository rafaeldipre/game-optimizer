<div align="center">

# 🎮 Game Optimizer

**A Windows 11 system optimizer for gaming, built specifically for VR.**

[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Windows%2011-blue?logo=windows)](https://www.microsoft.com/windows/windows-11)
[![License](https://img.shields.io/badge/License-MIT-green)](LICENSE)
[![Build](https://img.shields.io/badge/Toolchain-GNU%20%2F%20MinGW--w64-lightgrey)](https://winlibs.com/)

🌐 [Leer en Español](docs/README-ES.md)

</div>

---

## 💡 The Problem — Why does this exist?

It all started with a very specific problem: **micro-stutters**.

When playing in VR, especially in demanding simulators like **DCS World** or **Microsoft Flight Simulator 2024**, Windows keeps running dozens of background processes that compete for CPU, RAM, and disk I/O with the game. The result are those small (and not-so-small) frame hitches that are barely noticeable on a monitor, but **in VR they become genuinely awful**: they break immersion, cause nausea, and ruin the experience.

The idea is simple:

> *When I sit down to play, Windows should know it — and act accordingly.*

Hand all available CPU to the game, free up RAM, shut down services that are completely useless while gaming (updates, indexing, telemetry), and **restore everything automatically** when the session ends.

No existing tool did exactly that in a clean, per-profile configurable way — and none of them understood how complex games like DCS work (it has a two-stage launch before the actual simulator loads). So I built one.

---

## ✨ What does it do?

Game Optimizer is a Windows 11 desktop application written in Rust. Before launching a game, it applies a set of optimizations defined in a **JSON profile**. When the game closes, **everything is restored automatically**.

<br>

<div align="center">

| Optimization | What it does |
|:---|:---|
| 🔴 **Process priority** | Elevates the game to `High` or `Realtime` in the Windows scheduler |
| 🧠 **CPU affinity** | Pins specific CPU cores to the game process |
| 🎨 **GPU preference** | Forces dedicated GPU (DXGI High Performance) via registry |
| 🛑 **Windows services** | Stops SysMain, Windows Search, Windows Update, telemetry |
| ❌ **Background processes** | Kills OneDrive, Discord, SearchIndexer, and others you configure |
| 🧹 **Memory trim** | Frees idle RAM from background processes (100 MB minimum threshold) |
| ♻️ **Auto-restore** | On game exit, restarts services and relaunches closed processes |

</div>

<br>

### 🔄 Smart two-stage launcher handling

Some games (especially **DCS World**) don't launch directly into the simulator — they first open a **launcher** with configuration options, and from there you start the actual game.

Game Optimizer detects this transition automatically:

```
[Launcher PID 1234] ──exits──▶ waits grace period ──▶ [DCS.exe PID 5678] ──monitors──▶ restore
```

- When the launcher exits, it waits a configurable grace period (up to **60 seconds** for DCS, since the game may already be loading in the background before its screen appears)
- If it detects the game process appeared, it transfers monitoring to the new PID
- Re-applies priority and affinity to the actual game process

---

## 🕹️ Included Profiles

### ✈️ DCS World VR

> *The most complete profile — air combat simulator with native VR headset support.*

- **Launch args:** `--force_enable_VR`
- **Services stopped:** SysMain · Windows Search · Windows Update · Telemetry
- **Processes killed:** OneDrive · Discord · SearchIndexer
- **Priority:** `High`
- **Grace period:** 60 s (launcher → simulator transition)

---

### ✈️ Microsoft Flight Simulator 2024

> *Civil aviation simulator — extremely demanding on both RAM and CPU.*

- Similar optimizations to DCS
- **Priority:** `High`
- Aggressive management of Windows background services

---

### 🏎️ F1 24

> *Official Formula 1 game.*

- Tuned for consistent framerate and low frametime variance
- **Priority:** `High`

---

### 🏁 Assetto Corsa

> *The sim-racing community's reference driving simulator.*

- Focused on eliminating micro-stutters during racing sessions
- **Priority:** `High`

---

### 🚀 Star Wars: Squadrons

> *First-person space combat with full VR support.*

- Configured for VR with elevated priority
- **Launch args:** set for VR mode

---

## 📸 Screenshots

*(coming soon)*

---

## 🛠️ Architecture

```
game-optimizer/
├── src/
│   ├── main.rs                  # Entry point, eframe initialization
│   ├── app.rs                   # Shared global state (Arc<Mutex<SessionState>>)
│   ├── core/
│   │   ├── model.rs             # Types: Profile, DelayConfig, SessionState…
│   │   ├── optimizer.rs         # Main logic: apply/restore optimizations
│   │   └── errors.rs            # AppError
│   ├── ui/
│   │   ├── dashboard.rs         # Main screen: Launch/Stop, session status
│   │   ├── profiles_tab.rs      # Profile list and editing
│   │   ├── settings_tab.rs      # General settings
│   │   └── logs_tab.rs          # Real-time log viewer
│   ├── windows/
│   │   ├── monitor.rs           # PID monitoring with relaunch support
│   │   ├── priority.rs          # SetPriorityClass, SetProcessAffinityMask
│   │   ├── memory.rs            # EmptyWorkingSet, GetProcessMemoryInfo
│   │   ├── services.rs          # StartService, ControlService (Win32 SCM)
│   │   ├── processes.rs         # ToolHelp snapshot, kill by name
│   │   ├── gpu.rs               # Registry: GpuPreference DXGI
│   │   └── elevation.rs         # UAC elevation check and request
│   ├── persistence/
│   │   ├── profiles.rs          # JSON profile read/write
│   │   └── session.rs           # session.json with atomic writes
│   └── logging/
│       └── logger.rs            # In-memory ring buffer + rotating log file
├── profiles/                    # Bundled JSON profiles
│   ├── dcs-vr.json
│   ├── msfs-2024.json
│   ├── f1-24.json
│   ├── assetto-corsa.json
│   └── star-wars-squadrons.json
├── assets/
│   └── app.manifest             # UAC manifest (requests elevation automatically)
└── build.rs                     # Embeds the manifest into the binary via winres
```

<br>

**Tech stack:**

| Crate | Version | Purpose |
|:---|:---|:---|
| [`eframe`](https://github.com/emilk/egui) | `0.27` | Immediate-mode UI (egui) |
| [`windows-sys`](https://crates.io/crates/windows-sys) | `0.52` | Win32 bindings — GNU toolchain compatible |
| [`tokio`](https://tokio.rs) | `1` | Async runtime for the monitor loop |
| [`serde`](https://serde.rs) + `serde_json` | `1` | Profile and session serialization |
| [`winres`](https://crates.io/crates/winres) | `0.1` | Embed UAC manifest into the executable |
| [`rfd`](https://crates.io/crates/rfd) | `0.14` | Native file dialogs |
| [`tracing`](https://crates.io/crates/tracing) | `0.1` | Structured logging |

---

## 🚀 Building from source

### Prerequisites

- **Windows 11** (or Windows 10)
- **Rust 1.75+** with target `x86_64-pc-windows-gnu`
- **MinGW-w64 14.2+** — download from [WinLibs](https://winlibs.com/) (UCRT, without LLVM)

### Step 1 — Install Rust and the GNU target

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add x86_64-pc-windows-gnu
```

### Step 2 — Install MinGW-w64

Extract WinLibs into `C:\tmp\mingw-extracted\` so that `gcc.exe` ends up at:
```
C:\tmp\mingw-extracted\mingw64\bin\gcc.exe
```

> The project's `.cargo/config.toml` already points to that path. If you use a different one, edit that file.

### Step 3 — Build

```bash
# In Git Bash
export PATH="/c/tmp/mingw-extracted/mingw64/bin:$PATH:/c/Users/<YOUR_USER>/.cargo/bin"
cd "path/to/game-optimizer"

CARGO_TARGET_DIR="C:/tmp/game-build" cargo build --release
```

> `CARGO_TARGET_DIR` points to a path without spaces to avoid a `dlltool` bug with long paths.

The binary will be at `C:/tmp/game-build/release/game-optimizer.exe`.

---

## 📖 Usage

1. Run `game-optimizer.exe` as **Administrator** (or let the UAC manifest request it automatically)
2. Go to the **Profiles** tab and select an existing profile or create a new one
3. Set the path to the game's executable
4. Go back to the **Dashboard** and press **Launch**
5. The optimizer applies the optimizations, launches the game, and monitors its process
6. When the game closes (or you press **Stop**), **everything is restored automatically**

<br>

### 📝 Creating a custom profile

Profiles are JSON files in the `profiles/` folder. They can be created through the UI or edited manually:

```json
{
  "executable_path": "C:\\path\\to\\game.exe",
  "launch_args": "--some-flag",
  "process_priority": "High",
  "services": [
    { "service_name": "SysMain", "stop_on_launch": true },
    { "service_name": "WSearch", "stop_on_launch": true }
  ],
  "processes": [
    { "exe_name": "OneDrive.exe", "kill_on_launch": true, "relaunch_after": true },
    { "exe_name": "Discord.exe",  "kill_on_launch": true, "relaunch_after": true }
  ],
  "delays": {
    "after_launch_ms": 5000,
    "relaunch_grace_secs": 30
  }
}
```

> **`relaunch_grace_secs`** — seconds the optimizer waits before deciding the game has definitively closed. Useful for two-stage launchers. Default: 30 s. Recommended for DCS: 60 s.

---

## 🤔 Why Rust?

- **No runtime** — the executable is self-contained, no .NET or JVM dependency
- **No overhead** — the optimizer itself consumes negligible resources while monitoring
- **Memory safety** — no risk of corruption when manipulating Win32 handles
- **Static compilation** — a single `.exe`, no installer, no DLLs

---

## ⚠️ Known limitations

- Windows only (uses Win32 API directly)
- Requires **Administrator** privileges
- Built with GNU toolchain; no MSVC support for now
- UI uses egui (immediate mode), not a native Windows framework

---

## 📄 License

[MIT](LICENSE)

---

<div align="center">

*Built because I wanted to fly in DCS without micro-stutters.*

</div>
