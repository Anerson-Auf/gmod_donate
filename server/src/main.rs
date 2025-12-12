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
    eprintln!("Starting GMod TCP Server...");
    info!("Starting GMod TCP Server");
    
    eprintln!("Creating TCP server...");
    let tcp_server = match TcpServer::new().await {
        Ok(server) => {
            eprintln!("TCP server created successfully");
            Arc::new(server)
        },
        Err(e) => {
            eprintln!("Failed to create TCP server: {}", e);
            error!("Failed to create TCP server: {}", e);
            error!("Error details: {:?}", e);
            std::process::exit(1);
        }
    };
    
    eprintln!("Initializing database...");
    if let Err(e) = tcp_server.init_database().await {
        eprintln!("Failed to initialize database: {}", e);
        error!("Failed to initialize database: {}", e);
        error!("Error details: {:?}", e);
        std::process::exit(1);
    }
    eprintln!("Database initialized");
    info!("Database initialized");
    
    eprintln!("Spawning REST server task...");
    let tcp_server_clone = tcp_server.clone();
    tokio::spawn(async move {
        info!("Spawning REST server task...");
        match RestServer::new(tcp_server_clone).await {
            Ok(_) => {
                eprintln!("REST server exited normally");
                info!("REST server exited normally");
            },
            Err(e) => {
                eprintln!("Failed to start HTTP API server: {}", e);
                error!("Failed to start HTTP API server: {}", e);
                error!("Error details: {:?}", e);
            }
        }
    });
    
    eprintln!("TCP server listening for connections");
    info!("TCP server listening for connections");
    if let Err(e) = tcp_server.listen().await {
        eprintln!("TCP server error: {}", e);
        error!("TCP server error: {}", e);
        error!("Error details: {:?}", e);
        std::process::exit(1);
    }
    eprintln!("TCP server exited");
    Ok(())
}