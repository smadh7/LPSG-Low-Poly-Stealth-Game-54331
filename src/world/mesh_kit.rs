//! Mesh authoring with UVs, smooth normals and material buckets.
//!
//! Earlier this module produced untested low-poly primitives with flat shading
//! and per-vertex colours. The realistic pipeline needs three things it did not
//! offer:
//!
//! * **UVs.** Every surface is textured, so primitives generate UVs in *world
//!   units* (a 4 m wall has UVs spanning 4 metres). Each material declares how
//!   many metres one texture repeat covers, and Bevy's `uv_transform` turns
//!   that into texture scale. One shared texture per material, tiled correctly
//!   on every object regardless of size.
//! * **Smooth normals.** Cylinders, spheres and foliage get analytic normals so
//!   palms and tanks do not look faceted.
//! * **Buckets.** Primitives are tagged with a [`MatId`]; the builder groups
//!   geometry by material so a whole building is one draw call per material.

use bevy::{
    asset::RenderAssetUsages,
    math::Vec3,
    prelude::*,
    render::mesh::{Indices, Mesh, PrimitiveTopology, VertexAttributeValues},
};
use std::f32::consts::{PI, TAU};

/// Material identity used to bucket geometry. Mirrors `world::materials`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MatId {
    Grass,
    Sand,
    Marble,
    Limestone,
    Travertine,
    Plaster,
    Wood,
    Terracotta,
    Concrete,
    Brick,
    Slate,
    Metal,
    StoneWall,
    Roof,
    Hedge,
    PalmBark,
    Frond,
    Water,
    WhitePaint,
    DarkTrim,
    Brass,
    Rubber,
    Asphalt,
    Glass,
    Fabric,
    Skin,
}

impl MatId {
    /// Stable index, also used as the array position in [`MaterialLibrary`].
    pub const ALL: [MatId; 26] = [
        MatId::Grass,
        MatId::Sand,
        MatId::Marble,
        MatId::Limestone,
        MatId::Travertine,
        MatId::Plaster,
        MatId::Wood,
        MatId::Terracotta,
        MatId::Concrete,
        MatId::Brick,
        MatId::Slate,
        MatId::Metal,
        MatId::StoneWall,
        MatId::Roof,
        MatId::Hedge,
        MatId::PalmBark,
        MatId::Frond,
        MatId::Water,
        MatId::WhitePaint,
        MatId::DarkTrim,
        MatId::Brass,
        MatId::Rubber,
        MatId::Asphalt,
        MatId::Glass,
        MatId::Fabric,
        MatId::Skin,
    ];

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|m| *m == self).unwrap_or(0)
    }

    pub fn from_index(index: usize) -> Self {
        Self::ALL[index.min(Self::ALL.len() - 1)]
    }
}

/// Geometry accumulator for one material bucket.
#[derive(Default)]
pub struct Bucket {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

impl Bucket {
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Append a triangle with explicit normals and UVs.
    #[allow(clippy::too_many_arguments)]
    pub fn tri(
        &mut self,
        p: [Vec3; 3],
        n: [Vec3; 3],
        uv: [[f32; 2]; 3],
    ) {
        let base = self.positions.len() as u32;
        for i in 0..3 {
            self.positions.push(p[i].to_array());
            self.normals.push(n[i].to_array());
            self.uvs.push(uv[i]);
        }
        self.indices.extend_from_slice(&[base, base + 1, base + 2]);
    }

