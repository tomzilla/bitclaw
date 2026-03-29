use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Bincode error: {0}")]
    Bincode(#[from] bincode::Error),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Connection not found")]
    ConnectionNotFound,

    #[error("Not connected to hub")]
    NotConnected,

    #[error("Hub not found: {0}")]
    HubNotFound(String),

    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    #[error("Handshake failed: {0}")]
    HandshakeFailed(String),

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Message too large")]
    MessageTooLarge,

    #[error("Protocol error: {0}")]
    ProtocolError(String),
}

pub type Result<T> = std::result::Result<T, ClientError>;
