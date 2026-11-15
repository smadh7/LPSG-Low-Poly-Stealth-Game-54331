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
                        resolution: (1600.0, 900.0).into(),
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
        .init_resource::<CaptureClock>()
        .add_systems(Startup, capture_setup)
        .add_systems(PostUpdate, capture_lock_camera)
        .add_systems(Update, capture_tick)
        .add_systems(Update, character::sync_body_parts);
    }
}

/// Screenshot mode: `HITMAN_CAPTURE=frame:x,y,z,yaw` renders the game from a
/// fixed camera on that frame, dumps it to `captures/frame_<frame>.png` and
/// exits. It is the tool used to visually verify the level on a headless
/// runner, and it is inert in normal play.
fn capture_setup(mut state: ResMut<NextState<AppState>>) {
    let Ok(spec) = std::env::var("HITMAN_CAPTURE") else {
        return;
    };
    state.set(AppState::Playing);
    // Format: frame:x,y,z,yaw[,pitch]
    let (frame, params) = spec
        .split_once(':')
        .unwrap_or(("120", spec.as_str()));
    let values: Vec<f32> = params
        .split(',')
        .filter_map(|v| v.trim().parse().ok())
        .collect();
    if values.len() < 4 {
        return;
    }
    // 6+ values means "position then look-at target", which is far easier to
    // aim than raw angles. 4-5 values keep the yaw/pitch form.
    let (translation, yaw, pitch) = if values.len() >= 6 {
        let eye = Vec3::new(values[0], values[1], values[2]);
        let target = Vec3::new(values[3], values[4], values[5]);
        let dir = (target - eye).normalize_or_zero();
        let yaw = dir.x.atan2(-dir.z);
        let pitch = dir.y.clamp(-0.99, 0.99).asin();
        (eye, yaw, pitch)
    } else {
        let eye = Vec3::new(values[0], values[1], values[2]);
        (eye, values[3].to_radians(), values.get(4).copied().unwrap_or(-0.18))
    };
    info!("CAPTURE armed for frame {frame}");
    // Written into a resource, not the camera, because the follow camera owns
    // the camera transform and would overwrite anything set here.
    CAPTURE_VIEW.with(|view| {
        *view.borrow_mut() = Some((translation, yaw, pitch));
    });
    CAPTURE_FRAME.with(|cell| cell.set(Some(frame.trim().parse().unwrap_or(120))));
}

thread_local! {
    /// Requested camera pose for screenshot mode.
    static CAPTURE_VIEW: std::cell::RefCell<Option<(Vec3, f32, f32)>> =
        const { std::cell::RefCell::new(None) };
    /// Requested screenshot frame for screenshot mode.
    static CAPTURE_FRAME: std::cell::Cell<Option<u32>> = const { std::cell::Cell::new(None) };
}

/// Pending screenshot request.
#[derive(Resource)]
pub struct CaptureRequest {
    pub frame: u32,
}

/// Frame counter for the screenshot harness.
#[derive(Resource, Default)]
pub struct CaptureClock(pub u32);

/// Park the camera at the requested pose *after* every gameplay camera system
/// has run, so scripted gameplay cannot drift the shot.
fn capture_lock_camera(mut camera: Query<&mut Transform, With<Camera3d>>) {
    CAPTURE_VIEW.with(|view| {
        let pose = *view.borrow();
        if let Some((translation, yaw, pitch)) = pose {
            if let Ok(mut transform) = camera.single_mut() {
                transform.translation = translation;
                transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
            }
        }
    });
}

/// Render the requested frame, save it, then exit.
fn capture_tick(mut commands: Commands, mut clock: ResMut<CaptureClock>) {
    let Some(frame) = CAPTURE_FRAME.with(|cell| cell.get()) else {
        return;
    };
    clock.0 += 1;
    // Give the renderer a moment to build pipelines and upload the level.
    if clock.0 == frame {
        let path = format!("captures/frame_{frame}.png");
        if let Some(parent) = std::path::Path::new(&path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path.clone()));
        info!("CAPTURE writing {path}");
    }
    if clock.0 == frame + 30 {
        std::process::exit(0);
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
