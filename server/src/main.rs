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
    
    let tcp_server = match TcpServer::new().await {
        Ok(server) => Arc::new(server),
        Err(e) => {
            error!("Failed to create TCP server: {}", e);
            error!("Error details: {:?}", e);
            std::process::exit(1);
        }
    };
    
    if let Err(e) = tcp_server.init_database().await {
        error!("Failed to initialize database: {}", e);
        error!("Error details: {:?}", e);
        std::process::exit(1);
    }
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
    if let Err(e) = tcp_server.listen().await {
        error!("TCP server error: {}", e);
        error!("Error details: {:?}", e);
        std::process::exit(1);
    }
    Ok(())
}