//! Third-person orbit camera.
//!
//! The rig keeps a yaw/pitch pair, a smoothed distance and a smoothed pivot so
//! the camera glides behind the player instead of snapping. Mouse movement is
//! sampled from Bevy's `AccumulatedMouseMotion` resource which is already
//! frame-rate independent.

use super::character::{ActorBody, PlayerAvatar};
use super::{Stance, CROUCH_EYE_HEIGHT, EYE_HEIGHT};
use crate::core::math::{smooth_vec3, yaw_to_dir};
use crate::world::builder::WorldCollision;
use bevy::{input::mouse::AccumulatedMouseMotion, prelude::*, window::CursorGrabMode};
use std::f32::consts::{FRAC_PI_2, FRAC_PI_3};

/// Physical camera entity.
#[derive(Component)]
pub struct ThirdPersonCamera;

/// Camera state that other systems (interaction, HUD) can read.
#[derive(Resource, Debug)]
pub struct CameraRig {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub target_distance: f32,
    pub pivot: Vec3,
    pub sensitivity: f32,
    /// Smoothed camera position, exposed for ray casts.
    pub position: Vec3,
    pub forward: Vec3,
}

impl Default for CameraRig {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: -0.22,
            distance: 4.6,
            target_distance: 4.6,
            pivot: Vec3::new(0.0, 1.6, 0.0),
            sensitivity: 0.0025,
            position: Vec3::new(0.0, 3.0, -8.0),
            forward: Vec3::NEG_Z,
        }
    }
}

impl CameraRig {
    /// Direction the camera is looking.
    pub fn look_direction(&self) -> Vec3 {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        Vec3::new(sin_yaw * cos_pitch, sin_pitch, cos_yaw * cos_pitch)
    }

    /// Camera-relative movement basis on the ground plane.
    ///
    /// `forward` points away from the camera, `right` is 90 degrees clockwise
    /// when viewed from above so that `W`, `D` map to intuitive directions.
    pub fn movement_basis(&self) -> (Vec3, Vec3) {
        let forward = Vec3::new(self.yaw.sin(), 0.0, self.yaw.cos());
        let right = Vec3::new(forward.z, 0.0, -forward.x);
        (forward, right)
    }
}

/// Read mouse motion and update the orbit angles.
pub fn camera_look(
    mouse_motion: Res<AccumulatedMouseMotion>,
    windows: Query<&Window>,
    mut rig: ResMut<CameraRig>,
    mut intent: ResMut<super::PlayerIntent>,
) {
    let delta = mouse_motion.delta;
    if delta == Vec2::ZERO {
        return;
    }
    // Only rotate while the cursor is captured, otherwise the mouse would
    // spin the camera while the player uses menus.
    let captured = windows
        .iter()
        .any(|w| w.cursor_options.grab_mode != CursorGrabMode::None);
    if !captured {
        return;
    }

    rig.yaw += delta.x * rig.sensitivity;
    rig.pitch = (rig.pitch - delta.y * rig.sensitivity)
        .clamp(-FRAC_PI_3 - 0.15, FRAC_PI_2 - 0.35);
    rig.yaw = rig.yaw.rem_euclid(std::f32::consts::TAU);
    intent.look += delta;
}

/// Overrides the follow camera entirely.
///
/// Used by the screenshot harness (and available for future cinematic beats):
/// when present, `camera_follow` leaves the camera exactly where the lock says
/// it should be. Owning the pose through the camera's own resource is
/// order-independent, unlike writing the transform from another system.
#[derive(Resource, Debug, Clone, Copy)]
pub struct CameraLock(pub Vec3, pub f32, pub f32);

