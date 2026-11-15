//! Geometry authoring for the Coral Palm Resort.
//!
//! Everything here is measured in metres and built from [`LayoutBuilder`]
//! primitives. The site is laid out as a real beach resort would be: arrival
//! drive and porte-cochère on the north, a double-height lobby with a mezzanine,
//! a raised restaurant terrace to the east, guest wings upstairs, a lagoon pool
//! and beach gardens to the south, service and security levels underground, and
//! the owner's penthouse on the roof.

use super::{
    palette::*, DecoKind, DoorSpec, LayoutBuilder, ResortLayout, LOBBY_CEILING, PENTHOUSE_Y,
    RESTAURANT_DECK, SERVICE_Y, UPPER_CEILING,
};
use crate::core::ZoneId::*;
use crate::world::mesh_kit::MatId;
use bevy::prelude::*;

pub fn build() -> ResortLayout {
    let mut b = LayoutBuilder::new();

    // =======================================================================
    // SITE: ground, beach, sea
    // =======================================================================
    let mut floors = Vec::new();
    let mut water = Vec::new();

    // Outdoor ground is built as plates around the below-ground service level
    // (x -62.6..-29, z -31..22.6) so that space is genuinely open to the sky.
    let plate = |min: Vec2, max: Vec2, top: f32| {
        crate::core::Block2::new(
            Vec3::new((min.x + max.x) * 0.5, (top - 2.0) * 0.5, (min.y + max.y) * 0.5),
            Vec3::new(max.x - min.x, top + 2.0, max.y - min.y),
        )
    };
    floors.push(plate(Vec2::new(-74.0, -34.0), Vec2::new(74.0, 62.0), 0.0));
    floors.push(plate(Vec2::new(-74.0, -31.0), Vec2::new(-62.6, 22.6), -3.4));
    floors.push(plate(Vec2::new(-29.0, -31.0), Vec2::new(74.0, 22.6), 0.0));

    // Beach slopes gently into the lagoon (south).
    b.slab("Beach", Vec2::new(-74.0, -62.0), Vec2::new(74.0, -34.0), -2.4, 0.0, MatId::Sand);
    floors.push(plate(Vec2::new(-74.0, -62.0), Vec2::new(74.0, -34.0), 0.0));

    // Lagoon: water plate + collision so actors do not walk into the sea.
    b.slab("Lagoon", Vec2::new(-90.0, -96.0), Vec2::new(90.0, -58.0), -2.6, -0.35, MatId::Water);
    water.push(crate::core::Block2::new(
        Vec3::new(0.0, -1.5, -77.0),
        Vec3::new(180.0, 3.0, 38.0),
    ));

    // =======================================================================
    // ARRIVAL: drive, porte-cochere, forecourt
    // =======================================================================
    b.slab("Arrival Drive", Vec2::new(-13.0, 26.0), Vec2::new(13.0, 62.0), 0.0, 0.06, MatId::Asphalt);
    b.detail(Vec3::new(0.0, 0.07, 44.0), Vec3::new(26.0, 0.02, 36.0), MatId::Asphalt);
    b.slab("Forecourt Kerb L", Vec2::new(-14.0, 26.0), Vec2::new(-13.0, 62.0), 0.0, 0.18, MatId::Limestone);
    b.slab("Forecourt Kerb R", Vec2::new(13.0, 26.0), Vec2::new(14.0, 62.0), 0.0, 0.18, MatId::Limestone);

    // Porte-cochère: four travertine columns and a cantilevered roof.
    for z in [30.0_f32, 34.0] {
        for x in [-9.0_f32, 9.0] {
            b.slab("Porte Column", Vec2::new(x - 0.55, z - 0.55), Vec2::new(x + 0.55, z + 0.55), 0.0, 5.2, MatId::Travertine);
        }
    }
    b.slab("Porte Roof", Vec2::new(-11.0, 27.0), Vec2::new(11.0, 37.0), 5.2, 5.8, MatId::Travertine);
    b.slab("Porte Roof Fascia", Vec2::new(-11.2, 26.8), Vec2::new(11.2, 37.2), 5.1, 5.3, MatId::DarkTrim);
    b.deco(DecoKind::Planter, Vec3::new(-8.0, 0.0, 31.0), 0.0, 1.3);
    b.deco(DecoKind::Planter, Vec3::new(8.0, 0.0, 31.0), 0.0, 1.3);
    b.deco(DecoKind::LampPost, Vec3::new(-11.5, 0.0, 40.0), 0.0, 1.0);
    b.deco(DecoKind::LampPost, Vec3::new(11.5, 0.0, 40.0), 0.0, 1.0);

    // =======================================================================
    // MAIN HOTEL: two storeys, 58 x 40 footprint, centred on the origin
    // =======================================================================
    let hotel_min = Vec2::new(-29.0, -14.0);
    let hotel_max = Vec2::new(29.0, 26.0);
    let t = 0.45; // wall thickness

    // --- Floors -------------------------------------------------------
    b.slab("Lobby Floor", hotel_min, hotel_max, -0.4, 0.0, MatId::Marble);
    b.slab("Upper Floor", hotel_min, hotel_max, LOBBY_CEILING, LOBBY_CEILING + 0.35, MatId::Limestone);
    floors.push(crate::core::Block2::new(
        Vec3::new(0.0, -0.2, 6.0),
        Vec3::new(58.0, 0.4, 40.0),
    ));
    floors.push(crate::core::Block2::new(
        Vec3::new(0.0, LOBBY_CEILING + 0.17, 6.0),
        Vec3::new(58.0, 0.35, 40.0),
    ));

    // --- Roof ---------------------------------------------------------
    // A flat accessible roof, not a hip roof: the penthouse terrace sits on it
    // and the external stair climbs the west face. A pitched roof would have
    // buried (and visually deleted) the top floor.
    b.slab("Hotel Roof Deck", Vec2::new(-29.4, -14.4), Vec2::new(29.4, 26.4), UPPER_CEILING, UPPER_CEILING + 0.4, MatId::Concrete);
    // Parapet around the roof edge.
    b.slab("Roof Parapet S", Vec2::new(-29.4, -14.4), Vec2::new(29.4, -13.8), UPPER_CEILING + 0.4, UPPER_CEILING + 1.0, MatId::Concrete);
    b.slab("Roof Parapet N", Vec2::new(-29.4, 25.8), Vec2::new(29.4, 26.4), UPPER_CEILING + 0.4, UPPER_CEILING + 1.0, MatId::Concrete);
    b.slab("Roof Parapet W", Vec2::new(-29.4, -14.4), Vec2::new(-28.8, 26.4), UPPER_CEILING + 0.4, UPPER_CEILING + 1.0, MatId::Concrete);
    b.slab("Roof Parapet E", Vec2::new(28.8, -14.4), Vec2::new(29.4, 26.4), UPPER_CEILING + 0.4, UPPER_CEILING + 1.0, MatId::Concrete);
    // Roof plant: AC units and vents (visible from the terrace).
    for x in [-22.0_f32, 22.0] {
        for z in [0.0_f32, 12.0] {
            b.deco(DecoKind::AirUnit, Vec3::new(x, UPPER_CEILING + 0.4, z), 0.0, 1.6);
        }
    }
    b.slab("Roof Plant Room", Vec2::new(19.0, 18.0), Vec2::new(27.0, 25.0), UPPER_CEILING + 0.4, UPPER_CEILING + 3.4, MatId::Concrete);
    b.slab("Roof Plant Roof", Vec2::new(18.6, 17.6), Vec2::new(27.4, 25.4), UPPER_CEILING + 3.4, UPPER_CEILING + 3.8, MatId::Slate);

    // --- Exterior walls with real window openings ---------------------
    // South facade: guest entrance (4 m) plus windows either side.
    b.wall_with_opening("South Wall", Vec2::new(-29.0, -14.0), Vec2::new(29.0, -14.0 + t), t, UPPER_CEILING, 0.0, 4.4, 0.0, 3.4, MatId::Travertine);
    for x in [-22.0_f32, -16.0, -10.0, 10.0, 16.0, 22.0] {
        b.wall_with_opening("South Window", Vec2::new(x - 1.9, -13.9), Vec2::new(x + 1.9, -13.9 + t), t, UPPER_CEILING, x, 2.6, 1.1, 2.9, MatId::Travertine);
        // Glass pane + frame for the ground floor window.
        b.detail(Vec3::new(x, 2.0, -13.7), Vec3::new(2.6, 1.8, 0.06), MatId::Glass);
        b.slab("Window Sill", Vec2::new(x - 1.9, -14.0), Vec2::new(x + 1.9, -13.5), 1.05, 1.15, MatId::Limestone);
    }

    // North facade (arrival side): main entrance and window band.
    b.wall_with_opening("North Wall", Vec2::new(-29.0, 26.0 - t), Vec2::new(29.0, 26.0), t, UPPER_CEILING, 0.0, 5.0, 0.0, 3.6, MatId::Travertine);
    for x in [-24.0_f32, -17.0, 17.0, 24.0] {
        b.wall_with_opening("North Window", Vec2::new(x - 1.7, 25.9 - t), Vec2::new(x + 1.7, 25.9), t, UPPER_CEILING, x, 2.4, 1.1, 2.9, MatId::Travertine);
        b.detail(Vec3::new(x, 2.0, 25.8), Vec3::new(2.4, 1.8, 0.06), MatId::Glass);
    }

    // West and east facades.
    b.wall_with_opening("West Wall", Vec2::new(-29.0, -14.0), Vec2::new(-29.0 + t, 26.0), t, UPPER_CEILING, 6.0, 3.2, 1.1, 2.9, MatId::Travertine);
    b.wall_with_opening("East Wall", Vec2::new(29.0 - t, -14.0), Vec2::new(29.0, 26.0), t, UPPER_CEILING, -1.0, 3.0, 0.0, 3.2, MatId::Travertine);
    for z in [-10.0_f32, -4.0, 10.0, 16.0, 22.0] {
        b.detail(Vec3::new(-28.7, 2.0, z), Vec3::new(0.06, 1.8, 2.4), MatId::Glass);
        b.detail(Vec3::new(28.7, 2.0, z), Vec3::new(0.06, 1.8, 2.4), MatId::Glass);
        // Upper-storey windows read from outside.
        b.detail(Vec3::new(-28.7, 7.0, z), Vec3::new(0.06, 1.9, 2.2), MatId::Glass);
        b.detail(Vec3::new(28.7, 7.0, z), Vec3::new(0.06, 1.9, 2.2), MatId::Glass);
    }

    // --- Interior: lobby shell, mezzanine, reception -------------------
    // Double-height lobby in the centre; side wings are enclosed rooms.
    b.slab("Lobby Inner Wall W", Vec2::new(-12.0, -10.0), Vec2::new(-11.4, 22.0), 0.0, LOBBY_CEILING, MatId::Plaster);
    b.slab("Lobby Inner Wall E", Vec2::new(11.4, -10.0), Vec2::new(12.0, 22.0), 0.0, LOBBY_CEILING, MatId::Plaster);
    b.wall_with_opening("Lobby Inner S", Vec2::new(-12.0, -10.0), Vec2::new(12.0, -9.4), 0.6, LOBBY_CEILING, 0.0, 3.0, 0.0, 3.0, MatId::Plaster);
    b.wall_with_opening("Lobby Inner N", Vec2::new(-12.0, 21.4), Vec2::new(12.0, 22.0), 0.6, LOBBY_CEILING, 0.0, 3.4, 0.0, 3.2, MatId::Plaster);

    // Mezzanine over the lobby's north half, with a glass balustrade.
    b.slab("Mezzanine", Vec2::new(-11.4, 12.0), Vec2::new(11.4, 21.4), LOBBY_CEILING - 0.35, LOBBY_CEILING, MatId::Wood);
    b.slab("Mezzanine Rail", Vec2::new(-11.4, 11.8), Vec2::new(11.4, 12.0), LOBBY_CEILING, LOBBY_CEILING + 1.05, MatId::Glass);
    b.slab("Mezzanine Rail Top", Vec2::new(-11.4, 11.75), Vec2::new(11.4, 12.05), LOBBY_CEILING + 1.05, LOBBY_CEILING + 1.12, MatId::Brass);

    // Lobby ceiling with recessed light coffers.
    b.slab("Lobby Ceiling", Vec2::new(-12.0, -10.0), Vec2::new(12.0, 12.0), LOBBY_CEILING, LOBBY_CEILING + 0.3, MatId::Plaster);
    for x in [-7.0_f32, 0.0, 7.0] {
        for z in [-6.0_f32, 0.0, 6.0] {
            b.detail(Vec3::new(x, LOBBY_CEILING - 0.08, z), Vec3::new(3.2, 0.08, 2.0), MatId::WhitePaint);
        }
    }
    // Feature chandelier over the lobby centre.
    b.detail(Vec3::new(0.0, LOBBY_CEILING - 0.9, 2.0), Vec3::new(2.6, 0.25, 2.6), MatId::Brass);

    // Marble carpet inlays and the reception area.
    b.detail(Vec3::new(0.0, 0.012, -2.0), Vec3::new(9.0, 0.02, 9.0), MatId::Marble);
    b.slab("Reception Desk", Vec2::new(-7.6, 6.4), Vec2::new(-2.4, 7.6), 0.0, 1.12, MatId::Wood);
    b.detail(Vec3::new(-5.0, 1.14, 7.0), Vec3::new(5.0, 0.04, 1.1), MatId::Marble);
    b.slab("Reception Back Wall", Vec2::new(-7.8, 8.2), Vec2::new(-2.2, 8.6), 0.0, 3.2, MatId::Wood);
    b.deco(DecoKind::Reception, Vec3::new(-5.0, 0.0, 7.0), 0.0, 1.0);

    // Lobby columns (structural, 0.8 m square) marking the atrium.
    for (x, z) in [(-7.0_f32, -4.0_f32), (7.0, -4.0), (-7.0, 8.0), (7.0, 8.0)] {
        b.slab("Lobby Column Base", Vec2::new(x - 0.55, z - 0.55), Vec2::new(x + 0.55, z + 0.55), 0.0, 0.35, MatId::Marble);
        b.slab("Lobby Column", Vec2::new(x - 0.4, z - 0.4), Vec2::new(x + 0.4, z + 0.4), 0.35, LOBBY_CEILING, MatId::Marble);
    }

    // Lobby furniture: sofas, coffee tables, planters.
    for (x, z, yaw) in [(-8.0_f32, -7.0_f32, 0.0_f32), (8.0, -7.0, std::f32::consts::PI), (0.0, 9.0, std::f32::consts::FRAC_PI_2)] {
        b.deco(DecoKind::Armchair, Vec3::new(x, 0.0, z), yaw, 1.0);
    }
    b.deco(DecoKind::ConsoleTable, Vec3::new(0.0, 0.0, -6.0), 0.0, 1.0);
    b.deco(DecoKind::IndoorPalm, Vec3::new(-10.0, 0.0, -8.5), 0.0, 1.2);
    b.deco(DecoKind::IndoorPalm, Vec3::new(10.0, 0.0, -8.5), 0.0, 1.2);
    b.deco(DecoKind::IndoorPalm, Vec3::new(-9.0, 0.0, 18.0), 0.0, 1.0);
    b.deco(DecoKind::IndoorPalm, Vec3::new(9.0, 0.0, 18.0), 0.0, 1.0);
    b.deco(DecoKind::Bench, Vec3::new(-14.0, 0.0, 2.0), std::f32::consts::FRAC_PI_2, 1.0);
    b.deco(DecoKind::Bench, Vec3::new(14.0, 0.0, 2.0), std::f32::consts::FRAC_PI_2, 1.0);

    // --- Guest wing rooms (ground + upper floor) ----------------------
    // Rooms on both sides of the lobby, each with its own door and window.
    for (side, x0) in [("W", -29.0_f32), ("E", 20.0)] {
        for i in 0..3 {
            let z0 = -12.0 + i as f32 * 8.0;
            let z1 = z0 + 7.4;
            b.slab(
                if side == "W" { "Guest Room Divider" } else { "Guest Room Divider" },
                Vec2::new(x0, z1),
                Vec2::new(x0 + 9.0, z1 + 0.3),
                0.0,
                LOBBY_CEILING,
                MatId::Plaster,
            );
            b.wall_with_opening(
                "Guest Room Door",
                Vec2::new(x0 + 8.7, z0 + 3.2),
                Vec2::new(x0 + 9.0, z0 + 4.4),
                0.3,
                LOBBY_CEILING,
                z0 + 3.8,
                1.1,
                0.0,
                2.5,
                MatId::Plaster,
            );
            // Furnish each room.
            let cx = x0 + 4.5;
            b.deco(DecoKind::Bed, Vec3::new(cx, 0.0, z0 + 1.8), 0.0, 1.0);
            b.deco(DecoKind::Nightstand, Vec3::new(cx + 1.6, 0.0, z0 + 1.1), 0.0, 1.0);
            b.deco(DecoKind::ConsoleTable, Vec3::new(cx, 0.0, z1 - 1.3), 0.0, 0.8);
            b.deco(DecoKind::Armchair, Vec3::new(cx - 2.6, 0.0, z1 - 2.4), 2.4, 0.9);
            // Upper storey: same rooms one floor up (walls only; the camera
            // rarely sees inside, but the windows must line up).
            b.slab(
                "Upper Room Divider",
                Vec2::new(x0, z1),
                Vec2::new(x0 + 9.0, z1 + 0.3),
                LOBBY_CEILING + 0.35,
                UPPER_CEILING,
                MatId::Plaster,
            );
        }
        // Corridor wall separating rooms from the lobby, with door openings.
        b.wall_with_opening(
            "Guest Corridor Wall",
            Vec2::new(x0 + 8.7, -12.0),
            Vec2::new(x0 + 9.0, 10.2),
            0.3,
            LOBBY_CEILING,
            -8.2,
            1.2,
            0.0,
            2.4,
            MatId::Plaster,
        );
    }
    // Guest corridor signage and lighting.
    for z in [-9.0_f32, -1.0, 7.0] {
        b.detail(Vec3::new(-19.6, LOBBY_CEILING - 0.15, z), Vec3::new(0.9, 0.1, 0.9), MatId::WhitePaint);
        b.detail(Vec3::new(19.6, LOBBY_CEILING - 0.15, z), Vec3::new(0.9, 0.1, 0.9), MatId::WhitePaint);
    }

    // =======================================================================
    // RESTAURANT TERRACE: raised 2.4 m, east of the hotel, sea view
    // =======================================================================
    let deck = RESTAURANT_DECK;
    b.slab("Restaurant Deck", Vec2::new(31.0, -14.0), Vec2::new(58.0, 14.0), deck - 0.4, deck, MatId::Wood);
    floors.push(crate::core::Block2::new(
        Vec3::new(44.5, deck - 0.2, 0.0),
        Vec3::new(27.0, 0.4, 28.0),
    ));
    // Retaining wall around the deck (stone, walkable top not needed).
    b.slab("Deck Retaining S", Vec2::new(31.0, -14.4), Vec2::new(58.0, -14.0), -0.2, deck, MatId::StoneWall);
    b.slab("Deck Retaining E", Vec2::new(58.0, -14.4), Vec2::new(58.4, 14.0), -0.2, deck, MatId::StoneWall);
    // Balustrade along the east and south edges (stone posts + rail).
    for i in 0..9 {
        let z = -13.0 + i as f32 * 3.4;
        b.slab("Deck Baluster", Vec2::new(57.6, z), Vec2::new(58.0, z + 0.3), deck, deck + 0.95, MatId::StoneWall);
    }
    b.slab("Deck Rail", Vec2::new(57.5, -14.0), Vec2::new(58.0, 14.0), deck + 0.95, deck + 1.1, MatId::StoneWall);

    // Restaurant building on the deck: kitchen wall, roof, open sea-facing side.
    b.slab("Restaurant Back Wall", Vec2::new(54.5, -14.0), Vec2::new(58.0, 14.0), deck, deck + 3.6, MatId::Travertine);
    b.slab("Restaurant Roof", Vec2::new(31.0, -14.0), Vec2::new(58.0, 14.0), deck + 3.6, deck + 4.0, MatId::Roof);
    b.slab("Restaurant Roof Beam", Vec2::new(31.0, -14.0), Vec2::new(31.4, 14.0), deck + 3.1, deck + 3.6, MatId::Wood);
    for z in [-14.0_f32, -7.0, 0.0, 7.0, 14.0] {
        b.slab("Restaurant Post", Vec2::new(31.0, z - 0.2), Vec2::new(31.5, z + 0.2), deck, deck + 3.6, MatId::Wood);
    }
    // Kitchen interior at the back of the restaurant.
    b.slab("Kitchen Counter", Vec2::new(50.0, -10.0), Vec2::new(54.0, 10.0), deck, deck + 0.95, MatId::Metal);
    b.detail(Vec3::new(52.0, deck + 0.97, 0.0), Vec3::new(3.8, 0.04, 19.6), MatId::Metal);
    for z in [-9.0_f32, -3.0, 3.0, 9.0] {
        b.deco(DecoKind::Counter, Vec3::new(48.0, deck, z), 0.0, 1.0);
    }
    // Dining: tables with chairs across the terrace.
    for (i, (x, z)) in [
        (35.0_f32, -9.0_f32),
        (35.0, -1.0),
        (35.0, 7.0),
        (41.0, -9.0),
        (41.0, -1.0),
        (41.0, 7.0),
        (46.5, -5.0),
        (46.5, 3.0),
    ]
    .iter()
    .enumerate()
    {
        b.deco(DecoKind::DiningSet, Vec3::new(*x, deck, *z), i as f32 * 0.4, 1.0);
    }
    b.deco(DecoKind::BarSet, Vec3::new(44.5, deck, 11.0), 0.0, 1.0);
    b.deco(DecoKind::PottedPlant, Vec3::new(32.0, deck, -12.0), 0.0, 1.2);
    b.deco(DecoKind::PottedPlant, Vec3::new(32.0, deck, 12.0), 0.0, 1.2);

    // =======================================================================
    // POOL DECK & LAGOON POOL (south of the restaurant, east gardens)
    // =======================================================================
    b.slab("Pool Deck", Vec2::new(16.0, -34.0), Vec2::new(58.0, -16.0), -0.3, 0.05, MatId::Limestone);
    floors.push(crate::core::Block2::new(
        Vec3::new(37.0, -0.1, -25.0),
        Vec3::new(42.0, 0.3, 18.0),
    ));
    // Infinity pool: water surface + coping; the pool void is collidable water.
    b.slab("Infinity Pool", Vec2::new(24.0, -31.0), Vec2::new(50.0, -21.0), -1.6, -0.12, MatId::Water);
    water.push(crate::core::Block2::new(
        Vec3::new(37.0, -0.8, -26.0),
        Vec3::new(26.0, 1.6, 10.0),
    ));
    b.slab("Pool Coping N", Vec2::new(24.0, -21.0), Vec2::new(50.0, -20.4), -0.3, 0.22, MatId::Limestone);
    b.slab("Pool Coping S", Vec2::new(24.0, -31.6), Vec2::new(50.0, -31.0), -0.3, 0.22, MatId::Limestone);
    b.slab("Pool Coping W", Vec2::new(23.4, -31.0), Vec2::new(24.0, -21.0), -0.3, 0.22, MatId::Limestone);
    b.slab("Pool Coping E", Vec2::new(50.0, -31.0), Vec2::new(50.6, -21.0), -0.3, 0.22, MatId::Limestone);
    // Sun loungers and parasols around the pool.
    for i in 0..5 {
        let x = 25.5 + i as f32 * 5.2;
        b.deco(DecoKind::SunLounger, Vec3::new(x, 0.05, -33.2), 0.0, 1.0);
        b.deco(DecoKind::Parasol, Vec3::new(x + 2.4, 0.05, -33.6), 0.4, 1.0);
    }
    for i in 0..4 {
        let x = 26.0 + i as f32 * 6.0;
        b.deco(DecoKind::SunLounger, Vec3::new(x, 0.05, -18.6), std::f32::consts::PI, 1.0);
    }
    b.deco(DecoKind::BarSet, Vec3::new(53.0, 0.05, -26.0), 0.0, 1.0);
    b.deco(DecoKind::BarSet, Vec3::new(53.0, 0.05, -22.0), 0.0, 1.0);

    // =======================================================================
    // GARDENS: gravel paths, lawn, hedges, palms, flower beds, rockery
    // =======================================================================
    // Paths radiate from the lobby entrance to the pool and the beach.
    b.slab("Garden Path Centre", Vec2::new(-2.6, -34.0), Vec2::new(2.6, -14.0), 0.0, 0.08, MatId::Terracotta);
    b.slab("Garden Path East", Vec2::new(2.6, -24.0), Vec2::new(20.0, -21.4), 0.0, 0.08, MatId::Terracotta);
    b.slab("Garden Path West", Vec2::new(-20.0, -24.0), Vec2::new(-2.6, -21.4), 0.0, 0.08, MatId::Terracotta);
    b.slab("Garden Path Beach", Vec2::new(-2.6, -40.0), Vec2::new(2.6, -34.0), 0.0, 0.08, MatId::Terracotta);

    // Formal hedge parterres flanking the axis (they double as cover).
    for (mx, mz, sx, sz) in [
        (-16.0_f32, -18.0_f32, 12.0_f32, 1.2_f32),
        (16.0, -18.0, 12.0, 1.2),
        (-16.0, -30.0, 1.2, 10.0),
        (16.0, -30.0, 1.2, 10.0),
        (-10.0, -38.0, 8.0, 1.2),
        (10.0, -38.0, 8.0, 1.2),
    ] {
        b.hedge(Vec2::new(mx, mz), Vec2::new(sx, sz), 1.5);
    }
    // Rounded hedge caps: a slightly wider leafy crown breaks the box edge.
    for (mx, mz, sx, sz) in [
        (-16.0_f32, -18.0_f32, 12.0_f32, 1.2_f32),
        (16.0, -18.0, 12.0, 1.2),
        (-16.0, -30.0, 1.2, 10.0),
        (16.0, -30.0, 1.2, 10.0),
        (-10.0, -38.0, 8.0, 1.2),
        (10.0, -38.0, 8.0, 1.2),
    ] {
        b.detail(
            Vec3::new(mx, 1.52, mz),
            Vec3::new(sx * 0.96, 0.22, sz * 0.96),
            MatId::Hedge,
        );
    }

    // Flower beds inside the parterres.
    for (x, z) in [(-16.0_f32, -22.0_f32), (16.0, -22.0), (-10.0, -42.0), (10.0, -42.0)] {
        b.deco(DecoKind::FlowerBed, Vec3::new(x, 0.0, z), 0.0, 1.0);
    }

    // Palms: an avenue along the beach and clusters in the gardens.
    for i in 0..13 {
        let x = -60.0 + i as f32 * 10.0;
        b.deco(DecoKind::Palm, Vec3::new(x, 0.0, -44.0 + (i % 3) as f32 * 1.4), x * 0.37, 1.0 + (i % 4) as f32 * 0.12);
    }
    for (x, z, s) in [
        (-24.0_f32, -46.0_f32, 1.15_f32),
        (-38.0, -40.0, 1.0),
        (24.0, -46.0, 1.1),
        (38.0, -40.0, 1.05),
        (-48.0, -22.0, 1.0),
        (48.0, -16.0, 0.95),
        (-52.0, -56.0, 1.2),
        (52.0, -56.0, 1.2),
        (-14.0, -58.0, 1.1),
        (14.0, -58.0, 1.05),
    ] {
        b.deco(DecoKind::Palm, Vec3::new(x, 0.0, z), x * 0.21, s);
    }

    // Shade trees and shrub masses for depth and visual layering.
    for (x, z, s) in [
        (-30.0_f32, -26.0_f32, 1.1_f32),
        (30.0, -34.0, 1.0),
        (-44.0, -34.0, 1.2),
        (44.0, -44.0, 1.05),
        (-20.0, -50.0, 1.15),
        (20.0, -50.0, 1.0),
    ] {
        b.deco(DecoKind::Tree, Vec3::new(x, 0.0, z), x * 0.4, s);
    }
    for (x, z) in [
        (-26.0_f32, -34.0_f32),
        (26.0, -34.0),
        (-34.0, -44.0),
        (34.0, -44.0),
        (-8.0, -44.0),
        (8.0, -44.0),
        (-40.0, -20.0),
        (40.0, -20.0),
    ] {
        b.deco(DecoKind::ShrubCluster, Vec3::new(x, 0.0, z), 0.0, 1.0);
    }
    for i in 0..18 {
        let x = -52.0 + (i as f32 * 6.1) % 104.0;
        let z = -48.0 - (i as f32 * 1.7) % 6.0;
        b.deco(DecoKind::GrassTuft, Vec3::new(x, 0.0, z), i as f32, 1.0);
    }

    // Lamp posts along the paths.
    for i in 0..5 {
        let z = -18.0 - i as f32 * 5.0;
        b.deco(DecoKind::LampPost, Vec3::new(-3.6, 0.0, z), 0.0, 1.0);
        b.deco(DecoKind::LampPost, Vec3::new(3.6, 0.0, z), 0.0, 1.0);
    }
    b.deco(DecoKind::Bench, Vec3::new(-6.0, 0.0, -20.0), std::f32::consts::FRAC_PI_2, 1.0);
    b.deco(DecoKind::Bench, Vec3::new(6.0, 0.0, -20.0), -std::f32::consts::FRAC_PI_2, 1.0);
    b.deco(DecoKind::Bench, Vec3::new(-6.0, 0.0, -32.0), std::f32::consts::FRAC_PI_2, 1.0);
    b.deco(DecoKind::Bench, Vec3::new(6.0, 0.0, -32.0), -std::f32::consts::FRAC_PI_2, 1.0);

    // =======================================================================
    // BEACH: umbrellas, loungers, life rings, showers, boardwalk
    // =======================================================================
    b.slab("Boardwalk", Vec2::new(-2.2, -60.0), Vec2::new(2.2, -40.0), 0.0, 0.14, MatId::Wood);
    for i in 0..6 {
        let x = -34.0 + i as f32 * 13.0;
        b.deco(DecoKind::Umbrella, Vec3::new(x, 0.0, -47.0 - (i % 2) as f32 * 3.0), i as f32 * 0.7, 1.0);
        b.deco(DecoKind::SunLounger, Vec3::new(x + 1.4, 0.0, -48.4), 0.2, 1.0);
        b.deco(DecoKind::SunLounger, Vec3::new(x - 1.4, 0.0, -48.4), -0.2, 1.0);
    }
    b.deco(DecoKind::LifeRing, Vec3::new(-18.0, 0.0, -57.0), 0.0, 1.0);
    b.deco(DecoKind::LifeRing, Vec3::new(18.0, 0.0, -57.0), 0.0, 1.0);
    b.deco(DecoKind::BeachShower, Vec3::new(-9.0, 0.0, -42.0), 0.0, 1.0);
    b.deco(DecoKind::BeachShower, Vec3::new(9.0, 0.0, -42.0), 0.0, 1.0);

    // =======================================================================
    // SITE BOUNDARY: retaining wall + planting, so the map never looks cut off
    // =======================================================================
    let bound = 73.0;
    b.wall("Boundary W", Vec2::new(-bound, -62.0), Vec2::new(-bound, 64.0), 1.6, MatId::StoneWall);
    b.wall("Boundary E", Vec2::new(bound, -62.0), Vec2::new(bound, 64.0), 1.6, MatId::StoneWall);
    b.wall("Boundary N", Vec2::new(-bound, 63.0), Vec2::new(bound, 64.0), 1.6, MatId::StoneWall);
    for i in 0..16 {
        let t = i as f32 / 15.0;
        b.deco(DecoKind::ShrubCluster, Vec3::new(-70.0 + t * 140.0, 0.0, 60.0), 0.0, 1.2);
    }

    // =======================================================================
    // SERVICE LEVEL (y = -3.4): staff corridors, stores, plant room
    // =======================================================================
    let svc = SERVICE_Y;
    b.slab("Service Floor", Vec2::new(-48.0, -30.0), Vec2::new(-30.0, 22.0), svc - 0.3, svc, MatId::Concrete);
    floors.push(crate::core::Block2::new(
        Vec3::new(-39.0, svc - 0.15, -4.0),
        Vec3::new(18.0, 0.3, 52.0),
    ));
    b.slab("Service Wall W", Vec2::new(-48.6, -30.0), Vec2::new(-48.0, 22.0), svc, svc + 4.0, MatId::Concrete);
    b.slab("Service Wall E A", Vec2::new(-30.0, -30.0), Vec2::new(-29.4, -14.0), svc, svc + 4.0, MatId::Concrete);
    b.wall_with_opening(
        "Service Wall E B",
        Vec2::new(-30.0, 2.0),
        Vec2::new(-29.4, 22.0),
        0.6,
        4.0,
        12.0,
        2.4,
        0.0,
        2.6,
        MatId::Concrete,
    );
    // The service level needs a ceiling: otherwise the gardens float above it.
    b.slab("Service Ceiling", Vec2::new(-48.6, -31.0), Vec2::new(-29.4, 22.6), svc + 4.0, svc + 4.35, MatId::Concrete);
    b.slab("Service Wall S", Vec2::new(-48.0, -30.6), Vec2::new(-29.4, -30.0), svc, svc + 4.0, MatId::Concrete);

    // Stair from the gardens down (north side): 28 solid columns, 0.12 m risers.
    for i in 0..28 {
        let top = -(i as f32 + 1.0) * (3.4 / 28.0);
        let z0 = 22.4 - i as f32 * 0.55;
        b.slab(
            "Service Ramp",
            Vec2::new(-35.0, z0 - 0.57),
            Vec2::new(-31.0, z0),
            svc,
            top,
            MatId::Concrete,
        );
    }
    // Service props: shelving, crates, pipes, lockers, boiler.
    b.deco(DecoKind::Crates, Vec3::new(-44.0, svc, -6.0), 0.0, 1.0);
    b.deco(DecoKind::Crates, Vec3::new(-40.0, svc, -8.0), 0.5, 0.8);
    b.deco(DecoKind::Crates, Vec3::new(-46.0, svc, 2.0), 0.2, 1.2);
    b.slab("Service Lockers", Vec2::new(-47.6, 10.0), Vec2::new(-45.2, 16.0), svc, svc + 2.2, MatId::Metal);
    b.slab("Service Shelf A", Vec2::new(-46.5, -20.0), Vec2::new(-40.0, -19.2), svc, svc + 2.0, MatId::Metal);
    b.slab("Service Shelf B", Vec2::new(-46.5, -14.0), Vec2::new(-40.0, -13.2), svc, svc + 2.0, MatId::Metal);
    b.slab("Boiler", Vec2::new(-36.0, -26.0), Vec2::new(-33.0, -23.0), svc, svc + 2.6, MatId::Metal);
    b.deco(DecoKind::AirUnit, Vec3::new(-33.0, svc + 2.4, -6.0), 0.0, 1.0);
    b.deco(DecoKind::AirUnit, Vec3::new(-33.0, svc + 2.4, 6.0), 0.0, 1.0);
    // Ceiling strip lights.
    for z in [-24.0_f32, -16.0, -8.0, 0.0, 8.0, 16.0] {
        b.detail(Vec3::new(-39.0, svc + 3.92, z), Vec3::new(1.2, 0.08, 3.0), MatId::WhitePaint);
    }

    // =======================================================================
    // SECURITY WING (restricted): CCTV room, interview room, armoury
    // =======================================================================
    b.slab("Security Floor", Vec2::new(-62.0, -30.0), Vec2::new(-48.0, -8.0), svc - 0.3, svc, MatId::Concrete);
    floors.push(crate::core::Block2::new(
        Vec3::new(-55.0, svc - 0.15, -19.0),
        Vec3::new(14.0, 0.3, 22.0),
    ));
    b.slab("Security Wall N", Vec2::new(-62.0, -8.6), Vec2::new(-48.0, -8.0), svc, svc + 4.0, MatId::Brick);
    b.slab("Security Wall S", Vec2::new(-62.0, -30.6), Vec2::new(-48.0, -30.0), svc, svc + 4.0, MatId::Brick);
    b.slab("Security Wall W", Vec2::new(-62.6, -30.0), Vec2::new(-62.0, -8.0), svc, svc + 4.0, MatId::Brick);
    b.wall_with_opening(
        "Security Wall E A",
        Vec2::new(-48.0, -30.0),
        Vec2::new(-47.4, -8.0),
        0.6,
        4.0,
        -22.0,
        2.2,
        0.0,
        2.6,
        MatId::Brick,
    );
    b.slab("Security Ceiling", Vec2::new(-62.6, -30.6), Vec2::new(-47.4, -8.0), svc + 4.0, svc + 4.35, MatId::Concrete);
    // CCTV control room: monitor wall + desks.
    b.slab("CCTV Desk", Vec2::new(-60.0, -26.0), Vec2::new(-53.0, -24.0), svc, svc + 1.1, MatId::Metal);
    b.slab("CCTV Monitor Wall", Vec2::new(-61.8, -26.5), Vec2::new(-61.4, -20.0), svc + 1.2, svc + 3.0, MatId::DarkTrim);
    for i in 0..3 {
        for j in 0..2 {
            b.detail(
                Vec3::new(-61.2, svc + 1.6 + j as f32 * 0.8, -25.4 + i as f32 * 1.6),
                Vec3::new(0.08, 0.62, 1.1),
                MatId::Glass,
            );
        }
    }
    // Interview room.
    b.slab("Interview Table", Vec2::new(-57.0, -16.0), Vec2::new(-53.0, -14.4), svc, svc + 0.78, MatId::Wood);
    b.deco(DecoKind::Armchair, Vec3::new(-55.0, svc, -18.0), 0.0, 1.0);
    b.deco(DecoKind::Armchair, Vec3::new(-55.0, svc, -12.5), std::f32::consts::PI, 1.0);
    // Armoury lockers.
    b.slab("Armoury Lockers", Vec2::new(-61.6, -12.0), Vec2::new(-59.4, -9.0), svc, svc + 2.0, MatId::Metal);
    b.detail(Vec3::new(-55.0, svc + 3.94, -19.0), Vec3::new(10.0, 0.08, 0.5), MatId::WhitePaint);

    // =======================================================================
    // PENTHOUSE (restricted): the target's terrace, suite and hot tub
    // =======================================================================
    let pent = PENTHOUSE_Y;
    b.slab("Penthouse Terrace", Vec2::new(-18.0, 27.0), Vec2::new(18.0, 50.0), pent - 0.35, pent, MatId::Marble);
    floors.push(crate::core::Block2::new(
        Vec3::new(0.0, pent - 0.17, 38.5),
        Vec3::new(36.0, 0.35, 23.0),
    ));
    // Parapet with balustrades.
    b.slab("Penthouse Rail W", Vec2::new(-18.0, 27.0), Vec2::new(-17.5, 50.0), pent, pent + 1.1, MatId::StoneWall);
    b.slab("Penthouse Rail E", Vec2::new(17.5, 27.0), Vec2::new(18.0, 50.0), pent, pent + 1.1, MatId::StoneWall);
    b.slab("Penthouse Rail N", Vec2::new(-18.0, 49.5), Vec2::new(18.0, 50.0), pent, pent + 1.1, MatId::StoneWall);
    // Suite: full room with walls, doorway, windows and roof.
    b.wall_with_opening("Suite South", Vec2::new(-9.0, 33.0), Vec2::new(9.0, 33.8), 0.8, 4.0, 0.0, 2.2, 0.0, 3.0, MatId::Plaster);
    b.slab("Suite West", Vec2::new(-9.0, 33.0), Vec2::new(-8.2, 46.0), pent, pent + 4.0, MatId::Plaster);
    b.slab("Suite East", Vec2::new(8.2, 33.0), Vec2::new(9.0, 46.0), pent, pent + 4.0, MatId::Plaster);
    b.slab("Suite North", Vec2::new(-9.0, 45.2), Vec2::new(9.0, 46.0), pent, pent + 4.0, MatId::Plaster);
    b.slab("Suite Roof", Vec2::new(-9.4, 32.6), Vec2::new(9.4, 46.4), pent + 4.0, pent + 4.5, MatId::Roof);
    // Suite interior: bed, desk, safe, wardrobe.
    b.deco(DecoKind::Bed, Vec3::new(0.0, pent, 42.0), 0.0, 1.2);
    b.deco(DecoKind::Nightstand, Vec3::new(2.2, pent, 40.6), 0.0, 1.0);
    b.deco(DecoKind::ConsoleTable, Vec3::new(-1.5, pent, 34.6), 0.0, 1.0);
    b.slab("Penthouse Safe", Vec2::new(5.6, 43.6), Vec2::new(7.0, 45.0), pent, pent + 1.9, MatId::Metal);
    // Terrace: hot tub with steps, sun beds, planting.
    b.slab("Hot Tub Rim", Vec2::new(9.0, 30.0), Vec2::new(15.0, 36.0), pent, pent + 0.75, MatId::StoneWall);
    b.slab("Hot Tub Water", Vec2::new(9.4, 30.4), Vec2::new(14.6, 35.6), pent + 0.45, pent + 0.62, MatId::Water);
    b.deco(DecoKind::SunLounger, Vec3::new(-13.0, pent, 29.5), 0.0, 1.0);
    b.deco(DecoKind::SunLounger, Vec3::new(-10.5, pent, 29.5), 0.0, 1.0);
    b.deco(DecoKind::PottedPlant, Vec3::new(-16.0, pent, 28.5), 0.0, 1.2);
    b.deco(DecoKind::PottedPlant, Vec3::new(16.0, pent, 28.5), 0.0, 1.2);
    b.deco(DecoKind::PottedPlant, Vec3::new(-16.5, pent, 48.0), 0.0, 1.0);

    // External staircase: 72 solid columns (0.41 m risers) up the west face.
    let stair_steps = 72;
    let riser = pent / stair_steps as f32;
    for i in 0..stair_steps {
        let top = (i as f32 + 1.0) * riser;
        let z0 = 28.0 + i as f32 * 0.28;
        b.slab(
            "Penthouse Stair",
            Vec2::new(-32.0, z0 - 0.01),
            Vec2::new(-29.0, z0 + 0.31),
            -0.2,
            top,
            MatId::Limestone,
        );
    }
    b.slab("Stair Landing", Vec2::new(-32.0, 48.2), Vec2::new(-18.0, 49.6), pent - 0.35, pent, MatId::Limestone);
    b.slab("Stair Gate Post L", Vec2::new(-32.6, 27.2), Vec2::new(-31.6, 28.0), 0.0, 2.4, MatId::Metal);
    b.slab("Stair Gate Post R", Vec2::new(-29.4, 27.2), Vec2::new(-28.4, 28.0), 0.0, 2.4, MatId::Metal);

    // =======================================================================
    // Zones, doors, spawn
    // =======================================================================
    let zones = vec![
        crate::core::ZoneDefinition { id: Beach, rect: crate::core::ZoneRect::new(-74.0, -64.0, 74.0, -38.0), priority: 2 },
        crate::core::ZoneDefinition { id: Gardens, rect: crate::core::ZoneRect::new(-74.0, -38.0, 74.0, -14.0), priority: 3 },
        crate::core::ZoneDefinition { id: Lobby, rect: crate::core::ZoneRect::new(-29.0, -14.0, 29.0, 26.0), priority: 6 },
        crate::core::ZoneDefinition { id: Restaurant, rect: crate::core::ZoneRect::new(31.0, -14.0, 58.0, 14.0), priority: 5 },
        crate::core::ZoneDefinition { id: PoolDeck, rect: crate::core::ZoneRect::new(16.0, -34.0, 58.0, -16.0), priority: 4 },
        crate::core::ZoneDefinition { id: ServiceCorridor, rect: crate::core::ZoneRect::new(-48.0, -30.0, -30.0, 22.0), priority: 7 },
        crate::core::ZoneDefinition { id: SecurityWing, rect: crate::core::ZoneRect::new(-62.0, -30.0, -48.0, -8.0), priority: 9 },
        crate::core::ZoneDefinition { id: Penthouse, rect: crate::core::ZoneRect::new(-32.0, 26.0, 18.0, 50.0), priority: 8 },
    ];

    let mut doors = Vec::new();
    doors.push(DoorSpec { name: "Main Entrance", position: Vec2::new(0.0, 26.0), width: 2.5, zone: Lobby });
    doors.push(DoorSpec { name: "Garden Entrance", position: Vec2::new(0.0, -14.0), width: 2.2, zone: Lobby });
    doors.push(DoorSpec { name: "Restaurant Door", position: Vec2::new(31.3, -1.0), width: 1.5, zone: Restaurant });
    doors.push(DoorSpec { name: "Service Door", position: Vec2::new(-29.7, 12.0), width: 1.2, zone: ServiceCorridor });
    doors.push(DoorSpec { name: "Security Door", position: Vec2::new(-47.7, -22.0), width: 1.1, zone: SecurityWing });
    doors.push(DoorSpec { name: "Suite Door", position: Vec2::new(0.0, 33.4), width: 1.1, zone: Penthouse });

    ResortLayout {
        name: "Coral Palm Resort",
        solids: b.solids,
        deco: b.deco,
        details: b.details,
        zones,
        doors,
        water,
        floors,
        // On the marble forecourt, facing the lobby.
        player_spawn: Vec3::new(0.0, 0.05, -17.5),
    }
}
