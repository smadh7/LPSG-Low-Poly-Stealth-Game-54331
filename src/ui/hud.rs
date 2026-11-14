//! In-game HUD: stance, zone, interaction prompt, notice feed and inventory.
//!
//! Everything is plain Bevy UI (no image assets) so the game stays fully
//! offline and the HUD scales with the window.

use crate::core::ZoneId;
use crate::interaction::actions::NoticeFeed;
use crate::interaction::FocusTarget;
use crate::player::camera::CameraRig;
use crate::player::character::{ActorBody, PlayerAvatar};
use crate::player::Stance;
use crate::player_state::PlayerState;
use crate::world::builder::WorldCollision;
use bevy::prelude::*;

/// Root of all in-game HUD nodes.
#[derive(Component)]
pub struct HudRoot;

#[derive(Component)]
pub struct StanceText;

#[derive(Component)]
pub struct ZoneText;

#[derive(Component)]
pub struct PromptPanel;

#[derive(Component)]
pub struct PromptText;

#[derive(Component)]
pub struct PromptDetail;

#[derive(Component)]
pub struct NoticeText;

#[derive(Component)]
pub struct InventoryPanel;

#[derive(Component)]
pub struct InventoryText;

#[derive(Component)]
pub struct Crosshair;

#[derive(Component)]
pub struct CompassText;

#[derive(Resource, Default)]
pub struct HudCache {
    pub last_prompt: String,
    pub last_notice: String,
    pub last_inventory: String,
    pub last_stance: String,
    pub last_zone: String,
    pub last_compass: String,
}

pub fn spawn_hud(mut commands: Commands) {
    commands
        .spawn((
            HudRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                ..default()
            },
        ))
        .with_children(|root| {
            // --- Crosshair ---------------------------------------------
            root.spawn((
                Crosshair,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    top: Val::Percent(50.0),
                    width: Val::Px(6.0),
                    height: Val::Px(6.0),
                    margin: UiRect::all(Val::Px(-3.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.55)),
                BorderRadius::all(Val::Px(3.0)),
            ));

            // --- Top-left: stance + zone -------------------------------
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(24.0),
                    top: Val::Px(20.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    ..default()
                },
            ))
            .with_children(|col| {
                col.spawn((
                    Text::new("STANDING"),
                    TextFont {
                        font_size: 22.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.96, 0.94, 0.88)),
                    StanceText,
                ));
                col.spawn((
                    Text::new("Grand Lobby"),
                    TextFont {
                        font_size: 17.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.85, 0.78, 0.62)),
                    ZoneText,
                ));
            });

            // --- Top-right: compass + outfit ---------------------------
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(24.0),
                    top: Val::Px(20.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexEnd,
                    row_gap: Val::Px(4.0),
                    ..default()
                },
            ))
            .with_children(|col| {
                col.spawn((
                    Text::new("N"),
                    TextFont {
                        font_size: 22.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.96, 0.94, 0.88)),
                    CompassText,
                ));
                col.spawn((
                    Text::new("Tourist"),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.80, 0.80, 0.78)),
                    OutfitText,
                ));
            });

            // --- Centred interaction prompt ----------------------------
            root.spawn((
                PromptPanel,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    bottom: Val::Percent(22.0),
                    width: Val::Px(520.0),
                    margin: UiRect::left(Val::Px(-260.0)),
                    padding: UiRect::all(Val::Px(14.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.05, 0.06, 0.08, 0.72)),
                BorderRadius::all(Val::Px(8.0)),
                Visibility::Hidden,
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new("[E] Inspect"),
                    TextFont {
                        font_size: 21.0,
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 0.90, 0.62)),
                    PromptText,
                ));
                panel.spawn((
                    Text::new(""),
                    TextFont {
                        font_size: 15.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.82, 0.84, 0.86)),
                    TextLayout::new_with_justify(JustifyText::Center),
                    PromptDetail,
                ));
            });

            // --- Bottom-left: notice feed ------------------------------
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(24.0),
                    bottom: Val::Px(96.0),
                    max_width: Val::Px(620.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(2.0),
                    ..default()
                },
                NoticeText,
            ))
            .with_children(|col| {
                for _ in 0..4 {
                    col.spawn((
                        Text::new(""),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.93, 0.92, 0.88)),
                    ));
                }
            });

            // --- Bottom-right: inventory -------------------------------
            root.spawn((
                InventoryPanel,
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(24.0),
                    bottom: Val::Px(24.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(3.0),
                    min_width: Val::Px(180.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.05, 0.06, 0.08, 0.55)),
                BorderRadius::all(Val::Px(8.0)),
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new("INVENTORY"),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.72, 0.74, 0.78)),
                ));
                panel.spawn((
                    Text::new("— empty —"),
                    TextFont {
                        font_size: 15.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.90, 0.90, 0.86)),
                    InventoryText,
                ));
            });

            // --- Bottom-centre: controls hint --------------------------
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    bottom: Val::Px(18.0),
                    margin: UiRect::left(Val::Px(-320.0)),
                    width: Val::Px(640.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
            ))
            .with_children(|row| {
                row.spawn((
                    Text::new("WASD move   Shift run   Ctrl crouch   Space jump   E interact   Esc menu"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.92, 0.92, 0.90, 0.65)),
                ));
            });
        });

    commands.insert_resource(HudCache::default());
}

#[derive(Component)]
pub struct OutfitText;

