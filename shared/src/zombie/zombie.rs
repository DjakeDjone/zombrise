#[cfg(feature = "client")]
use bevy::animation::{AnimationEvent, AnimationEventTrigger};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Component, Serialize, Deserialize, Clone, Debug, Reflect, Default)]
#[reflect(Component)]
pub struct Zombie;

pub const ZOMBIE_SPEED: f32 = 0.5;
// Reduced from 4.0 to 2.0 to match movement speed better and reduce "tear back" effect from root motion
pub const ZOMBIE_ANIMATION_SPEED_MULTIPLIER: f32 = 2.0;

#[cfg(feature = "client")]
#[derive(Component)]
pub struct ZombieAnimations {
    pub idle: AnimationNodeIndex,
    pub walking: AnimationNodeIndex,
    pub attacking: AnimationNodeIndex,
    pub dying: AnimationNodeIndex,
}

#[cfg(feature = "client")]
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ZombieAnimationState {
    Idle,
    Walking,
    Attacking,
    Dying,
}

#[cfg(feature = "client")]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Reflect, Serialize, Deserialize, Message)]
pub enum ZombieAnimationEvent {
    Footstep,
    AttackHit,
}

#[cfg(feature = "client")]
impl Event for ZombieAnimationEvent {
    type Trigger<'a> = AnimationEventTrigger;
}

#[cfg(feature = "client")]
impl AnimationEvent for ZombieAnimationEvent {}

#[cfg(feature = "client")]
#[derive(Resource, Default)]
pub struct ZombieAnimationEventsState {
    pub events_added: bool,
}

#[cfg(feature = "client")]
impl Default for ZombieAnimationState {
    fn default() -> Self {
        Self::Idle
    }
}

#[cfg(feature = "client")]
#[cfg(feature = "client")]
pub struct ZombieAnimationConfig {
    pub model_path: &'static str,
    pub idle_animation: AnimationClipConfig,
    pub walking_animation: AnimationClipConfig,
    pub attacking_animation: AnimationClipConfig,
    pub dying_animation: AnimationClipConfig,
}

#[cfg(feature = "client")]
pub struct AnimationClipConfig {
    pub path: &'static str,
    pub speed: f32,
    pub repeat: bool,
}

#[cfg(feature = "client")]
impl Default for ZombieAnimationConfig {
    fn default() -> Self {
        Self {
            model_path: "zombie.glb#Scene0",
            idle_animation: AnimationClipConfig {
                path: "zombie.glb#Animation0",
                speed: 1.0,
                repeat: true,
            },
            walking_animation: AnimationClipConfig {
                path: "zombie.glb#Animation11",
                speed: ZOMBIE_SPEED * ZOMBIE_ANIMATION_SPEED_MULTIPLIER,
                repeat: true,
            },
            attacking_animation: AnimationClipConfig {
                path: "zombie.glb#Animation10",
                speed: 1.2,
                repeat: true,
            },
            dying_animation: AnimationClipConfig {
                path: "zombie.glb#Animation12",
                speed: 1.0,
                repeat: false,
            },
        }
    }
}

#[cfg(feature = "client")]
pub fn spawn_zombie(mut commands: Commands, asset_server: Res<AssetServer>) {
    let config = ZombieAnimationConfig::default();
    commands.spawn((
        SceneRoot(asset_server.load(config.model_path)),
        Transform::from_xyz(1.0, 0.0, 1.0).with_scale(Vec3::splat(1.0)),
        Zombie,
    ));
}

#[cfg(feature = "client")]
pub fn setup_zombie_animation(
    mut commands: Commands,
    mut animation_players: Query<(Entity, &mut AnimationPlayer), Added<AnimationPlayer>>,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let config = ZombieAnimationConfig::default();

    for (entity, mut player) in &mut animation_players {
        let mut graph = AnimationGraph::new();

        let idle_node = graph.add_clip(
            asset_server.load(config.idle_animation.path),
            config.idle_animation.speed,
            graph.root,
        );
        let walking_node = graph.add_clip(
            asset_server.load(config.walking_animation.path),
            config.walking_animation.speed,
            graph.root,
        );
        let attacking_node = graph.add_clip(
            asset_server.load(config.attacking_animation.path),
            config.attacking_animation.speed,
            graph.root,
        );
        let dying_node = graph.add_clip(
            asset_server.load(config.dying_animation.path),
            config.dying_animation.speed,
            graph.root,
        );

        commands
            .entity(entity)
            .insert(AnimationGraphHandle(graphs.add(graph)));
        commands.entity(entity).insert(ZombieAnimations {
            idle: idle_node,
            walking: walking_node,
            attacking: attacking_node,
            dying: dying_node,
        });
        commands
            .entity(entity)
            .insert(ZombieAnimationState::default());

        // Start with idle animation
        player.play(idle_node).repeat();
    }
}

