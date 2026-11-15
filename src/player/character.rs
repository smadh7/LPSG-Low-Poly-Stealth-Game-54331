//! The humanoid character rig.
//!
//! Characters are built from anatomically proportioned parts (a 1.78 m adult:
//! 8-head figure, shoulder width ≈ 2 head widths, tapered limbs) rather than
//! boxes. Each part is its own entity that follows its actor, which keeps the
//! animation code simple: a limb is placed at its joint, rotated around it, and
//! the actor transform supplies position and facing.
//!
//! Parts are shaded with real PBR materials (skin, fabric, hair, leather) and
//! cast shadows, so characters sit in the scene instead of floating over it.

use super::{Stance, PLAYER_RADIUS};
use crate::world::mesh_kit::{MatId, MeshBuilder};
use bevy::prelude::*;
use std::f32::consts::FRAC_PI_2;

/// Root marker of the player character.
#[derive(Component)]
pub struct PlayerAvatar;

/// Collision + kinematic state of an actor (player or NPC).
#[derive(Component, Debug, Clone)]
pub struct ActorBody {
    pub radius: f32,
    pub height: f32,
    pub velocity: Vec3,
    pub grounded: bool,
    /// Absolute Y of the floor currently below the actor.
    pub ground_y: f32,
    pub stance: Stance,
    /// Current gait phase (radians) for the limb swing.
    pub gait: f32,
}

impl Default for ActorBody {
    fn default() -> Self {
        Self {
            radius: PLAYER_RADIUS,
            height: 1.78,
            velocity: Vec3::ZERO,
            grounded: true,
            ground_y: 0.0,
            stance: Stance::Standing,
            gait: 0.0,
        }
    }
}

/// A body part that follows an actor.
#[derive(Component, Debug)]
pub struct BodyPart {
    /// Actor this part belongs to.
    pub owner: Entity,
    /// Offset from the actor's origin, in the actor's local space.
    pub offset: Vec3,
    /// Limb identifier (drives procedural animation). `None` for static parts.
    pub limb: Option<Limb>,
    /// Rotation applied around the joint, in the actor's local space.
    pub rest_rotation: Quat,
    /// Base scale, so stance changes can squash the figure without rebuilding.
    pub scale: Vec3,
}

/// Marks the player's own visual parts.
#[derive(Component)]
pub struct PlayerVisual;

/// The parts of a character that can be animated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Limb {
    Hips,
    Torso,
    Neck,
    Head,
    LeftArm,
    LeftForearm,
    LeftHand,
    RightArm,
    RightForearm,
    RightHand,
    LeftLeg,
    LeftShin,
    LeftFoot,
    RightLeg,
    RightShin,
    RightFoot,
}

/// Joint positions for the default 1.78 m figure.
///
/// Heights are measured from the sole: ankles 0.10, knees 0.50, hips 0.92,
/// shoulders 1.45, neck 1.52, head centre 1.64.
pub struct HumanoidRig;

impl HumanoidRig {
    pub const HIP_HEIGHT: f32 = 0.92;
    pub const SHOULDER_HEIGHT: f32 = 1.45;
    pub const TORSO: Vec3 = Vec3::new(0.0, 1.18, 0.0);
    pub const HEAD: Vec3 = Vec3::new(0.0, 1.64, 0.0);
    pub const LEFT_LEG: Vec3 = Vec3::new(-0.105, 0.92, 0.0);
    pub const RIGHT_LEG: Vec3 = Vec3::new(0.105, 0.92, 0.0);
    pub const LEFT_ARM: Vec3 = Vec3::new(-0.225, 1.42, 0.0);
    pub const RIGHT_ARM: Vec3 = Vec3::new(0.225, 1.42, 0.0);
    /// Feet rest here; used to keep the figure's soles on the ground.
    pub const FOOT_HEIGHT: f32 = 0.09;
}

/// Colours of the player's default outfit (a linen shirt and tailored trousers).
pub mod outfit_colors {
    use bevy::prelude::*;
    pub const SKIN: Color = Color::srgb(0.78, 0.60, 0.47);
    pub const HAIR: Color = Color::srgb(0.20, 0.14, 0.10);
    pub const SHIRT: Color = Color::srgb(0.93, 0.92, 0.88);
    pub const TROUSERS: Color = Color::srgb(0.30, 0.34, 0.42);
    pub const SHOES: Color = Color::srgb(0.12, 0.11, 0.11);
}

