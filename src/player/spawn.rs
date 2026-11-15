//! Spawns the player character, the third-person camera and the player's
//! personal fill light.

use super::camera::{CameraRig, ThirdPersonCamera};
use super::character::{
    spawn_humanoid, ActorBody, CharacterMeshes, GarmentColor, Outfit, OutfitMaterials, PlayerAvatar,
    PlayerVisual, SkinTone,
};
use crate::core::{VIEW_MODEL_LAYER, WORLD_LAYER};
use crate::world::layout::resort_layout;
use bevy::{core_pipeline::tonemapping::Tonemapping, prelude::*, render::view::RenderLayers};

/// The player's default look: linen shirt, navy trousers, tan skin.
pub const PLAYER_OUTFIT: Outfit = Outfit {
    skin: SkinTone::Tan,
    shirt: GarmentColor::Linen,
    trousers: GarmentColor::Navy,
    shoes: GarmentColor::Black,
    hair: GarmentColor::HairBrown,
};

pub fn spawn_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    existing_meshes: Option<Res<CharacterMeshes>>,
    outfits: Option<Res<OutfitMaterials>>,
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

    // --- Humanoid body ---------------------------------------------------
    let character_meshes = CharacterMeshes::get_or_insert(&mut meshes, &mut commands, existing_meshes);
    let Some(outfits) = outfits else {
        return;
    };

    spawn_humanoid(
        &mut commands,
        player,
        spawn,
        &character_meshes,
        &outfits,
        PLAYER_OUTFIT,
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
        bevy::render::camera::Exposure::BLENDER,
        Projection::from(PerspectiveProjection {
            fov: 68f32.to_radians(),
            near: 0.08,
            far: 420.0,
            ..default()
        }),
        // The world camera must see the world *and* the player's own model.
        RenderLayers::from_layers(&[WORLD_LAYER, VIEW_MODEL_LAYER]),
        Transform::from_translation(spawn + Vec3::new(0.0, 3.2, -6.0))
            .looking_at(spawn + Vec3::Y * 1.5, Vec3::Y),
        // Aerial perspective: warm, low-density haze that grows with distance.
        // Exponential-squared falloff looks far more natural than linear fog.
        bevy::pbr::DistanceFog {
            color: Color::srgba(0.72, 0.80, 0.88, 1.0),
            falloff: bevy::pbr::FogFalloff::ExponentialSquared { density: 0.0022 },
            directional_light_color: Color::srgba(1.0, 0.94, 0.82, 1.0),
            directional_light_exponent: 12.0,
            ..default()
        },
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


