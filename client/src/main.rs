use crate::network::start;

mod game_grid;
mod network;

fn main() {
    match start() {
        Ok(_) => println!("WASM application started successfully!"),
        Err(e) => eprintln!("Error starting WASM application: {:?}", e),
    }
}