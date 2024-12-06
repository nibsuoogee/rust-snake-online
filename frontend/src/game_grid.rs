//use std::cmp;
use futures::StreamExt;
use yew::{html, Component, Context, Html, classes, KeyboardEvent};
use rand::Rng;
use std::time::Duration;
use futures::Stream;
use yew::platform::time::interval;
use yew::Properties;
use std::sync::{Arc, Mutex};
use web_sys::WebSocket;
use serde::{Serialize, Deserialize};
use wasm_bindgen::JsValue;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::MessageEvent;

type HNum = i8;

const BOUNDARY_THICKNESS: HNum = 1;
const GRID_HEIGHT: HNum = 30;
const GRID_WIDTH: HNum = 30;
const GRID_OFFSET: HNum = BOUNDARY_THICKNESS * 2;
const TICK_TIME: u64 = 50;

pub fn start_game_tick(ms: u64) -> impl Stream<Item = ()> {
    interval(Duration::from_millis(ms))
}

/// Generate a random position within the grid
/// The position will not be on the boundary
fn get_random_position() -> PositionCoords {
    let mut rng = rand::thread_rng();
    PositionCoords::new(
        rng.gen_range(BOUNDARY_THICKNESS..=GRID_WIDTH),
        rng.gen_range(BOUNDARY_THICKNESS..=GRID_HEIGHT)
    )
}

fn get_random_direction() -> Direction {
    let mut rng = rand::thread_rng();
    match rng.gen_range(0..4) {
        0 => Direction::UP,
        1 => Direction::DOWN,
        2 => Direction::LEFT,
        _ => Direction::RIGHT,
    }
}

#[derive(Serialize, Deserialize, Debug)]
enum Direction {
    UP,
    DOWN,
    LEFT,
    RIGHT,
}

