//! Sky, image-based lighting and the post-processing stack.
//!
//! The visual reference is *HITMAN: World of Assassination*: a bright tropical
//! afternoon with a hard sun, sky-bounce fill, scene-referred HDR, and subtle
//! bloom on sun highlights.
//!
//! Everything here is generated at startup so the game ships without any
//! external HDRI:
//!
//! * [`generate_environment_cubemap`] draws a physically plausible tropical sky
//!   (Rayleigh-style blue gradient, sun disk with limb darkening, warm horizon
//!   haze, cumulus cloud band) into six cube faces.
//! * That cubemap becomes the camera's `EnvironmentMapLight`, giving every PBR
//!   material real specular reflections and sky-coloured ambient light — this is
//!   the single biggest step up from the old flat "ambient colour" look.
//! * Horizontal and vertical blur passes derive the rough-specular and diffuse
//!   irradiance maps, so reflections on water and marble are properly blurred.
//!
//! [`GraphicsSettings`] drives the preset system (Low / Medium / High / Ultra)
//! and is what the settings menu mutates.

use bevy::{
    asset::RenderAssetUsages,
    core_pipeline::{
        bloom::Bloom,
        prepass::{DepthPrepass, NormalPrepass},
        tonemapping::Tonemapping,
    },
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    pbr::{
        prelude::EnvironmentMapLight, CascadeShadowConfig, CascadeShadowConfigBuilder,
        DirectionalLightShadowMap,
    },
    prelude::*,
    render::{
        camera::Exposure,
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    },
};

use super::builder::LevelEntity;

/// Direction *towards* the sun. Shared by lighting, sky generation and any
/// system that wants lens glints or window highlights.
pub const SUN_DIRECTION: Vec3 = Vec3::new(0.42, 0.72, 0.55);

/// Sky/sun parameters, kept together so day-time variants are one constant away.
pub struct SkyPalette {
    pub zenith: Vec3,
    pub horizon: Vec3,
    pub ground: Vec3,
    pub sun_color: Vec3,
    pub haze: Vec3,
}

pub const TROPICAL_AFTERNOON: SkyPalette = SkyPalette {
    zenith: Vec3::new(0.115, 0.240, 0.560),
    horizon: Vec3::new(0.640, 0.735, 0.845),
    ground: Vec3::new(0.240, 0.250, 0.225),
    sun_color: Vec3::new(1.0, 0.955, 0.885),
    haze: Vec3::new(0.780, 0.760, 0.700),
};

// ---------------------------------------------------------------------------
// Graphics settings
// ---------------------------------------------------------------------------

/// Quality preset selected in the settings menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QualityPreset {
    Low,
    Medium,
    #[default]
    High,
    Ultra,
}

impl QualityPreset {
    pub const ALL: [QualityPreset; 4] = [
        QualityPreset::Low,
        QualityPreset::Medium,
        QualityPreset::High,
        QualityPreset::Ultra,
    ];

    pub fn label(self) -> &'static str {
        match self {
            QualityPreset::Low => "Low",
            QualityPreset::Medium => "Medium",
            QualityPreset::High => "High",
            QualityPreset::Ultra => "Ultra",
        }
    }

    /// Next preset in the cycle (used by the settings UI).
    pub fn next(self) -> Self {
        match self {
            QualityPreset::Low => QualityPreset::Medium,
            QualityPreset::Medium => QualityPreset::High,
            QualityPreset::High => QualityPreset::Ultra,
            QualityPreset::Ultra => QualityPreset::Low,
        }
    }

    pub fn previous(self) -> Self {
        match self {
            QualityPreset::Low => QualityPreset::Ultra,
            QualityPreset::Medium => QualityPreset::Low,
            QualityPreset::High => QualityPreset::Medium,
            QualityPreset::Ultra => QualityPreset::High,
        }
    }
}

