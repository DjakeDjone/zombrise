//! Combat systems for handling player attacks and damage.

use avian3d::prelude::SpatialQuery;
use bevy::prelude::*;
use lightyear::prelude::input::native::ActionState;

use zombrise_shared::entity2::Health;
use zombrise_shared::players::player::{DamageFlash, Player, PlayerAttackCooldown, PlayerOwner};
use zombrise_shared::protocol::GameInput;
use zombrise_shared::shared::ZombieDamageFlash;
use zombrise_shared::zombie::zombie::{Zombie, ZombieDying};

/// Attack constants
pub const ATTACK_RANGE: f32 = 2.5;
pub const ATTACK_DAMAGE: f32 = 25.0;
pub const ATTACK_COOLDOWN: f32 = 0.5;
pub const DAMAGE_RANGE: f32 = 1.5;
pub const DAMAGE_PER_SECOND: f32 = 10.0;

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
    for (
        _player_entity,
        _owner,
        mut player_transform,
        _player_health,
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
            let mut closest_zombie: Option<(Entity, Vec3, f32)> = None; // (entity, to_zombie, distance)

            for (zombie_entity, zombie_transform) in &zombie_query {
                let to_zombie = zombie_transform.translation - attack_origin;
                let distance = to_zombie.length();

                if distance <= ATTACK_RANGE
                    && (closest_zombie.is_none() || distance < closest_zombie.as_ref().unwrap().2)
                {
                    closest_zombie = Some((zombie_entity, to_zombie, distance));
                }
            }

            // If there's a zombie in range, rotate toward it and attack
            if let Some((zombie_entity, to_zombie, _distance)) = closest_zombie {
                // Always rotate player toward the closest zombie
                let dir = to_zombie.normalize();
                let flat_dir = Vec3::new(dir.x, 0.0, dir.z).normalize_or_zero();

                if flat_dir.length_squared() > 0.0 {
                    player_transform.look_to(flat_dir, Vec3::Y);
                }

                // Apply damage to this zombie
                if let Ok((mut zombie_health, mut zombie_flash)) =
                    zombie_health_query.get_mut(zombie_entity)
                {
                    zombie_health.current -= ATTACK_DAMAGE;
                    zombie_flash.timer = 0.15;

                    if zombie_health.current <= 0.0 {
                        // Start dying sequence
                        commands.entity(zombie_entity).insert(ZombieDying {
                            timer: 0.0,
                            fall_duration: 1.0,
                            burn_duration: 2.0,
                        });
                    }
                }
            }
        }
    }
}

/// Zombie collision damage to players
pub fn zombie_collision_damage(
    zombie_query: Query<&Transform, (With<Zombie>, Without<ZombieDying>)>,
    mut player_query: Query<(&Transform, &mut Health, &mut DamageFlash), With<Player>>,
    time: Res<Time>,
) {
    for zombie_transform in &zombie_query {
        for (player_transform, mut health, mut flash) in &mut player_query {
            if health.current <= 0.0 {
                continue;
            }

            let dist = zombie_transform
                .translation
                .distance(player_transform.translation);
            if dist <= DAMAGE_RANGE {
                health.current -= DAMAGE_PER_SECOND * time.delta_secs();
                flash.timer = 0.1;
            }
        }
    }
}