/// The materials a character is dressed in. Six slots are enough for every
/// outfit in the resort while keeping characters down to a handful of draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Outfit {
    pub skin: SkinTone,
    pub shirt: GarmentColor,
    pub trousers: GarmentColor,
    pub shoes: GarmentColor,
    pub hair: GarmentColor,
}

/// Skin tones, tuned across a realistic spectrum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SkinTone {
    Porcelain,
    Fair,
    Olive,
    Tan,
    Brown,
    Deep,
}

impl SkinTone {
    pub const ALL: [SkinTone; 6] = [
        SkinTone::Porcelain,
        SkinTone::Fair,
        SkinTone::Olive,
        SkinTone::Tan,
        SkinTone::Brown,
        SkinTone::Deep,
    ];

    pub fn color(self) -> Color {
        match self {
            SkinTone::Porcelain => Color::srgb(0.86, 0.71, 0.62),
            SkinTone::Fair => Color::srgb(0.80, 0.63, 0.51),
            SkinTone::Olive => Color::srgb(0.72, 0.55, 0.41),
            SkinTone::Tan => Color::srgb(0.62, 0.44, 0.30),
            SkinTone::Brown => Color::srgb(0.44, 0.29, 0.19),
            SkinTone::Deep => Color::srgb(0.28, 0.18, 0.12),
        }
    }
}

/// Clothing colours used by staff, guests and the player.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GarmentColor {
    Linen,
    Navy,
    Charcoal,
    Olive,
    Sand,
    Cream,
    Burgundy,
    Teal,
    White,
    Black,
    HotelGold,
    SecurityBlue,
    HairBrown,
    HairBlack,
    HairBlonde,
    HairGinger,
    HairGrey,
}

impl GarmentColor {
    pub fn rgb(self) -> Color {
        match self {
            GarmentColor::Linen => Color::srgb(0.88, 0.86, 0.80),
            GarmentColor::Navy => Color::srgb(0.13, 0.17, 0.29),
            GarmentColor::Charcoal => Color::srgb(0.16, 0.17, 0.18),
            GarmentColor::Olive => Color::srgb(0.28, 0.31, 0.20),
            GarmentColor::Sand => Color::srgb(0.74, 0.66, 0.52),
            GarmentColor::Cream => Color::srgb(0.93, 0.90, 0.82),
            GarmentColor::Burgundy => Color::srgb(0.35, 0.11, 0.14),
            GarmentColor::Teal => Color::srgb(0.10, 0.31, 0.32),
            GarmentColor::White => Color::srgb(0.95, 0.95, 0.93),
            GarmentColor::Black => Color::srgb(0.07, 0.07, 0.08),
            GarmentColor::HotelGold => Color::srgb(0.66, 0.51, 0.22),
            GarmentColor::SecurityBlue => Color::srgb(0.11, 0.15, 0.24),
            GarmentColor::HairBrown => Color::srgb(0.22, 0.15, 0.09),
            GarmentColor::HairBlack => Color::srgb(0.07, 0.06, 0.06),
            GarmentColor::HairBlonde => Color::srgb(0.68, 0.55, 0.32),
            GarmentColor::HairGinger => Color::srgb(0.45, 0.22, 0.10),
            GarmentColor::HairGrey => Color::srgb(0.60, 0.60, 0.60),
        }
    }
}

/// Per-character material handles, resolved once and shared.
#[derive(Resource, Clone)]
pub struct OutfitMaterials {
    pub skin: [Handle<StandardMaterial>; 6],
    pub garments: std::collections::HashMap<GarmentColor, Handle<StandardMaterial>>,
    pub hair: std::collections::HashMap<GarmentColor, Handle<StandardMaterial>>,
}

impl OutfitMaterials {
    pub fn skin(&self, tone: SkinTone) -> Handle<StandardMaterial> {
        let index = SkinTone::ALL.iter().position(|t| *t == tone).unwrap_or(3);
        self.skin[index].clone()
    }

    pub fn garment(&self, color: GarmentColor) -> Handle<StandardMaterial> {
        self.garments
            .get(&color)
            .or_else(|| self.garments.get(&GarmentColor::Linen))
            .cloned()
            .unwrap_or_default()
    }

    pub fn hair(&self, color: GarmentColor) -> Handle<StandardMaterial> {
        self.hair
            .get(&color)
            .or_else(|| self.hair.get(&GarmentColor::HairBrown))
            .cloned()
            .unwrap_or_default()
    }
}

