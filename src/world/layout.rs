//! The "Coral Palm Resort" level definition.
//!
//! The resort is authored as data ([`ResortLayout`]) and instantiated by
//! [`super::builder`]. Keeping the layout as plain data means future maps can be
//! added without touching gameplay code.
//!
//! ## Authoring model
//!
//! Buildings are described with [`LayoutBuilder::building`], which emits the
//! full architectural shell: floor slab, walls with real openings (doorways and
//! windows with sills and lintels), and a ceiling. That replaced the original
//! approach of stacking disconnected boxes, which is what produced walls that
//! did not meet, rooms without ceilings, and the sense of an unfinished map.
//!
//! Every solid still doubles as collision: `is_obstacle` decides whether it
//! blocks walking actors, so window openings stay walk-blocking while sills and
//! lintels do not.

use crate::core::{Block2, ZoneDefinition, ZoneId, ZoneId::*, ZoneRect};
use bevy::prelude::*;

/// Zone definitions and heights used throughout the resort.
pub const GROUND_Y: f32 = 0.0;
pub const LOBBY_CEILING: f32 = 4.6;
pub const UPPER_CEILING: f32 = 9.0;
pub const RESTAURANT_DECK: f32 = 2.4;
/// Height of the penthouse terrace floor.
///
/// It sits one storey above the hotel roof deck (which tops out at 9.4 m) with
/// the floor slab visible below it, so the building reads as one structure. At
/// its previous value the terrace floated 20 m in the air.
pub const PENTHOUSE_Y: f32 = 13.4;
pub const SERVICE_Y: f32 = -3.4;

/// Timeless resort palette (approximate sRGB; textures carry the detail).
pub mod palette {
    use bevy::prelude::*;

    pub const SAND: Color = Color::srgb(0.87, 0.79, 0.62);
    pub const SAND_DARK: Color = Color::srgb(0.79, 0.71, 0.55);
    pub const SEA: Color = Color::srgb(0.16, 0.52, 0.62);
    pub const GRASS: Color = Color::srgb(0.36, 0.62, 0.34);
    pub const GRASS_DARK: Color = Color::srgb(0.29, 0.51, 0.28);
    pub const STONE: Color = Color::srgb(0.82, 0.80, 0.75);
    pub const STONE_DARK: Color = Color::srgb(0.66, 0.64, 0.60);
    pub const MARBLE: Color = Color::srgb(0.93, 0.91, 0.86);
    pub const WALL: Color = Color::srgb(0.89, 0.86, 0.80);
    pub const WOOD: Color = Color::srgb(0.46, 0.32, 0.21);
    pub const WOOD_LIGHT: Color = Color::srgb(0.62, 0.45, 0.29);
    pub const ROOF: Color = Color::srgb(0.55, 0.27, 0.22);
    pub const ROOF_DARK: Color = Color::srgb(0.42, 0.20, 0.18);
    pub const SERVICE: Color = Color::srgb(0.55, 0.57, 0.60);
    pub const SERVICE_DARK: Color = Color::srgb(0.40, 0.42, 0.45);
    pub const SECURITY: Color = Color::srgb(0.32, 0.36, 0.44);
    pub const GLASS: Color = Color::srgba(0.55, 0.78, 0.85, 0.85);
    pub const PALM_TRUNK: Color = Color::srgb(0.42, 0.30, 0.20);
    pub const PALM_LEAF: Color = Color::srgb(0.24, 0.55, 0.29);
    pub const PALM_LEAF_LIGHT: Color = Color::srgb(0.32, 0.66, 0.34);
    pub const FLOWER_RED: Color = Color::srgb(0.85, 0.25, 0.28);
    pub const FLOWER_YELLOW: Color = Color::srgb(0.95, 0.80, 0.25);
    pub const CUSHION: Color = Color::srgb(0.86, 0.52, 0.38);
    pub const METAL: Color = Color::srgb(0.70, 0.72, 0.74);
    pub const CABLE: Color = Color::srgb(0.15, 0.15, 0.17);
    pub const WARNING: Color = Color::srgb(0.90, 0.72, 0.15);
    pub const HOT_TUB: Color = Color::srgb(0.22, 0.48, 0.58);
}

/// A single static box of the world, doubling as a collision volume.
#[derive(Debug, Clone)]
pub struct SolidSpec {
    pub center: Vec3,
    pub size: Vec3,
    /// Surface material used when the geometry is built.
    pub material: crate::world::mesh_kit::MatId,
    /// Debug / interaction label.
    pub label: &'static str,
}

impl SolidSpec {
    pub fn new(
        center: Vec3,
        size: Vec3,
        material: crate::world::mesh_kit::MatId,
        label: &'static str,
    ) -> Self {
        Self {
            center,
            size,
            material,
            label,
        }
    }

