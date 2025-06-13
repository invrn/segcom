// Remove the following 3 lines to enable compiler checkings
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

use axum::{
    extract::Extension,
    response::{sse::Event, Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use futures::stream::StreamExt;
use rand::{seq::IteratorRandom, SeedableRng};
use risc0_zkvm::Digest;
use std::{
    collections::HashMap,
    error::Error,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

use fleetcore::{BaseJournal, Command, CommunicationData, FireJournal, ReportJournal};
use methods::{FIRE_ID, JOIN_ID, REPORT_ID, WAVE_ID, WIN_ID};

struct Player {
    name: String,
    current_state: Digest,
}
struct Game {
    pmap: HashMap<String, Player>,
    next_player: Option<String>,
    next_report: Option<String>,
    last_shot_pos: Option<u8>,
    last_player: Option<String>,
}

#[derive(Clone)]
struct SharedData {
    tx: broadcast::Sender<String>,
    gmap: Arc<Mutex<HashMap<String, Game>>>,
    rng: Arc<Mutex<rand::rngs::StdRng>>,
}

#[tokio::main]
async fn main() {
    // Create a broadcast channel for log messages
    let (tx, _rx) = broadcast::channel::<String>(100);
    let shared = SharedData {
        tx: tx,
        gmap: Arc::new(Mutex::new(HashMap::new())),
        rng: Arc::new(Mutex::new(rand::rngs::StdRng::from_entropy())),
    };

    // Build our application with a route

    let app = Router::new()
        .route("/", get(index))
        .route("/logs", get(logs))
        .route("/chain", post(smart_contract))
        .layer(Extension(shared));

    // Run our app with hyper
    //let addr = SocketAddr::from(([127, 0, 0, 1], 3001));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));
    println!("Listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// Handler to serve the HTML page
async fn index() -> Html<&'static str> {
    Html(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Blockchain Emulator</title>
        </head>
        <body>
            <h1>Registered Transactions</h1>          
            <ul id="logs"></ul>
            <script>
                const eventSource = new EventSource('/logs');
                eventSource.onmessage = function(event) {
                    const logs = document.getElementById('logs');
                    const log = document.createElement('li');
                    log.textContent = event.data;
                    logs.appendChild(log);
                };
            </script>
        </body>
        </html>
        "#,
    )
}

// Handler to manage SSE connections
#[axum::debug_handler]
async fn logs(Extension(shared): Extension<SharedData>) -> impl IntoResponse {
    let rx = BroadcastStream::new(shared.tx.subscribe());
    let stream = rx.filter_map(|result| async move {
        match result {
            Ok(msg) => Some(Ok(Event::default().data(msg))),
            Err(_) => Some(Err(Box::<dyn Error + Send + Sync>::from("Error"))),
        }
    });

    axum::response::sse::Sse::new(stream)
}

fn xy_pos(pos: u8) -> String {
    let x = pos % 10;
    let y = pos / 10;
    format!("{}{}", (x + 65) as char, y)
}

async fn smart_contract(
    Extension(shared): Extension<SharedData>,
    Json(input_data): Json<CommunicationData>,
) -> String {
    match input_data.cmd {
        Command::Join => handle_join(&shared, &input_data),
        Command::Fire => handle_fire(&shared, &input_data),
        Command::Report => handle_report(&shared, &input_data),
        Command::Wave => handle_wave(&shared, &input_data),
        Command::Win => handle_win(&shared, &input_data),
    }
}

fn handle_join(shared: &SharedData, input_data: &CommunicationData) -> String {
    if input_data.receipt.verify(JOIN_ID).is_err() {
        shared
            .tx
            .send("Attempting to join game with invalid receipt".to_string())
            .unwrap();
        return "Could not verify receipt".to_string();
    }
    let data: BaseJournal = input_data.receipt.journal.decode().unwrap();
    let mut gmap = shared.gmap.lock().unwrap();
    let game = gmap.entry(data.gameid.clone()).or_insert(Game {
        pmap: HashMap::new(),
        next_player: Some(data.fleet.clone()),
        next_report: None,
        last_shot_pos: None,
        last_player: None,
    });
    let player_inserted = game
        .pmap
        .entry(data.fleet.clone())
        .or_insert_with(|| Player {
            name: data.fleet.clone(),
            current_state: data.board.clone(),
        })
        .name
        == data.fleet;
    let mesg = if player_inserted {
        format!("Player {} joined game {}", data.fleet, data.gameid)
    } else {
        format!("Player already in game {}", data.gameid)
    };
    shared.tx.send(mesg).unwrap();
    "OK".to_string()
}

fn handle_fire(shared: &SharedData, input_data: &CommunicationData) -> String {
    // Verify the receipt
    if input_data.receipt.verify(FIRE_ID).is_err() {
        shared
            .tx
            .send("Attempting to fire with invalid receipt".to_string())
            .unwrap();
        return "Could not verify receipt".to_string();
    }

    // Decode the FireJournal from the receipt
    let data: FireJournal = input_data.receipt.journal.decode().unwrap();

    // Lock the game map to update the state
    let mut gmap = shared.gmap.lock().unwrap();

    // Find the game by game ID
    let game = match gmap.get_mut(&data.gameid) {
        Some(game) => game,
        None => {
            let msg = format!("Game {} not found", data.gameid);
            shared.tx.send(msg.clone()).unwrap();
            return msg;
        }
    };

    // Find the firing player by fleet ID
    let player = match game.pmap.get_mut(&data.fleet) {
        Some(player) => player,
        None => {
            let msg = format!(
                "Firing fleet {} not found in game {}",
                data.fleet, data.gameid
            );
            shared.tx.send(msg.clone()).unwrap();
            return msg;
        }
    };

    // Enforce turn order
    match &game.next_player {
        Some(expected_player) => {
            if expected_player != &data.fleet {
                let msg = format!(
                    "It's not {}'s turn to fire in game {}. It's {}'s turn.",
                    data.fleet, data.gameid, expected_player
                );
                shared.tx.send(msg.clone()).unwrap();
                return msg;
            }
        }
        None => {
            let msg = format!(
                "No player is allowed to fire right now in game {}. Awaiting report.",
                data.gameid
            );
            shared.tx.send(msg.clone()).unwrap();
            return msg;
        }
    }

    // Ensure the current state matches the board hash in the fire request
    if player.current_state != data.board {
        let msg = format!(
            "Invalid fire: board hash does not match stored state for fleet {} in game {}",
            data.fleet, data.gameid
        );
        shared.tx.send(msg.clone()).unwrap();
        return msg;
    }

    // Find the target player by fleet ID
    let target_player = match game.pmap.get_mut(&data.target) {
        Some(player) => player,
        None => {
            let msg = format!(
                "Target fleet {} not found in game {}",
                data.target, data.gameid
            );
            shared.tx.send(msg.clone()).unwrap();
            return msg;
        }
    };

    // Set up for the next report
    game.next_report = Some(data.target.clone());
    game.next_player = None;
    game.last_player = Some(data.fleet.clone());

    // Broadcast the result of the fire action
    let pos = xy_pos(data.pos);
    let msg = format!(
        "Player {} fired at {}'s fleet at position {} in game {}",
        data.fleet, data.target, pos, data.gameid
    );
    shared.tx.send(msg.clone()).unwrap();

    game.last_shot_pos = Some(data.pos); //Store the last shot position
                                         // Return success
    "OK".to_string()
}

fn handle_report(shared: &SharedData, input_data: &CommunicationData) -> String {
    // Verify the receipt
    if input_data.receipt.verify(REPORT_ID).is_err() {
        shared
            .tx
            .send("Attempting to report with invalid receipt".to_string())
            .unwrap();
        return "Could not verify receipt".to_string();
    }

    // Decode the ReportJournal from the receipt
    let data: ReportJournal = input_data.receipt.journal.decode().unwrap();

    // Lock the game map to update the state
    let mut gmap = shared.gmap.lock().unwrap();

    // Find the game by game ID
    let game = match gmap.get_mut(&data.gameid) {
        Some(game) => game,
        None => {
            let msg = format!("Game {} not found", data.gameid);
            shared.tx.send(msg.clone()).unwrap();
            return msg;
        }
    };

    // Find the reporting player by fleet ID
    let player = match game.pmap.get_mut(&data.fleet) {
        Some(player) => player,
        None => {
            let msg = format!(
                "Reporting fleet {} not found in game {}",
                data.fleet, data.gameid
            );
            shared.tx.send(msg.clone()).unwrap();
            return msg;
        }
    };

    // Enforce report order
    if let Some(expected_reporter) = &game.next_report {
        if expected_reporter != &data.fleet {
            let msg = format!(
                "It's not {}'s turn to report in game {}. It's {}'s turn.",
                data.fleet, data.gameid, expected_reporter
            );
            shared.tx.send(msg.clone()).unwrap();
            return msg;
        }
    } else {
        let msg = format!("No report expected at this time in game {}.", data.gameid);
        shared.tx.send(msg.clone()).unwrap();
        return msg;
    }
    println!(
        "DEBUG: Board currently in blockchain {}: {:?}",
        data.fleet, data.board
    );
    println!(
        "DEBUG: Player current state in blockchain {}: {:?}",
        data.fleet, player.current_state
    );

    // Check if the last shot position is the same as the one being reported
    if let Some(last_pos) = game.last_shot_pos {
        if last_pos != data.pos {
            let msg = format!(
                "Invalid report: last shot position {} does not match reported position {} for fleet {} in game {}",
                xy_pos(last_pos), xy_pos(data.pos), data.fleet, data.gameid
            );
            shared.tx.send(msg.clone()).unwrap();
            return msg;
        }
    } else {
        let msg = format!(
            "Invalid report: no last shot position recorded for fleet {} in game {}",
            data.fleet, data.gameid
        );
        shared.tx.send(msg.clone()).unwrap();
        return msg;
    }

    // CHECK: Ensure the current state matches the board hash in the report
    // This check ensures that the player is reporting based on the correct board state.
    if player.current_state != data.board {
        let msg = format!(
            "Invalid report: board hash does not match stored state for fleet {} in game {}",
            data.fleet, data.gameid
        );
        shared.tx.send(msg.clone()).unwrap();
        return msg;
    }

    // Update the player's state with the new board hash (next_board)
    player.current_state = data.next_board.clone();

    // After a valid report, set the next player to the reporter
    game.next_player = Some(data.fleet.clone());
    game.next_report = None;

    // Broadcast the result of the report action
    let pos = xy_pos(data.pos);
    let msg = format!(
        "Player {} reported result '{}' at position {} in game {}",
        data.fleet, data.report, pos, data.gameid
    );
    shared.tx.send(msg.clone()).unwrap();

    // Return success
    "OK".to_string()
}

fn handle_wave(shared: &SharedData, input_data: &CommunicationData) -> String {
    // Verify the receipt
    if input_data.receipt.verify(WAVE_ID).is_err() {
        shared
            .tx
            .send("Attempting to wave with invalid receipt".to_string())
            .unwrap();
        return "Could not verify receipt".to_string();
    }

    // Decode the BaseJournal from the receipt
    let data: BaseJournal = input_data.receipt.journal.decode().unwrap();

    // Lock the game map to update the state
    let mut gmap = shared.gmap.lock().unwrap();

    // Find the game by game ID
    let game = match gmap.get_mut(&data.gameid) {
        Some(game) => game,
        None => {
            let msg = format!("Game {} not found", data.gameid);
            shared.tx.send(msg.clone()).unwrap();
            return msg;
        }
    };
    // Find the reporting player by fleet ID
    let player = match game.pmap.get_mut(&data.fleet) {
        Some(player) => player,
        None => {
            let msg = format!(
                "Reporting fleet {} not found in game {}",
                data.fleet, data.gameid
            );
            shared.tx.send(msg.clone()).unwrap();
            return msg;
        }
    };

    // This check ensures that the player is reporting based on the correct board state.
    if player.current_state != data.board {
        let msg = format!(
            "Invalid report: board hash does not match stored state for fleet {} in game {}",
            data.fleet, data.gameid
        );
        shared.tx.send(msg.clone()).unwrap();
        return msg;
    }
    // Enforce turn order
    if let Some(expected_player) = &game.next_player {
        if expected_player != &data.fleet {
            let msg = format!(
                "It's not {}'s turn to wave in game {}. It's {}'s turn.",
                data.fleet, data.gameid, expected_player
            );
            shared.tx.send(msg.clone()).unwrap();
            return msg;
        }
    } else {
        let msg = format!(
            "No player is allowed to wave right now in game {}. Awaiting report.",
            data.gameid
        );
        shared.tx.send(msg.clone()).unwrap();
        return msg;
    }

    // Broadcast the wave action
    let msg = format!(
        "Player {} waved their turn on game {}",
        data.fleet, data.gameid
    );
    shared.tx.send(msg.clone()).unwrap();

    // If the player waves their turn, we need to set the next player which is the last player
    if let Some(last_player) = &game.last_player {
        game.next_player = Some(last_player.clone());
    } else {
        // If no last player, we can't set the next player
        let msg = format!(
            "No last player found to set next player in game {}",
            data.gameid
        );
        shared.tx.send(msg.clone()).unwrap();
        return msg;
    }

    // Set last player to current player
    game.last_player = Some(data.fleet.clone());
    // Return success
    "OK".to_string()
}

fn handle_win(shared: &SharedData, input_data: &CommunicationData) -> String {
    // Verify the receipt
    if input_data.receipt.verify(WIN_ID).is_err() {
        shared
            .tx
            .send("Attempting to claim win with invalid receipt".to_string())
            .unwrap();
        return "Could not verify receipt".to_string();
    }

    // Decode the BaseJournal from the receipt
    let data: BaseJournal = input_data.receipt.journal.decode().unwrap();
    // Lock the game map to update the state
    let mut gmap = shared.gmap.lock().unwrap();

    // Find the game by game ID
    let game = match gmap.get_mut(&data.gameid) {
        Some(game) => game,
        None => {
            let msg = format!("Game {} not found", data.gameid);
            shared.tx.send(msg.clone()).unwrap();
            return msg;
        }
    };
    // Find the reporting player by fleet ID
    let player = match game.pmap.get_mut(&data.fleet) {
        Some(player) => player,
        None => {
            let msg = format!(
                "Reporting fleet {} not found in game {}",
                data.fleet, data.gameid
            );
            shared.tx.send(msg.clone()).unwrap();
            return msg;
        }
    };

    // This check ensures that the player is reporting based on the correct board state.
    if player.current_state != data.board {
        let msg = format!(
            "Invalid report: board hash does not match stored state for fleet {} in game {}",
            data.fleet, data.gameid
        );
        shared.tx.send(msg.clone()).unwrap();
        return msg;
    }

    // Broadcast the win claim
    let msg = format!(
        "Player {} claims victory in game {}!",
        data.fleet, data.gameid
    );
    shared.tx.send(msg.clone()).unwrap();

    "OK".to_string()
}