    /// Append a quad (two triangles) with the given winding order.
    pub fn quad(&mut self, p: [Vec3; 4], n: [Vec3; 4], uv: [[f32; 2]; 4]) {
        let base = self.positions.len() as u32;
        for i in 0..4 {
            self.positions.push(p[i].to_array());
            self.normals.push(n[i].to_array());
            self.uvs.push(uv[i]);
        }
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    /// Convert into a [`Mesh`] ready for rendering.
    pub fn into_mesh(self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh.insert_indices(Indices::U32(self.indices));
        mesh
    }
}

/// Merge `other`'s geometry into `target`, preserving every attribute.
///
/// Used for small multi-material meshes (character parts, prop submeshes) where
/// a single mesh per entity matters more than one draw call per material.
pub fn merge_into(target: &mut Mesh, other: Mesh) {
    let base = target.count_vertices() as u32;

    let mut positions: Vec<[f32; 3]> = attributes_float3(target, Mesh::ATTRIBUTE_POSITION);
    let mut normals: Vec<[f32; 3]> = attributes_float3(target, Mesh::ATTRIBUTE_NORMAL);
    let mut uvs: Vec<[f32; 2]> = attributes_float2(target, Mesh::ATTRIBUTE_UV_0);
    let mut indices: Vec<u32> = target
        .indices()
        .map(|i| i.iter().map(|v| v as u32).collect::<Vec<u32>>())
        .unwrap_or_default();

    positions.extend(attributes_float3(&other, Mesh::ATTRIBUTE_POSITION));
    normals.extend(attributes_float3(&other, Mesh::ATTRIBUTE_NORMAL));
    uvs.extend(attributes_float2(&other, Mesh::ATTRIBUTE_UV_0));
    if let Some(other_indices) = other.indices() {
        indices.extend(other_indices.iter().map(|i| i as u32 + base));
    }

    target.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    target.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    target.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    target.insert_indices(Indices::U32(indices));
}

fn attributes_float3(
    mesh: &Mesh,
    attribute: bevy::render::mesh::MeshVertexAttribute,
) -> Vec<[f32; 3]> {
    match mesh.attribute(attribute) {
        Some(values) => values
            .as_float3()
            .map(|v| v.to_vec())
            .unwrap_or_default(),
        None => Vec::new(),
    }
}

fn attributes_float2(
    mesh: &Mesh,
    attribute: bevy::render::mesh::MeshVertexAttribute,
) -> Vec<[f32; 2]> {
    match mesh.attribute(attribute) {
        Some(VertexAttributeValues::Float32x2(values)) => values.clone(),
        _ => Vec::new(),
    }
}

/// Collects geometry, bucketed by material, while a level is being built.
#[derive(Default)]
pub struct MeshBuilder {
    buckets: Vec<(MatId, Bucket)>,
}

impl MeshBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Borrow the bucket for `mat`, creating it on first use.
    pub fn bucket(&mut self, mat: MatId) -> &mut Bucket {
        if let Some(index) = self.buckets.iter().position(|(m, _)| *m == mat) {
            return &mut self.buckets[index].1;
        }
        self.buckets.push((mat, Bucket::default()));
        let last = self.buckets.len() - 1;
        &mut self.buckets[last].1
    }

    /// Finished buckets, in insertion order.
    pub fn finish(self) -> Vec<(MatId, Mesh)> {
        self.buckets
            .into_iter()
            .filter(|(_, b)| !b.is_empty())
            .map(|(m, b)| (m, b.into_mesh()))
            .collect()
    }

    pub fn triangle_count(&self) -> usize {
        self.buckets.iter().map(|(_, b)| b.triangle_count()).sum()
    }

    // -- primitives -------------------------------------------------------

    /// Axis-aligned box. UVs are generated per face in metres, so textures tile
    /// at a constant world scale no matter how large the box is.
    pub fn add_box(&mut self, mat: MatId, center: Vec3, size: Vec3) {
        let h = size * 0.5;
        add_box_vertices(self.bucket(mat), center - h, center + h);
    }

    /// Box from explicit bounds.
    pub fn add_box_bounds(&mut self, mat: MatId, min: Vec3, max: Vec3) {
        add_box_vertices(self.bucket(mat), min, max);
    }

    /// Box rotated by `yaw` around its own centre.
    pub fn add_box_rotated(&mut self, mat: MatId, center: Vec3, size: Vec3, yaw: f32) {
        add_rotated_box_vertices(self.bucket(mat), center, size, yaw);
    }

    /// Vertical cylinder with smooth side normals.
    pub fn add_cylinder(&mut self, mat: MatId, bottom: Vec3, radius: f32, height: f32, sides: usize) {
        add_cylinder_vertices(self.bucket(mat), bottom, radius, height, sides);
    }

