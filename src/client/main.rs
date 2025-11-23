use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::{
    RenetChannelsExt,
    renet::{
        ConnectionConfig,
        transport::{ClientAuthentication, NetcodeClientTransport},
        RenetClient,
    },
    RepliconRenetPlugins,
};
use dragon_queen::players::player::{MainCamera, Player, PlayerOwner, handle_input};
use dragon_queen::zombie::zombie::{control_zombie_animation, setup_zombie_animation, spawn_zombie};
use dragon_queen::shared::SharedPlugin;
use std::{net::{Ipv4Addr, SocketAddr, UdpSocket}, time::SystemTime};

#[derive(Resource)]
struct MyClientId(u64);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RepliconPlugins)
        .add_plugins(RepliconRenetPlugins)
        .add_plugins(SharedPlugin)
        .add_systems(Startup, (setup, setup_client, spawn_zombie))
        .add_systems(
            Update,
            (
                handle_input,
                camera_follow,
                spawn_player_visuals,
                setup_zombie_animation,
                control_zombie_animation,
            ),
        )
        .run();
}

fn setup_client(mut commands: Commands, network_channels: Res<RepliconChannels>) {
    let server_channels_config = network_channels.get_server_configs();
    let client_channels_config = network_channels.get_client_configs();

    let client = RenetClient::new(ConnectionConfig {
        server_channels_config,
        client_channels_config,
        ..Default::default()
    });

    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
    let client_id = current_time.as_millis() as u64;
    let server_addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 5000);
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let authentication = ClientAuthentication::Unsecure {
        client_id,
        protocol_id: 0,
        server_addr,
        user_data: None,
    };

    let transport = NetcodeClientTransport::new(current_time, authentication, socket).unwrap();

    commands.insert_resource(client);
    commands.insert_resource(transport);
    commands.insert_resource(MyClientId(client_id));
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        MainCamera,
    ));

    // Light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    let map_radius = 15.0;

    // Ground
    commands.spawn(PbrBundle {
        mesh: meshes.add(Cylinder::new(map_radius, 0.1)),
        material: materials.add(Color::WHITE),
        ..default()
    });
}

fn spawn_player_visuals(
    mut commands: Commands,
    query: Query<Entity, Added<Player>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for entity in query.iter() {
        commands.entity(entity).insert(PbrBundle {
            mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
            material: materials.add(Color::srgb(0.8, 0.7, 0.6)),
            ..default()
        });
    }
}

fn camera_follow(
    player_query: Query<(&Transform, &PlayerOwner), (With<Player>, Without<MainCamera>)>,
    mut camera_query: Query<&mut Transform, With<MainCamera>>,
    my_client_id: Res<MyClientId>,
) {
    for (player_transform, owner) in player_query.iter() {
        if owner.0.get() == my_client_id.0 {
             if let Ok(mut camera_transform) = camera_query.get_single_mut() {
                let offset = Vec3::new(0.0, 5.0, 10.0);
                camera_transform.translation = player_transform.translation + offset;
                camera_transform.look_at(player_transform.translation, Vec3::Y);
            }
        }
    }
}
