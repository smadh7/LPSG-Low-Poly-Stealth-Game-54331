//! Main menu, pause menu and settings overlay.
//!
//! Menus are plain Bevy UI with `Button` nodes. Every button carries a
//! [`MenuAction`] component, handled centrally in [`menu_button_system`].

use crate::{AppState, PauseState};
use bevy::prelude::*;

#[derive(Component)]
pub struct MainMenuRoot;

#[derive(Component)]
pub struct PauseMenuRoot;

#[derive(Component)]
pub struct SettingsRoot;

/// Action triggered when a menu button is activated.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    StartGame,
    QuitGame,
    Resume,
    OpenSettings,
    CloseSettings,
    BackToMenu,
}

#[derive(Component)]
pub struct MenuButtonVisual;

const BUTTON_NORMAL: Color = Color::srgba(0.10, 0.12, 0.15, 0.92);
const BUTTON_HOVER: Color = Color::srgba(0.28, 0.26, 0.20, 0.96);
const BUTTON_PRESSED: Color = Color::srgba(0.55, 0.46, 0.24, 0.98);
const TEXT_PRIMARY: Color = Color::srgb(0.96, 0.94, 0.88);

fn button(parent: &mut ChildSpawnerCommands, label: &str, action: MenuAction, width: f32) {
    parent
        .spawn((
            Button,
            MenuButtonVisual,
            action,
            Node {
                width: Val::Px(width),
                height: Val::Px(52.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.5)),
                ..default()
            },
            BackgroundColor(BUTTON_NORMAL),
            BorderColor(Color::srgba(0.85, 0.78, 0.52, 0.55)),
            BorderRadius::all(Val::Px(6.0)),
        ))
        .with_children(|b| {
            b.spawn((
                Text::new(label),
                TextFont {
                    font_size: 22.0,
                    ..default()
                },
                TextColor(TEXT_PRIMARY),
            ));
        });
}

fn full_screen_panel() -> impl Bundle {
    (
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(14.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.03, 0.05, 0.88)),
    )
}

/// Spawn the main menu (called on entering [`AppState::MainMenu`]).
pub fn spawn_main_menu(mut commands: Commands) {
    commands
        .spawn((MainMenuRoot, full_screen_panel()))
        .with_children(|root| {
            root.spawn((
                Text::new("HITMAN LOW-POLY"),
                TextFont {
                    font_size: 58.0,
                    ..default()
                },
                TextColor(Color::srgb(0.93, 0.86, 0.66)),
            ));
            root.spawn((
                Text::new("Coral Palm Resort  ·  a low-poly stealth sandbox"),
                TextFont {
                    font_size: 19.0,
                    ..default()
                },
                TextColor(Color::srgb(0.72, 0.74, 0.76)),
            ));
            root.spawn(Node {
                height: Val::Px(22.0),
                ..default()
            });
            button(root, "Start Mission", MenuAction::StartGame, 320.0);
            button(root, "Settings", MenuAction::OpenSettings, 320.0);
            button(root, "Quit", MenuAction::QuitGame, 320.0);
            root.spawn(Node {
                height: Val::Px(12.0),
                ..default()
            });
            root.spawn((
                Text::new(
                    "WASD move · Shift run · Ctrl crouch · Space jump · E interact · Mouse look · Esc pause",
                ),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.62, 0.64, 0.66)),
            ));
        });
}

/// Spawn the pause overlay.
pub fn spawn_pause_menu(mut commands: Commands) {
    commands
        .spawn((PauseMenuRoot, full_screen_panel()))
        .with_children(|root| {
            root.spawn((
                Text::new("PAUSED"),
                TextFont {
                    font_size: 44.0,
                    ..default()
                },
                TextColor(Color::srgb(0.93, 0.86, 0.66)),
            ));
            button(root, "Resume", MenuAction::Resume, 300.0);
            button(root, "Settings", MenuAction::OpenSettings, 300.0);
            button(root, "Save & Quit to Menu", MenuAction::BackToMenu, 300.0);
        });
}

/// Spawn the settings overlay (kept simple and dependency free).
pub fn spawn_settings(mut commands: Commands) {
    commands
        .spawn((SettingsRoot, full_screen_panel()))
        .with_children(|root| {
            root.spawn((
                Text::new("SETTINGS"),
                TextFont {
                    font_size: 40.0,
                    ..default()
                },
                TextColor(Color::srgb(0.93, 0.86, 0.66)),
            ));
            for line in [
                "Mouse sensitivity  —  scroll wheel or [ / ] while playing",
                "Camera distance  —  - / = while playing",
                "Fullscreen  —  F11",
                "Display mode and audio settings are applied automatically",
                "The game runs fully offline; progress is saved locally.",
            ] {
                root.spawn((
                    Text::new(line),
                    TextFont {
                        font_size: 17.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.80, 0.82, 0.84)),
                ));
            }
            button(root, "Back", MenuAction::CloseSettings, 260.0);
        });
}

/// Button hover / press visuals and action dispatch.
pub fn menu_button_system(
    mut interactions: Query<
        (&Interaction, &MenuAction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    mut commands: Commands,
    mut next_state: ResMut<NextState<AppState>>,
    mut next_pause: ResMut<NextState<PauseState>>,
    pause_state: Res<State<PauseState>>,
    mut settings: Query<Entity, With<SettingsRoot>>,
    mut exit: EventWriter<AppExit>,
) {
    for (interaction, action, mut color) in &mut interactions {
        match *interaction {
            Interaction::Pressed => {
                *color = BUTTON_PRESSED.into();
                match action {
                    MenuAction::StartGame => {
                        next_state.set(AppState::Playing);
                    }
                    MenuAction::QuitGame => {
                        exit.write(AppExit::Success);
                    }
                    MenuAction::Resume => {
                        next_pause.set(PauseState::Running);
                    }
                    MenuAction::OpenSettings => {
                        // Settings overlay is spawned by the state system.
                        commands.insert_resource(SettingsOpen(true));
                    }
                    MenuAction::CloseSettings => {
                        for entity in &mut settings {
                            commands.entity(entity).despawn();
                        }
                        commands.insert_resource(SettingsOpen(false));
                    }
                    MenuAction::BackToMenu => {
                        let _ = pause_state;
                        next_pause.set(PauseState::Running);
                        next_state.set(AppState::MainMenu);
                    }
                }
            }
            Interaction::Hovered => {
                *color = BUTTON_HOVER.into();
            }
            Interaction::None => {
                *color = BUTTON_NORMAL.into();
            }
        }
    }
}

/// Resource tracking whether the settings overlay should be visible.
#[derive(Resource, Debug)]
pub struct SettingsOpen(pub bool);

impl Default for SettingsOpen {
    fn default() -> Self {
        Self(false)
    }
}

/// Spawn the settings root when requested and nothing exists yet.
pub fn sync_settings_overlay(
    mut commands: Commands,
    open: Res<SettingsOpen>,
    existing: Query<Entity, With<SettingsRoot>>,
) {
    if !open.is_changed() {
        return;
    }
    if open.0 && existing.is_empty() {
        spawn_settings(commands.reborrow());
    }
}
