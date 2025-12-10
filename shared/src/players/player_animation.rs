#[cfg(feature = "client")]
use bevy::animation::AnimationPlayer;
use bevy::prelude::*;

/// Tracks if the player is currently attacking (for animation purposes)
#[cfg(feature = "client")]
#[derive(Component, Default)]
pub struct PlayerAttacking {
    pub is_attacking: bool,
    pub attack_timer: f32,
}

#[cfg(feature = "client")]
#[derive(Component)]
pub struct PlayerAnimations {
    pub idle: AnimationNodeIndex,
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
    Walking,
    Attacking,
}

#[cfg(feature = "client")]
pub struct PlayerAnimationConfig {
    pub model_path: &'static str,
    pub idle_animation: AnimationClipConfig,
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

        commands
            .entity(entity)
            .insert(AnimationGraphHandle(graphs.add(graph)));
        commands.entity(entity).insert(PlayerAnimations {
            idle: idle_node,
            walking: walking_node,
            attacking: attacking_node,
        });
        commands
            .entity(entity)
            .insert(PlayerAnimationState::default());
        // Link this AnimationPlayer to its root Player entity
        commands.entity(entity).insert(PlayerRoot(player_entity));

        // Start with idle animation
        player.play(idle_node).repeat();
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

        // Determine animation state
        let new_state = if is_attacking {
            PlayerAnimationState::Attacking
        } else if is_moving {
            PlayerAnimationState::Walking
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

#[cfg(feature = "client")]
pub fn control_player_animation(
    mut commands: Commands,
    mut animation_players: Query<(
        Entity,
        &mut AnimationPlayer,
        &PlayerAnimations,
        &PlayerAnimationState,
        Option<&LastPlayedPlayerAnimation>,
    )>,
) {
    let config = PlayerAnimationConfig::default();

    for (entity, mut player, animations, state, last_played) in &mut animation_players {
        // Check if we need to play this animation
        let should_play = match last_played {
            Some(last) => last.0 != Some(*state),
            None => true, // Never played anything, must play
        };

        if !should_play {
            continue;
        }

        println!("Playing player animation: {:?}", state);

        // Stop all current animations to ensure clean transition
        player.stop_all();

        match *state {
            PlayerAnimationState::Idle => {
                if config.idle_animation.repeat {
                    player.play(animations.idle).repeat();
                } else {
                    player.play(animations.idle);
                }
            }
            PlayerAnimationState::Walking => {
                if config.walking_animation.repeat {
                    player.play(animations.walking).repeat();
                } else {
                    player.play(animations.walking);
                }
            }
            PlayerAnimationState::Attacking => {
                if config.attacking_animation.repeat {
                    player.play(animations.attacking).repeat();
                } else {
                    player.play(animations.attacking);
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
