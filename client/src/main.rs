use bevy::prelude::*;
use std::{net::UdpSocket, time::SystemTime};

use bevy_renet::{
    renet::{transport::ClientAuthentication, ConnectionConfig, RenetClient},
    transport::NetcodeClientPlugin,
    RenetClientPlugin,
};
use renet::transport::{NetcodeClientTransport, NetcodeTransportError, NETCODE_USER_DATA_BYTES};

// This id needs to be the same as the server is using
const PROTOCOL_ID: u64 = 2878;

fn main() {
    // Get username from stdin args
    let args = std::env::args().collect::<Vec<String>>();
    let username = &args[1];

    let (client, transport) = new_renet_client(&username).unwrap();
    App::new()
        // Lets add a nice dark grey background color
        .insert_resource(ClearColor(Color::hex("282828").unwrap()))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                // Adding the username to the window title makes debugging a whole lot easier.
                title: format!("TricTrac <{}>", username),
                resolution: (480.0, 540.0).into(),
                ..default()
            }),
            ..default()
        }))
        // Renet setup
        .add_plugins(RenetClientPlugin)
        .add_plugins(NetcodeClientPlugin)
        .insert_resource(client)
        .insert_resource(transport)
        .add_systems(Update, panic_on_error_system)
        .run();
}

////////// RENET NETWORKING //////////
// Creates a RenetClient thats already connected to a server.
// Returns an Err if connection fails
fn new_renet_client(username: &String) -> anyhow::Result<(RenetClient, NetcodeClientTransport)> {
    let client = RenetClient::new(ConnectionConfig::default());
    let server_addr = "127.0.0.1:5000".parse()?;
    let socket = UdpSocket::bind("127.0.0.1:0")?;
    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
    let client_id = current_time.as_millis() as u64;

    // Place username in user data
    let mut user_data = [0u8; NETCODE_USER_DATA_BYTES];
    if username.len() > NETCODE_USER_DATA_BYTES - 8 {
        panic!("Username is too big");
    }
    user_data[0..8].copy_from_slice(&(username.len() as u64).to_le_bytes());
    user_data[8..username.len() + 8].copy_from_slice(username.as_bytes());

    let authentication = ClientAuthentication::Unsecure {
        server_addr,
        client_id,
        user_data: Some(user_data),
        protocol_id: PROTOCOL_ID,
    };
    let transport = NetcodeClientTransport::new(current_time, authentication, socket).unwrap();

    Ok((client, transport))
}

// If any error is found we just panic
fn panic_on_error_system(mut renet_error: EventReader<NetcodeTransportError>) {
    for e in renet_error.iter() {
        panic!("{}", e);
    }
}
