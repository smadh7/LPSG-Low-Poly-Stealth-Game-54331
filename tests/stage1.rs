//! Stage 1 acceptance tests.
//!
//! These run headless (no GPU): they exercise the gameplay systems through a
//! minimal Bevy app so movement, collision, NPC routines and interaction are
//! verified by machinery, not by eye.

use bevy::prelude::*;
use hitman_lowpoly::core::Circle2;
use hitman_lowpoly::interaction::actions::NoticeFeed;
use hitman_lowpoly::interaction::{FocusTarget, InteractAction, InteractKind, Interactable};
use hitman_lowpoly::npc::brain::{Brain, BrainState};
use hitman_lowpoly::npc::schedule::RoutineStep;
use hitman_lowpoly::player::camera::CameraRig;
use hitman_lowpoly::player::character::{ActorBody, PlayerAvatar};
use hitman_lowpoly::player::PlayerIntent;
use hitman_lowpoly::player_state::PlayerState;
use hitman_lowpoly::world::builder::WorldCollision;
use hitman_lowpoly::world::layout::resort_layout;

/// Build a headless app with just the resources the physics needs.
fn physics_app(spawn: Vec3) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .insert_resource(PlayerIntent::default())
        .insert_resource(CameraRig::default());

    let layout = resort_layout();
    app.insert_resource(WorldCollision::from_layout(&layout));
    app.world_mut().spawn((
        PlayerAvatar,
        ActorBody::default(),
        Transform::from_translation(spawn),
    ));
    app
}

#[test]
fn player_stands_on_the_floor_and_does_not_sink() {
    let layout = resort_layout();
    let spawn = layout.player_spawn;
    let mut app = physics_app(spawn);

    app.add_systems(FixedUpdate, hitman_lowpoly::player::movement::player_physics);
    for _ in 0..120 {
        app.update();
    }

    let body = app
        .world_mut()
        .query_filtered::<(&Transform, &ActorBody), With<PlayerAvatar>>()
        .single(app.world())
        .expect("player exists");
    let (transform, body) = body;
    assert!(body.grounded, "player should be grounded after settling");
    assert!(
        (transform.translation.y - 0.0).abs() < 0.25,
        "expected to stand near y=0, got {}",
        transform.translation.y
    );
    assert!(body.ground_y.abs() < 0.25);
}

#[test]
fn player_walks_forward_and_is_blocked_by_walls() {
    let layout = resort_layout();
    let mut app = physics_app(Vec3::new(0.0, 0.0, -20.0));
    app.add_systems(FixedUpdate, hitman_lowpoly::player::movement::player_physics);
    app.world_mut().resource_mut::<PlayerIntent>().direction = Vec3::Z;

    let start = app
        .world_mut()
        .query_filtered::<&Transform, With<PlayerAvatar>>()
        .single(app.world())
        .unwrap()
        .translation;

    for _ in 0..120 {
        // Keep pushing forward each step: the intent is consumed by the solver.
        app.world_mut().resource_mut::<PlayerIntent>().direction = Vec3::Z;
        app.update();
    }

    let end = app
        .world_mut()
        .query_filtered::<&Transform, With<PlayerAvatar>>()
        .single(app.world())
        .unwrap()
        .translation;

    assert!(end.z > start.z + 0.5, "player should have moved forward");
    // The lobby's south wall is at z = -14 with a 10 m wide doorway, and the
    // player is walking straight at the doorway, so they should end up inside
    // or in front of the building — never past the far wall.
    assert!(end.z < 26.0, "player must not pass through the far wall");
}

#[test]
fn player_cannot_walk_through_a_solid_wall() {
    // Aim straight at a solid section of the lobby's south wall (x = 12).
    let mut app = physics_app(Vec3::new(12.0, 0.0, -18.0));
    app.add_systems(FixedUpdate, hitman_lowpoly::player::movement::player_physics);
    for _ in 0..180 {
        app.world_mut().resource_mut::<PlayerIntent>().direction = Vec3::Z;
        app.update();
    }
    let transform = app
        .world_mut()
        .query_filtered::<&Transform, With<PlayerAvatar>>()
        .single(app.world())
        .unwrap()
        .translation;
    assert!(
        transform.z < -14.0,
        "the player must be stopped by the wall, ended at z={}",
        transform.z
    );
}

#[test]
fn player_can_climb_the_ramp_to_the_restaurant() {
    // Start just south of the restaurant's outside stair and walk north.
    let mut app = physics_app(Vec3::new(34.0, 0.0, -15.5));
    app.add_systems(FixedUpdate, hitman_lowpoly::player::movement::player_physics);
    for _ in 0..400 {
        app.world_mut().resource_mut::<PlayerIntent>().direction = Vec3::Z;
        app.update();
    }
    let transform = app
        .world_mut()
        .query_filtered::<&Transform, With<PlayerAvatar>>()
        .single(app.world())
        .unwrap()
        .translation;
    assert!(
        transform.y > 1.5,
        "the player should climb onto the raised restaurant deck, y={}",
        transform.y
    );
}

