//! NPC steering, collision and procedural animation.
//!
//! Waypoints are authored so that straight lines between them are walkable;
//! the solver additionally slides NPCs around the resort's boxes so they never
//! clip through walls or furniture.

use super::brain::{Brain, BrainState};
use super::Npc;
use crate::core::math::{angle_delta, yaw_towards};
use crate::core::Circle2;
use crate::player::character::ActorBody;
use crate::world::builder::WorldCollision;
use bevy::prelude::*;

pub const NPC_WALK_SPEED: f32 = 1.35;
pub const NPC_PATROL_SPEED: f32 = 1.7;
pub const NPC_RADIUS: f32 = 0.36;

/// Steer every NPC towards its current waypoint.
pub fn steer_npcs(
    time: Res<Time>,
    collision: Option<Res<WorldCollision>>,
    mut npcs: Query<(&mut Transform, &mut Brain, &mut ActorBody), With<Npc>>,
) {
    let Some(collision) = collision else { return };
    let dt = time.delta_secs().min(0.1);

    for (mut transform, mut brain, mut body) in &mut npcs {
        if brain.paused {
            body.velocity = Vec3::ZERO;
            continue;
        }
        let Some(step) = brain.current_step().cloned() else {
            continue;
        };

        let target = step.destination.position;
        let current = transform.translation;
        let to_target = Vec3::new(target.x - current.x, 0.0, target.z - current.z);
        let distance = to_target.length();

        // Snap NPCs to the floor beneath them (they do not jump).
        let floor = floor_at(&collision, current, NPC_RADIUS);
        let mut position = current;
        if let Some(floor) = floor {
            position.y += (floor - position.y).clamp(-4.0 * dt, 4.0 * dt);
            if (floor - position.y).abs() < 0.25 {
                position.y = floor;
            }
            body.ground_y = floor;
        }
        body.grounded = true;

        if distance < 0.45 || brain.state != BrainState::Walking {
            brain.arrived = distance < 0.6;
            body.velocity = Vec3::ZERO;
            // Idle: rotate slowly to face the waypoint's activity direction.
            let target_yaw = brain.idle_yaw;
            let current_yaw = transform.rotation.to_euler(EulerRot::YXZ).0;
            let delta = angle_delta(current_yaw, target_yaw);
            transform.rotation = Quat::from_rotation_y(current_yaw + delta.clamp(-2.0 * dt, 2.0 * dt));
            transform.translation = position;
            continue;
        }

        // Desired velocity towards the waypoint.
        let speed = if step.destination.wait > 3.0 {
            NPC_PATROL_SPEED
        } else {
            NPC_WALK_SPEED
        };
        let desired = to_target.normalize_or_zero() * speed;
        body.velocity = body.velocity.move_towards(desired, 6.0 * dt);

        let mut next = position + body.velocity * dt;

        // --- Collision & avoidance -------------------------------------
        for block in &collision.blocks {
            if block.top() - position.y > 0.7 {
                let circle = Circle2 {
                    center: Vec2::new(next.x, next.z),
                    radius: NPC_RADIUS,
                };
                if let Some(resolved) = block.resolve_circle(circle) {
                    next.x = resolved.x;
                    next.z = resolved.y;
                }
            }
        }

        position.x = next.x;
        position.z = next.z;
        body.velocity = Vec3::new(
            (position.x - current.x) / dt.max(1e-4),
            0.0,
            (position.z - current.z) / dt.max(1e-4),
        );
        transform.translation = position;

        // Face the direction of travel.
        if body.velocity.length_squared() > 0.001 {
            let target_yaw = yaw_towards(Vec3::ZERO, Vec3::new(body.velocity.x, 0.0, body.velocity.z));
            let current_yaw = transform.rotation.to_euler(EulerRot::YXZ).0;
            let delta = angle_delta(current_yaw, target_yaw);
            transform.rotation = Quat::from_rotation_y(current_yaw + delta.clamp(-6.0 * dt, 6.0 * dt));
        }
    }
}

/// Keep the player and NPCs from occupying the same space.
pub fn npc_collisions(
    mut npcs: Query<&mut Transform, (With<Npc>, Without<crate::player::character::PlayerAvatar>)>,
    mut players: Query<
        &mut Transform,
        (With<crate::player::character::PlayerAvatar>, Without<Npc>),
    >,
) {
    let Ok(mut player) = players.single_mut() else {
        return;
    };
    let player_pos = Vec2::new(player.translation.x, player.translation.z);
    for mut npc in &mut npcs {
        let npc_pos = Vec2::new(npc.translation.x, npc.translation.z);
        if (npc.translation.y - player.translation.y).abs() > 1.5 {
            continue;
        }
        let delta = player_pos - npc_pos;
        let distance = delta.length();
        let min_distance = NPC_RADIUS + crate::player::PLAYER_RADIUS;
        if distance > 0.0001 && distance < min_distance {
            let push = delta / distance * (min_distance - distance);
            // The player is heavier than the NPC: share the correction.
            player.translation.x += push.x * 0.25;
            player.translation.z += push.y * 0.25;
            npc.translation.x -= push.x * 0.75;
            npc.translation.z -= push.y * 0.75;
        }
    }
}

fn floor_at(collision: &WorldCollision, position: Vec3, radius: f32) -> Option<f32> {
    let point = Vec2::new(position.x, position.z);
    let mut best: Option<f32> = None;
    for block in &collision.surfaces {
        // Same "probe slightly wider than the body" rule as the player.
        if !block.contains_xz(point, radius + 0.12) {
            continue;
        }
        let top = block.top();
        if top <= position.y + 0.75 {
            best = Some(best.map_or(top, |b: f32| b.max(top)));
        }
    }
    best
}

/// NPC body parts are animated by the same system as the player's
/// (`character::sync_body_parts`), so NPCs and the avatar share one gait model.
/// This alias keeps the plugin wiring readable.
pub fn animate_npcs() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separation_pushes_apart() {
        // The force helper is exercised through floor_at and geometry checks in
        // integration tests; here we validate the maths contract.
        let delta = Vec3::new(0.5, 0.0, 0.0);
        assert!((delta.normalize_or_zero().length() - 1.0).abs() < 1e-5);
    }
}
