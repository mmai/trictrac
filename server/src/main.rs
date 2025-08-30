use log::{info, trace, warn};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use renet::{
    transport::{
        NetcodeServerTransport, ServerAuthentication, ServerConfig, NETCODE_USER_DATA_BYTES,
    },
    ConnectionConfig, RenetServer, ServerEvent,
};

// Only clients that can provide the same PROTOCOL_ID that the server is using will be able to connect.
// This can be used to make sure players use the most recent version of the client for instance.
pub const PROTOCOL_ID: u64 = 2878;

/// Utility function for extracting a players name from renet user data
fn name_from_user_data(user_data: &[u8; NETCODE_USER_DATA_BYTES]) -> String {
    let mut buffer = [0u8; 8];
    buffer.copy_from_slice(&user_data[0..8]);
    let mut len = u64::from_le_bytes(buffer) as usize;
    len = len.min(NETCODE_USER_DATA_BYTES - 8);
    let data = user_data[8..len + 8].to_vec();
    String::from_utf8(data).unwrap()
}

fn main() {
    env_logger::init();

    let mut server = RenetServer::new(ConnectionConfig::default());

    // Setup transport layer
    const SERVER_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 5000);
    let socket: UdpSocket = UdpSocket::bind(SERVER_ADDR).unwrap();
    let server_config = ServerConfig {
        max_clients: 2,
        protocol_id: PROTOCOL_ID,
        public_addr: SERVER_ADDR,
        authentication: ServerAuthentication::Unsecure,
    };
    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let mut transport = NetcodeServerTransport::new(current_time, server_config, socket).unwrap();

    trace!("❂ TricTrac server listening on {SERVER_ADDR}");

    let mut game_state = store::GameState::default();
    let mut last_updated = Instant::now();
    loop {
        // Update server time
        let now = Instant::now();
        let delta_time = now - last_updated;
        server.update(delta_time);
        transport.update(delta_time, &mut server).unwrap();
        last_updated = now;

        // Receive connection events from clients
        while let Some(event) = server.get_event() {
            match event {
                ServerEvent::ClientConnected { client_id } => {
                    let user_data = transport.user_data(client_id).unwrap();

                    // Tell the recently joined player about the other player
                    for (player_id, player) in game_state.players.iter() {
                        let event = store::GameEvent::PlayerJoined {
                            player_id: *player_id,
                            name: player.name.clone(),
                        };
                        server.send_message(client_id, 0, bincode::serialize(&event).unwrap());
                    }

                    // Add the new player to the game
                    let event = store::GameEvent::PlayerJoined {
                        player_id: client_id,
                        name: name_from_user_data(&user_data),
                    };
                    game_state.consume(&event);

                    // Tell all players that a new player has joined
                    server.broadcast_message(0, bincode::serialize(&event).unwrap());

                    info!("🎉 Client {client_id} connected.");
                    // In TicTacTussle the game can begin once two players has joined
                    if game_state.players.len() == 2 {
                        let event = store::GameEvent::BeginGame {
                            goes_first: client_id,
                        };
                        game_state.consume(&event);
                        server.broadcast_message(0, bincode::serialize(&event).unwrap());
                        trace!("The game gas begun");
                    }
                }
                ServerEvent::ClientDisconnected {
                    client_id,
                    reason: _,
                } => {
                    // First consume a disconnect event
                    let event = store::GameEvent::PlayerDisconnected {
                        player_id: client_id,
                    };
                    game_state.consume(&event);
                    server.broadcast_message(0, bincode::serialize(&event).unwrap());
                    info!("Client {client_id} disconnected");

                    // Then end the game, since tic tac toe can't go on with a single player
                    let event = store::GameEvent::EndGame {
                        reason: store::EndGameReason::PlayerLeft {
                            player_id: client_id,
                        },
                    };
                    game_state.consume(&event);
                    server.broadcast_message(0, bincode::serialize(&event).unwrap());

                    // NOTE: Since we don't authenticate users we can't do any reconnection attempts.
                    // We simply have no way to know if the next user is the same as the one that disconnected.
                }
            }
        }

        // Receive GameEvents from clients. Broadcast valid events.
        for client_id in server.clients_id().into_iter() {
            while let Some(message) = server.receive_message(client_id, 0) {
                if let Ok(event) = bincode::deserialize::<store::GameEvent>(&message) {
                    if game_state.validate(&event) {
                        game_state.consume(&event);
                        trace!("Player {client_id} sent:\n\t{event:#?}");
                        server.broadcast_message(0, bincode::serialize(&event).unwrap());

                        // Determine if a player has won the game
                        if let Some(winner) = game_state.determine_winner() {
                            let event = store::GameEvent::EndGame {
                                reason: store::EndGameReason::PlayerWon { winner },
                            };
                            server.broadcast_message(0, bincode::serialize(&event).unwrap());
                        }
                    } else {
                        warn!("Player {client_id} sent invalid event:\n\t{event:#?}");
                    }
                }
            }
        }

        transport.send_packets(&mut server);
        thread::sleep(Duration::from_millis(50));
    }
}