/// Shared meshes for humanoid characters (created once).
#[derive(Resource, Clone)]
pub struct CharacterMeshes {
    pub torso: Handle<Mesh>,
    pub hips: Handle<Mesh>,
    pub head: Handle<Mesh>,
    pub hair: [Handle<Mesh>; 3],
    pub upper_arm: Handle<Mesh>,
    pub forearm: Handle<Mesh>,
    pub hand: Handle<Mesh>,
    pub thigh: Handle<Mesh>,
    pub shin: Handle<Mesh>,
    pub shoe: Handle<Mesh>,
    pub collar: Handle<Mesh>,
}

impl CharacterMeshes {
    pub fn create(meshes: &mut Assets<Mesh>) -> Self {
        // Torso: tapered chest + shoulder yoke, pivoting at the hips.
        let torso = meshes.add(
            MeshBuilder::new()
                .tap(|m| {
                    // Chest (wide at the shoulders, narrowing to the waist).
                    m.add_taper(MatId::Fabric, Vec3::new(0.0, 0.10, 0.0), 0.145, 0.185, 0.46, 12);
                    // Shoulder caps.
                    m.add_sphere(MatId::Fabric, Vec3::new(-0.185, 0.55, 0.0), Vec3::new(0.085, 0.075, 0.095), 4, 8);
                    m.add_sphere(MatId::Fabric, Vec3::new(0.185, 0.55, 0.0), Vec3::new(0.085, 0.075, 0.095), 4, 8);
                    // Neck.
                    m.add_cylinder(MatId::Skin, Vec3::new(0.0, 0.52, 0.0), 0.055, 0.11, 8);
                })
                .into_mesh(),
        );
        let hips = meshes.add(
            MeshBuilder::new()
                .tap(|m| {
                    m.add_taper(MatId::Fabric, Vec3::new(0.0, 0.0, 0.0), 0.165, 0.150, 0.22, 12);
                })
                .into_mesh(),
        );
        // Head: skull, brow, jaw, nose, ears — enough landmarks for the
        // silhouette and for the face to catch light correctly.
        let head = meshes.add(
            MeshBuilder::new()
                .tap(|m| {
                    let c = Vec3::new(0.0, 0.0, 0.0);
                    m.add_sphere(MatId::Skin, c, Vec3::new(0.098, 0.122, 0.108), 10, 14);
                    // Jaw / chin.
                    m.add_sphere(MatId::Skin, c + Vec3::new(0.0, -0.062, 0.012), Vec3::new(0.078, 0.062, 0.086), 6, 10);
                    // Brow ridge.
                    m.add_sphere(MatId::Skin, c + Vec3::new(0.0, 0.040, 0.084), Vec3::new(0.088, 0.020, 0.030), 4, 8);
                    // Nose.
                    m.add_taper(MatId::Skin, c + Vec3::new(0.0, -0.012, 0.088), 0.020, 0.030, 0.062, 6);
                    // Cheekbones.
                    for sx in [-1.0_f32, 1.0] {
                        m.add_sphere(MatId::Skin, c + Vec3::new(sx * 0.062, -0.008, 0.070), Vec3::new(0.034, 0.030, 0.028), 4, 6);
                        // Ears.
                        m.add_sphere(MatId::Skin, c + Vec3::new(sx * 0.100, 0.002, -0.004), Vec3::new(0.016, 0.036, 0.026), 4, 6);
                    }
                    // Eye sockets (darker recess reads as eyes at distance).
                    for sx in [-1.0_f32, 1.0] {
                        m.add_sphere(MatId::DarkTrim, c + Vec3::new(sx * 0.038, 0.018, 0.098), Vec3::new(0.020, 0.014, 0.010), 4, 6);
                    }
                })
                .into_mesh(),
        );
        // Three hair shapes: short, long, tied back.
        let hair_short = meshes.add(
            MeshBuilder::new()
                .tap(|m| {
                    m.add_sphere(MatId::Rubber, Vec3::new(0.0, 0.028, -0.012), Vec3::new(0.104, 0.104, 0.114), 8, 12);
                })
                .into_mesh(),
        );
        let hair_long = meshes.add(
            MeshBuilder::new()
                .tap(|m| {
                    m.add_sphere(MatId::Rubber, Vec3::new(0.0, 0.026, -0.014), Vec3::new(0.106, 0.108, 0.118), 8, 12);
                    m.add_taper(MatId::Rubber, Vec3::new(0.0, -0.30, -0.02), 0.098, 0.070, 0.30, 10);
                })
                .into_mesh(),
        );
        let hair_ponytail = meshes.add(
            MeshBuilder::new()
                .tap(|m| {
                    m.add_sphere(MatId::Rubber, Vec3::new(0.0, 0.030, -0.010), Vec3::new(0.103, 0.100, 0.112), 8, 12);
                    m.add_taper(MatId::Rubber, Vec3::new(0.0, -0.16, -0.10), 0.048, 0.022, 0.20, 8);
                })
                .into_mesh(),
        );
        let upper_arm = meshes.add(
            MeshBuilder::new()
                .tap(|m| {
                    m.add_taper(MatId::Fabric, Vec3::new(0.0, -0.30, 0.0), 0.070, 0.056, 0.30, 10);
                })
                .into_mesh(),
        );
        let forearm = meshes.add(
            MeshBuilder::new()
                .tap(|m| {
                    m.add_taper(MatId::Skin, Vec3::new(0.0, -0.26, 0.0), 0.052, 0.038, 0.26, 10);
                })
                .into_mesh(),
        );
        let hand = meshes.add(
            MeshBuilder::new()
                .tap(|m| {
                    m.add_sphere(MatId::Skin, Vec3::new(0.0, -0.035, 0.0), Vec3::new(0.040, 0.062, 0.028), 4, 8);
                })
                .into_mesh(),
        );
        let thigh = meshes.add(
            MeshBuilder::new()
                .tap(|m| {
                    m.add_taper(MatId::Fabric, Vec3::new(0.0, -0.44, 0.0), 0.088, 0.068, 0.44, 10);
                })
                .into_mesh(),
        );
        let shin = meshes.add(
            MeshBuilder::new()
                .tap(|m| {
                    m.add_taper(MatId::Fabric, Vec3::new(0.0, -0.42, 0.0), 0.066, 0.048, 0.42, 10);
                })
                .into_mesh(),
        );
        let shoe = meshes.add(
            MeshBuilder::new()
                .tap(|m| {
                    m.add_box(MatId::Rubber, Vec3::new(0.0, -0.045, 0.035), Vec3::new(0.090, 0.075, 0.235));
                    m.add_sphere(MatId::Rubber, Vec3::new(0.0, -0.06, 0.140), Vec3::new(0.045, 0.035, 0.045), 4, 6);
                })
                .into_mesh(),
        );
        let collar = meshes.add(
            MeshBuilder::new()
                .tap(|m| {
                    m.add_cylinder(MatId::WhitePaint, Vec3::new(0.0, 0.0, 0.0), 0.062, 0.028, 10);
                })
                .into_mesh(),
        );

        Self {
            torso,
            hips,
            head,
            hair: [hair_short, hair_long, hair_ponytail],
            upper_arm,
            forearm,
            hand,
            thigh,
            shin,
            shoe,
            collar,
        }
    }

