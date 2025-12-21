#[cfg(feature = "client")]
use bevy::animation::{AnimationEvent, AnimationEventTrigger};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(feature = "client")]
use std::time::Duration;

#[derive(Component, Serialize, Deserialize, Clone, Debug, Reflect, Default)]
#[reflect(Component)]
pub struct Zombie;

#[derive(Component, Serialize, Deserialize, Reflect, Default)]
pub struct ZombieDamageFlash {
    pub timer: f32,
}

pub const ZOMBIE_SPEED: f32 = 2.0;
pub const ZOMBIE_ANIMATION_SPEED_MULTIPLIER: f32 = 1.0;

#[cfg(feature = "client")]
#[derive(Component, Clone, Debug)]
pub struct ZombieAnimations {
    pub idle: AnimationNodeIndex,
    pub walking: AnimationNodeIndex,
    pub running: AnimationNodeIndex,
    pub attacking: AnimationNodeIndex,
    pub dying: AnimationNodeIndex,
    pub hit: AnimationNodeIndex,
}

#[cfg(feature = "client")]
#[derive(Component, Default)]
pub struct ZombieUpdateTimer(pub f32);

/// Previous position
#[cfg(feature = "client")]
#[derive(Component)]
pub struct ZombiePrevPosition(pub Vec3);

/// Link to root
#[cfg(feature = "client")]
#[derive(Component)]
pub struct ZombieRoot(pub Entity);