/// Update every HUD element from gameplay state (only when it changes).
pub fn update_hud(
    time: Res<Time>,
    focus: Res<FocusTarget>,
    feed: Res<NoticeFeed>,
    player_state: Res<PlayerState>,
    rig: Res<CameraRig>,
    collision: Option<Res<WorldCollision>>,
    player: Query<(&Transform, &ActorBody), With<PlayerAvatar>>,
    mut cache: ResMut<HudCache>,
    mut stance_text: Query<&mut Text, (With<StanceText>, Without<ZoneText>, Without<PromptText>, Without<PromptDetail>, Without<InventoryText>, Without<CompassText>, Without<OutfitText>)>,
    mut zone_text: Query<&mut Text, (With<ZoneText>, Without<StanceText>, Without<PromptText>, Without<PromptDetail>, Without<InventoryText>, Without<CompassText>, Without<OutfitText>)>,
    mut prompt_text: Query<&mut Text, (With<PromptText>, Without<StanceText>, Without<ZoneText>, Without<PromptDetail>, Without<InventoryText>, Without<CompassText>, Without<OutfitText>)>,
    mut prompt_detail: Query<&mut Text, (With<PromptDetail>, Without<StanceText>, Without<ZoneText>, Without<PromptText>, Without<InventoryText>, Without<CompassText>, Without<OutfitText>)>,
    mut inventory_text: Query<&mut Text, (With<InventoryText>, Without<StanceText>, Without<ZoneText>, Without<PromptText>, Without<PromptDetail>, Without<CompassText>, Without<OutfitText>)>,
    mut compass_text: Query<&mut Text, (With<CompassText>, Without<StanceText>, Without<ZoneText>, Without<PromptText>, Without<PromptDetail>, Without<InventoryText>, Without<OutfitText>)>,
    mut outfit_text: Query<&mut Text, (With<OutfitText>, Without<StanceText>, Without<ZoneText>, Without<PromptText>, Without<PromptDetail>, Without<InventoryText>, Without<CompassText>)>,
    mut prompt_panel: Query<&mut Visibility, With<PromptPanel>>,
    mut notice_children: Query<&mut Text, (With<ChildOf>, Without<StanceText>, Without<ZoneText>, Without<PromptText>, Without<PromptDetail>, Without<InventoryText>, Without<CompassText>, Without<OutfitText>)>,
    notice_roots: Query<&Children, With<NoticeText>>,
) {
    let _ = time;
    let Ok((transform, body)) = player.single() else {
        return;
    };

    // --- Stance --------------------------------------------------------
    let stance_label = match body.stance {
        Stance::Sprinting => "RUNNING",
        Stance::Crouching => "CROUCHED",
        Stance::Airborne => "AIRBORNE",
        Stance::Standing => "STANDING",
    };
    if cache.last_stance != stance_label {
        if let Ok(mut text) = stance_text.single_mut() {
            **text = stance_label.to_string();
        }
        cache.last_stance = stance_label.to_string();
    }

    // --- Zone ----------------------------------------------------------
    let zone = collision
        .as_ref()
        .map(|c| c.zone_at(Vec2::new(transform.translation.x, transform.translation.z)))
        .unwrap_or(ZoneId::Gardens);
    let zone_label = format!("{}  ({:.0} m)", zone.display_name(), transform.translation.y);
    if cache.last_zone != zone_label {
        if let Ok(mut text) = zone_text.single_mut() {
            **text = zone_label.clone();
        }
        cache.last_zone = zone_label;
    }

    // --- Compass -------------------------------------------------------
    let heading = (rig.yaw.to_degrees().rem_euclid(360.0) + 360.0) % 360.0;
    let directions = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];
    let index = ((heading + 22.5) / 45.0).floor() as usize % 8;
    let compass = format!("{}  {:03.0}°", directions[index], heading);
    if cache.last_compass != compass {
        if let Ok(mut text) = compass_text.single_mut() {
            **text = compass.clone();
        }
        cache.last_compass = compass;
    }

    // --- Outfit --------------------------------------------------------
    if let Ok(mut text) = outfit_text.single_mut() {
        let label = format!("Outfit: {}", player_state.outfit.display_name());
        if text.0 != label {
            **text = label;
        }
    }

    // --- Interaction prompt -------------------------------------------
    let has_focus = focus.entity.is_some();
    if let Ok(mut visibility) = prompt_panel.single_mut() {
        let wanted = if has_focus {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != wanted {
            *visibility = wanted;
        }
    }
    if has_focus && cache.last_prompt != focus.prompt {
        if let Ok(mut text) = prompt_text.single_mut() {
            **text = if focus.exhausted {
                format!("{}  (used)", focus.prompt)
            } else {
                focus.prompt.clone()
            };
        }
        if let Ok(mut text) = prompt_detail.single_mut() {
            **text = focus.description.clone();
        }
        cache.last_prompt = focus.prompt.clone();
    }

    // --- Notices -------------------------------------------------------
    let notices: Vec<String> = feed.notices.iter().map(|n| n.text.clone()).collect();
    let joined = notices.join("|");
    if cache.last_notice != joined {
        if let Ok(children) = notice_roots.single() {
            for (i, child) in children.iter().enumerate() {
                if let Ok(mut text) = notice_children.get_mut(child) {
                    **text = notices.get(i).cloned().unwrap_or_default();
                }
            }
        }
        cache.last_notice = joined;
    }

    // --- Inventory -----------------------------------------------------
    let inventory = if feed.inventory.is_empty() {
        "— empty —".to_string()
    } else {
        feed.inventory.join("\n")
    };
    if cache.last_inventory != inventory {
        if let Ok(mut text) = inventory_text.single_mut() {
            **text = inventory.clone();
        }
        cache.last_inventory = inventory;
    }
}

/// Hide the HUD when leaving the gameplay state.
pub fn set_hud_visibility(visibility: Visibility, mut roots: Query<&mut Visibility, With<HudRoot>>) {
    for mut root in &mut roots {
        *root = visibility;
    }
}