impl Clone for Direction {
    fn clone(&self) -> Direction {
        use Direction as D;
        match self {
            D::UP => D::UP,
            D::DOWN => D::DOWN,
            D::LEFT => D::LEFT,
            D::RIGHT => D::RIGHT,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct BodySegment {
    x: HNum,
    y: HNum,
    direction: Direction,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct PositionCoords {
    x: HNum,
    y: HNum,
}

impl PositionCoords {
    fn new(x: HNum, y: HNum) -> Self {
        Self {
            x,
            y,
        }
    }
}

#[derive(Clone)]
pub struct WebSocketWrapper(pub Arc<Mutex<WebSocket>>);

impl WebSocketWrapper {
    // Method to access the inner Arc<Mutex<WebSocket>>
    pub fn lock(&self) -> std::sync::LockResult<std::sync::MutexGuard<WebSocket>> {
        self.0.lock() // Delegates to the inner Mutex lock
    }
}

impl PartialEq for WebSocketWrapper {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlayerStateData {
    player_id: u64,
    body_segments: Vec<BodySegment>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MapState {
    food_positions: Vec<PositionCoords>,
}

fn send_server_message(ws: &WebSocket, message: &str) -> Result<(), JsValue> {
    ws.send_with_str(message)
        .map_err(|e| JsValue::from_str(&format!("WebSocket send error: {:?}", e)))?;
    Ok(())
}

fn send_eat_food_message(ws: &WebSocket, player_id: u64, food_position: PositionCoords) -> Result<(), JsValue> {
    let food_position_json: String = serde_json::to_string(&food_position).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let server_message = ServerMessage {
        message_type: "eat_food".to_string(),
        player_id,
        message: food_position_json,
    };
    
    let message = serde_json::to_string(&server_message).map_err(|e| JsValue::from_str(&e.to_string()))?;
    send_server_message(ws, &message)
}
fn send_player_data(ws: &WebSocket, player_id: u64, player_data: &PlayerStateData) -> Result<(), JsValue> {
    // Serialize the PlayerStateData struct into a JSON string
    let player_state_json = serde_json::to_string(player_data).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let server_message = ServerMessage {
        message_type: "player_state".to_string(),
        player_id,
        message: player_state_json,
    };

    let message = serde_json::to_string(&server_message).map_err(|e| JsValue::from_str(&e.to_string()))?;
    send_server_message(ws, &message)
}

#[derive(Serialize, Deserialize, Debug)]
struct ServerMessage {
    message_type: String, // "player_message" or other types
    player_id: u64,
    message: String, // The JSON string containing `PlayerStateData`
}

#[derive(Properties, PartialEq, Clone)]
pub struct GameGridProps {
    pub ws: WebSocketWrapper, // Use the wrapper instead of Arc<Mutex<WebSocket>>
}

pub struct GameGridComponent{
    x: HNum,
    y: HNum,
    current_direction: Direction,
    pending_body_segment: bool,
    score: u64,
    paused: bool,
    dead: bool,
    food_positions: Vec<PositionCoords>,
    body_segments: Vec<BodySegment>,
    ws: Option<WebSocketWrapper>, // Store WebSocket in the component state
    network_id: u64,
    player_states: Vec<PlayerStateData>,
}
pub enum Msg {
    GameTicked(()),
    HandleKeyboardEvent(KeyboardEvent),
    RestartGame(()),
    HandlePause(()),
    UpdateNetworkId(u64),
    UpdatePlayerStates(Vec<PlayerStateData>),
    UpdateMapState(MapState),
}

impl GameGridComponent {
    fn connect_to_server(&mut self, ctx: &Context<Self>) {
        if let Some(ref ws_wrapper) = self.ws {
            let ws_clone = ws_wrapper.clone();
            let link = ctx.link().clone();
    
            let on_message: Closure<dyn FnMut(MessageEvent)> = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
                if let Some(message) = event.data().as_string() {
                    match serde_json::from_str::<ServerMessage>(&message) {
                        Ok(server_message) => {
                            match server_message.message_type.as_str() {
                                "assign_id" => {
                                    // Send message to update component's network_id
                                    link.send_message(Msg::UpdateNetworkId(server_message.player_id));
                                },
                                "player_states" => {
                                    if let Ok(player_states) = serde_json::from_str::<Vec<PlayerStateData>>(&server_message.message) {
                                        // Send message to update component's player states
                                        link.send_message(Msg::UpdatePlayerStates(player_states));
                                    }
                                },
                                "map_state" => {
                                    if let Ok(map_state) = serde_json::from_str::<MapState>(&server_message.message) {
                                        // Send message to update component's player states
                                        link.send_message(Msg::UpdateMapState(map_state));
                                    }
                                },
                                _ => {}
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to deserialize ServerMessage: {}", e);
                        }
                    }
                }
            }) as Box<dyn FnMut(web_sys::MessageEvent)>);
    
            ws_clone.lock().unwrap().set_onmessage(Some(on_message.as_ref().unchecked_ref()));
            on_message.forget();
        }
    }
    fn move_up(&mut self) {
        if self.y == 0 {
            self.y = GRID_HEIGHT + GRID_OFFSET - 1;
        } else {
            self.y -= 1;
        }
    }
    fn move_down(&mut self) {
        if self.y == GRID_HEIGHT + GRID_OFFSET - 1 {
            self.y = 0;
        } else {
            self.y += 1;
        }
    }
    fn move_left(&mut self) {
        if self.x == 0 {
            self.x = GRID_WIDTH + GRID_OFFSET - 1;
        } else {
            self.x -= 1;
        }
    }
    fn move_right(&mut self) {
        if self.x == GRID_WIDTH + GRID_OFFSET - 1 {
            self.x = 0;
        } else {
            self.x += 1;
        }
    }
    fn update_direction(&mut self, dir: Direction) {
        self.current_direction = dir;
    }
    fn update_pause(&mut self, pause: bool) {
        self.paused = pause;
    }
    fn handle_keydown(&mut self, event: KeyboardEvent) {
        match event.key().as_str() {
            "ArrowUp" => self.update_direction(Direction::UP),
            "ArrowDown" => self.update_direction(Direction::DOWN),
            "ArrowLeft" => self.update_direction(Direction::LEFT),
            "ArrowRight" => self.update_direction(Direction::RIGHT),
            //" " => self.update_pause(!self.paused), // spacebar
            _ => {}
        }
    }
    fn handle_tick(&mut self) {
        let pos = PositionCoords::new(self.x, self.y);
        match self.current_direction {
            Direction::UP => self.move_up(),
            Direction::DOWN => self.move_down(),
            Direction::LEFT => self.move_left(),
            Direction::RIGHT => self.move_right(),
        }
        
        // Add pending body segment if needed
        if self.pending_body_segment {
            self.body_segments.push(BodySegment {
                x: pos.x,
                y: pos.y,
                direction: self.current_direction.clone(),
            });
            self.pending_body_segment = false;
        }
        
        // update body segments
        for i in (0..self.body_segments.len()).rev() {
            if i == 0 {
                self.body_segments[i].x = pos.x;
                self.body_segments[i].y = pos.y;
                self.body_segments[i].direction = self.current_direction.clone();
            } else {
                self.body_segments[i].x = self.body_segments[i - 1].x;
                self.body_segments[i].y = self.body_segments[i - 1].y;
                self.body_segments[i].direction = self.body_segments[i - 1].direction.clone();
            }
        }

        // Handle the game tick, update positions, etc.
        let player_data: PlayerStateData = PlayerStateData {
            player_id: self.network_id,//"player1".to_string(), // Replace with actual player ID
            body_segments: {
                let mut segments = self.body_segments.clone();
                segments.push(BodySegment {
                    x: self.x,
                    y: self.y,
                    direction: self.current_direction.clone(),
                });
                segments
            },
        };

        // Send the player data to the server if an id has been 
        // assigned by the server
        if self.network_id == 0 {
            return;
        }
        if let Some(ref ws_wrapper) = &self.ws {
            // Now safely lock the WebSocketWrapper
            if let Ok(ws) = ws_wrapper.0.lock() {
                // If lock succeeds, send player data
                if send_player_data(&ws, self.network_id, &player_data).is_err() {
                    log::error!("Failed to send player data through WebSocket");
                }
            } else {
                log::error!("Failed to lock WebSocket");
            }
        } else {
            log::error!("WebSocket is not initialized");
        }
    }
    fn is_game_over(&self) -> bool {
        // is_boundary(self.x, self.y) ||
        self.is_body_segment(self.x, self.y) || self.is_other_player_segment(self.x, self.y)
    }
    fn increment_score(&mut self) {
        self.score += 1;
    }
    fn restart(&mut self) {
        let mut rng = rand::thread_rng();
        self.x = rng.gen_range(BOUNDARY_THICKNESS..=GRID_WIDTH);
        self.y = rng.gen_range(BOUNDARY_THICKNESS..=GRID_HEIGHT);
        self.current_direction = get_random_direction();
        self.score = 0;
        self.paused = false;
        self.dead = false;
        self.body_segments.clear();
    }
    fn is_food_coordinate(&self, x: HNum, y: HNum) -> bool {
        self.food_positions.iter().any(|pos| pos.x == x && pos.y == y)
    }
    fn is_food_eaten(&self) -> bool {
        let is_food = self.is_food_coordinate(self.x, self.y);
        
        if is_food {
            if let Some(ref ws_wrapper) = &self.ws {
                // Now safely lock the WebSocketWrapper
                if let Ok(ws) = ws_wrapper.0.lock() {
                    // If lock succeeds, send player data
                    let food_position = PositionCoords::new(self.x, self.y);
                    if send_eat_food_message(&ws, self.network_id, food_position).is_err() {
                        log::error!("Failed to send player data through WebSocket");
                    }
                } else {
                    log::error!("Failed to lock WebSocket");
                }
            } else {
                log::error!("WebSocket is not initialized");
            }
        }
        
        is_food
    }
    fn is_body_segment(&self, x: HNum, y: HNum) -> bool {
        self.body_segments.iter().any(|segment| segment.x == x && segment.y == y)
    }
    fn is_other_player_segment(&self, x: HNum, y: HNum) -> bool {
        self.player_states.iter().any(|player_state| {
            player_state.body_segments.iter().any(|segment| segment.x == x && segment.y == y)
        })
    }
}

fn is_boundary(x: HNum, y: HNum) -> bool {
    x < BOUNDARY_THICKNESS || x >= GRID_WIDTH + BOUNDARY_THICKNESS || y < BOUNDARY_THICKNESS || y >= GRID_HEIGHT + BOUNDARY_THICKNESS
}

impl Component for GameGridComponent {
    type Message = Msg;
    type Properties = GameGridProps; // Use the new GameGridProps

    fn create(ctx: &Context<Self>) -> Self {
        let game_tick = start_game_tick(TICK_TIME);
        ctx.link().send_stream(game_tick.map(Msg::GameTicked));
        let spawn_position = get_random_position();

        let mut component = Self {
            x: spawn_position.x,
            y: spawn_position.y,
            current_direction: Direction::RIGHT,
            pending_body_segment: false,
            score: 0,
            paused: false,
            dead: false,
            food_positions: vec![],
            body_segments: vec![],
            ws: Some(ctx.props().ws.clone()), // Store the WebSocket from props
            network_id: 0,
            player_states: Vec::new(),
        };

        component.connect_to_server(ctx);
        component
    }
    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::GameTicked(_) => {
                if self.paused {
                    return false; // don't re-render if paused
                }
                if self.is_game_over() {
                    self.update_pause(true);
                    self.dead = true;
                } else {
                    self.handle_tick();
                }
                if self.is_food_eaten() {
                    //self.respawn_food();
                    /*self.body_segments.push(BodySegment {
                        x: self.x,
                        y: self.y,
                        direction: self.current_direction.clone(),
                    });*/
                    self.pending_body_segment = true;
                    self.increment_score();
                }
            }
            Msg::HandleKeyboardEvent(event) => {
                self.handle_keydown(event);
            }
            Msg::RestartGame(_) => {
                self.restart();
            }
            Msg::HandlePause(_) => {
                self.update_pause(!self.paused);
            }
            Msg::UpdateNetworkId(id) => {
                self.network_id = id;
            }
            Msg::UpdatePlayerStates(states) => {
                self.player_states = states;
            }
            Msg::UpdateMapState(map_state) => {
                self.food_positions = map_state.food_positions;
            }
        }
        true
    }
    fn view(&self, ctx: &Context<Self>) -> Html {
        let handle_keydown = ctx.link().callback(|e: KeyboardEvent| {
            Msg::HandleKeyboardEvent(e)
        });
        let handle_restart = ctx.link().callback(|_| {
            Msg::RestartGame(())
        });

        html!(
            <div tabIndex="0" onkeydown={handle_keydown} class={classes!("app-ctn")}>
                { for (0..GRID_HEIGHT + GRID_OFFSET).map(|row| {
                    html! {
                        <div class="row" key={row}>
                            { for (0..GRID_WIDTH + GRID_OFFSET).map(|column| {
                                html! {
                                    <div key={column} class={classes!(
                                        "cell",
                                        if self.x == column && self.y == row {
                                            "cell--active"
                                        } else {
                                            ""
                                        },
                                        if is_boundary(column, row) {
                                            "cell--boundary"
                                        } else {
                                            ""
                                        },
                                        if self.is_food_coordinate(column, row) {
                                            "cell--food"
                                        } else {
                                            ""
                                        },
                                        if self.is_body_segment(column, row) {
                                            "cell--body"
                                        } else {
                                            ""
                                        },
                                        if self.is_other_player_segment(column, row) {
                                            "cell--other--player"
                                        } else {
                                            ""
                                        },
                                    )}/>
                                }
                            })}
                        </div>
                    }
                })}
                {if self.dead {
                    html! {
                        <div class={classes!("game-over-ctn")}>
                            <h1 class={classes!("game-over")}>{ "Game Over" }</h1>
                            <button class={classes!("btn")} onclick={handle_restart}>{ "Restart" }</button>
                        </div>
                    }
                } else {html!{<div></div>}}}
                
                    <div class="flex-none">
                        <a
                        href="https://github.com/mabdullahadeel/craby-snake/tree/master"
                        target="_blank"
                        rel="noopener noreferrer"
                        >
                            <div class="flex items-center p-4 space-x-4 hover:underline hover:underline-offset-4">
                                <img src="static/github-mark-white.svg" alt="A globe" class="text-zinc-700 fill-zinc-700" width="24" height="24"/>
                                <div class="flex-none">
                                    <span class="text-zinc-700">{ "Snake forked from mabdullahadeel/craby-snake" }</span>
                                </div>
                            </div>
                        </a>
                    </div>
                
         
            </div>
        )
    }
}