    /// Load the shared meshes, creating and inserting them on first use.
    ///
    /// Callers pass `Option<Res<CharacterMeshes>>` rather than `&World`: a
    /// shared `&World` parameter conflicts with the `ResMut<Assets<_>>`
    /// parameters of the spawning systems.
    pub fn get_or_insert(
        meshes: &mut Assets<Mesh>,
        commands: &mut Commands,
        existing: Option<Res<CharacterMeshes>>,
    ) -> Self {
        if let Some(existing) = existing {
            return existing.clone();
        }
        let created = Self::create(meshes);
        commands.insert_resource(created.clone());
        created
    }
}

/// Builder helper so mesh construction reads tidily above.
trait Tap: Sized {
    fn tap(mut self, f: impl FnOnce(&mut Self)) -> Self {
        f(&mut self);
        self
    }
}

impl Tap for MeshBuilder {
    fn tap(mut self, f: impl FnOnce(&mut Self)) -> Self {
        f(&mut self);
        self
    }
}

impl MeshBuilder {
    /// Build into a single merged mesh (character parts are one material each).
    pub fn into_mesh(self) -> Mesh {
        let mut buckets = self.finish();
        match buckets.len() {
            0 => crate::world::mesh_kit::Bucket::default().into_mesh(),
            1 => buckets.remove(0).1,
            _ => {
                // Merge every bucket into the first: character parts are small,
                // and a single mesh per part keeps the part count flat.
                let mut merged = buckets.remove(0).1;
                for (_, other) in buckets {
                    crate::world::mesh_kit::merge_into(&mut merged, other);
                }
                merged
            }
        }
    }
}

