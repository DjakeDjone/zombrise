//! Camera setup and control systems.

use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

use zombrise_shared::players::player::{
    CameraRotation, MainCamera, MyClientId, Player, PlayerOwner,
};

/// Camera sensitivity constant
pub const CAMERA_SENSITIVITY: f32 = 0.003;
pub const PITCH_LIMIT: f32 = 1.5;
pub const CAMERA_DISTANCE: f32 = 10.0;

/// Setup cameras (3D game camera and 2D UI camera)
pub fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Camera {
            order: 0,
            is_active: false,
            clear_color: ClearColorConfig::Custom(Color::srgb(0.64, 0.74, 0.88)),
            ..default()
        },
        Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        MainCamera,
        // Atmospheric winter fog - gives depth to the snowy landscape
        DistanceFog {
            color: Color::srgba(0.70, 0.75, 0.82, 1.0), // Cold, bluish-grey fog
            falloff: FogFalloff::Linear {
                start: 25.0, // No fog closer than this
                end: 100.0,  // Full fog at this distance
            },
            ..default()
        },
    ));

    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::Custom(Color::srgb(0.15, 0.15, 0.2)),
            ..default()
        },
        IsDefaultUiCamera,
    ));
}

/// Activate game cameras when entering playing state
pub fn activate_game_cameras(
    mut camera_3d_query: Query<&mut Camera, With<MainCamera>>,
    mut camera_2d_query: Query<&mut Camera, (With<Camera2d>, Without<MainCamera>)>,
) {
    // Activate 3D camera
    if let Ok(mut camera) = camera_3d_query.single_mut() {
        camera.is_active = true;
    }

    // Set UI transparent
    if let Ok(mut camera) = camera_2d_query.single_mut() {
        camera.clear_color = ClearColorConfig::None;
    }
}

/// Camera follows the local player
pub fn camera_follow(
    player_query: Query<(&Transform, &PlayerOwner), (With<Player>, Without<MainCamera>)>,
    mut camera_query: Query<&mut Transform, With<MainCamera>>,
    my_client_id: Option<Res<MyClientId>>,
    camera_rotation: Res<CameraRotation>,
) {
    let Some(my_client_id) = my_client_id else {
        return;
    };
    for (player_transform, owner) in player_query.iter() {
        if owner.0 == my_client_id.0 {
            if let Ok(mut camera_transform) = camera_query.single_mut() {
                // Calculate camera offset using yaw and pitch
                let yaw = camera_rotation.yaw;
                let pitch = camera_rotation.pitch;

                // Calculate the offset vector from yaw and pitch
                let offset = Vec3::new(
                    CAMERA_DISTANCE * pitch.cos() * yaw.sin(),
                    2.0,
                    CAMERA_DISTANCE * pitch.cos() * yaw.cos(),
                );

                camera_transform.translation = player_transform.translation + offset;
                camera_transform.look_at(player_transform.translation, Vec3::Y);
            }
        }
    }
}

/// Handle camera rotation from mouse movement
pub fn handle_camera_rotation(
    mut mouse_motion: EventReader<MouseMotion>,
    mut camera_rotation: ResMut<CameraRotation>,
) {
    for motion in mouse_motion.read() {
        camera_rotation.yaw -= motion.delta.x * CAMERA_SENSITIVITY;
        camera_rotation.pitch = (camera_rotation.pitch - motion.delta.y * CAMERA_SENSITIVITY)
            .clamp(-PITCH_LIMIT, PITCH_LIMIT);
    }
}

/// Lock cursor when entering playing state
pub fn lock_cursor(mut cursor_query: Query<&mut CursorOptions, With<PrimaryWindow>>) {
    if let Ok(mut options) = cursor_query.single_mut() {
        options.grab_mode = CursorGrabMode::Locked;
        options.visible = false;
    }
}

/// Handle escape key to release cursor
pub fn handle_escape_key(
    keys: Res<ButtonInput<KeyCode>>,
    mut cursor_query: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        if let Ok(mut options) = cursor_query.single_mut() {
            options.grab_mode = CursorGrabMode::None;
            options.visible = true;
        }
    }
}

/// Handle L key to lock cursor
pub fn handle_lock_key(
    keys: Res<ButtonInput<KeyCode>>,
    mut cursor_query: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if keys.just_pressed(KeyCode::KeyL) {
        if let Ok(mut options) = cursor_query.single_mut() {
            options.grab_mode = CursorGrabMode::Locked;
            options.visible = false;
        }
    }
}