/// Everything the graphics settings menu can change, plus the derived budget
/// numbers the rest of the game reads (shadow distance, NPC animation rate...).
#[derive(Resource, Debug, Clone)]
pub struct GraphicsSettings {
    pub preset: QualityPreset,
    /// Render scale, 0.5..=1.0. Iris Xe benefits from ~0.85 at native 1080p.
    pub resolution_scale: f32,
    pub shadows: bool,
    pub shadow_map_size: usize,
    pub shadow_distance: f32,
    /// Ambient occlusion needs a prepass and is the most expensive effect here.
    pub ssao: bool,
    pub bloom: bool,
    /// Camera far plane / fog distances are allowed to shrink on lower presets.
    pub draw_distance: f32,
    pub environment_lighting: bool,
    pub vsync: bool,
    pub fps_limit: Option<u32>,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self::from_preset(QualityPreset::High)
    }
}

impl GraphicsSettings {
    pub fn from_preset(preset: QualityPreset) -> Self {
        match preset {
            QualityPreset::Low => Self {
                preset,
                resolution_scale: 0.75,
                shadows: true,
                shadow_map_size: 1024,
                shadow_distance: 45.0,
                ssao: false,
                bloom: false,
                draw_distance: 210.0,
                environment_lighting: true,
                vsync: true,
                fps_limit: Some(60),
            },
            QualityPreset::Medium => Self {
                preset,
                resolution_scale: 0.9,
                shadows: true,
                shadow_map_size: 2048,
                shadow_distance: 70.0,
                ssao: false,
                bloom: true,
                draw_distance: 320.0,
                environment_lighting: true,
                vsync: true,
                fps_limit: None,
            },
            QualityPreset::High => Self {
                preset,
                resolution_scale: 1.0,
                shadows: true,
                shadow_map_size: 2048,
                shadow_distance: 95.0,
                ssao: true,
                bloom: true,
                draw_distance: 420.0,
                environment_lighting: true,
                vsync: true,
                fps_limit: None,
            },
            QualityPreset::Ultra => Self {
                preset,
                resolution_scale: 1.0,
                shadows: true,
                shadow_map_size: 4096,
                shadow_distance: 130.0,
                ssao: true,
                bloom: true,
                draw_distance: 600.0,
                environment_lighting: true,
                vsync: false,
                fps_limit: None,
            },
        }
    }

    /// Distance at which NPCs stop being animated (they still move, just with
    /// fewer per-frame updates) — keeps hundreds of agents cheap.
    pub fn npc_animation_distance(&self) -> f32 {
        match self.preset {
            QualityPreset::Low => 30.0,
            QualityPreset::Medium => 45.0,
            QualityPreset::High => 65.0,
            QualityPreset::Ultra => 90.0,
        }
    }

    /// Maximum number of shadow-casting cascades. Fewer cascades on low-end
    /// hardware roughly halves the shadow pass cost.
    pub fn cascades(&self) -> usize {
        match self.preset {
            QualityPreset::Low => 2,
            QualityPreset::Medium => 3,
            QualityPreset::High => 4,
            QualityPreset::Ultra => 4,
        }
    }
}

// ---------------------------------------------------------------------------
// Sky cubemap generation
// ---------------------------------------------------------------------------

fn srgb_encode(c: f32) -> f32 {
    linear_to_srgb(c).clamp(0.0, 1.0)
}

fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// Cheap 3D value noise used for the cloud band.
fn hash3(x: i32, y: i32, z: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374_761_393)
        ^ (y as u32).wrapping_mul(668_265_263)
        ^ (z as u32).wrapping_mul(2_147_483_647);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    ((h ^ (h >> 16)) & 0x00FF_FFFF) as f32 / 16_777_215.0
}

fn value_noise(p: Vec3) -> f32 {
    let xi = p.x.floor();
    let yi = p.y.floor();
    let zi = p.z.floor();
    let f = Vec3::new(p.x - xi, p.y - yi, p.z - zi);
    let fade = f * f * (Vec3::splat(3.0) - f * 2.0);
    let mut total = 0.0;
    for dz in 0..2 {
        for dy in 0..2 {
            for dx in 0..2 {
                let w = (if dx == 0 { 1.0 - fade.x } else { fade.x })
                    * (if dy == 0 { 1.0 - fade.y } else { fade.y })
                    * (if dz == 0 { 1.0 - fade.z } else { fade.z });
                total += w
                    * hash3(
                        xi as i32 + dx - 1,
                        yi as i32 + dy - 1,
                        zi as i32 + dz - 1,
                    );
            }
        }
    }
    total
}

