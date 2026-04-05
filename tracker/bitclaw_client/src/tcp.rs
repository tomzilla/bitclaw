//! TCP communication module for client-to-client connections
//!
//! Handles:
//! - TCP listener for incoming connections
//! - Outgoing connections to other clients
//! - Message framing and serialization
//! - Bidirectional message streaming

use bytes::{Buf, BufMut, BytesMut};
use futures_util::sink::SinkExt;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tokio::net::{TcpListener as TokioTcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::StreamExt;
use tokio_util::codec::{Decoder, Encoder};
use uuid::Uuid;

use crate::error::{ClientError, Result};
use crate::protocol::{
    ClientMessage, HandshakeRequest, HandshakeResponse, MessageFrame, MessageType, MAGIC, VERSION,
};

/// Message handler callback type - called when a message is received
pub type MessageHandler = Arc<dyn Fn(Uuid, ClientMessage) + Send + Sync + 'static>;

/// Maximum message size (10 MB)
const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024;

/// Codec for framing messages over TCP
pub struct MessageCodec;

impl Decoder for MessageCodec {
    type Item = MessageFrame;
    type Error = ClientError;

    fn decode(&mut self, src: &mut BytesMut) -> std::result::Result<Option<Self::Item>, Self::Error> {
        // Frame format: magic (4) + version (1) + type (1) + length (4) + payload
        // Need at least: magic (4) + version (1) + type (1) + length (4) = 10 bytes
        if src.len() < 10 {
            return Ok(None);
        }

        // Check magic bytes
        if src[0..4] != MAGIC {
            return Err(ClientError::ProtocolError("Invalid magic bytes".to_string()));
        }

        // Check version
        if src[4] != VERSION {
            return Err(ClientError::ProtocolError(format!(
                "Unsupported version: {}",
                src[4]
            )));
        }

        // Read message length (u32, little endian) - starts at byte 6
        let msg_len = u32::from_le_bytes([src[6], src[7], src[8], src[9]]) as usize;

        if msg_len > MAX_MESSAGE_SIZE {
            return Err(ClientError::MessageTooLarge);
        }

        // Check if we have the full message
        if src.len() < 10 + msg_len {
            return Ok(None);
        }

        // Extract the message type (byte 5)
        let message_type = MessageType::try_from(src[5])?;

        // Advance past the header (10 bytes)
        src.advance(10);

        // Extract payload
        let payload = src.split_to(msg_len).to_vec();

        Ok(Some(MessageFrame {
            magic: MAGIC,
            version: VERSION,
            message_type,
            payload,
        }))
    }
}

impl Encoder<MessageFrame> for MessageCodec {
    type Error = ClientError;

    fn encode(&mut self, item: MessageFrame, dst: &mut BytesMut) -> std::result::Result<(), Self::Error> {
        // Write header: magic (4) + version (1) + type (1) + length (4)
        dst.put_slice(&item.magic);
        dst.put_u8(item.version);
        dst.put_u8(item.message_type as u8);
        dst.put_u32_le(item.payload.len() as u32);

        // Write payload
        dst.put_slice(&item.payload);

        Ok(())
    }
}

impl TryFrom<u8> for MessageType {
    type Error = ClientError;

    fn try_from(value: u8) -> std::result::Result<Self, ClientError> {
        match value {
            0 => Ok(MessageType::HandshakeRequest),
            1 => Ok(MessageType::HandshakeResponse),
            2 => Ok(MessageType::AgentInfo),
            3 => Ok(MessageType::Discovery),
            4 => Ok(MessageType::Message),
            5 => Ok(MessageType::Error),
            6 => Ok(MessageType::KeepAlive),
            7 => Ok(MessageType::Close),
            _ => Err(ClientError::ProtocolError(format!(
                "Unknown message type: {}",
                value
            ))),
        }
    }
}

/// Connection state for a peer - holds both send and receive channels
#[derive(Debug, Clone)]
pub struct PeerConnection {
    pub peer_id: Uuid,
    pub address: SocketAddr,
    pub tx: mpsc::Sender<Vec<u8>>,
    pub features: Vec<String>,
}

/// TCP Listener for incoming client connections
#[derive(Clone)]
pub struct ClientTcpListener {
    local_addr: SocketAddr,
    connections: Arc<RwLock<HashMap<Uuid, PeerConnection>>>,
    shutdown_tx: mpsc::Sender<()>,
    #[allow(dead_code)]
    message_handler: Option<MessageHandler>,
}

impl ClientTcpListener {
    /// Create and start a new TCP listener
    pub async fn bind(
        _client_id: Uuid,
        addr: IpAddr,
        port: u16,
        message_handler: Option<MessageHandler>,
    ) -> Result<Self> {
        let local_addr = SocketAddr::new(addr, port);
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        let connections: Arc<RwLock<HashMap<Uuid, PeerConnection>>> = Arc::new(RwLock::new(HashMap::new()));

        let listener = TokioTcpListener::bind(local_addr).await?;
        let actual_addr = listener.local_addr()?;

        let conn_clone = Arc::clone(&connections);
        let handler_clone = message_handler.clone();

        // Spawn listener task
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    result = listener.accept() => {
                        match result {
                            Ok((stream, addr)) => {
                                log::info!("New connection from {}", addr);
                                let conn_clone = Arc::clone(&conn_clone);
                                let handler_clone = handler_clone.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = handle_incoming_connection(stream, conn_clone, handler_clone).await {
                                        // Log protocol errors as debug since they're expected from non-BitClaw connections
                                        if matches!(e, ClientError::ProtocolError(_)) {
                                            log::debug!("Rejected non-BitClaw connection from {}: {}", addr, e);
                                        } else {
                                            log::error!("Error handling connection from {}: {}", addr, e);
                                        }
                                    }
                                });
                            }
                            Err(e) => {
                                log::error!("Failed to accept connection: {}", e);
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        log::info!("Shutting down TCP listener");
                        break;
                    }
                }
            }
        });

        Ok(Self {
            local_addr: actual_addr,
            connections,
            shutdown_tx,
            message_handler,
        })
    }

    /// Get the local address this listener is bound to
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Get all connected peers
    pub async fn get_peers(&self) -> Vec<Uuid> {
        self.connections.read().await.keys().copied().collect()
    }

    /// Send a message to a specific peer
    pub async fn send_to_peer(&self, peer_id: &Uuid, data: &[u8]) -> Result<()> {
        let connections = self.connections.read().await;
        if let Some(conn) = connections.get(peer_id) {
            conn.tx.send(data.to_vec()).await
                .map_err(|_| ClientError::ConnectionClosed)?;
            Ok(())
        } else {
            Err(ClientError::ConnectionNotFound)
        }
    }

    /// Broadcast a message to all connected peers
    pub async fn broadcast(&self, data: &[u8]) -> Result<()> {
        let connections = self.connections.read().await;
        for conn in connections.values() {
            let _ = conn.tx.send(data.to_vec()).await;
        }
        Ok(())
    }

    /// Stop the listener
    pub async fn shutdown(self) -> Result<()> {
        self.shutdown_tx.send(()).await
            .map_err(|_| ClientError::ConnectionClosed)?;
        Ok(())
    }
}