    pub fn block(&self) -> Block2 {
        Block2::new(self.center, self.size)
    }

    /// Whether this solid blocks walking actors.
    ///
    /// Walls, doors and furniture do; flat floor plates, sills, lintels and
    /// ceilings do not (the player walks under them).
    pub fn is_obstacle(&self) -> bool {
        let top = self.center.y + self.size.y * 0.5;
        let bottom = self.center.y - self.size.y * 0.5;
        // Anything at head height and at least knee-high blocks movement.
        self.size.y > 0.9 && top > GROUND_Y + 0.9 && bottom < GROUND_Y + 1.8
    }
}

/// Kinds of decorative prop. Each is built from real geometry in
/// [`super::props`], not a single primitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DecoKind {
    Palm,
    Bush,
    FlowerBed,
    LampPost,
    Parasol,
    SunLounger,
    Planter,
    PottedPlant,
    Bench,
    Umbrella,
    // --- additions for the realistic pass ---
    /// Large shade tree with layered canopy.
    Tree,
    /// Dense tropical shrub cluster.
    ShrubCluster,
    /// Grass tuft patch.
    GrassTuft,
    /// Outdoor dining table with four chairs.
    DiningSet,
    /// Bar table with stools.
    BarSet,
    /// Reception desk unit.
    Reception,
    /// Sofa / lounge chair.
    Armchair,
    /// Hotel bed.
    Bed,
    /// Nightstand with lamp.
    Nightstand,
    /// Long dining/console table.
    ConsoleTable,
    /// Kitchen island / prep counter.
    Counter,
    /// Stacked service crates.
    Crates,
    /// Wall-mounted air conditioning unit.
    AirUnit,
    /// Life-ring post on the beach.
    LifeRing,
    /// Beach shower.
    BeachShower,
    /// Stone balustrade segment.
    Balustrade,
    /// Potted palm (indoor planter).
    IndoorPalm,
}

#[derive(Debug, Clone)]
pub struct DecoSpec {
    pub kind: DecoKind,
    pub position: Vec3,
    pub yaw: f32,
    pub scale: f32,
}

/// Door / entrance definition used by accessibility checks and navigation.
#[derive(Debug, Clone)]
pub struct DoorSpec {
    pub name: &'static str,
    /// Horizontal position of the door.
    pub position: Vec2,
    /// Half width of the doorway used to check reachability.
    pub width: f32,
    /// Zone on either side (used for trespass checks).
    pub zone: ZoneId,
}

/// A flat/interior detail piece with no collision.
#[derive(Debug, Clone)]
pub struct InteriorDetail {
    pub position: Vec3,
    pub size: Vec3,
    pub material: crate::world::mesh_kit::MatId,
}

/// The complete static level description.
#[derive(Debug, Clone)]
pub struct ResortLayout {
    pub name: &'static str,
    pub solids: Vec<SolidSpec>,
    pub deco: Vec<DecoSpec>,
    /// Non-colliding detail geometry (rugs, glazing, ceiling panels).
    pub details: Vec<InteriorDetail>,
    pub zones: Vec<ZoneDefinition>,
    pub doors: Vec<DoorSpec>,
    /// Water areas: actors cannot enter.
    pub water: Vec<Block2>,
    /// Walkable floor surfaces that are not part of [`Self::solids`].
    pub floors: Vec<Block2>,
    /// Player spawn point.
    pub player_spawn: Vec3,
    /// Interior light sources: position, colour, luminous power and range.
    pub lights: Vec<LightSpec>,
}

/// An interior light placed by the layout.
#[derive(Clone, Debug)]
pub struct LightSpec {
    pub position: Vec3,
    pub color: Color,
    /// Luminous power in lumens.
    pub lumens: f32,
    /// Cutoff distance in metres.
    pub range: f32,
}

impl ResortLayout {
    /// Look up the zone for a horizontal position.
    pub fn zone_at(&self, point: Vec2) -> ZoneId {
        let mut best: Option<&ZoneDefinition> = None;
        for zone in &self.zones {
            if zone.rect.contains(point) {
                match best {
                    Some(current) if current.priority > zone.priority => {}
                    _ => best = Some(zone),
                }
            }
        }
        best.map(|z| z.id).unwrap_or(Beach)
    }

    /// All solids that block walking actors.
    pub fn obstacle_blocks(&self) -> Vec<Block2> {
        let mut blocks: Vec<Block2> = self
            .solids
            .iter()
            .filter(|s| s.is_obstacle())
            .map(|s| s.block())
            .collect();
        blocks.extend(self.water.iter().copied());
        blocks
    }

    /// Rough world bounds used for minimap projection.
    pub fn bounds(&self) -> ZoneRect {
        ZoneRect::new(-74.0, -66.0, 74.0, 68.0)
    }
}

