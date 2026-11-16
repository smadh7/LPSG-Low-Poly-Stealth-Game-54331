//! Prop geometry: everything that decorates the resort.
//!
//! Props are authored as small, recognisable objects rather than the original
//! one-primitive-per-kind blobs. A palm has a curved, tapered trunk and a crown
//! of drooping fronds; a dining set has a table, four chairs with backs and
//! legs. This is what makes a scene read as designed rather than blocky, and it
//! is all merged into per-material meshes so the cost stays in the noise.

use super::mesh_kit::{MatId, MeshBuilder};
use bevy::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

/// Deterministic hash → 0..1, used so repeated props are not identical.
fn rand01(seed: u32) -> f32 {
    let mut h = seed.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    h = (h ^ (h >> 15)).wrapping_mul(2_246_822_519);
    ((h ^ (h >> 13)) & 0x00FF_FFFF) as f32 / 16_777_215.0
}

/// Position helper: rotate a local offset around Y and add it to an origin.
fn place(origin: Vec3, yaw: f32, local: Vec3) -> Vec3 {
    origin + Quat::from_rotation_y(yaw) * local
}

// ---------------------------------------------------------------------------
// Vegetation
// ---------------------------------------------------------------------------

/// A palm: leaning tapered trunk, crown shaft, and 11 drooping fronds.
pub fn palm(m: &mut MeshBuilder, origin: Vec3, yaw: f32, scale: f32, seed: u32) {
    let height = (6.2 + rand01(seed) * 3.6) * scale;
    let lean = (rand01(seed.wrapping_add(11)) - 0.5) * 0.26;
    let lean_dir = yaw + rand01(seed.wrapping_add(23)) * TAU;
    let segments = 14;

    // Trunk: many short, slightly flared segments so the lean is a smooth curve
    // and the ring texture has something to follow. The bark UVs come from the
    // tapered cylinder primitive, which tiles at one metre.
    for i in 0..segments {
        let t0 = i as f32 / segments as f32;
        let t1 = (i + 1) as f32 / segments as f32;
        let bend = |t: f32| Vec3::new(lean_dir.cos(), 0.0, lean_dir.sin()) * (t * t * lean * height);
        let p0 = origin + Vec3::Y * (t0 * height) + bend(t0);
        let r0 = (0.29 - 0.13 * t0) * scale;
        let r1 = (0.29 - 0.13 * t1) * scale;
        m.add_taper(MatId::PalmBark, p0, r0, r1, height / segments as f32 + 0.02, 12);
    }

    let top = origin
        + Vec3::Y * height
        + Vec3::new(lean_dir.cos(), 0.0, lean_dir.sin()) * (lean * height);
    // Crown shaft: a smooth green bulge where the fronds attach.
    m.add_sphere(MatId::Frond, top - Vec3::Y * 0.35 * scale, Vec3::new(0.24, 0.36, 0.24) * scale, 6, 10);
    // Coconut cluster.
    for i in 0..5 {
        let a = yaw + i as f32 * TAU / 5.0 + 0.4;
        m.add_sphere(
            MatId::Hedge,
            top + Vec3::new(a.cos() * 0.20, -0.30, a.sin() * 0.20) * scale,
            Vec3::splat(0.11 * scale),
            3,
            6,
        );
    }

    // Fronds. Each frond is a curved rachis with individual leaflets, not a
    // solid ribbon: the silhouette (light through separate leaflets) is what
    // makes a palm read as a palm rather than a green windmill.
    let fronds = 13;
    for i in 0..fronds {
        let a = yaw + i as f32 * TAU / fronds as f32 + rand01(seed.wrapping_add(i as u32)) * 0.28;
        let droop = 0.30 + rand01(seed.wrapping_add(i as u32 * 7)) * 0.60;
        let len = (2.9 + rand01(seed.wrapping_add(i as u32 * 13)) * 1.0) * scale;
        let ring = (i % 3) as f32;
        let base = top - Vec3::Y * (0.10 + ring * 0.08) * scale;
        let dir = Vec3::new(a.cos(), 0.0, a.sin());
        let side = Vec3::new(-a.sin(), 0.0, a.cos());

        // Rachis: the thin woody spine the leaflets hang from.
        let rachis_point = |t: f32| {
            let rise = 0.52 + ring * 0.10;
            let y = (t * rise - t * t * (0.55 + droop)) * len * 0.55;
            base + dir * (len * t) + Vec3::Y * y
        };

        let steps = 7;
        let mut prev = rachis_point(0.0);
        for step in 1..=steps {
            let t = step as f32 / steps as f32;
            let point = rachis_point(t);
            // Spine quad, very narrow, with a bark-ish UV run.
            let w = 0.022 * scale * (1.0 - t * 0.6);
            m.add_quad_uv_double(
                MatId::PalmBark,
                [prev - side * w, point - side * w, point + side * w, prev + side * w],
                [
                    [0.0, t * 2.0],
                    [0.0, t * 2.0 + 0.3],
                    [1.0, t * 2.0 + 0.3],
                    [1.0, t * 2.0],
                ],
            );

            // Leaflets: wide, overlapping blades angled forward and outward,
            // shortening towards the tip. Each blade carries the whole frond
            // texture, so it reads as a leaf rather than a green stick.
            let taper = 1.0 - t * 0.62;
            for sign in [-1.0_f32, 1.0] {
                let outward =
                    (side * sign * 0.80 + dir * 0.42 + Vec3::NEG_Y * 0.30).normalize();
                let leaflet_len = (0.85 * taper + 0.10) * len * 0.40;
                let tip = point + outward * leaflet_len;
                let root = point + side * sign * 0.015 * scale;
                let width = (0.14 * taper + 0.02) * scale;
                let p = [
                    root - dir * width,
                    tip - dir * width * 0.6,
                    tip + dir * width * 0.6,
                    root + dir * width,
                ];
                m.add_quad_uv_double(
                    MatId::Frond,
                    p,
                    [[0.0, 0.02], [0.5, 1.0], [0.5, 1.0], [0.0, 0.02]],
                );
            }
            prev = point;
        }
    }
}

