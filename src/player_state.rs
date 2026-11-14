//! Shared player state that multiple systems read (HUD, accessibility, NPC
//! recognition).

use bevy::prelude::*;

/// Which general role the player's current outfit belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutfitKind {
    /// Casual linen clothes — a tourist.
    #[default]
    Tourist,
    /// Staff uniform.
    Staff,
    /// Security uniform.
    Security,
}

impl OutfitKind {
    pub fn display_name(self) -> &'static str {
        match self {
            OutfitKind::Tourist => "Tourist",
            OutfitKind::Staff => "Resort Staff",
            OutfitKind::Security => "Security",
        }
    }

    /// All outfit kinds available to the player.
    pub fn all() -> [OutfitKind; 3] {
        [OutfitKind::Tourist, OutfitKind::Staff, OutfitKind::Security]
    }

    /// A short colour hint used by the HUD.
    pub fn color(self) -> Color {
        match self {
            OutfitKind::Tourist => Color::srgb(0.93, 0.93, 0.88),
            OutfitKind::Staff => Color::srgb(0.85, 0.72, 0.48),
            OutfitKind::Security => Color::srgb(0.35, 0.45, 0.62),
        }
    }
}

/// Global player state shared across gameplay systems.
#[derive(Resource, Debug)]
pub struct PlayerState {
    /// Current outfit. Stage 1 always starts as a tourist.
    pub outfit: OutfitKind,
    /// Whether the player is currently detected / compromised.
    pub detected: bool,
    /// Distance the player has travelled, used by the mission summary.
    pub distance_travelled: f32,
    /// Total elapsed play time.
    pub play_time: f32,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            outfit: OutfitKind::Tourist,
            detected: false,
            distance_travelled: 0.0,
            play_time: 0.0,
        }
    }
}
