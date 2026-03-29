//! UPnP port forwarding support
//!
//! Provides automatic port forwarding through UPnP-enabled routers
//! Can be disabled for LAN-only operation

use std::net::{IpAddr, SocketAddr, SocketAddrV4, Ipv4Addr};
use tokio::task::spawn_blocking;

use crate::error::{ClientError, Result};

/// UPnP port forwarding configuration
#[derive(Debug, Clone)]
pub struct UpnpConfig {
    /// Enable UPnP port forwarding (default: true)
    pub enabled: bool,
    /// External port to forward (default: same as local port)
    pub external_port: Option<u16>,
    /// Lease duration in seconds (0 = permanent)
    pub lease_duration: u32,
    /// Friendly name for the port mapping
    pub description: String,
}

impl Default for UpnpConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            external_port: None,
            lease_duration: 0,
            description: "Arcadia Client".to_string(),
        }
    }
}

/// UPnP port mapping result
#[derive(Debug, Clone)]
pub struct PortMapping {
    pub internal_addr: SocketAddr,
    pub external_addr: SocketAddr,
    pub public_ip: IpAddr,
}

/// Attempt to set up UPnP port forwarding
pub async fn setup_port_forwarding(
    local_addr: SocketAddr,
    config: &UpnpConfig,
) -> Result<PortMapping> {
    if !config.enabled {
        // LAN mode - just return local address
        return Ok(PortMapping {
            internal_addr: local_addr,
            external_addr: local_addr,
            public_ip: local_addr.ip(),
        });
    }

    let external_port = config.external_port.unwrap_or(local_addr.port());
    let description = config.description.clone();
    let lease_duration = config.lease_duration;

    // UPnP operations are blocking, so run in a blocking task
    let result = spawn_blocking(move || {
        match igd::search_gateway(Default::default()) {
            Ok(gateway) => {
                // Convert local_addr to SocketAddrV4 for igd
                let local_addr_v4 = match local_addr {
                    SocketAddr::V4(v4) => v4,
                    SocketAddr::V6(v6) => {
                        // Try to map IPv6 to IPv4 if possible
                        if let Some(v4) = v6.ip().to_ipv4() {
                            SocketAddrV4::new(v4, local_addr.port())
                        } else {
                            log::warn!("IPv6 address cannot be mapped to IPv4 for UPnP");
                            return Ok(PortMapping {
                                internal_addr: local_addr,
                                external_addr: local_addr,
                                public_ip: local_addr.ip(),
                            });
                        }
                    }
                };

                // Add port mapping
                match gateway.add_port(
                    igd::PortMappingProtocol::TCP,
                    external_port,
                    local_addr_v4,
                    lease_duration,
                    &description,
                ) {
                    Ok(()) => {
                        log::info!(
                            "UPnP port forwarding established: {} -> {}",
                            SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), external_port),
                            local_addr
                        );

                        // Get external IP
                        let public_ip = gateway.get_external_ip().ok()
                            .map(|ip| IpAddr::V4(ip));

                        Ok::<PortMapping, ClientError>(PortMapping {
                            internal_addr: local_addr,
                            external_addr: SocketAddr::new(
                                public_ip.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
                                external_port,
                            ),
                            public_ip: public_ip.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
                        })
                    }
                    Err(e) => {
                        log::warn!("UPnP port forwarding failed: {}. Operating in LAN mode.", e);
                        Ok(PortMapping {
                            internal_addr: local_addr,
                            external_addr: local_addr,
                            public_ip: local_addr.ip(),
                        })
                    }
                }
            }
            Err(e) => {
                log::info!("No UPnP gateway found: {}. Operating in LAN mode.", e);
                Ok(PortMapping {
                    internal_addr: local_addr,
                    external_addr: local_addr,
                    public_ip: local_addr.ip(),
                })
            }
        }
    })
    .await
    .map_err(|e| ClientError::Connection(format!("UPnP task failed: {}", e)))??;

    Ok(result)
}

/// Remove a UPnP port forwarding
pub async fn remove_port_forwarding(external_port: u16) -> Result<()> {
    spawn_blocking(move || {
        if let Ok(gateway) = igd::search_gateway(Default::default()) {
            match gateway.remove_port(igd::PortMappingProtocol::TCP, external_port) {
                Ok(()) => {
                    log::info!("UPnP port forwarding removed for port {}", external_port);
                }
                Err(e) => {
                    log::warn!("Failed to remove UPnP port forwarding: {}", e);
                }
            }
        }
        Ok::<(), ClientError>(())
    })
    .await
    .map_err(|e| ClientError::Connection(format!("UPnP task failed: {}", e)))??;

    Ok(())
}

/// Check if UPnP is available on the network
pub async fn is_upnp_available() -> bool {
    spawn_blocking(|| {
        igd::search_gateway(Default::default()).is_ok()
    })
    .await
    .unwrap_or(false)
}
