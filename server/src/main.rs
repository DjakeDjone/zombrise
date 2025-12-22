use bevy::{
    app::ScheduleRunnerPlugin, asset::AssetPlugin, mesh::MeshPlugin, prelude::*,
    scene::ScenePlugin, state::app::StatesPlugin,
};
use std::time::Duration;

use avian3d::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon::shared::backend::connected_client::NetworkId;
use bevy_replicon_renet2::{
    netcode::{NetcodeServerTransport, ServerAuthentication},
    renet2::{ConnectionConfig, RenetServer},
    RenetChannelsExt, RepliconRenetPlugins,
};
use rand::Rng;
use renet2_netcode::NativeSocket;
use std::{
    net::{SocketAddr, UdpSocket},
    time::SystemTime,
};
use zombrise_shared::shared::{MapMarker, MovePlayer, SharedPlugin, TreeMarker, ZombieDamageFlash};
use zombrise_shared::zombie::zombie::{Zombie, ZOMBIE_SPEED};
use zombrise_shared::{
    entity2::Health,
    players::player::{DamageFlash, Player, PlayerAttack, PlayerAttackCooldown, PlayerOwner},
};

#[derive(Resource)]
struct ZombieSpawnTimer(Timer);

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum ZombieAiState {
    #[default]
    Idle,
    Wandering,
    Chasing,
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
        .add_plugins(MeshPlugin)
        .add_plugins(ScenePlugin)
        .add_plugins(StatesPlugin)
        .add_plugins(RepliconPlugins)
        // .add_message::<ServerEvent>(Channel::Reliable)
        .add_plugins(RepliconRenetPlugins)
        .add_plugins(SharedPlugin)
        .add_plugins(PhysicsPlugins::default())
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .insert_resource(ZombieSpawnTimer(Timer::from_seconds(
            20.0,
            TimerMode::Repeating,
        )))
        .add_observer(spawn_clients)
        .add_observer(despawn_clients)
        .add_systems(Startup, setup_server)
        .add_systems(
            Update,
            (
                handle_move_player,
                handle_player_attack,
                update_map_size,
                spawn_zombies,
            ),
        )
        .add_systems(
            FixedUpdate,
            (
                zombie_movement,
                zombie_collision_damage,
                update_damage_flash,
                update_zombie_damage_flash,
                update_attack_cooldown,
                remove_dead_players,
                remove_fallen_entities,
                passive_health_regeneration,
            ),
        )
        .run();
}

fn setup_server(mut commands: Commands, network_channels: Res<RepliconChannels>) {
    let server_channels_config = network_channels.server_configs();
    let client_channels_config = network_channels.client_configs();

    let server = RenetServer::new(ConnectionConfig {
        server_channels_config,
        client_channels_config,
        available_bytes_per_tick: 16 * 1024,
    });

    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();

    let public_addr: SocketAddr = "0.0.0.0:5000".parse().unwrap();
    let socket = UdpSocket::bind(public_addr).unwrap();
    let native_socket = NativeSocket::new(socket).unwrap();

    let socket_addresses = vec![vec![public_addr]];
    let server_setup_config = bevy_replicon_renet2::netcode::ServerSetupConfig {
        current_time,
        max_clients: 10,
        protocol_id: 0,
        socket_addresses,
        authentication: ServerAuthentication::Unsecure,
    };

    let transport = NetcodeServerTransport::new(server_setup_config, native_socket).unwrap();

    commands.insert_resource(server);
    commands.insert_resource(transport);

    // Add ground (flat surface)
    commands.spawn((
        MapMarker,
        Replicated,
        Transform::from_xyz(0.0, -0.05, 0.0),
        RigidBody::Static,
        Collider::cuboid(56.0, 0.1, 56.0),
    ));

    // Spawn trees with collision - positioned at ground level (Y=0)
    let radius = 28.0;
    let tree_positions = [
        // Original trees
        Vec3::new(radius * 0.34, 0.0, radius * 0.4),
        Vec3::new(-radius * 0.36, 0.0, -radius * 0.38),
        Vec3::new(-radius * 0.12, 0.0, -radius * 0.55),
        Vec3::new(radius * 0.55, 0.0, 0.22),
        // Additional trees for denser forest
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
            Replicated,
            Transform::from_translation(position),
            GlobalTransform::default(),
            RigidBody::Static,
            Collider::cylinder(0.3, 2.0),
        ));
    }

    // Giant tree in the corner of the map
    commands.spawn((
        TreeMarker,
        Replicated,
        Transform::from_translation(Vec3::new(radius * 0.9, 0.0, radius * 0.9)),
        GlobalTransform::default(),
        RigidBody::Static,
        Collider::cylinder(0.6, 4.0), // Larger collider for giant tree
    ));

    println!("Server started on {}", public_addr);
}

