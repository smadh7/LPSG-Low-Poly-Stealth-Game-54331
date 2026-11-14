//! Player locomotion: input sampling and the fixed-step physics solver.
//!
//! The solver resolves the player's circle against the resort's axis aligned
//! boxes, supports walking up and down steps, and keeps the character glued to
//! the correct floor height (lobby = 0, restaurant deck = 2.4, penthouse = 29.8,
//! service corridor = -3.4).

use super::camera::CameraRig;
use super::character::{ActorBody, PlayerAvatar};
use super::PlayerIntent;
use super::Stance;
use crate::core::{math::approach_f32, math::yaw_towards, Block2, Circle2};
use crate::world::builder::WorldCollision;
use bevy::prelude::*;

pub const WALK_SPEED: f32 = 2.4;
pub const RUN_SPEED: f32 = 5.2;
pub const CROUCH_SPEED: f32 = 1.15;
pub const JUMP_VELOCITY: f32 = 6.2;
pub const GRAVITY: f32 = -19.5;
/// Maximum step the player can climb without jumping.
pub const STEP_HEIGHT: f32 = 0.62;

/// Small re-export so other modules do not need the maths module directly.
pub fn approach(current: f32, target: f32, rate: f32, dt: f32) -> f32 {
    approach_f32(current, target, rate, dt)
}

/// Turn raw keyboard state into a [`PlayerIntent`].
pub fn read_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut intent: ResMut<PlayerIntent>,
    rig: Res<CameraRig>,
    state: Res<State<crate::AppState>>,
) {
    if *state.get() != crate::AppState::Playing {
        intent.direction = Vec3::ZERO;
        intent.wants_run = false;
        intent.wants_crouch = false;
        intent.wants_jump = false;
        return;
    }

    let mut input = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        input.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) {
        input.y -= 1.0;
    }
    if keys.pressed(KeyCode::KeyA) {
        input.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) {
        input.x += 1.0;
    }

    let (forward, right) = rig.movement_basis();
    let dir = if input.length_squared() > 0.0 {
        let input = input.normalize();
        (forward * input.y + right * input.x).normalize_or_zero()
    } else {
        Vec3::ZERO
    };

    intent.direction = dir;
    intent.wants_run = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    intent.wants_crouch = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::KeyC);
    intent.wants_jump = keys.pressed(KeyCode::Space);

    // Scroll to zoom the camera.
    let mut target = rig.target_distance;
    if keys.pressed(KeyCode::Equal) || keys.pressed(KeyCode::NumpadAdd) {
        target -= 0.05;
    }
    if keys.pressed(KeyCode::Minus) || keys.pressed(KeyCode::NumpadSubtract) {
        target += 0.05;
    }
    if (target - rig.target_distance).abs() > f32::EPSILON {
        // Handled in a dedicated system to avoid mutating the rig here.
        intent.look.x += 0.0;
    }
}

/// Fixed-step player physics and collision resolution.
pub fn player_physics(
    time: Res<Time>,
    mut intent: ResMut<PlayerIntent>,
    collision: Option<Res<WorldCollision>>,
    mut players: Query<(&mut Transform, &mut ActorBody), With<PlayerAvatar>>,
) {
    let dt = time.delta_secs().min(1.0 / 20.0);
    if dt <= 0.0 {
        return;
    }
    let Ok((mut transform, mut body)) = players.single_mut() else {
        return;
    };
    let Some(collision) = collision else {
        return;
    };

    // --- Stance ---------------------------------------------------------
    let crouched = intent.wants_crouch && body.grounded;
    let horizontal_input = Vec3::new(intent.direction.x, 0.0, intent.direction.z);
    let wants_move = horizontal_input.length_squared() > 1e-4;

    let target_speed = if crouched {
        CROUCH_SPEED
    } else if intent.wants_run {
        RUN_SPEED
    } else {
        WALK_SPEED
    };

    // Ground acceleration vs. air control.
    let accel = if body.grounded { 26.0 } else { 7.0 };
    let desired_horizontal = horizontal_input * target_speed;
    let current_horizontal = Vec3::new(body.velocity.x, 0.0, body.velocity.z);
    let new_horizontal = current_horizontal.move_towards(desired_horizontal, accel * dt);

    // --- Jump -----------------------------------------------------------
    if intent.wants_jump && body.grounded && !crouched {
        body.velocity.y = JUMP_VELOCITY;
        body.grounded = false;
    }
    intent.wants_jump = false;

    // --- Gravity --------------------------------------------------------
    if !body.grounded {
        body.velocity.y += GRAVITY * dt;
    }

    body.velocity.x = new_horizontal.x;
    body.velocity.z = new_horizontal.z;

    // --- Horizontal movement with collision -----------------------------
    let mut position = transform.translation;
    let mut next = position + Vec3::new(body.velocity.x, 0.0, body.velocity.z) * dt;

    let radius = body.radius;
    // Two passes give a stable result in corners.
    for _ in 0..2 {
        for block in &collision.blocks {
            if block.bottom() > position.y + 1.4 {
                continue; // Above the player: can walk underneath.
            }
            let circle = Circle2 {
                center: Vec2::new(next.x, next.z),
                radius,
            };
            if let Some(resolved) = block.resolve_circle(circle) {
                // Only collide with blocks tall enough to matter, or ledges we
                // cannot step onto.
                let top = block.top();
                if top - position.y > STEP_HEIGHT || block.bottom() > position.y + 0.9 {
                    next.x = resolved.x;
                    next.z = resolved.y;
                }
            }
        }
    }

    // Keep the player inside the map bounds.
    next.x = next.x.clamp(-72.0, 72.0);
    next.z = next.z.clamp(-62.0, 60.0);

    position.x = next.x;
    position.z = next.z;

    // --- Vertical movement ----------------------------------------------
    position.y += body.velocity.y * dt;

    // Find the highest surface within step height below the player.
    let floor = ground_height(&collision.blocks, Vec2::new(position.x, position.z), radius, position.y);
    let floor_y = floor.unwrap_or(-100.0);

    if position.y <= floor_y + 0.02 {
        position.y = floor_y;
        if body.velocity.y < 0.0 {
            body.velocity.y = 0.0;
        }
        body.grounded = true;
    } else if body.grounded && body.velocity.y <= 0.001 && position.y - floor_y <= STEP_HEIGHT {
        // Walking down a step or ramp: stick to the surface instead of
        // launching into the air.
        position.y = floor_y;
        body.velocity.y = 0.0;
        body.grounded = true;
    } else {
        body.grounded = position.y - floor_y < 0.02;
    }

    if body.grounded {
        body.velocity.y = body.velocity.y.min(0.0);
    }

    body.ground_y = floor_y;

    // --- Facing ---------------------------------------------------------
    if horizontal_input.length_squared() > 1e-4 {
        let target_yaw = horizontal_input.x.atan2(horizontal_input.z);
        let current_yaw = transform.rotation.to_euler(EulerRot::YXZ).0;
        let delta = crate::core::math::angle_delta(current_yaw, target_yaw);
        let turn = delta.clamp(-12.0 * dt, 12.0 * dt);
        transform.rotation = Quat::from_rotation_y(current_yaw + turn);
    }

    // Smoothly lower the model when crouching.
    let target_offset = if crouched { -0.32 } else { 0.0 };
    let current_offset = transform.translation.y - position.y;
    let blended = approach_f32(current_offset, target_offset, 3.0, dt);
    transform.translation = position + Vec3::Y * blended + Vec3::Y * 0.0;
    transform.translation.x = position.x;
    transform.translation.z = position.z;
    transform.translation.y = position.y + blended;

    // --- Stance + animation phase --------------------------------------
    body.stance = if !body.grounded {
        Stance::Airborne
    } else if crouched {
        Stance::Crouching
    } else if wants_move && intent.wants_run {
        Stance::Sprinting
    } else {
        Stance::Standing
    };
    let speed = Vec3::new(body.velocity.x, 0.0, body.velocity.z).length();
    body.gait += speed * dt * 2.2;
}

