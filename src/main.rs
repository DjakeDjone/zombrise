use bevy::prelude::*;

use crate::players::player::{MainCamera, Player, camera_follow, move_player};
use crate::zombie::zombie::{control_zombie_animation, setup_zombie_animation, spawn_zombie};

pub mod players;
pub mod zombie;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup, spawn_zombie))
        .add_systems(
            Update,
            (
                move_player,
                camera_follow,
                setup_zombie_animation,
                control_zombie_animation,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        MainCamera,
    ));

    // Light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    let map_radius = 15.0;

    // Ground
    commands.spawn(PbrBundle {
        mesh: meshes.add(Cylinder::new(map_radius, 0.1)),
        material: materials.add(Color::WHITE),
        ..default()
    });

    // Player (Cube)
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
            material: materials.add(Color::srgb(0.8, 0.7, 0.6)),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..default()
        },
        Player,
    ));
}
