//! Turns the [`ResortLayout`] into rendered geometry and collision data.
//!
//! Static architecture is merged into **one mesh per material** (24 buckets at
//! most) rather than one entity per box, so the entire resort draws in a few
//! dozen calls even on integrated graphics. Repeated foliage — palms, trees,
//! shrubs, grass — is emitted once per *unique* variant and then instanced with
//! `Mesh3d` + `Transform` children, which keeps the vertex budget flat while
//! still giving every palm a different shape.

use crate::core::{Block2, ZoneDefinition, ZoneId};
use crate::world::layout::{DecoKind, InteriorDetail, ResortLayout, SolidSpec};
use crate::world::materials::MaterialLibrary;
use crate::world::mesh_kit::{MatId, MeshBuilder};
use crate::world::props;
use bevy::prelude::*;
use bevy::render::view::RenderLayers;
use std::collections::BTreeMap;

use crate::core::WORLD_LAYER;

/// Marks every entity that belongs to the level, so it can be despawned when a
/// different map is loaded.
#[derive(Component)]
pub struct LevelEntity;

/// Shared handles created by the builder.
#[derive(Resource, Clone)]
pub struct LevelAssets {
    /// One merged mesh per material bucket.
    pub static_meshes: BTreeMap<MatId, Handle<Mesh>>,
    pub triangle_count: usize,
}

/// Append a solid to the builder.
fn add_solid(builder: &mut MeshBuilder, solid: &SolidSpec) {
    builder.add_box(solid.material, solid.center, solid.size);
}

/// Generate the architecture mesh buckets for the whole layout.
pub fn build_static_meshes(layout: &ResortLayout) -> Vec<(MatId, Mesh)> {
    let mut builder = MeshBuilder::new();
    for solid in &layout.solids {
        add_solid(&mut builder, solid);
    }
    for detail in &layout.details {
        add_detail(&mut builder, detail);
    }
    builder.finish()
}

fn add_detail(builder: &mut MeshBuilder, detail: &InteriorDetail) {
    builder.add_box(detail.material, detail.position, detail.size);
}

/// Ground surfaces (grass, sand, paths, water) as flat textured plates.
pub fn build_ground_meshes(layout: &ResortLayout) -> Vec<(MatId, Mesh)> {
    let mut builder = MeshBuilder::new();

    // Outdoor lawn covers everything outside the hotel and pool deck.
    builder.add_ground_quad(
        MatId::Grass,
        Vec2::new(0.0, 4.0),
        Vec2::new(148.0, 96.0),
        0.0,
    );
    // The beaches and lagoon read as separate materials.
    builder.add_ground_quad(MatId::Sand, Vec2::new(0.0, -48.0), Vec2::new(148.0, 28.0), 0.0);
    // Paths.
    builder.add_ground_quad(MatId::Terracotta, Vec2::new(0.0, -24.0), Vec2::new(5.2, 20.0), 0.08);
    builder.add_ground_quad(MatId::Asphalt, Vec2::new(0.0, 44.0), Vec2::new(26.0, 36.0), 0.06);
    builder.add_ground_quad(MatId::Marble, Vec2::new(0.0, 28.0), Vec2::new(24.0, 6.0), 0.07);
    // Pool deck paving.
    builder.add_ground_quad(MatId::Limestone, Vec2::new(37.0, -25.0), Vec2::new(42.0, 18.0), 0.06);
    let _ = layout;
    builder.finish()
}