/// Move the camera towards its ideal position, avoiding geometry.
pub fn camera_follow(
    time: Res<Time>,
    mut rig: ResMut<CameraRig>,
    collision: Option<Res<WorldCollision>>,
    lock: Option<Res<CameraLock>>,
    mut camera: Query<&mut Transform, (With<ThirdPersonCamera>, Without<PlayerAvatar>)>,
    player: Query<(&Transform, &ActorBody), With<PlayerAvatar>>,
) {
    if let Some(lock) = lock.as_ref() {
        if let Ok(mut transform) = camera.single_mut() {
            transform.translation = lock.0;
            transform.rotation = Quat::from_euler(EulerRot::YXZ, lock.1, lock.2, 0.0);
        }
        return;
    }
    let Ok((player_transform, body)) = player.single() else {
        return;
    };
    let dt = time.delta_secs().min(0.1);
    let eye = match body.stance {
        Stance::Crouching => CROUCH_EYE_HEIGHT,
        _ => EYE_HEIGHT,
    };

    // Pivot follows the player smoothly but the vertical response is a little
    // slower so stairs feel less jarring.
    let desired_pivot = player_transform.translation + Vec3::Y * (eye - 0.15);
    rig.pivot = smooth_vec3(rig.pivot, desired_pivot, 0.055, dt);

    rig.distance = super::movement::approach(rig.distance, rig.target_distance, 14.0, dt);

    // Pull the camera in when something is between the pivot and the camera.
    let mut wanted_distance = rig.distance;
    let dir = -rig.look_direction();
    if let Some(collision) = collision {
        if let Some(hit) = raycast_blocks(
            rig.pivot,
            dir,
            rig.distance + 0.45,
            &collision.blocks,
        ) {
            wanted_distance = (hit - 0.35).clamp(0.65, rig.distance);
        }
    }

    let desired = rig.pivot + dir * wanted_distance;
    let smoothed = smooth_vec3(rig.position, desired, 0.045, dt);
    rig.position = smoothed;

    rig.forward = rig.look_direction();
    let look_at = rig.pivot;
    if let Ok(mut transform) = camera.single_mut() {
        transform.translation = smoothed;
        transform.look_at(look_at, Vec3::Y);
    }
}

/// Cheap ray march against the world's axis aligned boxes.
///
/// Returns the distance to the first hit, if any. The step size is small
/// enough for camera occlusion and interaction checks, and the resort only has
/// a few dozen boxes to test per step.
pub fn raycast_blocks(
    origin: Vec3,
    dir: Vec3,
    max_distance: f32,
    blocks: &[crate::core::Block2],
) -> Option<f32> {
    let steps = ((max_distance / 0.15).ceil() as usize).max(1);
    for step in 1..=steps {
        let t = max_distance * step as f32 / steps as f32;
        let p = origin + dir * t;
        let p2 = Vec2::new(p.x, p.z);
        for block in blocks {
            if block.contains_xz(p2, 0.0) && p.y >= block.bottom() && p.y <= block.top() + 0.05 {
                return Some(t);
            }
        }
    }
    None
}

/// Smoothly look the camera towards a yaw angle (used by scripted moments).
pub fn face_direction(current: &mut f32, target: f32, dt: f32) {
    let delta = crate::core::math::angle_delta(*current, target);
    *current += delta * (1.0 - (-12.0 * dt).exp());
}

/// Camera forward projected onto the ground for movement.
pub fn ground_forward(rig: &CameraRig) -> Vec3 {
    yaw_to_dir(rig.yaw).normalize_or_zero()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Block2;

    #[test]
    fn rig_look_direction_is_normalised() {
        let rig = CameraRig::default();
        assert!((rig.look_direction().length() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn movement_basis_is_orthogonal() {
        let rig = CameraRig {
            yaw: 1.234,
            ..CameraRig::default()
        };
        let (f, r) = rig.movement_basis();
        assert!(f.dot(r).abs() < 1e-5);
        assert!((f.length() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn raycast_hits_a_wall() {
        let blocks = vec![Block2::new(Vec3::new(0.0, 1.5, 5.0), Vec3::new(4.0, 3.0, 1.0))];
        let hit = raycast_blocks(Vec3::new(0.0, 1.5, 0.0), Vec3::Z, 10.0, &blocks);
        assert!(hit.is_some());
        let h = hit.unwrap();
        assert!(h > 4.0 && h < 5.5, "unexpected hit distance {h}");
        assert!(raycast_blocks(Vec3::new(0.0, 1.5, 0.0), Vec3::NEG_Z, 10.0, &blocks).is_none());
    }
}
