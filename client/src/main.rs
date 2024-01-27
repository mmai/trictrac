use std::{net::UdpSocket, time::SystemTime};

use renet::transport::{NetcodeClientTransport, NetcodeTransportError, NETCODE_USER_DATA_BYTES};
use store::{GameEvent, GameState, CheckerMove};

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_renet::{
    renet::{transport::ClientAuthentication, ConnectionConfig, RenetClient},
    transport::{client_connected, NetcodeClientPlugin},
    RenetClientPlugin,
};

#[derive(Debug, Resource)]
struct CurrentClientId(u64);

#[derive(Resource)]
struct BevyGameState(GameState);

impl Default for BevyGameState {
    fn default() -> Self {
        Self {
            0: GameState::default(),
        }
    }
}

#[derive(Resource, Deref, DerefMut)]
struct GameUIState {
    selected_tile: Option<usize>,
}

impl Default for GameUIState {
    fn default() -> Self {
        Self {
            selected_tile: None,
        }
    }
}

#[derive(Event)]
struct BevyGameEvent(GameEvent);

// This id needs to be the same as the server is using
const PROTOCOL_ID: u64 = 2878;

fn main() {
    // Get username from stdin args
    let args = std::env::args().collect::<Vec<String>>();
    let username = &args[1];

    let (client, transport, client_id) = new_renet_client(&username).unwrap();
    App::new()
        // Lets add a nice dark grey background color
        .insert_resource(ClearColor(Color::hex("282828").unwrap()))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                // Adding the username to the window title makes debugging a whole lot easier.
                title: format!("TricTrac <{}>", username),
                resolution: (1080.0, 1080.0).into(),
                ..default()
            }),
            ..default()
        }))
        // Add our game state and register GameEvent as a bevy event
        .insert_resource(BevyGameState::default())
        .insert_resource(GameUIState::default())
        .add_event::<BevyGameEvent>()
        // Renet setup
        .add_plugins(RenetClientPlugin)
        .add_plugins(NetcodeClientPlugin)
        .insert_resource(client)
        .insert_resource(transport)
        .insert_resource(CurrentClientId(client_id))
        .add_systems(Startup, setup)
        .add_systems(Update, (update_waiting_text, input, update_board, panic_on_error_system))
        .add_systems(
            PostUpdate,
            receive_events_from_server.run_if(client_connected()),
        )
        .run();
}

////////// COMPONENTS //////////
#[derive(Component)]
struct UIRoot;

#[derive(Component)]
struct WaitingText;

#[derive(Component)]
struct Board {
    squares: [Square; 26]
}

impl Default for Board {
    fn default() -> Self {
        Self {
            squares: [Square { count: 0, color: None, position: 0}; 26]
        }
    }
}

impl Board {
    fn square_at(&self, position: usize) -> Square {
       self.squares[position] 
    }
}

#[derive(Component, Clone, Copy)]
struct Square {
    count: usize,
    color: Option<bool>,
    position: usize,
}

////////// UPDATE SYSTEMS //////////
fn update_board(
    mut commands: Commands,
    game_state: Res<BevyGameState>,
    mut game_events: EventReader<BevyGameEvent>,
    asset_server: Res<AssetServer>,
) {
    for event in game_events.iter() {
        match event.0 {
            GameEvent::Move { player_id, moves } => {
                // trictrac positions, TODO : dépend de player_id
                let (x, y) = if moves.0.get_to() < 13 { (13 - moves.0.get_to(), 1) } else { (moves.0.get_to() - 13, 0)};
                let texture =
                    asset_server.load(match game_state.0.players[&player_id].color {
                        store::Color::Black => "tac.png",
                        store::Color::White => "tic.png",
                    });

            info!("spawning tictac sprite");
                commands.spawn(SpriteBundle {
                    transform: Transform::from_xyz(
                        83.0 * (x as f32 - 1.0),
                        -30.0 + 540.0 * (y as f32 - 1.0),
                        0.0,
                    ),
                    sprite: Sprite {
                        custom_size: Some(Vec2::new(83.0, 83.0)),
                        ..default()
                    },
                    texture: texture.into(),
                    ..default()
                });
            }
            _ => {}
        }
    }
}

fn update_waiting_text(mut text_query: Query<&mut Text, With<WaitingText>>, time: Res<Time>) {
    if let Ok(mut text) = text_query.get_single_mut() {
        let num_dots = (time.elapsed_seconds() as usize % 3) + 1;
        text.sections[0].value = format!(
            "Waiting for an opponent{}{}",
            ".".repeat(num_dots as usize),
            // Pad with spaces to avoid text changing width and dancing all around the screen 🕺
            " ".repeat(3 - num_dots as usize)
        );
    }
}

