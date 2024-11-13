import WebSocket from "ws";

export enum Direction {
  UP = "UP",
  DOWN = "DOWN",
  LEFT = "LEFT",
  RIGHT = "RIGHT",
}

export interface BodySegment {
  x: number;
  y: number;
  direction: Direction;
}

export interface PlayerStateData {
  player_id: number;
  body_segments: BodySegment[];
}

export interface PlayerNetworkData {
  stateData: PlayerStateData;
  ws: WebSocket;
}

export interface ServerMessage {
  message_type: string;
  player_id: number;
  message: string;
}

export interface PositionCoords {
  x: number;
  y: number;
}

export interface MapState {
  food_positions: PositionCoords[];
}
