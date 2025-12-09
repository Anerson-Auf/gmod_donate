mod tcp;
mod database;
mod rest;
mod rest_handlers;

use anyhow::Result;
use std::sync::Arc;
use crate::tcp::TcpServer;
use crate::rest::RestServer;

use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    info!("Starting GMod TCP Server");
    
    let tcp_server = Arc::new(TcpServer::new().await?);
    tcp_server.init_database().await?;
    info!("Database initialized");
    
    let tcp_server_clone = tcp_server.clone();
    tokio::spawn(async move {
        info!("Spawning REST server task...");
        match RestServer::new(tcp_server_clone).await {
            Ok(_) => {
                info!("REST server exited normally");
            },
            Err(e) => {
                error!("Failed to start HTTP API server: {}", e);
                error!("Error details: {:?}", e);
            }
        }
    });
    
    info!("TCP server listening for connections");
    tcp_server.listen().await?;
    Ok(())
}