//! Networking setup for the server.

use bevy::prelude::*;
use lightyear::connection::client::Connected;
use lightyear::prelude::input::native::ActionState;
use lightyear::prelude::server::*;
use lightyear::prelude::*;
use std::net::{Ipv4Addr, SocketAddr};

use zombrise_shared::entity2::Health;
use zombrise_shared::players::player::{DamageFlash, Player, PlayerAttackCooldown, PlayerOwner};
use zombrise_shared::protocol::GameInput;

/// Setup networking - spawns the server entity with networking components
pub fn setup_networking(mut commands: Commands) {
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

/// Spawns a player when a client connects (Lightyear 0.25 pattern)
pub fn spawn_clients(
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
            avian3d::prelude::RigidBody::Dynamic,
            avian3d::prelude::Collider::capsule(0.3, 1.4),
            avian3d::prelude::LinearVelocity::ZERO,
            avian3d::prelude::AngularVelocity::ZERO,
            avian3d::prelude::LockedAxes::new()
                .lock_rotation_x()
                .lock_rotation_z(),
            avian3d::prelude::LinearDamping(0.5),
            avian3d::prelude::AngularDamping(20.0),
        ))
        .id();

    let _ = player_entity; // Suppress unused variable warning
}

/// Despawns a player when a client disconnects
pub fn despawn_clients(
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
