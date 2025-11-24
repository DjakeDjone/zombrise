use bevy::prelude::*;

use bevy::app::ScheduleRunnerPlugin;
use bevy_rapier3d::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::{
    renet::{
        transport::{NetcodeServerTransport, ServerAuthentication, ServerConfig},
        ConnectionConfig, RenetServer,
    },
    RenetChannelsExt, RepliconRenetPlugins,
};
use rand::Rng;
use std::{
    net::UdpSocket,
    time::{Duration, SystemTime},
};
use zombrise_shared::players::player::{DamageFlash, Health, Player, PlayerAttack, PlayerOwner};
use zombrise_shared::shared::{MapMarker, MovePlayer, SharedPlugin, TreeMarker};
use zombrise_shared::zombie::zombie::Zombie;

#[derive(Resource)]
struct ZombieSpawnTimer(Timer);

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .build()
                .disable::<bevy::render::RenderPlugin>()
                .disable::<bevy::core_pipeline::CorePipelinePlugin>()
                .disable::<bevy::sprite::SpritePlugin>()
                .disable::<bevy::pbr::PbrPlugin>()
                .disable::<bevy::ui::UiPlugin>()
                .disable::<bevy::text::TextPlugin>()
                .disable::<bevy::gizmos::GizmoPlugin>()
                .disable::<bevy::gltf::GltfPlugin>(),
        )
        .add_plugins(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
        .init_asset::<Mesh>()
        .init_asset::<Scene>()
        .add_plugins(RepliconPlugins)
        .add_plugins(RepliconRenetPlugins)
        .add_plugins(SharedPlugin)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .insert_resource(ZombieSpawnTimer(Timer::from_seconds(
            20.0,
            TimerMode::Repeating,
        )))
        .add_systems(Startup, setup_server)
        .add_systems(
            Update,
            (server_event_system, update_map_size, spawn_zombies),
        )
        .add_systems(
            FixedUpdate,
            (
                handle_move_player,
                zombie_movement,
                zombie_collision_damage,
                handle_player_attack,
                update_damage_flash,
                remove_dead_players,
            ),
        )
        .run();
}

fn setup_server(mut commands: Commands, network_channels: Res<RepliconChannels>) {
    let server_channels_config = network_channels.get_server_configs();
    let client_channels_config = network_channels.get_client_configs();

    let server = RenetServer::new(ConnectionConfig {
        server_channels_config,
        client_channels_config,
        ..Default::default()
    });

    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();

    let public_addr = "0.0.0.0:5000";
    let socket = UdpSocket::bind(public_addr).unwrap();

    let server_config = ServerConfig {
        current_time,
        max_clients: 10,
        protocol_id: 0,
        public_addresses: vec![public_addr.parse().unwrap()],
        authentication: ServerAuthentication::Unsecure,
    };

    let transport = NetcodeServerTransport::new(server_config, socket).unwrap();

    commands.insert_resource(server);
    commands.insert_resource(transport);

    // Add ground (flat surface)
    commands.spawn((
        MapMarker,
        Replicated,
        SpatialBundle::from_transform(Transform::from_xyz(0.0, -0.55, 0.0)),
        Collider::cuboid(28.0, 0.05, 28.0), // Flat ground: 56x0.1x56 units
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
            Collider::cylinder(1.0, 0.3), // Collision cylinder for tree trunk and canopy
        ));
    }

    println!("Server started on {}", public_addr);
}

fn server_event_system(mut commands: Commands, mut server_events: EventReader<ServerEvent>) {
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
                    Transform::from_xyz(0.0, 1.0, 0.0),
                    GlobalTransform::default(),
                    RigidBody::Dynamic,
                    Collider::capsule_y(0.5, 0.5),
                    Velocity::zero(),
                    LockedAxes::ROTATION_LOCKED,
                    Damping {
                        linear_damping: 0.5,
                        angular_damping: 0.0,
                    },
                ));
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                println!("Client {:?} disconnected: {:?}", client_id, reason);
            }
        }
    }
}

