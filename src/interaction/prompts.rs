//! Focus selection *and* the authored interactable objects of the resort.
//!
//! Works out which interactable the player is looking at and publishes it as
//! [`FocusTarget`]. Candidates are scored by `dist - alignment * weight` so
//! objects near the centre of the screen win.

use super::{InteractAction, InteractKind, Interactable};
use crate::core::ZoneId;
use crate::player::camera::{raycast_blocks, CameraRig};
use crate::player::character::PlayerAvatar;
use crate::world::builder::WorldCollision;
use bevy::prelude::*;

/// Maximum distance the player can focus on something.
pub const FOCUS_RANGE: f32 = 3.6;
/// Cosine of the half-angle of the focus cone.
pub const FOCUS_CONE: f32 = 0.45;

pub fn update_focus(
    rig: Res<CameraRig>,
    collision: Option<Res<WorldCollision>>,
    mut focus: ResMut<super::FocusTarget>,
    player: Query<&Transform, With<PlayerAvatar>>,
    interactables: Query<(Entity, &Transform, &Interactable)>,
) {
    let Ok(player_transform) = player.single() else {
        focus.entity = None;
        return;
    };
    // Look from the character's chest so low objects are reachable.
    let eye = player_transform.translation + Vec3::Y * 1.35;
    let forward = rig.look_direction();

    let mut best: Option<(Entity, f32, f32)> = None; // (entity, score, distance)

    for (entity, transform, interactable) in &interactables {
        let to_object = transform.translation - eye;
        let distance = to_object.length();
        let max_range = interactable.range.max(FOCUS_RANGE);
        if distance > max_range || distance < 0.001 {
            continue;
        }
        let dir = to_object / distance;
        let alignment = dir.dot(forward);
        if alignment < FOCUS_CONE {
            continue;
        }

        // Skip objects hidden behind walls.
        if let Some(collision) = collision.as_ref() {
            if raycast_blocks(eye, dir, (distance - 0.35).max(0.0), &collision.blocks).is_some() {
                continue;
            }
        }

        let score = distance - alignment * 2.5;
        if best.map_or(true, |(_, best_score, _)| score < best_score) {
            best = Some((entity, score, distance));
        }
    }

    match best {
        Some((entity, _, distance)) => {
            focus.entity = Some(entity);
            focus.distance = distance;
            if let Ok((_, _, interactable)) = interactables.get(entity) {
                focus.prompt = interactable.prompt();
                focus.label = interactable.label.to_string();
                focus.description = interactable.description.to_string();
                focus.exhausted = interactable.used;
            }
        }
        None => {
            focus.entity = None;
            focus.prompt.clear();
            focus.label.clear();
            focus.description.clear();
            focus.exhausted = false;
            focus.distance = 0.0;
        }
    }
}

/// Shorthand for spawn entries below.
fn spec(
    label: &'static str,
    description: &'static str,
    kind: InteractKind,
    action: InteractAction,
    zone: ZoneId,
) -> Interactable {
    Interactable::new(label, description, kind, action, zone)
}

