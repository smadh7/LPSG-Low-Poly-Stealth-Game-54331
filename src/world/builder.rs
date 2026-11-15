//! Turns the [`ResortLayout`] data into Bevy entities.
//!
//! All static geometry is merged into a handful of meshes (one per material
//! "bucket") which keeps draw calls low on modest hardware.

use crate::core::{Block2, ZoneDefinition, ZoneId};
use crate::world::layout::{palette, DecoKind, ResortLayout, SolidSpec};
use crate::world::mesh_kit::{self, MeshAccumulator};
use bevy::prelude::*;

/// Marks every entity that belongs to the level so it can be despawned when
/// a different map is loaded.
#[derive(Component)]
pub struct LevelEntity;

/// Shared handles to the meshes and materials created by the builder.
#[derive(Resource)]
pub struct LevelAssets {
    pub solid_mesh: Handle<Mesh>,
    pub deco_mesh: Handle<Mesh>,
    pub ground_mesh: Handle<Mesh>,
    pub material: Handle<StandardMaterial>,
    pub glow_material: Handle<StandardMaterial>,
}

/// Friction / step parameters for a solid block requested by a system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Step {
    /// Height of the ledge the player will be snapped to.
    pub height: f32,
    /// Solid that must be re-queried on the next frame.
    pub block_index: usize,
}

/// Append a single solid to a mesh accumulator (targeted chunks with blocks).
fn add_solid(acc: &mut MeshAccumulator, solid: &SolidSpec) {
    acc.add_box(solid.center, solid.size, solid.color);
}

/// Generate the resort geometry for a chunk of solids.
pub fn build_solid_mesh(solids: &[SolidSpec]) -> Mesh {
    mesh_kit::build_mesh(|acc| {
        for solid in solids {
            add_solid(acc, solid);
        }
    })
}

/// Build the ground plates (grass, sand, sea, paths).
fn build_ground(layout: &ResortLayout) -> Mesh {
    mesh_kit::build_mesh(|acc| {
        // The sea bed and water.
        acc.add_ground_quad(Vec2::new(0.0, -70.0), Vec2::new(220.0, 40.0), -0.6, palette::SEA);
        acc.add_ground_quad(Vec2::new(0.0, -64.0), Vec2::new(220.0, 8.0), -0.4, palette::SEA);
        // Beach strip.
        acc.add_ground_quad(Vec2::new(0.0, -54.0), Vec2::new(150.0, 20.0), 0.0, palette::SAND);
        acc.add_ground_quad(Vec2::new(0.0, -46.0), Vec2::new(150.0, 6.0), 0.02, palette::SAND_DARK);
        // Gardens.
        acc.add_ground_quad(Vec2::new(0.0, -30.0), Vec2::new(150.0, 32.0), 0.0, palette::GRASS);
        acc.add_ground_quad(Vec2::new(0.0, -14.0), Vec2::new(150.0, 4.0), 0.03, palette::STONE);
        // Ground around/under the hotel and side areas.
        acc.add_ground_quad(Vec2::new(0.0, 6.0), Vec2::new(150.0, 40.0), 0.0, palette::GRASS);
        acc.add_ground_quad(Vec2::new(0.0, 38.0), Vec2::new(150.0, 30.0), 0.0, palette::GRASS_DARK);
        // Front driveway / plaza leading to the lobby entrance.
        acc.add_ground_quad(Vec2::new(0.0, -16.5), Vec2::new(26.0, 7.0), 0.04, palette::MARBLE);
        acc.add_ground_quad(Vec2::new(0.0, -24.0), Vec2::new(12.0, 12.0), 0.04, palette::STONE);
        // Staff terrace east of the lobby.
        acc.add_ground_quad(Vec2::new(38.0, 21.0), Vec2::new(30.0, 20.0), 0.02, palette::STONE_DARK);
        let _ = layout;
    })
}

