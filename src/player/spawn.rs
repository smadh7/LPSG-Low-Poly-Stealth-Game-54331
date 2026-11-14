//! Spawns the player character and the third-person camera.

use super::camera::{CameraRig, ThirdPersonCamera};
use super::character::{spawn_avatar_visual, ActorBody, PlayerAvatar};
use crate::core::VIEW_MODEL_LAYER;
use crate::world::builder::LevelEntity;
use crate::world::layout::resort_layout;
use crate::world::sky;
use bevy::{
    core_pipeline::tonemapping::Tonemapping,
    prelude::*,
    render::view::RenderLayers,
};

pub fn spawn_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let layout = resort_layout();
    let spawn = layout.player_spawn;

    let player = commands
        .spawn((
            Name::new("Player"),
            PlayerAvatar,
            ActorBody::default(),
            Transform::from_translation(spawn),
            Visibility::default(),
        ))
        .id();

    spawn_avatar_visual(&mut commands, player, &mut meshes, &mut materials);

    // Third person camera; HDR + tonemapping for a warm, filmic look.
    commands.spawn((
        Name::new("Player Camera"),
        ThirdPersonCamera,
        Camera3d::default(),
        Camera {
            hdr: true,
            ..default()
        },
        Tonemapping::TonyMcMapface,
        Projection::from(PerspectiveProjection {
            fov: 68f32.to_radians(),
            near: 0.08,
            far: 800.0,
            ..default()
        }),
        // The world camera must see both the world and the player's model.
        RenderLayers::from_layers(&[crate::core::WORLD_LAYER, VIEW_MODEL_LAYER]),
        Transform::from_translation(spawn + Vec3::new(0.0, 3.0, -6.0))
            .looking_at(spawn + Vec3::Y * 1.4, Vec3::Y),
        bevy::pbr::DistanceFog {
            color: Color::srgba(0.72, 0.80, 0.86, 1.0),
            falloff: bevy::pbr::FogFalloff::Linear {
                start: 60.0,
                end: 190.0,
            },
            ..default()
        },
    ));

    // A small warm light that follows the player keeps the character readable
    // even in shaded corridors.
    commands.spawn((
        Name::new("Player Fill Light"),
        LevelEntity,
        PointLight {
            color: Color::srgb(1.0, 0.95, 0.88),
            intensity: 22_000.0,
            range: 9.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_translation(spawn + Vec3::Y * 3.0),
    ));

    let mut rig = CameraRig::default();
    rig.pivot = spawn + Vec3::Y * 1.5;
    rig.position = spawn + Vec3::new(0.0, 3.0, -6.0);
    // Start looking into the resort (towards +Z).
    rig.yaw = 0.0;
    commands.insert_resource(rig);

    let _ = sky::camera_bundle;
    let _ = materials.add(StandardMaterial::default());
}
