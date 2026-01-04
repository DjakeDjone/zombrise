#![allow(clippy::type_complexity)]
// Server main for Zombrise using Lightyear 0.25 networking
use bevy::prelude::*;
use bevy::{
    app::ScheduleRunnerPlugin, asset::AssetPlugin, scene::ScenePlugin, state::app::StatesPlugin,
};
use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

use avian3d::prelude::*;
use lightyear::connection::client::Connected;
use lightyear::prelude::input::native::ActionState;
use lightyear::prelude::server::*;
use lightyear::prelude::*;
use rand::Rng;

use zombrise_shared::protocol::GameInput;
use zombrise_shared::shared::{MapMarker, SharedPlugin, TreeMarker, ZombieDamageFlash};
use zombrise_shared::zombie::zombie::{Zombie, ZombieAnimationState, ZombieDying, ZOMBIE_SPEED};
use zombrise_shared::{
    entity2::Health,
    players::player::{DamageFlash, Player, PlayerAttackCooldown, PlayerOwner},
};

#[derive(Resource)]
struct ZombieSpawnTimer(Timer);

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum ZombieAiState {
    #[default]
    Idle,
    Wandering,
    Chasing,
    Attacking,
}

#[derive(Component)]
struct ZombieBehavior {
    state: ZombieAiState,
    timer: Timer,
    wander_direction: Vec3,
}

fn main() {
    App::new()
        .add_plugins(
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
                1.0 / 60.0,
            ))),
        )
        .add_plugins(AssetPlugin::default())
        .add_plugins(bevy::log::LogPlugin {
            level: bevy::log::Level::INFO,
            filter: "wgpu=error,bevy_render=info,bevy_ecs=info".to_string(),
            ..default()
        })
        .add_plugins(ScenePlugin)
        .add_plugins(StatesPlugin)
        .add_plugins(bevy_mesh::MeshPlugin) // Required for Avian3D
        .add_plugins(ServerPlugins::default()) // Lightyear ServerPlugins
        .add_plugins(SharedPlugin)
        .add_plugins(PhysicsPlugins::default())
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .insert_resource(ZombieSpawnTimer(Timer::from_seconds(
            20.0,
            TimerMode::Repeating,
        )))
        .add_observer(spawn_clients)
        .add_observer(despawn_clients)
        .add_systems(Startup, (setup_networking, setup_server).chain())
        .add_systems(
            FixedUpdate,
            (
                handle_player_attack,
                zombie_movement,
                zombie_collision_damage,
                update_damage_flash,
                update_zombie_damage_flash,
                update_attack_cooldown,
                update_dying_zombies,
                // remove_dead_players, // Don't despawn players immediately so death screen can show
                remove_fallen_entities,
                passive_health_regeneration,
            ),
        )
        .add_systems(Update, (update_map_size, spawn_zombies))
        .run();
}

/// Setup networking - spawns the server entity with networking components
fn setup_networking(mut commands: Commands) {
    use lightyear::prelude::server::{NetcodeConfig, NetcodeServer, Start};
    use lightyear_udp::server::ServerUdpIo;

    let server_addr = SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::UNSPECIFIED), 5000);

    // Create netcode config with matching private key and protocol id
    let netcode_config = NetcodeConfig {
        private_key: [0u8; 32], // Must match client's private_key
        protocol_id: 0,         // Must match client's protocol_id
        ..Default::default()
    };

    let netcode_server = NetcodeServer::new(netcode_config);

    // Spawn the server networking entity with netcode support
    use lightyear::prelude::{ReplicationReceiver, ReplicationSender};
    let server_entity = commands
        .spawn((
            Name::new("Server"),
            LocalAddr(server_addr),
            ServerUdpIo::default(),
            netcode_server,
            ReplicationSender::default(),
            ReplicationReceiver::default(),
        ))
        .id();

    // Trigger Start event to begin accepting connections
    commands.trigger(Start {
        entity: server_entity,
    });

    info!(
        "Server listening on {} with netcode authentication",
        server_addr
    );
}

