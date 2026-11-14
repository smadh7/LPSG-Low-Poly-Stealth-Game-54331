//! Execution of interaction actions and the notification feed.

use super::{FocusTarget, InteractAction, Interactable};
use crate::player_state::PlayerState;
use bevy::prelude::*;

/// A transient message shown at the bottom of the screen.
#[derive(Debug, Clone)]
pub struct Notice {
    pub text: String,
    pub remaining: f32,
}

impl Notice {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            remaining: 4.5,
        }
    }
}

/// Queue of notices shown by the HUD (newest first).
#[derive(Resource, Debug, Default)]
pub struct NoticeFeed {
    pub notices: Vec<Notice>,
    pub inventory: Vec<String>,
    pub clues: Vec<String>,
}

impl NoticeFeed {
    pub fn push(&mut self, text: impl Into<String>) {
        self.notices.insert(0, Notice::new(text));
        self.notices.truncate(4);
    }

    pub fn tick(&mut self, dt: f32) {
        for notice in &mut self.notices {
            notice.remaining -= dt;
        }
        self.notices.retain(|n| n.remaining > 0.0);
    }

    pub fn add_item(&mut self, item: &str) {
        if !self.inventory.iter().any(|existing| existing == item) {
            self.inventory.push(item.to_string());
        }
    }

    pub fn add_clue(&mut self, clue: &str) {
        if !self.clues.iter().any(|existing| existing == clue) {
            self.clues.push(clue.to_string());
        }
    }
}

/// Read the interact key and run the focused action.
pub fn handle_interaction(
    keys: Res<ButtonInput<KeyCode>>,
    mut focus: ResMut<FocusTarget>,
    mut interactables: Query<&mut Interactable>,
    mut feed: ResMut<NoticeFeed>,
    mut player_state: ResMut<PlayerState>,
) {
    if !keys.just_pressed(KeyCode::KeyE) {
        return;
    }
    let Some(entity) = focus.entity else {
        return;
    };
    let Ok(mut interactable) = interactables.get_mut(entity) else {
        return;
    };
    if interactable.used && matches!(interactable.action, InteractAction::TakeItem(_)) {
        feed.push(format!("{} is empty.", interactable.label));
        return;
    }

    perform_action(&mut interactable, &mut feed, &mut player_state);
    focus.exhausted = interactable.used;
}

/// Apply an action's effects. Kept public so Stage 2 systems can trigger
/// actions programmatically.
pub fn perform_action(
    interactable: &mut Interactable,
    feed: &mut NoticeFeed,
    player_state: &mut PlayerState,
) {
    match &interactable.action {
        InteractAction::Message(text) => {
            feed.push(text.clone());
            interactable.used = true;
        }
        InteractAction::OpenDoor => {
            feed.push(format!("{} opened.", interactable.label));
        }
        InteractAction::TakeItem(item) => {
            feed.add_item(item);
            feed.push(format!("Acquired: {item}"));
            interactable.used = true;
        }
        InteractAction::ToggleSit => {
            feed.push("You take a seat for a moment.".to_string());
        }
        InteractAction::ChangeDisguise(outfit) => {
            player_state.outfit = *outfit;
            feed.push(format!("Disguise changed: {}", outfit.display_name()));
            interactable.used = true;
        }
        InteractAction::ToggleDevice => {
            feed.push(format!("{} activated.", interactable.label));
            interactable.used = true;
        }
    }
}

/// Age out notices every frame.
pub fn tick_notices(time: Res<Time>, mut feed: ResMut<NoticeFeed>) {
    feed.tick(time.delta_secs());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ZoneId;
    use crate::interaction::InteractKind;

    #[test]
    fn taking_an_item_fills_inventory_once() {
        let mut interactable = Interactable::new(
            "Desk Drawer",
            "A locked drawer.",
            InteractKind::Container,
            InteractAction::TakeItem("Master Keycard"),
            ZoneId::Lobby,
        );
        let mut feed = NoticeFeed::default();
        let mut state = PlayerState::default();
        perform_action(&mut interactable, &mut feed, &mut state);
        assert!(interactable.used);
        assert_eq!(feed.inventory, vec!["Master Keycard".to_string()]);
        assert!(!feed.notices.is_empty());
    }

    #[test]
    fn notices_expire() {
        let mut feed = NoticeFeed::default();
        feed.push("hello");
        feed.tick(10.0);
        assert!(feed.notices.is_empty());
    }

    #[test]
    fn duplicate_items_are_not_added_twice() {
        let mut feed = NoticeFeed::default();
        feed.add_item("Key");
        feed.add_item("Key");
        assert_eq!(feed.inventory.len(), 1);
    }
}
