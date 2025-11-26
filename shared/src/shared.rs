pub use crate::players::player::{
    DamageFlash, DamagePlayer, Health, MovePlayer, Player, PlayerAttack, PlayerOwner,
};
pub use crate::zombie::zombie::Zombie;
use bevy::prelude::*;
use bevy_replicon::prelude::{*, Channel};
use serde::{Deserialize, Serialize};

#[derive(Component, Serialize, Deserialize, Clone, Debug, Reflect, Default)]
#[reflect(Component)]
pub struct MapMarker;

#[derive(Component, Serialize, Deserialize, Clone, Debug, Reflect, Default)]
#[reflect(Component)]
pub struct TreeMarker;

pub struct SharedPlugin;

impl Plugin for SharedPlugin {
    fn build(&self, app: &mut App) {
        app.replicate::<Player>();
        app.replicate::<PlayerOwner>();
        app.replicate::<Health>();
        app.replicate::<DamageFlash>();
        app.replicate::<Zombie>();
        app.replicate::<Transform>();
        app.replicate::<MapMarker>();
        app.replicate::<TreeMarker>();
        app.add_client_message::<MovePlayer>(Channel::Unreliable);
        app.add_client_message::<PlayerAttack>(Channel::Unreliable);
    }
}