/// Handle an incoming connection - performs handshake and manages message loop
async fn handle_incoming_connection(
    stream: TcpStream,
    connections: Arc<RwLock<HashMap<Uuid, PeerConnection>>>,
    message_handler: Option<MessageHandler>,
) -> Result<()> {
    let (msg_tx, mut msg_rx) = mpsc::channel::<Vec<u8>>(100);

    // Split stream into read and write halves
    let (read_half, write_half) = stream.into_split();

    // Do handshake
    let mut framed_write = tokio_util::codec::FramedWrite::new(write_half, MessageCodec);
    let mut framed_read = tokio_util::codec::FramedRead::new(read_half, MessageCodec);

    // Wait for handshake request
    let frame = match framed_read.next().await {
        Some(Ok(f)) => f,
        Some(Err(e)) => return Err(e),
        None => return Err(ClientError::ConnectionClosed),
    };

    if frame.message_type == MessageType::HandshakeRequest {
        let handshake: HandshakeRequest = bincode::deserialize(&frame.payload)?;

        // Send handshake response
        let response = HandshakeResponse {
            accepted: true,
            client_id: Uuid::new_v4(),
            client_version: "0.1.0".to_string(),
            supported_features: vec!["discovery".to_string(), "messaging".to_string()],
            error: None,
        };

        let response_payload = bincode::serialize(&response)?;
        framed_write.send(MessageFrame::new(MessageType::HandshakeResponse, response_payload)).await?;

        let peer_addr = framed_write.get_ref().peer_addr()?;

        // Store connection
        let peer_conn = PeerConnection {
            peer_id: handshake.client_id,
            address: peer_addr,
            tx: msg_tx,
            features: handshake.supported_features.clone(),
        };

        connections.write().await.insert(handshake.client_id, peer_conn);

        log::info!("Accepted connection from {} ({}), local addr: {}", handshake.client_id, peer_addr, peer_addr);

        let conn_clone = Arc::clone(&connections);
        let peer_id = handshake.client_id;
        let handler = message_handler.clone();

        // Spawn writer task - sends outgoing messages
        tokio::spawn(async move {
            let mut framed_write = framed_write;
            while let Some(data) = msg_rx.recv().await {
                // Log outgoing message
                match bincode::deserialize::<crate::protocol::ClientMessage>(&data) {
                    Ok(msg) => {
                        match msg.content {
                            crate::protocol::MessageContent::Text(ref text) => {
                                log::info!(">> SENDING to {}: Text: {}", peer_id, text);
                            }
                            crate::protocol::MessageContent::Json(ref json) => {
                                log::info!(">> SENDING to {}: Json: {}", peer_id, json);
                            }
                            crate::protocol::MessageContent::Binary(ref d) => {
                                log::info!(">> SENDING to {}: Binary ({} bytes)", peer_id, d.len());
                            }
                        }
                    }
                    Err(_) => {
                        log::info!(">> SENDING to {}: Raw ({} bytes)", peer_id, data.len());
                    }
                }

                let frame = MessageFrame::new(MessageType::Message, data);
                if framed_write.send(frame).await.is_err() {
                    break;
                }
            }
            // Connection closed, remove from map
            conn_clone.write().await.remove(&peer_id);
        });

        // Reader loop - receives incoming messages
        while let Some(result) = framed_read.next().await {
            match result {
                Ok(frame) => {
                    match frame.message_type {
                        MessageType::Close => {
                            log::info!("Received Close from {}", peer_id);
                            break;
                        }
                        MessageType::KeepAlive => continue,
                        MessageType::Message => {
                            // Try to decode as ClientMessage
                            match bincode::deserialize::<crate::protocol::ClientMessage>(&frame.payload) {
                                Ok(msg) => {
                                    match msg.content {
                                        crate::protocol::MessageContent::Text(ref text) => {
                                            log::info!("<< RECEIVED from {}: Text: {}", peer_id, text);
                                        }
                                        crate::protocol::MessageContent::Json(ref json) => {
                                            log::info!("<< RECEIVED from {}: Json: {}", peer_id, json);
                                        }
                                        crate::protocol::MessageContent::Binary(ref data) => {
                                            log::info!("<< RECEIVED from {}: Binary ({} bytes)", peer_id, data.len());
                                        }
                                    }

                                    // Call the message handler if provided
                                    if let Some(handler) = &handler {
                                        handler(peer_id, msg);
                                    }
                                }
                                Err(e) => {
                                    log::info!("<< RECEIVED from {}: Raw ({} bytes, decode error: {})", peer_id, frame.payload.len(), e);
                                }
                            }
                        }
                        _ => {
                            log::info!("Received message of type {:?} from {} ({} bytes)", frame.message_type, peer_id, frame.payload.len());
                        }
                    }
                }
                Err(e) => {
                    log::error!("Error receiving message: {}", e);
                    break;
                }
            }
        }
    }

    Ok(())
}