/// Spawns a player when a client connects
fn spawn_clients(
    trigger: On<Add, ConnectedClient>,
    mut commands: Commands,
    network_id_query: Query<&NetworkId>,
) {
    let client_entity = trigger.event().entity;

    // Get the NetworkId (renet2 client_id) from the client entity
    let network_id = network_id_query
        .get(client_entity)
        .expect("ConnectedClient should have NetworkId");
    let client_id = network_id.get();

    println!(
        "Client {:?} connected with network_id: {}",
        client_entity, client_id
    );

    commands.spawn((
        Player,
        PlayerOwner(client_id),
        Health::default(),
        DamageFlash::default(),
        PlayerAttackCooldown::default(),
        Replicated,
        Transform::from_xyz(0.0, 1.0, 0.0),
        GlobalTransform::default(),
        RigidBody::Dynamic,
        Collider::capsule(0.5, 1.0),
        LinearVelocity::ZERO,
        AngularVelocity::ZERO,
        LockedAxes::new().lock_rotation_x().lock_rotation_z(),
        LinearDamping(0.5),
        AngularDamping(20.0),
    ));
}

/// Despawns a player when a client disconnects
fn despawn_clients(
    trigger: On<Remove, ConnectedClient>,
    mut commands: Commands,
    players: Query<(Entity, &PlayerOwner)>,
    network_id_query: Query<&NetworkId>,
) {
    let client_entity = trigger.event().entity;

    // Get the NetworkId before the entity is fully removed
    let client_id = network_id_query
        .get(client_entity)
        .map(|id| id.get())
        .unwrap_or(0);

    println!(
        "Client {:?} disconnected (network_id: {})",
        client_entity, client_id
    );

    // Find and despawn the player owned by this client
    for (entity, owner) in &players {
        if owner.0 == client_id {
            commands.entity(entity).despawn();
            break;
        }
    }
}