// ---------------------------------------------------------------------------
// Layout construction helpers
// ---------------------------------------------------------------------------

pub struct LayoutBuilder {
    pub solids: Vec<SolidSpec>,
    pub deco: Vec<DecoSpec>,
    pub doors: Vec<DoorSpec>,
    /// Non-colliding interior detail (rugs, ceiling panels, glass).
    pub details: Vec<InteriorDetail>,
    /// Interior light placements.
    pub lights: Vec<LightSpec>,
}

impl Default for LayoutBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutBuilder {
    pub fn new() -> Self {
        Self {
            solids: Vec::new(),
            deco: Vec::new(),
            doors: Vec::new(),
            details: Vec::new(),
            lights: Vec::new(),
        }
    }

    /// Add a solid from its XZ rectangle and vertical extent.
    pub fn slab(
        &mut self,
        name: &'static str,
        min: Vec2,
        max: Vec2,
        bottom: f32,
        top: f32,
        material: crate::world::mesh_kit::MatId,
    ) {
        let center = Vec3::new(
            (min.x + max.x) * 0.5,
            (bottom + top) * 0.5,
            (min.y + max.y) * 0.5,
        );
        let size = Vec3::new(max.x - min.x, top - bottom, max.y - min.y);
        self.solids.push(SolidSpec::new(center, size, material, name));
    }

    /// Wall running from `a` to `b` (axis aligned), `thickness` wide.
    pub fn wall(
        &mut self,
        name: &'static str,
        a: Vec2,
        b: Vec2,
        height: f32,
        material: crate::world::mesh_kit::MatId,
    ) {
        self.wall_with_base(name, a, b, 0.0, height, material);
    }

    pub fn wall_with_base(
        &mut self,
        name: &'static str,
        a: Vec2,
        b: Vec2,
        base: f32,
        height: f32,
        material: crate::world::mesh_kit::MatId,
    ) {
        let min = a.min(b);
        let max = a.max(b);
        self.slab(name, min, max, base, base + height, material);
    }

    /// A wall with a rectangular opening (door or window) cut out of it.
    ///
    /// The wall runs along its longer axis; the opening is centred on
    /// `opening_center` and spans `opening_width` along the wall, from
    /// `opening_bottom` to `opening_top`.
    #[allow(clippy::too_many_arguments)]
    pub fn wall_with_opening(
        &mut self,
        name: &'static str,
        a: Vec2,
        b: Vec2,
        thickness: f32,
        height: f32,
        opening_center: f32,
        opening_width: f32,
        opening_bottom: f32,
        opening_top: f32,
        material: crate::world::mesh_kit::MatId,
    ) {
        let horizontal = (b.x - a.x).abs() >= (b.y - a.y).abs();
        let (start, end, fixed) = if horizontal {
            (a.x.min(b.x), a.x.max(b.x), (a.y + b.y) * 0.5)
        } else {
            (a.y.min(b.y), a.y.max(b.y), (a.x + b.x) * 0.5)
        };
        let half = thickness * 0.5;
        let mut piece = |name: &'static str, from: f32, to: f32, bottom: f32, top: f32| {
            if to - from <= 0.001 || top - bottom <= 0.001 {
                return;
            }
            if horizontal {
                self.slab(name, Vec2::new(from, fixed - half), Vec2::new(to, fixed + half), bottom, top, material);
            } else {
                self.slab(name, Vec2::new(fixed - half, from), Vec2::new(fixed + half, to), bottom, top, material);
            }
        };

        let open_lo = (opening_center - opening_width * 0.5).max(start);
        let open_hi = (opening_center + opening_width * 0.5).min(end);
        // Sections either side of the opening, full height.
        piece(name, start, open_lo, 0.0, height);
        piece(name, open_hi, end, 0.0, height);
        // Below the opening (sill / door threshold) and above (lintel).
        if opening_bottom > 0.02 {
            piece(name, open_lo, open_hi, 0.0, opening_bottom);
        }
        if opening_top < height - 0.02 {
            piece(name, open_lo, open_hi, opening_top, height);
        }
    }

    /// Place an interior light. `lumens` is luminous power; `range` is the
    /// distance at which it fades out.
    pub fn light(&mut self, position: Vec3, color: Color, lumens: f32, range: f32) {
        self.lights.push(LightSpec {
            position,
            color,
            lumens,
            range,
        });
    }

    /// Non-colliding detail geometry (glazing, rugs, ceiling light panels).
    pub fn detail(&mut self, position: Vec3, size: Vec3, material: crate::world::mesh_kit::MatId) {
        self.details.push(InteriorDetail {
            position,
            size,
            material,
        });
    }

    /// A hedge block: solid (it blocks movement and is real cover) and drawn
    /// with the foliage material.
    pub fn hedge(&mut self, center: Vec2, size: Vec2, height: f32) {
        self.slab(
            "Garden Hedge",
            center - size * 0.5,
            center + size * 0.5,
            0.0,
            height,
            crate::world::mesh_kit::MatId::Hedge,
        );
    }

    pub fn deco(&mut self, kind: DecoKind, position: Vec3, yaw: f32, scale: f32) {
        self.deco.push(DecoSpec {
            kind,
            position,
            yaw,
            scale,
        });
    }

    pub fn door(&mut self, spec: DoorSpec) {
        self.doors.push(spec);
    }
}

