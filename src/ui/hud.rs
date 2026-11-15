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
use bevy::{ecs::query::QueryData, prelude::*};

/// Optional markers for the left-hand status texts.
#[derive(QueryData)]
pub struct StatusTextParts {
    pub stance: Has<StanceText>,
    pub zone: Has<ZoneText>,
    pub compass: Has<CompassText>,
}

/// Marker/flag bundle for the HUD texts that `update_hud_detail` updates.
#[derive(QueryData)]
pub struct HudTextParts {
    pub prompt: Has<PromptText>,
    pub detail: Has<PromptDetail>,
    pub inventory: Has<InventoryText>,
    pub outfit: Has<OutfitText>,
}

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

/// One line of the scrolling notice feed.
#[derive(Component)]
pub struct NoticeLine;

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
                        NoticeLine,
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

/// Update the left-hand HUD block: stance, zone and compass.
pub fn update_hud_status(
    focus: Res<FocusTarget>,
    rig: Res<CameraRig>,
    collision: Option<Res<WorldCollision>>,
    player: Query<(&Transform, &ActorBody), With<PlayerAvatar>>,
    mut cache: ResMut<HudCache>,
    mut texts: Query<(&mut Text, StatusTextParts)>,
) {
    let Ok((transform, body)) = player.single() else {
        return;
    };

    let stance_label = match body.stance {
        Stance::Sprinting => "RUNNING",
        Stance::Crouching => "CROUCHED",
        Stance::Airborne => "AIRBORNE",
        Stance::Standing => "STANDING",
    };
    let zone = collision
        .as_ref()
        .map(|c| c.zone_at(Vec2::new(transform.translation.x, transform.translation.z)))
        .unwrap_or(ZoneId::Gardens);
    let zone_label = format!("{}  ·  {:.0} m", zone.display_name(), transform.translation.y);

    let heading = rig.yaw.to_degrees().rem_euclid(360.0);
    let directions = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];
    let index = ((heading + 22.5) / 45.0).floor() as usize % 8;
    let compass = format!("{}   {:03.0} deg", directions[index], heading);

    for (mut text, parts) in &mut texts {
        if parts.stance {
            if cache.last_stance != stance_label {
                **text = stance_label.to_string();
            }
        } else if parts.zone {
            if cache.last_zone != zone_label {
                **text = zone_label.clone();
            }
        } else if parts.compass {
            if cache.last_compass != compass {
                **text = compass.clone();
            }
        }
    }
    cache.last_stance = stance_label.to_string();
    cache.last_zone = zone_label;
    cache.last_compass = compass;
    let _ = &focus;
}

/// Update the prompt panel, notice feed, inventory and outfit text.
pub fn update_hud_detail(
    focus: Res<FocusTarget>,
    feed: Res<NoticeFeed>,
    player_state: Res<PlayerState>,
    mut cache: ResMut<HudCache>,
    mut prompt_panel: Query<&mut Visibility, With<PromptPanel>>,
    mut singles: Query<(HudTextParts, &mut Text)>,
) {
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
        cache.last_prompt = focus.prompt.clone();
    }

    // --- Inventory + outfit + prompt text -----------------------------
    let inventory = if feed.inventory.is_empty() {
        "— empty —".to_string()
    } else {
        feed.inventory.join("\n")
    };
    let outfit_label = format!("Outfit: {}", player_state.outfit.display_name());
    for (parts, mut text) in &mut singles {
        if parts.prompt {
            let value = if focus.exhausted {
                format!("{}  (used)", focus.prompt)
            } else {
                focus.prompt.clone()
            };
            if text.0 != value {
                **text = value;
            }
        } else if parts.detail && text.0 != focus.description {
            **text = focus.description.clone();
        } else if parts.inventory {
            if cache.last_inventory != inventory {
                **text = inventory.clone();
            }
        } else if parts.outfit && text.0 != outfit_label {
            **text = outfit_label.clone();
        }
    }
    cache.last_inventory = inventory;
}

/// Push the notice feed into its four text lines.
pub fn update_hud_notices(
    feed: Res<NoticeFeed>,
    mut cache: ResMut<HudCache>,
    notice_roots: Query<&Children, (With<NoticeText>, Without<NoticeLine>)>,
    mut lines: Query<&mut Text, With<NoticeLine>>,
) {
    let notices: Vec<String> = feed.notices.iter().map(|n| n.text.clone()).collect();
    let joined = notices.join("|");
    if cache.last_notice == joined {
        return;
    }
    cache.last_notice = joined;
    if let Ok(children) = notice_roots.single() {
        for (index, child) in children.iter().enumerate() {
            if let Ok(mut text) = lines.get_mut(child) {
                **text = notices.get(index).cloned().unwrap_or_default();
            }
        }
    }
}

/// Hide the HUD when leaving the gameplay state.
pub fn set_hud_visibility(visibility: Visibility, mut roots: Query<&mut Visibility, With<HudRoot>>) {
    for mut root in &mut roots {
        *root = visibility;
    }
}