fn setup_server(mut commands: Commands) {
    let radius = 28.0;

    // Spawn map with collision
    commands.spawn((
        MapMarker,
        Replicate::to_clients(NetworkTarget::All),
        Transform::from_xyz(0.0, -0.05, 0.0),
        GlobalTransform::default(),
        RigidBody::Static,
        Collider::cuboid(56.0, 0.1, 56.0),
    ));

    // Spawn trees
    let tree_positions = [
        Vec3::new(radius * 0.34, 0.0, radius * 0.4),
        Vec3::new(-radius * 0.36, 0.0, -radius * 0.38),
        Vec3::new(-radius * 0.12, 0.0, -radius * 0.55),
        Vec3::new(radius * 0.55, 0.0, 0.22),
        Vec3::new(radius * 0.7, 0.0, radius * 0.65),
        Vec3::new(-radius * 0.72, 0.0, radius * 0.58),
        Vec3::new(radius * 0.15, 0.0, -radius * 0.78),
        Vec3::new(-radius * 0.8, 0.0, -radius * 0.15),
        Vec3::new(radius * 0.82, 0.0, -radius * 0.45),
        Vec3::new(-radius * 0.25, 0.0, radius * 0.72),
        Vec3::new(radius * 0.48, 0.0, -radius * 0.68),
        Vec3::new(-radius * 0.62, 0.0, radius * 0.32),
        Vec3::new(radius * 0.22, 0.0, radius * 0.85),
        Vec3::new(-radius * 0.45, 0.0, -radius * 0.75),
        Vec3::new(radius * 0.75, 0.0, radius * 0.18),
        Vec3::new(radius * 0.38, 0.0, -radius * 0.22),
        Vec3::new(-radius * 0.85, 0.0, radius * 0.08),
        Vec3::new(radius * 0.05, 0.0, radius * 0.62),
    ];

    for position in tree_positions {
        commands.spawn((
            TreeMarker,
            Replicate::to_clients(NetworkTarget::All),
            Transform::from_translation(position),
            GlobalTransform::default(),
            RigidBody::Static,
            Collider::cylinder(0.3, 2.0),
        ));
    }

    // Giant tree
    commands.spawn((
        TreeMarker,
        Replicate::to_clients(NetworkTarget::All),
        Transform::from_translation(Vec3::new(radius * 0.9, 0.0, radius * 0.9)),
        GlobalTransform::default(),
        RigidBody::Static,
        Collider::cylinder(0.6, 4.0),
    ));
}

/// Spawns a player when a client connects (Lightyear 0.25 pattern)
fn spawn_clients(
    trigger: On<Add, Connected>,
    query: Query<&RemoteId, With<ClientOf>>,
    mut commands: Commands,
) {
    let Ok(remote_id) = query.get(trigger.entity) else {
        return;
    };
    let client_id = remote_id.0;
    info!("Client connected: {:?}", client_id);

    // ClientOf entity needs ReplicationSender for Lightyear to send replicated data
    use lightyear::prelude::{ReplicationReceiver, ReplicationSender};
    commands
        .entity(trigger.entity)
        .insert((ReplicationSender::default(), ReplicationReceiver::default()));

    let player_entity = commands
        .spawn((
            Player,
            PlayerOwner(client_id.to_bits()),
            Health::default(),
            DamageFlash::default(),
            PlayerAttackCooldown::default(),
            Replicate::to_clients(NetworkTarget::All),
            PredictionTarget::to_clients(NetworkTarget::Single(client_id)),
            ControlledBy {
                owner: trigger.entity,
                lifetime: Default::default(),
            },
            Transform::from_xyz(0.0, 1.0, 0.0),
            GlobalTransform::default(),
            ActionState::<GameInput>::default(), // Required for input handling
        ))
        .insert((
            RigidBody::Dynamic,
            Collider::capsule(0.5, 1.0),
            LinearVelocity::ZERO,
            AngularVelocity::ZERO,
            LockedAxes::new().lock_rotation_x().lock_rotation_z(),
            LinearDamping(0.5),
            AngularDamping(20.0),
        ))
        .id();

    let _ = player_entity; // Suppress unused variable warning
}

