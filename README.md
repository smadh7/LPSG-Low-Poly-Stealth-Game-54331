# HITMAN LOW-POLY

**An original, offline, third-person low-poly stealth sandbox built in Rust with Bevy 0.16.**
Setting: the *Coral Palm Resort* — a luxury island hotel with a lobby, restaurant, gardens,
beach, service corridors, a security wing and a penthouse terrace.

> This is a **from-scratch game**, not a clone: all geometry, characters, level data and
> systems are generated/written in this repository. It is inspired by the *structure* of
> modern social-stealth games (routines, disguises, restricted areas) but uses its own
> setting, cast and objectives.

---

## Quick start (Windows 10/11)

### 1. Install the toolchain

1. Install **Rust** (stable) from <https://rustup.rs> — the default `x86_64-pc-windows-msvc`
   toolchain is correct.
2. Install **Visual Studio Build Tools** with the *Desktop development with C++* workload
   (provides the MSVC linker that Rust needs on Windows).

Check it works:

```powershell
rustc --version    # >= 1.85
cargo --version
```

### 2. Build and run

```powershell
git clone <this-repository>
cd LPSG-Low-Poly-Stealth-Game-54331

cargo run --release
```

The first build takes a while (Bevy is a big dependency graph). Later builds are incremental.

### 3. Make a distributable Windows build

```powershell
cargo build --release
```

The executable is written to:

```
target\release\hitman_lowpoly.exe
```

Copy that single `.exe` anywhere you like — **there are no external asset files to ship**.
All meshes, materials, levels and characters are generated procedurally at runtime, and
the only asset Bevy loads is its built-in default font (compiled into the binary).

> Tip: `cargo build --release` also works from PowerShell/CMD; nothing needs admin rights.

---

## Controls

| Input | Action |
|---|---|
| `W` `A` `S` `D` | Walk (camera relative) |
| `Shift` | Run |
| `Ctrl` / `C` | Crouch |
| `Space` | Jump |
| `E` | Interact with the highlighted object |
| Mouse | Orbit the third-person camera |
| Click (while playing) | Re-capture the mouse if it was released |
| `Esc` | Pause menu |
| `-` / `=` | Zoom the camera out / in (see `Settings`) |

---

## Stage 1 — what is implemented and tested

### Player
* Third-person camera rig with smoothed pivot, smoothed distance and collision-aware
  zoom (the camera pulls in when a wall would occlude the player).
* Walk, run, crouch and jump with acceleration, air control and a bespoke
  swept-circle solver — no external physics crate.
* Steps and ramps: the player climbs the restaurant stairs (0 → 2.4 m), the penthouse
  stair (0 → 29.8 m) and descends the service ramp (0 → −3.4 m).
* Procedural run cycle: arms/legs counter-swing by speed, different pose when crouched
  or airborne.

### Environment
* Connected, explorable resort (~148 × 134 m) with
  **Grand Lobby**, **Azure Restaurant** (raised deck), **Palm Gardens** (hedge maze,
  fountain), **Sunset Beach**, **Service Corridors** (below ground),
  **Security Wing** (restricted) and the **Penthouse Terrace** (restricted, 29.8 m up).
* Low-poly flat-shaded geometry, merged into three draw calls (ground / architecture /
  props) for performance.
* Directional sun with three shadow cascades, soft ambient fill, interior point lights,
  distance fog, ACES-style tonemapping and a sky dome.

### NPC foundation
* 15 NPCs with roles (guests, waiter, chef, concierge, housekeeping, bartender, manager,
  two security guards and the penthouse target).
* Looping schedules of waypoints with dwell times and activity labels.
* Deterministic navigation with wall sliding, step snapping and floor orientation.
* NPC ↔ world and NPC ↔ player collision, plus a role-coloured badge so you can tell
  staff from guards at a glance.

### Interaction
* Raycast + cone focus selection, wall occlusion, distance falloff.
* Contextual prompts (`[E] Inspect Guest Register`) with a description panel.
* 24 authored interactables across every zone: registers, blueprints, keycards,
  wardrobes (disguise hooks), a security camera panel, a safe, a hot tub and more.
