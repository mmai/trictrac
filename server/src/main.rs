use log::{info, trace};
use std::net::{SocketAddr, UdpSocket, IpAddr, Ipv4Addr};
use std::time::{Duration, Instant, SystemTime};

use renet::{
    transport::{
        NetcodeServerTransport, ServerAuthentication, ServerConfig, NETCODE_USER_DATA_BYTES,
    },
    ConnectionConfig, DefaultChannel, RenetClient, RenetServer, ServerEvent,
};

// Only clients that can provide the same PROTOCOL_ID that the server is using will be able to connect.
// This can be used to make sure players use the most recent version of the client for instance.
pub const PROTOCOL_ID: u64 = 2878;

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
    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
    let mut transport = NetcodeServerTransport::new(current_time, server_config, socket).unwrap();

    trace!("❂ TricTrac server listening on {}", SERVER_ADDR);

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
                    info!("🎉 Client {} connected.", client_id);
                }
                ServerEvent::ClientDisconnected { client_id, reason } => {
                    info!("👋 Client {} disconnected", client_id);
                }
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