/// Create every skin, garment and hair material once.
///
/// Garments share the woven-fabric PBR maps (with normal and roughness detail)
/// and differ only in base colour; skin shares the pore-level skin maps. That
/// keeps the material count small while every character still reads as cloth.
pub fn build_outfit_materials(
    asset_server: &AssetServer,
    materials: &mut Assets<StandardMaterial>,
) -> OutfitMaterials {
    let fabric_base = asset_server.load(format!("{}/fabric_basecolor.png", crate::world::materials::TEXTURE_DIR));
    let fabric_normal = asset_server.load(format!("{}/fabric_normal.png", crate::world::materials::TEXTURE_DIR));
    let fabric_orm = asset_server.load(format!("{}/fabric_orm.png", crate::world::materials::TEXTURE_DIR));
    let skin_base = asset_server.load(format!("{}/skin_basecolor.png", crate::world::materials::TEXTURE_DIR));
    let skin_normal = asset_server.load(format!("{}/skin_normal.png", crate::world::materials::TEXTURE_DIR));
    let skin_orm = asset_server.load(format!("{}/skin_orm.png", crate::world::materials::TEXTURE_DIR));

    let skin = std::array::from_fn(|index| {
        materials.add(StandardMaterial {
            base_color: SkinTone::ALL[index].color(),
            base_color_texture: Some(skin_base.clone()),
            normal_map_texture: Some(skin_normal.clone()),
            occlusion_texture: Some(skin_orm.clone()),
            metallic_roughness_texture: Some(skin_orm.clone()),
            perceptual_roughness: 1.0,
            metallic: 1.0,
            reflectance: 0.35,
            uv_transform: bevy::math::Affine2::from_scale(Vec2::splat(2.0)),
            ..default()
        })
    });

    use GarmentColor as G;
    let garment_colors = [
        G::Linen, G::Navy, G::Charcoal, G::Olive, G::Sand, G::Cream, G::Burgundy, G::Teal,
        G::White, G::Black, G::HotelGold, G::SecurityBlue,
    ];
    let garment = |materials: &mut Assets<StandardMaterial>, color: Color, rough: f32| {
        materials.add(StandardMaterial {
            base_color: color,
            base_color_texture: Some(fabric_base.clone()),
            normal_map_texture: Some(fabric_normal.clone()),
            occlusion_texture: Some(fabric_orm.clone()),
            metallic_roughness_texture: Some(fabric_orm.clone()),
            perceptual_roughness: rough,
            metallic: 1.0,
            uv_transform: bevy::math::Affine2::from_scale(Vec2::splat(2.0)),
            ..default()
        })
    };

    let mut garments = std::collections::HashMap::new();
    for color in garment_colors {
        // Uniforms and dark fabrics are a little tighter and glossier.
        let rough = match color {
            G::HotelGold | G::SecurityBlue | G::Black => 0.86,
            _ => 0.95,
        };
        garments.insert(color, garment(materials, color.rgb(), rough));
    }

    let hair_colors = [
        G::HairBrown,
        G::HairBlack,
        G::HairBlonde,
        G::HairGinger,
        G::HairGrey,
    ];
    let mut hair = std::collections::HashMap::new();
    for color in hair_colors {
        hair.insert(
            color,
            materials.add(StandardMaterial {
                base_color: color.rgb(),
                perceptual_roughness: 0.55,
                metallic: 0.0,
                reflectance: 0.25,
                ..default()
            }),
        );
    }

    OutfitMaterials {
        skin,
        garments,
        hair,
    }
}

