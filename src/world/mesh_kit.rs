//! Low-poly mesh generation.
//!
//! The project contains **no external model files**: every prop, building and
//! character in the game is built from a handful of procedurally generated,
//! flat-shaded meshes.  That keeps the game fully offline and the repository
//! small while still giving the world a cohesive low-poly look.

use bevy::{
    asset::RenderAssetUsages,
    math::Vec3,
    prelude::*,
    render::mesh::{Indices, Mesh, PrimitiveTopology},
};
use std::f32::consts::{FRAC_PI_2, PI};

/// Winding helper: pushes a triangle with per-face normals (flat shading).
fn push_face(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
    verts: [[f32; 3]; 3],
    color: [f32; 4],
) {
    let a = Vec3::from(verts[0]);
    let b = Vec3::from(verts[1]);
    let c = Vec3::from(verts[2]);
    let n = (b - a).cross(c - a).normalize_or_zero();
    let base = positions.len() as u32;
    for v in verts {
        positions.push(v);
        normals.push(n.to_array());
        colors.push(color);
    }
    indices.extend_from_slice(&[base, base + 1, base + 2]);
}

/// Begin an empty vertex-coloured mesh.
pub fn new_mesh() -> (Mesh, MeshAccumulator) {
    (
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        ),
        MeshAccumulator::default(),
    )
}

/// Mutable geometry buffer used while building a mesh.
#[derive(Default)]
pub struct MeshAccumulator {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub colors: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
}

impl MeshAccumulator {
    /// Add a triangle fan from a list of vertices (assumed in order).
    pub fn add_polygon(&mut self, verts: &[Vec3], color: Color) {
        let c = color.to_linear().to_f32_array();
        for i in 1..verts.len().saturating_sub(1) {
            push_face(
                &mut self.positions,
                &mut self.normals,
                &mut self.colors,
                &mut self.indices,
                [verts[0].to_array(), verts[i].to_array(), verts[i + 1].to_array()],
                c,
            );
        }
    }

    /// Add an axis aligned box. `center` is the box centre.
    pub fn add_box(&mut self, center: Vec3, size: Vec3, color: Color) {
        let h = size * 0.5;
        let c = color.to_linear().to_f32_array();
        let corner = |sx: f32, sy: f32, sz: f32| {
            Vec3::new(center.x + sx * h.x, center.y + sy * h.y, center.z + sz * h.z)
        };
        let p = [
            corner(-1.0, -1.0, -1.0),
            corner(1.0, -1.0, -1.0),
            corner(1.0, -1.0, 1.0),
            corner(-1.0, -1.0, 1.0),
            corner(-1.0, 1.0, -1.0),
            corner(1.0, 1.0, -1.0),
            corner(1.0, 1.0, 1.0),
            corner(-1.0, 1.0, 1.0),
        ];
        let faces: [([usize; 4], [f32; 3]); 6] = [
            ([4, 5, 6, 7], [0.0, 1.0, 0.0]),  // top
            ([0, 3, 2, 1], [0.0, -1.0, 0.0]), // bottom
            ([0, 1, 5, 4], [0.0, 0.0, -1.0]), // -Z
            ([2, 3, 7, 6], [0.0, 0.0, 1.0]),  // +Z
            ([3, 0, 4, 7], [-1.0, 0.0, 0.0]), // -X
            ([1, 2, 6, 5], [1.0, 0.0, 0.0]),  // +X
        ];
        for (quad, n) in faces {
            let base = self.positions.len() as u32;
            for idx in quad {
                self.positions.push(p[idx].to_array());
                self.normals.push(n);
                self.colors.push(c);
            }
            // Ensure counter-clockwise winding for the outward normal.
            let a = p[quad[0]];
            let b = p[quad[1]];
            let d = p[quad[2]];
            let face_n = (b - a).cross(d - a).normalize_or_zero();
            if face_n.dot(Vec3::from(n)) >= 0.0 {
                self.indices
                    .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
            } else {
                self.indices
                    .extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
            }
        }
    }