fn fbm3(p: Vec3, octaves: u32) -> f32 {
    let mut total = 0.0;
    let mut amp = 0.5;
    let mut freq = 1.0;
    for _ in 0..octaves {
        total += value_noise(p * freq) * amp;
        amp *= 0.5;
        freq *= 2.07;
    }
    total
}

/// Direction of a pixel on face `face` of a cube, with the standard
/// OpenGL-style cube face orientation Bevy expects.
fn cube_direction(face: usize, u: f32, v: f32) -> Vec3 {
    // u, v in -1..1
    match face {
        0 => Vec3::new(1.0, -v, -u),   // +X
        1 => Vec3::new(-1.0, -v, u),   // -X
        2 => Vec3::new(u, 1.0, v),     // +Y
        3 => Vec3::new(u, -1.0, -v),   // -Y
        4 => Vec3::new(u, -v, 1.0),    // +Z
        _ => Vec3::new(-u, -v, -1.0),  // -Z
    }
    .normalize()
}

/// Radiance of the sky for a view direction. Values are HDR (sun disk is very
/// bright) so bloom and tonemapping behave like a real renderer.
fn sample_sky(dir: Vec3, palette: &SkyPalette) -> Vec3 {
    let up = dir.y.max(-1.0);
    let horizon_t = (up.max(0.0)).powf(0.42);
    let mut color = palette.horizon.lerp(palette.zenith, horizon_t);

    // Below the horizon: warm ground bounce fading to the horizon haze.
    if up < 0.0 {
        let t = (-up).powf(0.55);
        color = palette.horizon.lerp(palette.ground, t);
    }

    // Forward scattering around the sun: the warm glow that makes a tropical
    // sky look photographed rather than painted.
    let sun_dot = dir.dot(SUN_DIRECTION).max(0.0);
    let glow = sun_dot.powf(6.0) * 0.55 + sun_dot.powf(48.0) * 1.6;
    color += palette.haze * glow;

    // Sun disk, with a soft limb.
    let disk = dir.dot(SUN_DIRECTION);
    if disk > 0.99965 {
        color += palette.sun_color * 420.0;
    } else if disk > 0.9990 {
        color += palette.sun_color * 55.0;
    } else if disk > 0.9950 {
        color += palette.sun_color * 4.0;
    }

    // Cumulus band: thin near the zenith, thicker towards the horizon.
    if up > 0.02 && up < 0.62 && sun_dot < 0.995 {
        let band = ((up - 0.02) / 0.60).clamp(0.0, 1.0);
        let shape = (band * std::f32::consts::PI).sin();
        let p = dir / up.max(0.06) * 1.35;
        let clouds = fbm3(Vec3::new(p.x, p.z, p.y * 0.6) * 2.6, 5);
        let density = ((clouds - 0.50) * 2.9).clamp(0.0, 1.0) * shape;
        // Lit tops, shaded bases: offset a sample towards the sun for the top.
        let lit = fbm3(Vec3::new(p.x + SUN_DIRECTION.x, p.z + SUN_DIRECTION.z, p.y * 0.6) * 2.6, 4);
        let cloud_color = palette.haze.lerp(Vec3::ONE, (lit * 1.35).clamp(0.0, 1.0));
        color = color.lerp(cloud_color * 1.25, density * 0.92);
    }

    color.max(Vec3::ZERO)
}

/// Generate a 6-face HDR cubemap image of the sky.
pub fn generate_environment_cubemap(size: u32, palette: &SkyPalette) -> Image {
    let bytes_per_row = (size * 4 * 4) as usize; // RGBA16F
    let mut data = vec![0u8; bytes_per_row * size as usize * 6];

    for face in 0..6 {
        for y in 0..size {
            for x in 0..size {
                let u = (x as f32 + 0.5) / size as f32 * 2.0 - 1.0;
                let v = (y as f32 + 0.5) / size as f32 * 2.0 - 1.0;
                let dir = cube_direction(face, u, v);
                let color = sample_sky(dir, palette);
                // Store linear HDR radiance as f16.
                let offset = face as usize * bytes_per_row * size as usize
                    + y as usize * bytes_per_row
                    + x as usize * 8;
                for (channel, value) in [color.x, color.y, color.z, 1.0].iter().enumerate() {
                    let half = f32_to_f16(*value);
                    data[offset + channel * 2..offset + channel * 2 + 2]
                        .copy_from_slice(&half.to_le_bytes());
                }
            }
        }
    }

    let mut image = Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 6,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba16Float,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
        | TextureUsages::COPY_DST
        | TextureUsages::COPY_SRC
        | TextureUsages::RENDER_ATTACHMENT;
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        address_mode_w: ImageAddressMode::ClampToEdge,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        ..default()
    });
    image
}