/// Spawn a complete humanoid visual for `owner`.
#[allow(clippy::too_many_arguments)]
pub fn spawn_humanoid(
    commands: &mut Commands,
    owner: Entity,
    origin: Vec3,
    rig: &CharacterMeshes,
    outfits: &OutfitMaterials,
    outfit: Outfit,
    marker: Option<PlayerVisual>,
) -> Vec<Entity> {
    let skin = outfits.skin(outfit.skin);
    let shirt = outfits.garment(outfit.shirt);
    let trousers = outfits.garment(outfit.trousers);
    let shoes = outfits.garment(outfit.shoes);
    let hair = outfits.hair(outfit.hair);

    // Which hair mesh depends on the colour enum: long styles for two of them.
    let hair_mesh = match outfit.hair {
        GarmentColor::HairBlonde | GarmentColor::HairGinger => rig.hair[1].clone(),
        GarmentColor::HairGrey => rig.hair[2].clone(),
        _ => rig.hair[0].clone(),
    };

    struct Part {
        mesh: Handle<Mesh>,
        material: Handle<StandardMaterial>,
        offset: Vec3,
        limb: Option<Limb>,
        rest: Quat,
        scale: Vec3,
    }
    let rot = |x: f32| Quat::from_rotation_x(x);
    let parts = vec![
        Part { mesh: rig.hips.clone(), material: trousers.clone(), offset: Vec3::new(0.0, HumanoidRig::HIP_HEIGHT, 0.0), limb: Some(Limb::Hips), rest: Quat::IDENTITY, scale: Vec3::ONE },
        Part { mesh: rig.torso.clone(), material: shirt.clone(), offset: HumanoidRig::TORSO, limb: Some(Limb::Torso), rest: Quat::IDENTITY, scale: Vec3::ONE },
        Part { mesh: rig.collar.clone(), material: shirt.clone(), offset: HumanoidRig::TORSO + Vec3::new(0.0, 0.53, 0.0), limb: Some(Limb::Neck), rest: Quat::IDENTITY, scale: Vec3::ONE },
        Part { mesh: rig.head.clone(), material: skin.clone(), offset: HumanoidRig::HEAD, limb: Some(Limb::Head), rest: Quat::IDENTITY, scale: Vec3::ONE },
        Part { mesh: hair_mesh, material: hair, offset: HumanoidRig::HEAD, limb: Some(Limb::Head), rest: Quat::IDENTITY, scale: Vec3::ONE },
        // Arms: upper arm pivots at the shoulder, forearm at the elbow, hand at the wrist.
        Part { mesh: rig.upper_arm.clone(), material: shirt.clone(), offset: HumanoidRig::LEFT_ARM, limb: Some(Limb::LeftArm), rest: Quat::IDENTITY, scale: Vec3::ONE },
        Part { mesh: rig.forearm.clone(), material: skin.clone(), offset: HumanoidRig::LEFT_ARM + Vec3::new(-0.015, -0.30, 0.0), limb: Some(Limb::LeftForearm), rest: Quat::IDENTITY, scale: Vec3::ONE },
        Part { mesh: rig.hand.clone(), material: skin.clone(), offset: HumanoidRig::LEFT_ARM + Vec3::new(-0.015, -0.56, 0.02), limb: Some(Limb::LeftHand), rest: Quat::IDENTITY, scale: Vec3::ONE },
        Part { mesh: rig.upper_arm.clone(), material: shirt.clone(), offset: HumanoidRig::RIGHT_ARM, limb: Some(Limb::RightArm), rest: Quat::IDENTITY, scale: Vec3::ONE },
        Part { mesh: rig.forearm.clone(), material: skin.clone(), offset: HumanoidRig::RIGHT_ARM + Vec3::new(0.015, -0.30, 0.0), limb: Some(Limb::RightForearm), rest: Quat::IDENTITY, scale: Vec3::ONE },
        Part { mesh: rig.hand.clone(), material: skin.clone(), offset: HumanoidRig::RIGHT_ARM + Vec3::new(0.015, -0.56, 0.02), limb: Some(Limb::RightHand), rest: Quat::IDENTITY, scale: Vec3::ONE },
        // Legs: thigh at the hip, shin at the knee, shoe at the ankle.
        Part { mesh: rig.thigh.clone(), material: trousers.clone(), offset: HumanoidRig::LEFT_LEG, limb: Some(Limb::LeftLeg), rest: Quat::IDENTITY, scale: Vec3::ONE },
        Part { mesh: rig.shin.clone(), material: trousers.clone(), offset: HumanoidRig::LEFT_LEG + Vec3::new(0.0, -0.44, 0.0), limb: Some(Limb::LeftShin), rest: rot(0.10), scale: Vec3::ONE },
        Part { mesh: rig.shoe.clone(), material: shoes.clone(), offset: HumanoidRig::LEFT_LEG + Vec3::new(0.0, -0.83, 0.0), limb: Some(Limb::LeftFoot), rest: Quat::IDENTITY, scale: Vec3::ONE },
        Part { mesh: rig.thigh.clone(), material: trousers.clone(), offset: HumanoidRig::RIGHT_LEG, limb: Some(Limb::RightLeg), rest: Quat::IDENTITY, scale: Vec3::ONE },
        Part { mesh: rig.shin.clone(), material: trousers.clone(), offset: HumanoidRig::RIGHT_LEG + Vec3::new(0.0, -0.44, 0.0), limb: Some(Limb::RightShin), rest: rot(0.10), scale: Vec3::ONE },
        Part { mesh: rig.shoe.clone(), material: shoes, offset: HumanoidRig::RIGHT_LEG + Vec3::new(0.0, -0.83, 0.0), limb: Some(Limb::RightFoot), rest: Quat::IDENTITY, scale: Vec3::ONE },
    ];

    let mut spawned = Vec::new();
    for part in parts {
        let mut entity = commands.spawn((
            BodyPart {
                owner,
                offset: part.offset,
                limb: part.limb,
                rest_rotation: part.rest,
                scale: part.scale,
            },
            Mesh3d(part.mesh),
            MeshMaterial3d(part.material),
            Transform::from_translation(origin + part.offset),
        ));
        if marker.is_some() {
            entity.insert(PlayerVisual);
        }
        spawned.push(entity.id());
    }
    spawned
}

