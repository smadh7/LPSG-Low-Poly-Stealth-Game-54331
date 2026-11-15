//! The PBR material library.
//!
//! Every surface in the game is a [`StandardMaterial`] fed by three generated
//! maps (albedo, normal, ORM). Two details make the difference between "textured
//! boxes" and a believable environment:
//!
//! * **World-scale tiling.** [`MatId`] carries metres-per-repeat, so a 40 m
//!   terrace and a 0.6 m planter show the same stone at the same physical size.
//!   The geometry supplies UVs in metres; the shader repeats them.
//! * **Real roughness/metallic.** Stone is rough and dielectric, brass is metal
//!   with a low roughness, water is nearly a mirror. That contrast is most of
//!   what makes PBR lighting read as realistic.

use bevy::{
    image::{ImageAddressMode, ImageFilterMode, ImageSampler},
    pbr::{ExtendedMaterial, MaterialExtension, StandardMaterial},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef},
};

use super::mesh_kit::MatId;

/// Where the generated texture maps live, relative to the executable.
pub const TEXTURE_DIR: &str = "textures";

/// `uv_transform` scale that makes one texture image cover `metres`.
///
/// Geometry UVs are authored in **metres** (see `mesh_kit`), so the scale is the
/// reciprocal of the tile size. Getting this backwards is subtle and very
/// visible: it packs ~40 texture repeats into every metre, which reads as grey
/// mush at any distance.
fn uv_scale(metres: f32) -> Vec2 {
    Vec2::splat(1.0 / metres.max(0.01))
}

/// Metres covered by one texture repeat, per material.
fn tiling(mat: MatId) -> Vec2 {
    let m = match mat {
        MatId::Grass => 2.5,
        MatId::Sand => 3.0,
        MatId::Marble => 2.5,
        MatId::Limestone => 3.0,
        MatId::Travertine => 2.5,
        MatId::Plaster => 3.0,
        MatId::Wood => 2.0,
        MatId::Terracotta => 2.0,
        MatId::Concrete => 3.0,
        MatId::Brick => 1.5,
        MatId::Slate => 2.0,
        MatId::Metal => 2.0,
        MatId::StoneWall => 2.5,
        MatId::Roof => 4.0,
        MatId::Hedge => 1.5,
        MatId::PalmBark => 1.0,
        MatId::Frond => 1.0,
        MatId::Water => 8.0,
        MatId::WhitePaint => 2.0,
        MatId::DarkTrim => 1.0,
        MatId::Brass => 1.0,
        MatId::Rubber => 1.0,
        MatId::Asphalt => 4.0,
        MatId::Glass => 2.0,
        MatId::Fabric => 0.5,
        MatId::Skin => 0.5,
    };
    uv_scale(m)
}

/// Textured material name in the asset folder (water uses its normal map only).
fn texture_name(mat: MatId) -> &'static str {
    match mat {
        MatId::Grass => "grass",
        MatId::Sand => "sand",
        MatId::Marble => "marble",
        MatId::Limestone => "limestone",
        MatId::Travertine => "travertine",
        MatId::Plaster => "plaster",
        MatId::Wood => "wood",
        MatId::Terracotta => "terracotta",
        MatId::Concrete => "concrete",
        MatId::Brick => "brick",
        MatId::Slate => "slate",
        MatId::Metal => "metal",
        MatId::StoneWall => "stone_wall",
        MatId::Roof => "roof",
        MatId::Hedge => "hedge",
        MatId::PalmBark => "palm_bark",
        MatId::Frond => "frond",
        MatId::Water => "water_normal",
        MatId::WhitePaint => "white_paint",
        MatId::DarkTrim => "dark_trim",
        MatId::Brass => "brass",
        MatId::Rubber => "rubber",
        MatId::Asphalt => "asphalt",
        MatId::Glass => "dark_trim",
        MatId::Fabric => "fabric",
        MatId::Skin => "skin",
    }
}

/// Per-material PBR tuning that is not carried by the maps.
fn base_color(mat: MatId) -> LinearRgba {
    match mat {
        MatId::Glass => LinearRgba::new(0.06, 0.09, 0.10, 1.0),
        _ => LinearRgba::WHITE,
    }
}

/// Handles for one material's maps.
#[derive(Resource, Clone)]
pub struct MaterialLibrary {
    /// `StandardMaterial` per [`MatId`], indexed by `MatId::index()`.
    pub standard: [Handle<StandardMaterial>; MatId::ALL.len()],
    /// Water uses a custom extension with UV-scrolled normals.
    pub water: Handle<ExtendedMaterial<StandardMaterial, WaterMaterial>>,
    /// Unlit emissive material for light fixtures and screens.
    pub emissive_warm: Handle<StandardMaterial>,
    pub emissive_cool: Handle<StandardMaterial>,
    /// Translucent glass for balustrades and windows.
    pub glass: Handle<StandardMaterial>,
    pub greenery: Handle<StandardMaterial>,
}

impl MaterialLibrary {
    pub fn get(&self, mat: MatId) -> Handle<StandardMaterial> {
        self.standard[mat.index()].clone()
    }

