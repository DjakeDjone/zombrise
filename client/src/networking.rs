//! Client networking setup and input management.

use bevy::prelude::*;
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::SystemTime;

use crate::startup_screen::ServerConfig;
use zombrise_shared::players::player::{MyClientId, Player, PlayerOwner};

/// Setup client networking connection
pub fn setup_client(mut commands: Commands, server_config: Res<ServerConfig>) {
    #[cfg(target_arch = "wasm32")]
    {
        // Networking disabled on WASM
        return;
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        use lightyear::prelude::client::{NetcodeClient, NetcodeConfig};
        use lightyear::prelude::Authentication;
        use lightyear_udp::UdpIo;

        let server_addr: SocketAddr = server_config
            .url
            .to_socket_addrs()
            .expect("Failed to resolve server address")
            .find(|addr| addr.is_ipv4()) // Prefer IPv4
            .or_else(|| server_config.url.to_socket_addrs().ok()?.next())
            .expect("No address found for server");

        info!("Connecting to server at: {}", server_addr);

        let current_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        let client_id = current_time.as_millis() as u64;

        // Create authentication with Manual mode for testing
        let auth = Authentication::Manual {
            server_addr,
            client_id,
            private_key: [0u8; 32], // Match server's private key
            protocol_id: 0,         // Match server's protocol id
        };

        // Create NetcodeClient with authentication
        let netcode_config = NetcodeConfig::default();
        match NetcodeClient::new(auth, netcode_config) {
            Ok(netcode_client) => {
                // Spawn the client networking entity with LocalAddr for UDP binding
                use lightyear::prelude::{LocalAddr, ReplicationReceiver, ReplicationSender};
                let client_local_addr = SocketAddr::new(
                    std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
                    0, // Let OS assign an available port
                );

                let client_entity = commands
                    .spawn((
                        Name::new("NetworkClient"),
                        LocalAddr(client_local_addr),
                        netcode_client,
                        UdpIo::default(),
                        ReplicationReceiver::default(),
                        ReplicationSender::default(),
                    ))
                    .id();

                // Trigger Connect event to initiate connection
                use lightyear::prelude::client::Connect;
                commands.trigger(Connect {
                    entity: client_entity,
                });
            }
            Err(e) => {
                error!("Failed to create NetcodeClient: {:?}", e);
            }
        }

        // Set the client ID immediately so we can identify our player
        commands.insert_resource(MyClientId(client_id));
    }
}

/// Add input manager components to local player
pub fn add_input_manager(
    mut commands: Commands,
    player_query: Query<(Entity, &PlayerOwner), With<Player>>,
    my_client_id: Option<Res<MyClientId>>,
    input_query: Query<
        Entity,
        With<lightyear::prelude::input::native::InputMarker<zombrise_shared::protocol::GameInput>>,
    >,
) {
    let Some(my_client_id) = my_client_id else {
        return;
    };

    // Add InputMarker to the local player entity so inputs are attached to it
    for (entity, owner) in &player_query {
        // Only add input components if they are not already present
        if owner.0 == my_client_id.0 && input_query.get(entity).is_err() {
            use lightyear::prelude::input::native::{ActionState, InputMarker};
            use zombrise_shared::protocol::GameInput;

            commands.entity(entity).insert((
                InputMarker::<GameInput>::default(),
                ActionState::<GameInput>::default(),
            ));
        }
    }
}
