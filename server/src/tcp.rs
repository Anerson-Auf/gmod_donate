use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use anyhow::Result;
use chrono::Utc;
use tokio::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, error};

use gmod_tcp_shared::types::{Message, Donate, ClientRequest, ServerResponse};

pub struct TcpServer {
    listener: Arc<TcpListener>,
}

impl TcpServer {
    pub async fn new() -> Result<Self> {
        dotenvy::dotenv().ok();
        let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = std::env::var("PORT").unwrap_or_else(|_| "25565".to_string());
        let addr = format!("{}:{}", host, port);
        let listener = TcpListener::bind(&addr).await.unwrap();
        info!("TCP server bound to {}", addr);
        Ok(Self { 
            listener: Arc::new(listener), 
        })
    }
    pub async fn listen(self: Arc<Self>) -> Result<()> {
        let clone_self = Arc::clone(&self);
        tokio::spawn(async move {
            loop {
                if let Err(e) = clone_self.clear_delivered_messages().await {
                    error!("Error clearing delivered messages: {}", e);
                };
                info!("Cleared delivered messages");
                tokio::time::sleep(Duration::from_secs(60 * 60)).await;
            }
        });
        let another_one_clone = Arc::clone(&self);
        tokio::spawn(async move {
            loop {
                match another_one_clone.listener.accept().await {
                    Ok((socket, addr)) => {
                        info!("New TCP connection from {}", addr);
                        let server_clone = Arc::clone(&another_one_clone);
                        tokio::spawn(async move {
                            if let Err(e) = server_clone.handle_socket_messsages(socket).await {
                                error!("Error handling socket messages from {}: {}", addr, e);
                            };
                        });
                    }
                    Err(e) => {
                        error!("Failed to accept connection: {}", e);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        });
        
        loop {
            tokio::time::sleep(Duration::from_secs(3600)).await;
        }
    }

    async fn read_message(socket: &mut TcpStream) -> Result<Vec<u8>> {
        let mut length_bytes = [0u8; 4];
        socket.read_exact(&mut length_bytes).await?;
        let length = u32::from_le_bytes(length_bytes) as usize;
        let mut buffer = vec![0u8; length];
        socket.read_exact(&mut buffer).await?;
        Ok(buffer)
    }

    async fn write_message(socket: &mut TcpStream, data: &[u8]) -> Result<()> {
        let length = data.len() as u32;
        socket.write_all(&length.to_le_bytes()).await?;
        socket.write_all(data).await?;
        socket.flush().await?;
        Ok(())
    }

    pub async fn handle_socket_messsages(&self, mut socket: TcpStream) -> Result<()> {
        let message_data = Self::read_message(&mut socket).await?;
        let request: ClientRequest = serde_json::from_slice(&message_data)?;
        let client_uuid = request.uuid.clone();
        
        info!("Received request: action={}, uuid={}", request.action, client_uuid);
        
        if request.action == "pool" {
            self.proof_client(client_uuid.clone()).await?;
            let mut messages = self.get_pending_messages(client_uuid.clone()).await?;
            info!("Polling request from client {}: {} messages found", client_uuid, messages.len());
            let response = ServerResponse {
                status: "ok".to_string(),
                message: Some(serde_json::to_value(messages.clone())?),
            };
            let response_data = serde_json::to_vec(&response)?;
            Self::write_message(&mut socket, &response_data).await?;
            messages.iter_mut().for_each(|message| {
                if message.client_uuid == client_uuid {
                    message.status = "delivered".to_string();
                    message.delivered_at = Some(Utc::now());
                }
            });
            self.update_last_seen(client_uuid.clone()).await?;
            self.mark_messages_delivered(messages.iter().map(|message| message.id).collect()).await?;
        } else if request.action == "register" {
            info!("Registering new client: {}", client_uuid);
            self.register_client(client_uuid.clone()).await?;
            let response = ServerResponse {
                status: "ok".to_string(),
                message: Some(serde_json::to_value(format!("Registered successfully: {}", client_uuid))?),
            };
            let response_data = serde_json::to_vec(&response)?;
            Self::write_message(&mut socket, &response_data).await?;
            info!("Client {} registered successfully", client_uuid);
        } else {
            error!("Unknown action received: {}", request.action);
            let response = ServerResponse {
                status: "error".to_string(),
                message: Some(serde_json::to_value(format!("Unknown action: {}", request.action))?),
            };
            let response_data = serde_json::to_vec(&response)?;
            Self::write_message(&mut socket, &response_data).await?;
        }
        Ok(())
    }

    #[allow(unused)]
    async fn save_message(&mut self, message: Message) -> Result<u64> {
        self.create_message(message).await
    }
    #[allow(unused)]
    async fn create_donate_message(&mut self, donate: Donate, client_uuid: String) -> Result<()> {
        let message = Message {
            id: 0,
            client_uuid: client_uuid,
            message_type: "donate".to_string(),
            message_data: serde_json::to_value(donate)?,
            created_at: Utc::now(),
            delivered_at: None,
            status: "pending".to_string(),
        };
        self.save_message(message).await?;
        Ok(())
    }

}