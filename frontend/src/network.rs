use yew::Renderer;

use crate::game_grid::{GameGridComponent, GameGridProps, WebSocketWrapper}; // Import GameGridProps

use wasm_bindgen::prelude::*;
use web_sys::WebSocket;
use std::sync::{Arc, Mutex};

use console_log;
use log::Level;

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_log::init_with_level(Level::Info).expect("Failed to initialize logger");

    // Renderer::<GameGridComponent>::new().render();
    // Create a WebSocket connection
    let ws = WebSocket::new("ws://localhost:8080")?;
    let ws = Arc::new(Mutex::new(ws)); // Wrap in Arc<Mutex> for shared ownership

    // Render the GameGridComponent with WebSocket passed as a prop
    Renderer::<GameGridComponent>::with_props(GameGridProps {
        ws: WebSocketWrapper(ws),
    }).render();

    Ok(())
}
