<div align="center">

# 🎮 Game Optimizer

**Optimizador de sistema para Windows 11 enfocado en gaming, especialmente en VR.**

[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Windows%2011-blue?logo=windows)](https://www.microsoft.com/windows/windows-11)
[![License](https://img.shields.io/badge/License-MIT-green)](LICENSE)
[![Build](https://img.shields.io/badge/Toolchain-GNU%20%2F%20MinGW--w64-lightgrey)](https://winlibs.com/)

</div>

---

## 💡 El problema — ¿Por qué existe esto?

Todo comenzó con un problema concreto: los **micro-stutters**.

Cuando juego en VR, especialmente en simuladores exigentes como **DCS World** o **Microsoft Flight Simulator 2024**, Windows sigue ejecutando en segundo plano decenas de procesos que compiten por CPU, RAM y disco con el juego. El resultado son esos pequeños (y no tan pequeños) congelamientos de imagen que en un monitor se notan poco, pero **en VR se convierten en algo genuinamente molesto**: rompen la inmersión, generan mareo y arruinan la experiencia.

La idea es simple:

> *Cuando voy a jugar, Windows debería saberlo y comportarse en consecuencia.*

Darle toda la CPU al juego, liberar RAM, apagar servicios que no sirven para nada mientras juego (actualizaciones, indexación, telemetría), y **restaurar todo automáticamente** cuando termine.

No existía una herramienta que hiciera exactamente eso de forma limpia, configurable por perfil, y que entendiera cómo funcionan juegos complejos como DCS (que tiene un launcher de dos etapas antes de cargar el simulador real). Así que la construí.

---

## ✨ ¿Qué hace?

Game Optimizer es una aplicación de escritorio para Windows 11 escrita en Rust. Antes de lanzar un juego, aplica una serie de optimizaciones configuradas en un **perfil JSON**. Cuando el juego se cierra, **revierte todo automáticamente**.

<br>

<div align="center">

| Optimización | Qué hace |
|:---|:---|
| 🔴 **Prioridad de proceso** | Eleva el juego a `High` o `Realtime` en el scheduler de Windows |
| 🧠 **Afinidad de CPU** | Asigna núcleos específicos al proceso del juego |
| 🎨 **Preferencia GPU** | Fuerza GPU dedicada (DXGI High Performance) vía registro |
| 🛑 **Servicios de Windows** | Detiene SysMain, Windows Search, Windows Update, telemetría |
| ❌ **Procesos en segundo plano** | Cierra OneDrive, Discord, SearchIndexer y otros configurados |
| 🧹 **Recorte de memoria** | Libera RAM ociosa de procesos en segundo plano (mín. 100 MB) |
| ♻️ **Restauración automática** | Al cerrar el juego, reinicia servicios y relanza procesos cerrados |

</div>

<br>

### 🔄 Manejo inteligente de launchers de dos etapas

Algunos juegos (especialmente **DCS World**) no se lanzan directamente: primero abren un **launcher** con opciones de configuración y desde ahí se lanza el simulador real.

Game Optimizer detecta esta transición de forma automática:

```
[Launcher PID 1234] ──cierra──▶ espera grace period ──▶ [DCS.exe PID 5678] ──monitorea──▶ restore
```

- Cuando el launcher cierra, espera un período configurable (hasta **60 segundos** para DCS, ya que puede estar cargando en segundo plano antes de mostrar la pantalla del juego)
- Si detecta que el proceso del juego apareció, transfiere el monitoreo al nuevo PID
- Re-aplica prioridad y afinidad al proceso real del juego

---

## 🕹️ Perfiles incluidos

### ✈️ DCS World VR

> *El perfil más completo — simulador de combate aéreo con soporte nativo de headset VR.*

- **Launch args:** `--force_enable_VR`
- **Servicios detenidos:** SysMain · Windows Search · Windows Update · Telemetría
- **Procesos cerrados:** OneDrive · Discord · SearchIndexer
- **Prioridad:** `High`
- **Grace period:** 60 s (transición launcher → simulador)

---

### ✈️ Microsoft Flight Simulator 2024

> *Simulador de aviación civil extremadamente exigente en RAM y CPU.*

- Optimizaciones similares a DCS
- **Prioridad:** `High`
- Gestión agresiva de servicios de Windows en segundo plano

---

### 🏎️ F1 24

> *Juego oficial de Fórmula 1.*

- Configurado para framerate consistente y baja varianza de frametime
- **Prioridad:** `High`

---

### 🏁 Assetto Corsa

> *Simulador de conducción de referencia en la comunidad sim-racing.*

- Enfocado en reducir micro-stutters durante las sesiones de carrera
- **Prioridad:** `High`

---

### 🚀 Star Wars: Squadrons

> *Combate espacial en primera persona con soporte VR.*

- Configurado para VR con prioridad elevada
- **Launch args:** configurados para modo VR

---

## 📸 Capturas de pantalla

*(próximamente)*

---

## 🛠️ Arquitectura

```
game-optimizer/
├── src/
│   ├── main.rs                  # Punto de entrada, inicialización de eframe
│   ├── app.rs                   # Estado global compartido (Arc<Mutex<SessionState>>)
│   ├── core/
│   │   ├── model.rs             # Tipos: Profile, DelayConfig, SessionState…
│   │   ├── optimizer.rs         # Lógica principal: aplicar/restaurar optimizaciones
│   │   └── errors.rs            # AppError
│   ├── ui/
│   │   ├── dashboard.rs         # Pantalla principal: Launch/Stop, estado de sesión
│   │   ├── profiles_tab.rs      # Lista y edición de perfiles
│   │   ├── settings_tab.rs      # Configuración general
│   │   └── logs_tab.rs          # Log en tiempo real
│   ├── windows/
│   │   ├── monitor.rs           # Monitoreo de PID con soporte de re-lanzamiento
│   │   ├── priority.rs          # SetPriorityClass, SetProcessAffinityMask
│   │   ├── memory.rs            # EmptyWorkingSet, GetProcessMemoryInfo
│   │   ├── services.rs          # StartService, ControlService (Win32 SCM)
│   │   ├── processes.rs         # ToolHelp snapshot, kill por nombre
│   │   ├── gpu.rs               # Registro: GpuPreference DXGI
│   │   └── elevation.rs         # Verificación y solicitud de UAC
│   ├── persistence/
│   │   ├── profiles.rs          # Lectura/escritura de perfiles JSON
│   │   └── session.rs           # session.json con escritura atómica
│   └── logging/
│       └── logger.rs            # Buffer en memoria + archivo de log rotado
├── profiles/                    # Perfiles JSON incluidos
│   ├── dcs-vr.json
│   ├── msfs-2024.json
│   ├── f1-24.json
│   ├── assetto-corsa.json
│   └── star-wars-squadrons.json
├── assets/
│   └── app.manifest             # Manifiesto UAC (solicita elevación automática)
└── build.rs                     # Embebe el manifiesto en el ejecutable via winres
```

<br>

**Stack técnico:**

| Crate | Versión | Uso |
|:---|:---|:---|
| [`eframe`](https://github.com/emilk/egui) | `0.27` | UI inmediata (egui) |
| [`windows-sys`](https://crates.io/crates/windows-sys) | `0.52` | Bindings Win32 — compatible con GNU toolchain |
| [`tokio`](https://tokio.rs) | `1` | Runtime async para el loop de monitoreo |
| [`serde`](https://serde.rs) + `serde_json` | `1` | Serialización de perfiles y sesión |
| [`winres`](https://crates.io/crates/winres) | `0.1` | Embeber manifiesto UAC en el ejecutable |
| [`rfd`](https://crates.io/crates/rfd) | `0.14` | Diálogos nativos de archivo |
| [`tracing`](https://crates.io/crates/tracing) | `0.1` | Logging estructurado |

---

## 🚀 Compilar desde código fuente

### Requisitos previos

- **Windows 11** (o Windows 10)
- **Rust 1.75+** con target `x86_64-pc-windows-gnu`
- **MinGW-w64 14.2+** — descargar desde [WinLibs](https://winlibs.com/) (UCRT, sin LLVM)

### Paso 1 — Instalar Rust y el target GNU

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add x86_64-pc-windows-gnu
```

### Paso 2 — Instalar MinGW-w64

Extraer WinLibs en `C:\tmp\mingw-extracted\` de forma que `gcc.exe` quede en:
```
C:\tmp\mingw-extracted\mingw64\bin\gcc.exe
```

> El `.cargo/config.toml` del proyecto ya apunta a esa ruta. Si usás otra, editá ese archivo.

### Paso 3 — Compilar

```bash
# En Git Bash
export PATH="/c/tmp/mingw-extracted/mingw64/bin:$PATH:/c/Users/<USUARIO>/.cargo/bin"
cd "ruta/al/proyecto/game-optimizer"

CARGO_TARGET_DIR="C:/tmp/game-build" cargo build --release
```

> `CARGO_TARGET_DIR` usa una ruta sin espacios para evitar un bug de `dlltool` con rutas largas.

El binario quedará en `C:/tmp/game-build/release/game-optimizer.exe`.

---

## 📖 Uso

1. Ejecutar `game-optimizer.exe` como **Administrador** (o dejar que el manifiesto UAC lo solicite automáticamente)
2. Ir a la pestaña **Perfiles** y seleccionar uno existente o crear uno nuevo
3. Configurar la ruta al ejecutable del juego
4. Volver al **Dashboard** y presionar **Launch**
5. El optimizador aplica las optimizaciones, lanza el juego y monitorea su proceso
6. Al cerrar el juego (o presionar **Stop**), **todo se restaura automáticamente**

<br>

### 📝 Crear un perfil personalizado

Los perfiles son archivos JSON en la carpeta `profiles/`. Se pueden crear desde la UI o editar manualmente:

```json
{
  "executable_path": "C:\\ruta\\al\\juego.exe",
  "launch_args": "--opciones",
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

> **`relaunch_grace_secs`** — segundos que el optimizer espera antes de decidir que el juego cerró definitivamente. Útil para launchers de dos etapas. Default: 30 s. Recomendado para DCS: 60 s.

---

## 🤔 ¿Por qué Rust?

- **Sin runtime** — el ejecutable es autocontenido, sin dependencias de .NET o JVM
- **Sin overhead** — el optimizer en sí no consume recursos perceptibles mientras monitorea
- **Seguridad de memoria** — sin riesgo de corrupción al manipular handles de Win32
- **Compilación estática** — un solo `.exe`, sin instalador, sin DLLs

---

## ⚠️ Limitaciones conocidas

- Solo Windows (usa Win32 API directamente)
- Requiere permisos de **Administrador**
- Compilado con GNU toolchain; sin soporte MSVC por ahora
- La UI usa egui (inmediata), no un framework nativo de Windows

---

## 📄 Licencia

[MIT](LICENSE)

---

<div align="center">

*Construido porque quería volar en DCS sin micro-stutters.*

</div>
