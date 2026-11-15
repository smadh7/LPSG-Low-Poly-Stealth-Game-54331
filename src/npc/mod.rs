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

    /// Outfit used for this role.
    ///
    /// Staff wear the resort uniform (gold-accented linen for front of house,
    /// navy for security); guests are dressed in plausible holiday clothing and
    /// each one gets a different skin tone so the crowd never looks cloned.
    pub fn outfit(self, variety: u8) -> crate::player::character::Outfit {
        use crate::player::character::{GarmentColor as G, Outfit, SkinTone};
        let skin = match variety % 6 {
            0 => SkinTone::Porcelain,
            1 => SkinTone::Fair,
            2 => SkinTone::Olive,
            3 => SkinTone::Tan,
            4 => SkinTone::Brown,
            _ => SkinTone::Deep,
        };
        let hair = match (variety / 6) % 5 {
            0 => G::HairBrown,
            1 => G::HairBlack,
            2 => G::HairBlonde,
            3 => G::HairGinger,
            _ => G::HairGrey,
        };
        let (shirt, trousers, shoes) = match self {
            NpcRole::Guest => {
                let palettes = [
                    (G::Linen, G::Sand, G::White),
                    (G::Teal, G::Cream, G::Sand),
                    (G::Cream, G::Olive, G::HairBrown),
                    (G::Burgundy, G::Charcoal, G::Black),
                    (G::Sand, G::White, G::Black),
                    (G::Olive, G::Cream, G::Black),
                ];
                palettes[(variety as usize) % palettes.len()]
            }
            NpcRole::Waiter | NpcRole::Bartender => (G::White, G::Black, G::Black),
            NpcRole::Chef => (G::White, G::Charcoal, G::Black),
            NpcRole::Concierge => (G::HotelGold, G::Navy, G::Black),
            NpcRole::Housekeeping => (G::Cream, G::Sand, G::White),
            NpcRole::Manager => (G::Navy, G::Charcoal, G::Black),
            NpcRole::SecurityGuard => (G::SecurityBlue, G::Black, G::Black),
            NpcRole::Target => (G::White, G::Charcoal, G::Black),
        };
        Outfit {
            skin,
            shirt,
            trousers,
            shoes,
            hair,
        }
    }

    /// Accent colour used for the role badge above an NPC's head.
    pub fn accent(self) -> Color {
        match self {
            NpcRole::Guest => Color::srgb(0.55, 0.72, 0.92),
            NpcRole::Waiter | NpcRole::Bartender => Color::srgb(0.92, 0.92, 0.92),
            NpcRole::Chef => Color::srgb(0.95, 0.95, 0.95),
            NpcRole::Concierge => Color::srgb(0.88, 0.72, 0.32),
            NpcRole::Housekeeping => Color::srgb(0.85, 0.78, 0.62),
            NpcRole::Manager => Color::srgb(0.35, 0.45, 0.70),
            NpcRole::SecurityGuard => Color::srgb(0.90, 0.35, 0.32),
            NpcRole::Target => Color::srgb(0.85, 0.30, 0.55),
        }
    }
}

/// Query helper: NPCs are entities with [`Npc`], [`Brain`] and [`Transform`].
pub type NpcQuery<'w, 's> = Query<'w, 's, (&'static Transform, &'static Brain, &'static NpcIdentity), With<Npc>>;
