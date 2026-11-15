//! Core building blocks shared by every gameplay module.
//!
//! This module intentionally contains only *data* and small helper types so
//! that gameplay systems can depend on it without creating cycles.

use bevy::prelude::*;

// ---------------------------------------------------------------------------
// Rendering layers
// ---------------------------------------------------------------------------

/// Everything in the 3D world renders on the default layer (0).
pub const WORLD_LAYER: usize = 0;
/// Layer 1 is reserved for the player's first-person "view model" (held props).
pub const VIEW_MODEL_LAYER: usize = 1;

// ---------------------------------------------------------------------------
// Zone / access system
// ---------------------------------------------------------------------------

/// Areas of the resort. Every horizontal position maps onto exactly one zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ZoneId {
    Beach,
    Gardens,
    Lobby,
    Restaurant,
    PoolDeck,
    ServiceCorridor,
    SecurityWing,
    Penthouse,
}

impl ZoneId {
    /// Whether this zone is under a roof.
    ///
    /// Used for lighting: the camera adapts its exposure when the player steps
    /// inside, the way a real camera (and every modern game) does.
    pub fn is_interior(self) -> bool {
        matches!(
            self,
            ZoneId::Lobby
                | ZoneId::Restaurant
                | ZoneId::ServiceCorridor
                | ZoneId::SecurityWing
                | ZoneId::Penthouse
        )
    }

    /// Human readable zone name used by the HUD.
    pub fn display_name(self) -> &'static str {
        match self {
            ZoneId::Beach => "Sunset Beach",
            ZoneId::Gardens => "Palm Gardens",
            ZoneId::Lobby => "Grand Lobby",
            ZoneId::Restaurant => "Azure Restaurant",
            ZoneId::PoolDeck => "Infinity Pool Deck",
            ZoneId::ServiceCorridor => "Service Corridor",
            ZoneId::SecurityWing => "Security Wing",
            ZoneId::Penthouse => "Penthouse Terrace",
        }
    }

    /// Whether walking here without the proper disguise is trespassing.
    pub fn is_restricted(self) -> bool {
        matches!(self, ZoneId::SecurityWing | ZoneId::Penthouse)
    }

    /// Whether this zone counts as "staff only" (guests are not allowed).
    pub fn is_staff_only(self) -> bool {
        matches!(self, ZoneId::ServiceCorridor)
    }
}

/// Axis-aligned rectangle on the XZ plane, used for zone lookup and triggers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoneRect {
    pub min: Vec2,
    pub max: Vec2,
}

impl ZoneRect {
    pub fn new(min_x: f32, min_z: f32, max_x: f32, max_z: f32) -> Self {
        Self {
            min: Vec2::new(min_x.min(max_x), min_z.min(max_z)),
            max: Vec2::new(min_x.max(max_x), min_z.max(max_z)),
        }
    }

    pub fn center(&self) -> Vec2 {
        (self.min + self.max) * 0.5
    }

    pub fn size(&self) -> Vec2 {
        self.max - self.min
    }

    pub fn contains(&self, point: Vec2) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    /// Expand (or shrink, with negative amounts) the rectangle on all sides.
    pub fn inflated(&self, amount: f32) -> Self {
        Self {
            min: self.min - Vec2::splat(amount),
            max: self.max + Vec2::splat(amount),
        }
    }
}

/// A rectangular region of the map with gameplay meaning.
#[derive(Debug, Clone)]
pub struct ZoneDefinition {
    pub id: ZoneId,
    pub rect: ZoneRect,
    /// Higher priority zones win when rectangles overlap.
    pub priority: u8,
}

// ---------------------------------------------------------------------------
// Collision primitives
// ---------------------------------------------------------------------------

/// 2D circle used for actor collision on the ground plane.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Circle2 {
    pub center: Vec2,
    pub radius: f32,
}

/// Axis-aligned solid box on the XZ plane with a vertical extent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Block2 {
    pub min: Vec2,
    pub max: Vec2,
    pub floor: f32,
    pub height: f32,
}

impl Block2 {
    pub fn new(center: Vec3, size: Vec3) -> Self {
        let half = Vec2::new(size.x, size.z) * 0.5;
        let c = Vec2::new(center.x, center.z);
        Self {
            min: c - half,
            max: c + half,
            floor: center.y - size.y * 0.5,
            height: size.y,
        }
    }

    pub fn top(&self) -> f32 {
        self.floor + self.height
    }

    pub fn bottom(&self) -> f32 {
        self.floor
    }

    pub fn center2(&self) -> Vec2 {
        (self.min + self.max) * 0.5
    }

    pub fn contains_xz(&self, point: Vec2, margin: f32) -> bool {
        point.x >= self.min.x - margin
            && point.x <= self.max.x + margin
            && point.y >= self.min.y - margin
            && point.y <= self.max.y + margin
    }

    /// Closest point on the box (XZ) to `point`.
    pub fn closest_point(&self, point: Vec2) -> Vec2 {
        Vec2::new(
            point.x.clamp(self.min.x, self.max.x),
            point.y.clamp(self.min.y, self.max.y),
        )
    }

