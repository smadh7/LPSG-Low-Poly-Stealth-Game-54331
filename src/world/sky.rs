//! Lighting and sky for the outdoor resort.
//!
//! A warm late-afternoon sun with shadows, cool ambient fill and a simple
//! gradient "sky" dome keep the scene readable without any texture assets.

use bevy::{
    core_pipeline::tonemapping::Tonemapping,
    pbr::{CascadeShadowConfigBuilder, DirectionalLightShadowMap},
    prelude::*,
    render::camera::Exposure,
};

use crate::world::builder::LevelEntity;

/// Sun direction, shared with systems that need it (e.g. window glints).
pub const SUN_DIRECTION: Vec3 = Vec3::new(-0.55, -0.72, -0.42);

pub fn setup_lighting(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Warm late-afternoon sun.
    commands.spawn((
        LevelEntity,
        Name::new("Sun"),
        DirectionalLight {
            color: Color::srgb(1.0, 0.94, 0.84),
            illuminance: 11_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -0.85,
            -0.65,
            0.0,
        )),
        CascadeShadowConfigBuilder {
            num_cascades: 3,
            minimum_distance: 0.5,
            maximum_distance: 140.0,
            first_cascade_far_bound: 24.0,
            overlap_proportion: 0.25,
        }
        .build(),
    ));

    // Soft blue ambient fill from the sky.
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.62, 0.72, 0.92),
        brightness: 420.0,
        ..default()
    });

    // Sky dome: a large inverted sphere with a simple two-tone look.
    let sky_mesh = {
        let sphere = Sphere::new(400.0);
        match sphere.mesh().ico(2) {
            Ok(mesh) => meshes.add(mesh),
            Err(_) => meshes.add(Cuboid::new(800.0, 800.0, 800.0)),
        }
    };
    let sky_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.36, 0.58, 0.82),
        unlit: true,
        cull_mode: Some(bevy::render::render_resource::Face::Front),
        ..default()
    });
    commands.spawn((
        LevelEntity,
        Name::new("Sky"),
        Mesh3d(sky_mesh),
        MeshMaterial3d(sky_material),
        Transform::default(),
        bevy::pbr::NotShadowCaster,
    ));

    // A few warm fill lights for the lobby and the restaurant interior.
    for (name, position, color) in [
        ("Lobby Light", Vec3::new(0.0, 3.6, 6.0), Color::srgb(1.0, 0.86, 0.66)),
        ("Lobby Light 2", Vec3::new(-14.0, 3.6, 12.0), Color::srgb(1.0, 0.86, 0.66)),
        ("Restaurant Light", Vec3::new(42.0, 5.6, 0.0), Color::srgb(1.0, 0.82, 0.60)),
        ("Service Light", Vec3::new(-40.0, -1.0, -6.0), Color::srgb(0.72, 0.82, 0.90)),
        ("Security Light", Vec3::new(-55.0, -1.0, -18.0), Color::srgb(0.62, 0.78, 0.95)),
        ("Penthouse Light", Vec3::new(0.0, 32.4, 38.0), Color::srgb(1.0, 0.84, 0.62)),
    ] {
        commands.spawn((
            LevelEntity,
            Name::new(name),
            PointLight {
                color,
                intensity: 90_000.0,
                range: 26.0,
                shadows_enabled: false,
                ..default()
            },
            Transform::from_translation(position),
        ));
    }
}

/// Camera settings shared by every gameplay camera.
pub fn camera_bundle() -> impl Bundle {
    (
        Camera3d::default(),
        Camera {
            hdr: true,
            ..default()
        },
        Tonemapping::TonyMcMapface,
        Exposure::default(),
        Projection::from(PerspectiveProjection {
            fov: 65f32.to_radians(),
            near: 0.08,
            far: 600.0,
            ..default()
        }),
    )
}

/// Global shadow-map resolution tweak, applied once at startup.
pub fn shadow_settings() -> DirectionalLightShadowMap {
    DirectionalLightShadowMap { size: 2048 }
}