#[cfg(feature = "client")]
pub fn update_zombie_animation_state(
    mut zombie_query: Query<(&mut ZombieAnimationState, &GlobalTransform), With<Zombie>>,
    player_query: Query<&GlobalTransform, With<crate::players::player::Player>>,
) {
    const CHASE_RANGE: f32 = 10.0;
    const ATTACK_RANGE: f32 = 1.5;

    for (mut anim_state, zombie_transform) in &mut zombie_query {
        let zombie_pos = zombie_transform.translation();

        // Find nearest player
        let mut nearest_distance = f32::MAX;
        for player_transform in &player_query {
            let distance = zombie_pos.distance(player_transform.translation());
            if distance < nearest_distance {
                nearest_distance = distance;
            }
        }

        // Determine animation state based on distance to nearest player
        let new_state = if nearest_distance < ATTACK_RANGE {
            ZombieAnimationState::Attacking
        } else if nearest_distance < CHASE_RANGE {
            ZombieAnimationState::Walking
        } else {
            ZombieAnimationState::Idle
        };

        if *anim_state != new_state {
            *anim_state = new_state;
        }
    }
}

#[cfg(feature = "client")]
pub fn control_zombie_animation(
    mut animation_players: Query<
        (
            &mut AnimationPlayer,
            &ZombieAnimations,
            &ZombieAnimationState,
        ),
        Changed<ZombieAnimationState>,
    >,
) {
    let config = ZombieAnimationConfig::default();

    for (mut player, animations, state) in &mut animation_players {
        match *state {
            ZombieAnimationState::Idle => {
                if config.idle_animation.repeat {
                    player.play(animations.idle).repeat();
                } else {
                    player.play(animations.idle);
                }
            }
            ZombieAnimationState::Walking => {
                if config.walking_animation.repeat {
                    player.play(animations.walking).repeat();
                } else {
                    player.play(animations.walking);
                }
            }
            ZombieAnimationState::Attacking => {
                if config.attacking_animation.repeat {
                    player.play(animations.attacking).repeat();
                } else {
                    player.play(animations.attacking);
                }
            }
            ZombieAnimationState::Dying => {
                if config.dying_animation.repeat {
                    player.play(animations.dying).repeat();
                } else {
                    player.play(animations.dying);
                }
            }
        }
    }
}

#[cfg(feature = "client")]
pub fn add_zombie_animation_events(
    mut events_state: ResMut<ZombieAnimationEventsState>,
    asset_server: Res<AssetServer>,
    mut clips: ResMut<Assets<AnimationClip>>,
) {
    if events_state.events_added {
        return;
    }

    let config = ZombieAnimationConfig::default();

    // Check if clips are loaded
    let walking_handle = asset_server.load(config.walking_animation.path);
    let attacking_handle = asset_server.load(config.attacking_animation.path);

    if let Some(clip) = clips.get_mut(&walking_handle) {
        // Add footsteps at 0.0s and 0.5s (assuming 1s loop for simplicity, adjust as needed)
        clip.add_event(0.2, ZombieAnimationEvent::Footstep);
        clip.add_event(0.7, ZombieAnimationEvent::Footstep);
        events_state.events_added = true; // Mark as done (at least for walking)
        println!("Added footstep events to walking animation");
    }

    if let Some(clip) = clips.get_mut(&attacking_handle) {
        // Add attack hit at 0.5s
        clip.add_event(0.5, ZombieAnimationEvent::AttackHit);
        println!("Added attack hit event to attacking animation");
    }
}

#[cfg(feature = "client")]
pub fn handle_zombie_animation_events(mut animation_events: MessageReader<ZombieAnimationEvent>) {
    for event in animation_events.read() {
        match event {
            ZombieAnimationEvent::Footstep => {
                println!("Zombie Footstep");
            }
            ZombieAnimationEvent::AttackHit => {
                println!("Zombie Attack Hit");
            }
        }
    }
}
