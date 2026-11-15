//! Spawns the player character, the third-person camera and the player's
//! personal fill light.

use super::camera::{CameraRig, ThirdPersonCamera};
use super::character::{
    outfit_colors, spawn_humanoid, ActorBody, CharacterMeshes, PlayerAvatar, PlayerVisual,
};
use crate::core::{VIEW_MODEL_LAYER, WORLD_LAYER};
use crate::world::layout::resort_layout;
use bevy::{core_pipeline::tonemapping::Tonemapping, prelude::*, render::view::RenderLayers};

pub fn spawn_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    existing_meshes: Option<Res<CharacterMeshes>>,
) {
    let layout = resort_layout();
    let spawn = layout.player_spawn;

    // --- Actor root ------------------------------------------------------
    let player = commands
        .spawn((
            Name::new("Player"),
            PlayerAvatar,
            ActorBody::default(),
            Transform::from_translation(spawn),
        ))
        .id();

    // --- Procedural body -------------------------------------------------
    let character_meshes = CharacterMeshes::get_or_insert(&mut meshes, &mut commands, existing_meshes);
    let mat = |materials: &mut Assets<StandardMaterial>, color: Color| {
        materials.add(StandardMaterial {
            base_color: color,
            perceptual_roughness: 0.85,
            ..default()
        })
    };
    let shirt = mat(&mut materials, outfit_colors::SHIRT);
    let trousers = mat(&mut materials, outfit_colors::TROUSERS);
    let skin = mat(&mut materials, outfit_colors::SKIN);
    let hair = mat(&mut materials, outfit_colors::HAIR);
    let shoes = mat(&mut materials, outfit_colors::SHOES);

    spawn_humanoid(
        &mut commands,
        player,
        spawn,
        &character_meshes,
        shirt,
        trousers,
        skin,
        hair,
        shoes,
        Some(PlayerVisual),
    );

    // --- Camera ----------------------------------------------------------
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
            far: 900.0,
            ..default()
        }),
        // The world camera must see the world *and* the player's own model.
        RenderLayers::from_layers(&[WORLD_LAYER, VIEW_MODEL_LAYER]),
        Transform::from_translation(spawn + Vec3::new(0.0, 3.2, -6.0))
            .looking_at(spawn + Vec3::Y * 1.5, Vec3::Y),
        bevy::pbr::DistanceFog {
            color: Color::srgba(0.70, 0.79, 0.86, 1.0),
            falloff: bevy::pbr::FogFalloff::Linear {
                start: 70.0,
                end: 210.0,
            },
            ..default()
        },
    ));

    // --- Personal fill light --------------------------------------------
    commands.spawn((
        Name::new("Player Fill Light"),
        PointLight {
            color: Color::srgb(1.0, 0.95, 0.88),
            intensity: 16_000.0,
            range: 10.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_translation(spawn + Vec3::Y * 2.6),
    ));

    // --- Camera rig resource --------------------------------------------
    // Start looking north, into the resort.
    let rig = CameraRig {
        yaw: 0.0,
        pivot: spawn + Vec3::Y * 1.5,
        position: spawn + Vec3::new(0.0, 3.2, -6.0),
        ..CameraRig::default()
    };
    commands.insert_resource(rig);
}

/// Keeps the player's fill light above them.
pub fn follow_player_light(
    players: Query<&Transform, With<PlayerAvatar>>,
    mut lights: Query<&mut Transform, (With<PointLight>, Without<PlayerAvatar>)>,
) {
    let Ok(player) = players.single() else { return };
    for mut light in &mut lights {
        light.translation = player.translation + Vec3::Y * 2.6;
    }
}
