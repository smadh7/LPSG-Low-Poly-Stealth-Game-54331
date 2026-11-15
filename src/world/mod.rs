//! World construction: the resort layout, PBR materials, props and lighting.
//!
//! The world is fully data driven (see [`layout::ResortLayout`]) so additional
//! maps can be added by writing another layout function. Geometry is merged per
//! material and repeated props are instanced, which keeps the resort at a few
//! dozen draw calls.

pub mod builder;
pub mod environment;
pub mod layout;
pub mod materials;
pub mod mesh_kit;
pub mod props;
pub mod water;

use crate::core::ZoneId;
use bevy::prelude::*;
use builder::{LevelEntity, WorldCollision};
use environment::GraphicsSettings;
use materials::{MaterialLibrary, WaterPlugin};

/// Resource holding the active map layout (useful for future multi-map support).
#[derive(Resource)]
pub struct ActiveMap {
    pub layout: layout::ResortLayout,
}

/// Tracks which zone the player is currently standing in.
#[derive(Resource, Debug, Default)]
pub struct PlayerZone {
    pub current: Option<ZoneId>,
    /// Seconds spent in restricted zones (drives the trespass meter in Stage 2).
    pub trespass_timer: f32,
}

/// Plugin that builds the resort when the game starts.
pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GraphicsSettings>()
            .init_resource::<PlayerZone>()
            .init_resource::<water::WaterClock>()
            .add_plugins(WaterPlugin)
            .add_systems(PreStartup, (load_material_library, setup_world).chain())
            .add_systems(
                Startup,
                (
                    environment::setup_environment,
                    environment::build_sky_images,
                    update_player_zone,
                    environment::apply_shadow_settings,
                )
                    .chain()
                    .after(setup_world),
            )
            .add_systems(
                Update,
                (
                    update_player_zone,
                    environment::apply_shadow_settings,
                    environment::apply_post_processing,
                    environment::apply_sky_environment,
                    environment::toggle_fog_by_zone,
                    water::animate_water,
                ),
            );
    }
}

/// Load every PBR material up front.
fn load_material_library(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut water: ResMut<Assets<ExtendedWaterMaterial>>,
) {
    let library = MaterialLibrary::load(&asset_server, &mut materials, &mut water);
    commands.insert_resource(library);
}

fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    library: Option<Res<MaterialLibrary>>,
) {
    let layout = layout::resort_layout();
    if let Some(library) = library.as_deref() {
        builder::spawn_level(&mut commands, &mut meshes, &layout, library);
    }
    // Interior light sources. Shadowless on purpose: they are fill light for
    // rooms the sun cannot reach, and shadow-map budget is better spent on the
    // sun's cascades. `illuminance` is lux at one metre, so it is scaled from
    // luminous power to stay comparable with the 82,000 lux sun.
    for light in &layout.lights {
        commands.spawn((
            LevelEntity,
            Name::new("Interior Light"),
            PointLight {
                color: light.color,
                intensity: light.lumens,
                range: light.range,
                shadows_enabled: false,
                ..default()
            },
            Transform::from_translation(light.position),
        ));
    }
    commands.insert_resource(WorldCollision::from_layout(&layout));
    info!(
        "Loaded map '{}' with {} solids, {} props and {} lights",
        layout.name,
        layout.solids.len(),
        layout.deco.len(),
        layout.lights.len()
    );
    commands.insert_resource(ActiveMap { layout });
}

/// Keep [`PlayerZone`] in sync with where the player is standing.
///
/// Several systems depend on this (eye adaptation, the trespass timer, the HUD
/// zone readout), so it is computed once here rather than in each of them.
fn update_player_zone(
    time: Res<Time>,
    mut zone: ResMut<PlayerZone>,
    collision: Option<Res<WorldCollision>>,
    player: Query<&Transform, With<crate::player::character::PlayerAvatar>>,
    pause: Option<Res<State<crate::PauseState>>>,
) {
    let Some(collision) = collision else { return };
    let Ok(transform) = player.single() else { return };
    let point = Vec2::new(transform.translation.x, transform.translation.z);
    let found = collision.zone_at(point);
    zone.current = Some(found);
    // Restricted zones fill the trespass meter, which existing gameplay reads.
    if found.is_restricted() {
        let paused = pause
            .map(|p| *p.get() != crate::PauseState::Running)
            .unwrap_or(false);
        if !paused {
            zone.trespass_timer += time.delta_secs();
        }
    } else {
        zone.trespass_timer = 0.0;
    }
}

/// Despawn all entities tagged with [`LevelEntity`].
pub fn clear_level(commands: &mut Commands, query: &Query<Entity, With<LevelEntity>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

/// Water material alias, so the type does not have to be spelled out everywhere.
pub type ExtendedWaterMaterial =
    bevy::pbr::ExtendedMaterial<StandardMaterial, materials::WaterMaterial>;
