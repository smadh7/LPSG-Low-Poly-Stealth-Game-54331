//! The "Coral Palm Resort" level definition.
//!
//! The layout is authored as data (`ResortLayout`) and then instantiated by
//! [`super::builder`].  Keeping the layout as plain data means future maps can
//! be added without touching any gameplay code.

use crate::core::{Block2, ZoneDefinition, ZoneId, ZoneRect, ZoneId::*};
use bevy::prelude::*;

/// Palette used across the resort.
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

/// A single static box of the world. These double as collision volumes.
#[derive(Debug, Clone)]
pub struct SolidSpec {
    pub center: Vec3,
    pub size: Vec3,
    /// Base colour for the generated geometry.
    pub color: Color,
    /// Optional name for debugging / interaction hooks.
    pub label: &'static str,
}

impl SolidSpec {
    pub fn new(center: Vec3, size: Vec3, color: Color, label: &'static str) -> Self {
        Self {
            center,
            size,
            color,
            label,
        }
    }

    pub fn block(&self) -> Block2 {
        Block2::new(self.center, self.size)
    }

    /// Whether this solid should block walking actors (very flat solids do not).
    pub fn is_obstacle(&self) -> bool {
        self.size.y > 0.6 && self.center.y + self.size.y * 0.5 > 1.2
    }
}

/// A decorative mesh blob that is merged into a single draw call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

#[derive(Debug, Clone)]
pub struct DecoSpec {
    pub kind: DecoKind,
    pub position: Vec3,
    pub yaw: f32,
    pub scale: f32,
}

/// Door / entrance definition used by the accessibility system and navigation.
#[derive(Debug, Clone)]
pub struct DoorSpec {
    pub name: &'static str,
    /// Horizontal position of the door.
    pub position: Vec2,
    /// Half extents of the "doorway" area used to check reachability.
    pub width: f32,
    /// Zone on either side (used for trespass checks).
    pub zone: ZoneId,
}

/// The complete static level description.
#[derive(Debug, Clone)]
pub struct ResortLayout {
    pub name: &'static str,
    pub solids: Vec<SolidSpec>,
    pub deco: Vec<DecoSpec>,
    pub zones: Vec<ZoneDefinition>,
    pub doors: Vec<DoorSpec>,
    /// Water areas: actors cannot enter.
    pub water: Vec<Block2>,
    /// Player spawn point.
    pub player_spawn: Vec3,
}

impl ResortLayout {
    /// Look up the zone for a horizontal position.
    pub fn zone_at(&self, point: Vec2) -> ZoneId {
        let mut best: Option<(&ZoneDefinition, u8)> = None;
        for zone in &self.zones {
            if zone.rect.contains(point) {
                match best {
                    Some((current, prio)) if prio >= zone.priority && current.rect.size().length() <= zone.rect.size().length() => {}
                    _ => best = Some((zone, zone.priority)),
                }
            }
        }
        best.map(|(z, _)| z.id).unwrap_or(Beach)
    }

    /// All walkable static collision volumes.
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

struct LayoutBuilder {
    solids: Vec<SolidSpec>,
    deco: Vec<DecoSpec>,
    doors: Vec<DoorSpec>,
}

impl LayoutBuilder {
    fn new() -> Self {
        Self {
            solids: Vec::new(),
            deco: Vec::new(),
            doors: Vec::new(),
        }
    }

    /// Add a solid from its XZ bounding rectangle and vertical extent.
    fn slab(
        &mut self,
        name: &'static str,
        min: Vec2,
        max: Vec2,
        bottom: f32,
        top: f32,
        color: Color,
    ) {
        let center = Vec3::new((min.x + max.x) * 0.5, (bottom + top) * 0.5, (min.y + max.y) * 0.5);
        let size = Vec3::new(max.x - min.x, top - bottom, max.y - min.y);
        self.solids.push(SolidSpec::new(center, size, color, name));
    }

    fn wall(&mut self, name: &'static str, min: Vec2, max: Vec2, height: f32, color: Color) {
        self.slab(name, min, max, 0.0, height, color);
    }