/// Active connection to another client
pub struct ClientConnection {
    peer_id: Uuid,
    address: SocketAddr,
    tx: mpsc::Sender<Vec<u8>>,
}

impl ClientConnection {
    pub fn peer_id(&self) -> Uuid {
        self.peer_id
    }

    pub fn peer_addr(&self) -> SocketAddr {
        self.address
    }

    /// Send a message to the connected peer
    pub async fn send(&self, data: &[u8]) -> Result<()> {
        self.tx.send(data.to_vec()).await
            .map_err(|_| ClientError::ConnectionClosed)
    }
}

/// Connect to another client and establish a bidirectional connection
pub async fn connect_to_client(
    client_id: Uuid,
    addr: IpAddr,
    port: u16,
) -> Result<ClientConnection> {
    let socket_addr = SocketAddr::new(addr, port);
    let stream = TcpStream::connect(socket_addr).await?;

    let peer_addr = stream.peer_addr()?;
    let (msg_tx, mut msg_rx) = mpsc::channel::<Vec<u8>>(100);

    // Split stream into read and write halves
    let (read_half, write_half) = stream.into_split();

    // Do handshake using framed write
    let mut framed_write = tokio_util::codec::FramedWrite::new(write_half, MessageCodec);
    let mut framed_read = tokio_util::codec::FramedRead::new(read_half, MessageCodec);

    // Send handshake
    let handshake = HandshakeRequest {
        client_id,
        client_version: "0.1.0".to_string(),
        supported_features: vec!["discovery".to_string(), "messaging".to_string()],
    };

    let payload = bincode::serialize(&handshake)?;
    framed_write.send(MessageFrame::new(MessageType::HandshakeRequest, payload)).await?;

    // Wait for response
    let response_frame = match framed_read.next().await {
        Some(Ok(f)) => f,
        Some(Err(e)) => return Err(e),
        None => return Err(ClientError::HandshakeFailed("No response".to_string())),
    };

    if response_frame.message_type == MessageType::HandshakeResponse {
        let response: HandshakeResponse = bincode::deserialize(&response_frame.payload)?;
        if !response.accepted {
            return Err(ClientError::HandshakeFailed(
                response.error.unwrap_or_else(|| "Unknown error".to_string())
            ));
        }
    } else {
        return Err(ClientError::HandshakeFailed("Unexpected response type".to_string()));
    }

    let peer_id = Uuid::new_v4();

    // Spawn writer task with the write half
    tokio::spawn(async move {
        let mut framed_write = framed_write;
        while let Some(data) = msg_rx.recv().await {
            let frame = MessageFrame::new(MessageType::Message, data);
            if framed_write.send(frame).await.is_err() {
                break;
            }
        }
    });

    Ok(ClientConnection {
        peer_id,
        address: peer_addr,
        tx: msg_tx,
    })
}
