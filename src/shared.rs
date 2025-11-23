pub use crate::players::player::{MovePlayer, Player, PlayerOwner};
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};

pub struct SharedPlugin;

impl Plugin for SharedPlugin {
    fn build(&self, app: &mut App) {
        app.replicate::<Player>();
        app.replicate::<PlayerOwner>();
        app.replicate::<Transform>();
        app.add_client_event::<MovePlayer>(ChannelKind::Unordered);
    }
}