/// Despawns a player when a client disconnects
fn despawn_clients(
    trigger: On<Remove, Connected>,
    query: Query<&RemoteId>,
    mut commands: Commands,
    players: Query<(Entity, &PlayerOwner)>,
) {
    let Ok(remote_id) = query.get(trigger.entity) else {
        return;
    };
    let client_id = remote_id.0;
    info!("Client disconnected: {:?}", client_id);

    for (entity, owner) in &players {
        if owner.0 == client_id.to_bits() {
            commands.entity(entity).despawn();
            break;
        }
    }
}

/// Handle player attacks
fn handle_player_attack(
    mut player_query: Query<
        (
            Entity,
            &PlayerOwner,
            &mut Transform,
            &mut Health,
            &mut DamageFlash,
            &mut PlayerAttackCooldown,
            &ActionState<GameInput>,
        ),
        With<Player>,
    >,
    zombie_query: Query<
        (Entity, &Transform),
        (With<Zombie>, Without<Player>, Without<ZombieDying>),
    >,
    mut zombie_health_query: Query<
        (&mut Health, &mut ZombieDamageFlash),
        (With<Zombie>, Without<Player>, Without<ZombieDying>),
    >,
    mut commands: Commands,
    _spatial_query: SpatialQuery,
) {
    const ATTACK_RANGE: f32 = 2.5;
    const ATTACK_DAMAGE: f32 = 25.0;

    for (
        _player_entity,
        _owner,
        mut player_transform,
        _player_health,
        _damage_flash,
        mut cooldown,
        action_state,
    ) in &mut player_query
    {
        if cooldown.0 > 0.0 {
            continue;
        }

        if matches!(action_state.0, GameInput::Attack) {
            cooldown.0 = 0.5; // Attack cooldown

            let attack_origin = player_transform.translation;

            // Find the closest zombie within range
            let mut closest_zombie: Option<(Entity, Vec3, f32)> = None; // (entity, to_zombie, distance)

            for (zombie_entity, zombie_transform) in &zombie_query {
                let to_zombie = zombie_transform.translation - attack_origin;
                let distance = to_zombie.length();

                if distance <= ATTACK_RANGE
                    && (closest_zombie.is_none() || distance < closest_zombie.as_ref().unwrap().2)
                {
                    closest_zombie = Some((zombie_entity, to_zombie, distance));
                }
            }

            // If there's a zombie in range, rotate toward it and attack
            if let Some((zombie_entity, to_zombie, _distance)) = closest_zombie {
                // Always rotate player toward the closest zombie
                let dir = to_zombie.normalize();
                let flat_dir = Vec3::new(dir.x, 0.0, dir.z).normalize_or_zero();

                if flat_dir.length_squared() > 0.0 {
                    player_transform.look_to(flat_dir, Vec3::Y);
                }

                // Apply damage to this zombie
                if let Ok((mut zombie_health, mut zombie_flash)) =
                    zombie_health_query.get_mut(zombie_entity)
                {
                    zombie_health.current -= ATTACK_DAMAGE;
                    zombie_flash.timer = 0.15;

                    if zombie_health.current <= 0.0 {
                        // Start dying sequence
                        commands.entity(zombie_entity).insert(ZombieDying {
                            timer: 0.0,
                            fall_duration: 1.0,
                            burn_duration: 2.0,
                        });
                    }
                }
            }
        }
    }
}

fn update_map_size(
    player_query: Query<&Player>,
    mut map_query: Query<&mut Transform, With<MapMarker>>,
) {
    let player_count = player_query.iter().count();
    let scale = 1.0 + (player_count as f32 * 0.1).min(2.0);

    for mut transform in &mut map_query {
        transform.scale = Vec3::splat(scale);
    }
}