/// Build prop meshes and their placements.
///
/// Repeated props (palms, shrubs, loungers...) are emitted as a small number of
/// *variants*; every placement of a variant becomes an instance of the same
/// mesh, which is what keeps a thousand-plant garden inside an integrated-GPU
/// vertex budget.
pub fn build_props(
    layout: &ResortLayout,
    meshes: &mut Assets<Mesh>,
) -> Vec<(Handle<Mesh>, MatId, Vec<(Vec3, f32, f32)>)> {
    // Group placements by kind, keeping authoring order per kind.
    let mut by_kind: BTreeMap<u8, Vec<&crate::world::layout::DecoSpec>> = BTreeMap::new();
    for deco in &layout.deco {
        by_kind.entry(deco.kind as u8).or_default().push(deco);
    }

    let mut out = Vec::new();
    for (_, placements) in by_kind {
        let kind = placements[0].kind;
        // 4 variants for foliage-heavy kinds, 2 for the rest.
        let variants = match kind {
            DecoKind::Palm
            | DecoKind::Tree
            | DecoKind::ShrubCluster
            | DecoKind::Bush
            | DecoKind::GrassTuft
            | DecoKind::FlowerBed
            | DecoKind::Crates => 4,
            _ => 2,
        }
        .min(placements.len().max(1));

        // yaw/scale are per-placement, so variants differ only by seed; the
        // transform supplies rotation and scale at spawn time.
        let mut buckets_per_variant: Vec<(Vec<(MatId, Handle<Mesh>)>, Vec<(Vec3, f32, f32)>)> =
            Vec::new();
        for variant in 0..variants {
            let mut builder = MeshBuilder::new();
            props::build_prop(&mut builder, kind, Vec3::ZERO, 0.0, 1.0, variant as u32 * 37 + 5);
            let built = builder
                .finish()
                .into_iter()
                .map(|(mat, mesh)| (mat, meshes.add(mesh)))
                .collect::<Vec<_>>();
            let assigned = placements
                .iter()
                .enumerate()
                .filter(|(i, _)| i % variants == variant)
                .map(|(_, d)| (d.position, d.yaw, d.scale))
                .collect::<Vec<_>>();
            buckets_per_variant.push((built, assigned));
        }

        for (meshes_for_variant, assigned) in buckets_per_variant {
            if assigned.is_empty() {
                continue;
            }
            // Split the placements by material of the submesh, so each bucket
            // instances independently.
            for (mat, handle) in meshes_for_variant {
                out.push((handle, mat, assigned.clone()));
            }
        }
    }
    out
}

/// Spawn the resort: merged architecture, ground plates, instanced props and
/// interior lighting.
pub fn spawn_level(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    layout: &ResortLayout,
    library: &MaterialLibrary,
) {
    let mut triangle_count = 0;
    let mut static_meshes = BTreeMap::new();

    for (mat, mesh) in build_static_meshes(layout) {
        triangle_count += mesh.indices().map(|i| i.len() / 3).unwrap_or(0);
        let handle = meshes.add(mesh);
        commands.spawn((
            LevelEntity,
            Name::new(format!("Architecture {:?}", mat)),
            Mesh3d(handle.clone()),
            MeshMaterial3d(library.get(mat)),
            Transform::default(),
            RenderLayers::layer(WORLD_LAYER),
        ));
        static_meshes.insert(mat, handle);
    }

    for (mat, mesh) in build_ground_meshes(layout) {
        triangle_count += mesh.indices().map(|i| i.len() / 3).unwrap_or(0);
        let handle = meshes.add(mesh);
        commands.spawn((
            LevelEntity,
            Name::new(format!("Ground {:?}", mat)),
            Mesh3d(handle.clone()),
            MeshMaterial3d(library.get(mat)),
            Transform::default(),
            RenderLayers::layer(WORLD_LAYER),
        ));
        static_meshes.insert(mat, handle);
    }

    // Props. Multi-material props are merged per material when they are built
    // at the origin, so one variant bucket == one draw call for N instances.
    let prop_groups = build_props(layout, meshes);
    let mut prop_entities = 0;
    for (mesh, mat, placements) in prop_groups {
        for (position, yaw, scale) in placements {
            prop_entities += 1;
            commands.spawn((
                LevelEntity,
                Name::new("Prop"),
                Mesh3d(mesh.clone()),
                MeshMaterial3d(library.get(mat)),
                Transform::from_translation(position)
                    .with_rotation(Quat::from_rotation_y(yaw))
                    .with_scale(Vec3::splat(scale)),
                RenderLayers::layer(WORLD_LAYER),
            ));
        }
    }

    info!(
        "Level built: {} triangles of architecture, {} static meshes, {} prop instances",
        triangle_count,
        static_meshes.len(),
        prop_entities
    );

    commands.insert_resource(LevelAssets {
        static_meshes,
        triangle_count,
    });
}

/// Static collision data shared with every movement system.
#[derive(Resource, Debug)]
pub struct WorldCollision {
    /// Solids that block horizontal movement.
    pub blocks: Vec<Block2>,
    /// Every surface an actor can stand on (solids + explicit floors).
    pub surfaces: Vec<Block2>,
    pub zones: Vec<ZoneDefinition>,
}

