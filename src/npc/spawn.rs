//! Population of the resort.
//!
//! Every NPC gets a role, a looping routine of waypoints and a procedural
//! body. Routines are authored along walkable paths so Stage 1 navigation is
//! reliable; the collision solver is a safety net, not the primary planner.

use super::brain::{Brain, BrainState};
use super::schedule::RoutineStep;
use super::{NpcIdentity, Waypoint};
use super::{Npc, NpcRole};
use crate::core::ZoneId;
use crate::player::character::{
    spawn_humanoid, ActorBody, CharacterMeshes, HumanoidRig, OutfitMaterials,
};
use crate::player::Stance;
use crate::world::layout::resort_layout;
use bevy::prelude::*;

/// Authored NPC definition: identity plus routine.
struct NpcSpec {
    name: &'static str,
    role: NpcRole,
    zone: ZoneId,
    routine: Vec<(Vec3, f32, &'static str)>,
    /// Preferred walking speed (currently informational: navigation derives
    /// speed from the waypoint's dwell time).
    speed_hint: f32,
}

impl NpcSpec {
    fn new(name: &'static str, role: NpcRole, zone: ZoneId, speed_hint: f32) -> Self {
        Self {
            name,
            role,
            zone,
            routine: Vec::new(),
            speed_hint,
        }
    }

    fn stop(mut self, position: Vec3, wait: f32, activity: &'static str) -> Self {
        self.routine.push((position, wait, activity));
        self
    }

