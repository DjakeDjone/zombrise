//! Combat systems for handling player attacks and damage.

use avian3d::prelude::SpatialQuery;
use bevy::prelude::*;
use lightyear::prelude::input::native::ActionState;

use zombrise_shared::entity2::Health;
use zombrise_shared::players::player::{DamageFlash, Player, PlayerAttackCooldown, PlayerOwner};
use zombrise_shared::protocol::GameInput;
use zombrise_shared::shared::ZombieDamageFlash;
use zombrise_shared::zombie::zombie::{Zombie, ZombieDying};

use super::zombie_ai::{ZombieBehavior, ZombieAiState};

/// Attack constants
pub const ATTACK_RANGE: f32 = 2.5;
pub const ATTACK_DAMAGE: f32 = 25.0;
pub const ATTACK_COOLDOWN: f32 = 0.5;
pub const DAMAGE_RANGE: f32 = 1.5;
pub const DAMAGE_PER_SECOND: f32 = 10.0;
/// Max health bonus awarded to player for each zombie kill
pub const MAX_HEALTH_BONUS_PER_KILL: f32 = 5.0;

/// Handle player attacks
pub fn handle_player_attack(
    mut player_query: Query<
        (
            Entity,
            &PlayerOwner,
            &mut Transform,
            &mut Health,
            &mut DamageFlash,
            &mut PlayerAttackCooldown,
            &ActionState<GameInput>,
        ),
        With<Player>,
    >,
    zombie_query: Query<
        (Entity, &Transform),
        (With<Zombie>, Without<Player>, Without<ZombieDying>),
    >,
    mut zombie_health_query: Query<
        (&mut Health, &mut ZombieDamageFlash),
        (With<Zombie>, Without<Player>, Without<ZombieDying>),
    >,
    mut commands: Commands,
    _spatial_query: SpatialQuery,
) {
    // Collect player positions and entities for PvP targeting
    // We need this because we can't have mutable access to the same query twice
    let player_positions: Vec<(Entity, u64, Vec3)> = player_query
        .iter()
        .map(|(e, owner, t, _, _, _, _)| (e, owner.0, t.translation))
        .collect();

    for (
        player_entity,
        owner,
        mut player_transform,
        mut player_health,
        _damage_flash,
        mut cooldown,
        action_state,
    ) in &mut player_query
    {
        if cooldown.0 > 0.0 {
            continue;
        }

        if matches!(action_state.0, GameInput::Attack) {
            cooldown.0 = ATTACK_COOLDOWN;

            let attack_origin = player_transform.translation;

            // Find the closest zombie within range
            let mut closest_zombie: Option<(Entity, Vec3, f32)> = None; // (entity, to_target, distance)

            for (zombie_entity, zombie_transform) in &zombie_query {
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

            for (other_entity, other_owner, other_pos) in &player_positions {
                // Don't attack self
                if *other_entity == player_entity || *other_owner == owner.0 {
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
            let attack_target: Option<(Entity, Vec3, bool)> = match (closest_zombie, closest_player)
            {
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
            };

            // If there's a target in range, rotate toward it and attack
            if let Some((target_entity, to_target, is_zombie)) = attack_target {
                // Always rotate player toward the target
                let dir = to_target.normalize();
                let flat_dir = Vec3::new(dir.x, 0.0, dir.z).normalize_or_zero();

                if flat_dir.length_squared() > 0.0 {
                    player_transform.look_to(flat_dir, Vec3::Y);
                }

                if is_zombie {
                    // Apply damage to zombie
                    if let Ok((mut zombie_health, mut zombie_flash)) =
                        zombie_health_query.get_mut(target_entity)
                    {
                        zombie_health.current -= ATTACK_DAMAGE;
                        zombie_flash.timer = 0.15;

                        if zombie_health.current <= 0.0 {
                            // Reward the player with increased max health
                            player_health.max += MAX_HEALTH_BONUS_PER_KILL;
                            player_health.current += MAX_HEALTH_BONUS_PER_KILL;

                            // Start dying sequence
                            commands.entity(target_entity).insert(ZombieDying {
                                timer: 0.0,
                                fall_duration: 1.0,
                                burn_duration: 2.0,
                            });
                        }
                    }
                } else {
                    // Apply damage to other player - we need to defer this
                    // because we can't mutably borrow player_query again
                    commands.entity(target_entity).insert(PendingDamage {
                        amount: ATTACK_DAMAGE,
                    });
                }
            }
        }
    }
}

/// Marker component for pending damage to apply next frame
#[derive(Component)]
pub struct PendingDamage {
    pub amount: f32,
}

/// Apply pending damage to players (runs after handle_player_attack)
pub fn apply_pending_player_damage(
    mut commands: Commands,
    mut player_query: Query<(Entity, &mut Health, &mut DamageFlash, &PendingDamage), With<Player>>,
) {
    for (entity, mut health, mut flash, pending) in &mut player_query {
        health.current -= pending.amount;
        flash.timer = 0.15;
        commands.entity(entity).remove::<PendingDamage>();
    }
}

/// Zombie collision damage to players — only when zombie is attacking
pub fn zombie_collision_damage(
    zombie_query: Query<(&Transform, &ZombieBehavior), (With<Zombie>, Without<ZombieDying>)>,
    mut player_query: Query<(&Transform, &mut Health, &mut DamageFlash), With<Player>>,
    time: Res<Time>,
) {
    // Only consider zombies that are in their Attacking state
    let zombie_positions: Vec<Vec3> = zombie_query
        .iter()
        .filter(|(_, behavior)| behavior.state == ZombieAiState::Attacking)
        .map(|(t, _)| t.translation)
        .collect();

    for (player_transform, mut health, mut flash) in &mut player_query {
        // Early exit for dead players
        if health.current <= 0.0 {
            continue;
        }

        // Check if any attacking zombie is in range
        let in_range = zombie_positions
            .iter()
            .any(|zombie_pos| zombie_pos.distance(player_transform.translation) <= DAMAGE_RANGE);

        if in_range {
            health.current -= DAMAGE_PER_SECOND * time.delta_secs();
            flash.timer = 0.1;
        }
    }
}