fn handle_move_player(
    mut events: MessageReader<FromClient<MovePlayer>>,
    mut query: Query<(&PlayerOwner, &mut LinearVelocity, &mut Transform)>,
    network_id_query: Query<&NetworkId>,
) {
    let speed = 5.0;
    for FromClient {
        message: event,
        client_id,
    } in events.read()
    {
        // Get NetworkId from the client entity
        let client_network_id = client_id
            .entity()
            .and_then(|e| network_id_query.get(e).ok())
            .map(|id| id.get())
            .unwrap_or(0);

        for (owner, mut velocity, mut transform) in &mut query {
            if owner.0 != client_network_id {
                continue;
            }

            let yaw_rotation = Quat::from_rotation_y(event.camera_yaw);
            let rotated_direction = yaw_rotation * event.direction;

            velocity.x = rotated_direction.x * speed;
            velocity.z = rotated_direction.z * speed;
            let horizontal_direction = Vec3::new(rotated_direction.x, 0.0, rotated_direction.z);
            if horizontal_direction.length() > 0.01 {
                let target_rotation =
                    Quat::from_rotation_arc(Vec3::NEG_Z, horizontal_direction.normalize());
                transform.rotation = target_rotation;
            }

            if event.direction.y > 0.0 {
                // Ground check
                if velocity.y.abs() < 0.1 {
                    velocity.y = 5.0; // jump velocity
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
    if let Ok(mut transform) = map_query.single_mut() {
        let target_scale = 1.0 + (player_count as f32 * 0.2);
        if (transform.scale.x - target_scale).abs() > 0.01 {
            transform.scale = Vec3::splat(target_scale);
        }
    }
}

fn spawn_zombies(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<ZombieSpawnTimer>,
    zombie_query: Query<&Zombie>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        let zombie_count = zombie_query.iter().count();
        if zombie_count >= 30 {
            return;
        }

        let mut rng = rand::rng();
        let x = rng.random_range(-20.0..20.0);
        let z = rng.random_range(-20.0..20.0);

        commands.spawn((
            Zombie,
            Replicated,
            Health::default(),
            ZombieDamageFlash::default(),
            Transform::from_xyz(x, 1.0, z),
            GlobalTransform::default(),
            RigidBody::Dynamic,
            Collider::capsule(0.3, 0.8),
            LinearVelocity::ZERO,
            AngularVelocity::ZERO,
            LockedAxes::new().lock_rotation_x().lock_rotation_z(),
            LinearDamping(0.5),
            AngularDamping(20.0),
            ZombieBehavior {
                state: ZombieAiState::Idle,
                timer: Timer::from_seconds(rng.random_range(1.0..3.0), TimerMode::Once),
                wander_direction: Vec3::ZERO,
            },
        ));
        println!("Zombie spawned at {}, {}", x, z);
    }
}

fn zombie_movement(
    mut zombie_query: Query<
        (&mut LinearVelocity, &mut Transform, &mut ZombieBehavior),
        (With<Zombie>, Without<Player>),
    >,
    player_query: Query<&Transform, (With<Player>, Without<Zombie>)>,
    time: Res<Time>,
) {
    let speed = ZOMBIE_SPEED;
    let chase_range = 10.0;

    for (mut lin_vel, mut zombie_transform, mut behavior) in &mut zombie_query {
        let mut nearest_player_pos: Option<Vec3> = None;
        let mut min_dist = f32::MAX;

        for player_transform in &player_query {
            let dist = zombie_transform
                .translation
                .distance(player_transform.translation);
            if dist < min_dist {
                min_dist = dist;
                nearest_player_pos = Some(player_transform.translation);
            }
        }

        // Check if we should chase
        if let Some(player_pos) = nearest_player_pos {
            if min_dist < chase_range {
                behavior.state = ZombieAiState::Chasing;

                // Chase logic
                let direction = (player_pos - zombie_transform.translation).normalize_or_zero();
                lin_vel.x = direction.x * speed;
                lin_vel.z = direction.z * speed;

                // Rotate to face player
                let horizontal_direction = Vec3::new(direction.x, 0.0, direction.z);
                if horizontal_direction.length() > 0.01 {
                    let target_rotation =
                        Quat::from_rotation_arc(Vec3::NEG_Z, horizontal_direction.normalize());
                    zombie_transform.rotation = target_rotation;
                }
                continue;
            }
        }

        // If we were chasing but lost the player, go back to idle
        if behavior.state == ZombieAiState::Chasing {
            behavior.state = ZombieAiState::Idle;
            behavior.timer = Timer::from_seconds(1.0, TimerMode::Once);
        }

        // Handle Idle and Wandering states
        behavior.timer.tick(time.delta());

        match behavior.state {
            ZombieAiState::Idle => {
                lin_vel.x = 0.0;
                lin_vel.z = 0.0;

                if behavior.timer.is_finished() {
                    // Switch to Wandering
                    behavior.state = ZombieAiState::Wandering;
                    behavior.timer =
                        Timer::from_seconds(rand::random::<f32>() * 2.0 + 2.0, TimerMode::Once);

                    // Pick a random direction
                    behavior.wander_direction = Vec3::new(
                        rand::random::<f32>() * 2.0 - 1.0,
                        0.0,
                        rand::random::<f32>() * 2.0 - 1.0,
                    )
                    .normalize_or_zero();
                }
            }
            ZombieAiState::Wandering => {
                lin_vel.x = behavior.wander_direction.x * speed;
                lin_vel.z = behavior.wander_direction.z * speed;

                // Rotate to face movement direction
                let horizontal_direction = Vec3::new(
                    behavior.wander_direction.x,
                    0.0,
                    behavior.wander_direction.z,
                );
                if horizontal_direction.length() > 0.01 {
                    let target_rotation =
                        Quat::from_rotation_arc(Vec3::NEG_Z, horizontal_direction.normalize());
                    zombie_transform.rotation = target_rotation;
                }

                if behavior.timer.is_finished() {
                    // Switch to Idle
                    behavior.state = ZombieAiState::Idle;
                    behavior.timer =
                        Timer::from_seconds(rand::random::<f32>() * 2.0 + 1.0, TimerMode::Once);
                }
            }
            _ => {} // Chasing is handled above
        }
    }
}

fn zombie_collision_damage(
    zombie_query: Query<&Transform, With<Zombie>>,
    mut player_query: Query<(&Transform, &mut Health, &mut DamageFlash), With<Player>>,
    time: Res<Time>,
) {
    const DAMAGE_PER_SECOND: f32 = 10.0;
    const COLLISION_DISTANCE: f32 = 1.5;

    for zombie_transform in &zombie_query {
        for (player_transform, mut health, mut damage_flash) in &mut player_query {
            let distance = zombie_transform
                .translation
                .distance(player_transform.translation);

            if distance < COLLISION_DISTANCE && health.current > 0.0 {
                let damage = DAMAGE_PER_SECOND * time.delta_secs();
                health.current = (health.current - damage).max(0.0);
                damage_flash.timer = 0.3;

                if health.current <= 0.0 {
                    println!("Player died!");
                }
            }
        }
    }
}

fn handle_player_attack(
    mut events: MessageReader<FromClient<PlayerAttack>>,
    mut player_query: Query<
        (
            Entity,
            &PlayerOwner,
            &Transform,
            &mut Health,
            &mut DamageFlash,
            &mut PlayerAttackCooldown,
        ),
        (With<Player>, Without<Zombie>),
    >,
    mut zombie_query: Query<
        (Entity, &Transform, &mut Health, &mut ZombieDamageFlash),
        (With<Zombie>, Without<Player>),
    >,
    mut commands: Commands,
    network_id_query: Query<&NetworkId>,
    spatial_query: SpatialQuery,
) {
    const ATTACK_RANGE: f32 = 2.0;
    const ATTACK_ANGLE: f32 = 0.5; // ~60 degrees half-angle
    const PLAYER_DAMAGE: f32 = 10.0;
    const ZOMBIE_DAMAGE: f32 = 20.0;

    for FromClient { client_id, .. } in events.read() {
        let mut attacker_info: Option<(Entity, Transform)> = None;

        // Get NetworkId from the client entity
        let client_network_id = client_id
            .entity()
            .and_then(|e| network_id_query.get(e).ok())
            .map(|id| id.get())
            .unwrap_or(0);

        // Find attacker corresponding to this client_id
        for (entity, owner, transform, _, _, _) in &player_query {
            if owner.0 == client_network_id {
                attacker_info = Some((entity, *transform));
                break;
            }
        }

        if let Some((attacker_entity, attacker_transform)) = attacker_info {
            let (_, _, _, _, _, mut cooldown) = player_query.get_mut(attacker_entity).unwrap();

            if cooldown.0 > 0.0 {
                continue;
            }

            // Reset cooldown
            cooldown.0 = 0.5; // 0.5 seconds cooldown

            let attacker_pos = attacker_transform.translation;
            let attacker_forward = *attacker_transform.forward();

            // Attack zombies
            for (zombie_entity, zombie_transform, mut zombie_health, mut zombie_damage_flash) in
                &mut zombie_query
            {
                let distance = attacker_pos.distance(zombie_transform.translation);

                if distance < ATTACK_RANGE {
                    // Calculate direction to target
                    let diff = zombie_transform.translation - attacker_pos;

                    // If they are too close, just hit. Otherwise check angle and line of sight.
                    if let Ok(to_target) = Dir3::new(diff) {
                        // Check angle
                        if attacker_forward.dot(*to_target) < ATTACK_ANGLE {
                            continue;
                        }

                        // Check line of sight (raycast)
                        let filter = SpatialQueryFilter::from_excluded_entities([attacker_entity]);
                        if let Some(hit) =
                            spatial_query.cast_ray(attacker_pos, to_target, distance, true, &filter)
                        {
                            if hit.entity != zombie_entity {
                                continue;
                            }
                        }
                    }

                    // Hit confirmed
                    zombie_health.current = (zombie_health.current - ZOMBIE_DAMAGE).max(0.0);
                    zombie_damage_flash.timer = 0.5; // Trigger hit animation

                    if zombie_health.current == 0.0 {
                        commands.entity(zombie_entity).despawn();
                        // Reward player with max health increase
                        if let Ok((_, _, _, mut attacker_health, _, _)) =
                            player_query.get_mut(attacker_entity)
                        {
                            attacker_health.max += 5.0;
                            attacker_health.current += 5.0; // Also heal the amount gained
                        }
                    }

                    println!("Player attacked zombie at distance {}", distance);
                }
            }

            // Attack players
            let mut killed_player_count = 0;
            for (entity, _, transform, mut health, mut damage_flash, _) in &mut player_query {
                if entity != attacker_entity {
                    let distance = attacker_pos.distance(transform.translation);

                    if distance < ATTACK_RANGE {
                        let diff = transform.translation - attacker_pos;

                        if let Ok(to_target) = Dir3::new(diff) {
                            // Check angle
                            if attacker_forward.dot(*to_target) < ATTACK_ANGLE {
                                continue;
                            }

                            // Check line of sight (raycast)
                            let filter =
                                SpatialQueryFilter::from_excluded_entities([attacker_entity]);
                            if let Some(hit) = spatial_query.cast_ray(
                                attacker_pos,
                                to_target,
                                distance,
                                true,
                                &filter,
                            ) {
                                if hit.entity != entity {
                                    continue;
                                }
                            }
                        }

                        // Hit confirmed
                        health.current = (health.current - PLAYER_DAMAGE).max(0.0);
                        damage_flash.timer = 0.3;

                        if health.current == 0.0 {
                            killed_player_count += 1;
                        }

                        println!("Player attacked another player at distance {}", distance);
                    }
                }
            }

            if killed_player_count > 0 {
                // Reward attacker with max health increase
                if let Ok((_, _, _, mut attacker_health, _, _)) =
                    player_query.get_mut(attacker_entity)
                {
                    let bonus = 10.0 * killed_player_count as f32;
                    attacker_health.max += bonus;
                    attacker_health.current += bonus; // Also heal the amount gained
                }
            }
        }
    }
}

fn update_damage_flash(mut query: Query<&mut DamageFlash>, time: Res<Time>) {
    for mut damage_flash in &mut query {
        if damage_flash.timer > 0.0 {
            damage_flash.timer -= time.delta_secs();
            if damage_flash.timer < 0.0 {
                damage_flash.timer = 0.0;
            }
        }
    }
}

fn update_zombie_damage_flash(mut query: Query<&mut ZombieDamageFlash>, time: Res<Time>) {
    for mut damage_flash in &mut query {
        if damage_flash.timer > 0.0 {
            damage_flash.timer -= time.delta_secs();
            if damage_flash.timer < 0.0 {
                damage_flash.timer = 0.0;
            }
        }
    }
}

fn remove_dead_players(
    mut commands: Commands,
    player_query: Query<(Entity, &Health, &PlayerOwner), With<Player>>,
    mut server: ResMut<RenetServer>,
) {
    for (entity, health, owner) in &player_query {
        if health.current <= 0.0 {
            println!("Removing dead player (Client ID: {:?})", owner.0);
            commands.entity(entity).despawn();
            server.disconnect(owner.0);
        }
    }
}

fn remove_fallen_entities(
    mut commands: Commands,
    player_query: Query<(Entity, &Transform, &PlayerOwner), With<Player>>,
    zombie_query: Query<(Entity, &Transform), With<Zombie>>,
    mut server: ResMut<RenetServer>,
) {
    const FALL_DEATH_Y: f32 = -10.0;

    // Remove fallen players
    for (entity, transform, owner) in &player_query {
        if transform.translation.y < FALL_DEATH_Y {
            println!("Player fell to death (Client ID: {:?})", owner.0);
            commands.entity(entity).despawn();
            server.disconnect(owner.0);
        }
    }

    // Remove fallen zombies
    for (entity, transform) in &zombie_query {
        if transform.translation.y < FALL_DEATH_Y {
            println!(
                "Zombie fell to death at position: {:?}",
                transform.translation
            );
            commands.entity(entity).despawn();
        }
    }
}

fn update_attack_cooldown(mut query: Query<&mut PlayerAttackCooldown>, time: Res<Time>) {
    for mut cooldown in &mut query {
        if cooldown.0 > 0.0 {
            cooldown.0 -= time.delta_secs();
            if cooldown.0 < 0.0 {
                cooldown.0 = 0.0;
            }
        }
    }
}

fn passive_health_regeneration(mut query: Query<&mut Health, With<Player>>, time: Res<Time>) {
    const REGEN_PER_SECOND: f32 = 1.0;

    for mut health in &mut query {
        if health.current > 0.0 && health.current < health.max {
            health.current =
                (health.current + REGEN_PER_SECOND * time.delta_secs()).min(health.max);
        }
    }
}
