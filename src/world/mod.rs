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
            .add_plugins(WaterPlugin)
            .add_systems(PreStartup, (load_material_library, setup_world).chain())
            .add_systems(
                Startup,
                (
                    environment::setup_environment,
                    environment::setup_sky_mesh,
                    environment::apply_sky_environment,
                    environment::apply_shadow_settings,
                )
                    .chain()
                    .after(setup_world),
            )
            .add_systems(
                Update,
                (
                    environment::apply_shadow_settings,
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
    commands.insert_resource(WorldCollision::from_layout(&layout));
    info!(
        "Loaded map '{}' with {} solids and {} props",
        layout.name,
        layout.solids.len(),
        layout.deco.len()
    );
    commands.insert_resource(ActiveMap { layout });
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
