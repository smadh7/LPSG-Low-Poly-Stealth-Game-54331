//! Stage 1 acceptance tests.
//!
//! These run headless (no GPU): they exercise the gameplay systems through a
//! minimal Bevy app so movement, collision, NPC routines and interaction are
//! verified by machinery, not by eye.

use bevy::{prelude::*, time::TimeUpdateStrategy};
use hitman_lowpoly::core::Circle2;
use hitman_lowpoly::interaction::actions::NoticeFeed;
use hitman_lowpoly::interaction::{FocusTarget, InteractAction, InteractKind, Interactable};
use hitman_lowpoly::npc::brain::{Brain, BrainState};
use hitman_lowpoly::npc::schedule::RoutineStep;
use hitman_lowpoly::player::camera::CameraRig;
use hitman_lowpoly::player::character::{ActorBody, PlayerAvatar, PlayerVisual};
use hitman_lowpoly::player::PlayerIntent;
use hitman_lowpoly::player_state::PlayerState;
use hitman_lowpoly::world::builder::WorldCollision;
use hitman_lowpoly::world::layout::resort_layout;

/// Read the player's transform without keeping a world borrow alive.
fn player_transform(app: &mut App) -> Vec3 {
    let world = app.world_mut();
    let mut query = world.query_filtered::<&Transform, With<PlayerAvatar>>();
    let transform = query.single(world).expect("player exists");
    transform.translation
}

/// Read the player's physics body state.
fn player_body(app: &mut App) -> ActorBody {
    let world = app.world_mut();
    let mut query = world.query_filtered::<&ActorBody, With<PlayerAvatar>>();
    query.single(world).expect("player exists").clone()
}

/// Run the physics schedule `frames` times with a fixed 1/60 s step.
///
/// The headless app does not advance real time, so driving `FixedUpdate`
/// directly makes the simulation deterministic and fast.
fn step(app: &mut App, frames: usize) {
    // Bevy's own test recipe: drive real time manually so each `update()` is
    // exactly one fixed timestep. Deterministic and instant.
    for _ in 0..frames {
        app.update();
    }
}

/// Build a headless app with just the resources the physics needs.
///
/// The player's visual parts are included (invisibly, no meshes needed) so that
/// the same code paths as the real game are exercised.
fn physics_app(spawn: Vec3) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .insert_resource(PendingPlayerSpawn { position: spawn })
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f64(1.0 / 60.0),
        ));
    app.add_systems(Startup, spawn_test_player);
    app.update();
    app.init_resource::<PlayerIntent>();
    app.init_resource::<CameraRig>();
    app
}

#[derive(Resource, Clone, Copy)]
struct PendingPlayerSpawn {
    position: Vec3,
}

/// Startup system that mirrors `player::spawn::spawn_player` without visuals.
fn spawn_test_player(mut commands: Commands, spawn: Res<PendingPlayerSpawn>) {
    let layout = resort_layout();
    commands.insert_resource(WorldCollision::from_layout(&layout));
    let player = commands
        .spawn((
            PlayerAvatar,
            ActorBody::default(),
            Transform::from_translation(spawn.position),
        ))
        .id();
    // A couple of inert body parts, as the real player has.
    for offset in [Vec3::Y, Vec3::Y * 0.5] {
        commands.spawn((
            hitman_lowpoly::player::character::BodyPart {
                owner: player,
                offset,
                limb: None,
            },
            PlayerVisual,
            Transform::from_translation(spawn.position + offset),
        ));
    }
}

#[test]
fn player_stands_on_the_floor_and_does_not_sink() {
    let layout = resort_layout();
    let spawn = layout.player_spawn;
    let mut app = physics_app(spawn);

    app.add_systems(FixedUpdate, hitman_lowpoly::player::movement::player_physics);
    step(&mut app, 120);

    let transform = player_transform(&mut app);
    let body = player_body(&mut app);
    assert!(body.grounded, "player should be grounded after settling");
    assert!(
        transform.y.abs() < 0.25,
        "expected to stand near y=0, got {}",
        transform.y
    );
    assert!(body.ground_y.abs() < 0.25);
}

#[test]
fn player_walks_forward_and_is_blocked_by_walls() {
    // Open marble plaza in front of the lobby, clear of the garden hedges.
    let mut app = physics_app(Vec3::new(0.0, 0.0, -17.5));
    app.add_systems(FixedUpdate, hitman_lowpoly::player::movement::player_physics);

    let start = player_transform(&mut app);

    // Keep pushing forward: the intent is re-applied every simulated step.
    for _ in 0..120 {
        app.world_mut().resource_mut::<PlayerIntent>().direction = Vec3::Z;
        step(&mut app, 1);
    }

    let end = player_transform(&mut app);

    assert!(
        end.z > start.z + 3.0,
        "player should walk forward into the lobby: start={start:?} end={end:?}"
    );
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
        step(&mut app, 1);
    }
    let transform = player_transform(&mut app);
    assert!(
        transform.z < -14.0,
        "the player must be stopped by the wall, ended at z={}",
        transform.z
    );
}