    /// Push a circle out of the box. Returns the corrected center, or `None`
    /// when the circle does not overlap.
    pub fn resolve_circle(&self, circle: Circle2) -> Option<Vec2> {
        let closest = self.closest_point(circle.center);
        let delta = circle.center - closest;
        let dist_sq = delta.length_squared();
        let r_sq = circle.radius * circle.radius;

        if dist_sq > r_sq {
            return None;
        }

        if dist_sq > 1e-6 {
            let dist = dist_sq.sqrt();
            Some(closest + delta / dist * circle.radius)
        } else {
            // Centre is inside the box: push out through the nearest face.
            let left = (circle.center.x - self.min.x).abs();
            let right = (self.max.x - circle.center.x).abs();
            let down = (circle.center.y - self.min.y).abs();
            let up = (self.max.y - circle.center.y).abs();
            let min = left.min(right).min(down).min(up);
            let center = if min == left {
                Vec2::new(self.min.x - circle.radius, circle.center.y)
            } else if min == right {
                Vec2::new(self.max.x + circle.radius, circle.center.y)
            } else if min == down {
                Vec2::new(circle.center.x, self.min.y - circle.radius)
            } else {
                Vec2::new(circle.center.x, self.max.y + circle.radius)
            };
            Some(center)
        }
    }

    /// True if the segment (a -> b) intersects the box in the XZ plane.
    pub fn segment_intersects(&self, a: Vec2, b: Vec2) -> bool {
        if self.contains_xz(a, 0.0) || self.contains_xz(b, 0.0) {
            return true;
        }
        let corners = [
            Vec2::new(self.min.x, self.min.y),
            Vec2::new(self.max.x, self.min.y),
            Vec2::new(self.max.x, self.max.y),
            Vec2::new(self.min.x, self.max.y),
        ];
        for i in 0..4 {
            if segments_intersect(a, b, corners[i], corners[(i + 1) % 4]) {
                return true;
            }
        }
        false
    }
}

/// Standard 2D segment intersection test.
pub fn segments_intersect(p1: Vec2, p2: Vec2, p3: Vec2, p4: Vec2) -> bool {
    fn cross(a: Vec2, b: Vec2) -> f32 {
        a.x * b.y - a.y * b.x
    }
    let d1 = cross(p4 - p3, p1 - p3);
    let d2 = cross(p4 - p3, p2 - p3);
    let d3 = cross(p2 - p1, p3 - p1);
    let d4 = cross(p2 - p1, p4 - p1);
    ((d1 > 0.0) != (d2 > 0.0)) && ((d3 > 0.0) != (d4 > 0.0))
}

/// Maths helpers shared between gameplay systems.
pub mod math {
    use bevy::prelude::*;

    /// Move `current` towards `target` at a maximum rate of `rate` per second.
    pub fn approach_f32(current: f32, target: f32, rate: f32, dt: f32) -> f32 {
        let delta = target - current;
        let step = rate * dt;
        if delta.abs() <= step {
            target
        } else {
            current + delta.signum() * step
        }
    }

    /// Move a vector towards a target with exponential smoothing that is
    /// stable regardless of the frame rate.
    pub fn smooth_vec3(current: Vec3, target: Vec3, halflife: f32, dt: f32) -> Vec3 {
        let factor = if halflife <= 0.0 {
            1.0
        } else {
            1.0 - (-std::f32::consts::LN_2 * dt / halflife).exp()
        };
        current.lerp(target, factor.clamp(0.0, 1.0))
    }

    /// Shortest signed difference between two angles (radians).
    pub fn angle_delta(from: f32, to: f32) -> f32 {
        let mut d = (to - from) % std::f32::consts::TAU;
        if d > std::f32::consts::PI {
            d -= std::f32::consts::TAU;
        } else if d < -std::f32::consts::PI {
            d += std::f32::consts::TAU;
        }
        d
    }

    /// Yaw angle (radians) that points from `from` towards `to` on the XZ plane.
    pub fn yaw_towards(from: Vec3, to: Vec3) -> f32 {
        let d = to - from;
        d.x.atan2(d.z)
    }

    /// Unit vector for a yaw angle where 0 rad points along +Z.
    pub fn yaw_to_dir(yaw: f32) -> Vec3 {
        Vec3::new(yaw.sin(), 0.0, yaw.cos())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zone_rect_contains_points() {
        let r = ZoneRect::new(-1.0, -2.0, 3.0, 4.0);
        assert!(r.contains(Vec2::new(0.0, 0.0)));
        assert!(r.contains(Vec2::new(-1.0, -2.0)));
        assert!(!r.contains(Vec2::new(3.5, 0.0)));
        assert_eq!(r.size(), Vec2::new(4.0, 6.0));
    }

    #[test]
    fn block2_pushes_circle_out() {
        let b = Block2::new(Vec3::new(0.0, 1.0, 0.0), Vec3::new(2.0, 2.0, 2.0));
        // Circle overlapping the +X face.
        let c = Circle2 {
            center: Vec2::new(1.2, 0.0),
            radius: 0.5,
        };
        let resolved = b.resolve_circle(c).expect("should overlap");
        assert!(resolved.x >= 1.5 - 1e-4);

        // Circle far away should not collide.
        let far = Circle2 {
            center: Vec2::new(10.0, 10.0),
            radius: 0.5,
        };
        assert!(b.resolve_circle(far).is_none());
    }

    #[test]
    fn angle_helpers_wrap_correctly() {
        use std::f32::consts::{PI, TAU};
        let d = math::angle_delta(PI - 0.1, -PI + 0.1);
        assert!((d - 0.2).abs() < 1e-4);
        assert!(math::angle_delta(0.0, TAU).abs() < 1e-4);
    }
}