/// Joint chain: a part offset is rotated by its own joint *and* every parent
/// joint, so forearms follow the shoulder and shins follow the hip.
fn joint_chain(limb: Limb) -> &'static [Limb] {
    match limb {
        Limb::LeftArm => &[Limb::LeftArm],
        Limb::LeftForearm => &[Limb::LeftArm, Limb::LeftForearm],
        Limb::LeftHand => &[Limb::LeftArm, Limb::LeftForearm, Limb::LeftHand],
        Limb::RightArm => &[Limb::RightArm],
        Limb::RightForearm => &[Limb::RightArm, Limb::RightForearm],
        Limb::RightHand => &[Limb::RightArm, Limb::RightForearm, Limb::RightHand],
        Limb::LeftLeg => &[Limb::LeftLeg],
        Limb::LeftShin => &[Limb::LeftLeg, Limb::LeftShin],
        Limb::LeftFoot => &[Limb::LeftLeg, Limb::LeftShin, Limb::LeftFoot],
        Limb::RightLeg => &[Limb::RightLeg],
        Limb::RightShin => &[Limb::RightLeg, Limb::RightShin],
        Limb::RightFoot => &[Limb::RightLeg, Limb::RightShin, Limb::RightFoot],
        Limb::Hips | Limb::Torso | Limb::Neck | Limb::Head => &[],
    }
}

/// Rotation for one joint, given the actor's motion.
fn joint_angle(limb: Limb, body: &ActorBody, phase: f32, swing: f32) -> f32 {
    let grounded = body.grounded;
    let s = phase.sin() * swing;
    match limb {
        // Hips/legs.
        Limb::LeftLeg => {
            if grounded {
                s
            } else {
                0.45
            }
        }
        Limb::RightLeg => {
            if grounded {
                -s
            } else {
                -0.35
            }
        }
        // Knees bend backwards, and only on the forward swing.
        Limb::LeftShin => {
            if grounded {
                (1.0 - phase.sin()).max(0.0) * swing * 0.85
            } else {
                0.7
            }
        }
        Limb::RightShin => {
            if grounded {
                (1.0 + phase.sin()).max(0.0) * swing * 0.85
            } else {
                0.7
            }
        }
        Limb::LeftFoot | Limb::RightFoot => 0.0,
        // Arms counter-swing the legs.
        Limb::LeftArm => {
            if grounded {
                -s * 0.75
            } else {
                -0.55
            }
        }
        Limb::RightArm => {
            if grounded {
                s * 0.75
            } else {
                -0.55
            }
        }
        // Elbows bend slightly and follow the arm.
        Limb::LeftForearm | Limb::RightForearm => 0.18 + swing * 0.25,
        Limb::LeftHand | Limb::RightHand => 0.0,
        Limb::Hips => 0.0,
        Limb::Torso | Limb::Neck => match body.stance {
            Stance::Sprinting => -0.14,
            Stance::Crouching => 0.16,
            _ => 0.0,
        },
        Limb::Head => {
            if body.stance == Stance::Crouching {
                -0.22
            } else {
                0.0
            }
        }
    }
}