#[test]
fn player_can_climb_the_ramp_to_the_restaurant() {
    // Start just south of the restaurant's outside stair and walk north.
    let mut app = physics_app(Vec3::new(34.0, 0.0, -24.0));
    app.add_systems(FixedUpdate, hitman_lowpoly::player::movement::player_physics);
    for _ in 0..600 {
        app.world_mut().resource_mut::<PlayerIntent>().direction = Vec3::Z;
        step(&mut app, 1);
    }
    let transform = player_transform(&mut app);
    assert!(
        transform.y > 1.5,
        "the player should climb onto the raised restaurant deck, pos={transform:?}"
    );
}

#[test]
fn player_can_descend_into_the_service_corridor() {
    // Walk south from the gardens onto the service staircase and down to the
    // corridor floor (-3.4 m): this proves negative-height spaces are walkable.
    let mut app = physics_app(Vec3::new(-33.0, 0.0, 24.0));
    app.add_systems(FixedUpdate, hitman_lowpoly::player::movement::player_physics);
    let mut lowest = f32::MAX;
    for _ in 0..900 {
        app.world_mut().resource_mut::<PlayerIntent>().direction = Vec3::NEG_Z;
        step(&mut app, 1);
        lowest = lowest.min(player_transform(&mut app).y);
    }
    assert!(
        lowest < -3.0,
        "player should reach the service corridor floor, lowest y={lowest}"
    );
}

#[test]
fn player_can_climb_the_penthouse_stair() {
    // The restricted penthouse is 29.8 m up a 72-step staircase.
    let mut app = physics_app(Vec3::new(-30.5, 0.0, 24.0));
    app.add_systems(FixedUpdate, hitman_lowpoly::player::movement::player_physics);
    // Only walk as far as the terrace: the test asserts the climb, and the
    // terrace's north edge is open, so an unbounded walk would fall off it.
    let mut highest: f32 = f32::MIN;
    for _ in 0..900 {
        app.world_mut().resource_mut::<PlayerIntent>().direction = Vec3::Z;
        step(&mut app, 1);
        highest = highest.max(player_transform(&mut app).y);
    }
    assert!(
        highest > 25.0,
        "player should climb towards the penthouse terrace, highest y={highest}"
    );
}

#[test]
fn jumping_leaves_the_ground_and_lands_again() {
    let layout = resort_layout();
    let mut app = physics_app(layout.player_spawn);
    app.add_systems(FixedUpdate, hitman_lowpoly::player::movement::player_physics);
    step(&mut app, 30);
    app.world_mut().resource_mut::<PlayerIntent>().wants_jump = true;
    step(&mut app, 1);

    let airborne = player_body(&mut app);
    assert!(!airborne.grounded, "player should be airborne right after jumping");
    let y_after_jump = player_transform(&mut app).y;
    assert!(y_after_jump > 0.05, "player should gain height, y={y_after_jump}");

    step(&mut app, 150);
    let landed = player_body(&mut app);
    assert!(landed.grounded, "player should land again");
    assert!(player_transform(&mut app).y.abs() < 0.3);
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
fn every_interactable_sits_on_or_near_a_surface() {
    // Props must be reachable: each authored interactable has to sit on (or
    // just above) a walkable surface rather than floating in mid-air.
    let layout = resort_layout();
    // Use every solid (including low furniture such as the reception desk,
    // which is deliberately excluded from the walk-blocking collision set).
    let blocks: Vec<hitman_lowpoly::core::Block2> = layout
        .solids
        .iter()
        .map(|s| hitman_lowpoly::core::Block2::new(s.center, s.size))
        .collect();
    let checks = [
        // (interactable position, description)
        (Vec3::new(-4.0, 1.35, 9.0), "guest register"),
        (Vec3::new(-8.0, 0.95, -20.4), "garden bench"),
        (Vec3::new(36.0, 3.4, -8.0), "restaurant menu"),
        (Vec3::new(-46.3, -1.2, 13.0), "staff locker"),
        (Vec3::new(6.2, 31.4, 44.2), "penthouse safe"),
        (Vec3::new(12.0, 30.9, 33.0), "hot tub dataslate"),
    ];
    for (position, what) in checks {
        let xz = Vec2::new(position.x, position.z);
        let mut surface: Option<f32> = None;
        for block in &blocks {
            if block.contains_xz(xz, 0.9) && block.top() <= position.y + 0.05 {
                surface = Some(surface.map_or(block.top(), |s: f32| s.max(block.top())));
            }
        }
        match surface {
            Some(top) => assert!(
                position.y - top < 2.0,
                "{what} floats {:.2} m above its surface",
                position.y - top
            ),
            None => assert!(
                position.y < 1.2,
                "{what} has no surface beneath it and is above ground level"
            ),
        }
    }
}