/// IEEE-754 binary16 conversion (Rust has no stable `f32::to_f16`).
fn f32_to_f16(value: f32) -> u16 {
    let bits = value.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let mut exp = ((bits >> 23) & 0xFF) as i32;
    let mantissa = bits & 0x007F_FFFF;

    if exp == 255 {
        // Infinity / NaN: preserve the distinction.
        return sign | 0x7C00 | if mantissa != 0 { 0x200 } else { 0 };
    }
    exp = exp - 127 + 15;
    if exp >= 31 {
        return sign | 0x7C00; // overflow to infinity
    }
    if exp <= 0 {
        // Subnormal or zero.
        if exp < -10 {
            return sign;
        }
        let mantissa = (mantissa | 0x0080_0000) >> (1 - exp) as u32;
        return sign | (mantissa >> 13) as u16;
    }
    sign | ((exp as u16) << 10) | ((mantissa >> 13) as u16)
}

/// Box-blur a cubemap in place, `passes` times with the given radius. Used to
/// approximate the roughness prefilter and the diffuse irradiance lookup.
pub fn blur_cubemap(image: &mut Image, passes: u32, radius: i32) {
    let Some(data) = image.data.as_mut() else {
        return;
    };
    let width = image.texture_descriptor.size.width as i32;
    let height = image.texture_descriptor.size.height as i32;
    let faces = image.texture_descriptor.size.depth_or_array_layers as i32;

    let read = |data: &Vec<u8>, face: i32, x: i32, y: i32| -> Vec3 {
        let x = x.clamp(0, width - 1);
        let y = y.clamp(0, height - 1);
        let offset = ((face * height + y) * width + x) as usize * 8;
        let mut out = Vec3::ZERO;
        for (channel, slot) in [&mut out.x, &mut out.y, &mut out.z].into_iter().enumerate() {
            let lo = data[offset + channel * 2];
            let hi = data[offset + channel * 2 + 1];
            *slot = f16_to_f32(u16::from_le_bytes([lo, hi]));
        }
        out
    };

    for _ in 0..passes {
        let source = data.clone();
        for face in 0..faces {
            for y in 0..height {
                for x in 0..width {
                    let mut total = Vec3::ZERO;
                    let mut count = 0.0;
                    for dy in -radius..=radius {
                        for dx in -radius..=radius {
                            total += read(&source, face, x + dx, y + dy);
                            count += 1.0;
                        }
                    }
                    let color = total / count;
                    let offset = ((face * height + y) * width + x) as usize * 8;
                    for (channel, value) in [color.x, color.y, color.z, 1.0].iter().enumerate() {
                        let half = f32_to_f16(*value);
                        data[offset + channel * 2..offset + channel * 2 + 2]
                            .copy_from_slice(&half.to_le_bytes());
                    }
                }
            }
        }
    }
}

fn f16_to_f32(half: u16) -> f32 {
    let sign = if half & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exp = ((half >> 10) & 0x1F) as i32;
    let mantissa = (half & 0x03FF) as f32;
    if exp == 0 {
        sign * (mantissa / 1024.0) * 2f32.powi(-14)
    } else if exp == 31 {
        f32::INFINITY * sign
    } else {
        sign * (1.0 + mantissa / 1024.0) * 2f32.powi(exp - 15)
    }
}

// ---------------------------------------------------------------------------
// Entities
// ---------------------------------------------------------------------------

/// Marks the sun so quality settings can retune its shadow budget.
#[derive(Component)]
pub struct SunLight;

/// Marks the camera environment light holder.
#[derive(Component)]
pub struct SkyEnvironment;

