use bevy::prelude::*;
use bevy::scene::ScenePlugin;
use bevy_rapier3d::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::{
    RenetChannelsExt, RepliconRenetPlugins,
    renet::{
        ConnectionConfig, RenetServer,
        transport::{NetcodeServerTransport, ServerAuthentication, ServerConfig},
    },
};
use dragon_queen::players::player::{Player, PlayerOwner};
use dragon_queen::shared::{MapMarker, MovePlayer, SharedPlugin, TreeMarker};
use dragon_queen::zombie::zombie::Zombie;
use rand::Rng;
use std::{
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    time::SystemTime,
};

#[derive(Resource)]
struct ZombieSpawnTimer(Timer);

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .add_plugins(HierarchyPlugin)
        .add_plugins(TransformPlugin)
        .add_plugins(ScenePlugin)
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
        .add_systems(FixedUpdate, (handle_move_player, zombie_movement))
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
    let public_addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 5000);
    let socket = UdpSocket::bind(public_addr).unwrap();
    let server_config = ServerConfig {
        current_time,
        max_clients: 10,
        protocol_id: 0,
        public_addresses: vec![public_addr],
        authentication: ServerAuthentication::Unsecure,
    };

    let transport = NetcodeServerTransport::new(server_config, socket).unwrap();

    commands.insert_resource(server);
    commands.insert_resource(transport);

    // Add ground
    commands.spawn((
        MapMarker,
        Replicated,
        SpatialBundle::from_transform(Transform::from_xyz(0.0, -0.55, 0.0)),
        Collider::cylinder(0.05, 28.0),
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
                    Replicated,
                    Transform::from_xyz(0.0, 3.0, 0.0),
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
            Transform::from_xyz(x, 5.0, z),
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
    let mut rng = rand::rng();

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
                continue;
            }
        }

        // Random movement
        if rng.random_bool(0.02) {
            let x = rng.random_range(-1.0..1.0);
            let z = rng.random_range(-1.0..1.0);
            let dir = Vec3::new(x, 0.0, z).normalize_or_zero();
            velocity.linvel.x = dir.x * speed * 0.5;
            velocity.linvel.z = dir.z * speed * 0.5;
        }
    }
}
