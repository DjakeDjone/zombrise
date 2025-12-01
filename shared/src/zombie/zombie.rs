use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Component, Serialize, Deserialize, Clone, Debug, Reflect, Default)]
#[reflect(Component)]
pub struct Zombie;

#[derive(Component, Serialize, Deserialize, Clone, Debug, Reflect, Default, PartialEq, Eq)]
#[reflect(Component)]
pub enum ZombieState {
    #[default]
    Idle,
    Walking,
    Attacking,
    Dead,
}

impl Zombie {
    pub const RADIUS: f32 = 0.5;
    pub const HALF_HEIGHT: f32 = 0.5;
    pub const COLLISION_DAMAGE_RADIUS: f32 = 1.5;

    // Animation Paths
    pub const ANIMATION_PATH_IDLE: &'static str = "zombie.glb#Animation10";
    pub const ANIMATION_PATH_WALK: &'static str = "zombie.glb#Animation0";
    pub const ANIMATION_PATH_ATTACK: &'static str = "zombie.glb#Animation1";
    pub const ANIMATION_PATH_DEATH: &'static str = "zombie.glb#Animation12";
}

#[cfg(feature = "client")]
#[derive(Component)]
pub struct ZombieAnimations(pub Vec<AnimationNodeIndex>);

#[cfg(feature = "client")]
#[derive(Component)]
pub struct CurrentZombieState(pub ZombieState);

// #[cfg(feature = "client")]
// pub fn spawn_zombie(mut commands: Commands, asset_server: Res<AssetServer>) {
//     commands
//         .spawn(SceneBundle {
//             scene: asset_server.load("zombie.glb#Scene0"),
//             transform: Transform::from_xyz(1.0, 0.0, 1.0).with_scale(Vec3::splat(1.0)),
//             ..default()
//         })
//         .insert(Zombie);
// }

#[cfg(feature = "client")]
pub fn setup_zombie_animation(
    mut commands: Commands,
    animation_players: Query<Entity, Added<AnimationPlayer>>,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    for entity in &animation_players {
        let mut graph = AnimationGraph::new();
        let node_indices = vec![
            graph.add_clip(asset_server.load(Zombie::ANIMATION_PATH_IDLE), 1.0, graph.root),
            graph.add_clip(asset_server.load(Zombie::ANIMATION_PATH_WALK), 1.0, graph.root),
            graph.add_clip(asset_server.load(Zombie::ANIMATION_PATH_ATTACK), 1.0, graph.root),
            graph.add_clip(asset_server.load(Zombie::ANIMATION_PATH_DEATH), 1.0, graph.root),
        ];

        commands.entity(entity).insert(graphs.add(graph));
        commands
            .entity(entity)
            .insert(ZombieAnimations(node_indices.clone()));
    }
}

#[cfg(feature = "client")]
pub fn control_zombie_animation(
    mut commands: Commands,
    mut animation_players: Query<(
        Entity,
        &mut AnimationPlayer,
        &ZombieAnimations,
        Option<&mut CurrentZombieState>,
    )>,
    parents: Query<&Parent>,
    zombie_states: Query<&ZombieState>,
) {
    for (entity, mut player, animations, mut current_state) in &mut animation_players {
        // Find the target state
        let mut target_state = ZombieState::Idle; // Default

        // Walk up to find ZombieState
        let mut current_entity = entity;
        loop {
            if let Ok(state) = zombie_states.get(current_entity) {
                target_state = state.clone();
                break;
            }
            
            if let Ok(parent) = parents.get(current_entity) {
                current_entity = parent.get();
            } else {
                break;
            }
        }

        // Check if we need to update
        let update = if let Some(current) = current_state.as_ref() {
            current.0 != target_state
        } else {
            true
        };

        if update {
            let animation_index = match target_state {
                ZombieState::Idle => 0,
                ZombieState::Walking => 1,
                ZombieState::Attacking => 2,
                ZombieState::Dead => 3,
            };

            if animation_index < animations.0.len() {
                player.play(animations.0[animation_index]).repeat();
            }

            if let Some(mut current) = current_state {
                current.0 = target_state;
            } else {
                commands.entity(entity).insert(CurrentZombieState(target_state));
            }
        }
    }
}
