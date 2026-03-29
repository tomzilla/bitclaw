//! Protocol definitions for client-to-client communication
//!
//! Uses a simple binary protocol with bincode serialization

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Magic bytes for protocol identification
pub const MAGIC: [u8; 4] = [0x41, 0x52, 0x43, 0x41]; // "ARCA"
pub const VERSION: u8 = 1;

/// Message frame for TCP communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageFrame {
    pub magic: [u8; 4],
    pub version: u8,
    pub message_type: MessageType,
    pub payload: Vec<u8>,
}

impl MessageFrame {
    pub fn new(message_type: MessageType, payload: Vec<u8>) -> Self {
        Self {
            magic: MAGIC,
            version: VERSION,
            message_type,
            payload,
        }
    }
}

/// Type of message being sent
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageType {
    HandshakeRequest = 0,
    HandshakeResponse = 1,
    AgentInfo = 2,
    Discovery = 3,
    Message = 4,
    Error = 5,
    KeepAlive = 6,
    Close = 7,
}

/// Handshake request sent when connecting to another client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeRequest {
    pub client_id: Uuid,
    pub client_version: String,
    pub supported_features: Vec<String>,
}

/// Handshake response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeResponse {
    pub accepted: bool,
    pub client_id: Uuid,
    pub client_version: String,
    pub supported_features: Vec<String>,
    pub error: Option<String>,
}

/// Agent information for discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub agent_id: String,
    pub name: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub endpoint: Option<String>,
    pub status: String,
}

/// Discovery request/response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryMessage {
    pub query: String,
    pub hub_id: Option<String>,
    pub limit: Option<usize>,
}

/// Generic message for client-to-client communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientMessage {
    pub from: Uuid,
    pub to: Option<Uuid>,
    pub content: MessageContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContent {
    Text(String),
    Binary(Vec<u8>),
    Json(String), // JSON as string for bincode compatibility
}

/// Server/broadcast message to clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMessage {
    pub message_type: ServerMessageType,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessageType {
    HubUpdate,
    AgentJoined,
    AgentLeft,
    Broadcast,
}

/// Error message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMessage {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
}

impl ErrorMessage {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
            details: None,
        }
    }
}
