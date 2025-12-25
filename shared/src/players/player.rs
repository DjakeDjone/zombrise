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

#[derive(Component, Serialize, Deserialize, Reflect, Default)]
pub struct DamageFlash {
    pub timer: f32,
}

#[derive(Component, Serialize, Deserialize, Reflect)]
pub struct PlayerOwner(pub u64);

#[derive(Component)]
pub struct MainCamera;

#[derive(Component, Default)]
pub struct PlayerAttackCooldown(pub f32);

/// Resource to track the local client's ID
#[cfg(feature = "client")]
#[derive(bevy::prelude::Resource, Default)]
pub struct MyClientId(pub u64);

/// Component to store the predicted local position of the player to avoid jitter from server updates
#[cfg(feature = "client")]
#[derive(Component, Default)]
pub struct LocalPlayerPosition(pub Vec3);

/// Component to store the predicted local rotation of the player
#[cfg(feature = "client")]
#[derive(Component, Default)]
pub struct LocalPlayerRotation(pub bevy::math::Quat);

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
#[cfg(feature = "client")]
pub fn handle_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    suduxu_input: Option<Res<ButtonInput<crate::suduxu::SuduxuButton>>>,
    mut move_events: bevy::prelude::MessageWriter<MovePlayer>,
    mut attack_events: bevy::prelude::MessageWriter<PlayerAttack>,
    camera_rotation: Option<Res<CameraRotation>>,
    player_attacking_query: Query<(&PlayerAttacking, &PlayerOwner)>,
    my_client_id: Res<MyClientId>,
    mut local_player_query: Query<
        (
            &mut bevy::prelude::Transform,
            Option<&mut LocalPlayerPosition>,
            Option<&mut LocalPlayerRotation>,
            &PlayerOwner,
        ),
        bevy::prelude::With<Player>,
    >,
) {
    let mut direction = Vec3::ZERO;

    // Check if local player is attacking
    let is_attacking = player_attacking_query
        .iter()
        .any(|(a, owner)| owner.0 == my_client_id.0 && a.is_attacking);

    // Helpers to check suduxu input safely
    let suduxu_pressed = |btn| suduxu_input.as_ref().map_or(false, |s| s.pressed(btn));
    let suduxu_just_pressed = |btn| suduxu_input.as_ref().map_or(false, |s| s.just_pressed(btn));

    if !is_attacking {
        let up_key =
            keyboard_input.pressed(KeyCode::ArrowUp) || keyboard_input.pressed(KeyCode::KeyW);
        let down_key =
            keyboard_input.pressed(KeyCode::ArrowDown) || keyboard_input.pressed(KeyCode::KeyS);
        let left_key =
            keyboard_input.pressed(KeyCode::ArrowLeft) || keyboard_input.pressed(KeyCode::KeyA);
        let right_key =
            keyboard_input.pressed(KeyCode::ArrowRight) || keyboard_input.pressed(KeyCode::KeyD);

        let up_suduxu = suduxu_pressed(crate::suduxu::SuduxuButton::Up);
        let down_suduxu = suduxu_pressed(crate::suduxu::SuduxuButton::Down);
        let left_suduxu = suduxu_pressed(crate::suduxu::SuduxuButton::Left);
        let right_suduxu = suduxu_pressed(crate::suduxu::SuduxuButton::Right);

        if up_key || up_suduxu {
            direction.z -= 1.0;
        }
        if down_key || down_suduxu {
            direction.z += 1.0;
        }
        if left_key || left_suduxu {
            direction.x -= 1.0;
        }
        if right_key || right_suduxu {
            direction.x += 1.0;
        }

        if direction.length() > 0.0 {
            direction = direction.normalize();
            let camera_yaw = camera_rotation.as_ref().map(|r| r.yaw).unwrap_or(0.0);
            move_events.write(MovePlayer {
                direction,
                camera_yaw,
            });
            // Server handles rotation - don't overwrite here
        }
    }

    // Sync local state FROM server (position and rotation come from server via replication)
    // This allows server-side auto-aim rotation to be visible on client
    for (mut transform, local_pos_opt, local_rot_opt, owner) in &mut local_player_query {
        if owner.0 == my_client_id.0 {
            // Sync local_pos from server
            if let Some(mut local_pos) = local_pos_opt {
                local_pos.0 = transform.translation;
            }
            // Sync local_rot from server (so server auto-aim works)
            if let Some(mut local_rot) = local_rot_opt {
                if is_attacking {
                    // PRIORITIZE local prediction when attacking to disable jitter from server updates
                    // The client auto-aim system sets local_rot, so we enforce it on the transform here
                    transform.rotation = local_rot.0;
                } else {
                    local_rot.0 = transform.rotation;
                }
            }
        }
    }

    if keyboard_input.just_pressed(KeyCode::Space)
        || suduxu_just_pressed(crate::suduxu::SuduxuButton::A)
    {
        if !is_attacking {
            // Just send attack - server handles auto-aim rotation
            attack_events.write(PlayerAttack);
        }
    }
}

#[derive(bevy::prelude::Resource)]
pub struct CameraRotation {
    pub yaw: f32,
    pub pitch: f32,
}
