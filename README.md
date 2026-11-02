# MITOSIS — Minecraft Headless Bot Engine

Designed and created by **MitosV**.

**Mitosis** is an ultra-high-density headless bot simulation platform for Minecraft Java Edition (target version: **1.21.1**, protocol version: **767**). It is engineered to simulate hundreds or thousands of high-fidelity bots concurrently without launching graphical Minecraft clients, delivering realistic player behavior from the server's perspective for **end-to-end (E2E) testing, performance benchmarks, player simulations, and minigame automation**.

---

## 1. Architectural Highlights

- **Ultra-Low Memory Footprint**:
  - `Minimal` mode: ~30-50 KB per bot for massive AFK simulations.
  - `Fidelity` mode: Shares immutable chunks, block states, and collision boxes across all bots in the same server/dimension via `Arc<SharedWorldCache>`, avoiding duplicate chunk storage.
- **Vanilla Minecraft 1.21.1 Physics Engine**:
  - Tick-accurate gravity (0.08), air drag (0.91 / 0.98), block friction (default 0.6, ice 0.98, slime 0.8, soul sand 0.4), jump impulses (0.42 + sprint boost), 0.6-block step assist, sneak ledge preservation, and onGround detection.
- **WebAssembly Plugin System**:
  - Plugins compile directly to WebAssembly (`.wasm`) with self-contained metadata embedded in a custom section (`mitosis_metadata`).
  - Powered by the **Wasmtime** runtime with fuel-based CPU execution limits, memory bounds (max 32 MB per plugin), and strict sandboxing.
- **Mitosis Bridge (Fabric 1.21.1 Server Mod)**:
  - Authorizes bots using HMAC-SHA256 signed session tokens without turning off global server `online-mode`.
  - Normal human players continue authenticating through official Mojang session servers.
  - Injects bots into standard `ServerPlayerEntity`, `PlayerManager`, TabList, Scoreboards, and chunk ticket managers.
- **Modern Web Dashboard & REST/WebSocket API**:
  - Built with Axum and WebSocket telemetry stream (`/ws/telemetry`).
  - Real-time CPU, RAM, TPS, MSPT metrics, group management, plugin actions launcher, skin gallery, and benchmark runner.
- **Persistent UUID v5 Identities**:
  - Deterministic UUID v5 generated from immutable internal paths (`group/{name}/bot/{id}`).

---

## 2. Benchmark & Performance Results

Tested on AMD Ryzen / Linux (Rust 1.97.1, release profile with thin LTO):

| Benchmark | Metric | Value |
|---|---|---|
| **Physics Simulation** | Latency | **516.24 ns** per bot tick |
| | Throughput | **1,937,099** bot-ticks/second |
| | Max bots / CPU core @ 20 TPS | **~96,855 bots** |
| **Protocol Framing** | Throughput | **61,345,131** packets/second |
| | Network bandwidth | **2,047.61 MB/s** |

---

## 3. Repository Structure

```text
mitosis/
├── crates/
│   ├── orchestrator/      # CLI application entrypoint & config loader
│   ├── protocol/          # Minecraft 1.21.1 protocol 767 wire framing, packets, VarInts
│   ├── networking/        # TCP framing, AES-128 CFB8 cipher, Zlib compression, reconnect
│   ├── identity/          # UUID v5 generator & HMAC-SHA256 Bridge auth tokens
│   ├── world-cache/       # Multi-bot shared chunk cache & palette decoder
│   ├── physics/           # Vanilla 1.21.1 physics engine & collision sweeps
│   ├── bot-runtime/       # Bot state machine (Fidelity/Minimal) & action queue
│   ├── scheduler/         # 20 TPS tick alignment & Tokio worker pool
│   ├── skins/             # Skin PNG validator, model detector (classic/slim), signatures
│   ├── proxy/             # SOCKS5 & HTTP CONNECT proxy connector
│   ├── plugin-sdk/        # Rust SDK for authoring Mitosis WASM plugins
│   ├── plugin-host/       # Wasmtime runtime loader, sandboxing, and action runner
│   ├── benchmarking/      # Scenario engine & session recorder/replayer
│   ├── metrics/           # Sysinfo collector, telemetry snapshot & Prometheus exporter
│   └── web-api/           # Axum REST API, WebSocket streams, and dashboard UI
├── fabric-bridge/         # Fabric 1.21.1 server-side mod (Java 21, Gradle 8.11, Loom)
├── plugins/               # Compiled WebAssembly plugins (.wasm)
├── skins/                 # Skin PNG repository (Steve, Alex, etc.)
├── web/                   # Web dashboard frontend
├── docker/                # Multistage Dockerfile & docker-compose.yml
├── pterodactyl/           # Pterodactyl egg definition
├── config.toml            # Master TOML configuration file
└── README.md
```

