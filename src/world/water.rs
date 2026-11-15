//! Water animation.
//!
//! The shimmering surface itself is a shader (see `assets/shaders/water.wgsl`);
//! this module only advances the clock the shader reads and keeps the pool and
//! lagoon surfaces visually distinct.

use crate::world::materials::WaterMaterial;
use bevy::pbr::ExtendedMaterial;
use bevy::prelude::*;

/// Seconds of accumulated wave time, read by the water shader.
#[derive(Resource, Default)]
pub struct WaterClock(pub f32);

/// Advance the wave clock. Cheap enough to run every frame.
pub fn animate_water(
    time: Res<Time>,
    mut clock: ResMut<WaterClock>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, WaterMaterial>>>,
) {
    clock.0 += time.delta_secs();
    // Only touch the material when it actually needs to change; the shader does
    // most of the motion, this is the slow drift between wave sets.
    if (clock.0 * 10.0) as u32 % 6 == 0 {
        for (_, material) in materials.iter_mut() {
            material.extension.wave_speed = 0.045 + (clock.0 * 0.05).sin() * 0.012;
        }
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<WaterClock>()
        .add_systems(Update, animate_water);
}
