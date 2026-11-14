//! The player character: a low-poly figure assembled from procedural parts.
//!
//! The visual is deliberately simple but fully articulated: the limbs are
//! separate entities so a small procedural animation system can swing them
//! while walking, running and crouching.

use super::{PLAYER_RADIUS, Stance};
use crate::core::VIEW_MODEL_LAYER;
use crate::world::mesh_kit::build_mesh;
use bevy::{prelude::*, render::view::RenderLayers};
use std::f32::consts::FRAC_PI_2;

/// Root marker of the player character.
#[derive(Component)]
pub struct PlayerAvatar;

/// Collision + kinematic state of the player.
#[derive(Component, Debug)]
pub struct ActorBody {
    pub radius: f32,
    pub height: f32,
    pub velocity: Vec3,
    pub grounded: bool,
    /// Absolute Y of the floor currently below the actor.
    pub ground_y: f32,
    pub stance: Stance,
    /// Current animation phase (radians) for the limb swing.
    pub gait: f32,
}

impl Default for ActorBody {
    fn default() -> Self {
        Self {
            radius: PLAYER_RADIUS,
            height: 1.8,
            velocity: Vec3::ZERO,
            grounded: false,
            ground_y: 0.0,
            stance: Stance::Standing,
            gait: 0.0,
        }
    }
}

/// Limb identifiers used by the procedural animation.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Limb {
    LeftLeg,
    RightLeg,
    LeftArm,
    RightArm,
    Torso,
    Head,
}

/// Marks the player's own visual so the camera can avoid clipping.
#[derive(Component)]
pub struct PlayerVisual;

/// Colours of the player's default outfit (a light linen "tourist" outfit).
pub mod outfit_colors {
    use bevy::prelude::*;
    pub const SKIN: Color = Color::srgb(0.80, 0.62, 0.48);
    pub const HAIR: Color = Color::srgb(0.22, 0.16, 0.12);
    pub const SHIRT: Color = Color::srgb(0.93, 0.93, 0.88);
    pub const TROUSERS: Color = Color::srgb(0.36, 0.40, 0.48);
    pub const SHOES: Color = Color::srgb(0.20, 0.20, 0.22);
}

/// Build (or rebuild) the player's visual meshes as children of `parent`.
pub fn spawn_avatar_visual(
    commands: &mut Commands,
    parent: Entity,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    use outfit_colors::*;

    let part = |color: Color| materials.add(StandardMaterial {
        base_color: color,
        perceptual_roughness: 0.85,
        ..default()
    });

    let skin = part(SKIN);
    let hair = part(HAIR);
    let shirt = part(SHIRT);
    let trousers = part(TROUSERS);
    let shoes = part(SHOES);

    let mut spawn_part = |mesh: Handle<Mesh>, mat: Handle<StandardMaterial>, pos: Vec3, limb: Option<Limb>| {
        let mut e = commands.spawn((
            PlayerVisual,
            Mesh3d(mesh),
            MeshMaterial3d(mat),
            Transform::from_translation(pos),
            RenderLayers::layer(VIEW_MODEL_LAYER),
        ));
        if let Some(limb) = limb {
            e.insert(limb);
        }
        let child = e.id();
        commands.entity(parent).add_child(child);
        child
    };

    // Torso
    let torso = meshes.add(build_mesh(|m| {
        m.add_box(Vec3::new(0.0, 0.12, 0.0), Vec3::new(0.56, 0.72, 0.32), SHIRT);
    }));
    spawn_part(torso, shirt.clone(), Vec3::new(0.0, 1.08, 0.0), Some(Limb::Torso));

    // Head + hair
    let head = meshes.add(build_mesh(|m| {
        m.add_box(Vec3::ZERO, Vec3::new(0.30, 0.34, 0.30), SKIN);
        // Nose gives the low-poly head a clear facing direction.
        m.add_box(Vec3::new(0.0, -0.02, 0.17), Vec3::new(0.09, 0.09, 0.08), SKIN);
    }));
    let head_entity = spawn_part(head, skin.clone(), Vec3::new(0.0, 1.62, 0.0), Some(Limb::Head));
    let hair_mesh = meshes.add(build_mesh(|m| {
        m.add_box(Vec3::new(0.0, 0.10, -0.02), Vec3::new(0.33, 0.14, 0.33), HAIR);
    }));
    let hair_child = commands
        .spawn((
            PlayerVisual,
            Mesh3d(hair_mesh),
            MeshMaterial3d(hair),
            Transform::default(),
            RenderLayers::layer(VIEW_MODEL_LAYER),
        ))
        .id();
    commands.entity(head_entity).add_child(hair_child);

    // Legs (pivot at the hip so rotation swings naturally)
    let leg_mesh = meshes.add(build_mesh(|m| {
        m.add_box(Vec3::new(0.0, -0.36, 0.0), Vec3::new(0.20, 0.72, 0.22), TROUSERS);
        m.add_box(Vec3::new(0.0, -0.74, 0.04), Vec3::new(0.22, 0.10, 0.30), SHOES);
    }));
    spawn_part(leg_mesh.clone(), trousers.clone(), Vec3::new(-0.15, 0.74, 0.0), Some(Limb::LeftLeg));
    spawn_part(leg_mesh, trousers, Vec3::new(0.15, 0.74, 0.0), Some(Limb::RightLeg));

    // Arms
    let arm_mesh = meshes.add(build_mesh(|m| {
        m.add_box(Vec3::new(0.0, -0.28, 0.0), Vec3::new(0.14, 0.56, 0.16), SHIRT);
        m.add_box(Vec3::new(0.0, -0.58, 0.0), Vec3::new(0.13, 0.14, 0.15), SKIN);
    }));
    spawn_part(arm_mesh.clone(), shirt.clone(), Vec3::new(-0.36, 1.38, 0.0), Some(Limb::LeftArm));
    spawn_part(arm_mesh, shirt, Vec3::new(0.36, 1.38, 0.0), Some(Limb::RightArm));

    // Player orientation convention: the avatar's forward is +Z.
    let _ = FRAC_PI_2;
}

