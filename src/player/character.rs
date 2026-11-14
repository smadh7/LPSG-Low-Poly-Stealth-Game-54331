//! The player character: a low-poly figure assembled from procedural parts.
//!
//! The visual parts are separate world entities that follow the avatar every
//! frame (see [`sync_body_parts`]). Tweaking limb rotations in world space is
//! far simpler than fighting parent transforms, and it means the same
//! [`BodyPart`] component can be reused for NPC bodies.

use super::{PLAYER_RADIUS, Stance};
use crate::world::mesh_kit::build_mesh;
use bevy::prelude::*;
use std::f32::consts::FRAC_PI_2;

/// Root marker of the player character.
#[derive(Component)]
pub struct PlayerAvatar;

/// Collision + kinematic state of an actor (player or NPC).
#[derive(Component, Debug)]
pub struct ActorBody {
    pub radius: f32,
    pub height: f32,
    pub velocity: Vec3,
    pub grounded: bool,
    /// Absolute Y of the floor currently below the actor.
    pub ground_y: f32,
    pub stance: Stance,
    /// Current gait phase (radians) for the limb swing.
    pub gait: f32,
}

impl Default for ActorBody {
    fn default() -> Self {
        Self {
            radius: PLAYER_RADIUS,
            height: 1.8,
            velocity: Vec3::ZERO,
            grounded: true,
            ground_y: 0.0,
            stance: Stance::Standing,
            gait: 0.0,
        }
    }
}

/// A procedural body part that follows an actor.
#[derive(Component, Debug)]
pub struct BodyPart {
    /// Actor this part belongs to.
    pub owner: Entity,
    /// Offset from the actor's origin, in the actor's local space.
    pub offset: Vec3,
    /// Limb identifier (drives procedural animation). `None` for static parts.
    pub limb: Option<Limb>,
}

/// Marks the player's own visual parts (used for render layers / hiding).
#[derive(Component)]
pub struct PlayerVisual;

/// The parts of a character that can be animated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Limb {
    LeftLeg,
    RightLeg,
    LeftArm,
    RightArm,
    Torso,
    Head,
}

/// Resting offsets for a standard humanoid built from the shared part meshes.
pub struct HumanoidRig;

impl HumanoidRig {
    pub const TORSO: Vec3 = Vec3::new(0.0, 1.08, 0.0);
    pub const HEAD: Vec3 = Vec3::new(0.0, 1.62, 0.0);
    pub const LEFT_LEG: Vec3 = Vec3::new(-0.15, 0.74, 0.0);
    pub const RIGHT_LEG: Vec3 = Vec3::new(0.15, 0.74, 0.0);
    pub const LEFT_ARM: Vec3 = Vec3::new(-0.36, 1.38, 0.0);
    pub const RIGHT_ARM: Vec3 = Vec3::new(0.36, 1.38, 0.0);
}

/// Colours of the player's default outfit (a light linen "tourist" outfit).
pub mod outfit_colors {
    use bevy::prelude::*;
    pub const SKIN: Color = Color::srgb(0.80, 0.62, 0.48);
    pub const HAIR: Color = Color::srgb(0.22, 0.16, 0.12);
    pub const SHIRT: Color = Color::srgb(0.93, 0.93, 0.88);
    pub const TROUSERS: Color = Color::srgb(0.36, 0.40, 0.48);
    pub const SHOES: Color = Color::srgb(0.20, 0.20, 0.22);
}

/// Shared meshes for humanoid characters (created once).
#[derive(Resource, Clone)]
pub struct CharacterMeshes {
    pub torso: Handle<Mesh>,
    pub head: Handle<Mesh>,
    pub hair: Handle<Mesh>,
    pub leg: Handle<Mesh>,
    pub arm: Handle<Mesh>,
}

impl CharacterMeshes {
    pub fn create(meshes: &mut Assets<Mesh>) -> Self {
        Self {
            torso: meshes.add(build_mesh(|m| {
                m.add_box(Vec3::ZERO, Vec3::new(0.56, 0.72, 0.32), Color::WHITE);
            })),
            head: meshes.add(build_mesh(|m| {
                m.add_box(Vec3::ZERO, Vec3::new(0.30, 0.34, 0.30), Color::WHITE);
                // A nose reads clearly and gives the head an obvious facing.
                m.add_box(Vec3::new(0.0, -0.02, 0.17), Vec3::new(0.09, 0.09, 0.08), Color::WHITE);
            })),
            hair: meshes.add(build_mesh(|m| {
                m.add_box(Vec3::new(0.0, 0.10, -0.02), Vec3::new(0.33, 0.14, 0.33), Color::WHITE);
            })),
            // Legs and arms pivot around their top so a rotation swings them.
            leg: meshes.add(build_mesh(|m| {
                m.add_box(Vec3::new(0.0, -0.36, 0.0), Vec3::new(0.20, 0.72, 0.22), Color::WHITE);
            })),
            arm: meshes.add(build_mesh(|m| {
                m.add_box(Vec3::new(0.0, -0.30, 0.0), Vec3::new(0.14, 0.60, 0.16), Color::WHITE);
            })),
        }
    }

