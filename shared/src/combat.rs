use bevy::prelude::*;

use lightyear::prelude::input::native::ActionState;
use serde::{Deserialize, Serialize};


use crate::players::player::{Player, PlayerAttackCooldown, PlayerOwner};
use crate::protocol::GameInput;
use crate::zombie::zombie::{Zombie, ZombieDying};

/// Attack constants
pub const ATTACK_RANGE: f32 = 2.5;
pub const ATTACK_DAMAGE: f32 = 25.0;
pub const ATTACK_COOLDOWN: f32 = 0.5;
pub const DAMAGE_RANGE: f32 = 1.5;
pub const DAMAGE_PER_SECOND: f32 = 10.0;
pub const MAX_HEALTH_BONUS_PER_KILL: f32 = 5.0;

#[derive(Event, Debug, Serialize, Deserialize, Clone, PartialEq, Reflect)]
pub struct PlayerAttackedEvent {
    pub attacker: Entity,
    pub target: Entity,
    pub is_zombie: bool,
}

/// Update attack cooldown timer
pub fn update_attack_cooldown(mut query: Query<&mut PlayerAttackCooldown>, time: Res<Time>) {
    for mut cooldown in &mut query {
        if cooldown.0 > 0.0 {
            cooldown.0 -= time.delta_secs();
        }
    }
}

/// Identify the closest target (player or zombie) to the attacker
pub fn find_closest_target(
    attacker_transform: &Transform,
    attacker_entity: Entity,
    attacker_owner: u64,
    zombies: &Query<(Entity, &Transform), (With<Zombie>, Without<Player>, Without<ZombieDying>)>,
    all_players_data: &[(Entity, Vec3, u64)],
) -> Option<(Entity, Vec3, bool)> {
    let attack_origin = attacker_transform.translation;

    // Find the closest zombie within range
    let mut closest_zombie: Option<(Entity, Vec3, f32)> = None; // (entity, to_target, distance)

    for (zombie_entity, zombie_transform) in zombies {
        let to_zombie = zombie_transform.translation - attack_origin;
        let distance = to_zombie.length();

        if distance <= ATTACK_RANGE
            && (closest_zombie.is_none() || distance < closest_zombie.as_ref().unwrap().2)
        {
            closest_zombie = Some((zombie_entity, to_zombie, distance));
        }
    }

    // Find the closest OTHER player within range
    let mut closest_player: Option<(Entity, Vec3, f32)> = None;

    for (other_entity, other_pos, other_owner) in all_players_data {
        // Don't attack self
        if *other_entity == attacker_entity || *other_owner == attacker_owner {
            continue;
        }

        let to_player = *other_pos - attack_origin;
        let distance = to_player.length();

        if distance <= ATTACK_RANGE
            && (closest_player.is_none() || distance < closest_player.as_ref().unwrap().2)
        {
            closest_player = Some((*other_entity, to_player, distance));
        }
    }

    // Determine which target is closer: zombie or player
    match (closest_zombie, closest_player) {
        (Some((z_ent, z_dir, z_dist)), Some((p_ent, p_dir, p_dist))) => {
            if z_dist <= p_dist {
                Some((z_ent, z_dir, true)) // true = is zombie
            } else {
                Some((p_ent, p_dir, false)) // false = is player
            }
        }
        (Some((z_ent, z_dir, _)), None) => Some((z_ent, z_dir, true)),
        (None, Some((p_ent, p_dir, _))) => Some((p_ent, p_dir, false)),
        (None, None) => None,
    }
}

/// Initiate attack: rotate towards target, set cooldown, emit event

pub fn initiate_attack(
    mut commands: Commands,
    mut player_queries: ParamSet<(
        Query<
            (
                Entity,
                &PlayerOwner,
                &Transform,
                &PlayerAttackCooldown,
                &ActionState<GameInput>,
            ),
            With<Player>,
        >,
        Query<(Entity, &mut Transform, &mut PlayerAttackCooldown), With<Player>>,
    )>,
    zombie_query: Query<
        (Entity, &Transform),
        (With<Zombie>, Without<Player>, Without<ZombieDying>),
    >,
) {
    let mut attacks_to_perform = Vec::new();

    // Pass 1: Identify attacks (Read-only on Players)
    {
        let players = player_queries.p0();
        // Collect all player data for PvP targeting
        let all_players_data: Vec<(Entity, Vec3, u64)> = players
            .iter()
            .map(|(e, o, t, _, _)| (e, t.translation, o.0))
            .collect();

        for (entity, owner, transform, cooldown, action_state) in players.iter() {
            // Skip if on cooldown
            if cooldown.0 > 0.0 {
                continue;
            }

            // Check for attack input
            if matches!(action_state.0, GameInput::Attack) {
                // Find target
                if let Some((target_entity, to_target, is_zombie)) = find_closest_target(
                    transform,
                    entity,
                    owner.0,
                    &zombie_query,
                    &all_players_data,
                ) {
                    attacks_to_perform.push((entity, target_entity, to_target, is_zombie));
                }
            }
        }
    }

    // Pass 2: Execute attacks (Write to Players)
    {
        let mut players_mut = player_queries.p1();
        for (attacker_entity, target_entity, to_target, is_zombie) in attacks_to_perform {
            if let Ok((_, mut transform, mut cooldown)) = players_mut.get_mut(attacker_entity) {
                // Set cooldown
                cooldown.0 = ATTACK_COOLDOWN;

                // Determine rotation direction
                let dir = to_target.normalize();
                let flat_dir = Vec3::new(dir.x, 0.0, dir.z).normalize_or_zero();

                // Rotate towards target
                if flat_dir.length_squared() > 0.0 {
                    transform.look_to(flat_dir, Vec3::Y);
                }

                // Send event (Server will handle damage, Client uses for prediction/visuals)
                commands.trigger(PlayerAttackedEvent {
                    attacker: attacker_entity,
                    target: target_entity,
                    is_zombie,
                });
            }
        }
    }
}
