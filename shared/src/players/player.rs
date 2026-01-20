use bevy::{
    ecs::component::Component,
    math::Vec3,
    prelude::{Event, Reflect, ReflectComponent},
};

use serde::{Deserialize, Serialize};

// Shared imports for movement (used by both client and server)
use crate::protocol::GameInput;
use avian3d::prelude::LinearVelocity;
use bevy::ecs::system::Query;
use lightyear::prelude::input::native::ActionState;

#[cfg(feature = "client")]
use bevy::{
    ecs::system::{Res, ResMut},
    input::{keyboard::KeyCode, ButtonInput},
};
#[cfg(feature = "client")]
use lightyear::prelude::input::native::InputMarker;

#[cfg(feature = "client")]
use super::player_animation::PlayerAttacking;

#[derive(Component, Serialize, Deserialize, Reflect, PartialEq, Clone, Debug)]
pub struct Player;

#[derive(Component, Serialize, Deserialize, Reflect, Default, PartialEq, Clone)]
pub struct DamageFlash {
    pub timer: f32,
}

#[derive(Component, Serialize, Deserialize, Reflect, PartialEq, Clone, Debug)]
pub struct PlayerOwner(pub u64);

#[derive(Component)]
pub struct MainCamera;

#[derive(Component, Default)]
pub struct PlayerAttackCooldown(pub f32);

/// Component to track player death sequence.
/// When a player dies, they fall to the ground, then burn before disappearing.
#[derive(Component, Serialize, Deserialize, Clone, Debug, Reflect, Default, PartialEq)]
#[reflect(Component)]
pub struct PlayerDying {
    /// Total time since death started
    pub timer: f32,
    /// Duration of falling phase
    pub fall_duration: f32,
    /// Duration of burning phase
    pub burn_duration: f32,
}

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

#[derive(Event, Serialize, Deserialize)]
pub struct DamagePlayer {
    pub client_id: u64,
    pub amount: f32,
}

#[cfg(feature = "client")]
#[derive(bevy::prelude::Resource, Default)]
pub struct LocalInputState {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub jump: bool,
}

/// System to gather inputs in PreUpdate to ensure they are constrained to the frame
#[cfg(feature = "client")]
pub fn gather_input(
    mut input_state: ResMut<LocalInputState>,
    keyboard_input: Option<Res<ButtonInput<KeyCode>>>,
) {
    if let Some(keyboard_input) = keyboard_input {
        input_state.up =
            keyboard_input.pressed(KeyCode::ArrowUp) || keyboard_input.pressed(KeyCode::KeyW);
        input_state.down =
            keyboard_input.pressed(KeyCode::ArrowDown) || keyboard_input.pressed(KeyCode::KeyS);
        input_state.left =
            keyboard_input.pressed(KeyCode::ArrowLeft) || keyboard_input.pressed(KeyCode::KeyA);
        input_state.right =
            keyboard_input.pressed(KeyCode::ArrowRight) || keyboard_input.pressed(KeyCode::KeyD);
        input_state.jump = keyboard_input.just_pressed(KeyCode::Space);
    }
}