    /// Box with the given bounds (min corner, max corner).
    pub fn add_box_bounds(&mut self, min: Vec3, max: Vec3, color: Color) {
        let center = (min + max) * 0.5;
        let size = max - min;
        self.add_box(center, size, color);
    }

    /// Add a box rotated by `yaw` radians around Y.
    pub fn add_box_rotated(&mut self, center: Vec3, size: Vec3, yaw: f32, color: Color) {
        let before = self.positions.len();
        self.add_box(Vec3::ZERO, size, color);
        let rot = Quat::from_rotation_y(yaw);
        for i in before..self.positions.len() {
            let p = Vec3::from(self.positions[i]);
            self.positions[i] = (rot * p + center).to_array();
        }
        for i in before..self.normals.len() {
            let n = Vec3::from(self.normals[i]);
            self.normals[i] = (rot * n).to_array();
        }
    }

    /// Add an n-sided cylinder (low poly by nature).
    pub fn add_cylinder(&mut self, bottom: Vec3, radius: f32, height: f32, sides: usize, color: Color) {
        let c = color.to_linear().to_f32_array();
        let top = bottom + Vec3::Y * height;
        for i in 0..sides {
            let a0 = i as f32 / sides as f32 * std::f32::consts::TAU;
            let a1 = (i + 1) as f32 / sides as f32 * std::f32::consts::TAU;
            let p0 = bottom + Vec3::new(a0.cos() * radius, 0.0, a0.sin() * radius);
            let p1 = bottom + Vec3::new(a1.cos() * radius, 0.0, a1.sin() * radius);
            let p2 = p1 + Vec3::Y * height;
            let p3 = p0 + Vec3::Y * height;
            push_face(
                &mut self.positions,
                &mut self.normals,
                &mut self.colors,
                &mut self.indices,
                [p0.to_array(), p1.to_array(), p2.to_array()],
                c,
            );
            push_face(
                &mut self.positions,
                &mut self.normals,
                &mut self.colors,
                &mut self.indices,
                [p0.to_array(), p2.to_array(), p3.to_array()],
                c,
            );
            // Caps.
            push_face(
                &mut self.positions,
                &mut self.normals,
                &mut self.colors,
                &mut self.indices,
                [top.to_array(), p3.to_array(), p2.to_array()],
                c,
            );
        }
    }

    /// Add a cone with its apex pointing up. Useful for foliage and lamps.
    pub fn add_cone(&mut self, bottom: Vec3, radius: f32, height: f32, sides: usize, color: Color) {
        let c = color.to_linear().to_f32_array();
        let top = bottom + Vec3::Y * height;
        for i in 0..sides {
            let a0 = i as f32 / sides as f32 * std::f32::consts::TAU;
            let a1 = (i + 1) as f32 / sides as f32 * std::f32::consts::TAU;
            let p0 = bottom + Vec3::new(a0.cos() * radius, 0.0, a0.sin() * radius);
            let p1 = bottom + Vec3::new(a1.cos() * radius, 0.0, a1.sin() * radius);
            push_face(
                &mut self.positions,
                &mut self.normals,
                &mut self.colors,
                &mut self.indices,
                [p0.to_array(), p1.to_array(), top.to_array()],
                c,
            );
            push_face(
                &mut self.positions,
                &mut self.normals,
                &mut self.colors,
                &mut self.indices,
                [bottom.to_array(), p1.to_array(), p0.to_array()],
                c,
            );
        }
    }