---

## 4. Quickstart Guide

### Prerequisites
- **Rust 1.85+** (`rustc`, `cargo`)
- **Java 21** & `gradlew` (for building `fabric-bridge`)

### Build the Orchestrator & WASM Plugins

```bash
# 1. Compile WASM plugin (d4games.wasm)
RUSTFLAGS="-C link-arg=--import-undefined" cargo build -p d4games --target wasm32-unknown-unknown --release
mkdir -p plugins && cp target/wasm32-unknown-unknown/release/d4games.wasm plugins/

# 2. Build Mitosis Orchestrator
cargo build --release --bin mitosis-orchestrator

# 3. Build Mitosis Bridge (Fabric Mod)
cd fabric-bridge
./gradlew remapJar
# Built JAR is in fabric-bridge/build/libs/mitosis-bridge-1.0.0.jar
cd ..
```

### Running the Orchestrator

```bash
cargo run --release --bin mitosis-orchestrator
```
Open your browser at `http://localhost:8080` to access the live dashboard.

---

## 5. Configuration (`config.toml`)

```toml
[orchestrator]
web_host = "0.0.0.0"
web_port = 8080
secret_key = "mitosis_super_secure_installation_key_change_me"

[performance]
worker_threads = 0
memory_limit_mb = 4096
metrics = true

[group.afk]
server = "127.0.0.1:25565"
prefix = "botafk_"
count = 500
mode = "minimal"

[group.minigame]
server = "127.0.0.1:25565"
prefix = "botgame_"
count = 200
mode = "fidelity"
plugins = ["d4games"]

[group.pvp]
server = "127.0.0.1:25565"
prefix = "botpvp_"
count = 100
mode = "fidelity"
plugins = []
```

---

## 6. WebAssembly Plugin SDK

Create high-performance plugins in Rust without host dependencies:

```rust
use mitosis_plugin_sdk::*;

export_mitosis_plugin!(b"{\
  \"id\":\"d4games\",\
  \"name\":\"D4Games Automation Plugin\",\
  \"version\":\"1.0.0\",\
  \"authors\":[\"MitosV\"],\
  \"description\":\"Minigame automation\",\
  \"api_version\":\"1.0.0\",\
  \"minecraft_versions\":[\"1.21.1\"],\
  \"capabilities\":[\"custom_payloads\",\"actions\",\"assertions\"]\
}");

#[no_mangle]
pub extern "C" fn mitosis_init() {
    log_info("D4Games loaded.");
    register_action("complete_game", "Automates minigame completion");
}

#[no_mangle]
pub extern "C" fn mitosis_execute_action(bot_id: u32, action_ptr: *const u8, action_len: usize, args_ptr: *const u8, args_len: usize) -> u32 {
    // Custom action logic
    0
}
```

---

## 7. Minecraft In-Game Commands

Mitosis Bridge registers Brigadier commands in Fabric:

```text
/player botgame_1 move forward
/player botgame_1 jump
/player botgame_1 attack continuous
/player botgame_1 look at 100 70 200
/player botgame_1 stop

/player group afk connect
/player group afk disconnect
/player group minigame plugin d4games complete_game

/player benchmark start stress_test
/player benchmark stop
/player config reload
```
*(If Carpet mod is installed on the server, the root command automatically aliases to `/bot` to avoid command collisions).*

---

## 8. Docker & Pterodactyl Deployment

### Docker Compose
```bash
cd docker
docker-compose up --build -d
```

### Pterodactyl Egg
Import `pterodactyl/egg-mitosis.json` directly into your Pterodactyl or Wings panel.

---

## 9. Real-World Limitations & Engineering Notes

1. **Server CPU & Network Bottlenecks**: While Mitosis Orchestrator easily handles 10,000+ bots locally, vanilla Minecraft and Fabric dedicated servers typically bottleneck on entity tick loops and chunk packet serialization when more than 200-500 active `Fidelity` bots interact in the same area.
2. **Minimal Mode vs Fidelity Mode**: Use `Minimal` mode for mass AFK stress tests; use `Fidelity` mode for anticheat verification, combat simulation, and minigame E2E tests.
3. **Session Replay Timing**: The Session Replayer reproduces relative packet intervals but network jitter on public connections can affect packet arrival timing.

---

## License

Created and maintained by **MitosV**. Released under the MIT License.