/// Build the Coral Palm Resort layout.
///
/// Coordinates are metres; the site is roughly 148 x 134. Sea lies to the south
/// (−Z), the hotel in the middle, the gardens between them, the service level
/// under the west gardens, and the penthouse on the hotel roof.
pub fn resort_layout() -> ResortLayout {
    build::build()
}

pub mod build;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resort_has_content() {
        let l = resort_layout();
        assert!(l.solids.len() > 300, "expected a dense layout, got {}", l.solids.len());
        assert!(!l.deco.is_empty());
        assert!(!l.obstacle_blocks().is_empty());
        assert!(l.floors.len() >= 5);
    }

    #[test]
    fn zone_lookup_prefers_specific_areas() {
        let l = resort_layout();
        assert_eq!(l.zone_at(Vec2::new(0.0, 0.0)), Lobby);
        assert_eq!(l.zone_at(Vec2::new(40.0, 0.0)), Restaurant);
        assert_eq!(l.zone_at(Vec2::new(-55.0, -20.0)), SecurityWing);
        assert_eq!(l.zone_at(Vec2::new(-40.0, 10.0)), ServiceCorridor);
        assert_eq!(l.zone_at(Vec2::new(0.0, 40.0)), Penthouse);
        assert_eq!(l.zone_at(Vec2::new(0.0, -52.0)), Beach);
    }

    #[test]
    fn player_spawn_is_walkable() {
        let l = resort_layout();
        let spawn = l.player_spawn;
        let floor = crate::player::movement::ground_height(
            &l.solids.iter().map(|s| s.block()).chain(l.floors.iter().copied()).collect::<Vec<_>>(),
            Vec2::new(spawn.x, spawn.z),
            0.38,
            spawn.y,
        );
        assert!(floor.is_some_and(|f| f.abs() < 0.3), "spawn floor was {floor:?}");
    }

    #[test]
    fn every_solid_has_positive_extent() {
        for solid in resort_layout().solids {
            assert!(
                solid.size.x > 0.0 && solid.size.y > 0.0 && solid.size.z > 0.0,
                "{} has degenerate size {:?}",
                solid.label,
                solid.size
            );
        }
    }

    fn clear_at(solids: &[crate::world::layout::SolidSpec], x: f32, y: f32) -> bool {
        !solids.iter().any(|s| {
            let bottom = s.center.y - s.size.y * 0.5;
            let top = s.center.y + s.size.y * 0.5;
            (s.center.x - x).abs() < 0.6 && bottom < y && top > y
        })
    }

    #[test]
    fn doorways_are_walkable_and_windows_are_not() {
        // A doorway: opening from the floor to 2.6 m, so it must be clear at
        // chest height but still have a solid lintel above.
        let mut door = LayoutBuilder::new();
        door.wall_with_opening(
            "Door",
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.3),
            0.3,
            4.6,
            5.0,
            1.8,
            0.0,
            2.6,
            crate::world::mesh_kit::MatId::Plaster,
        );
        assert!(clear_at(&door.solids, 5.0, 1.2), "a doorway must be walkable");
        assert!(clear_at(&door.solids, 5.0, 0.2));
        assert!(!clear_at(&door.solids, 5.0, 3.6), "the lintel must remain solid");

        // A window: 1.1 m sill, 2.9 m head, so the wall blocks at knee height
        // and keeps a lintel and sill.
        let mut window = LayoutBuilder::new();
        window.wall_with_opening(
            "Window",
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.3),
            0.3,
            4.6,
            5.0,
            1.8,
            1.1,
            2.9,
            crate::world::mesh_kit::MatId::Plaster,
        );
        assert!(!clear_at(&window.solids, 5.0, 0.5), "the sill must be solid");
        assert!(!clear_at(&window.solids, 5.0, 3.6), "the lintel must be solid");
        // The glass itself occupies 1.1..2.9 m, so the wall is not see-through
        // at head height either: the pane is a detail entity, not a hole.
        assert!(clear_at(&window.solids, 5.0, 2.0), "the window aperture is open");
    }
}
