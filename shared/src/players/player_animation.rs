#[cfg(feature = "client")]
use bevy::animation::AnimationPlayer;
use bevy::prelude::*;
#[cfg(feature = "client")]
use std::time::Duration;

/// Tracks if the player is currently attacking (for animation purposes)
#[cfg(feature = "client")]
#[derive(Component, Default)]
pub struct PlayerAttacking {
    pub is_attacking: bool,
    pub attack_timer: f32,
}

/// Tracks idle time for triggering idle variations
#[cfg(feature = "client")]
#[derive(Component)]
pub struct PlayerIdleTimer {
    pub time_idle: f32,
    pub next_variation_time: f32,
    pub is_playing_variation: bool,
}

#[cfg(feature = "client")]
impl Default for PlayerIdleTimer {
    fn default() -> Self {
        Self {
            time_idle: 0.0,
            next_variation_time: rand_variation_time(),
            is_playing_variation: false,
        }
    }
}

#[cfg(feature = "client")]
fn rand_variation_time() -> f32 {
    // Random time between 3 and 8 seconds
    3.0 + (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos() as f32
        / 1_000_000_000.0)
        * 5.0
}

#[cfg(feature = "client")]
#[derive(Component)]
pub struct PlayerAnimations {
    pub idle: AnimationNodeIndex,
    pub idle_nervous: AnimationNodeIndex,
    pub walking: AnimationNodeIndex,
    pub attacking: AnimationNodeIndex,
}

/// Tracks previous position to compute velocity for animation state
#[cfg(feature = "client")]
#[derive(Component)]
pub struct PlayerPrevPosition(pub Vec3);

/// Links an AnimationPlayer entity back to its root Player entity
#[cfg(feature = "client")]
#[derive(Component)]
pub struct PlayerRoot(pub Entity);

#[cfg(feature = "client")]
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum PlayerAnimationState {
    #[default]
    Idle,
    IdleNervous,
    Walking,
    Attacking,
}

#[cfg(feature = "client")]
pub struct PlayerAnimationConfig {
    pub model_path: &'static str,
    pub idle_animation: AnimationClipConfig,
    pub idle_nervous_animation: AnimationClipConfig,
    pub walking_animation: AnimationClipConfig,
    pub attacking_animation: AnimationClipConfig,
}

#[cfg(feature = "client")]
pub struct AnimationClipConfig {
    pub path: &'static str,
    pub speed: f32,
    pub repeat: bool,
}

#[cfg(feature = "client")]
impl Default for PlayerAnimationConfig {
    fn default() -> Self {
        Self {
            model_path: "player.glb#Scene0",
            idle_animation: AnimationClipConfig {
                path: "player.glb#Animation6", // Ninja Idle
                speed: 1.0,
                repeat: true,
            },
            idle_nervous_animation: AnimationClipConfig {
                path: "player.glb#Animation5", // Nervously Look Around
                speed: 1.0,
                repeat: false, // Play once then return to idle
            },
            walking_animation: AnimationClipConfig {
                path: "player.glb#Animation10", // Standing Run Forward
                speed: 1.0,
                repeat: true,
            },
            attacking_animation: AnimationClipConfig {
                path: "player.glb#Animation7", // Punching
                speed: 1.5,
                repeat: false,
            },
        }
    }
}

