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
    player_transform_query: Query<
        (&bevy::prelude::GlobalTransform, &PlayerOwner),
        bevy::prelude::With<Player>,
    >,
    zombie_query: Query<
        &bevy::prelude::GlobalTransform,
        bevy::prelude::With<crate::zombie::zombie::Zombie>,
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

        if direction.length() == 0.0
            && (up_key
                || down_key
                || left_key
                || right_key
                || up_suduxu
                || down_suduxu
                || left_suduxu
                || right_suduxu)
        {
            println!("Movement cancelled! Inputs - Up(K:{},S:{}) Down(K:{},S:{}) Left(K:{},S:{}) Right(K:{},S:{}) -> Dir:{:?} | Attacking: {}",
                 up_key, up_suduxu, down_key, down_suduxu, left_key, left_suduxu, right_key, right_suduxu, direction, is_attacking);
        } else if direction.length() > 0.0 {
            // println!("Moving: {:?}", direction); // Uncomment for spammy movement logs
        }

        if direction.length() > 0.0 {
            direction = direction.normalize();
            let camera_yaw = camera_rotation.as_ref().map(|r| r.yaw).unwrap_or(0.0);
            move_events.write(MovePlayer {
                direction,
                camera_yaw,
            });
        }
    }

    if keyboard_input.just_pressed(KeyCode::Space)
        || suduxu_just_pressed(crate::suduxu::SuduxuButton::A)
    {
        if !is_attacking {
            // Auto-aim at nearest zombie
            let mut nearest_zombie_pos: Option<Vec3> = None;
            let mut nearest_dist = 20.0; // Max auto-aim distance

            // Find my position
            if let Some((my_transform, _)) = player_transform_query
                .iter()
                .find(|(_, owner)| owner.0 == my_client_id.0)
            {
                let my_pos = my_transform.translation();

                for zombie_transform in zombie_query.iter() {
                    let dist = my_pos.distance(zombie_transform.translation());
                    if dist < nearest_dist {
                        nearest_dist = dist;
                        nearest_zombie_pos = Some(zombie_transform.translation());
                    }
                }

                if let Some(target_pos) = nearest_zombie_pos {
                    let mut direction_to_target = (target_pos - my_pos).normalize_or_zero();
                    direction_to_target.y = 0.0; // Flatten

                    if direction_to_target.length_squared() > 0.0 {
                        direction_to_target = direction_to_target.normalize();

                        // Compensate for camera rotation
                        let camera_yaw = camera_rotation.as_ref().map(|r| r.yaw).unwrap_or(0.0);
                        let rotation_correction = bevy::math::Quat::from_rotation_y(-camera_yaw);
                        let corrected_direction = rotation_correction * direction_to_target;

                        // Send a move event just for rotation (small magnitude to avoid movement)
                        move_events.write(MovePlayer {
                            direction: corrected_direction * 0.02, // Just enough to trigger rotation (> 0.01)
                            camera_yaw,
                        });
                    }
                }
            }

            attack_events.write(PlayerAttack);
        }
    }
}

#[derive(bevy::prelude::Resource)]
pub struct CameraRotation {
    pub yaw: f32,
    pub pitch: f32,
}