    /// Cone with its apex pointing up.
    pub fn add_cone(&mut self, mat: MatId, bottom: Vec3, radius: f32, height: f32, sides: usize) {
        add_cone_vertices(self.bucket(mat), bottom, radius, height, sides);
    }

    /// Tapered cylinder (used for trunks: wide at the base, thin at the top).
    pub fn add_taper(
        &mut self,
        mat: MatId,
        bottom: Vec3,
        bottom_radius: f32,
        top_radius: f32,
        height: f32,
        sides: usize,
    ) {
        add_taper_vertices(self.bucket(mat), bottom, bottom_radius, top_radius, height, sides);
    }

    /// Ellipsoid with smooth normals.
    pub fn add_sphere(&mut self, mat: MatId, center: Vec3, radius: Vec3, rings: usize, segments: usize) {
        add_sphere_vertices(self.bucket(mat), center, radius, rings, segments);
    }

    /// Horizontal quad facing up.
    pub fn add_ground_quad(&mut self, mat: MatId, center: Vec2, size: Vec2, y: f32) {
        let h = size * 0.5;
        let b = self.bucket(mat);
        let p = [
            Vec3::new(center.x - h.x, y, center.y - h.y),
            Vec3::new(center.x - h.x, y, center.y + h.y),
            Vec3::new(center.x + h.x, y, center.y + h.y),
            Vec3::new(center.x + h.x, y, center.y - h.y),
        ];
        b.quad(
            p,
            [Vec3::Y; 4],
            [[p[0].x, p[0].z], [p[1].x, p[1].z], [p[2].x, p[2].z], [p[3].x, p[3].z]],
        );
    }

