//! Player: third-person camera rig, movement, crouching, jumping and physics.
//!
//! The player is *not* a physics-engine body: the resort is made of axis
//! aligned boxes, so a small bespoke swept-circle solver is both faster and far
//! easier to keep predictable than a full rigid-body simulation.

pub mod camera;
pub mod character;
pub mod movement;
pub mod spawn;

use bevy::prelude::*;
use camera::ThirdPersonCamera;
use character::PlayerAvatar;

/// Input intent produced by [`movement::read_input`] and consumed by the
/// physics step. Keeping the intent in a resource makes it easy to drive the
/// player from other systems later (cutscenes, scripted sequences).
#[derive(Resource, Debug, Default)]
pub struct PlayerIntent {
    /// Desired movement direction, in world space, normalised.
    pub direction: Vec3,
    pub wants_run: bool,
    pub wants_crouch: bool,
    pub wants_jump: bool,
    /// Camera-relative look input applied by the camera rig.
    pub look: Vec2,
}

/// High level locomotion state used by animation and the HUD.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Stance {
    #[default]
    Standing,
    Crouching,
    Sprinting,
    Airborne,
}

impl Stance {
    pub fn label(self) -> &'static str {
        match self {
            Stance::Standing => "STANDING",
            Stance::Crouching => "CROUCHED",
            Stance::Sprinting => "RUNNING",
            Stance::Airborne => "AIRBORNE",
        }
    }
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerIntent>()
            .init_resource::<camera::CameraRig>()
            .add_systems(PreStartup, build_character_materials)
            .add_systems(Startup, spawn::spawn_player)
            .add_systems(
                Update,
                (
                    movement::read_input,
                    camera::camera_look,
                    camera::camera_follow,
                )
                    .chain(),
            )
            .add_systems(FixedUpdate, movement::player_physics);
    }
}

/// Create the shared character material palette once.
fn build_character_materials(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let outfits = character::build_outfit_materials(&asset_server, &mut materials);
    commands.insert_resource(outfits);
}

/// Simple marker component for anything that should track the player.
#[derive(Component)]
pub struct FollowsPlayer;

/// Shared constant for the player's collision radius.
pub const PLAYER_RADIUS: f32 = 0.38;
/// Eye offset above the player origin used by the camera.
pub const EYE_HEIGHT: f32 = 1.62;
/// Crouched eye height.
pub const CROUCH_EYE_HEIGHT: f32 = 1.02;

/// Convenience query item describing the player.
pub type PlayerQuery<'w, 's> = Query<'w, 's, (&'static Transform, &'static PlayerAvatar), (With<PlayerAvatar>, Without<ThirdPersonCamera>)>;
