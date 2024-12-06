import WebSocket, { WebSocketServer } from "ws";
import { Direction, } from "./types.js";
const BOUNDARY_THICKNESS = 1;
const GRID_HEIGHT = 30;
const GRID_WIDTH = 30;
// Initialize WebSocket server
const wss = new WebSocketServer({ port: 8080 });
// Store connected players and their ids
let nextplayer_id = 1; // This is the next ID to assign
const players = new Map(); // Map of player IDs to their WebSocket connections
const mapState = {
    food_positions: [getRandomPosition()],
};
function getRandomPosition() {
    return {
        x: BOUNDARY_THICKNESS + Math.floor(Math.random() * GRID_WIDTH),
        y: BOUNDARY_THICKNESS + Math.floor(Math.random() * GRID_HEIGHT),
    };
}
// Helper function to broadcast a message to all connected players
function broadcast(data) {
    // Send to all connected players
    wss.clients.forEach((client) => {
        if (client.readyState === WebSocket.OPEN) {
            client.send(JSON.stringify(data));
        }
    });
}
function validatePlayerData(data) {
    return (typeof data.player_id === "number" &&
        Array.isArray(data.body_segments) &&
        data.body_segments.every((segment) => typeof segment.x === "number" &&
            typeof segment.y === "number" &&
            Object.values(Direction).includes(segment.direction)));
}
function savePlayerData(playerData) {
    const playerId = parseInt(playerData.player_id);
    // Check if the player exists in the map
    if (players.has(playerId)) {
        // Retrieve the current PlayerNetworkData for the player
        const playerNetworkData = players.get(playerId);
        if (playerData) {
            // Update only the stateData field
            playerNetworkData.stateData = playerData;
            // Re-insert the modified object into the map if necessary
            players.set(playerId, playerNetworkData);
        }
    }
    else {
        console.error("Player not found in the map");
    }
}
function handlePlayerState(playerData) {
    if (validatePlayerData(playerData)) {
        // Process and save player data as needed
        savePlayerData(playerData);
    }
    else {
        console.error("Invalid player data format");
        console.log("playerData: ", playerData);
    }
}
function handleAddFood() {
    mapState.food_positions.push(getRandomPosition());
}
function handleEatFood(foodPosition) {
    // if the food exists in the map, remove it
    const foodIndex = mapState.food_positions.findIndex((position) => position.x === foodPosition.x && position.y === foodPosition.y);
    if (foodIndex !== -1) {
        mapState.food_positions.splice(foodIndex, 1);
    }
    handleAddFood();
    broadcastMapState();
    return;
}
function handlePlayerMessage(jsonMessage) {
    try {
        // Parse the JSON message into a JavaScript object
        const serverMessage = JSON.parse(jsonMessage);
        // Validate the structure (optional but recommended)
        // switch case match the serverMessage.message_type to the expected values
        switch (serverMessage.message_type) {
            case "player_state":
                handlePlayerState(JSON.parse(serverMessage.message));
                break;
            case "eat_food":
                handleEatFood(JSON.parse(serverMessage.message));
                break;
            default:
                console.error("Invalid message type");
                break;
        }
    }
    catch (error) {
        console.error("Error parsing JSON message:", error);
    }
}
wss.on("connection", (ws) => {
    // Assign a unique player ID
    const player_id = nextplayer_id++;
    const stateData = {
        stateData: { player_id: player_id, body_segments: [] },
        ws: ws,
    };
    players.set(player_id, stateData);
    // Send the player their unique ID
    const serverMessage = {
        message_type: "assign_id",
        player_id: player_id,
        message: "",
    };
    //console.log(`Player ${player_id} connected. Assigned ID: ${player_id}`);
    ws.send(JSON.stringify(serverMessage));
    broadcastMapState();
    // Listen for messages from this player
    ws.on("message", (message) => {
        // update the state of the player in the server when they send their data
        handlePlayerMessage(message.toString());
    });
    // Handle when a player disconnects
    ws.on("close", () => {
        // Remove the player from the list when they disconnect
        players.delete(player_id);
    });
});
function broadcastMapState() {
    const serverMessage = {
        message_type: "map_state",
        player_id: 0,
        message: JSON.stringify(mapState),
    };
    broadcast(serverMessage);
}
function broadcastPlayerStates() {
    players.forEach((playerNetworkData, playerId) => {
        if (playerNetworkData.ws.readyState === WebSocket.OPEN) {
            // Filter out the current player's data before sending
            const otherPlayersData = Array.from(players.entries())
                .filter(([id, _]) => id !== playerId)
                .map(([_, data]) => data.stateData);
            const serverMessage = {
                message_type: "player_states",
                player_id: 0,
                message: JSON.stringify(otherPlayersData),
            };
            playerNetworkData.ws.send(JSON.stringify(serverMessage));
        }
    });
}
// TODO
function networkTic() {
    broadcastPlayerStates();
}
function foodSpawn() {
    if (mapState.food_positions.length < 3) {
        handleAddFood();
        broadcastMapState();
    }
}
// Periodically broadcast player states every second (1000 ms)
setInterval(networkTic, 50);
setInterval(foodSpawn, 10000);
console.log("WebSocket server running on ws://localhost:8080");
import express from "express";
const app = express();
// Serve WASM files with correct MIME type
app.use((req, res, next) => {
    if (req.url.endsWith(".wasm")) {
        res.set("Content-Type", "application/wasm");
    }
    next();
});
// Serve all static files from public directory
app.use(express.static("public"));
app.listen(8000, () => {
    console.log("Server is running on http://localhost:8000");
});
//# sourceMappingURL=server.js.map