/// Buffer input system for Lightyear
/// This must run in FixedPreUpdate, InputSystems::WriteClientInputs
#[cfg(feature = "client")]
pub fn buffer_input(
    mut query: Query<&mut ActionState<GameInput>, bevy::prelude::With<InputMarker<GameInput>>>,
    input_state: Option<Res<LocalInputState>>, // Use the resource instead of direct input
    #[cfg(not(target_arch = "wasm32"))] suduxu_input: Option<
        Res<ButtonInput<crate::suduxu::SuduxuButton>>,
    >,
    camera_rotation: Option<Res<CameraRotation>>,
    player_attacking_query: Query<(&PlayerAttacking, &PlayerOwner)>,
    my_client_id: Option<Res<MyClientId>>,
) {
    // Return early if required resources don't exist (e.g., on server)
    let Some(input_state) = input_state else {
        return;
    };
    let Some(my_client_id) = my_client_id else {
        return;
    };

    if query.is_empty() {
        // bevy::log::warn!("[buffer_input] Query is empty! No entity with InputMarker.");
        return;
    }

    let Ok(mut action_state) = query.single_mut() else {
        bevy::log::error!("[buffer_input] Query has multiple entities! Expected exactly one.");
        return;
    };

    // bevy::log::info!("[buffer_input] System running for client {}", my_client_id.0);
    let is_attacking = player_attacking_query
        .iter()
        .any(|(a, owner)| owner.0 == my_client_id.0 && a.is_attacking);

    // Helpers to check suduxu input safely
    #[cfg(not(target_arch = "wasm32"))]
    let suduxu_pressed = |btn| suduxu_input.as_ref().is_some_and(|s| s.pressed(btn));
    #[cfg(not(target_arch = "wasm32"))]
    let suduxu_just_pressed = |btn| suduxu_input.as_ref().is_some_and(|s| s.just_pressed(btn));

    let mut direction = Vec3::ZERO;

    if !is_attacking {
        #[cfg(not(target_arch = "wasm32"))]
        let up_suduxu = suduxu_pressed(crate::suduxu::SuduxuButton::Up);
        #[cfg(target_arch = "wasm32")]
        let up_suduxu = false;

        #[cfg(not(target_arch = "wasm32"))]
        let down_suduxu = suduxu_pressed(crate::suduxu::SuduxuButton::Down);
        #[cfg(target_arch = "wasm32")]
        let down_suduxu = false;

        #[cfg(not(target_arch = "wasm32"))]
        let left_suduxu = suduxu_pressed(crate::suduxu::SuduxuButton::Left);
        #[cfg(target_arch = "wasm32")]
        let left_suduxu = false;

        #[cfg(not(target_arch = "wasm32"))]
        let right_suduxu = suduxu_pressed(crate::suduxu::SuduxuButton::Right);
        #[cfg(target_arch = "wasm32")]
        let right_suduxu = false;

        if input_state.up || up_suduxu {
            direction.z -= 1.0;
        }
        if input_state.down || down_suduxu {
            direction.z += 1.0;
        }
        if input_state.left || left_suduxu {
            direction.x -= 1.0;
        }
        if input_state.right || right_suduxu {
            direction.x += 1.0;
        }
    }

    // Set the input action state
    #[cfg(not(target_arch = "wasm32"))]
    let suduxu_attack = suduxu_just_pressed(crate::suduxu::SuduxuButton::A);
    #[cfg(target_arch = "wasm32")]
    let suduxu_attack = false;

    // Send attack input when Space is pressed - don't check is_attacking here
    // because the animation state is set in the same frame before buffer_input runs.
    // The server has its own cooldown check.
    if input_state.jump || suduxu_attack {
        action_state.0 = GameInput::Attack;
    } else if direction.length() > 0.0 && !is_attacking {
        let dir_norm = direction.normalize();
        let camera_yaw = camera_rotation.as_ref().map(|r| r.yaw).unwrap_or(0.0);
        action_state.0 = GameInput::Move {
            direction: bevy::math::Vec2::new(dir_norm.x, dir_norm.z),
            yaw: camera_yaw,
        };
    } else {
        action_state.0 = GameInput::None;
    }
}

/// Sync local state from server updates
#[cfg(feature = "client")]
pub fn sync_local_state(
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
    let is_attacking = player_attacking_query
        .iter()
        .any(|(a, owner)| owner.0 == my_client_id.0 && a.is_attacking);

    for (mut transform, local_pos_opt, local_rot_opt, owner) in &mut local_player_query {
        if owner.0 == my_client_id.0 {
            if let Some(mut local_pos) = local_pos_opt {
                local_pos.0 = transform.translation;
            }
            if let Some(mut local_rot) = local_rot_opt {
                if is_attacking {
                    transform.rotation = local_rot.0;
                } else {
                    local_rot.0 = transform.rotation;
                }
            }
        }
    }
}

#[derive(bevy::prelude::Resource)]
pub struct CameraRotation {
    pub yaw: f32,
    pub pitch: f32,
}

#[cfg(feature = "client")]
pub use lightyear::prelude::client::input::InputSystems;

/// Shared player movement system for Client-Side Prediction and Server Authority
/// Must run in FixedUpdate on both client and server.
pub fn handle_player_movement(
    mut query: Query<
        (
            &mut LinearVelocity,
            &mut bevy::prelude::Transform,
            &ActionState<GameInput>,
        ),
        bevy::prelude::With<Player>,
    >,
) {
    let speed = 3.0;
    let damping_factor = 0.85; // Smooth deceleration
    let velocity_threshold = 0.02; // Minimum velocity to prevent micro-movements

    for (mut velocity, mut transform, action_state) in &mut query {
        match &action_state.0 {
            GameInput::Move { direction, yaw } => {
                let yaw_rotation = bevy::math::Quat::from_rotation_y(*yaw);
                let input_dir = Vec3::new(direction.x, 0.0, direction.y);
                let rotated_direction = yaw_rotation * input_dir;

                velocity.x = rotated_direction.x * speed;
                velocity.z = rotated_direction.z * speed;

                let horizontal_direction = Vec3::new(rotated_direction.x, 0.0, rotated_direction.z);
                if horizontal_direction.length() > 0.01 {
                    let target_rotation = bevy::math::Quat::from_rotation_arc(
                        Vec3::NEG_Z,
                        horizontal_direction.normalize(),
                    );
                    transform.rotation = target_rotation;
                }
            }
            GameInput::None => {
                // Apply dampening for smooth deceleration
                velocity.x *= damping_factor;
                velocity.z *= damping_factor;

                // Stop completely if velocity is below threshold
                if velocity.x.abs() < velocity_threshold {
                    velocity.x = 0.0;
                }
                if velocity.z.abs() < velocity_threshold {
                    velocity.z = 0.0;
                }
            }
            _ => {}
        }
    }
}
