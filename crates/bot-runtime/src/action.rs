use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BotAction {
    Move { direction: String },
    LookAt { yaw: f32, pitch: f32 },
    LookAtPos { x: f64, y: f64, z: f64 },
    Jump,
    Attack { continuous: bool },
    Use { continuous: bool },
    Stop,
    Chat { message: String },
    Command { command: String },
    Plugin {
        plugin_id: String,
        action: String,
        args: String,
    },
}