    /// Add a low-poly sphere. `rings`/`segments` stay small for the art style.
    pub fn add_uv_sphere(&mut self, center: Vec3, radius: f32, rings: usize, segments: usize, color: Color) {
        let c = color.to_linear().to_f32_array();
        let point = |ring: usize, seg: usize| {
            let v = ring as f32 / rings as f32 * PI;
            let u = seg as f32 / segments as f32 * std::f32::consts::TAU;
            center
                + Vec3::new(
                    v.sin() * u.cos() * radius,
                    v.cos() * radius,
                    v.sin() * u.sin() * radius,
                )
        };
        for ring in 0..rings {
            for seg in 0..segments {
                let a = point(ring, seg);
                let b = point(ring + 1, seg);
                let cc = point(ring + 1, seg + 1);
                let d = point(ring, seg + 1);
                push_face(
                    &mut self.positions,
                    &mut self.normals,
                    &mut self.colors,
                    &mut self.indices,
                    [a.to_array(), b.to_array(), cc.to_array()],
                    c,
                );
                push_face(
                    &mut self.positions,
                    &mut self.normals,
                    &mut self.colors,
                    &mut self.indices,
                    [a.to_array(), cc.to_array(), d.to_array()],
                    c,
                );
            }
        }
    }

    /// Add a flat quad on the XZ plane (y is constant).
    pub fn add_ground_quad(&mut self, center: Vec2, size: Vec2, y: f32, color: Color) {
        let c = color.to_linear().to_f32_array();
        let h = size * 0.5;
        let p = [
            Vec3::new(center.x - h.x, y, center.y - h.y),
            Vec3::new(center.x + h.x, y, center.y - h.y),
            Vec3::new(center.x + h.x, y, center.y + h.y),
            Vec3::new(center.x - h.x, y, center.y + h.y),
        ];
        let base = self.positions.len() as u32;
        for v in p {
            self.positions.push(v.to_array());
            self.normals.push([0.0, 1.0, 0.0]);
            self.colors.push(c);
        }
        self.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    /// Finish the mesh by attaching the vertex buffers.
    pub fn build(self, mut mesh: Mesh) -> Mesh {
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.colors);
        mesh.insert_indices(Indices::U32(self.indices));
        mesh
    }
}

/// Convenience helper: build a mesh with a user supplied closure.
pub fn build_mesh(f: impl FnOnce(&mut MeshAccumulator)) -> Mesh {
    let (mesh, mut acc) = new_mesh();
    f(&mut acc);
    acc.build(mesh)
}

/// Material used by all procedural geometry: unlit-ish flat colours with a bit
/// of roughness so the directional light still creates readable shadows.
pub fn vertex_material() -> StandardMaterial {
    StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.92,
        metallic: 0.0,
        reflectance: 0.05,
        ..default()
    }
}

/// A slightly emissive material for lamps / windows that should glow.
pub fn glow_material() -> StandardMaterial {
    StandardMaterial {
        base_color: Color::WHITE,
        emissive: LinearRgba::new(2.4, 1.9, 1.1, 1.0),
        ..default()
    }
}

/// Rotate a direction vector by a yaw angle.
pub fn yaw_vec(v: Vec3, yaw: f32) -> Vec3 {
    Quat::from_rotation_y(yaw) * v
}

/// Constant used by the character meshes.
pub const CHARACTER_FORWARD_OFFSET: f32 = FRAC_PI_2;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_mesh_has_12_triangles() {
        let mesh = build_mesh(|m| m.add_box(Vec3::ZERO, Vec3::ONE, Color::WHITE));
        assert_eq!(
            mesh.indices().expect("indices").len(),
            36,
            "a cube must be 12 triangles"
        );
    }

    #[test]
    fn cylinder_and_sphere_build() {
        let mesh = build_mesh(|m| {
            m.add_cylinder(Vec3::ZERO, 0.5, 2.0, 8, Color::WHITE);
            m.add_uv_sphere(Vec3::Y, 0.4, 3, 6, Color::WHITE);
            m.add_cone(Vec3::X, 0.4, 1.0, 6, Color::WHITE);
            m.add_ground_quad(Vec2::ZERO, Vec2::splat(4.0), 0.0, Color::WHITE);
        });
        assert!(mesh.count_vertices() > 0);
        assert!(mesh.indices().is_some());
    }
}
