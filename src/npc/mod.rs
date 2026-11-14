//! NPC foundation: schedules, waypoint navigation, idle behaviour, collisions
//! with the player and a low-poly body with simple walk animation.
//!
//! Stage 1 keeps the intelligence deliberately simple but the architecture is
//! ready for Stage 2's suspicion / recognition systems: NPCs already expose a
//! [`Brain`] component, a perception cone and a schedule of [`RoutineStep`]s.

pub mod brain;
pub mod navigation;
pub mod schedule;
pub mod spawn;

use bevy::prelude::*;
use brain::Brain;

use crate::core::ZoneId;
use crate::player_state::OutfitKind;

/// Marker for an NPC actor.
#[derive(Component)]
pub struct Npc;

/// The role an NPC plays at the resort.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcRole {
    Guest,
    Waiter,
    Bartender,
    Concierge,
    Housekeeping,
    Chef,
    SecurityGuard,
    Manager,
    Target,
}

impl NpcRole {
    pub fn display_name(self) -> &'static str {
        match self {
            NpcRole::Guest => "Guest",
            NpcRole::Waiter => "Waiter",
            NpcRole::Bartender => "Bartender",
            NpcRole::Concierge => "Concierge",
            NpcRole::Housekeeping => "Housekeeping",
            NpcRole::Chef => "Chef",
            NpcRole::SecurityGuard => "Security",
            NpcRole::Manager => "Manager",
            NpcRole::Target => "Target",
        }
    }

    /// Which outfit the NPC wears (used for recognition in Stage 2).
    pub fn outfit(self) -> OutfitKind {
        match self {
            NpcRole::Guest | NpcRole::Target => OutfitKind::Tourist,
            NpcRole::SecurityGuard => OutfitKind::Security,
            _ => OutfitKind::Staff,
        }
    }

    /// Whether the NPC belongs to the security team.
    pub fn is_security(self) -> bool {
        matches!(self, NpcRole::SecurityGuard)
    }

    /// Whether this NPC is allowed in restricted zones.
    pub fn can_enter_restricted(self) -> bool {
        matches!(self, NpcRole::SecurityGuard | NpcRole::Target | NpcRole::Manager)
    }

    /// Primary body colours for the procedural model.
    pub fn colors(self) -> (Color, Color, Color) {
        // (torso, legs, accent)
        match self {
            NpcRole::Guest => (
                Color::srgb(0.85, 0.55, 0.45),
                Color::srgb(0.90, 0.86, 0.74),
                Color::srgb(0.95, 0.82, 0.35),
            ),
            NpcRole::Waiter => (
                Color::srgb(0.16, 0.17, 0.20),
                Color::srgb(0.14, 0.15, 0.18),
                Color::srgb(0.95, 0.95, 0.95),
            ),
            NpcRole::Bartender => (
                Color::srgb(0.20, 0.24, 0.30),
                Color::srgb(0.16, 0.17, 0.20),
                Color::srgb(0.80, 0.65, 0.30),
            ),
            NpcRole::Concierge => (
                Color::srgb(0.30, 0.36, 0.52),
                Color::srgb(0.20, 0.22, 0.28),
                Color::srgb(0.95, 0.82, 0.35),
            ),
            NpcRole::Housekeeping => (
                Color::srgb(0.80, 0.76, 0.66),
                Color::srgb(0.55, 0.52, 0.48),
                Color::srgb(0.92, 0.92, 0.92),
            ),
            NpcRole::Chef => (
                Color::srgb(0.95, 0.95, 0.95),
                Color::srgb(0.22, 0.22, 0.24),
                Color::srgb(0.90, 0.85, 0.75),
            ),
            NpcRole::SecurityGuard => (
                Color::srgb(0.20, 0.26, 0.38),
                Color::srgb(0.14, 0.17, 0.24),
                Color::srgb(0.70, 0.72, 0.76),
            ),
            NpcRole::Manager => (
                Color::srgb(0.18, 0.16, 0.15),
                Color::srgb(0.14, 0.13, 0.12),
                Color::srgb(0.85, 0.70, 0.35),
            ),
            NpcRole::Target => (
                Color::srgb(0.93, 0.90, 0.84),
                Color::srgb(0.24, 0.22, 0.28),
                Color::srgb(0.85, 0.30, 0.35),
            ),
        }
    }
}

/// Identity data shown when the player looks at an NPC.
#[derive(Component, Debug, Clone)]
pub struct NpcIdentity {
    pub name: &'static str,
    pub role: NpcRole,
    pub zone: ZoneId,
}

/// Marks the NPC's limbs for procedural animation.
/// A navigation waypoint an NPC walks to.
#[derive(Debug, Clone)]
pub struct Waypoint {
    pub position: Vec3,
    /// How long the NPC idles at this waypoint before moving on.
    pub wait: f32,
    /// Label used by the HUD when the NPC arrives.
    pub activity: &'static str,
}

/// Plugin gathering all NPC systems.
pub struct NpcPlugin;

impl Plugin for NpcPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn::spawn_npcs)
            .add_systems(
                FixedUpdate,
                (
                    schedule::advance_routines,
                    navigation::steer_npcs,
                    navigation::npc_collisions.after(navigation::steer_npcs),
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (navigation::animate_npcs, spawn::update_badges, brain::npc_tick_debug),
            );
    }
}

/// Query helper: NPCs are entities with [`Npc`], [`Brain`] and [`Transform`].
pub type NpcQuery<'w, 's> = Query<'w, 's, (&'static Transform, &'static Brain, &'static NpcIdentity), With<Npc>>;
