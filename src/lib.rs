//! # HITMAN — Coral Palm Resort
//!
//! An original, offline, third-person stealth sandbox built with
//! [Bevy](https://bevyengine.org) — inspired by the *structure* of modern
//! stealth games (social stealth, disguises, restricted areas) but with its own
//! setting, characters and map: the **Coral Palm Resort**.
//!
//! ## Stage 1 — Playable foundation
//!
//! * Third-person camera with smooth movement and collision-aware zoom.
//! * Walking, running, crouching and jumping with a bespoke swept-circle
//!   physics solver (no external physics dependency).
//! * A connected low-poly resort: lobby, restaurant, gardens, beach, service
//!   corridors, security wing and penthouse terrace, with lighting + shadows.
//! * NPCs that follow looping schedules between destinations, idle, collide
//!   with the world and with the player.
//! * A contextual interaction system with prompts, an inventory and notices.
//!
//! Stage 2 extends the same modules with disguises, detection, mission
//! objectives, save/load and audio without replacing any Stage 1 feature.
//!
//! ## Architecture
//!
//! | Module          | Responsibility                                    |
//! |-----------------|---------------------------------------------------|
//! | [`core`]        | Shared data types (zones, collision primitives)    |
//! | [`world`]       | Level data, procedural meshes, lighting            |
//! | [`player`]      | Camera rig, locomotion, physics, character visuals |
//! | [`npc`]         | Routines, navigation, NPC bodies                   |
//! | [`interaction`] | Focus selection, prompts, actions                  |
//! | [`ui`]          | HUD and menus                                      |

pub mod core;
pub mod interaction;
pub mod npc;
pub mod player;
pub mod player_state;
pub mod ui;
pub mod world;

use bevy::{prelude::*, window::CursorGrabMode};

use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use player::character;

/// Top level application flow.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, States)]
pub enum AppState {
    /// Title screen.
    #[default]
    MainMenu,
    /// In the resort.
    Playing,
    /// Mission complete screen (Stage 2).
    MissionComplete,
}

/// Pause overlay state, independent of [`AppState`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, States)]
pub enum PauseState {
    #[default]
    Running,
    Paused,
}

/// Number of frames to run before exiting, set via `HITMAN_SMOKE_FRAMES`.
///
/// This powers the CI smoke test: the real game (window, renderer, all
/// gameplay plugins) boots, runs for a fixed number of frames and exits
/// cleanly. It is inert in normal play.
#[derive(Resource, Debug)]
pub struct SmokeTest {
    pub frames_left: u32,
    pub total: u32,
}

/// Root plugin: inserts every subsystem in the right order.
pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        let smoke_frames = std::env::var("HITMAN_SMOKE_FRAMES")
            .ok()
            .and_then(|value| value.parse::<u32>().ok());

        app.add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    // Asset paths are written relative to the `assets` folder
                    // ("textures/marble_basecolor.png"), so the folder itself is
                    // the root. `main` points BEVY_ASSET_ROOT at the copy that
                    // sits next to the executable, which is what makes the
                    // shipped build portable.
                    file_path: String::new(),
                    // A shipped game has no .meta files; never look for them and
                    // never fail a load because one is absent.
                    meta_check: bevy::asset::AssetMetaCheck::Never,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "HITMAN — Coral Palm Resort".to_string(),
                        resolution: (1600.0f32, 900.0f32).into(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .init_state::<AppState>()
        .init_state::<PauseState>()
        .add_systems(Startup, move |mut commands: Commands, mut next: ResMut<NextState<AppState>>| {
            if let Some(frames) = smoke_frames.filter(|f| *f > 0) {
                info!("SMOKE: running {frames} frames");
                commands.insert_resource(SmokeTest {
                    frames_left: frames,
                    total: frames,
                });
                // Skip the menu so the whole gameplay stack is exercised.
                next.set(AppState::Playing);
            }
        })
        .add_systems(Update, smoke_test_tick)
        .init_resource::<player_state::PlayerState>()
        .insert_resource(bevy::pbr::DirectionalLightShadowMap { size: 2048 })
        .add_plugins((
            world::WorldPlugin,
            player::PlayerPlugin,
            npc::NpcPlugin,
            interaction::InteractionPlugin,
            ui::UiPlugin,
        ))
        .add_systems(Startup, setup_global_environment)

        .add_systems(Update, (handle_state_transitions, capture_cursor, log_smoke_progress))
        .add_systems(Startup, capture_setup)
        // Must run *inside* Update, after the gameplay camera: PostUpdate would
        // place it after transform propagation, so the new pose never reaches
        // the render world.
        .add_systems(
            Update,
            capture_lock_camera.after(player::camera::camera_follow),
        )
        .add_systems(Update, capture_tick)
        .add_systems(Update, character::sync_body_parts);
    }
}

