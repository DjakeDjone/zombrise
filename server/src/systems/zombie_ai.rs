//! Zombie AI behavior and spawning systems.

use avian3d::prelude::*;
use bevy::prelude::*;
use lightyear::prelude::*;
use rand::Rng;

use zombrise_shared::entity2::Health;
use zombrise_shared::players::player::Player;
use zombrise_shared::shared::ZombieDamageFlash;
use zombrise_shared::zombie::zombie::{Zombie, ZombieAnimationState, ZombieDying, ZOMBIE_SPEED};

/// Resource for zombie spawning timer
#[derive(Resource)]
pub struct ZombieSpawnTimer(pub Timer);

impl Default for ZombieSpawnTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(SPAWN_INTERVAL, TimerMode::Repeating))
    }
}

/// AI state enum for zombie behavior
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ZombieAiState {
    #[default]
    Idle,
    Wandering,
    Chasing,
    Attacking,
}

/// Component for zombie behavior state
#[derive(Component)]
pub struct ZombieBehavior {
    pub state: ZombieAiState,
    pub timer: Timer,
    pub wander_direction: Vec3,
}

impl Default for ZombieBehavior {
    fn default() -> Self {
        Self {
            state: ZombieAiState::default(),
            timer: Timer::from_seconds(2.0, TimerMode::Repeating),
            wander_direction: Vec3::ZERO,
        }
    }
}

/// AI constants
pub const CHASE_RANGE: f32 = 15.0;
pub const ATTACK_RANGE: f32 = 1.5;
pub const MAX_ZOMBIES: usize = 50;
pub const SPAWN_INTERVAL: f32 = 20.0;
pub const SPAWN_RADIUS: f32 = 25.0;

/// Spawn zombies periodically
pub fn spawn_zombies(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<ZombieSpawnTimer>,
    zombie_query: Query<&Zombie>,
) {
    timer.0.tick(time.delta());

    if timer.0.just_finished() && zombie_query.iter().count() < MAX_ZOMBIES {
        let mut rng = rand::rng();
        let angle: f32 = rng.random_range(0.0..std::f32::consts::TAU);
        let dist: f32 = rng.random_range(5.0..SPAWN_RADIUS);
        let x = angle.cos() * dist;
        let z = angle.sin() * dist;

        commands.spawn((
            Zombie,
            Health {
                current: 100.0,
                max: 100.0,
            },
            ZombieDamageFlash { timer: 0.0 },
            ZombieAnimationState::default(),
            ZombieBehavior::default(),
            Replicate::to_clients(NetworkTarget::All),
            Transform::from_xyz(x, 1.0, z),
            GlobalTransform::default(),
            RigidBody::Dynamic,
            Collider::capsule(0.3, 1.0),
            LinearVelocity::ZERO,
            AngularVelocity::ZERO,
            LockedAxes::new().lock_rotation_x().lock_rotation_z(),
            LinearDamping(0.5),
            AngularDamping(20.0),
        ));
    }
}

/// Zombie movement and AI behavior
pub fn zombie_movement(
    player_query: Query<(&Transform, &Health), (With<Player>, Without<Zombie>)>,
    mut zombie_query: Query<
        (
            &mut LinearVelocity,
            &mut Transform,
            &mut ZombieBehavior,
            &mut ZombieAnimationState,
        ),
        (With<Zombie>, Without<Player>, Without<ZombieDying>),
    >,
    time: Res<Time>,
) {
    // Pre-collect active player positions (typically 1-4 players)
    // This avoids repeated query iteration for each zombie
    let players: Vec<Vec3> = player_query
        .iter()
        .filter(|(_, health)| health.current > 0.0)
        .map(|(t, _)| t.translation)
        .collect();

    for (mut velocity, mut transform, mut behavior, mut anim_state) in &mut zombie_query {
        behavior.timer.tick(time.delta());

        // Find closest player from pre-collected list
        let (closest_player, closest_dist) = players
            .iter()
            .map(|p| (*p, transform.translation.distance(*p)))
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(p, d)| (Some(p), d))
            .unwrap_or((None, f32::MAX));

        // Update AI state
        if let Some(player_pos) = closest_player {
            if closest_dist <= ATTACK_RANGE {
                behavior.state = ZombieAiState::Attacking;
                *anim_state = ZombieAnimationState::Attacking;
                velocity.x = 0.0;
                velocity.z = 0.0;
            } else if closest_dist <= CHASE_RANGE {
                behavior.state = ZombieAiState::Chasing;
                *anim_state = ZombieAnimationState::Running;

                let direction = (player_pos - transform.translation).normalize();
                velocity.x = direction.x * ZOMBIE_SPEED * 2.0;
                velocity.z = direction.z * ZOMBIE_SPEED * 2.0;

                // Face player
                if direction.length() > 0.01 {
                    let target = Quat::from_rotation_arc(
                        Vec3::NEG_Z,
                        Vec3::new(direction.x, 0.0, direction.z).normalize(),
                    );
                    transform.rotation = target;
                }
            } else {
                // Wander
                if behavior.timer.just_finished() {
                    let mut rng = rand::rng();
                    let angle: f32 = rng.random_range(0.0..std::f32::consts::TAU);
                    behavior.wander_direction = Vec3::new(angle.cos(), 0.0, angle.sin());
                    behavior.state = ZombieAiState::Wandering;
                }

                match behavior.state {
                    ZombieAiState::Wandering => {
                        *anim_state = ZombieAnimationState::Walking;
                        velocity.x = behavior.wander_direction.x * ZOMBIE_SPEED;
                        velocity.z = behavior.wander_direction.z * ZOMBIE_SPEED;

                        // Face the wander direction to prevent moonwalking
                        if behavior.wander_direction.length() > 0.01 {
                            let target = Quat::from_rotation_arc(
                                Vec3::NEG_Z,
                                Vec3::new(
                                    behavior.wander_direction.x,
                                    0.0,
                                    behavior.wander_direction.z,
                                )
                                .normalize(),
                            );
                            transform.rotation = target;
                        }
                    }
                    _ => {
                        *anim_state = ZombieAnimationState::Idle;
                        velocity.x = 0.0;
                        velocity.z = 0.0;
                    }
                }
            }
        } else {
            // No players, idle
            *anim_state = ZombieAnimationState::Idle;
            velocity.x = 0.0;
            velocity.z = 0.0;
        }
    }
}
