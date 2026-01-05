//! Player-related systems for damage, cooldowns, and regeneration.

use bevy::prelude::*;

use zombrise_shared::entity2::Health;
use zombrise_shared::players::player::{DamageFlash, Player, PlayerAttackCooldown};

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

/// Update attack cooldown timer
pub fn update_attack_cooldown(mut query: Query<&mut PlayerAttackCooldown>, time: Res<Time>) {
    for mut cooldown in &mut query {
        if cooldown.0 > 0.0 {
            cooldown.0 -= time.delta_secs();
        }
    }
}

/// Passive health regeneration for players
pub fn passive_health_regeneration(mut query: Query<&mut Health, With<Player>>, time: Res<Time>) {
    for mut health in &mut query {
        if health.current < health.max {
            health.current =
                (health.current + HEALTH_REGEN_RATE * time.delta_secs()).min(health.max);
        }
    }
}