/// Author every interactable object in the resort.
///
/// Positions are chosen to sit on top of existing furniture so the player has
/// obvious things to aim at while learning the interaction system.
pub fn spawn_interactables(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let marker_mesh = meshes.add(Mesh::from(Cuboid::new(0.26, 0.26, 0.26)));
    let warm = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.86, 0.42),
        emissive: LinearRgba::new(0.55, 0.42, 0.10, 1.0),
        ..default()
    });
    let cool = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.82, 0.95),
        emissive: LinearRgba::new(0.15, 0.35, 0.55, 1.0),
        ..default()
    });
    let red = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.45, 0.40),
        emissive: LinearRgba::new(0.5, 0.1, 0.1, 1.0),
        ..default()
    });

    struct Entry {
        interactable: Interactable,
        position: Vec3,
        material: u8,
    }

    let entries = vec![
        // ---------------- Lobby --------------------------------------
        Entry {
            interactable: spec(
                "Guest Register",
                "A leather-bound register listing every suite. Written in flowing cursive.",
                InteractKind::Inspect,
                InteractAction::Message(
                    "Register: 'Suite 1201 — L. Voss, arriving under a private security detail.'".into(),
                ),
                ZoneId::Lobby,
            ),
            position: Vec3::new(-4.0, 1.35, 9.0),
            material: 0,
        },
        Entry {
            interactable: spec(
                "Blueprints",
                "Folded architectural drawings of the resort, marked 'CONFIDENTIAL'.",
                InteractKind::Inspect,
                InteractAction::Message(
                    "Blueprints: the penthouse terrace sits above the west stair — the only way up.".into(),
                ),
                ZoneId::Lobby,
            ),
            position: Vec3::new(-6.6, 1.35, 9.2),
            material: 0,
        },
        Entry {
            interactable: spec(
                "Luggage Trolley",
                "A brass trolley loaded with matching travel cases.",
                InteractKind::Pickup,
                InteractAction::TakeItem("Brass Keycard"),
                ZoneId::Lobby,
            ),
            position: Vec3::new(6.0, 0.9, -4.0),
            material: 1,
        },
        Entry {
            interactable: spec(
                "Lobby Phone",
                "A house phone with a direct line to the service wing.",
                InteractKind::Inspect,
                InteractAction::Message(
                    "You hear kitchen chatter: 'Security is short-staffed tonight.'".into(),
                ),
                ZoneId::Lobby,
            ),
            position: Vec3::new(12.0, 1.1, 6.0),
            material: 1,
        },
        Entry {
            interactable: spec(
                "Lounge Sofa",
                "A deep velvet sofa. Temporary rest is possible, guards will still patrol.",
                InteractKind::Seat,
                InteractAction::ToggleSit,
                ZoneId::Lobby,
            ),
            position: Vec3::new(-16.5, 0.85, -4.6),
            material: 0,
        },
        // ---------------- Restaurant ---------------------------------
        Entry {
            interactable: spec(
                "Menu",
                "The Azure Restaurant's evening menu, embossed in gold.",
                InteractKind::Inspect,
                InteractAction::Message(
                    "Menu: tasting courses served on the terrace from eight o'clock.".into(),
                ),
                ZoneId::Restaurant,
            ),
            position: Vec3::new(36.0, 3.4, -8.0),
            material: 0,
        },
        Entry {
            interactable: spec(
                "Wine Rack",
                "Rows of dusted bottles. A staff apron hangs on the side.",
                InteractKind::Container,
                InteractAction::TakeItem("Waiter's Apron"),
                ZoneId::Restaurant,
            ),
            position: Vec3::new(53.0, 3.2, 3.0),
            material: 2,
        },
        Entry {
            interactable: spec(
                "Kitchen Order Rail",
                "Tickets pinned in order. One is blank — a perfect forgery template.",
                InteractKind::Inspect,
                InteractAction::Message(
                    "Order rail: only one ticket is unclaimed… for the penthouse.".into(),
                ),
                ZoneId::Restaurant,
            ),
            position: Vec3::new(49.0, 3.6, 8.0),
            material: 1,
        },
        // ---------------- Gardens ------------------------------------
        Entry {
            interactable: spec(
                "Garden Bench",
                "A mossy stone bench under the palms.",
                InteractKind::Seat,
                InteractAction::ToggleSit,
                ZoneId::Gardens,
            ),
            position: Vec3::new(-8.0, 0.95, -20.4),
            material: 0,
        },
        Entry {
            interactable: spec(
                "Fountain Plaque",
                "A bronze plaque, green with age.",
                InteractKind::Inspect,
                InteractAction::Message(
                    "Plaque: 'Coral Palm Resort — built 1912, rebuilt 1997.'".into(),
                ),
                ZoneId::Gardens,
            ),
            position: Vec3::new(1.0, 1.2, -25.0),
            material: 0,
        },
        Entry {
            interactable: spec(
                "Hidden Stash",
                "A loose paving stone conceals a small cache.",
                InteractKind::Container,
                InteractAction::TakeItem("Service Key"),
                ZoneId::Gardens,
            ),
            position: Vec3::new(-27.6, 0.45, -37.0),
            material: 2,
        },
        // ---------------- Beach --------------------------------------
        Entry {
            interactable: spec(
                "Beach Bar",
                "A thatched bar with a chipped ice bucket.",
                InteractKind::Drink,
                InteractAction::Message("You sip something cold. The sun is relentless.".into()),
                ZoneId::Beach,
            ),
            position: Vec3::new(-20.0, 1.2, -49.0),
            material: 1,
        },
        Entry {
            interactable: spec(
                "Sun Lounger",
                "A striped lounger right at the water's edge.",
                InteractKind::Seat,
                InteractAction::ToggleSit,
                ZoneId::Beach,
            ),
            position: Vec3::new(4.0, 0.7, -54.0),
            material: 0,
        },
        // ---------------- Service corridor ---------------------------
        Entry {
            interactable: spec(
                "Staff Locker",
                "A dented metal locker holding a folded uniform.",
                InteractKind::Wardrobe,
                InteractAction::ChangeDisguise(crate::player_state::OutfitKind::Staff),
                ZoneId::ServiceCorridor,
            ),
            position: Vec3::new(-46.3, -1.2, 13.0),
            material: 1,
        },
        Entry {
            interactable: spec(
                "Laundry Cart",
                "Freshly pressed linens conceal a spare keycard.",
                InteractKind::Container,
                InteractAction::TakeItem("Staff Keycard"),
                ZoneId::ServiceCorridor,
            ),
            position: Vec3::new(-44.0, -2.4, -2.0),
            material: 0,
        },
        Entry {
            interactable: spec(
                "Maintenance Chart",
                "A laminated map of every service duct in the resort.",
                InteractKind::Inspect,
                InteractAction::Message(
                    "The ducts run beside the penthouse stair — a way to avoid the gate.".into(),
                ),
                ZoneId::ServiceCorridor,
            ),
            position: Vec3::new(-47.8, -1.6, 6.0),
            material: 0,
        },
        // ---------------- Staff room ---------------------------------
        Entry {
            interactable: spec(
                "Uniform Rack",
                "Tailored staff uniforms in every size.",
                InteractKind::Wardrobe,
                InteractAction::ChangeDisguise(crate::player_state::OutfitKind::Staff),
                ZoneId::ServiceCorridor,
            ),
            position: Vec3::new(34.0, 1.1, 22.0),
            material: 1,
        },
        // ---------------- Security wing ------------------------------
        Entry {
            interactable: spec(
                "Security Locker",
                "A heavy locker with a security uniform inside.",
                InteractKind::Wardrobe,
                InteractAction::ChangeDisguise(crate::player_state::OutfitKind::Security),
                ZoneId::SecurityWing,
            ),
            position: Vec3::new(-49.0, -1.3, -12.0),
            material: 1,
        },
        Entry {
            interactable: spec(
                "Camera Panel",
                "A bank of monitor switches labelled by zone.",
                InteractKind::Device,
                InteractAction::ToggleDevice,
                ZoneId::SecurityWing,
            ),
            position: Vec3::new(-56.0, -2.0, -24.0),
            material: 2,
        },
        Entry {
            interactable: spec(
                "Duty Roster",
                "The guard rotation, scribbled in pencil.",
                InteractKind::Inspect,
                InteractAction::Message(
                    "Roster: Petra covers the terrace, Sal the wing. Change-over at dusk.".into(),
                ),
                ZoneId::SecurityWing,
            ),
            position: Vec3::new(-60.0, -1.6, -16.0),
            material: 0,
        },
        // ---------------- Penthouse ----------------------------------
        Entry {
            interactable: spec(
                "Penthouse Safe",
                "A floor safe, its door ajar.",
                InteractKind::Container,
                InteractAction::Message(
                    "The safe holds a dossier — 'Project Halcyon: Coral Palm acquisition.'".into(),
                ),
                ZoneId::Penthouse,
            ),
            position: Vec3::new(6.2, 15.0, 44.2),
            material: 2,
        },
        Entry {
            interactable: spec(
                "Hot Tub",
                "Steam curls off the water. Someone left a dataslate on the rim.",
                InteractKind::Inspect,
                InteractAction::Message(
                    "Dataslate: 'Terrace access code — 4-7-1-2. Do not share.'".into(),
                ),
                ZoneId::Penthouse,
            ),
            position: Vec3::new(12.0, 14.5, 33.0),
            material: 1,
        },
        Entry {
            interactable: spec(
                "Target's Desk",
                "Lady Voss's private desk, immaculately tidy.",
                InteractKind::Inspect,
                InteractAction::Message(
                    "A letter: 'The board meets at midnight. Bring the Halcyon files.'".into(),
                ),
                ZoneId::Penthouse,
            ),
            position: Vec3::new(0.0, 31.2, 43.5),
            material: 0,
        },
    ];

    let materials = [warm, cool, red];
    for entry in entries {
        let position = entry.position;
        commands.spawn((
            Name::new(format!("Interactable: {}", entry.interactable.label)),
            entry.interactable,
            Mesh3d(marker_mesh.clone()),
            MeshMaterial3d(materials[entry.material as usize].clone()),
            Transform::from_translation(position),
            bevy::pbr::NotShadowCaster,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_constants_are_reasonable() {
        let range = FOCUS_RANGE;
        let cone = FOCUS_CONE;
        assert!(range > 1.5);
        assert!(cone > 0.0 && cone < 1.0);
    }
}