impl WorldCollision {
    pub fn from_layout(layout: &ResortLayout) -> Self {
        let blocks = layout.obstacle_blocks();
        // Every solid can be stood on, including low stair treads and desks
        // that are deliberately excluded from the walk-blocking set.
        let mut surfaces: Vec<Block2> = layout.solids.iter().map(|solid| solid.block()).collect();
        surfaces.extend(layout.floors.iter().copied());
        Self {
            blocks,
            surfaces,
            zones: layout.zones.clone(),
        }
    }

    pub fn zone_at(&self, point: Vec2) -> ZoneId {
        let mut best: Option<(&ZoneDefinition, usize)> = None;
        for zone in &self.zones {
            if zone.rect.contains(point) && best.is_none_or(|(_, prio)| zone.priority as usize >= prio) {
                best = Some((zone, zone.priority as usize));
            }
        }
        best.map(|(z, _)| z.id).unwrap_or(ZoneId::Beach)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::layout::resort_layout;

    #[test]
    fn generated_meshes_are_non_empty() {
        let layout = resort_layout();
        let buckets = build_static_meshes(&layout);
        assert!(buckets.len() > 8, "expected many material buckets");
        let triangles: usize = buckets
            .iter()
            .map(|(_, m)| m.indices().map(|i| i.len() / 3).unwrap_or(0))
            .sum();
        // The resort should be a substantial scene, not a handful of boxes.
        assert!(triangles > 20_000, "only {triangles} triangles of architecture");
    }

    #[test]
    fn collision_matches_layout() {
        let layout = resort_layout();
        let collision = WorldCollision::from_layout(&layout);
        assert_eq!(collision.blocks.len(), layout.obstacle_blocks().len());
        assert_eq!(
            collision.surfaces.len(),
            layout.solids.len() + layout.floors.len()
        );
        assert!(collision.surfaces.len() > collision.blocks.len());
        assert_eq!(collision.zone_at(Vec2::new(0.0, 30.0)), ZoneId::Penthouse);
    }

    #[test]
    fn outdoor_ground_is_standable() {
        let layout = resort_layout();
        let collision = WorldCollision::from_layout(&layout);
        let spawn = layout.player_spawn;
        let floor = crate::player::movement::ground_height(
            &collision.surfaces,
            Vec2::new(spawn.x, spawn.z),
            0.38,
            spawn.y,
        );
        assert!(
            floor.is_some_and(|f| f.abs() < 0.2),
            "expected ground at y=0 at the spawn, got {floor:?}"
        );
    }

    #[test]
    fn mesh_vertex_counts_are_consistent() {
        let layout = resort_layout();
        for (mat, mesh) in build_static_meshes(&layout) {
            let indices = mesh.indices().expect("indices");
            assert_eq!(indices.len() % 3, 0, "{mat:?} produced an incomplete triangle");
            assert_eq!(
                mesh.attribute(Mesh::ATTRIBUTE_UV_0).map(|a| a.len()),
                Some(mesh.count_vertices()),
                "{mat:?} is missing UVs"
            );
        }
    }

    #[test]
    fn interiors_are_enclosed() {
        // Every walkable building interior needs a ceiling, otherwise the sky
        // shines through and the room reads as unfinished.
        let layout = resort_layout();
        let has_ceiling_near = |center: Vec2, y: f32| {
            layout.solids.iter().any(|s| {
                let min = Vec2::new(s.center.x, s.center.z) - Vec2::new(s.size.x, s.size.z) * 0.5;
                let max = Vec2::new(s.center.x, s.center.z) + Vec2::new(s.size.x, s.size.z) * 0.5;
                let bottom = s.center.y - s.size.y * 0.5;
                min.x <= center.x && max.x >= center.x && min.y <= center.y && max.y >= center.y
                    && (bottom - y).abs() < 1.2
                    && s.size.x > 1.5
                    && s.size.z > 1.5
            })
        };
        assert!(has_ceiling_near(Vec2::new(0.0, 0.0), crate::world::layout::LOBBY_CEILING));
        assert!(has_ceiling_near(Vec2::new(-39.0, 0.0), crate::world::layout::SERVICE_Y + 4.0));
        assert!(has_ceiling_near(Vec2::new(-55.0, -20.0), crate::world::layout::SERVICE_Y + 4.0));
    }
}
