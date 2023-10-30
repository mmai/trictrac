use bevy::prelude::*;
use bevy_renet::RenetClientPlugin;
use std::{net::UdpSocket, time::SystemTime};

use renet::{
    transport::{ClientAuthentication, NetcodeClientTransport, NETCODE_USER_DATA_BYTES},
    ConnectionConfig, RenetClient,
};

// This id needs to be the same as the server is using
const PROTOCOL_ID: u64 = 2878;

fn main() {
    // Get username from stdin args
    let args = std::env::args().collect::<Vec<String>>();
    let username = &args[1];

    let client = RenetClient::new(ConnectionConfig::default());
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
        .insert_resource(client)
        .insert_resource(new_renet_transport(&username).unwrap())
        // .add_system(handle_renet_error)
        .run();
}

////////// RENET NETWORKING //////////
// Creates a RenetClient thats already connected to a server.
// Returns an Err if connection fails
fn new_renet_transport(username: &String) -> anyhow::Result<NetcodeClientTransport> {
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

    Ok(transport)
}

// If there's any network error we just panic 🤷‍♂️
// Ie. Client has lost connection to server, if internet is gone or server shut down etc.
// fn handle_renet_error(mut renet_error: EventReader<RenetError>) {
//     for err in renet_error.iter() {
//         panic!("{}", err);
//     }
// }