/// Append a decorative prop to the mesh accumulator.
fn add_deco(acc: &mut MeshAccumulator, kind: DecoKind, pos: Vec3, yaw: f32, scale: f32) {
    match kind {
        DecoKind::Palm => {
            let h = 5.2 * scale;
            acc.add_cylinder(pos, 0.22 * scale, h * 0.7, 6, palette::PALM_TRUNK);
            let top = pos + Vec3::Y * h * 0.7;
            // A couple of fronds made from flattened, rotated boxes.
            for i in 0..7 {
                let a = yaw + i as f32 * std::f32::consts::TAU / 7.0;
                let leaf_pos = top + Vec3::new(a.cos() * 1.1 * scale, 0.15, a.sin() * 1.1 * scale);
                acc.add_box_rotated(
                    leaf_pos,
                    Vec3::new(2.6 * scale, 0.12 * scale, 0.9 * scale),
                    -a,
                    if i % 2 == 0 { palette::PALM_LEAF } else { palette::PALM_LEAF_LIGHT },
                );
            }
            acc.add_cone(top + Vec3::Y * 0.15, 0.9 * scale, 0.8 * scale, 6, palette::PALM_LEAF_LIGHT);
        }
        DecoKind::Bush => {
            acc.add_uv_sphere(pos + Vec3::Y * 0.5 * scale, 0.6 * scale, 2, 5, palette::GRASS_DARK);
        }
        DecoKind::FlowerBed => {
            acc.add_box_bounds(
                pos - Vec3::new(2.0 * scale, 0.0, 1.2 * scale),
                pos + Vec3::new(2.0 * scale, 0.35 * scale, 1.2 * scale),
                palette::WOOD,
            );
            for i in 0..8 {
                let x = pos.x - 1.6 * scale + i as f32 * 0.45 * scale;
                let color = if i % 2 == 0 { palette::FLOWER_RED } else { palette::FLOWER_YELLOW };
                acc.add_uv_sphere(Vec3::new(x, pos.y + 0.5 * scale, pos.z), 0.22 * scale, 2, 5, color);
            }
        }
        DecoKind::LampPost => {
            acc.add_cylinder(pos, 0.12 * scale, 3.6 * scale, 6, palette::METAL);
            acc.add_uv_sphere(pos + Vec3::Y * 3.7 * scale, 0.34 * scale, 2, 6, palette::FLOWER_YELLOW);
        }
        DecoKind::Parasol => {
            acc.add_cylinder(pos, 0.07 * scale, 2.5 * scale, 5, palette::METAL);
            acc.add_cone(pos + Vec3::Y * 2.3 * scale, 1.9 * scale, 0.7 * scale, 8, palette::CUSHION);
        }
        DecoKind::Umbrella => {
            acc.add_cylinder(pos, 0.1 * scale, 2.9 * scale, 5, palette::WOOD);
            acc.add_cone(pos + Vec3::Y * 2.6 * scale, 2.4 * scale, 0.9 * scale, 10, palette::FLOWER_RED);
        }
        DecoKind::SunLounger => {
            acc.add_box_rotated(
                pos + Vec3::Y * 0.35,
                Vec3::new(0.9, 0.18, 2.1),
                0.0,
                palette::WOOD_LIGHT,
            );
            acc.add_box_rotated(
                pos + Vec3::new(0.0, 0.62, -0.65),
                Vec3::new(0.9, 0.14, 0.8),
                -0.5,
                palette::CUSHION,
            );
        }
        DecoKind::Planter | DecoKind::PottedPlant => {
            acc.add_cylinder(pos, 0.45 * scale, 0.6 * scale, 6, palette::STONE_DARK);
            acc.add_uv_sphere(pos + Vec3::Y * 1.0 * scale, 0.55 * scale, 2, 6, palette::GRASS);
        }
        DecoKind::Bench => {
            acc.add_box_rotated(pos + Vec3::Y * 0.45, Vec3::new(2.2, 0.14, 0.6), 0.0, palette::WOOD);
            acc.add_box_bounds(
                pos + Vec3::new(-0.9, 0.0, -0.22),
                pos + Vec3::new(-0.75, 0.9, -0.1),
                palette::METAL,
            );
            acc.add_box_bounds(
                pos + Vec3::new(0.75, 0.0, -0.22),
                pos + Vec3::new(0.9, 0.9, -0.1),
                palette::METAL,
            );
        }
    }
}

/// Build the merged decoration mesh.
pub fn build_deco_mesh(layout: &ResortLayout) -> Mesh {
    mesh_kit::build_mesh(|acc| {
        for d in &layout.deco {
            add_deco(acc, d.kind, d.position, d.yaw, d.scale);
        }
    })
}

/// Spawn the whole level into the world.
pub fn spawn_level(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    layout: &ResortLayout,
) {
    let material = materials.add(mesh_kit::vertex_material());

    let ground = meshes.add(build_ground(layout));
    let solids = meshes.add(build_solid_mesh(&layout.solids));
    let deco = meshes.add(build_deco_mesh(layout));

    // Ground plate (no shadows cast, receives them).
    commands.spawn((
        LevelEntity,
        Name::new("Ground"),
        Mesh3d(ground.clone()),
        MeshMaterial3d(material.clone()),
        Transform::default(),
        bevy::pbr::NotShadowCaster,
    ));

    commands.spawn((
        LevelEntity,
        Name::new("Resort Architecture"),
        Mesh3d(solids.clone()),
        MeshMaterial3d(material.clone()),
        Transform::default(),
    ));

    commands.spawn((
        LevelEntity,
        Name::new("Resort Props"),
        Mesh3d(deco.clone()),
        MeshMaterial3d(material.clone()),
        Transform::default(),
        bevy::pbr::NotShadowCaster,
    ));

    commands.insert_resource(LevelAssets {
        solid_mesh: solids,
        deco_mesh: deco,
        ground_mesh: ground,
        material,
        glow_material: materials.add(mesh_kit::glow_material()),
    });
}

/// Static collision data shared with every movement system.
#[derive(Resource, Debug)]
pub struct WorldCollision {
    /// Solids that block horizontal movement.
    pub blocks: Vec<Block2>,
    /// Every surface an actor can stand on (obstacles + explicit floors).
    pub surfaces: Vec<Block2>,
    pub zones: Vec<ZoneDefinition>,
}

impl WorldCollision {
    pub fn from_layout(layout: &ResortLayout) -> Self {
        let blocks = layout.obstacle_blocks();
        // *Every* solid can be stood on — including low stair treads and desks
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
        let solids = build_solid_mesh(&layout.solids);
        let deco = build_deco_mesh(&layout);
        let ground = build_ground(&layout);
        assert!(solids.count_vertices() > 0);
        assert!(deco.count_vertices() > 0);
        assert!(ground.count_vertices() > 0);
        assert!(solids.indices().is_some());
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
        let mesh = build_solid_mesh(&layout.solids);
        let indices = mesh.indices().expect("solids have indices");
        assert_eq!(indices.len() % 3, 0, "triangle list must be complete");
    }
}