#[cfg(feature = "client")]
pub fn setup_player_animation(
    mut commands: Commands,
    mut animation_players: Query<(Entity, &mut AnimationPlayer), Added<AnimationPlayer>>,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    parent_query: Query<&bevy::ecs::hierarchy::ChildOf>,
    player_query: Query<Entity, With<crate::players::player::Player>>,
) {
    let config = PlayerAnimationConfig::default();

    for (entity, mut player) in &mut animation_players {
        // Find the root Player entity by traversing up the parent hierarchy
        let mut player_root = None;
        let mut current = entity;
        while let Ok(child_of) = parent_query.get(current) {
            current = child_of.parent();
            if player_query.get(current).is_ok() {
                player_root = Some(current);
                break;
            }
        }

        // Skip if this AnimationPlayer doesn't belong to a Player
        let Some(player_entity) = player_root else {
            continue;
        };

        let mut graph = AnimationGraph::new();

        let idle_node = graph.add_clip(
            asset_server.load(config.idle_animation.path),
            config.idle_animation.speed,
            graph.root,
        );
        let idle_nervous_node = graph.add_clip(
            asset_server.load(config.idle_nervous_animation.path),
            config.idle_nervous_animation.speed,
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

        let graph_handle = graphs.add(graph);

        // Create AnimationTransitions to manage smooth blending between animations
        let mut transitions = AnimationTransitions::new();

        // Start with idle animation via AnimationTransitions
        transitions
            .play(&mut player, idle_node, Duration::ZERO)
            .repeat();

        commands
            .entity(entity)
            .insert(AnimationGraphHandle(graph_handle))
            .insert(PlayerAnimations {
                idle: idle_node,
                idle_nervous: idle_nervous_node,
                walking: walking_node,
                attacking: attacking_node,
            })
            .insert(PlayerIdleTimer::default())
            .insert(PlayerAnimationState::default())
            .insert(PlayerRoot(player_entity))
            .insert(transitions);
    }
}

#[cfg(feature = "client")]
pub fn update_player_animation_state(
    mut anim_query: Query<(&mut PlayerAnimationState, &PlayerRoot)>,
    player_attacking_query: Query<&PlayerAttacking, With<crate::players::player::Player>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    // Check if movement keys are pressed
    let is_moving = keyboard_input.pressed(KeyCode::KeyW)
        || keyboard_input.pressed(KeyCode::KeyA)
        || keyboard_input.pressed(KeyCode::KeyS)
        || keyboard_input.pressed(KeyCode::KeyD)
        || keyboard_input.pressed(KeyCode::ArrowUp)
        || keyboard_input.pressed(KeyCode::ArrowDown)
        || keyboard_input.pressed(KeyCode::ArrowLeft)
        || keyboard_input.pressed(KeyCode::ArrowRight);

    for (mut anim_state, player_root) in &mut anim_query {
        // Check if player is attacking
        let is_attacking = player_attacking_query
            .get(player_root.0)
            .map(|a| a.is_attacking)
            .unwrap_or(false);

        // Determine animation state (don't override IdleNervous if not moving/attacking)
        let new_state = if is_attacking {
            PlayerAnimationState::Attacking
        } else if is_moving {
            PlayerAnimationState::Walking
        } else if *anim_state == PlayerAnimationState::IdleNervous {
            // Keep IdleNervous until the variation timer resets it
            PlayerAnimationState::IdleNervous
        } else {
            PlayerAnimationState::Idle
        };

        if *anim_state != new_state {
            *anim_state = new_state;
        }
    }
}

/// Tracks the last animation state that was played, to avoid replaying the same animation
#[cfg(feature = "client")]
#[derive(Component, Default, Clone, Copy, PartialEq, Eq)]
pub struct LastPlayedPlayerAnimation(Option<PlayerAnimationState>);

/// Transition duration for blending between animations
#[cfg(feature = "client")]
const ANIMATION_TRANSITION_DURATION: Duration = Duration::from_millis(200);

#[cfg(feature = "client")]
pub fn control_player_animation(
    mut commands: Commands,
    mut animation_players: Query<(
        Entity,
        &mut AnimationPlayer,
        &mut AnimationTransitions,
        &PlayerAnimations,
        &PlayerAnimationState,
        Option<&LastPlayedPlayerAnimation>,
    )>,
) {
    let config = PlayerAnimationConfig::default();

    for (entity, mut player, mut transitions, animations, state, last_played) in
        &mut animation_players
    {
        // Check if we need to play this animation
        let should_play = match last_played {
            Some(last) => last.0 != Some(*state),
            None => true, // Never played anything, must play
        };

        if !should_play {
            continue;
        }

        println!("Playing player animation: {:?}", state);

        // Use AnimationTransitions for smooth blending between animations
        match *state {
            PlayerAnimationState::Idle => {
                let active =
                    transitions.play(&mut player, animations.idle, ANIMATION_TRANSITION_DURATION);
                if config.idle_animation.repeat {
                    active.repeat();
                }
            }
            PlayerAnimationState::IdleNervous => {
                // Play once - doesn't repeat
                transitions.play(
                    &mut player,
                    animations.idle_nervous,
                    ANIMATION_TRANSITION_DURATION,
                );
            }
            PlayerAnimationState::Walking => {
                let active = transitions.play(
                    &mut player,
                    animations.walking,
                    ANIMATION_TRANSITION_DURATION,
                );
                if config.walking_animation.repeat {
                    active.repeat();
                }
            }
            PlayerAnimationState::Attacking => {
                let active = transitions.play(
                    &mut player,
                    animations.attacking,
                    ANIMATION_TRANSITION_DURATION,
                );
                if config.attacking_animation.repeat {
                    active.repeat();
                }
            }
        }

        // Update last played state
        commands
            .entity(entity)
            .insert(LastPlayedPlayerAnimation(Some(*state)));
    }
}

/// Updates the attack timer and resets attacking state when animation finishes
#[cfg(feature = "client")]
pub fn update_player_attack_timer(mut query: Query<&mut PlayerAttacking>, time: Res<Time>) {
    const ATTACK_DURATION: f32 = 0.6; // Duration of attack animation

    for mut attacking in &mut query {
        if attacking.is_attacking {
            attacking.attack_timer += time.delta_secs();
            if attacking.attack_timer >= ATTACK_DURATION {
                attacking.is_attacking = false;
                attacking.attack_timer = 0.0;
            }
        }
    }
}

/// Triggers attack animation when player attacks
#[cfg(feature = "client")]
pub fn trigger_player_attack_animation(
    mut query: Query<&mut PlayerAttacking>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        for mut attacking in &mut query {
            attacking.is_attacking = true;
            attacking.attack_timer = 0.0;
        }
    }
}

/// Triggers idle variations (nervous look around) after being idle for a while
#[cfg(feature = "client")]
pub fn update_player_idle_variations(
    mut anim_query: Query<(&mut PlayerAnimationState, &PlayerRoot, &mut PlayerIdleTimer)>,
    time: Res<Time>,
) {
    const NERVOUS_DURATION: f32 = 2.5; // Duration of nervous look animation

    for (mut anim_state, _player_root, mut idle_timer) in &mut anim_query {
        match *anim_state {
            PlayerAnimationState::Idle => {
                // Increment idle time
                idle_timer.time_idle += time.delta_secs();

                // Check if it's time to play a variation
                if idle_timer.time_idle >= idle_timer.next_variation_time {
                    *anim_state = PlayerAnimationState::IdleNervous;
                    idle_timer.is_playing_variation = true;
                    idle_timer.time_idle = 0.0;
                }
            }
            PlayerAnimationState::IdleNervous => {
                // Wait for animation to finish, then return to idle
                idle_timer.time_idle += time.delta_secs();
                if idle_timer.time_idle >= NERVOUS_DURATION {
                    *anim_state = PlayerAnimationState::Idle;
                    idle_timer.is_playing_variation = false;
                    idle_timer.time_idle = 0.0;
                    idle_timer.next_variation_time = rand_variation_time();
                }
            }
            _ => {
                // Reset idle timer when not idle
                idle_timer.time_idle = 0.0;
                idle_timer.is_playing_variation = false;
            }
        }
    }
}
