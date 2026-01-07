use crate::entity2::Health;
pub use crate::players::player::{DamageFlash, DamagePlayer, Player, PlayerDying, PlayerOwner};
pub use crate::zombie::zombie::{Zombie, ZombieDamageFlash, ZombieDying};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::protocol::ProtocolPlugin;

#[derive(Component, Serialize, Deserialize, Clone, Debug, Reflect, Default, PartialEq)]
#[reflect(Component)]
pub struct MapMarker;

#[derive(Component, Serialize, Deserialize, Clone, Debug, Reflect, Default, PartialEq)]
#[reflect(Component)]
pub struct TreeMarker;

pub struct SharedPlugin;

impl Plugin for SharedPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ProtocolPlugin);

        // In Lightyear 0.25, component replication is handled by adding the `Replicate`
        // component to entities at spawn time, rather than registering components globally.
        // The server spawns entities with `Replicate::to_clients(NetworkTarget::All)`.
        //
        // Components just need to derive Serialize/Deserialize and be registered as
        // Bevy types if needed for reflection.
        app.register_type::<Player>();
        app.register_type::<PlayerOwner>();
        app.register_type::<Health>();
        app.register_type::<DamageFlash>();
        app.register_type::<crate::players::player::PlayerDying>();
        app.register_type::<Zombie>();
        app.register_type::<ZombieDamageFlash>();
        app.register_type::<ZombieDying>();
        app.register_type::<MapMarker>();
        app.register_type::<TreeMarker>();
        app.register_type::<crate::zombie::zombie::ZombieAnimationState>();

        // Add buffer_input system for client builds
        // IMPORTANT: Must be in InputSystems::WriteClientInputs set for Lightyear to capture inputs
        #[cfg(feature = "client")]
        {
            use bevy::prelude::FixedPreUpdate;
            use lightyear::prelude::client::input::InputSystems;

            app.init_resource::<crate::players::player::LocalInputState>();
            app.add_systems(
                bevy::prelude::PreUpdate,
                crate::players::player::gather_input,
            );

            app.add_systems(
                FixedPreUpdate,
                crate::players::player::buffer_input.in_set(InputSystems::WriteClientInputs),
            );
        }

        // Register shared movement system for both client (prediction) and server
        app.add_systems(FixedUpdate, crate::players::player::handle_player_movement);
    }
}