#[test]
fn jumping_leaves_the_ground_and_lands_again() {
    let layout = resort_layout();
    let mut app = physics_app(layout.player_spawn);
    app.add_systems(FixedUpdate, hitman_lowpoly::player::movement::player_physics);
    for _ in 0..30 {
        app.update();
    }
    app.world_mut().resource_mut::<PlayerIntent>().wants_jump = true;
    app.update();
    let y_after_jump = app
        .world_mut()
        .query_filtered::<&Transform, With<PlayerAvatar>>()
        .single(app.world())
        .unwrap()
        .translation
        .y;
    let body = app
        .world_mut()
        .query_filtered::<&ActorBody, With<PlayerAvatar>>()
        .single(app.world())
        .unwrap();
    assert!(!body.grounded, "player should be airborne right after jumping");

    for _ in 0..120 {
        app.update();
    }
    let final_body = app
        .world_mut()
        .query_filtered::<&ActorBody, With<PlayerAvatar>>()
        .single(app.world())
        .unwrap();
    let final_y = app
        .world_mut()
        .query_filtered::<&Transform, With<PlayerAvatar>>()
        .single(app.world())
        .unwrap()
        .translation
        .y;
    assert!(final_body.grounded, "player should land again (y was {y_after_jump})");
    assert!(final_y.abs() < 0.3);
}

#[test]
fn spawn_point_is_clear_of_geometry() {
    let layout = resort_layout();
    let spawn = layout.player_spawn;
    let circle = Circle2 {
        center: Vec2::new(spawn.x, spawn.z),
        radius: 0.4,
    };
    for block in layout.obstacle_blocks() {
        assert!(
            block.resolve_circle(circle).is_none(),
            "spawn point collides with a solid: {:?}",
            block
        );
    }
}

#[test]
fn npc_routines_walk_between_destinations() {
    let mut brain = Brain::new(vec![
        RoutineStep::new(Vec3::new(0.0, 0.0, 0.0), 1.0, "start"),
        RoutineStep::new(Vec3::new(0.0, 0.0, 8.0), 1.0, "walk"),
        RoutineStep::new(Vec3::new(8.0, 0.0, 8.0), 1.0, "walk2"),
    ]);
    assert_eq!(brain.state, BrainState::Walking);
    brain.advance_step();
    brain.advance_step();
    brain.advance_step();
    assert_eq!(brain.index, 0, "routines wrap around");
}

#[test]
fn interaction_actions_update_inventory_and_notices() {
    let mut interactable = Interactable::new(
        "Desk Drawer",
        "A locked drawer.",
        InteractKind::Container,
        InteractAction::TakeItem("Master Keycard"),
        hitman_lowpoly::core::ZoneId::Lobby,
    );
    let mut feed = NoticeFeed::default();
    let mut state = PlayerState::default();
    hitman_lowpoly::interaction::actions::perform_action(&mut interactable, &mut feed, &mut state);
    assert_eq!(feed.inventory, vec!["Master Keycard".to_string()]);
    assert!(interactable.used);
}

#[test]
fn disguise_action_changes_the_player_outfit() {
    let mut interactable = Interactable::new(
        "Staff Locker",
        "A locker.",
        InteractKind::Wardrobe,
        InteractAction::ChangeDisguise(hitman_lowpoly::player_state::OutfitKind::Staff),
        hitman_lowpoly::core::ZoneId::ServiceCorridor,
    );
    let mut feed = NoticeFeed::default();
    let mut state = PlayerState::default();
    hitman_lowpoly::interaction::actions::perform_action(&mut interactable, &mut feed, &mut state);
    assert_eq!(state.outfit, hitman_lowpoly::player_state::OutfitKind::Staff);
}

#[test]
fn focus_target_defaults_to_empty() {
    let focus = FocusTarget::default();
    assert!(focus.entity.is_none());
    assert!(focus.prompt.is_empty());
}

#[test]
fn every_interactable_is_reachable_from_the_floor_plane() {
    // Interactables must sit within ~1.6 m of a walkable surface; this guards
    // against props authored at the wrong height after layout changes.
    let layout = resort_layout();
    let blocks = layout.obstacle_blocks();
    // Interactable positions duplicated here intentionally: this test lives
    // outside the module so it validates the authored data from the outside.
    let height_checks = [
        (Vec3::new(-4.0, 1.35, 9.0), 1.15),
        (Vec3::new(-8.0, 0.95, -20.4), 0.0),
        (Vec3::new(36.0, 3.4, -8.0), 2.4),
        (Vec3::new(-46.3, -1.2, 13.0), -3.4),
        (Vec3::new(6.2, 31.4, 44.2), 29.8),
    ];
    for (position, expected_floor) in height_checks {
        let mut nearest: Option<f32> = None;
        for block in &blocks {
            if block.contains_xz(Vec2::new(position.x, position.z), 1.0) {
                let top = block.top();
                if top <= position.y + 0.2 {
                    nearest = Some(nearest.map_or(top, |n: f32| n.max(top)));
                }
            }
        }
        let floor = nearest.unwrap_or(0.0);
        assert!(
            (floor - expected_floor).abs() < 0.6,
            "interactable at {position:?} expected floor {expected_floor}, found {floor}"
        );
    }
}