#[cfg(feature = "client")]
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ZombieAnimationState {
    Idle,
    Walking,
    Running,
    Attacking,
    Dying,
    Hit,
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
pub struct ZombieAnimationConfig {
    pub model_path: &'static str,
    pub idle_animation: AnimationClipConfig,
    pub walking_animation: AnimationClipConfig,
    pub running_animation: AnimationClipConfig,
    pub attacking_animation: AnimationClipConfig,
    pub dying_animation: AnimationClipConfig,
    pub hit_animation: AnimationClipConfig,
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
                path: "zombie.glb#Animation3", // Idle
                speed: 1.0,
                repeat: true,
            },
            walking_animation: AnimationClipConfig {
                path: "zombie.glb#Animation7", // Walk
                speed: ZOMBIE_SPEED * ZOMBIE_ANIMATION_SPEED_MULTIPLIER,
                repeat: true,
            },
            running_animation: AnimationClipConfig {
                path: "zombie.glb#Animation5", // Running
                speed: ZOMBIE_SPEED * ZOMBIE_ANIMATION_SPEED_MULTIPLIER * 1.5,
                repeat: true,
            },
            attacking_animation: AnimationClipConfig {
                path: "zombie.glb#Animation0", // Attack
                speed: 1.2,
                repeat: true,
            },
            dying_animation: AnimationClipConfig {
                path: "zombie.glb#Animation2", // Death
                speed: 1.0,
                repeat: false,
            },
            hit_animation: AnimationClipConfig {
                path: "zombie.glb#Animation4", // Punching (used as hit reaction)
                speed: 1.5,
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
#[derive(Clone, Debug)]
pub struct ZombieAnimationGraph {
    pub handle: Handle<AnimationGraph>,
    pub animations: ZombieAnimations,
}

#[cfg(feature = "client")]
pub fn setup_zombie_animation(
    mut commands: Commands,
    mut animation_players: Query<(Entity, &mut AnimationPlayer), Added<AnimationPlayer>>,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    parent_query: Query<&ChildOf>,
    zombie_query: Query<Entity, With<Zombie>>,
    mut graph_cache: Local<Option<ZombieAnimationGraph>>,
) {
    let config = ZombieAnimationConfig::default();

    // Initialize graph if not cached
    if graph_cache.is_none() {
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
        let running_node = graph.add_clip(
            asset_server.load(config.running_animation.path),
            config.running_animation.speed,
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
        let hit_node = graph.add_clip(
            asset_server.load(config.hit_animation.path),
            config.hit_animation.speed,
            graph.root,
        );

        let graph_handle = graphs.add(graph);

        *graph_cache = Some(ZombieAnimationGraph {
            handle: graph_handle,
            animations: ZombieAnimations {
                idle: idle_node,
                walking: walking_node,
                running: running_node,
                attacking: attacking_node,
                dying: dying_node,
                hit: hit_node,
            },
        });
    }

    let Some(cached_graph) = graph_cache.as_ref() else {
        return;
    };

    for (entity, mut player) in &mut animation_players {
        // Find root zombie
        let mut zombie_root = None;
        let mut current = entity;
        while let Ok(child_of) = parent_query.get(current) {
            current = child_of.parent();
            if zombie_query.get(current).is_ok() {
                zombie_root = Some(current);
                break;
            }
        }

        // Skip if not zombie
        let Some(zombie_entity) = zombie_root else {
            continue;
        };

        // Smooth transitions
        let mut transitions = AnimationTransitions::new();

        // Start idle
        transitions
            .play(&mut player, cached_graph.animations.idle, Duration::ZERO)
            .repeat();

        commands
            .entity(entity)
            .insert(AnimationGraphHandle(cached_graph.handle.clone()))
            .insert(cached_graph.animations.clone())
            .insert(ZombieAnimationState::default())
            .insert(ZombieUpdateTimer(rand::random::<f32>())) // Random offset
            .insert(ZombieRoot(zombie_entity))
            .insert(transitions);
    }
}

#[cfg(feature = "client")]
pub fn update_zombie_animation_state(
    mut commands: Commands,
    time: Res<Time>,
    mut anim_query: Query<(
        Entity,
        &mut ZombieAnimationState,
        &ZombieRoot,
        Option<&ZombiePrevPosition>,
        &mut ZombieUpdateTimer,
    )>,
    zombie_query: Query<(&GlobalTransform, Option<&ZombieDamageFlash>), With<Zombie>>,
    player_query: Query<&GlobalTransform, With<crate::players::player::Player>>,
) {
    const CHASE_RANGE: f32 = 10.0;
    const ATTACK_RANGE: f32 = 1.5;
    const MOVEMENT_THRESHOLD: f32 = 0.01; // Min velocity
    const UPDATE_INTERVAL: f32 = 0.2; // Check every 200ms

    for (entity, mut anim_state, zombie_root, prev_pos, mut timer) in &mut anim_query {
        // Always update previous position for velocity calculation every frame?
        // Actually, to correctly calculate velocity, we need per-frame updates or time-delta awareness.
        // Let's keep position update per frame but AI decision throttled.

        // Get transform and damage flash from root zombie
        let Ok((zombie_transform, damage_flash)) = zombie_query.get(zombie_root.0) else {
            continue;
        };
        let zombie_pos = zombie_transform.translation();

        // Check if zombie is being hit (damage flash is active)
        let is_hit = damage_flash.map(|df| df.timer > 0.0).unwrap_or(false);

        // Compute velocity
        let is_moving = if let Some(prev) = prev_pos {
            let velocity = (zombie_pos - prev.0).length();
            velocity > MOVEMENT_THRESHOLD
        } else {
            false
        };

        // Update prev pos
        commands
            .entity(entity)
            .insert(ZombiePrevPosition(zombie_pos));

        // Throttle expensive distance checks
        timer.0 += time.delta_secs();
        if timer.0 < UPDATE_INTERVAL && !is_hit {
            // If hit, react immediately
            continue;
        }
        timer.0 = 0.0;

        // Find nearest player
        let mut nearest_distance = f32::MAX;
        for player_transform in &player_query {
            let distance = zombie_pos.distance(player_transform.translation());
            if distance < nearest_distance {
                nearest_distance = distance;
            }
        }

        // Determine state - Hit takes priority
        let new_state = if is_hit {
            ZombieAnimationState::Hit
        } else if nearest_distance < ATTACK_RANGE {
            ZombieAnimationState::Attacking
        } else if nearest_distance < CHASE_RANGE {
            ZombieAnimationState::Running // Chasing=running
        } else if is_moving {
            ZombieAnimationState::Walking // Wandering=walking
        } else {
            ZombieAnimationState::Idle // Stand=idle
        };

        if *anim_state != new_state {
            *anim_state = new_state;
        }
    }
}

/// Transition duration
#[cfg(feature = "client")]
const ANIMATION_TRANSITION_DURATION: Duration = Duration::from_millis(200);

#[cfg(feature = "client")]
pub fn control_zombie_animation(
    mut animation_players: Query<
        (
            &mut AnimationPlayer,
            &mut AnimationTransitions,
            &ZombieAnimations,
            &ZombieAnimationState,
        ),
        Changed<ZombieAnimationState>,
    >,
) {
    let config = ZombieAnimationConfig::default();

    for (mut player, mut transitions, animations, state) in &mut animation_players {
        // Smooth blend
        match *state {
            ZombieAnimationState::Idle => {
                let active =
                    transitions.play(&mut player, animations.idle, ANIMATION_TRANSITION_DURATION);
                if config.idle_animation.repeat {
                    active.repeat();
                }
            }
            ZombieAnimationState::Walking => {
                let active = transitions.play(
                    &mut player,
                    animations.walking,
                    ANIMATION_TRANSITION_DURATION,
                );
                if config.walking_animation.repeat {
                    active.repeat();
                }
            }
            ZombieAnimationState::Running => {
                let active = transitions.play(
                    &mut player,
                    animations.running,
                    ANIMATION_TRANSITION_DURATION,
                );
                if config.running_animation.repeat {
                    active.repeat();
                }
            }
            ZombieAnimationState::Attacking => {
                let active = transitions.play(
                    &mut player,
                    animations.attacking,
                    ANIMATION_TRANSITION_DURATION,
                );
                if config.attacking_animation.repeat {
                    active.repeat();
                }
            }
            ZombieAnimationState::Dying => {
                let active =
                    transitions.play(&mut player, animations.dying, ANIMATION_TRANSITION_DURATION);
                if config.dying_animation.repeat {
                    active.repeat();
                }
            }
            ZombieAnimationState::Hit => {
                let active =
                    transitions.play(&mut player, animations.hit, ANIMATION_TRANSITION_DURATION);
                if config.hit_animation.repeat {
                    active.repeat();
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

    // Check clips
    let walking_handle = asset_server.load(config.walking_animation.path);
    let attacking_handle = asset_server.load(config.attacking_animation.path);

    if let Some(clip) = clips.get_mut(&walking_handle) {
        // Add footsteps (0.0s, 0.5s)
        clip.add_event(0.2, ZombieAnimationEvent::Footstep);
        clip.add_event(0.7, ZombieAnimationEvent::Footstep);
        events_state.events_added = true; // Mark done
        println!("Added footstep events to walking animation");
    }

    if let Some(clip) = clips.get_mut(&attacking_handle) {
        // Add attack hit (0.5s)
        clip.add_event(0.5, ZombieAnimationEvent::AttackHit);
        println!("Added attack hit event to attacking animation");
    }
}

#[cfg(feature = "client")]
pub fn handle_zombie_animation_events(mut animation_events: MessageReader<ZombieAnimationEvent>) {
    // Process events but don't print
    for _event in animation_events.read() {
        // Handle specific events if needed regarding logic, but remove spammy logs
    }
}