    /// Build every material up front. Called once during startup.
    pub fn load(
        asset_server: &AssetServer,
        materials: &mut Assets<StandardMaterial>,
        water_materials: &mut Assets<ExtendedMaterial<StandardMaterial, WaterMaterial>>,
    ) -> Self {
        // Bevy's default sampler already repeats and filters with mipmaps
        // (see `ImagePlugin::default_nearest`/`linear`); UVs are in metres and
        // scaled here, so no per-image sampler override is needed.
        let _ = (
            ImageAddressMode::Repeat,
            ImageFilterMode::Linear,
            ImageSampler::linear(),
        );

        let mut standard: [Handle<StandardMaterial>; MatId::ALL.len()] =
            std::array::from_fn(|_| Handle::default());

        for mat in MatId::ALL {
            let name = texture_name(mat);
            let base = asset_server.load(format!("{TEXTURE_DIR}/{name}_basecolor.png"));
            let normal = asset_server.load(format!("{TEXTURE_DIR}/{name}_normal.png"));
            let orm = asset_server.load(format!("{TEXTURE_DIR}/{name}_orm.png"));

            let mut material = StandardMaterial {
                base_color: Color::from(base_color(mat)),
                base_color_texture: Some(base),
                normal_map_texture: Some(normal),
                occlusion_texture: Some(orm.clone()),
                metallic_roughness_texture: Some(orm),
                perceptual_roughness: 1.0,
                // The ORM map's blue channel is a *multiplier* on this value,
                // not a replacement, so 1.0 means "use the map as authored".
                metallic: 1.0,
                uv_transform: bevy::math::Affine2::from_scale(tiling(mat)),
                ..default()
            };

            // Per-material overrides that the maps cannot express.
            match mat {
                MatId::Glass => {
                    material.alpha_mode = AlphaMode::Blend;
                    material.perceptual_roughness = 0.05;
                    material.metallic = 0.0;
                    material.reflectance = 0.6;
                }
                MatId::Marble => {
                    // Polished stone: strong specular response, no metal.
                    material.reflectance = 0.55;
                }
                MatId::Metal | MatId::Brass => {
                    material.reflectance = 0.9;
                }
                MatId::Water => {
                    material.alpha_mode = AlphaMode::Blend;
                    material.base_color = Color::srgba(0.016, 0.055, 0.075, 0.82);
                    material.perceptual_roughness = 0.05;
                    material.metallic = 0.0;
                }
                MatId::Skin => {
                    // Skin is dielectric and slightly glossy; the ORM map carries
                    // pore-level roughness variation.
                    material.reflectance = 0.35;
                    material.metallic = 0.0;
                }
                MatId::Fabric => {
                    material.metallic = 0.0;
                    material.perceptual_roughness = 1.0;
                }
                MatId::Frond | MatId::Hedge | MatId::Grass => {
                    // Foliage: diffuse, slightly waxy, no metal response.
                    material.metallic = 0.0;
                    material.perceptual_roughness = 1.0;
                    material.double_sided = true;
                    material.cull_mode = None;
                }
                _ => {}
            }

            standard[mat.index()] = materials.add(material);
        }

        let water = water_materials.add(ExtendedMaterial {
            base: StandardMaterial {
                base_color: Color::srgba(0.010, 0.042, 0.058, 0.86),
                normal_map_texture: Some(asset_server.load(format!("{TEXTURE_DIR}/water_normal_normal.png"))),
                occlusion_texture: None,
                metallic_roughness_texture: None,
                perceptual_roughness: 0.04,
                metallic: 0.0,
                reflectance: 0.9,
                alpha_mode: AlphaMode::Blend,
                uv_transform: bevy::math::Affine2::from_scale(tiling(MatId::Water)),
                ..default()
            },
            extension: WaterMaterial {
                depth_tint: LinearRgba::new(0.012, 0.075, 0.095, 1.0),
                shallow_color: LinearRgba::new(0.055, 0.230, 0.230, 1.0),
                wave_strength: 0.55,
                wave_speed: 0.045,
                _marker: std::marker::PhantomData,
            },
        });

        // Interior lamps and screens, in radiance. `pbr_functions.wgsl` mixes
        // emissive by exposure when alpha is 1.0, so these are physical values:
        // roughly 10,000 cd/m² for a lamp diffuser, which clips to white the way
        // a bright light should.
        let emissive_warm = materials.add(StandardMaterial {
            base_color: Color::srgb(0.95, 0.85, 0.70),
            emissive: LinearRgba::new(9_000.0, 6_400.0, 3_600.0, 1.0),
            ..default()
        });
        let emissive_cool = materials.add(StandardMaterial {
            base_color: Color::srgb(0.80, 0.90, 1.0),
            emissive: LinearRgba::new(4_500.0, 7_000.0, 10_000.0, 1.0),
            ..default()
        });
        let glass = materials.add(StandardMaterial {
            base_color: Color::srgba(0.35, 0.45, 0.48, 0.30),
            perceptual_roughness: 0.02,
            metallic: 0.0,
            reflectance: 0.9,
            alpha_mode: AlphaMode::Blend,
            ..default()
        });
        let greenery = materials.add(StandardMaterial {
            base_color: Color::srgb(0.10, 0.22, 0.07),
            perceptual_roughness: 0.9,
            double_sided: true,
            cull_mode: None,
            ..default()
        });

        Self {
            standard,
            water,
            emissive_warm,
            emissive_cool,
            glass,
            greenery,
        }
    }
}

/// Custom water shader extension: two scrolling normal maps plus depth tinting.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct WaterMaterial {
    #[uniform(100)]
    pub depth_tint: LinearRgba,
    #[uniform(100)]
    pub shallow_color: LinearRgba,
    #[uniform(100)]
    pub wave_strength: f32,
    #[uniform(100)]
    pub wave_speed: f32,
    #[allow(dead_code)]
    pub _marker: std::marker::PhantomData<()>,
}

impl MaterialExtension for WaterMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path("shaders/water.wgsl".into())
    }
}

/// Marker so the water shader is registered with the asset system.
pub struct WaterPlugin;

impl Plugin for WaterPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<ExtendedMaterial<StandardMaterial, WaterMaterial>>::default());
    }
}
