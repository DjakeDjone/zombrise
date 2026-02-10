//! Player-related systems for damage, cooldowns, and regeneration.

use bevy::prelude::*;

use zombrise_shared::entity2::Health;
use zombrise_shared::players::player::{DamageFlash, Player, PlayerDying};

/// Health regeneration rate per second
pub const HEALTH_REGEN_RATE: f32 = 2.0;

/// Update damage flash timer
pub fn update_damage_flash(mut query: Query<&mut DamageFlash>, time: Res<Time>) {
    for mut flash in &mut query {
        if flash.timer > 0.0 {
            flash.timer -= time.delta_secs();
        }
    }
}

/// Passive health regeneration for players (only if not dying)
pub fn passive_health_regeneration(
    mut query: Query<&mut Health, (With<Player>, Without<PlayerDying>)>,
    time: Res<Time>,
) {
    for mut health in &mut query {
        if health.current < health.max {
            health.current =
                (health.current + HEALTH_REGEN_RATE * time.delta_secs()).min(health.max);
        }
    }
}

/// Detect when a player's health drops to 0 and trigger the dying sequence
pub fn detect_player_death(
    mut commands: Commands,
    query: Query<(Entity, &Health), (With<Player>, Without<PlayerDying>)>,
) {
    for (entity, health) in &query {
        if health.current <= 0.0 {
            commands.entity(entity).insert(PlayerDying {
                timer: 0.0,
                fall_duration: 1.0,
                burn_duration: 3.0,
            });
        }
    }
}

/// Update dying players timer and despawn when complete
pub fn update_dying_players(
    mut commands: Commands,
    mut query: Query<(Entity, &mut PlayerDying), With<Player>>,
    time: Res<Time>,
) {
    for (entity, mut dying) in &mut query {
        dying.timer += time.delta_secs();

        // Despawn the player after the full death sequence
        let total_duration = dying.fall_duration + dying.burn_duration;
        if dying.timer >= total_duration {
            commands.entity(entity).despawn();
        }
    }
}