/// Copy each actor's transform onto its body parts and animate the limbs.
///
/// The gait comes from the actor's speed, so walking, jogging and running all
/// read correctly without any animation assets, and the torso bobs with the
/// stride the way a real gait does.
pub fn sync_body_parts(
    time: Res<Time>,
    actors: Query<(&Transform, &ActorBody)>,
    mut parts: Query<(&BodyPart, &mut Transform), Without<ActorBody>>,
) {
    let clock = time.elapsed_secs();
    for (part, mut transform) in &mut parts {
        let Ok((actor_transform, body)) = actors.get(part.owner) else {
            continue;
        };
        let yaw = actor_transform.rotation.to_euler(EulerRot::YXZ).0;
        let root_yaw = Quat::from_rotation_y(yaw);
        let Some(limb) = part.limb else {
            continue;
        };

        let speed = Vec3::new(body.velocity.x, 0.0, body.velocity.z).length();
        let gait = (speed / 2.0).clamp(0.0, 1.6);
        let swing = (gait * 0.52).min(0.9);
        let phase = clock * (0.9 + gait * 4.6);

        // Crouching lowers the pelvis and shortens the stance.
        let crouch = if body.stance == Stance::Crouching {
            0.30
        } else {
            0.0
        };
        // Vertical bob: two steps per gait cycle, plus weapon-free idle breathing.
        let bob = if body.grounded && speed > 0.2 {
            (phase * 2.0).sin() * 0.022 * gait
        } else if speed < 0.1 && body.grounded {
            (clock * 1.9).sin() * 0.006
        } else {
            0.0
        };

        // Accumulate the parent joint rotations.
        let mut rotation = root_yaw * part.rest_rotation;
        for joint in joint_chain(limb) {
            rotation = rotation
                * Quat::from_rotation_x(joint_angle(*joint, body, phase, swing));
        }
        if matches!(limb, Limb::Head) {
            // Idle look-around: small yaw oscillation makes crowds feel alive.
            let look = if speed < 0.2 {
                (clock * 0.42 + part.owner.index() as f32).sin() * 0.22
            } else {
                0.0
            };
            rotation = root_yaw * Quat::from_rotation_y(look) * part.rest_rotation;
            rotation = rotation * Quat::from_rotation_x(joint_angle(limb, body, phase, swing));
        }

        transform.translation =
            actor_transform.translation + root_yaw * (part.offset - Vec3::Y * crouch) + Vec3::Y * bob;
        transform.rotation = rotation;
        transform.scale = part.scale;
    }
}

/// The avatar's forward vector is +Z at yaw 0.
pub const AVATAR_FORWARD_YAW: f32 = 0.0;
const _: f32 = FRAC_PI_2;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actor_body_defaults_are_sane() {
        let body = ActorBody::default();
        assert!(body.radius > 0.0);
        assert!(body.height > 1.6 && body.height < 2.0);
        assert_eq!(body.stance, Stance::Standing);
    }

    #[test]
    fn rig_proportions_are_human() {
        // A 1.78 m figure: head ~1/7.5 of height, shoulders near 1.45 m.
        let head_top = HumanoidRig::HEAD.y + 0.13;
        assert!((head_top - 1.78).abs() < 0.08, "figure is {head_top} m tall");
        let head_ratio = head_top / 0.26;
        assert!(head_ratio > 6.0 && head_ratio < 8.5, "heads tall: {head_ratio}");
        assert!(HumanoidRig::SHOULDER_HEIGHT > HumanoidRig::HIP_HEIGHT);
        assert!(HumanoidRig::LEFT_LEG.y - HumanoidRig::FOOT_HEIGHT < 1.0);
    }

    #[test]
    fn skin_tones_are_all_distinct() {
        let mut seen = std::collections::HashSet::new();
        for tone in SkinTone::ALL {
            let c = tone.color().to_srgba();
            assert!(seen.insert((
                (c.red * 255.0) as u8,
                (c.green * 255.0) as u8,
                (c.blue * 255.0) as u8
            )));
        }
    }

    #[test]
    fn limb_meshes_are_tapered_not_boxes() {
        // Limbs must have circular cross-sections (more than 4 sides) so they do
        // not read as boxes in silhouette.
        let mut assets = Assets::<Mesh>::default();
        let rig = CharacterMeshes::create(&mut assets);
        let thigh = assets.get(&rig.thigh).expect("thigh mesh");
        assert!(thigh.count_vertices() > 60, "thigh has {} verts", thigh.count_vertices());
        let head = assets.get(&rig.head).expect("head mesh");
        assert!(head.count_vertices() > 400, "head is too simple: {} verts", head.count_vertices());
    }
}