    /// Arbitrary quad (e.g. a beach slope). The normal is derived from the
    /// winding order, so pass the vertices counter-clockwise when viewed from
    /// the visible side.
    pub fn add_quad(&mut self, mat: MatId, p: [Vec3; 4]) {
        let n = (p[1] - p[0]).cross(p[3] - p[0]).normalize_or_zero();
        let uv = [
            [p[0].x + p[0].z, p[0].y],
            [p[1].x + p[1].z, p[1].y],
            [p[2].x + p[2].z, p[2].y],
            [p[3].x + p[3].z, p[3].y],
        ];
        self.bucket(mat).quad(p, [n; 4], uv);
    }
}

// ---------------------------------------------------------------------------
// Primitive construction
// ---------------------------------------------------------------------------

fn add_box_vertices(b: &mut Bucket, min: Vec3, max: Vec3) {
    let p = [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(max.x, max.y, max.z),
        Vec3::new(min.x, max.y, max.z),
    ];
    // +Y
    b.quad(
        [p[4], p[7], p[6], p[5]],
        [Vec3::Y; 4],
        [[p[4].x, p[4].z], [p[7].x, p[7].z], [p[6].x, p[6].z], [p[5].x, p[5].z]],
    );
    // -Y
    b.quad(
        [p[0], p[1], p[2], p[3]],
        [Vec3::NEG_Y; 4],
        [[p[0].x, p[0].z], [p[1].x, p[1].z], [p[2].x, p[2].z], [p[3].x, p[3].z]],
    );
    // -Z
    b.quad(
        [p[0], p[4], p[5], p[1]],
        [Vec3::NEG_Z; 4],
        [[p[0].x, p[0].y], [p[4].x, p[4].y], [p[5].x, p[5].y], [p[1].x, p[1].y]],
    );
    // +Z
    b.quad(
        [p[3], p[2], p[6], p[7]],
        [Vec3::Z; 4],
        [[p[3].x, p[3].y], [p[2].x, p[2].y], [p[6].x, p[6].y], [p[7].x, p[7].y]],
    );
    // -X
    b.quad(
        [p[3], p[7], p[4], p[0]],
        [Vec3::NEG_X; 4],
        [[p[3].z, p[3].y], [p[7].z, p[7].y], [p[4].z, p[4].y], [p[0].z, p[0].y]],
    );
    // +X
    b.quad(
        [p[1], p[5], p[6], p[2]],
        [Vec3::X; 4],
        [[p[1].z, p[1].y], [p[5].z, p[5].y], [p[6].z, p[6].y], [p[2].z, p[2].y]],
    );
}

fn add_rotated_box_vertices(b: &mut Bucket, center: Vec3, size: Vec3, yaw: f32) {
    let h = size * 0.5;
    let rot = Quat::from_rotation_y(yaw);
    // Local corner helper: (sx, sy, sz) each -1 or +1.
    let corner = |sx: f32, sy: f32, sz: f32| center + rot * Vec3::new(sx * h.x, sy * h.y, sz * h.z);
    let n = |x: f32, z: f32| rot * Vec3::new(x, 0.0, z);
    let ax = rot * Vec3::X;
    let az = rot * Vec3::Z;
    // UV projection in the box's own frame, in metres.
    let uv = |p: Vec3, a: Vec3, c: Vec3| [p.dot(a) + p.dot(c), p.y];

    // +Y and -Y
    let (t0, t1, t2, t3) = (corner(-1.0, 1.0, -1.0), corner(-1.0, 1.0, 1.0), corner(1.0, 1.0, 1.0), corner(1.0, 1.0, -1.0));
    b.quad([t0, t1, t2, t3], [Vec3::Y; 4], [uv(t0, ax, az), uv(t1, ax, az), uv(t2, ax, az), uv(t3, ax, az)]);
    let (b0, b1, b2, b3) = (corner(-1.0, -1.0, -1.0), corner(-1.0, -1.0, 1.0), corner(1.0, -1.0, 1.0), corner(1.0, -1.0, -1.0));
    b.quad([b3, b2, b1, b0], [Vec3::NEG_Y; 4], [uv(b3, ax, az), uv(b2, ax, az), uv(b1, ax, az), uv(b0, ax, az)]);

    // Four vertical sides.
    let sides = [
        (corner(-1.0, -1.0, -1.0), corner(1.0, -1.0, -1.0), n(0.0, -1.0), ax),
        (corner(1.0, -1.0, -1.0), corner(1.0, -1.0, 1.0), n(1.0, 0.0), az),
        (corner(1.0, -1.0, 1.0), corner(-1.0, -1.0, 1.0), n(0.0, 1.0), ax),
        (corner(-1.0, -1.0, 1.0), corner(-1.0, -1.0, -1.0), n(-1.0, 0.0), az),
    ];
    for (a, c, normal, axis) in sides {
        let up = Vec3::Y * (h.y * 2.0);
        let p = [a, c, c + up, a + up];
        b.quad(
            p,
            [normal; 4],
            [
                [p[0].dot(axis), 0.0],
                [p[1].dot(axis), 0.0],
                [p[2].dot(axis), h.y * 2.0],
                [p[3].dot(axis), h.y * 2.0],
            ],
        );
    }
}

fn add_cylinder_vertices(b: &mut Bucket, bottom: Vec3, radius: f32, height: f32, sides: usize) {
    let sides = sides.max(3);
    let top = bottom + Vec3::Y * height;
    let circumference = TAU * radius;
    for i in 0..sides {
        let a0 = i as f32 / sides as f32 * TAU;
        let a1 = (i + 1) as f32 / sides as f32 * TAU;
        let d0 = Vec3::new(a0.cos(), 0.0, a0.sin());
        let d1 = Vec3::new(a1.cos(), 0.0, a1.sin());
        let p0 = bottom + d0 * radius;
        let p1 = bottom + d1 * radius;
        let p2 = top + d1 * radius;
        let p3 = top + d0 * radius;
        let u0 = i as f32 / sides as f32 * circumference;
        let u1 = (i + 1) as f32 / sides as f32 * circumference;
        b.quad(
            [p0, p1, p2, p3],
            [d0, d1, d1, d0],
            [[u0, 0.0], [u1, 0.0], [u1, height], [u0, height]],
        );
        // Caps (flat shaded).
        b.tri([top, p3, p2], [Vec3::Y; 3], [
            [top.x, top.z],
            [p3.x, p3.z],
            [p2.x, p2.z],
        ]);
        let c0 = Vec3::new(bottom.x, bottom.y, bottom.z);
        b.tri(
            [bottom, p1, p0],
            [Vec3::NEG_Y; 3],
            [[c0.x, c0.z], [p1.x, p1.z], [p0.x, p0.z]],
        );
    }
}

fn add_taper_vertices(
    b: &mut Bucket,
    bottom: Vec3,
    bottom_radius: f32,
    top_radius: f32,
    height: f32,
    sides: usize,
) {
    let sides = sides.max(3);
    let top = bottom + Vec3::Y * height;
    let slope = (bottom_radius - top_radius) / height.max(0.001);
    for i in 0..sides {
        let a0 = i as f32 / sides as f32 * TAU;
        let a1 = (i + 1) as f32 / sides as f32 * TAU;
        let d0 = Vec3::new(a0.cos(), 0.0, a0.sin());
        let d1 = Vec3::new(a1.cos(), 0.0, a1.sin());
        let p0 = bottom + d0 * bottom_radius;
        let p1 = bottom + d1 * bottom_radius;
        let p2 = top + d1 * top_radius;
        let p3 = top + d0 * top_radius;
        let n0 = (d0 + Vec3::Y * slope).normalize_or_zero();
        let n1 = (d1 + Vec3::Y * slope).normalize_or_zero();
        let circumference = TAU * bottom_radius.max(top_radius);
        let u0 = i as f32 / sides as f32 * circumference;
        let u1 = (i + 1) as f32 / sides as f32 * circumference;
        b.quad(
            [p0, p1, p2, p3],
            [n0, n1, n1, n0],
            [[u0, 0.0], [u1, 0.0], [u1, height], [u0, height]],
        );
    }
}

fn add_cone_vertices(b: &mut Bucket, bottom: Vec3, radius: f32, height: f32, sides: usize) {
    let sides = sides.max(3);
    let apex = bottom + Vec3::Y * height;
    for i in 0..sides {
        let a0 = i as f32 / sides as f32 * TAU;
        let a1 = (i + 1) as f32 / sides as f32 * TAU;
        let d0 = Vec3::new(a0.cos(), 0.0, a0.sin());
        let d1 = Vec3::new(a1.cos(), 0.0, a1.sin());
        let p0 = bottom + d0 * radius;
        let p1 = bottom + d1 * radius;
        let n0 = (d0 * height + Vec3::Y * radius).normalize_or_zero();
        let n1 = (d1 * height + Vec3::Y * radius).normalize_or_zero();
        b.tri(
            [p0, p1, apex],
            [n0, n1, (n0 + n1).normalize_or_zero()],
            [[0.0, 0.0], [radius * TAU / sides as f32, 0.0], [radius, height]],
        );
        b.tri(
            [bottom, p1, p0],
            [Vec3::NEG_Y; 3],
            [[bottom.x, bottom.z], [p1.x, p1.z], [p0.x, p0.z]],
        );
    }
}

fn add_sphere_vertices(
    b: &mut Bucket,
    center: Vec3,
    radius: Vec3,
    rings: usize,
    segments: usize,
) {
    let rings = rings.max(2);
    let segments = segments.max(3);
    let point = |ring: usize, seg: usize| {
        let v = ring as f32 / rings as f32 * PI;
        let u = seg as f32 / segments as f32 * TAU;
        let dir = Vec3::new(v.sin() * u.cos(), v.cos(), v.sin() * u.sin());
        (center + dir * radius, dir)
    };
    for ring in 0..rings {
        for seg in 0..segments {
            let (a, na) = point(ring, seg);
            let (bb, nb) = point(ring + 1, seg);
            let (cc, nc) = point(ring + 1, seg + 1);
            let (d, nd) = point(ring, seg + 1);
            let u0 = seg as f32 / segments as f32;
            let u1 = (seg + 1) as f32 / segments as f32;
            let v0 = ring as f32 / rings as f32;
            let v1 = (ring + 1) as f32 / rings as f32;
            // Emit as a quad unless we are at a pole (degenerate triangle).
            if ring == 0 {
                b.tri([a, bb, cc], [na, nb, nc], [[u0, v0], [u0, v1], [u1, v1]]);
            } else if ring + 1 == rings {
                b.tri([a, bb, d], [na, nb, nd], [[u0, v0], [u0, v1], [u1, v0]]);
            } else {
                b.quad(
                    [a, bb, cc, d],
                    [na, nb, nc, nd],
                    [[u0, v0], [u0, v1], [u1, v1], [u1, v0]],
                );
            }
        }
    }
}

/// Build a single-material mesh with a closure.
pub fn build_mesh(f: impl FnOnce(&mut MeshBuilder)) -> Mesh {
    let mut builder = MeshBuilder::new();
    f(&mut builder);
    builder
        .finish()
        .into_iter()
        .next()
        .map(|(_, mesh)| mesh)
        .unwrap_or_else(|| Bucket::default().into_mesh())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_has_twelve_triangles_and_uvs() {
        let mut b = Bucket::default();
        add_box_vertices(&mut b, Vec3::ZERO, Vec3::ONE);
        assert_eq!(b.indices.len(), 36);
        assert_eq!(b.positions.len(), 24);
        assert_eq!(b.uvs.len(), 24);
    }

    #[test]
    fn box_faces_point_outwards() {
        let mut b = Bucket::default();
        add_box_vertices(&mut b, Vec3::ZERO, Vec3::splat(2.0));
        // Every face normal must point away from the box centre.
        for chunk in b.positions.chunks(4).zip(b.normals.chunks(4)) {
            let (verts, normals) = chunk;
            for v in verts {
                let centre = Vec3::new(1.0, 1.0, 1.0);
                let outward = (Vec3::from(*v) - centre).normalize_or_zero();
                assert!(outward.dot(Vec3::from(normals[0])) > 0.1);
            }
        }
    }

    #[test]
    fn cylinders_and_spheres_build_with_smooth_normals() {
        let mut b = Bucket::default();
        add_cylinder_vertices(&mut b, Vec3::ZERO, 0.5, 2.0, 24);
        add_taper_vertices(&mut b, Vec3::ZERO, 0.3, 0.1, 4.0, 8);
        add_cone_vertices(&mut b, Vec3::ZERO, 0.4, 1.0, 8);
        add_sphere_vertices(&mut b, Vec3::Y, Vec3::splat(0.5), 8, 12);
        assert!(b.positions.len() > 400, "only {} verts", b.positions.len());
        for n in &b.normals {
            let len = Vec3::from(*n).length();
            assert!((len - 1.0).abs() < 0.01, "normals must be unit length");
        }
    }

    #[test]
    fn builder_buckets_by_material() {
        let mut builder = MeshBuilder::new();
        builder.add_box(MatId::Wood, Vec3::ZERO, Vec3::ONE);
        builder.add_box(MatId::Wood, Vec3::X, Vec3::ONE);
        builder.add_box(MatId::Metal, Vec3::Y, Vec3::ONE);
        assert_eq!(builder.triangle_count(), 36);
        let meshes = builder.finish();
        assert_eq!(meshes.len(), 2);
    }

    #[test]
    fn uvs_are_world_scaled() {
        // A 4 m wide wall must have UVs spanning ~4 units so textures tile at a
        // constant physical size.
        let mut builder = MeshBuilder::new();
        builder.add_box(MatId::Plaster, Vec3::ZERO, Vec3::new(4.0, 3.0, 0.2));
        let meshes = builder.finish();
        let mut wall = Bucket::default();
        add_box_vertices(&mut wall, Vec3::new(-2.0, -1.5, -0.1), Vec3::new(2.0, 1.5, 0.1));
        let span = wall.uvs.iter().map(|uv| uv[0]).fold(f32::MIN, f32::max)
            - wall.uvs.iter().map(|uv| uv[0]).fold(f32::MAX, f32::min);
        assert!(span > 3.5 && span < 4.5, "uv span was {span}");
        assert!(!meshes.is_empty());
    }
}