fn input(
    primary_query: Query<&Window, With<PrimaryWindow>>,
    // windows: Res<Windows>,
    input: Res<Input<MouseButton>>,
    game_state: Res<BevyGameState>,
    mut game_ui_state: ResMut<GameUIState>,
    mut client: ResMut<RenetClient>,
    client_id: Res<CurrentClientId>,
) {
    // We only want to handle inputs once we are ingame
    if game_state.0.stage != store::Stage::InGame {
        return;
    }

    let window = primary_query.get_single().unwrap();
    if let Some(mouse_position) = window.cursor_position() {
        // Determine the index of the tile that the mouse is currently over
        // NOTE: This calculation assumes a fixed window size.
        // That's fine for now, but consider using the windows size instead.
        let mut tile_x: usize = (mouse_position.x / 83.0).floor() as usize;
        let tile_y: usize = (mouse_position.y / 540.0).floor() as usize;
        if tile_x > 5 {
            // remove the middle bar offset
            tile_x = tile_x - 1
        } 
        // let tile = tile_x + tile_y * 12;

        // traduction en position backgammon
        let tile = if tile_y == 0 {
            13 + tile_x
        } else {
            12 - tile_x 
        };

        // If mouse is outside of board we do nothing
        if 23 < tile {
            return;
        }

        // If left mouse button is pressed, send a place tile event to the server
        if input.just_pressed(MouseButton::Left) {
            info!("select piece at tile {:?}", tile);
            if game_ui_state.selected_tile.is_some() {
                let from_tile = game_ui_state.selected_tile.unwrap();
                info!("sending movement from: {:?} to: {:?} ", from_tile, tile);
                let event = GameEvent::Move {
                    player_id: client_id.0,
                    moves: (
                        CheckerMove::new(from_tile, tile).unwrap(),
                        CheckerMove::new(from_tile, tile).unwrap()
                        )
                };
                client.send_message(0, bincode::serialize(&event).unwrap());
            }
            game_ui_state.selected_tile = if game_ui_state.selected_tile.is_some() {
                None
            } else {
                Some(tile)
            }
        }
    }
}

////////// SETUP //////////
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Tric Trac is a 2D game
    // To show 2D sprites we need a 2D camera
    commands.spawn(Camera2dBundle::default());

    // Spawn board background
    commands.spawn(SpriteBundle {
        transform: Transform::from_xyz(0.0, -30.0, 0.0),
        sprite: Sprite {
            custom_size: Some(Vec2::new(1080.0, 927.0)),
            ..default()
        },
        texture: asset_server.load("board.png").into(),
        ..default()
    });

    // Spawn pregame ui
    commands
        // A container that centers its children on the screen
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        })
        .insert(UIRoot)
        .with_children(|parent| {
            // parent.spawn(Board::default()); // panic
            parent
                .spawn(TextBundle::from_section(
                    "Waiting for an opponent...",
                    TextStyle {
                        font: asset_server.load("Inconsolata.ttf"),
                        font_size: 24.0,
                        color: Color::hex("ebdbb2").unwrap(),
                    },
                ))
                .insert(WaitingText);
        });
}

////////// RENET NETWORKING //////////
// Creates a RenetClient thats already connected to a server.
// Returns an Err if connection fails
fn new_renet_client(
    username: &String,
) -> anyhow::Result<(RenetClient, NetcodeClientTransport, u64)> {
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

    Ok((client, transport, client_id))
}

fn receive_events_from_server(
    mut client: ResMut<RenetClient>,
    mut game_state: ResMut<BevyGameState>,
    mut game_events: EventWriter<BevyGameEvent>,
) {
    while let Some(message) = client.receive_message(0) {
        // Whenever the server sends a message we know that it must be a game event
        let event: GameEvent = bincode::deserialize(&message).unwrap();
        trace!("{:#?}", event);

        // We trust the server - It's always been good to us!
        // No need to validate the events it is sending us
        game_state.0.consume(&event);

        // Send the event into the bevy event system so systems can react to it
        game_events.send(BevyGameEvent(event));
    }
}

// If any error is found we just panic
fn panic_on_error_system(mut renet_error: EventReader<NetcodeTransportError>) {
    for e in renet_error.iter() {
        panic!("{}", e);
    }
}