/// Screenshot mode: `HITMAN_CAPTURE="name:x,y,z,tx,ty,tz;..."` renders the
/// level from each listed pose inside a *single* run of the game and writes
/// `captures/<name>.png` for each one. Doing all viewpoints in one process
/// matters: booting the engine and uploading the level costs far more than the
/// frames themselves, especially on a software rasteriser.
///
/// Inert unless the variable is set, so normal play is unaffected.
fn capture_setup(mut state: ResMut<NextState<AppState>>, mut commands: Commands) {
    let Ok(spec) = std::env::var("HITMAN_CAPTURE") else {
        return;
    };
    let mut shots: Vec<CaptureShot> = Vec::new();
    for entry in spec.split(';').filter(|e| !e.trim().is_empty()) {
        let (name, params) = entry.split_once(':').unwrap_or(("frame", entry));
        let values: Vec<f32> = params
            .split(',')
            .filter_map(|v| v.trim().parse().ok())
            .collect();
        if values.len() < 3 {
            continue;
        }
        let eye = Vec3::new(values[0], values[1], values[2]);
        let (yaw, pitch) = if values.len() >= 6 {
            // Position then look-at target: much easier to aim than raw angles.
            let target = Vec3::new(values[3], values[4], values[5]);
            let dir = (target - eye).normalize_or_zero();
            (dir.x.atan2(-dir.z), dir.y.clamp(-0.99, 0.99).asin())
        } else {
            (
                values[3].to_radians(),
                values.get(4).copied().unwrap_or(-0.18),
            )
        };
        shots.push(CaptureShot {
            name: name.trim().to_string(),
            eye,
            yaw,
            pitch,
            // Optional 7th value overrides the camera exposure for this shot,
            // which is how the exposure is calibrated without rerunning CI for
            // every candidate value.
            ev100: values.get(6).copied(),
        });
    }
    if shots.is_empty() {
        warn!("HITMAN_CAPTURE had no usable viewpoints");
        return;
    }
    state.set(AppState::Playing);
    info!("CAPTURE armed for {} viewpoints", shots.len());
    commands.insert_resource(CaptureQueue {
        shots,
        index: 0,
        wait: CAPTURE_WARMUP_FRAMES,
        phase: CapturePhase::Warmup,
    });
}

/// Frames rendered before the first screenshot, giving the renderer time to
/// build pipelines and upload the level.
const CAPTURE_WARMUP_FRAMES: u32 = 90;
/// Frames rendered after moving the camera, so shadows and TAA settle.
const CAPTURE_SETTLE_FRAMES: u32 = 8;
/// Frames to wait for a screenshot to be encoded and written to disk.
const CAPTURE_SAVE_FRAMES: u32 = 12;

/// One scripted camera pose.
#[derive(Clone, Debug)]
pub struct CaptureShot {
    pub name: String,
    pub eye: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    /// Exposure override, in EV100.
    pub ev100: Option<f32>,
}

/// Walks through the requested viewpoints, one screenshot each.
#[derive(Resource)]
pub struct CaptureQueue {
    pub shots: Vec<CaptureShot>,
    pub index: usize,
    /// Frames still to render before the next action.
    pub wait: u32,
    /// What the queue is waiting for.
    pub phase: CapturePhase,
}

/// Where the capture queue is in its cycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapturePhase {
    /// Letting the renderer warm up before the first shot.
    Warmup,
    /// Holding the camera still so shadows, TAA and exposure settle.
    Settle,
    /// A screenshot has been requested; waiting for it to reach the disk.
    Saving,
    /// Every viewpoint has been written.
    Finished,
}

impl CaptureQueue {
    fn current(&self) -> Option<&CaptureShot> {
        self.shots.get(self.index)
    }
}

/// Park the camera at the current pose *after* every gameplay camera system has
/// run, so the follow camera cannot drift the shot.
fn capture_lock_camera(
    mut commands: Commands,
    mut camera: Query<(Entity, &mut Transform), With<Camera3d>>,
    queue: Option<Res<CaptureQueue>>,
) {
    let Some(queue) = queue else { return };
    if queue.phase == CapturePhase::Finished {
        return;
    }
    let Some(shot) = queue.current() else { return };
    if let Ok((entity, mut transform)) = camera.single_mut() {
        transform.translation = shot.eye;
        transform.rotation = Quat::from_euler(EulerRot::YXZ, shot.yaw, shot.pitch, 0.0);
        if let Some(ev100) = shot.ev100 {
            commands
                .entity(entity)
                .insert(bevy::render::camera::Exposure { ev100 });
        }
    }
}

