use log::{info, trace};
use renet::{RenetConnectionConfig, RenetServer, ServerAuthentication, ServerConfig, ServerEvent};
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant, SystemTime};

// Only clients that can provide the same PROTOCOL_ID that the server is using will be able to connect.
// This can be used to make sure players use the most recent version of the client for instance.
pub const PROTOCOL_ID: u64 = 2878;

fn main() {
    env_logger::init();

    let server_addr: SocketAddr = "127.0.0.1:5000".parse().unwrap();
    let mut server: RenetServer = RenetServer::new(
        // Pass the current time to renet, so it can use it to order messages
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap(),
        // Pass a server configuration specifying that we want to allow only 2 clients to connect
        // and that we don't want to authenticate them. Everybody is welcome!
        ServerConfig::new(2, PROTOCOL_ID, server_addr, ServerAuthentication::Unsecure),
        // Pass the default connection configuration. 
        // This will create a reliable, unreliable and blocking channel.
        // We only actually need the reliable one, but we can just not use the other two.
        RenetConnectionConfig::default(),
        UdpSocket::bind(server_addr).unwrap(),
    )
    .unwrap();

    trace!("❂ TricTrac server listening on {}", server_addr);

    let mut last_updated = Instant::now();
    loop {
        // Update server time
        let now = Instant::now();
        server.update(now - last_updated).unwrap();
        last_updated = now;

        // Receive connection events from clients
        while let Some(event) = server.get_event() {
            match event {
                ServerEvent::ClientConnected(id, _user_data) => {
                    info!("🎉 Client {} connected.", id);
                }
                ServerEvent::ClientDisconnected(id) => {
                    info!("👋 Client {} disconnected", id);
                }
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
