//! World construction: the resort layout, procedural meshes and lighting.
//!
//! The world is fully data driven (see [`layout::ResortLayout`]) so additional
//! maps can be added by writing another layout function.

pub mod builder;
pub mod layout;
pub mod mesh_kit;
pub mod sky;

use crate::core::ZoneId;
use bevy::prelude::*;
use builder::{LevelEntity, WorldCollision};

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
        app.init_resource::<PlayerZone>()
            .add_systems(PreStartup, setup_world)
            .add_systems(Startup, sky::setup_lighting.after(setup_world));
    }
}

fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let layout = layout::resort_layout();
    builder::spawn_level(&mut commands, &mut meshes, &mut materials, &layout);
    commands.insert_resource(WorldCollision::from_layout(&layout));
    info!(
        "Loaded map '{}' with {} solids",
        layout.name,
        layout.solids.len()
    );
    commands.insert_resource(ActiveMap { layout });
}

/// Despawn all entities tagged with [`LevelEntity`].
pub fn clear_level(commands: &mut Commands, query: &Query<Entity, With<LevelEntity>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}
