//! Temporary geometry diagnostics (removed once the level is verified).

use bevy::prelude::*;
use hitman_lowpoly::core::Circle2;
use hitman_lowpoly::player::movement::ground_height;
use hitman_lowpoly::world::builder::WorldCollision;
use hitman_lowpoly::world::layout::resort_layout;

fn dump(label: &str, pos: Vec2) {
    let layout = resort_layout();
    let collision = WorldCollision::from_layout(&layout);
    println!("--- {label} at {pos:?}");
    for block in &collision.blocks {
        if block.contains_xz(pos, 0.5) {
            println!(
                "  BLOCK [{:.1}..{:.1}]x[{:.1}..{:.1}] y {:.2}..{:.2}",
                block.min.x, block.max.x, block.min.y, block.max.y, block.bottom(), block.top()
            );
        }
    }
    let mut nearest: Vec<(f32, String)> = Vec::new();
    for spec in &layout.solids {
        let b = spec.block();
        if b.contains_xz(pos, 3.0) {
            nearest.push((b.top(), format!("{} top={:.2}", spec.label, b.top())));
        }
    }
    println!("  nearby solids: {nearest:?}");
    println!(
        "  ground_height(0.4) = {:?}",
        ground_height(&collision.surfaces, pos, 0.4, 0.0)
    );
    let _ = Circle2 { center: pos, radius: 0.4 };
}

#[test]
fn diag() {
    dump("garden west", Vec2::new(0.0, -20.0));
    dump("plaza x=12", Vec2::new(12.0, -16.0));
    dump("restaurant stair foot", Vec2::new(34.0, -19.0));
    dump("restaurant stair mid", Vec2::new(34.0, -16.0));
    dump("penthouse stair mid", Vec2::new(-30.5, 40.0));
    dump("penthouse stair start", Vec2::new(-30.5, 28.5));
    panic!("diagnostics printed");
}