* Inventory + notice feed wired into the HUD.

### Technical
* Separate modules: `core`, `world`, `player`, `npc`, `interaction`, `ui`.
* Procedural mesh toolkit (`world/mesh_kit.rs`) shared by level and characters.
* No unsafe code, no unwrap-heavy gameplay paths, and `Result`-free but total APIs
  (every fallible lookup is handled, e.g. missing player → systems no-op).
* Only one dependency: `bevy`. That is deliberate — no physics engine, no navmesh crate,
  no asset pipeline.
* **33 automated tests** (`cargo test`) covering geometry, collision, camera maths,
  routines, interaction and the state machine, plus headless gameplay simulations that
  walk, climb, jump and collide the player.

### Verification note

Every claim above is backed by a test in `cargo test` or reproducible by playing:
`cargo test` runs the headless simulation of the real movement system.

---

## Project structure

```
src/
├── main.rs              # binary entry point
├── lib.rs               # GamePlugin, AppState/PauseState, global setup
├── core.rs              # zones, collision primitives, maths helpers
├── player_state.rs      # outfit + shared player state
├── world/
│   ├── mod.rs           # WorldPlugin, plugin wiring
│   ├── layout.rs        # resort authored as data (solids, props, zones, doors)
│   ├── builder.rs       # layout → merged meshes + collision resource
│   ├── mesh_kit.rs      # procedural low-poly mesh generation
│   └── sky.rs           # lighting, sky dome, camera settings
├── player/
│   ├── mod.rs           # PlayerPlugin, stance, constants
│   ├── spawn.rs         # player + camera + fill light
│   ├── camera.rs        # third-person orbit rig + raycasts
│   ├── character.rs     # body parts, procedural animation, shared meshes
│   └── movement.rs      # input → intent, fixed-step physics
├── npc/
│   ├── mod.rs           # NpcPlugin, roles, identity
│   ├── brain.rs         # routine state machine
│   ├── schedule.rs      # routine steps + timers
│   ├── navigation.rs    # steering, collision, animation
│   └── spawn.rs         # authored population
├── interaction/
│   ├── mod.rs           # Interactable data model + plugin
│   ├── prompts.rs       # focus selection + authored objects
│   └── actions.rs       # action execution, notices, inventory
└── ui/
    ├── mod.rs           # UiPlugin, pause handling
    ├── hud.rs           # HUD
    └── menu.rs          # main menu, pause menu, settings
tests/stage1.rs          # headless acceptance tests
```

### Extending the project
* **New map:** add another function next to `world::layout::resort_layout()` returning a
  `ResortLayout`. Nothing else in the engine is map specific.
* **New prop/interactable:** append to the `entries` vector in
  `interaction::prompts::spawn_interactables` (kind + action + position).
* **New NPC:** add an `NpcSpec` in `npc::spawn::npc_specs()`.
* **New interaction verb:** add an `InteractAction` variant and handle it in
  `interaction::actions::perform_action`.

---

## Continuous integration

`.github/workflows/ci.yml` runs on every push:

1. `cargo check --all-targets`
2. `cargo clippy --all-targets`
3. `cargo test --all-targets`
4. publishes the compiler/test output to the `ci-logs` branch

A manual run with the `windows` input enabled also builds and packages the Windows
release `.exe` as a workflow artifact:

```
gh workflow run ci.yml -f windows=true
```

---

## Frequently asked questions

**Are there any asset files?**
No. Everything except Bevy's embedded default font is generated procedurally at startup,
so the game is a single self-contained executable and works fully offline.

**Why no physics crate?**
The world is made of axis-aligned boxes on a handful of floor levels. A purpose-built
swept-circle solver is ~200 lines, deterministic, trivially testable and avoids a large
dependency (and its build/link cost) for features the game does not need.

**Why Bevy 0.16?**
It is a stable, well-documented release whose API this project is written against.
`Cargo.toml` pins `bevy = "0.16.1"` exactly.