    /// Build a brain from the routine, starting on the second waypoint so the
    /// NPCs do not all begin by walking towards their first stop.
    fn into_brain(self) -> Brain {
        let steps: Vec<RoutineStep> = self
            .routine
            .iter()
            .map(|(p, w, a)| RoutineStep::new(*p, *w, *a))
            .collect();
        let mut brain = Brain::new(steps);
        brain.state = BrainState::Walking;
        brain
    }
}

/// All NPCs in the Stage 1 resort.
fn npc_specs() -> Vec<NpcSpec> {
    vec![
        // --- Lobby ------------------------------------------------------
        NpcSpec::new("Marisol the Concierge", NpcRole::Concierge, ZoneId::Lobby, 1.0)
            .stop(Vec3::new(-4.0, 0.0, 6.0), 12.0, "checking the reservation book")
            .stop(Vec3::new(-20.0, 0.0, 4.0), 6.0, "greeting arriving guests")
            .stop(Vec3::new(-2.0, 0.0, 20.0), 8.0, "watching the terrace door")
            .stop(Vec3::new(16.0, 0.0, -8.0), 5.0, "straightening the lounge"),
        NpcSpec::new("Guest: Mr Alvarez", NpcRole::Guest, ZoneId::Lobby, 1.0)
            .stop(Vec3::new(18.0, 0.0, -6.0), 9.0, "reading by the window")
            .stop(Vec3::new(0.0, 0.0, -10.0), 4.0, "admiring the sculpture")
            .stop(Vec3::new(22.0, 0.0, 16.0), 7.0, "waiting for the lift")
            .stop(Vec3::new(-18.0, 0.0, -6.0), 8.0, "sipping a cocktail"),
        NpcSpec::new("Guest: Mrs Okafor", NpcRole::Guest, ZoneId::Lobby, 1.0)
            .stop(Vec3::new(-18.0, 0.0, -5.0), 10.0, "chatting with friends")
            .stop(Vec3::new(12.0, 0.0, 2.0), 5.0, "taking a phone call")
            .stop(Vec3::new(-10.0, 0.0, 18.0), 6.0, "heading for the terrace")
            .stop(Vec3::new(6.0, 0.0, -8.0), 6.0, "browsing the lobby shops"),
        // --- Restaurant -------------------------------------------------
        NpcSpec::new("Waiter: Tobias", NpcRole::Waiter, ZoneId::Restaurant, 1.1)
            .stop(Vec3::new(36.0, 2.4, -6.0), 4.0, "serving table four")
            .stop(Vec3::new(48.0, 2.4, 6.0), 6.0, "collecting orders from the kitchen")
            .stop(Vec3::new(36.0, 2.4, 2.0), 4.0, "serving table two")
            .stop(Vec3::new(42.0, 2.4, 10.0), 7.0, "resetting a table"),
        NpcSpec::new("Chef: Rinaldo", NpcRole::Chef, ZoneId::Restaurant, 0.8)
            .stop(Vec3::new(51.0, 2.4, 10.0), 14.0, "Cooking at the line")
            .stop(Vec3::new(48.0, 2.4, 4.0), 8.0, "checking the order rail")
            .stop(Vec3::new(52.0, 2.4, 12.0), 10.0, "prepping the evening service"),
        NpcSpec::new("Guest: Mr Lindqvist", NpcRole::Guest, ZoneId::Restaurant, 1.0)
            .stop(Vec3::new(36.0, 2.4, -8.5), 16.0, "dining alone")
            .stop(Vec3::new(40.0, 2.4, -1.0), 6.0, "speaking with the waiter")
            .stop(Vec3::new(34.5, 2.4, -13.0), 8.0, "enjoying the terrace view"),
        // --- Gardens ----------------------------------------------------
        NpcSpec::new("Gardener: Ines", NpcRole::Housekeeping, ZoneId::Gardens, 1.0)
            .stop(Vec3::new(-18.0, 0.0, -22.0), 10.0, "watering the flower beds")
            .stop(Vec3::new(0.0, 0.0, -28.0), 6.0, "skimming the fountain")
            .stop(Vec3::new(12.0, 0.0, -22.0), 9.0, "trimming the hedges")
            .stop(Vec3::new(-6.0, 0.0, -40.0), 7.0, "raking the paths"),
        NpcSpec::new("Guest: Ms Tanaka", NpcRole::Guest, ZoneId::Gardens, 1.0)
            .stop(Vec3::new(-24.0, 0.0, -34.0), 8.0, "photographing the gardens")
            .stop(Vec3::new(0.0, 0.0, -26.0), 10.0, "resting by the fountain")
            .stop(Vec3::new(16.0, 0.0, -34.0), 6.0, "strolling the hedge walk")
            .stop(Vec3::new(0.0, 0.0, -18.0), 5.0, "looking back at the hotel"),
        NpcSpec::new("Guest: Mr Boom", NpcRole::Guest, ZoneId::Beach, 1.0)
            .stop(Vec3::new(-30.0, 0.0, -50.0), 12.0, "sunbathing")
            .stop(Vec3::new(-10.0, 0.0, -55.0), 6.0, "walking the shoreline")
            .stop(Vec3::new(16.0, 0.0, -55.0), 8.0, "playing beach volleyball")
            .stop(Vec3::new(30.0, 0.0, -48.0), 10.0, "buying a drink"),
        // --- Service corridor / staff room ------------------------------
        NpcSpec::new("Housekeeping: Ana", NpcRole::Housekeeping, ZoneId::ServiceCorridor, 1.0)
            .stop(Vec3::new(-38.0, -3.4, -26.0), 8.0, "sorting clean linens")
            .stop(Vec3::new(-34.0, -3.4, -6.0), 10.0, "closing a maintenance hatch")
            .stop(Vec3::new(-45.0, -3.4, 6.0), 12.0, "restocking the carts")
            .stop(Vec3::new(-31.0, -3.4, -20.0), 6.0, "returning to the lift"),
        NpcSpec::new("Staff: Rico", NpcRole::Bartender, ZoneId::ServiceCorridor, 1.0)
            .stop(Vec3::new(-44.0, -3.4, -2.0), 9.0, "moving crates")
            .stop(Vec3::new(-33.0, -3.4, -12.0), 7.0, "taking a break")
            .stop(Vec3::new(-35.0, 0.0, 17.0), 12.0, "delivering to the staff room")
            .stop(Vec3::new(-40.0, -3.4, -24.0), 6.0, "checking the stock"),
        // --- Security wing ----------------------------------------------
        NpcSpec::new("Guard: Sal", NpcRole::SecurityGuard, ZoneId::SecurityWing, 1.0)
            .stop(Vec3::new(-56.0, -3.4, -14.0), 12.0, "reviewing camera feeds")
            .stop(Vec3::new(-51.0, -3.4, -27.0), 8.0, "making a round")
            .stop(Vec3::new(-58.0, -3.4, -28.0), 10.0, "checking the equipment rack"),
        // --- Staff room --------------------------------------------------
        NpcSpec::new("Manager: Yves", NpcRole::Manager, ZoneId::ServiceCorridor, 0.9)
            .stop(Vec3::new(33.0, 0.1, 22.0), 14.0, "briefing the staff")
            .stop(Vec3::new(0.0, 0.0, 20.0), 10.0, "inspecting the lobby")
            .stop(Vec3::new(-28.0, 0.0, 8.0), 8.0, "walking the west promenade")
            .stop(Vec3::new(36.0, 0.1, 18.0), 9.0, "checking the staff room"),
        // --- Terrace / penthouse approach --------------------------------
        NpcSpec::new("Guard: Petra", NpcRole::SecurityGuard, ZoneId::Penthouse, 1.0)
            .stop(Vec3::new(-24.0, 29.8, 46.0), 14.0, "patrolling the stair landing")
            .stop(Vec3::new(0.0, 29.8, 30.0), 10.0, "guarding the terrace doors")
            .stop(Vec3::new(14.0, 29.8, 44.0), 12.0, "checking the balcony"),
        // --- The target --------------------------------------------------
        NpcSpec::new("Target: Lady Voss", NpcRole::Target, ZoneId::Penthouse, 0.9)
            .stop(Vec3::new(0.0, 29.8, 38.0), 20.0, "hosting a private meeting")
            .stop(Vec3::new(-12.0, 29.8, 30.0), 10.0, "looking over the resort")
            .stop(Vec3::new(12.0, 29.8, 33.0), 12.0, "conferring with security"),
    ]
}

/// Spawn the resort population.
///
/// Each NPC is dressed by its role (see `NpcRole::outfit`) and given a
/// different skin tone and hair colour, so the crowd reads as a crowd.
pub fn spawn_npcs(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    existing_meshes: Option<Res<CharacterMeshes>>,
    outfits: Option<Res<OutfitMaterials>>,
) {
    let Some(outfits) = outfits else {
        return;
    };
    let character_meshes = CharacterMeshes::get_or_insert(&mut meshes, &mut commands, existing_meshes);

    for (index, spec) in npc_specs().into_iter().enumerate() {
        let variety = index as u8;
        let outfit = spec.role.character_outfit(variety);

        // Start at the NPC's first waypoint so routines spread out.
        let start = spec
            .routine
            .first()
            .map(|(p, _, _)| *p)
            .unwrap_or(Vec3::new(0.0, 0.0, -20.0));

        let identity = NpcIdentity {
            name: spec.name,
            role: spec.role,
            zone: spec.zone,
        };
        let npc_name = spec.name;
        let brain = spec.into_brain();

        let npc = commands
            .spawn((
                Name::new(npc_name),
                Npc,
                identity,
                brain,
                ActorBody {
                    radius: super::navigation::NPC_RADIUS,
                    height: 1.78,
                    velocity: Vec3::ZERO,
                    grounded: true,
                    ground_y: start.y,
                    stance: Stance::Standing,
                    gait: 0.0,
                },
                Transform::from_translation(start + Vec3::Y * 0.01),
            ))
            .id();

        spawn_humanoid(
            &mut commands,
            npc,
            start,
            &character_meshes,
            &outfits,
            outfit,
            None,
        );

        // A small coloured disc above the head identifies the role at a glance
        // without breaking the realistic silhouette.
        commands.spawn((
            NpcBadge { owner: npc },
            Mesh3d(character_meshes.hair[0].clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: spec.role.accent(),
                emissive: LinearRgba::from(spec.role.accent()) * 0.6,
                perceptual_roughness: 0.6,
                ..default()
            })),
            Transform::from_translation(start + HumanoidRig::HEAD + Vec3::Y * 0.24)
                .with_scale(Vec3::new(0.30, 0.09, 0.30)),
            bevy::pbr::NotShadowCaster,
        ));
    }

    info!("Spawned resort NPCs");
}

/// Badge entity that follows an NPC's head.
#[derive(Component)]
pub struct NpcBadge {
    pub owner: Entity,
}

/// Keep badges on top of their NPC's head.
pub fn update_badges(
    npcs: Query<&Transform, With<Npc>>,
    mut badges: Query<(&NpcBadge, &mut Transform), Without<Npc>>,
) {
    for (badge, mut transform) in &mut badges {
        if let Ok(npc) = npcs.get(badge.owner) {
            let yaw = npc.rotation.to_euler(EulerRot::YXZ).0;
            let offset = Quat::from_rotation_y(yaw) * (HumanoidRig::HEAD + Vec3::Y * 0.24);
            transform.translation = npc.translation + offset;
        }
    }
}