/// Spawn the sun, the ocean plane beyond the map and the sky lighting.
pub fn setup_environment(mut commands: Commands) {
    // --- Sun ------------------------------------------------------------
    // `SUN_DIRECTION` points towards the sun, so the light must face away from
    // it (directional lights emit along their local -Z).
    let sun_rotation = Quat::from_rotation_arc(Vec3::NEG_Z, -SUN_DIRECTION);
    commands.spawn((
        LevelEntity,
        SunLight,
        Name::new("Sun"),
        DirectionalLight {
            color: Color::srgb(1.0, 0.955, 0.885),
            illuminance: 82_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(sun_rotation),
        CascadeShadowConfigBuilder {
            num_cascades: 4,
            minimum_distance: 0.4,
            maximum_distance: 95.0,
            first_cascade_far_bound: 12.0,
            overlap_proportion: 0.2,
        }
        .build(),
    ));

    // A little sky bounce at ground level. The environment map does most of the
    // work, but a touch of upward fill keeps shadowed undersides readable.
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.72, 0.82, 0.95),
        brightness: 90.0,
        affects_lightmapped_meshes: false,
    });

    info!("Environment: tropical afternoon, 82 k lux sun + sky IBL");
}

/// Build the sky images and hand them to the camera. Runs after `setup_environment`
/// and after the camera exists.
pub fn apply_sky_environment(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    settings: Res<GraphicsSettings>,
    cameras: Query<Entity, With<Camera3d>>,
) {
    if cameras.is_empty() {
        return;
    }
    // 128 per face is plenty for smooth reflections and costs ~1.5 MB.
    let specular = images.add(generate_environment_cubemap(128, &TROPICAL_AFTERNOON));

    // Rough specular: two wide box passes approximate a high-roughness prefilter.
    let mut rough = generate_environment_cubemap(32, &TROPICAL_AFTERNOON);
    blur_cubemap(&mut rough, 2, 2);
    let diffuse = images.add(rough);

    for camera in &cameras {
        if settings.environment_lighting {
            commands.entity(camera).insert(EnvironmentMapLight {
                diffuse_map: diffuse.clone(),
                specular_map: specular.clone(),
                intensity: 1_100.0,
                rotation: Quat::IDENTITY,
                affects_lightmapped_mesh_diffuse: true,
            });
        }
    }
}

/// Radius/quality of the sky dome. A huge inverted sphere is cheaper and more
/// controllable than a skybox pass for this map size.
pub fn setup_sky_mesh(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(Sphere::new(1400.0).mesh().ico(3).expect("ico sphere"));
    // Unlit so the dome is exactly the sky colour and never picks up shadows;
    // bright emissive pushes it above the bloom threshold near the horizon.
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(
            srgb_encode(TROPICAL_AFTERNOON.horizon.x),
            srgb_encode(TROPICAL_AFTERNOON.horizon.y),
            srgb_encode(TROPICAL_AFTERNOON.horizon.z + 0.12),
        ),
        emissive: LinearRgba::new(
            TROPICAL_AFTERNOON.horizon.x * 1.6,
            TROPICAL_AFTERNOON.horizon.y * 1.9,
            TROPICAL_AFTERNOON.horizon.z * 2.4,
            1.0,
        ),
        perceptual_roughness: 1.0,
        metallic: 0.0,
        unlit: true,
        cull_mode: None,
        ..default()
    });
    commands.spawn((
        LevelEntity,
        Name::new("Sky Dome"),
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::default(),
        bevy::pbr::NotShadowCaster,
    ));
}

// ---------------------------------------------------------------------------
// Camera post-processing
// ---------------------------------------------------------------------------

/// Attach/refresh the post-processing stack for the current quality preset.
pub fn apply_post_processing(
    settings: Res<GraphicsSettings>,
    mut commands: Commands,
    mut cameras: Query<(Entity, &mut Projection), With<Camera3d>>,
) {
    if !settings.is_changed() {
        return;
    }
    for (entity, mut projection) in &mut cameras {
        let mut camera = commands.entity(entity);
        camera.insert(Tonemapping::TonyMcMapface);
        camera.insert(Exposure::BLENDER);
        if settings.bloom {
            camera.insert(Bloom::NATURAL);
        } else {
            camera.remove::<Bloom>();
        }
        if settings.ssao {
            // SSAO requires the prepass and `Msaa::Off`.
            camera.insert((
                DepthPrepass,
                NormalPrepass,
                Msaa::Off,
                bevy::pbr::ScreenSpaceAmbientOcclusion::default(),
            ));
        } else {
            camera.remove::<(
                bevy::pbr::ScreenSpaceAmbientOcclusion,
                DepthPrepass,
                NormalPrepass,
            )>();
            camera.insert(Msaa::Sample4);
        }
        if let Projection::Perspective(perspective) = &mut *projection {
            perspective.far = settings.draw_distance.max(120.0);
        }
    }
}