/// Procedurally animate the avatar limbs based on the actor's movement.
///
/// This is a tiny "run cycle": legs and arms counter-swing with an amplitude
/// driven by speed, with a different pose while crouched or airborne.
pub fn animate_avatar(
    time: Res<Time>,
    mut avatars: Query<(&ActorBody, &mut Transform), (With<PlayerAvatar>, Without<Limb>)>,
    mut limbs: Query<(&Limb, &ChildOf, &mut Transform), (With<Limb>, Without<PlayerAvatar>)>,
    body: Query<&ActorBody, With<PlayerAvatar>>,
) {
    let Ok(player_body) = body.single() else {
        return;
    };

    // Update the gait phase on the root (once per frame).
    if let Ok((_, mut root)) = avatars.single_mut() {
        // Reset the visual root; the body handles world placement.
        root.translation = Vec3::ZERO;
    }

    let speed = player_body.velocity.length();
    let phase = (time.elapsed_secs() * 9.0).rem_euclid(std::f32::consts::TAU);
    let swing = (speed / 7.0).clamp(0.0, 1.0) * 0.85;

    for (limb, child_of, mut transform) in &mut limbs {
        // Only animate the player's own limbs.
        if body.get(child_of.parent()).is_err() {
            continue;
        }
        match limb {
            Limb::LeftLeg => {
                let angle = if player_body.grounded { (phase).sin() * swing } else { 0.35 };
                transform.rotation = Quat::from_rotation_x(angle);
            }
            Limb::RightLeg => {
                let angle = if player_body.grounded { -(phase).sin() * swing } else { -0.25 };
                transform.rotation = Quat::from_rotation_x(angle);
            }
            Limb::LeftArm => {
                let angle = if player_body.grounded { -(phase).sin() * swing * 0.8 } else { -0.5 };
                transform.rotation = Quat::from_rotation_x(angle);
            }
            Limb::RightArm => {
                let angle = if player_body.grounded { (phase).sin() * swing * 0.8 } else { -0.5 };
                transform.rotation = Quat::from_rotation_x(angle);
            }
            Limb::Torso => {
                let bob = if player_body.grounded { (phase * 2.0).sin() * 0.02 * swing } else { 0.0 };
                transform.translation.y = 1.08 + bob;
                let lean = if player_body.stance == Stance::Sprinting { 0.12 } else { 0.0 };
                transform.rotation = Quat::from_rotation_x(-lean);
            }
            Limb::Head => {
                let crouch = if player_body.stance == Stance::Crouching { -0.12 } else { 0.0 };
                transform.translation.y = 1.62 - if player_body.stance == Stance::Crouching { 0.55 } else { 0.0 };
                transform.rotation = Quat::from_rotation_x(crouch);
            }
        }
    }
}