fn spawn_zombies(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<ZombieSpawnTimer>,
    zombie_query: Query<&Zombie>,
) {
    timer.0.tick(time.delta());

    if timer.0.just_finished() && zombie_query.iter().count() < 50 {
        let mut rng = rand::rng();
        let radius = 25.0;
        let angle: f32 = rng.random_range(0.0..std::f32::consts::TAU);
        let dist: f32 = rng.random_range(5.0..radius);
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
            ZombieBehavior {
                state: ZombieAiState::default(),
                timer: Timer::from_seconds(2.0, TimerMode::Repeating),
                wander_direction: Vec3::ZERO,
            },
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

fn zombie_movement(
    mut zombie_query: Query<
        (
            &mut LinearVelocity,
            &mut Transform,
            &mut ZombieBehavior,
            &mut ZombieAnimationState,
        ),
        (With<Zombie>, Without<Player>, Without<ZombieDying>),
    >,
    player_query: Query<(&Transform, &Health), (With<Player>, Without<Zombie>)>,
    time: Res<Time>,
) {
    const CHASE_RANGE: f32 = 15.0;
    const ATTACK_RANGE: f32 = 1.5;

    for (mut velocity, mut transform, mut behavior, mut anim_state) in &mut zombie_query {
        behavior.timer.tick(time.delta());

        // Find closest player
        let mut closest_player: Option<Vec3> = None;
        let mut closest_dist = f32::MAX;

        for (player_transform, health) in &player_query {
            if health.current <= 0.0 {
                continue;
            }
            let dist = transform.translation.distance(player_transform.translation);
            if dist < closest_dist {
                closest_dist = dist;
                closest_player = Some(player_transform.translation);
            }
        }

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

fn zombie_collision_damage(
    zombie_query: Query<&Transform, (With<Zombie>, Without<ZombieDying>)>,
    mut player_query: Query<(&Transform, &mut Health, &mut DamageFlash), With<Player>>,
    time: Res<Time>,
) {
    const DAMAGE_RANGE: f32 = 1.5;
    const DAMAGE_PER_SECOND: f32 = 10.0;

    for zombie_transform in &zombie_query {
        for (player_transform, mut health, mut flash) in &mut player_query {
            if health.current <= 0.0 {
                continue;
            }

            let dist = zombie_transform
                .translation
                .distance(player_transform.translation);
            if dist <= DAMAGE_RANGE {
                health.current -= DAMAGE_PER_SECOND * time.delta_secs();
                flash.timer = 0.1;
            }
        }
    }
}

fn update_damage_flash(mut query: Query<&mut DamageFlash>, time: Res<Time>) {
    for mut flash in &mut query {
        if flash.timer > 0.0 {
            flash.timer -= time.delta_secs();
        }
    }
}

fn update_zombie_damage_flash(mut query: Query<&mut ZombieDamageFlash>, time: Res<Time>) {
    for mut flash in &mut query {
        if flash.timer > 0.0 {
            flash.timer -= time.delta_secs();
        }
    }
}

fn update_attack_cooldown(mut query: Query<&mut PlayerAttackCooldown>, time: Res<Time>) {
    for mut cooldown in &mut query {
        if cooldown.0 > 0.0 {
            cooldown.0 -= time.delta_secs();
        }
    }
}

fn update_dying_zombies(
    mut commands: Commands,
    mut dying_query: Query<
        (
            Entity,
            &mut ZombieDying,
            &mut ZombieAnimationState,
            &mut LinearVelocity,
        ),
        With<Zombie>,
    >,
    time: Res<Time>,
) {
    for (entity, mut dying, mut anim_state, mut velocity) in &mut dying_query {
        dying.timer += time.delta_secs();
        *anim_state = ZombieAnimationState::Dying;
        velocity.x = 0.0;
        velocity.z = 0.0;

        if dying.timer >= dying.fall_duration + dying.burn_duration {
            commands.entity(entity).despawn();
        }
    }
}

fn remove_fallen_entities(
    mut commands: Commands,
    player_query: Query<(Entity, &Transform), With<Player>>,
    zombie_query: Query<(Entity, &Transform), With<Zombie>>,
) {
    const FALL_THRESHOLD: f32 = -10.0;

    for (entity, transform) in &player_query {
        if transform.translation.y < FALL_THRESHOLD {
            commands.entity(entity).despawn();
        }
    }

    for (entity, transform) in &zombie_query {
        if transform.translation.y < FALL_THRESHOLD {
            commands.entity(entity).despawn();
        }
    }
}

fn passive_health_regeneration(mut query: Query<&mut Health, With<Player>>, time: Res<Time>) {
    const REGEN_RATE: f32 = 2.0;

    for mut health in &mut query {
        if health.current < health.max {
            health.current = (health.current + REGEN_RATE * time.delta_secs()).min(health.max);
        }
    }
}