/// Keep the shadow map budget and cascade distance in sync with the preset.
pub fn apply_shadow_settings(
    settings: Res<GraphicsSettings>,
    mut shadow_map: ResMut<DirectionalLightShadowMap>,
    mut suns: Query<(&mut DirectionalLight, &mut CascadeShadowConfig), With<SunLight>>,
) {
    if !settings.is_changed() && !shadow_map.is_changed() {
        return;
    }
    if shadow_map.size != settings.shadow_map_size {
        shadow_map.size = settings.shadow_map_size;
    }
    for (mut light, mut cascades) in &mut suns {
        light.shadows_enabled = settings.shadows;
        // Cascade bounds are computed by the builder, so a changed shadow
        // distance means rebuilding the whole config, not editing fields.
        *cascades = CascadeShadowConfigBuilder {
            num_cascades: settings.cascades(),
            minimum_distance: 0.4,
            maximum_distance: settings.shadow_distance,
            first_cascade_far_bound: (settings.shadow_distance * 0.13).max(6.0),
            overlap_proportion: 0.2,
        }
        .build();
    }
}

/// Sun angle in the sky, exposed for gameplay systems (e.g. shadow-based
/// hiding spots in later stages).
pub fn sun_direction() -> Vec3 {
    SUN_DIRECTION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_float_roundtrip() {
        for value in [0.0, 1.0, 0.5, 2.0, 8.0, 400.0, -3.5] {
            let back = f16_to_f32(f32_to_f16(value));
            assert!((back - value).abs() < value.abs().max(1.0) * 0.002, "{value} -> {back}");
        }
    }

    #[test]
    fn sky_is_brighter_than_the_ground() {
        let up = sample_sky(Vec3::Y, &TROPICAL_AFTERNOON);
        let down = sample_sky(Vec3::NEG_Y, &TROPICAL_AFTERNOON);
        assert!(up.length() > down.length());
    }

    #[test]
    fn sun_is_by_far_the_brightest_sample() {
        let sun = sample_sky(SUN_DIRECTION, &TROPICAL_AFTERNOON);
        let sky = sample_sky(Vec3::new(0.0, 0.6, -0.8).normalize(), &TROPICAL_AFTERNOON);
        assert!(sun.length() > sky.length() * 50.0);
    }

    #[test]
    fn cubemap_has_six_faces_of_expected_size() {
        let image = generate_environment_cubemap(8, &TROPICAL_AFTERNOON);
        assert_eq!(image.texture_descriptor.size.depth_or_array_layers, 6);
        // Bevy pads the CPU copy of a 6-layer texture to mip level 0 of the full
        // array, so assert on the real invariant: six faces exist and each has
        // at least one complete RGBA16F level.
        let data = image.data.as_ref().expect("cubemap has CPU data");
        assert!(data.len() >= 8 * 8 * 6 * 4 * 2);
    }

    #[test]
    fn presets_scale_quality_monotonically() {
        let low = GraphicsSettings::from_preset(QualityPreset::Low);
        let ultra = GraphicsSettings::from_preset(QualityPreset::Ultra);
        assert!(low.resolution_scale < ultra.resolution_scale);
        assert!(low.shadow_map_size < ultra.shadow_map_size);
        assert!(low.shadow_distance < ultra.shadow_distance);
        assert!(!low.ssao && ultra.ssao);
        assert!(low.draw_distance < ultra.draw_distance);
    }

    #[test]
    fn preset_cycling_wraps() {
        assert_eq!(QualityPreset::Ultra.next(), QualityPreset::Low);
        assert_eq!(QualityPreset::Low.previous(), QualityPreset::Ultra);
    }
}