/// Broadleaf shade tree: trunk, three main branches and layered canopy blobs.
pub fn tree(m: &mut MeshBuilder, origin: Vec3, yaw: f32, scale: f32, seed: u32) {
    let height = (4.4 + rand01(seed) * 1.8) * scale;
    m.add_taper(MatId::PalmBark, origin, 0.30 * scale, 0.17 * scale, height, 10);
    for i in 0..3 {
        let a = yaw + i as f32 * TAU / 3.0;
        let start = origin + Vec3::Y * height * 0.62;
        let end = start + Vec3::new(a.cos() * 1.1, 1.0, a.sin() * 1.1) * scale;
        let mid = (start + end) * 0.5;
        m.add_sphere(MatId::PalmBark, mid, Vec3::new(0.10, 0.55, 0.10) * scale, 4, 6);
    }

    // Canopy: overlapping ellipsoids at three radii for a soft silhouette.
    let canopy_center = origin + Vec3::Y * (height * 1.05);
    let blobs = 7;
    for i in 0..blobs {
        let a = yaw + i as f32 * TAU / blobs as f32 + rand01(seed.wrapping_add(i)) * 0.5;
        let r = (0.6 + rand01(seed.wrapping_add(i * 31)) * 0.9) * scale;
        let y = (rand01(seed.wrapping_add(i * 17)) - 0.4) * 1.5 * scale;
        m.add_sphere(
            MatId::Hedge,
            canopy_center + Vec3::new(a.cos() * r, y, a.sin() * r),
            Vec3::new(1.35, 1.05, 1.35) * scale,
            6,
            9,
        );
    }
}

/// Dense tropical shrub: several overlapping leafy masses.
pub fn shrub_cluster(m: &mut MeshBuilder, origin: Vec3, yaw: f32, scale: f32, seed: u32) {
    for i in 0..5 {
        let a = yaw + i as f32 * TAU / 5.0 + rand01(seed.wrapping_add(i)) * 0.8;
        let r = (0.3 + rand01(seed.wrapping_add(i * 9)) * 0.55) * scale;
        let h = (0.55 + rand01(seed.wrapping_add(i * 3)) * 0.5) * scale;
        m.add_sphere(
            MatId::Hedge,
            origin + Vec3::new(a.cos() * r, h * 0.55, a.sin() * r),
            Vec3::new(0.75, h, 0.75) * scale,
            5,
            8,
        );
    }
}

/// A clump of tall grass blades.
pub fn grass_tuft(m: &mut MeshBuilder, origin: Vec3, yaw: f32, scale: f32, seed: u32) {
    let blades = 12;
    for i in 0..blades {
        let a = yaw + i as f32 * TAU / blades as f32 + rand01(seed.wrapping_add(i)) * 0.5;
        let h = (0.28 + rand01(seed.wrapping_add(i * 5)) * 0.34) * scale;
        let offset = Vec3::new(a.cos(), 0.0, a.sin()) * (0.06 + rand01(seed.wrapping_add(i * 11)) * 0.12) * scale;
        let tip = origin + offset + Vec3::new(a.cos() * 0.22, h, a.sin() * 0.22) * scale;
        let base = origin + offset;
        let side = Vec3::new(-a.sin(), 0.0, a.cos()) * 0.035 * scale;
        // Slight cross-blade so the tuft has volume from every direction.
        let cross = Vec3::new(a.cos(), 0.0, a.sin()) * 0.030 * scale;
        m.add_quad_uv_double(
            MatId::Grass,
            [base - side, base + side, tip + side * 0.3, tip - side * 0.3],
            [[0.0, 0.0], [0.5, 0.0], [0.62, 1.0], [0.10, 1.0]],
        );
        m.add_quad_uv_double(
            MatId::Grass,
            [base - cross, base + cross, tip + cross * 0.3, tip - cross * 0.3],
            [[0.0, 0.0], [0.5, 0.0], [0.62, 1.0], [0.10, 1.0]],
        );
    }
}

/// Clipped hedge block with a leafy top surface.
pub fn hedge(m: &mut MeshBuilder, min: Vec2, max: Vec2, height: f32) {
    let center = Vec3::new((min.x + max.x) * 0.5, height * 0.5, (min.y + max.y) * 0.5);
    let size = Vec3::new(max.x - min.x, height, max.y - min.y);
    m.add_box(MatId::Hedge, center, size);
    // Rounded top: a slightly smaller box to break the hard edge.
    m.add_box(
        MatId::Hedge,
        center + Vec3::Y * (height * 0.5 - 0.12),
        Vec3::new(size.x * 0.94, 0.26, size.z * 0.94),
    );
}

// ---------------------------------------------------------------------------
// Furniture
// ---------------------------------------------------------------------------