    /// Load the shared meshes from the world if present, creating and inserting
    /// them on first use.
    pub fn get_or_insert(meshes: &mut Assets<Mesh>, commands: &mut Commands, world: &World) -> Self {
        if let Some(existing) = world.get_resource::<CharacterMeshes>() {
            return existing.clone();
        }
        let created = Self::create(meshes);
        commands.insert_resource(created.clone());
        created
    }
}

/// Spawn a complete humanoid visual for `owner`.
pub fn spawn_humanoid(
    commands: &mut Commands,
    owner: Entity,
    origin: Vec3,
    meshes: &CharacterMeshes,
    torso_mat: Handle<StandardMaterial>,
    legs_mat: Handle<StandardMaterial>,
    skin_mat: Handle<StandardMaterial>,
    hair_mat: Handle<StandardMaterial>,
    shoes_mat: Handle<StandardMaterial>,
    marker: Option<PlayerVisual>,
) -> Vec<Entity> {
    let mut spawned = Vec::new();
    let parts: Vec<(Handle<Mesh>, Handle<StandardMaterial>, Vec3, Option<Limb>)> = vec![
        (meshes.torso.clone(), torso_mat.clone(), HumanoidRig::TORSO, Some(Limb::Torso)),
        (meshes.head.clone(), skin_mat.clone(), HumanoidRig::HEAD, Some(Limb::Head)),
        (meshes.hair.clone(), hair_mat, HumanoidRig::HEAD + Vec3::Y * 0.02, None),
        (meshes.leg.clone(), legs_mat.clone(), HumanoidRig::LEFT_LEG, Some(Limb::LeftLeg)),
        (meshes.leg.clone(), legs_mat.clone(), HumanoidRig::RIGHT_LEG, Some(Limb::RightLeg)),
        (meshes.arm.clone(), torso_mat.clone(), HumanoidRig::LEFT_ARM, Some(Limb::LeftArm)),
        (meshes.arm.clone(), torso_mat, HumanoidRig::RIGHT_ARM, Some(Limb::RightArm)),
    ];
    let _ = shoes_mat; // shoes are baked into the leg mesh

    for (mesh, material, offset, limb) in parts {
        let mut entity = commands.spawn((
            BodyPart {
                owner,
                offset,
                limb,
            },
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_translation(origin + offset),
            bevy::pbr::NotShadowCaster,
        ));
        if let Some(_p) = marker.as_ref() {
            entity.insert(PlayerVisual);
        }
        spawned.push(entity.id());
    }
    spawned
}

/// Copy each actor's transform onto its body parts and animate the limbs.
pub fn sync_body_parts(
    time: Res<Time>,
    actors: Query<(&Transform, &ActorBody)>,
    mut parts: Query<(&BodyPart, &mut Transform), Without<ActorBody>>,
) {
    let phase = time.elapsed_secs() * 8.6;
    for (part, mut transform) in &mut parts {
        let Ok((actor_transform, body)) = actors.get(part.owner) else {
            continue;
        };
        let yaw = actor_transform.rotation.to_euler(EulerRot::YXZ).0;
        let root_yaw = Quat::from_rotation_y(yaw);

        // Crouching lowers the whole silhouette.
        let crouch = if body.stance == Stance::Crouching {
            0.34
        } else {
            0.0
        };
        let offset = part.offset - Vec3::Y * crouch;
        transform.translation = actor_transform.translation + root_yaw * offset;

        let speed = Vec3::new(body.velocity.x, 0.0, body.velocity.z).length();
        let swing = (speed / 6.0).clamp(0.0, 1.0) * 0.75;
        let swing_phase = phase * (0.55 + speed * 0.09);

        let angle = match part.limb {
            Some(Limb::LeftLeg) => {
                if body.grounded {
                    swing_phase.sin() * swing
                } else {
                    0.4
                }
            }
            Some(Limb::RightLeg) => {
                if body.grounded {
                    -swing_phase.sin() * swing
                } else {
                    -0.3
                }
            }
            Some(Limb::LeftArm) => {
                if body.grounded {
                    -swing_phase.sin() * swing * 0.8
                } else {
                    -0.55
                }
            }
            Some(Limb::RightArm) => {
                if body.grounded {
                    swing_phase.sin() * swing * 0.8
                } else {
                    -0.55
                }
            }
            Some(Limb::Torso) => {
                if body.stance == Stance::Sprinting {
                    -0.12
                } else {
                    0.0
                }
            }
            Some(Limb::Head) => {
                if body.stance == Stance::Crouching {
                    -0.16
                } else {
                    0.0
                }
            }
            None => 0.0,
        };

        transform.rotation = root_yaw * Quat::from_rotation_x(angle);
    }
}

/// Unused marker offset kept for clarity of the character's facing convention:
/// the avatar's forward vector is +Z (yaw 0).
pub const AVATAR_FORWARD_YAW: f32 = 0.0;
const _: f32 = FRAC_PI_2;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actor_body_defaults_are_sane() {
        let body = ActorBody::default();
        assert!(body.radius > 0.0);
        assert!(body.height > 1.0);
        assert_eq!(body.stance, Stance::Standing);
    }

    #[test]
    fn humanoid_offsets_are_stacked() {
        assert!(HumanoidRig::HEAD.y > HumanoidRig::TORSO.y);
        assert!(HumanoidRig::LEFT_LEG.y < HumanoidRig::TORSO.y);
    }
}