/// Advance the capture queue: render, save, move on, exit when finished.
fn capture_tick(mut commands: Commands, queue: Option<ResMut<CaptureQueue>>) {
    let Some(mut queue) = queue else { return };
    if queue.phase == CapturePhase::Finished {
        return;
    }
    if queue.wait > 0 {
        queue.wait -= 1;
        return;
    }
    match queue.phase {
        CapturePhase::Warmup => {
            queue.phase = CapturePhase::Settle;
            queue.wait = CAPTURE_SETTLE_FRAMES;
        }
        CapturePhase::Settle => {
            let Some(shot) = queue.current().cloned() else {
                queue.phase = CapturePhase::Finished;
                std::process::exit(0);
            };
            let _ = std::fs::create_dir_all("captures");
            let path = format!("captures/{}.png", shot.name);
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path.clone()));
            info!(
                "CAPTURE {} -> {} (eye {:?} yaw {:.1}deg)",
                shot.name,
                path,
                shot.eye,
                shot.yaw.to_degrees()
            );
            queue.index += 1;
            queue.phase = CapturePhase::Saving;
            queue.wait = CAPTURE_SAVE_FRAMES;
        }
        CapturePhase::Saving => {
            if queue.current().is_none() {
                queue.phase = CapturePhase::Finished;
                info!("CAPTURE finished {} viewpoints", queue.shots.len());
                std::process::exit(0);
            }
            queue.phase = CapturePhase::Settle;
            queue.wait = CAPTURE_SETTLE_FRAMES;
        }
        CapturePhase::Finished => {}
    }
}

/// Count frames down and exit cleanly when the smoke test finishes.
fn smoke_test_tick(mut smoke: Option<ResMut<SmokeTest>>, mut exit: EventWriter<AppExit>) {
    let Some(smoke) = smoke.as_mut() else {
        return;
    };
    if smoke.frames_left == 0 {
        return;
    }
    smoke.frames_left -= 1;
    if smoke.frames_left == 0 {
        println!("SMOKE OK frames={}", smoke.total);
        info!("SMOKE OK frames={}", smoke.total);
        exit.write(AppExit::Success);
    }
}

/// Global render + input setup that must run once.
fn setup_global_environment(mut commands: Commands) {
    // 4x MSAA gives the low-poly silhouettes clean edges. `Msaa` is a
    // per-camera component in Bevy 0.16, so it is added to each camera.
    let _ = &mut commands;
    let _ = Msaa::Sample4;
    info!("HITMAN LOW-POLY starting — offline build");
}

/// React to state changes: cursor capture, time scaling and HUD visibility.
fn handle_state_transitions(
    state: Res<State<AppState>>,
    pause: Res<State<PauseState>>,
    mut windows: Query<&mut Window>,
    mut time: ResMut<Time<Virtual>>,
    hud_roots: Query<&mut Visibility, With<ui::hud::HudRoot>>,
) {
    let playing = *state.get() == AppState::Playing;
    let paused = *pause.get() == PauseState::Paused;

    let should_run = playing && !paused;
    if should_run && time.is_paused() {
        time.unpause();
    } else if !should_run && !time.is_paused() {
        time.pause();
    }

    let want_capture = should_run;
    for mut window in &mut windows {
        let want_mode = if want_capture {
            CursorGrabMode::Locked
        } else {
            CursorGrabMode::None
        };
        if window.cursor_options.grab_mode != want_mode {
            window.cursor_options.grab_mode = want_mode;
        }
        let want_visible = !want_capture;
        if window.cursor_options.visible != want_visible {
            window.cursor_options.visible = want_visible;
        }
    }

    let want_hud = if playing {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut visibility in hud_roots {
        if *visibility != want_hud {
            *visibility = want_hud;
        }
    }
}

/// Prints a one-line status dump during smoke tests so CI can verify the
/// simulation actually advanced (player moved, NPCs walked, FPS measured).
fn log_smoke_progress(
    smoke: Option<Res<SmokeTest>>,
    player: Query<&Transform, With<player::character::PlayerAvatar>>,
    npcs: Query<&Transform, With<npc::Npc>>,
    time: Res<Time<Real>>,
    mut last_report: Local<f32>,
) {
    let Some(smoke) = smoke else { return };
    let elapsed = time.elapsed_secs();
    if elapsed - *last_report < 0.5 {
        return;
    }
    *last_report = elapsed;
    let p = player.single().map(|t| t.translation).unwrap_or(Vec3::ZERO);
    let mut npc_moved = 0;
    for t in &npcs {
        if t.translation.length() > 0.1 {
            npc_moved += 1;
        }
    }
    info!(
        "SMOKE frame {} / {} | player ({:.1}, {:.1}, {:.1}) | npcs {}",
        smoke.total - smoke.frames_left,
        smoke.total,
        p.x,
        p.y,
        p.z,
        npc_moved
    );
}

/// Some window managers drop the cursor lock; re-apply it when needed.
fn capture_cursor(
    state: Res<State<AppState>>,
    pause: Res<State<PauseState>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut windows: Query<&mut Window>,
) {
    if *state.get() != AppState::Playing || *pause.get() != PauseState::Running {
        return;
    }
    // Clicking inside the window while the cursor is free re-captures it.
    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }
    for mut window in &mut windows {
        if window.cursor_options.grab_mode == CursorGrabMode::None {
            window.cursor_options.grab_mode = CursorGrabMode::Locked;
            window.cursor_options.visible = false;
        }
    }
}

impl AppState {
    pub fn is_playing(&self) -> bool {
        *self == AppState::Playing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_main_menu() {
        assert_eq!(AppState::default(), AppState::MainMenu);
        assert!(!AppState::default().is_playing());
    }

    #[test]
    fn pause_defaults_to_running() {
        assert_eq!(PauseState::default(), PauseState::Running);
    }
}