    fn deco(&mut self, kind: DecoKind, position: Vec3, yaw: f32, scale: f32) {
        self.deco.push(DecoSpec {
            kind,
            position,
            yaw,
            scale,
        });
    }
}

/// Build the Coral Palm Resort layout. All coordinates are in metres; the map is
/// roughly 148 x 134 units and is fully enclosed by the sea / cliffs.
pub fn resort_layout() -> ResortLayout {
    let mut b = LayoutBuilder::new();
    use palette::*;

    // -----------------------------------------------------------------------
    // Ground plates (visual only, generated separately from collision blocks)
    // -----------------------------------------------------------------------
    // The beach and gardens are outdoor ground; the hotel sits in the middle.

    // -----------------------------------------------------------------------
    // HOTEL — main building. Two storeys, ~58 x 40 footprint.
    // Outer walls are 0.6 thick with doorway gaps.
    // -----------------------------------------------------------------------
    let hotel_min = Vec2::new(-29.0, -14.0);
    let hotel_max = Vec2::new(29.0, 26.0);
    let wall_t = 0.7;
    let h1 = 4.6; // storey height
    let h2 = 9.2; // top of the second storey

    // --- North wall (z = 26) with the gated Penthouse terrace stair behind it
    b.wall("Hotel North Wall L", Vec2::new(hotel_min.x, hotel_max.y - wall_t), Vec2::new(-4.5, hotel_max.y), h2, WALL);
    b.wall("Hotel North Wall R", Vec2::new(4.5, hotel_max.y - wall_t), Vec2::new(hotel_max.x, hotel_max.y), h2, WALL);
    // Lintel above the lobby's north door so the doorway itself stays walkable.
    b.slab(
        "Hotel North Door Lintel",
        Vec2::new(-4.5, hotel_max.y - wall_t),
        Vec2::new(4.5, hotel_max.y),
        3.6,
        h2,
        WALL,
    );

    // --- South wall (z = -14) faces the gardens. Guest entrance in the middle.
    b.wall("Hotel South Wall L", Vec2::new(hotel_min.x, hotel_min.y), Vec2::new(-5.0, hotel_min.y + wall_t), h2, WALL);
    b.wall("Hotel South Wall R", Vec2::new(5.0, hotel_min.y), Vec2::new(hotel_max.x, hotel_min.y + wall_t), h2, WALL);
    // Lintel above the guest entrance.
    let lintel = Vec2::new(-5.0, hotel_min.y);
    b.slab(
        "Hotel South Entrance Lintel",
        lintel,
        Vec2::new(5.0, hotel_min.y + wall_t),
        3.6,
        h2,
        WALL,
    );

    // --- West wall (service side) with a service door to the corridors.
    b.wall("Hotel West Wall N", Vec2::new(hotel_min.x, 2.0), Vec2::new(hotel_min.x + wall_t, hotel_max.y), h2, WALL);
    b.wall("Hotel West Wall S", Vec2::new(hotel_min.x, hotel_min.y), Vec2::new(hotel_min.x + wall_t, -2.0), h2, WALL);
    b.slab(
        "Hotel West Door Lintel",
        Vec2::new(hotel_min.x, -2.0),
        Vec2::new(hotel_min.x + wall_t, 2.0),
        3.4,
        h2,
        WALL,
    );

    // --- East wall (glass frontage towards the gardens)
    b.wall("Hotel East Wall", Vec2::new(hotel_max.x - wall_t, hotel_min.y), Vec2::new(hotel_max.x, hotel_max.y), h2, WALL);

    // --- Second storey floor slab (also the lobby ceiling). Leaves an opening
    // at the central atrium.
    b.slab(
        "Second Floor West",
        Vec2::new(hotel_min.x, hotel_min.y),
        Vec2::new(-7.0, hotel_max.y),
        h1,
        h1 + 0.5,
        MARBLE,
    );
    b.slab(
        "Second Floor East",
        Vec2::new(7.0, hotel_min.y),
        Vec2::new(hotel_max.x, hotel_max.y),
        h1,
        h1 + 0.5,
        MARBLE,
    );
    b.slab(
        "Second Floor South",
        Vec2::new(-7.0, hotel_min.y),
        Vec2::new(7.0, -2.0),
        h1,
        h1 + 0.5,
        MARBLE,
    );
    b.slab(
        "Second Floor North",
        Vec2::new(-7.0, 8.0),
        Vec2::new(7.0, hotel_max.y),
        h1,
        h1 + 0.5,
        MARBLE,
    );
    // Atrium railing around the opening (visual + safety collider).
    b.slab("Atrium Railing N", Vec2::new(-7.0, 7.8), Vec2::new(7.0, 8.0), h1 + 0.5, h1 + 1.6, WOOD);
    b.slab("Atrium Railing S", Vec2::new(-7.0, -2.0), Vec2::new(7.0, -1.8), h1 + 0.5, h1 + 1.6, WOOD);

    // --- Roof (visual)
    b.slab(
        "Hotel Roof",
        Vec2::new(hotel_min.x - 0.6, hotel_min.y - 0.6),
        Vec2::new(hotel_max.x + 0.6, hotel_max.y + 0.6),
        h2,
        h2 + 0.5,
        ROOF,
    );

    // --- Lobby reception desk & pillars
    b.slab("Reception Desk", Vec2::new(-8.0, 8.0), Vec2::new(0.0, 10.0), 0.0, 1.15, WOOD);
    b.slab("Reception Back Wall", Vec2::new(-8.0, 10.0), Vec2::new(0.0, 10.6), 0.0, 3.2, WOOD_LIGHT);
    for i in 0..4 {
        let x = -18.0 + i as f32 * 12.0;
        b.slab(
            "Lobby Pillar",
            Vec2::new(x - 0.7, 1.4),
            Vec2::new(x + 0.7, 2.8),
            0.0,
            h1,
            MARBLE,
        );
    }
    // Lobby lounge seating islands.
    b.slab("Lobby Lounge Sofa A", Vec2::new(-20.0, -6.0), Vec2::new(-13.0, -3.2), 0.0, 0.75, CUSHION);
    b.slab("Lobby Lounge Sofa B", Vec2::new(13.0, -6.0), Vec2::new(20.0, -3.2), 0.0, 0.75, CUSHION);

    // -----------------------------------------------------------------------
    // RESTAURANT — raised terrace (2.4 m) east of the lobby.
    // -----------------------------------------------------------------------
    let rest = 2.4;
    b.slab("Restaurant Deck", Vec2::new(31.0, -14.0), Vec2::new(56.0, 14.0), -0.1, rest, WOOD_LIGHT);
    b.wall("Restaurant North Wall", Vec2::new(31.0, 13.3), Vec2::new(56.0, 14.0), 4.4, WALL);
    if let Some(w) = b.solids.last_mut() {
        w.center.y += rest;
    }
    b.wall("Restaurant South Wall", Vec2::new(31.0, -14.0), Vec2::new(56.0, -13.3), 4.4, WALL);
    if let Some(w) = b.solids.last_mut() {
        w.center.y += rest;
    }
    b.wall("Restaurant East Wall", Vec2::new(55.3, -14.0), Vec2::new(56.0, 14.0), 4.4, WALL);
    if let Some(w) = b.solids.last_mut() {
        w.center.y += rest;
    }
    // West face opens onto the lobby through a doorway at z = -1..1.
    b.wall("Restaurant West Wall A", Vec2::new(31.0, -14.0), Vec2::new(31.7, -1.0), 4.4, WALL);
    if let Some(w) = b.solids.last_mut() {
        w.center.y += rest;
    }
    b.wall("Restaurant West Wall B", Vec2::new(31.0, 1.0), Vec2::new(31.7, 14.0), 4.4, WALL);
    if let Some(w) = b.solids.last_mut() {
        w.center.y += rest;
    }
    b.slab("Restaurant Door Lintel", Vec2::new(31.0, -1.0), Vec2::new(31.7, 1.0), rest + 3.0, rest + 4.4, WALL);
    b.slab("Restaurant Roof", Vec2::new(30.6, -14.4), Vec2::new(56.4, 14.4), rest + 4.4, rest + 4.9, ROOF);
    // Kitchen counter and partition in the north-east corner.
    b.slab("Kitchen Counter", Vec2::new(47.0, 6.0), Vec2::new(55.0, 8.0), rest, rest + 1.1, METAL);
    b.slab("Kitchen Partition", Vec2::new(46.6, 8.0), Vec2::new(47.4, 13.3), rest, rest + 4.4, SERVICE);
    // Tables.
    for (x, z) in [(36.0f32, -8.0f32), (42.0, -8.0), (36.0, 0.0), (42.0, 0.0), (36.0, 8.0), (42.0, 8.0)] {
        b.slab(
            "Restaurant Table",
            Vec2::new(x - 1.0, z - 1.0),
            Vec2::new(x + 1.0, z + 1.0),
            rest,
            rest + 0.78,
            MARBLE,
        );
    }
    // Stairs from the gardens up to the terrace (east side).
    for i in 0..10 {
        let z = -13.0 + i as f32 * 1.0;
        b.slab(
            "Restaurant Stair",
            Vec2::new(33.0, z),
            Vec2::new(35.0, z + 1.02),
            -0.1,
            (i as f32 + 1.0) * (rest / 10.0),
            STONE,
        );
    }

    // -----------------------------------------------------------------------
    // PENTHOUSE TERRACE — the target's private level on top of the hotel.
    // Elevated +29.8 m; reached by an external stair on the west face.
    // -----------------------------------------------------------------------
    let pent = 29.8;
    b.slab("Penthouse Terrace", Vec2::new(-18.0, 27.0), Vec2::new(18.0, 50.0), pent - 0.3, pent, MARBLE);
    b.slab("Penthouse Rail W", Vec2::new(-18.0, 27.0), Vec2::new(-17.4, 50.0), pent, pent + 1.1, MARBLE);
    b.slab("Penthouse Rail E", Vec2::new(17.4, 27.0), Vec2::new(18.0, 50.0), pent, pent + 1.1, MARBLE);
    b.slab("Penthouse Rail N", Vec2::new(-18.0, 49.4), Vec2::new(18.0, 50.0), pent, pent + 1.1, MARBLE);
    // Suite shell with a doorway facing south.
    for (min, max) in [(-9.0, -1.1), (1.1, 9.0)] {
        b.slab(
            "Penthouse Suite South",
            Vec2::new(min, 33.0),
            Vec2::new(max, 33.8),
            pent,
            pent + 4.0,
            WALL,
        );
    }
    b.slab("Penthouse Suite West", Vec2::new(-9.0, 33.0), Vec2::new(-8.2, 46.0), pent, pent + 4.0, WALL);
    b.slab("Penthouse Suite East", Vec2::new(8.2, 33.0), Vec2::new(9.0, 46.0), pent, pent + 4.0, WALL);
    b.slab("Penthouse Suite North", Vec2::new(-9.0, 45.2), Vec2::new(9.0, 46.0), pent, pent + 4.0, WALL);
    b.slab("Penthouse Suite Roof", Vec2::new(-9.4, 32.6), Vec2::new(9.4, 46.4), pent + 4.0, pent + 4.4, ROOF_DARK);
    // Terrace furniture.
    b.slab("Hot Tub", Vec2::new(9.0, 30.0), Vec2::new(15.0, 36.0), pent, pent + 0.8, HOT_TUB);
    b.slab("Penthouse Desk", Vec2::new(-4.0, 43.0), Vec2::new(4.0, 44.4), pent, pent + 1.1, WOOD);
    b.slab("Penthouse Safe", Vec2::new(5.5, 43.5), Vec2::new(7.0, 45.0), pent, pent + 2.0, SERVICE_DARK);
    b.deco(DecoKind::PottedPlant, Vec3::new(-16.0, pent, 28.5), 0.0, 1.0);
    b.deco(DecoKind::PottedPlant, Vec3::new(16.0, pent, 28.5), 0.0, 1.0);

    // External stair: 34 steps climbing from the gardens (z = 28) to the terrace.
    let steps = 34;
    for i in 0..steps {
        let y = i as f32 * (pent / steps as f32);
        let z = 28.0 + i as f32 * 0.6;
        b.slab(
            "Penthouse Stair",
            Vec2::new(-32.0, z),
            Vec2::new(-29.0, z + 0.62),
            y,
            y + 0.35,
            STONE,
        );
    }
    b.slab("Stair Landing", Vec2::new(-32.0, 48.0), Vec2::new(-18.0, 49.4), pent - 0.3, pent, STONE);
    // Security gate at the foot of the stairs (restricted access).
    b.slab("Stair Gate Post L", Vec2::new(-32.6, 27.2), Vec2::new(-31.6, 28.0), 0.0, 2.4, SECURITY);
    b.slab("Stair Gate Post R", Vec2::new(-29.4, 27.2), Vec2::new(-28.4, 28.0), 0.0, 2.4, SECURITY);

    // -----------------------------------------------------------------------
    // SERVICE CORRIDORS — below the gardens (floor at -3.4), staff only.
    // Reached by a ramp that starts just east of the western perimeter.
    // -----------------------------------------------------------------------
    let svc = -3.4;
    b.slab("Service Floor", Vec2::new(-48.0, -30.0), Vec2::new(-30.0, 22.0), svc - 0.2, svc, SERVICE_DARK);
    b.wall("Service Wall W", Vec2::new(-48.6, -30.0), Vec2::new(-48.0, 22.0), 6.0, SERVICE);
    if let Some(w) = b.solids.last_mut() {
        w.center.y += svc;
    }
    b.wall("Service Wall E A", Vec2::new(-30.0, -30.0), Vec2::new(-29.4, -14.0), 6.0, SERVICE);
    if let Some(w) = b.solids.last_mut() {
        w.center.y += svc;
    }
    b.wall("Service Wall N", Vec2::new(-48.0, 22.0), Vec2::new(-48.0 + 18.6, 22.6), 6.0, SERVICE);
    if let Some(w) = b.solids.last_mut() {
        w.center.y += svc;
    }
    b.wall("Service Wall S", Vec2::new(-48.0, -30.6), Vec2::new(-29.4, -30.0), 6.0, SERVICE);
    if let Some(w) = b.solids.last_mut() {
        w.center.y += svc;
    }
    // Ramp: 28 steps south from the gardens (x = -34) down into the corridor.
    for i in 0..28 {
        let y = -(i as f32 + 1.0) * (3.4 / 28.0);
        let z = -30.4 + i as f32 * 0.55;
        b.slab(
            "Service Ramp",
            Vec2::new(-35.0, z),
            Vec2::new(-31.0, z + 0.57),
            y,
            y + 0.3,
            STONE_DARK,
        );
    }
    b.slab("Service Crates A", Vec2::new(-46.0, -4.0), Vec2::new(-42.0, 0.0), svc, svc + 1.5, WOOD);
    b.slab("Service Crates B", Vec2::new(-46.0, -10.0), Vec2::new(-43.0, -7.5), svc, svc + 1.0, WOOD_LIGHT);
    b.slab("Service Lockers", Vec2::new(-47.6, 10.0), Vec2::new(-45.0, 16.0), svc, svc + 2.2, SERVICE);
    b.slab("Service Door Frame", Vec2::new(-30.0, -14.4), Vec2::new(-29.4, -13.6), svc, svc + 3.0, SERVICE_DARK);

    // -----------------------------------------------------------------------
    // SECURITY WING — attached to the service corridor. Restricted.
    // -----------------------------------------------------------------------
    b.slab("Security Floor", Vec2::new(-62.0, -30.0), Vec2::new(-48.0, -8.0), svc - 0.2, svc, SERVICE_DARK);
    b.wall("Security Wall N", Vec2::new(-62.0, -8.6), Vec2::new(-48.0, -8.0), 6.0, SECURITY);
    if let Some(w) = b.solids.last_mut() {
        w.center.y += svc;
    }
    b.wall("Security Wall S", Vec2::new(-62.0, -30.6), Vec2::new(-48.0, -30.0), 6.0, SECURITY);
    if let Some(w) = b.solids.last_mut() {
        w.center.y += svc;
    }
    b.wall("Security Wall W", Vec2::new(-62.6, -30.0), Vec2::new(-62.0, -8.0), 6.0, SECURITY);
    if let Some(w) = b.solids.last_mut() {
        w.center.y += svc;
    }
    // East wall has the doorway that links to the corridor.
    b.wall("Security Wall E", Vec2::new(-48.0, -30.0), Vec2::new(-47.4, -22.0), 6.0, SECURITY);
    if let Some(w) = b.solids.last_mut() {
        w.center.y += svc;
    }
    // CCTV control desks.
    b.slab("Camera Desk", Vec2::new(-60.0, -26.0), Vec2::new(-52.0, -23.5), svc, svc + 1.1, SERVICE);
    b.slab("Evidence Rack", Vec2::new(-61.6, -18.0), Vec2::new(-59.0, -12.0), svc, svc + 1.5, METAL);
    b.deco(DecoKind::PottedPlant, Vec3::new(-50.0, svc, -10.0), 0.0, 0.8);

    // -----------------------------------------------------------------------
    // STAFF ROOM / LOCKER ROOM (east of the lobby, behind the restaurant).
    // -----------------------------------------------------------------------
    b.slab("Staff Room Floor", Vec2::new(31.0, 16.0), Vec2::new(46.0, 26.0), -0.1, 0.1, WOOD_LIGHT);
    b.wall("Staff Room N", Vec2::new(31.0, 26.0), Vec2::new(46.0, 26.6), 3.2, WALL);
    b.wall("Staff Room E", Vec2::new(46.0, 16.0), Vec2::new(46.6, 26.6), 3.2, WALL);
    b.wall("Staff Room W", Vec2::new(31.0, 16.0), Vec2::new(31.6, 26.0), 3.2, WALL);
    b.slab("Staff Room Roof", Vec2::new(30.6, 15.6), Vec2::new(46.6, 26.6), 3.2, 3.6, ROOF_DARK);
    b.slab("Staff Lockers", Vec2::new(43.0, 17.0), Vec2::new(45.6, 25.0), 0.0, 2.2, SERVICE);
    b.slab("Staff Table", Vec2::new(35.0, 20.0), Vec2::new(38.0, 22.0), 0.0, 0.8, WOOD);

    // -----------------------------------------------------------------------
    // GARDENS — south of the hotel. Hedges create a hedge maze.
    // -----------------------------------------------------------------------
    let hedge = GRASS_DARK;
    let hedges: &[(f32, f32, f32, f32)] = &[
        (-40.0, -20.0, -10.0, -19.0),
        (-2.0, -20.0, 30.0, -19.0),
        (-40.0, -30.0, -39.0, -20.0),
        (29.0, -30.0, 30.0, -20.0),
        (-28.0, -32.0, -10.0, -31.0),
        (2.0, -32.0, 20.0, -31.0),
        (-28.0, -42.0, -27.0, -32.0),
        (19.0, -42.0, 20.0, -32.0),
        (-14.0, -44.0, 14.0, -43.0),
        (-14.0, -44.0, -13.0, -36.0),
        (13.0, -44.0, 14.0, -36.0),
    ];
    for (min_x, min_z, max_x, max_z) in hedges {
        b.slab("Hedge", Vec2::new(*min_x, *min_z), Vec2::new(*max_x, *max_z), 0.0, 1.5, hedge);
    }
    // Fountain in the garden centre.
    b.slab("Fountain Base", Vec2::new(-4.0, -26.0), Vec2::new(6.0, -24.0), 0.0, 0.7, STONE);
    b.slab("Fountain Pillar", Vec2::new(0.0, -25.6), Vec2::new(2.0, -24.4), 0.7, 2.2, STONE_DARK);
    b.deco(DecoKind::FlowerBed, Vec3::new(-20.0, 0.0, -22.0), 0.0, 1.0);
    b.deco(DecoKind::FlowerBed, Vec3::new(12.0, 0.0, -22.0), 0.0, 1.0);
    b.deco(DecoKind::FlowerBed, Vec3::new(-24.0, 0.0, -38.0), 0.0, 1.0);
    b.deco(DecoKind::FlowerBed, Vec3::new(10.0, 0.0, -48.0), 0.0, 1.0);
    b.deco(DecoKind::Bench, Vec3::new(-8.0, 0.0, -20.6), 0.0, 1.0);
    b.deco(DecoKind::Bench, Vec3::new(8.0, 0.0, -20.6), 0.0, 1.0);

    // Palm trees around the gardens & beach.
    let palms: &[(f32, f32, f32)] = &[
        (-44.0, -24.0, 1.1),
        (-44.0, -36.0, 0.9),
        (-16.0, -38.0, 1.2),
        (24.0, -38.0, 1.0),
        (24.0, -24.0, 1.15),
        (-34.0, -48.0, 1.0),
        (34.0, -48.0, 1.05),
        (-52.0, -18.0, 0.95),
        (52.0, -20.0, 1.0),
        (-56.0, 10.0, 1.0),
        (58.0, 12.0, 0.9),
        (-44.0, 34.0, 1.05),
        (40.0, 34.0, 1.1),
    ];
    for (x, z, s) in palms {
        b.deco(DecoKind::Palm, Vec3::new(*x, 0.0, *z), (*x).atan2(*z), *s);
    }

    // -----------------------------------------------------------------------
    // BEACH — south edge of the map with parasols and loungers.
    // -----------------------------------------------------------------------
    for i in 0..9 {
        let x = -52.0 + i as f32 * 13.0;
        b.deco(DecoKind::Parasol, Vec3::new(x, 0.12, -54.0), i as f32, 1.0);
        b.deco(DecoKind::SunLounger, Vec3::new(x + 2.4, 0.12, -54.0), 0.0, 1.0);
    }
    b.deco(DecoKind::Umbrella, Vec3::new(-30.0, 0.12, -50.0), 0.0, 1.0);

    // Lobby & restaurant decorative lamps.
    for (x, z) in [(-22.0f32, 20.0f32), (22.0, 20.0), (-22.0, -10.0), (22.0, -10.0)] {
        b.deco(DecoKind::LampPost, Vec3::new(x, 0.0, z), 0.0, 1.0);
    }
    for i in 0..6 {
        b.deco(DecoKind::LampPost, Vec3::new(-50.0 + i as f32 * 20.0, 0.0, -16.0), 0.0, 1.0);
    }

    // -----------------------------------------------------------------------
    // Doors (also used for navigation + accessibility hints)
    // -----------------------------------------------------------------------
    b.doors.push(DoorSpec { name: "Lobby Entrance", position: Vec2::new(0.0, -14.3), width: 5.0, zone: Lobby });
    b.doors.push(DoorSpec { name: "Lobby Terrace Door", position: Vec2::new(0.0, 26.3), width: 4.5, zone: Penthouse });
    b.doors.push(DoorSpec { name: "Restaurant Terrace Door", position: Vec2::new(31.3, -1.0), width: 3.0, zone: Restaurant });
    b.doors.push(DoorSpec { name: "Service Door", position: Vec2::new(-29.3, 0.0), width: 4.0, zone: ServiceCorridor });
    b.doors.push(DoorSpec { name: "Security Door", position: Vec2::new(-47.7, -26.0), width: 4.0, zone: SecurityWing });
    b.doors.push(DoorSpec { name: "Staff Room Door", position: Vec2::new(31.3, 21.0), width: 3.0, zone: ServiceCorridor });

    let zones = vec![
        ZoneDefinition { id: Beach, rect: ZoneRect::new(-74.0, -66.0, 74.0, -46.0), priority: 1 },
        ZoneDefinition { id: Gardens, rect: ZoneRect::new(-74.0, -46.0, 74.0, -14.0), priority: 2 },
        ZoneDefinition { id: Lobby, rect: ZoneRect::new(-29.0, -14.0, 29.0, 26.0), priority: 6 },
        ZoneDefinition { id: Restaurant, rect: ZoneRect::new(29.0, -14.0, 56.0, 14.0), priority: 5 },
        ZoneDefinition { id: ServiceCorridor, rect: ZoneRect::new(-48.0, -30.0, -29.0, 22.0), priority: 7 },
        ZoneDefinition { id: SecurityWing, rect: ZoneRect::new(-62.0, -30.0, -47.0, -8.0), priority: 9 },
        ZoneDefinition { id: Penthouse, rect: ZoneRect::new(-32.0, 26.0, 18.0, 50.0), priority: 8 },
    ];

    ResortLayout {
        name: "Coral Palm Resort",
        solids: b.solids,
        deco: b.deco,
        zones,
        doors: b.doors,
        water: vec![
            // The sea beyond the beach.
            Block2::new(Vec3::new(0.0, -1.0, -62.0), Vec3::new(160.0, 2.0, 10.0)),
        ],
        // On the marble plaza just south of the lobby doors.
        player_spawn: Vec3::new(0.0, 0.05, -17.5),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_has_expected_structures() {
        let l = resort_layout();
        assert!(l.solids.len() > 60, "resort should have many solids, got {}", l.solids.len());
        assert!(!l.deco.is_empty());
        assert_eq!(l.zones.len(), 7);
        assert!(!l.obstacle_blocks().is_empty());
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
        assert_eq!(l.zone_at(Vec2::new(spawn.x, spawn.z)), Gardens);
        for block in l.obstacle_blocks() {
            let c = crate::core::Circle2 {
                center: Vec2::new(spawn.x, spawn.z),
                radius: 0.4,
            };
            assert!(
                block.resolve_circle(c).is_none(),
                "player spawn must not be inside a solid"
            );
        }
    }
}
