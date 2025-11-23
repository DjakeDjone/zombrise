use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::{
    RenetChannelsExt, RepliconRenetPlugins,
    renet::{
        ConnectionConfig, RenetServer,
        transport::{NetcodeServerTransport, ServerAuthentication, ServerConfig},
    },
};
use dragon_queen::players::player::{Player, PlayerOwner};
use dragon_queen::shared::{MovePlayer, SharedPlugin};
use std::{
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    time::SystemTime,
};

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugins(RepliconPlugins)
        .add_plugins(RepliconRenetPlugins)
        .add_plugins(SharedPlugin)
        .insert_resource(Time::<Fixed>::from_hz(120.0))
        .add_systems(Startup, setup_server)
        .add_systems(Update, server_event_system)
        .add_systems(FixedUpdate, handle_move_player)
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
                    Transform::from_xyz(0.0, 0.5, 0.0),
                    GlobalTransform::default(),
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
    mut query: Query<(&PlayerOwner, &mut Transform)>,
    time: Res<Time<Fixed>>,
) {
    let speed = 5.0;
    for FromClient { client_id, event } in events.read() {
        for (owner, mut transform) in &mut query {
            if owner.0 == *client_id {
                transform.translation += event.direction * speed * time.delta_seconds();
            }
        }
    }
}
