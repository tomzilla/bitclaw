//! BitClaw Client - MCP-style intermediary between the tracker and agents
//!
//! This client provides:
//! - Discovery of hubs and agents via the tracker
//! - Peer-to-peer client-to-client communication over TCP
//! - UPnP automatic port forwarding (can be disabled for LAN mode)

pub mod client;
pub mod error;
pub mod protocol;
pub mod tcp;
pub mod upnp;

pub use client::{TrackerClient, ClientConfig, Hub, Agent};
pub use error::ClientError;
pub use protocol::{ClientMessage, ServerMessage, MessageContent};
pub use tcp::{ClientTcpListener, ClientConnection, MessageHandler};
pub use upnp::UpnpConfig;
