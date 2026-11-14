//! NPC routines: each NPC follows a looping list of waypoints, idling at each
//! destination for a while before moving on.
//!
//! Stage 2 extends this with time-of-day gating (`start_time`), so an NPC's
//! routine can specify *when* a step is available.

use super::brain::{Brain, BrainState};
use super::Waypoint;
use bevy::prelude::*;

/// A single step in an NPC's daily routine.
#[derive(Debug, Clone)]
pub struct RoutineStep {
    pub destination: Waypoint,
    /// Normalised time of day (0..1) this step becomes available.
    pub start_time: f32,
}

impl RoutineStep {
    pub fn new(position: Vec3, wait: f32, activity: &'static str) -> Self {
        Self {
            destination: Waypoint {
                position,
                wait,
                activity,
            },
            start_time: 0.0,
        }
    }

    pub fn at(mut self, time: f32) -> Self {
        self.start_time = time;
        self
    }
}

/// Advance NPC routine timers and hand out the next destination.
pub fn advance_routines(time: Res<Time>, mut brains: Query<&mut Brain>) {
    let dt = time.delta_secs();
    for mut brain in &mut brains {
        if brain.routine.is_empty() {
            continue;
        }

        // Idle / walk timers.
        match brain.state {
            BrainState::Idle | BrainState::Working | BrainState::Talking => {
                brain.wait_timer -= dt;
                if brain.wait_timer <= 0.0 && !brain.paused {
                    brain.advance_step();
                }
            }
            BrainState::Walking => {
                if brain.arrived {
                    brain.arrived = false;
                    let wait = brain
                        .current_step()
                        .map(|s| s.destination.wait)
                        .unwrap_or(2.0);
                    brain.wait_timer = wait.max(0.5);
                    brain.state = if wait >= 4.0 {
                        BrainState::Working
                    } else {
                        BrainState::Idle
                    };
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::npc::brain::{Brain, BrainState};

    #[test]
    fn routine_advances_after_wait() {
        let routine = vec![
            RoutineStep::new(Vec3::ZERO, 1.0, "idle"),
            RoutineStep::new(Vec3::new(0.0, 0.0, 5.0), 1.0, "walk"),
        ];
        let mut brain = Brain::new(routine);
        // Start heading to the first waypoint.
        brain.wait_timer = 0.0;
        brain.advance_step();
        assert_eq!(brain.index, 1);
        assert_eq!(brain.state, BrainState::Walking);
        brain.advance_step();
        assert_eq!(brain.index, 0, "routines must loop");
    }
}
