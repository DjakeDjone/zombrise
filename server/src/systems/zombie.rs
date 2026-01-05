//! Zombie-specific systems for damage flash and death.

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;

use zombrise_shared::shared::ZombieDamageFlash;
use zombrise_shared::zombie::zombie::{Zombie, ZombieAnimationState, ZombieDying};

/// Update zombie damage flash timer
pub fn update_zombie_damage_flash(mut query: Query<&mut ZombieDamageFlash>, time: Res<Time>) {
    for mut flash in &mut query {
        if flash.timer > 0.0 {
            flash.timer -= time.delta_secs();
        }
    }
}

/// Update dying zombies (fall and burn sequence)
pub fn update_dying_zombies(
    mut commands: Commands,
    mut dying_query: Query<
        (
            Entity,
            &mut ZombieDying,
            &mut ZombieAnimationState,
            &mut LinearVelocity,
        ),
        With<Zombie>,
    >,
    time: Res<Time>,
) {
    for (entity, mut dying, mut anim_state, mut velocity) in &mut dying_query {
        dying.timer += time.delta_secs();
        *anim_state = ZombieAnimationState::Dying;
        velocity.x = 0.0;
        velocity.z = 0.0;

        if dying.timer >= dying.fall_duration + dying.burn_duration {
            commands.entity(entity).despawn();
        }
    }
}
