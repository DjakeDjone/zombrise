use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Component, Serialize, Deserialize, Clone, Debug, Reflect, Default)]
#[reflect(Component)]
pub struct Zombie;

#[derive(Component)]
pub struct ZombieAnimations(pub Vec<AnimationNodeIndex>);

pub fn spawn_zombie(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn(SceneBundle {
            scene: asset_server.load("zombie.glb#Scene0"),
            transform: Transform::from_xyz(1.0, 0.0, 1.0).with_scale(Vec3::splat(1.0)),
            ..default()
        })
        .insert(Zombie);
}

pub fn setup_zombie_animation(
    mut commands: Commands,
    mut animation_players: Query<(Entity, &mut AnimationPlayer), Added<AnimationPlayer>>,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    for (entity, mut player) in &mut animation_players {
        let mut graph = AnimationGraph::new();
        let node_indices = vec![
            graph.add_clip(asset_server.load("zombie.glb#Animation11"), 1.0, graph.root),
            graph.add_clip(asset_server.load("zombie.glb#Animation0"), 1.0, graph.root),
            graph.add_clip(asset_server.load("zombie.glb#Animation10"), 1.0, graph.root),
            graph.add_clip(asset_server.load("zombie.glb#Animation12"), 1.0, graph.root),
        ];

        commands.entity(entity).insert(graphs.add(graph));
        commands
            .entity(entity)
            .insert(ZombieAnimations(node_indices.clone()));

        player.play(node_indices[0]).repeat();
    }
}

pub fn control_zombie_animation(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut animation_players: Query<(&mut AnimationPlayer, &ZombieAnimations)>,
) {
    for (mut player, animations) in &mut animation_players {
        if keyboard_input.just_pressed(KeyCode::Digit1) {
            player.play(animations.0[0]).repeat();
        }
        if keyboard_input.just_pressed(KeyCode::Digit2) {
            player.play(animations.0[1]).repeat();
        }
        if keyboard_input.just_pressed(KeyCode::Digit3) {
            player.play(animations.0[2]).repeat();
        }
    }
}