fn handle_move_player(
    mut events: EventReader<FromClient<MovePlayer>>,
    mut query: Query<(&PlayerOwner, &mut Velocity, &mut Transform)>,
) {
    let speed = 5.0;
    for FromClient { client_id, event } in events.read() {
        for (owner, mut velocity, mut transform) in &mut query {
            if owner.0 == *client_id {
                // Rotate the input direction by the camera yaw
                let yaw_rotation = Quat::from_rotation_y(event.camera_yaw);
                let rotated_direction = yaw_rotation * event.direction;

                velocity.linvel.x = rotated_direction.x * speed;
                velocity.linvel.z = rotated_direction.z * speed; // Rotate player to face movement direction (only in XZ plane)
                let horizontal_direction = Vec3::new(rotated_direction.x, 0.0, rotated_direction.z);
                if horizontal_direction.length() > 0.01 {
                    let target_rotation =
                        Quat::from_rotation_arc(Vec3::NEG_Z, horizontal_direction.normalize());
                    transform.rotation = target_rotation;
                }

                if event.direction.y > 0.0 {
                    // Check if on ground, for simplicity, assume if y velocity is small
                    if velocity.linvel.y.abs() < 0.1 {
                        velocity.linvel.y = 5.0; // jump velocity
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
    if let Ok(mut transform) = map_query.get_single_mut() {
        let target_scale = 1.0 + (player_count as f32 * 0.2);
        if (transform.scale.x - target_scale).abs() > 0.01 {
            transform.scale = Vec3::splat(target_scale);
        }
    }
}

fn spawn_zombies(mut commands: Commands, time: Res<Time>, mut timer: ResMut<ZombieSpawnTimer>) {
    if timer.0.tick(time.delta()).just_finished() {
        let mut rng = rand::rng();
        let x = rng.random_range(-20.0..20.0);
        let z = rng.random_range(-20.0..20.0);

        commands.spawn((
            Zombie,
            Replicated,
            Transform::from_xyz(x, 1.0, z),
            GlobalTransform::default(),
            RigidBody::Dynamic,
            Collider::capsule_y(0.5, 0.5),
            Velocity::zero(),
            LockedAxes::ROTATION_LOCKED,
            Damping {
                linear_damping: 0.5,
                angular_damping: 0.0,
            },
        ));
        println!("Zombie spawned at {}, {}", x, z);
    }
}

fn zombie_movement(
    mut zombie_query: Query<(&mut Velocity, &Transform), With<Zombie>>,
    player_query: Query<&Transform, With<Player>>,
) {
    let speed = 2.0;
    let chase_range = 10.0;

    for (mut velocity, zombie_transform) in &mut zombie_query {
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
                velocity.linvel.x = direction.x * speed;
                velocity.linvel.z = direction.z * speed;
                // also rotate
                let rotation = (player_pos - zombie_transform.translation).normalize_or_zero();
                velocity.angvel.x = rotation.x * speed;
                velocity.angvel.z = rotation.z * speed;
                continue;
            }
        }

        // Random movement
        let change_direction_probability = 0.02;
        let random_number = rand::random::<f32>();
        
        let mut direction = Vec3::ZERO;
        if velocity.linvel.length_squared() > 0.01 {
             direction = velocity.linvel.normalize();
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
        velocity.linvel.x = direction.x * speed;
        velocity.linvel.z = direction.z * speed;
        // also rotate
        velocity.angvel.x = direction.x * speed;
        velocity.angvel.z = direction.z * speed;
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
                let damage = DAMAGE_PER_SECOND * time.delta_seconds();
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
    mut events: EventReader<FromClient<PlayerAttack>>,
    mut player_query: Query<(Entity, &PlayerOwner, &Transform, &mut Health, &mut DamageFlash), With<Player>>,
    mut zombie_query: Query<(Entity, &Transform), With<Zombie>>,
    mut commands: Commands,
) {
    const ATTACK_RANGE: f32 = 2.0;
    const PLAYER_DAMAGE: f32 = 10.0;

    for FromClient { client_id, .. } in events.read() {
        let mut attacker_pos: Option<Vec3> = None;
        let mut attacker_entity: Option<Entity> = None;

        // Find the attacking player
        for (entity, owner, transform, _, _) in &player_query {
            if owner.0 == *client_id {
                attacker_pos = Some(transform.translation);
                attacker_entity = Some(entity);
                break;
            }
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
            damage_flash.timer -= time.delta_seconds();
            if damage_flash.timer < 0.0 {
                damage_flash.timer = 0.0;
            }
        }
    }
}

fn remove_dead_players(
    mut commands: Commands,
    player_query: Query<(Entity, &Health, &PlayerOwner), With<Player>>,
) {
    for (entity, health, owner) in &player_query {
        if health.current <= 0.0 {
            println!("Removing dead player (Client ID: {:?})", owner.0);
            commands.entity(entity).despawn_recursive();
        }
    }
}
