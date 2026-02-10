//! Combat systems for handling player attacks and damage.

use bevy::prelude::*;

use zombrise_shared::entity2::Health;

use zombrise_shared::players::player::{DamageFlash, Player};
use zombrise_shared::shared::ZombieDamageFlash;
use zombrise_shared::zombie::zombie::{Zombie, ZombieDying};

use super::zombie_ai::{ZombieBehavior, ZombieAiState};

use zombrise_shared::combat::{DAMAGE_PER_SECOND, DAMAGE_RANGE};

/// Handle player attacks event (apply damage)
pub fn apply_attack_damage(
    trigger: On<zombrise_shared::combat::PlayerAttackedEvent>,
    mut zombie_health_query: Query<
        (&mut Health, &mut ZombieDamageFlash),
        (With<Zombie>, Without<Player>, Without<ZombieDying>),
    >,
    mut player_query: Query<(&mut Health, &mut DamageFlash), With<Player>>,
    mut commands: Commands,
) {
    let event = trigger.event();
    let attacker_entity = event.attacker;
    let target_entity = event.target;
    let is_zombie = event.is_zombie;

    // We need to fetch the attacker's health to apply bonuses
    // We can't fetch it in the same query if it's PvP, so handle separately

    if is_zombie {
        // Apply damage to zombie
        if let Ok((mut zombie_health, mut zombie_flash)) =
            zombie_health_query.get_mut(target_entity)
        {
            zombie_health.current -= zombrise_shared::combat::ATTACK_DAMAGE;
            zombie_flash.timer = 0.15;

            if zombie_health.current <= 0.0 {
                // Reward the player with increased max health
                if let Ok((mut player_health, _)) = player_query.get_mut(attacker_entity) {
                    player_health.max += zombrise_shared::combat::MAX_HEALTH_BONUS_PER_KILL;
                    player_health.current += zombrise_shared::combat::MAX_HEALTH_BONUS_PER_KILL;
                }

                // Start dying sequence
                commands.entity(target_entity).insert(ZombieDying {
                    timer: 0.0,
                    fall_duration: 1.0,
                    burn_duration: 2.0,
                });
            }
        }
    } else {
        // Apply damage to other player
        // Just insert pending damage, simpler for now
        commands.entity(target_entity).insert(PendingDamage {
            amount: zombrise_shared::combat::ATTACK_DAMAGE,
        });
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
