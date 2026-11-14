//! # HITMAN LOW-POLY
//!
//! An original, offline, third-person low-poly stealth sandbox built with
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

use bevy::{
    prelude::*,
    window::{CursorGrabMode, PresentMode, WindowResolution},
};

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

/// Root plugin: inserts every subsystem in the right order.
pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "HITMAN LOW-POLY — Coral Palm Resort".to_string(),
                resolution: WindowResolution::new(1600.0, 900.0),
                present_mode: PresentMode::AutoVsync,
                resizable: true,
                ..default()
            }),
            ..default()
        }))
        .init_state::<AppState>()
        .init_state::<PauseState>()
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
        .add_systems(Update, (handle_state_transitions, capture_cursor));
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
