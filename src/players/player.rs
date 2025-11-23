use bevy::{
    ecs::{
        component::Component,
        event::{Event, EventWriter},
        query::{With, Without},
        system::{Query, Res},
    },
    input::{ButtonInput, keyboard::KeyCode},
    math::Vec3,
    prelude::Reflect,
    time::Time,
    transform::components::Transform,
};
use bevy_replicon::prelude::ClientId;
use serde::{Deserialize, Serialize};

#[derive(Component, Serialize, Deserialize, Reflect)]
pub struct Player;

#[derive(Component, Serialize, Deserialize, Reflect)]
pub struct PlayerOwner(pub ClientId);

#[derive(Component)]
pub struct MainCamera;

#[derive(Event, Serialize, Deserialize)]
pub struct MovePlayer {
    pub direction: Vec3,
}

pub fn handle_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut move_events: EventWriter<MovePlayer>,
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
        move_events.send(MovePlayer { direction });
    }
}

pub fn camera_follow(
    player_query: Query<&Transform, (With<Player>, Without<MainCamera>)>,
    mut camera_query: Query<&mut Transform, With<MainCamera>>,
) {
    if let Ok(player_transform) = player_query.get_single() {
        if let Ok(mut camera_transform) = camera_query.get_single_mut() {
            let offset = Vec3::new(0.0, 5.0, 10.0);
            camera_transform.translation = player_transform.translation + offset;
            camera_transform.look_at(player_transform.translation, Vec3::Y);
        }
    }
}
