use bevy::{
    ecs::component::Component,
    math::Vec3,
    prelude::{Event, Message, Reflect},
};

#[cfg(feature = "client")]
#[cfg(feature = "client")]
use bevy::{
    ecs::system::{Query, Res},
    input::{keyboard::KeyCode, ButtonInput},
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "client")]
use super::player_animation::PlayerAttacking;

#[derive(Component, Serialize, Deserialize, Reflect)]
pub struct Player;

#[derive(Component, Serialize, Deserialize, Reflect, Clone)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Default for Health {
    fn default() -> Self {
        Self {
            current: 100.0,
            max: 100.0,
        }
    }
}

#[derive(Component, Serialize, Deserialize, Reflect, Default)]
pub struct DamageFlash {
    pub timer: f32,
}

#[derive(Component, Serialize, Deserialize, Reflect)]
pub struct PlayerOwner(pub u64);

#[derive(Component)]
pub struct MainCamera;

/// Resource to track the local client's ID
#[cfg(feature = "client")]
#[derive(bevy::prelude::Resource, Default)]
pub struct MyClientId(pub u64);

#[derive(Event, Message, Serialize, Deserialize)]
pub struct MovePlayer {
    pub direction: Vec3,
    pub camera_yaw: f32,
}

#[derive(Event, Message, Serialize, Deserialize)]
pub struct PlayerAttack;

#[derive(Event, Message, Serialize, Deserialize)]
pub struct DamagePlayer {
    pub client_id: u64,
    pub amount: f32,
}

#[cfg(feature = "client")]
pub fn handle_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut move_events: bevy::prelude::MessageWriter<MovePlayer>,
    mut attack_events: bevy::prelude::MessageWriter<PlayerAttack>,
    camera_rotation: Option<Res<CameraRotation>>,
    player_attacking_query: Query<&PlayerAttacking>,
) {
    let mut direction = Vec3::ZERO;

    // Check if any player (specifically the local one ideally, but simplified here) is attacking
    let is_attacking = player_attacking_query.iter().any(|a| a.is_attacking);

    if !is_attacking {
        if keyboard_input.pressed(KeyCode::ArrowUp) || keyboard_input.pressed(KeyCode::KeyW) {
            direction.z -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::ArrowDown) || keyboard_input.pressed(KeyCode::KeyS) {
            direction.z += 1.0;
        }
        if keyboard_input.pressed(KeyCode::ArrowLeft) || keyboard_input.pressed(KeyCode::KeyA) {
            direction.x -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::ArrowRight) || keyboard_input.pressed(KeyCode::KeyD) {
            direction.x += 1.0;
        }

        if direction.length() > 0.0 {
            direction = direction.normalize();
            let camera_yaw = camera_rotation.map(|r| r.yaw).unwrap_or(0.0);
            move_events.write(MovePlayer {
                direction,
                camera_yaw,
            });
        }
    }

    if keyboard_input.just_pressed(KeyCode::Space) {
        attack_events.write(PlayerAttack);
    }
}

#[derive(bevy::prelude::Resource)]
pub struct CameraRotation {
    pub yaw: f32,
    pub pitch: f32,
}