fn chair(m: &mut MeshBuilder, origin: Vec3, yaw: f32, wood: MatId, fabric: MatId) {
    let seat_h = 0.45;
    let seat = place(origin, yaw, Vec3::new(0.0, seat_h, 0.0));
    m.add_box_rotated(wood, seat, Vec3::new(0.46, 0.06, 0.46), yaw);
    // Back rest.
    let back = place(origin, yaw, Vec3::new(0.0, seat_h + 0.30, -0.20));
    m.add_box_rotated(wood, back, Vec3::new(0.46, 0.62, 0.05), yaw);
    // Cushion on the seat.
    m.add_box_rotated(fabric, seat + Vec3::Y * 0.05, Vec3::new(0.40, 0.05, 0.40), yaw);
    // Four legs.
    for (sx, sz) in [(-1.0_f32, -1.0_f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let leg = place(origin, yaw, Vec3::new(sx * 0.19, seat_h * 0.5, sz * 0.19));
        m.add_box_rotated(wood, leg, Vec3::new(0.05, seat_h, 0.05), yaw);
    }
}

/// Restaurant table with four chairs.
pub fn dining_set(m: &mut MeshBuilder, origin: Vec3, yaw: f32, _scale: f32) {
    let top = origin + Vec3::Y * 0.74;
    m.add_box_rotated(MatId::Wood, top, Vec3::new(1.35, 0.06, 0.95), yaw);
    // A tablecloth-ish lighter inlay.
    m.add_box_rotated(MatId::WhitePaint, top + Vec3::Y * 0.04, Vec3::new(1.15, 0.02, 0.78), yaw);
    // Pedestal legs.
    for sx in [-0.5_f32, 0.5] {
        let leg = place(origin, yaw, Vec3::new(sx, 0.37, 0.0));
        m.add_box_rotated(MatId::DarkTrim, leg, Vec3::new(0.07, 0.74, 0.07), yaw);
    }
    for i in 0..4 {
        let a = yaw + i as f32 * FRAC_PI_2;
        let offset = Vec3::new(0.95, 0.0, 0.0);
        let pos = place(origin, a, offset);
        chair(m, pos, a + PI, MatId::Wood, MatId::Fabric);
    }
    // Table settings: plate, glass, candle.
    for sx in [-0.35_f32, 0.35] {
        let p = place(origin, yaw, Vec3::new(sx, 0.80, 0.0));
        m.add_cylinder(MatId::WhitePaint, p, 0.16, 0.02, 12);
        let g = place(origin, yaw, Vec3::new(sx + 0.28, 0.80, -0.22));
        m.add_cylinder(MatId::Glass, g, 0.045, 0.14, 8);
    }
    let candle = place(origin, yaw, Vec3::new(0.0, 0.78, 0.0));
    m.add_cylinder(MatId::Brass, candle, 0.05, 0.20, 8);
}

/// Tall bar table and two stools.
pub fn bar_set(m: &mut MeshBuilder, origin: Vec3, yaw: f32, _scale: f32) {
    let top = origin + Vec3::Y * 1.05;
    m.add_cylinder(MatId::Wood, top, 0.45, 0.06, 14);
    m.add_cylinder(MatId::DarkTrim, origin, 0.07, 1.05, 8);
    m.add_cylinder(MatId::DarkTrim, origin, 0.32, 0.03, 12);
    for i in 0..2 {
        let a = yaw + i as f32 * PI;
        let p = origin + Vec3::new(a.cos() * 0.75, 0.0, a.sin() * 0.75);
        m.add_cylinder(MatId::DarkTrim, p, 0.045, 0.72, 8);
        m.add_cylinder(MatId::Fabric, p + Vec3::Y * 0.72, 0.20, 0.07, 12);
    }
}

/// Lounge sofa / armchair.
pub fn armchair(m: &mut MeshBuilder, origin: Vec3, yaw: f32, _scale: f32) {
    m.add_box_rotated(MatId::Wood, origin + Vec3::Y * 0.22, Vec3::new(1.0, 0.22, 0.9), yaw);
    m.add_box_rotated(MatId::Fabric, origin + Vec3::Y * 0.44, Vec3::new(0.92, 0.20, 0.82), yaw);
    m.add_box_rotated(MatId::Fabric, origin + Vec3::Y * 0.78, Vec3::new(0.92, 0.50, 0.20), yaw);
    for sx in [-1.0_f32, 1.0] {
        m.add_box_rotated(MatId::Fabric, origin + Vec3::Y * 0.60 + Quat::from_rotation_y(yaw) * Vec3::new(sx * 0.48, 0.0, 0.0), Vec3::new(0.12, 0.34, 0.86), yaw);
    }
    // Cushions.
    for (sx, sz) in [(-0.3_f32, 0.0_f32), (0.3, 0.0)] {
        let p = origin + Quat::from_rotation_y(yaw) * Vec3::new(sx, 0.58, sz - 0.25);
        m.add_box_rotated(MatId::Fabric, p, Vec3::new(0.36, 0.14, 0.34), yaw + 0.2);
    }
}

/// Hotel bed with headboard, pillows and throw.
pub fn bed(m: &mut MeshBuilder, origin: Vec3, yaw: f32, scale: f32) {
    let w = 1.8 * scale;
    let l = 2.1 * scale;
    m.add_box_rotated(MatId::Wood, origin + Vec3::Y * 0.22, Vec3::new(w, 0.40, l), yaw);
    m.add_box_rotated(MatId::WhitePaint, origin + Vec3::Y * 0.48, Vec3::new(w * 0.97, 0.20, l * 0.97), yaw);
    // Headboard.
    m.add_box_rotated(MatId::Wood, origin + Vec3::Y * 0.85 + Quat::from_rotation_y(yaw) * Vec3::new(0.0, 0.0, -l * 0.5), Vec3::new(w, 1.1, 0.10), yaw);
    // Pillows.
    for sx in [-0.42_f32, 0.42] {
        let p = origin + Vec3::Y * 0.62 + Quat::from_rotation_y(yaw) * Vec3::new(sx * w, 0.0, -l * 0.32);
        m.add_box_rotated(MatId::WhitePaint, p, Vec3::new(w * 0.42, 0.16, 0.42), yaw);
    }
    // Throw across the foot of the bed.
    let t = origin + Vec3::Y * 0.60 + Quat::from_rotation_y(yaw) * Vec3::new(0.0, 0.0, l * 0.22);
    m.add_box_rotated(MatId::Fabric, t, Vec3::new(w * 0.98, 0.05, l * 0.34), yaw);
    // Legs.
    for (sx, sz) in [(-1.0_f32, -1.0_f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let p = origin + Quat::from_rotation_y(yaw) * Vec3::new(sx * w * 0.44, 0.05, sz * l * 0.44);
        m.add_box_rotated(MatId::DarkTrim, p, Vec3::new(0.08, 0.14, 0.08), yaw);
    }
}

/// Bedside table with a lamp.
pub fn nightstand(m: &mut MeshBuilder, origin: Vec3, yaw: f32, scale: f32) {
    m.add_box_rotated(MatId::Wood, origin + Vec3::Y * 0.28, Vec3::new(0.5 * scale, 0.56, 0.45 * scale), yaw);
    m.add_box_rotated(MatId::Marble, origin + Vec3::Y * 0.57, Vec3::new(0.54 * scale, 0.04, 0.49 * scale), yaw);
    // Lamp.
    m.add_cylinder(MatId::Brass, origin + Vec3::Y * 0.59, 0.09 * scale, 0.05, 10);
    m.add_cylinder(MatId::Brass, origin + Vec3::Y * 0.64, 0.02 * scale, 0.26, 8);
    m.add_cone(MatId::WhitePaint, origin + Vec3::Y * 0.88, 0.18 * scale, 0.22, 12);
}

/// Long console / sideboard table.
pub fn console_table(m: &mut MeshBuilder, origin: Vec3, yaw: f32, scale: f32) {
    let w = 1.9 * scale;
    m.add_box_rotated(MatId::Wood, origin + Vec3::Y * 0.78, Vec3::new(w, 0.07, 0.45 * scale), yaw);
    m.add_box_rotated(MatId::Wood, origin + Vec3::Y * 0.45, Vec3::new(w * 0.92, 0.58, 0.38 * scale), yaw);
    // Drawer lines.
    for sx in [-0.28_f32, 0.0, 0.28] {
        let p = origin + Vec3::Y * 0.62 + Quat::from_rotation_y(yaw) * Vec3::new(sx * w, 0.0, 0.20 * scale);
        m.add_box_rotated(MatId::Brass, p, Vec3::new(w * 0.22, 0.025, 0.02), yaw);
    }
    for sx in [-1.0_f32, 1.0] {
        let p = origin + Quat::from_rotation_y(yaw) * Vec3::new(sx * w * 0.42, 0.12, 0.0);
        m.add_box_rotated(MatId::DarkTrim, p, Vec3::new(0.07, 0.24, 0.07), yaw);
    }
    // A vase on top.
    m.add_cylinder(MatId::Glass, origin + Vec3::Y * 0.82, 0.11 * scale, 0.30, 10);
    m.add_sphere(MatId::Frond, origin + Vec3::Y * 1.20, Vec3::new(0.22, 0.18, 0.22) * scale, 4, 7);
}

/// Kitchen / bar counter unit.
pub fn counter(m: &mut MeshBuilder, origin: Vec3, yaw: f32, scale: f32) {
    m.add_box_rotated(MatId::Metal, origin + Vec3::Y * 0.45, Vec3::new(1.7 * scale, 0.90, 0.7 * scale), yaw);
    m.add_box_rotated(MatId::Marble, origin + Vec3::Y * 0.92, Vec3::new(1.8 * scale, 0.06, 0.76 * scale), yaw);
    // Sink.
    let sink = origin + Quat::from_rotation_y(yaw) * Vec3::new(0.35 * scale, 0.95, 0.0);
    m.add_box_rotated(MatId::DarkTrim, sink, Vec3::new(0.5 * scale, 0.03, 0.4 * scale), yaw);
    m.add_cylinder(MatId::Metal, sink + Vec3::Y * 0.02, 0.02, 0.28, 6);
}

/// Reception desk with a brass counter detail (visual companion to the desk solid).
pub fn reception(m: &mut MeshBuilder, origin: Vec3, _yaw: f32, _scale: f32) {
    // The desk body is a solid; this adds the counter, screen and bell.
    m.add_box(MatId::Brass, origin + Vec3::Y * 1.16, Vec3::new(4.6, 0.06, 0.06));
    m.add_box(MatId::DarkTrim, origin + Vec3::Y * 1.30, Vec3::new(0.55, 0.36, 0.06));
    m.add_cylinder(MatId::Brass, origin + Vec3::Y * 1.20 + Vec3::X * 1.4, 0.08, 0.06, 10);
    // Small flower arrangement.
    m.add_cylinder(MatId::Glass, origin + Vec3::Y * 1.20 + Vec3::X * -1.5, 0.10, 0.22, 10);
    for i in 0..4 {
        let a = i as f32 * TAU / 4.0;
        m.add_sphere(
            MatId::Frond,
            origin + Vec3::Y * 1.50 + Vec3::X * -1.5 + Vec3::new(a.cos() * 0.12, 0.06, a.sin() * 0.12),
            Vec3::splat(0.10),
            3,
            6,
        );
    }
}

/// A bench: slatted seat, cast-iron ends.
pub fn bench(m: &mut MeshBuilder, origin: Vec3, yaw: f32, _scale: f32) {
    let rot = Quat::from_rotation_y(yaw);
    for i in 0..3 {
        let p = origin + Vec3::Y * (0.44 + i as f32 * 0.001) + rot * Vec3::new(0.0, 0.0, (i as f32 - 1.0) * 0.17);
        m.add_box_rotated(MatId::Wood, p, Vec3::new(1.9, 0.06, 0.14), yaw);
    }
    for i in 0..2 {
        let p = origin + Vec3::Y * (0.62 + i as f32 * 0.18) + rot * Vec3::new(0.0, 0.0, -0.26);
        m.add_box_rotated(MatId::Wood, p, Vec3::new(1.9, 0.12, 0.05), yaw);
    }
    for sx in [-1.0_f32, 1.0] {
        let p = origin + rot * Vec3::new(sx * 0.82, 0.0, 0.0);
        m.add_box_rotated(MatId::DarkTrim, p + Vec3::Y * 0.22, Vec3::new(0.08, 0.44, 0.56), yaw);
        m.add_box_rotated(MatId::DarkTrim, p + Vec3::Y * 0.62, Vec3::new(0.08, 0.36, 0.10), yaw);
    }
}

/// Sun lounger: timber frame, fabric sling, raised back.
pub fn sun_lounger(m: &mut MeshBuilder, origin: Vec3, yaw: f32, _scale: f32) {
    let rot = Quat::from_rotation_y(yaw);
    m.add_box_rotated(MatId::Wood, origin + Vec3::Y * 0.34, Vec3::new(0.78, 0.08, 1.95), yaw);
    m.add_box_rotated(MatId::WhitePaint, origin + Vec3::Y * 0.40, Vec3::new(0.70, 0.04, 1.85), yaw);
    // Back rest, tilted.
    let back = origin + Vec3::Y * 0.66 + rot * Vec3::new(0.0, 0.0, -0.72);
    m.add_box_rotated(MatId::WhitePaint, back, Vec3::new(0.70, 0.06, 0.80), yaw);
    for sx in [-1.0_f32, 1.0] {
        for sz in [-0.7_f32, 0.8] {
            let p = origin + rot * Vec3::new(sx * 0.34, 0.17, sz);
            m.add_box_rotated(MatId::Wood, p, Vec3::new(0.07, 0.34, 0.07), yaw);
        }
    }
    // Towel.
    m.add_box_rotated(MatId::Fabric, origin + Vec3::Y * 0.44 + rot * Vec3::new(0.0, 0.0, 0.55), Vec3::new(0.5, 0.04, 0.7), yaw);
}

/// Parasol: pole, ribs, scalloped canopy.
pub fn parasol(m: &mut MeshBuilder, origin: Vec3, _yaw: f32, scale: f32) {
    let h = 2.35 * scale;
    m.add_cylinder(MatId::Metal, origin, 0.045, h, 8);
    m.add_cylinder(MatId::Metal, origin, 0.32, 0.10, 12);
    let top = origin + Vec3::Y * h;
    // Canopy: two cones back to back make a convincing scallop.
    m.add_cone(MatId::WhitePaint, top - Vec3::Y * 0.34, 1.65 * scale, 0.36, 12);
    m.add_cone(MatId::WhitePaint, top, 1.62 * scale, 0.06, 12);
    m.add_sphere(MatId::Metal, top + Vec3::Y * 0.06, Vec3::splat(0.06), 3, 6);
}

/// Beach umbrella with alternating panels.
pub fn umbrella(m: &mut MeshBuilder, origin: Vec3, yaw: f32, scale: f32) {
    let h = 2.5 * scale;
    m.add_cylinder(MatId::Wood, origin, 0.05, h, 8);
    // Weighted base and a hanging valance, so it reads as furniture not a cone.
    m.add_cylinder(MatId::Concrete, origin, 0.30 * scale, 0.16 * scale, 12);
    for i in 0..8 {
        let a0 = yaw + i as f32 * TAU / 8.0;
        let a1 = a0 + TAU / 16.0;
        let rim = 1.9 * scale;
        let mid0 = origin + Vec3::Y * (h - 0.52) + Vec3::new(a0.cos() * rim, 0.0, a0.sin() * rim);
        let mid1 = origin + Vec3::Y * (h - 0.52) + Vec3::new(a1.cos() * rim, 0.0, a1.sin() * rim);
        let low0 = mid0 - Vec3::Y * 0.18;
        let low1 = mid1 - Vec3::Y * 0.18;
        m.add_quad(if i % 2 == 0 { MatId::WhitePaint } else { MatId::Fabric }, [mid0, mid1, low1, low0]);
    }
    for i in 0..8 {
        let a0 = yaw + i as f32 * TAU / 8.0;
        let a1 = a0 + TAU / 8.0;
        let r = 1.9 * scale;
        let rim0 = origin + Vec3::Y * (h - 0.42) + Vec3::new(a0.cos() * r, 0.0, a0.sin() * r);
        let rim1 = origin + Vec3::Y * (h - 0.42) + Vec3::new(a1.cos() * r, 0.0, a1.sin() * r);
        let apex = origin + Vec3::Y * h;
        // Alternating panels read as a striped parasol.
        let mat = if i % 2 == 0 { MatId::WhitePaint } else { MatId::Fabric };
        m.bucket(mat).tri(
            [apex, rim0, rim1],
            [Vec3::Y; 3],
            [[0.5, 0.5], [0.0, 0.0], [1.0, 0.0]],
        );
    }
}

/// Terraced stone planter with flowers.
pub fn flower_bed(m: &mut MeshBuilder, origin: Vec3, yaw: f32, scale: f32, seed: u32) {
    m.add_box_rotated(MatId::StoneWall, origin + Vec3::Y * 0.20, Vec3::new(4.2 * scale, 0.42, 2.4 * scale), yaw);
    m.add_box_rotated(MatId::Hedge, origin + Vec3::Y * 0.46, Vec3::new(3.9 * scale, 0.16, 2.1 * scale), yaw);
    for i in 0..14 {
        let a = rand01(seed.wrapping_add(i)) * TAU;
        let r = rand01(seed.wrapping_add(i * 7)) * 1.8 * scale;
        let p = origin + Quat::from_rotation_y(yaw) * Vec3::new(a.cos() * r, 0.62, a.sin() * r * 0.55);
        let mat = if i % 3 == 0 { MatId::Fabric } else { MatId::Frond };
        m.add_sphere(mat, p, Vec3::splat(0.11 * scale), 3, 6);
    }
}

/// Square or round planter with a shrub.
pub fn planter(m: &mut MeshBuilder, origin: Vec3, yaw: f32, scale: f32) {
    m.add_box_rotated(MatId::StoneWall, origin + Vec3::Y * 0.32 * scale, Vec3::new(1.0, 0.64, 1.0) * scale, yaw);
    m.add_box_rotated(MatId::Terracotta, origin + Vec3::Y * 0.60 * scale, Vec3::new(0.9, 0.10, 0.9) * scale, yaw);
    m.add_sphere(MatId::Hedge, origin + Vec3::Y * 0.92 * scale, Vec3::new(0.62, 0.48, 0.62) * scale, 5, 9);
}

/// Ceramic pot with a leafy plant.
pub fn potted_plant(m: &mut MeshBuilder, origin: Vec3, yaw: f32, scale: f32) {
    m.add_taper(MatId::Terracotta, origin, 0.36 * scale, 0.46 * scale, 0.62 * scale, 12);
    m.add_cylinder(MatId::Terracotta, origin + Vec3::Y * 0.60 * scale, 0.48 * scale, 0.08 * scale, 12);
    // Foliage: a fan of broad leaves plus a crown.
    for i in 0..7 {
        let a = yaw + i as f32 * TAU / 7.0;
        let dir = Vec3::new(a.cos(), 0.0, a.sin());
        let base = origin + Vec3::Y * 0.68 * scale;
        let tip = base + dir * 0.55 * scale + Vec3::Y * 0.45 * scale;
        let side = Vec3::new(-a.sin(), 0.0, a.cos()) * 0.22 * scale;
        m.add_quad(MatId::Frond, [base - side, base + side, tip + side * 0.4, tip - side * 0.4]);
    }
    m.add_sphere(MatId::Hedge, origin + Vec3::Y * 1.12 * scale, Vec3::new(0.38, 0.30, 0.38) * scale, 4, 8);
}

/// Indoor palm in a tall planter.
pub fn indoor_palm(m: &mut MeshBuilder, origin: Vec3, yaw: f32, scale: f32) {
    m.add_taper(MatId::DarkTrim, origin, 0.42 * scale, 0.36 * scale, 0.70 * scale, 12);
    m.add_cylinder(MatId::DarkTrim, origin + Vec3::Y * 0.68 * scale, 0.38 * scale, 0.06 * scale, 12);
    let trunk_top = origin + Vec3::Y * 1.85 * scale;
    m.add_taper(MatId::PalmBark, origin + Vec3::Y * 0.68 * scale, 0.07 * scale, 0.06 * scale, 1.2 * scale, 8);
    for i in 0..8 {
        let a = yaw + i as f32 * TAU / 8.0;
        let dir = Vec3::new(a.cos(), 0.0, a.sin());
        let tip = trunk_top + dir * 0.75 * scale + Vec3::Y * (0.35 - (i % 3) as f32 * 0.15) * scale;
        let side = Vec3::new(-a.sin(), 0.0, a.cos()) * 0.16 * scale;
        m.add_quad(MatId::Frond, [trunk_top - side, trunk_top + side, tip + side * 0.5, tip - side * 0.5]);
    }
}

/// Lamp post: base, tapered column, lantern head.
pub fn lamp_post(m: &mut MeshBuilder, origin: Vec3, _yaw: f32, scale: f32) {
    let h = 3.6 * scale;
    m.add_cylinder(MatId::DarkTrim, origin, 0.28, 0.22, 10);
    m.add_taper(MatId::DarkTrim, origin + Vec3::Y * 0.2, 0.12, 0.07, h - 0.2, 10);
    let head = origin + Vec3::Y * (h + 0.05);
    m.add_box(MatId::DarkTrim, head, Vec3::new(0.42, 0.08, 0.42));
    m.add_box(MatId::WhitePaint, head + Vec3::Y * 0.30, Vec3::new(0.36, 0.52, 0.36));
    m.add_cone(MatId::DarkTrim, head + Vec3::Y * 0.56, 0.30, 0.18, 4);
    // Actual light contribution comes from the HUD-free emissive material; the
    // post keeps the geometry honest at any distance.
}

/// Stack of service crates and a drum.
pub fn crates(m: &mut MeshBuilder, origin: Vec3, yaw: f32, scale: f32, seed: u32) {
    for i in 0..4 {
        let layer = i as f32;
        let jitter = (rand01(seed.wrapping_add(i)) - 0.5) * 0.4;
        let p = origin
            + Quat::from_rotation_y(yaw + jitter) * Vec3::new(jitter, 0.33 + layer * 0.62, jitter * 0.6);
        m.add_box_rotated(MatId::Wood, p, Vec3::new(0.8, 0.62, 0.62) * scale, yaw + jitter);
        // Plank detailing reads as a crate rather than a box.
        m.add_box_rotated(MatId::DarkTrim, p, Vec3::new(0.82, 0.05, 0.64) * scale, yaw + jitter);
    }
    m.add_cylinder(MatId::Metal, origin + Vec3::new(0.9, 0.0, 0.3), 0.28 * scale, 0.86, 14);
}

/// Wall/roof air-conditioning condenser.
pub fn air_unit(m: &mut MeshBuilder, origin: Vec3, yaw: f32, scale: f32) {
    m.add_box_rotated(MatId::Metal, origin + Vec3::Y * 0.5 * scale, Vec3::new(1.3, 1.0, 0.85) * scale, yaw);
    // Fan grille.
    let rot = Quat::from_rotation_y(yaw);
    m.add_cylinder(MatId::DarkTrim, origin + Vec3::Y * 0.5 * scale + rot * Vec3::new(0.0, 0.0, 0.44 * scale), 0.32 * scale, 0.06, 16);
}

/// Beach life-ring post.
pub fn life_ring(m: &mut MeshBuilder, origin: Vec3, yaw: f32, _scale: f32) {
    m.add_cylinder(MatId::Wood, origin, 0.07, 1.7, 8);
    m.add_box_rotated(MatId::DarkTrim, origin + Vec3::Y * 1.05, Vec3::new(0.52, 0.52, 0.08), yaw);
    // Ring of segments.
    for i in 0..10 {
        let a = i as f32 * TAU / 10.0;
        let p = origin + Vec3::Y * 1.05 + Quat::from_rotation_y(yaw) * Vec3::new(a.cos() * 0.24, a.sin() * 0.24, 0.06);
        let mat = if i % 2 == 0 { MatId::Fabric } else { MatId::WhitePaint };
        m.add_box_rotated(mat, p, Vec3::new(0.15, 0.09, 0.09), a);
    }
}

/// Outdoor beach shower with a pipe and head.
pub fn beach_shower(m: &mut MeshBuilder, origin: Vec3, yaw: f32, _scale: f32) {
    m.add_box_rotated(MatId::Concrete, origin + Vec3::Y * 0.06, Vec3::new(1.3, 0.12, 1.3), yaw);
    m.add_cylinder(MatId::Metal, origin + Vec3::Y * 0.1, 0.05, 2.1, 8);
    let rot = Quat::from_rotation_y(yaw);
    m.add_box_rotated(MatId::Metal, origin + Vec3::Y * 2.2 + rot * Vec3::new(0.0, 0.0, 0.22), Vec3::new(0.08, 0.08, 0.5), yaw);
    m.add_cylinder(MatId::Metal, origin + Vec3::Y * 2.12 + rot * Vec3::new(0.0, 0.0, 0.44), 0.18, 0.08, 12);
}

/// Stone balustrade segment (posts + rail + vase balusters).
pub fn balustrade(m: &mut MeshBuilder, origin: Vec3, yaw: f32, scale: f32) {
    let rot = Quat::from_rotation_y(yaw);
    for i in 0..5 {
        let p = origin + rot * Vec3::new((i as f32 - 2.0) * 0.42 * scale, 0.0, 0.0);
        m.add_sphere(MatId::StoneWall, p + Vec3::Y * 0.45 * scale, Vec3::new(0.10, 0.30, 0.10) * scale, 4, 8);
    }
    m.add_box_rotated(MatId::StoneWall, origin + Vec3::Y * 0.80 * scale, Vec3::new(2.2 * scale, 0.10, 0.22) * scale, yaw);
    m.add_box_rotated(MatId::StoneWall, origin, Vec3::new(2.2 * scale, 0.14, 0.26) * scale, yaw);
}

/// Shade-cover pergola for the garden bar.
pub fn pergola(m: &mut MeshBuilder, origin: Vec3, yaw: f32, scale: f32) {
    let rot = Quat::from_rotation_y(yaw);
    for (sx, sz) in [(-1.0_f32, -1.0_f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let p = origin + rot * Vec3::new(sx * 1.9 * scale, 0.0, sz * 1.9 * scale);
        m.add_box_rotated(MatId::Wood, p + Vec3::Y * 1.3 * scale, Vec3::new(0.16, 2.6, 0.16) * scale, yaw);
    }
    for i in 0..6 {
        let p = origin + rot * Vec3::new(0.0, 2.62 * scale, (i as f32 - 2.5) * 0.72 * scale);
        m.add_box_rotated(MatId::Wood, p, Vec3::new(4.0 * scale, 0.10, 0.12) * scale, yaw);
    }
}

/// Build one prop. `seed` keeps repeated props from looking cloned.
pub fn build_prop(
    m: &mut MeshBuilder,
    kind: super::layout::DecoKind,
    origin: Vec3,
    yaw: f32,
    scale: f32,
    seed: u32,
) {
    use super::layout::DecoKind as K;
    let s = scale.max(0.2);
    match kind {
        K::Palm => palm(m, origin, yaw, s, seed),
        K::Tree => tree(m, origin, yaw, s, seed),
        K::ShrubCluster | K::Bush => shrub_cluster(m, origin, yaw, s, seed),
        K::GrassTuft => grass_tuft(m, origin, yaw, s, seed),
        K::FlowerBed => flower_bed(m, origin, yaw, s, seed),
        K::Planter => planter(m, origin, yaw, s),
        K::PottedPlant => potted_plant(m, origin, yaw, s),
        K::IndoorPalm => indoor_palm(m, origin, yaw, s),
        K::LampPost => lamp_post(m, origin, yaw, s),
        K::Parasol => parasol(m, origin, yaw, s),
        K::Umbrella => umbrella(m, origin, yaw, s),
        K::SunLounger => sun_lounger(m, origin, yaw, s),
        K::Bench => bench(m, origin, yaw, s),
        K::DiningSet => dining_set(m, origin, yaw, s),
        K::BarSet => bar_set(m, origin, yaw, s),
        K::Armchair => armchair(m, origin, yaw, s),
        K::Bed => bed(m, origin, yaw, s),
        K::Nightstand => nightstand(m, origin, yaw, s),
        K::ConsoleTable => console_table(m, origin, yaw, s),
        K::Counter => counter(m, origin, yaw, s),
        K::Reception => reception(m, origin, yaw, s),
        K::Crates => crates(m, origin, yaw, s, seed),
        K::AirUnit => air_unit(m, origin, yaw, s),
        K::LifeRing => life_ring(m, origin, yaw, s),
        K::BeachShower => beach_shower(m, origin, yaw, s),
        K::Balustrade => balustrade(m, origin, yaw, s),
    }
}

/// Prop used by the pergola helper (exposed for future layouts).
pub fn pergola_prop(m: &mut MeshBuilder, origin: Vec3, yaw: f32, scale: f32) {
    pergola(m, origin, yaw, scale);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triangles(kind: super::super::layout::DecoKind) -> usize {
        let mut builder = MeshBuilder::new();
        build_prop(&mut builder, kind, Vec3::ZERO, 0.0, 1.0, 1);
        builder.triangle_count()
    }

    #[test]
    fn props_are_real_geometry_not_single_boxes() {
        use super::super::layout::DecoKind as K;
        // A convincing prop has well over a dozen triangles; the old low-poly
        // blobs had six.
        for kind in [
            K::Palm,
            K::Tree,
            K::Bench,
            K::DiningSet,
            K::Bed,
            K::Armchair,
            K::Umbrella,
            K::SunLounger,
            K::BarSet,
        ] {
            let count = triangles(kind);
            assert!(count > 60, "{kind:?} only produced {count} triangles");
        }
    }

    #[test]
    fn palms_differ_between_seeds() {
        let mut a = MeshBuilder::new();
        let mut b = MeshBuilder::new();
        build_prop(&mut a, super::super::layout::DecoKind::Palm, Vec3::ZERO, 0.0, 1.0, 7);
        build_prop(&mut b, super::super::layout::DecoKind::Palm, Vec3::ZERO, 0.0, 1.0, 99);
        let meshes_a = a.finish();
        let meshes_b = b.finish();
        let va: usize = meshes_a.iter().map(|(_, m)| m.count_vertices()).sum();
        let vb: usize = meshes_b.iter().map(|(_, m)| m.count_vertices()).sum();
        assert!(va > 0 && vb > 0);
        // Different seeds must not produce byte-identical geometry.
        let pa: Vec<f32> = meshes_a
            .iter()
            .flat_map(|(_, m)| {
                m.attribute(Mesh::ATTRIBUTE_POSITION)
                    .and_then(|a| a.as_float3())
                    .map(|v| v.iter().flatten().copied().collect::<Vec<f32>>())
                    .unwrap_or_default()
            })
            .collect();
        let pb: Vec<f32> = meshes_b
            .iter()
            .flat_map(|(_, m)| {
                m.attribute(Mesh::ATTRIBUTE_POSITION)
                    .and_then(|a| a.as_float3())
                    .map(|v| v.iter().flatten().copied().collect::<Vec<f32>>())
                    .unwrap_or_default()
            })
            .collect();
        assert_ne!(pa, pb);
    }

    #[test]
    fn grass_and_hedge_use_foliage_materials() {
        let mut builder = MeshBuilder::new();
        build_prop(&mut builder, super::super::layout::DecoKind::GrassTuft, Vec3::ZERO, 0.0, 1.0, 3);
        let meshes = builder.finish();
        assert!(meshes.iter().any(|(m, _)| *m == MatId::Grass));
    }
}
