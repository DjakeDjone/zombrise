#[cfg(feature = "client")]
use bevy::animation::AnimationPlayer;
use bevy::prelude::*;
#[cfg(feature = "client")]
use std::time::Duration;

/// Tracks attacking state
#[cfg(feature = "client")]
#[derive(Component, Default)]
pub struct PlayerAttacking {
    pub is_attacking: bool,
    pub attack_timer: f32,
}

/// Tracks idle time
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
    // Random 3-8s
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

/// Previous position
#[cfg(feature = "client")]
#[derive(Component)]
pub struct PlayerPrevPosition(pub Vec3);

/// Link to root
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
                path: "player.glb#Animation5", // Nervously

                speed: 1.0,
                repeat: false, // Play once
            },
            walking_animation: AnimationClipConfig {
                path: "player.glb#Animation10", // Stand Run
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
        // Find root player
        let mut player_root = None;
        let mut current = entity;
        while let Ok(child_of) = parent_query.get(current) {
            current = child_of.parent();
            if player_query.get(current).is_ok() {
                player_root = Some(current);
                break;
            }
        }

        // Skip if not player
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

        // Smooth transitions
        let mut transitions = AnimationTransitions::new();

        // Start idle
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
    mut anim_query: Query<(
        &mut PlayerAnimationState,
        &PlayerRoot,
        Option<&mut PlayerPrevPosition>,
    )>,
    player_query: Query<
        (
            &crate::players::player::PlayerOwner,
            &bevy::transform::components::Transform,
        ),
        With<crate::players::player::Player>,
    >,
    player_attacking_query: Query<&PlayerAttacking, With<crate::players::player::Player>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    suduxu_input: Option<Res<ButtonInput<crate::suduxu::SuduxuButton>>>,
    my_client_id: Option<Res<crate::players::player::MyClientId>>,
) {
    // Check local movement input
    let is_moving_input = keyboard_input.pressed(KeyCode::KeyW)
        || keyboard_input.pressed(KeyCode::KeyA)
        || keyboard_input.pressed(KeyCode::KeyS)
        || keyboard_input.pressed(KeyCode::KeyD)
        || keyboard_input.pressed(KeyCode::ArrowUp)
        || keyboard_input.pressed(KeyCode::ArrowDown)
        || keyboard_input.pressed(KeyCode::ArrowLeft)
        || keyboard_input.pressed(KeyCode::ArrowRight)
        || suduxu_input.as_ref().map_or(false, |s| {
            s.pressed(crate::suduxu::SuduxuButton::Up)
                || s.pressed(crate::suduxu::SuduxuButton::Down)
                || s.pressed(crate::suduxu::SuduxuButton::Left)
                || s.pressed(crate::suduxu::SuduxuButton::Right)
        });

    let local_client_id = my_client_id.map(|id| id.0).unwrap_or(0);

    for (mut anim_state, player_root, prev_position) in &mut anim_query {
        // Get player info for this animation entity
        let Ok((owner, transform)) = player_query.get(player_root.0) else {
            continue;
        };

        let is_local_player = local_client_id != 0 && owner.0 == local_client_id;

        // Check attacking
        let is_attacking = player_attacking_query
            .get(player_root.0)
            .map(|a| a.is_attacking)
            .unwrap_or(false);

        // Determine if player is moving
        let is_moving = if is_local_player {
            // For local player, use keyboard input
            is_moving_input
        } else {
            // For remote players, check position changes
            if let Some(prev_pos) = prev_position {
                let distance_moved = transform.translation.distance(prev_pos.0);
                distance_moved > 0.01
            } else {
                false
            }
        };

        // Determine state
        let new_state = if is_attacking {
            PlayerAnimationState::Attacking
        } else if is_moving {
            PlayerAnimationState::Walking
        } else if *anim_state == PlayerAnimationState::IdleNervous {
            // Keep nervous
            PlayerAnimationState::IdleNervous
        } else {
            PlayerAnimationState::Idle
        };

        if *anim_state != new_state {
            *anim_state = new_state;
        }
    }
}

