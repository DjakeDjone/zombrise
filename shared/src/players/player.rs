use bevy::{
    ecs::{
        component::Component,
        event::{Event, EventWriter},
        query::{With, Without},
        system::{Query, Res},
    },
    math::Vec3,
    prelude::Reflect,
    transform::components::Transform,
};

#[cfg(feature = "client")]
use bevy::input::{ButtonInput, keyboard::KeyCode};
use bevy_replicon_renet2::renet2::ClientId;
use serde::{Deserialize, Serialize};

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
pub struct PlayerOwner(pub ClientId);

#[derive(Component)]
pub struct MainCamera;

#[derive(Event, Serialize, Deserialize)]
pub struct MovePlayer {
    pub direction: Vec3,
    pub camera_yaw: f32,
}

#[derive(Event, Serialize, Deserialize)]
pub struct PlayerAttack;

#[derive(Event, Serialize, Deserialize)]
pub struct DamagePlayer {
    pub client_id: ClientId,
    pub amount: f32,
}


#[cfg(feature = "client")]
pub fn handle_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut move_events: EventWriter<MovePlayer>,
    mut attack_events: EventWriter<PlayerAttack>,
    camera_rotation: Option<Res<CameraRotation>>,
) {
    let mut direction = Vec3::ZERO;

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

    if keyboard_input.just_pressed(KeyCode::Space) {
        attack_events.write(PlayerAttack);
    }
}

#[derive(bevy::prelude::Resource)]
pub struct CameraRotation {
    pub yaw: f32,
    pub pitch: f32,
}

pub fn camera_follow(
    player_query: Query<&Transform, (With<Player>, Without<MainCamera>)>,
    mut camera_query: Query<&mut Transform, With<MainCamera>>,
) {
    if let Ok(player_transform) = player_query.single() {
        if let Ok(mut camera_transform) = camera_query.single_mut() {
            let offset = Vec3::new(0.0, 5.0, 10.0);
            camera_transform.translation = player_transform.translation + offset;
            camera_transform.look_at(player_transform.translation, Vec3::Y);
        }
    }
}
