pub use crate::players::player::{MovePlayer, Player, PlayerOwner};
pub use crate::zombie::zombie::Zombie;
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Component, Serialize, Deserialize, Clone, Debug, Reflect, Default)]
#[reflect(Component)]
pub struct MapMarker;

pub struct SharedPlugin;

impl Plugin for SharedPlugin {
    fn build(&self, app: &mut App) {
        app.replicate::<Player>();
        app.replicate::<PlayerOwner>();
        app.replicate::<Zombie>();
        app.replicate::<Transform>();
        app.replicate::<MapMarker>();
        app.add_client_event::<MovePlayer>(ChannelKind::Unordered);
    }
}