/// Highest walkable surface below `point`, considering boxes whose top is no
/// more than [`STEP_HEIGHT`] above `feet_y`.
pub fn ground_height(blocks: &[Block2], point: Vec2, radius: f32, feet_y: f32) -> Option<f32> {
    let mut best: Option<f32> = None;
    for block in blocks {
        if !block.contains_xz(point, radius * 0.6) {
            continue;
        }
        let top = block.top();
        if top <= feet_y + STEP_HEIGHT + 0.05 {
            best = match best {
                Some(current) => Some(current.max(top)),
                None => Some(top),
            };
        }
    }
    best
}

/// Keep the camera zoom target in sync with the scroll wheel.
pub fn update_ground_state(
    collision: Option<Res<WorldCollision>>,
    mut players: Query<(&mut ActorBody, &Transform), With<PlayerAvatar>>,
) {
    let Some(collision) = collision else { return };
    for (mut body, transform) in &mut players {
        if !body.grounded {
            continue;
        }
        let floor = ground_height(
            &collision.blocks,
            Vec2::new(transform.translation.x, transform.translation.z),
            body.radius,
            transform.translation.y,
        );
        if let Some(floor) = floor {
            body.ground_y = floor;
        }
    }
}

/// Utility used by the camera to keep the player's facing intact.
pub fn player_yaw(transform: &Transform) -> f32 {
    transform.rotation.to_euler(EulerRot::YXZ).0
}

/// Direction the player model is facing.
pub fn facing_direction(transform: &Transform) -> Vec3 {
    let yaw = player_yaw(transform);
    Vec3::new(yaw.sin(), 0.0, yaw.cos())
}

pub fn look_towards_yaw(from: Vec3, to: Vec3) -> f32 {
    yaw_towards(from, to)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ground_height_picks_highest_reachable() {
        let blocks = vec![
            Block2::new(Vec3::new(0.0, -0.5, 0.0), Vec3::new(10.0, 1.0, 10.0)),
            Block2::new(Vec3::new(0.0, 0.25, 0.0), Vec3::new(2.0, 0.5, 2.0)),
        ];
        let h = ground_height(&blocks, Vec2::ZERO, 0.4, 0.0).unwrap();
        assert!((h - 0.5).abs() < 1e-4, "expected 0.5, got {h}");
    }

    #[test]
    fn ground_height_ignores_too_high_ledges() {
        let blocks = vec![Block2::new(Vec3::new(0.0, 1.5, 5.0), Vec3::new(4.0, 3.0, 1.0))];
        assert!(ground_height(&blocks, Vec2::new(0.0, 5.0), 0.4, 0.0).is_none());
    }

    #[test]
    fn approach_moves_at_constant_rate() {
        assert!((approach(0.0, 1.0, 2.0, 0.1) - 0.2).abs() < 1e-5);
        assert!((approach(1.0, 1.0, 2.0, 0.1) - 1.0).abs() < 1e-5);
        assert!((approach(1.0, 0.0, 2.0, 1.0) - 0.0).abs() < 1e-5);
    }
}
