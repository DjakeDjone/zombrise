pub use crate::players::player::{
    DamageFlash, DamagePlayer, Health, MovePlayer, Player, PlayerAttack, PlayerOwner,
};
pub use crate::zombie::zombie::Zombie;
use bevy::prelude::*;
use bevy_replicon::prelude::*;
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
        app.replicate::<NetworkTransform>();
        app.replicate::<MapMarker>();
        app.replicate::<TreeMarker>();
        app.add_client_event::<MovePlayer>(ChannelKind::Unordered);
        app.add_client_event::<PlayerAttack>(ChannelKind::Unordered);
    }
}

#[derive(Component, Serialize, Deserialize, Clone, Debug, Reflect, Default)]
#[reflect(Component)]
pub struct NetworkTransform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl From<Transform> for NetworkTransform {
    fn from(t: Transform) -> Self {
        Self {
            translation: t.translation,
            rotation: t.rotation,
            scale: t.scale,
        }
    }
}

impl NetworkTransform {
    pub fn as_transform(&self) -> Transform {
        Transform {
            translation: self.translation,
            rotation: self.rotation,
            scale: self.scale,
        }
    }
}
