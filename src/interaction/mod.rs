//! Contextual interaction system.
//!
//! Design goals:
//! * **Data driven** — anything interactable carries an [`Interactable`]
//!   component describing its prompt text and the action it performs.
//! * **Player facing** — the player looks at things with the camera; the
//!   nearest interactable inside a cone + range becomes the current target.
//! * **Extensible** — actions are an enum. Adding a new verb is a matter of
//!   adding a variant and handling it in [`actions::perform_action`].
//!
//! Stage 2 builds on this by adding actions such as *Change Disguise*,
//! *Hack Camera* and *Take Evidence* — no new plumbing required.

pub mod actions;
pub mod prompts;

use bevy::prelude::*;

use crate::core::ZoneId;

/// What kind of object the player is looking at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractKind {
    /// Information only: the player can read / inspect it.
    Inspect,
    /// A door or gate that can be opened / passed through.
    Door,
    /// A container that yields an item.
    Container,
    /// Something that can be picked up.
    Pickup,
    /// A chair / sofa / bed.
    Seat,
    /// A drinking fountain, bar tap, coffee machine…
    Drink,
    /// A wardrobe / locker: source of disguises.
    Wardrobe,
    /// Stage 2: a camera or alarm panel.
    Device,
}

impl InteractKind {
    pub fn verb(self) -> &'static str {
        match self {
            InteractKind::Inspect => "Inspect",
            InteractKind::Door => "Open",
            InteractKind::Container => "Search",
            InteractKind::Pickup => "Pick up",
            InteractKind::Seat => "Sit",
            InteractKind::Drink => "Drink",
            InteractKind::Wardrobe => "Change",
            InteractKind::Device => "Use",
        }
    }
}

/// One possible action attached to an interactable.
#[derive(Debug, Clone, PartialEq)]
pub enum InteractAction {
    /// Show a line of text to the player.
    Message(String),
    /// Open a door (Stage 1 records the fact; collision gaps are authored).
    OpenDoor,
    /// Reveal an item by name.
    TakeItem(&'static str),
    /// Sit down / stand up.
    ToggleSit,
    /// Stage 2: put on the outfit stored in this wardrobe.
    ChangeDisguise(crate::player_state::OutfitKind),
    /// Stage 2: interact with a security device.
    ToggleDevice,
}

/// Component describing an interactable object.
#[derive(Component, Debug, Clone)]
pub struct Interactable {
    /// Short label shown above the prompt, e.g. "Reception Book".
    pub label: &'static str,
    /// Longer description shown in the prompt panel.
    pub description: &'static str,
    pub kind: InteractKind,
    pub action: InteractAction,
    /// Maximum interaction distance in metres.
    pub range: f32,
    /// Zone used for trespass / disguise hints.
    pub zone: ZoneId,
    /// Whether the object has already been used (single-use objects).
    pub used: bool,
}

impl Interactable {
    pub fn new(
        label: &'static str,
        description: &'static str,
        kind: InteractKind,
        action: InteractAction,
        zone: ZoneId,
    ) -> Self {
        Self {
            label,
            description,
            kind,
            action,
            range: 2.4,
            zone,
            used: false,
        }
    }

    pub fn with_range(mut self, range: f32) -> Self {
        self.range = range;
        self
    }

    /// The full prompt text shown to the player.
    pub fn prompt(&self) -> String {
        format!("[E] {} {}", self.kind.verb(), self.label)
    }
}

/// The interactable the player is currently focusing on.
#[derive(Resource, Debug, Default)]
pub struct FocusTarget {
    pub entity: Option<Entity>,
    pub distance: f32,
    pub prompt: String,
    pub label: String,
    pub description: String,
    /// True when the object has already been used and is inert.
    pub exhausted: bool,
}

/// Marker for interactable objects that have a physical mesh.
#[derive(Component)]
pub struct InteractableVisual;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_text_uses_verb_and_label() {
        let i = Interactable::new(
            "Reception Book",
            "A leather-bound guest register.",
            InteractKind::Inspect,
            InteractAction::Message("Room 214 is listed twice.".into()),
            ZoneId::Lobby,
        );
        assert_eq!(i.prompt(), "[E] Inspect Reception Book");
    }

    #[test]
    fn ranges_can_be_overridden() {
        let i = Interactable::new(
            "Vault",
            "",
            InteractKind::Container,
            InteractAction::TakeItem("Keycard"),
            ZoneId::SecurityWing,
        )
        .with_range(5.0);
        assert_eq!(i.range, 5.0);
    }
}

/// Plugin wiring the interaction systems together.
pub struct InteractionPlugin;

impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FocusTarget>()
            .init_resource::<actions::NoticeFeed>()
            .add_systems(Startup, prompts::spawn_interactables)
            .add_systems(
                Update,
                (
                    actions::tick_notices,
                    prompts::update_focus,
                    actions::handle_interaction.after(prompts::update_focus),
                ),
            );
    }
}
