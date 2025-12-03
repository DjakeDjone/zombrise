use bevy::{
    app::ScheduleRunnerPlugin, asset::AssetPlugin, mesh::MeshPlugin, prelude::*,
    scene::ScenePlugin, state::app::StatesPlugin,
};
use std::time::Duration;

use avian3d::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet2::{
    netcode::{NetcodeServerTransport, ServerAuthentication},
    renet2::{ConnectionConfig, RenetServer, ServerEvent},
    RenetChannelsExt, RepliconRenetPlugins,
};
use rand::Rng;
use renet2_netcode::NativeSocket;
use std::{
    net::{SocketAddr, UdpSocket},
    time::SystemTime,
};
use zombrise_shared::players::player::{DamageFlash, Health, Player, PlayerAttack, PlayerOwner};
use zombrise_shared::shared::{MapMarker, MovePlayer, SharedPlugin, TreeMarker};
use zombrise_shared::zombie::zombie::Zombie;

#[derive(Resource)]
struct ZombieSpawnTimer(Timer);

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
        .add_systems(Startup, setup_server)
        .add_systems(
            Update,
            (
                server_event_system,
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
                remove_dead_players,
                remove_fallen_entities,
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
        Collider::cuboid(56.0, 0.1, 56.0), // Flat ground: 56x0.1x56 units
    ));

    // Spawn trees with collision
    let radius = 28.0;
    let tree_positions = [
        Vec3::new(radius * 0.34, 0.0, radius * 0.4),
        Vec3::new(-radius * 0.36, 0.0, -radius * 0.38),
        Vec3::new(-radius * 0.12, 0.0, -radius * 0.55),
        Vec3::new(radius * 0.55, 0.0, 0.22),
        Vec3::new(-radius * 0.5, 0.0, 0.15),
    ];

    for position in tree_positions {
        commands.spawn((
            TreeMarker,
            Replicated,
            Transform::from_translation(position),
            GlobalTransform::default(),
            RigidBody::Static,
            Collider::cylinder(0.3, 2.0), // Collision cylinder for tree trunk and canopy
        ));
    }

    println!("Server started on {}", public_addr);
}

fn server_event_system(mut commands: Commands, mut server_events: MessageReader<ServerEvent>) {
    for event in server_events.read() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                println!("Client {:?} connected", client_id);
                // Spawn player for client
                commands.spawn((
                    Player,
                    PlayerOwner(*client_id),
                    Health::default(),
                    DamageFlash::default(),
                    Replicated,
                    Transform::from_xyz(0.0, 0.5, 0.0),
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
            ServerEvent::ClientDisconnected { client_id, reason } => {
                println!("Client {:?} disconnected: {:?}", client_id, reason);
            }
        }
    }
}

fn handle_move_player(
    mut events: MessageReader<FromClient<MovePlayer>>,
    mut query: Query<(&PlayerOwner, &mut LinearVelocity, &mut Transform)>,
) {
    let speed = 5.0;
    for FromClient {
        message: event,
        client_id: _,
    } in events.read()
    {
        for (_owner, mut velocity, mut transform) in &mut query {
            // if owner.0 != *client_id {
            //     continue;
            // }

            let yaw_rotation = Quat::from_rotation_y(event.camera_yaw);
            let rotated_direction = yaw_rotation * event.direction;

            velocity.x = rotated_direction.x * speed;
            velocity.z = rotated_direction.z * speed; // Rotate player to face movement direction (only in XZ plane)
            let horizontal_direction = Vec3::new(rotated_direction.x, 0.0, rotated_direction.z);
            if horizontal_direction.length() > 0.01 {
                let target_rotation =
                    Quat::from_rotation_arc(Vec3::NEG_Z, horizontal_direction.normalize());
                transform.rotation = target_rotation;
            }

            if event.direction.y > 0.0 {
                // Check if on ground, for simplicity, assume if y velocity is small
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
            Transform::from_xyz(x, 0.5, z),
            GlobalTransform::default(),
            RigidBody::Dynamic,
            Collider::capsule(0.5, 1.0),
            LinearVelocity::ZERO,
            AngularVelocity::ZERO,
            LockedAxes::new().lock_rotation_x().lock_rotation_z(),
            LinearDamping(0.5),
            AngularDamping(20.0),
        ));
        println!("Zombie spawned at {}, {}", x, z);
    }
}

fn zombie_movement(
    mut zombie_query: Query<(&mut LinearVelocity, &mut Transform), (With<Zombie>, Without<Player>)>,
    player_query: Query<&Transform, (With<Player>, Without<Zombie>)>,
) {
    let speed = 2.0;
    let chase_range = 10.0;

    for (mut lin_vel, mut zombie_transform) in &mut zombie_query {
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

        if let Some(player_pos) = nearest_player_pos {
            if min_dist < chase_range {
                // Chase
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

        // Random movement
        let change_direction_probability = 0.02;
        let random_number = rand::random::<f32>();

        let mut direction = Vec3::ZERO;
        if lin_vel.length_squared() > 0.01 {
            direction = lin_vel.normalize();
        }

        if random_number < change_direction_probability || direction == Vec3::ZERO {
            // Change direction
            direction = Vec3::new(
                rand::random::<f32>() * 2.0 - 1.0,
                0.0,
                rand::random::<f32>() * 2.0 - 1.0,
            )
            .normalize_or_zero();
        }

        // Move forward
        lin_vel.x = direction.x * speed;
        lin_vel.z = direction.z * speed;

        // Rotate to face movement direction
        let horizontal_direction = Vec3::new(direction.x, 0.0, direction.z);
        if horizontal_direction.length() > 0.01 {
            let target_rotation =
                Quat::from_rotation_arc(Vec3::NEG_Z, horizontal_direction.normalize());
            zombie_transform.rotation = target_rotation;
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
                damage_flash.timer = 0.3; // Flash for 0.3 seconds

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
        ),
        With<Player>,
    >,
    mut zombie_query: Query<(Entity, &Transform), With<Zombie>>,
    mut commands: Commands,
) {
    const ATTACK_RANGE: f32 = 2.0;
    const PLAYER_DAMAGE: f32 = 10.0;

    for FromClient { .. } in events.read() {
        let mut attacker_pos: Option<Vec3> = None;
        let mut attacker_entity: Option<Entity> = None;

        // Find the attacking player (there should only be one per event)
        for (entity, _, transform, _, _) in &player_query {
            attacker_pos = Some(transform.translation);
            attacker_entity = Some(entity);
            break;
        }

        if let Some(attacker_pos) = attacker_pos {
            // Attack Zombies
            for (zombie_entity, zombie_transform) in &mut zombie_query {
                let distance = attacker_pos.distance(zombie_transform.translation);

                if distance < ATTACK_RANGE {
                    commands.entity(zombie_entity).despawn();
                    println!("Player attacked zombie at distance {}", distance);
                }
            }

            // Attack other Players
            for (entity, _, transform, mut health, mut damage_flash) in &mut player_query {
                if Some(entity) != attacker_entity {
                    let distance = attacker_pos.distance(transform.translation);

                    if distance < ATTACK_RANGE {
                        health.current = (health.current - PLAYER_DAMAGE).max(0.0);
                        damage_flash.timer = 0.3;
                        println!("Player attacked another player at distance {}", distance);
                    }
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
