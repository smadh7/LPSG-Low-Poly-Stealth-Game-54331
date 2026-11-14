//! NPC brain: routine progression and (Stage 2) awareness state.
//!
//! The brain owns *what the NPC wants to do*. The navigation module owns *how*
//! it gets there, which keeps perception and movement independent and testable.

use super::schedule::RoutineStep;
use bevy::prelude::*;

/// High level behaviour state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrainState {
    /// Standing at a waypoint, looking around.
    Idle,
    /// Performing the waypoint's activity (chatting, serving, patrolling).
    Working,
    /// Walking to the current waypoint.
    Walking,
    /// Stopped to chat with another NPC or the player.
    Talking,
    /// Stage 2: investigating a suspicious sound or sighting.
    Investigating,
    /// Stage 2: actively searching for the player.
    Alerted,
    /// Stage 2: escorting the player out / calling for backup.
    Hostile,
}

impl BrainState {
    pub fn label(self) -> &'static str {
        match self {
            BrainState::Idle => "idle",
            BrainState::Working => "working",
            BrainState::Walking => "walking",
            BrainState::Talking => "talking",
            BrainState::Investigating => "investigating",
            BrainState::Alerted => "alerted",
            BrainState::Hostile => "hostile",
        }
    }
}

/// The NPC's decision making state.
#[derive(Component, Debug)]
pub struct Brain {
    pub routine: Vec<RoutineStep>,
    pub index: usize,
    pub state: BrainState,
    /// Seconds left at the current waypoint.
    pub wait_timer: f32,
    /// Set by navigation when the current waypoint has been reached.
    pub arrived: bool,
    /// Facing direction the NPC should use while idling (radians).
    pub idle_yaw: f32,
    /// Freezes the routine (used during dialogue / scripted moments).
    pub paused: bool,
}

impl Brain {
    pub fn new(routine: Vec<RoutineStep>) -> Self {
        let idle_yaw = routine
            .first()
            .map(|s| s.destination.position.x.atan2(s.destination.position.z))
            .unwrap_or(0.0);
        Self {
            routine,
            index: 0,
            state: BrainState::Walking,
            wait_timer: 0.0,
            arrived: false,
            idle_yaw,
            paused: false,
        }
    }

    pub fn current_step(&self) -> Option<&RoutineStep> {
        self.routine.get(self.index)
    }

    /// Move to the next routine step (wrapping around).
    pub fn advance_step(&mut self) {
        if self.routine.is_empty() {
            return;
        }
        self.index = (self.index + 1) % self.routine.len();
        self.state = BrainState::Walking;
        self.wait_timer = 0.0;
        self.arrived = false;
    }
}

/// Lightweight log used while developing Stage 2 awareness.
pub fn npc_tick_debug(brains: Query<(&Brain, &Name)>) {
    // Intentionally empty in release builds: kept as an extension point so the
    // Stage 2 systems have a stable scheduling anchor.
    for (_brain, _name) in &brains {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::npc::schedule::RoutineStep;

    #[test]
    fn brain_loops_routines() {
        let ctx = vec![
            RoutineStep::new(Vec3::ZERO, 1.0, "a"),
            RoutineStep::new(Vec3::Z, 1.0, "b"),
        ];
        let mut brain = Brain::new(ctx);
        assert_eq!(brain.index, 0);
        brain.advance_step();
        assert_eq!(brain.index, 1);
        brain.advance_step();
        assert_eq!(brain.index, 0);
    }

    #[test]
    fn empty_routines_are_safe() {
        let mut brain = Brain::new(vec![]);
        brain.advance_step();
        assert_eq!(brain.index, 0);
        assert!(brain.current_step().is_none());
    }
}
