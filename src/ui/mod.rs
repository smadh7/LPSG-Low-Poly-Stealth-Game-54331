//! All user interface: the in-game HUD plus the main / pause menus.

pub mod hud;
pub mod menu;

use crate::{AppState, PauseState};
use bevy::prelude::*;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<menu::SettingsOpen>()
            .add_systems(Startup, hud::spawn_hud)
            .add_systems(
                Update,
                (
                    hud::update_hud.run_if(in_state(AppState::Playing)),
                    menu::menu_button_system,
                    menu::sync_settings_overlay,
                    toggle_pause,
                ),
            )
            .add_systems(OnEnter(AppState::MainMenu), (menu::spawn_main_menu, show_cursor))
            .add_systems(OnExit(AppState::MainMenu), despawn_main_menu)
            .add_systems(OnEnter(PauseState::Paused), menu::spawn_pause_menu)
            .add_systems(OnExit(PauseState::Paused), despawn_pause_menu);
    }
}

fn despawn_main_menu(mut commands: Commands, roots: Query<Entity, With<menu::MainMenuRoot>>) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}

fn despawn_pause_menu(mut commands: Commands, roots: Query<Entity, With<menu::PauseMenuRoot>>) {
    for root in &roots {
        commands.entity(root).despawn();
    }
}

/// Esc pauses / unpauses during gameplay.
pub fn toggle_pause(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    pause: Res<State<PauseState>>,
    mut next_pause: ResMut<NextState<PauseState>>,
) {
    if *state.get() != AppState::Playing || !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    match pause.get() {
        PauseState::Running => next_pause.set(PauseState::Paused),
        PauseState::Paused => next_pause.set(PauseState::Running),
    }
}

fn show_cursor(mut windows: Query<&mut Window>) {
    for mut window in &mut windows {
        window.cursor_options.grab_mode = CursorGrabMode::None;
        window.cursor_options.visible = true;
    }
}