/// Updates previous position for tracking movement
#[cfg(feature = "client")]
pub fn update_player_prev_positions(
    mut commands: Commands,
    mut query: Query<(Entity, &PlayerRoot, Option<&mut PlayerPrevPosition>)>,
    player_query: Query<
        &bevy::transform::components::Transform,
        With<crate::players::player::Player>,
    >,
) {
    for (entity, player_root, prev_position) in &mut query {
        if let Ok(transform) = player_query.get(player_root.0) {
            if let Some(mut prev_pos) = prev_position {
                prev_pos.0 = transform.translation;
            } else {
                commands
                    .entity(entity)
                    .insert(PlayerPrevPosition(transform.translation));
            }
        }
    }
}

/// Tracks last animation
#[cfg(feature = "client")]
#[derive(Component, Default, Clone, Copy, PartialEq, Eq)]
pub struct LastPlayedPlayerAnimation(Option<PlayerAnimationState>);

/// Transition duration
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
        // Check if change needed
        let should_play = match last_played {
            Some(last) => last.0 != Some(*state),
            None => true, // Force play
        };

        if !should_play {
            continue;
        }

        println!("Playing player animation: {:?}", state);

        // Smooth blend
        match *state {
            PlayerAnimationState::Idle => {
                let active =
                    transitions.play(&mut player, animations.idle, ANIMATION_TRANSITION_DURATION);
                if config.idle_animation.repeat {
                    active.repeat();
                }
            }
            PlayerAnimationState::IdleNervous => {
                // Play once
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

/// Update attack timer
#[cfg(feature = "client")]
pub fn update_player_attack_timer(mut query: Query<&mut PlayerAttacking>, time: Res<Time>) {
    const ATTACK_DURATION: f32 = 0.6; // Attack duration

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

/// Trigger attack
#[cfg(feature = "client")]
pub fn trigger_player_attack_animation(
    mut query: Query<(&mut PlayerAttacking, &crate::players::player::PlayerOwner)>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    suduxu_input: Option<Res<ButtonInput<crate::suduxu::SuduxuButton>>>,
    my_client_id: Res<crate::players::player::MyClientId>,
) {
    let suduxu_clicked =
        suduxu_input.map_or(false, |s| s.just_pressed(crate::suduxu::SuduxuButton::A));

    if keyboard_input.just_pressed(KeyCode::Space) || suduxu_clicked {
        for (mut attacking, owner) in &mut query {
            if owner.0 != my_client_id.0 {
                continue;
            }

            if attacking.is_attacking {
                continue;
            }

            attacking.is_attacking = true;
            attacking.attack_timer = 0.0;
        }
    }
}

/// Trigger variations
#[cfg(feature = "client")]
pub fn update_player_idle_variations(
    mut anim_query: Query<(&mut PlayerAnimationState, &PlayerRoot, &mut PlayerIdleTimer)>,
    time: Res<Time>,
) {
    const NERVOUS_DURATION: f32 = 2.5; // Nervous duration

    for (mut anim_state, _player_root, mut idle_timer) in &mut anim_query {
        match *anim_state {
            PlayerAnimationState::Idle => {
                // Inc idle
                idle_timer.time_idle += time.delta_secs();

                // Check variation
                if idle_timer.time_idle >= idle_timer.next_variation_time {
                    *anim_state = PlayerAnimationState::IdleNervous;
                    idle_timer.is_playing_variation = true;
                    idle_timer.time_idle = 0.0;
                }
            }
            PlayerAnimationState::IdleNervous => {
                // Wait finish, return idle
                idle_timer.time_idle += time.delta_secs();
                if idle_timer.time_idle >= NERVOUS_DURATION {
                    *anim_state = PlayerAnimationState::Idle;
                    idle_timer.is_playing_variation = false;
                    idle_timer.time_idle = 0.0;
                    idle_timer.next_variation_time = rand_variation_time();
                }
            }
            _ => {
                // Reset timer
                idle_timer.time_idle = 0.0;
                idle_timer.is_playing_variation = false;
            }
        }
    }
}
