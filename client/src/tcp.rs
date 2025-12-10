use anyhow::Result;
use tokio::net::TcpStream;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use tokio::time::{Duration, timeout};
use gmod_tcp_shared::types::{ClientRequest, Message, ServerResponse};
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;

use uuid::Uuid;
use std::fs;

const DEFAULT_SERVER_HOST: &str = "127.0.0.1";
const DEFAULT_SERVER_PORT: &str = "25565";

#[derive(Debug)]
pub struct TcpClient {
    pub client_uuid: String,
    server_host: String,
    server_port: String,
}

impl TcpClient {
    pub async fn new() -> Result<Self> {
        let client_uuid = TcpClient::get_or_create_uuid().await?;
        let (server_host, server_port) = TcpClient::get_host_and_port().await?;
        Ok(Self { 
            client_uuid,
            server_host,
            server_port,
        })
    }
    
    #[allow(unused)]
    pub async fn new_with_server(server_host: String, server_port: String) -> Result<Self> {
        let client_uuid = TcpClient::get_or_create_uuid().await?;
        Ok(Self {
            client_uuid,
            server_host,
            server_port,
        })
    }

    pub async fn get_host_and_port() -> Result<(String, String)> {
        let host_dir = Path::new("data/gmod_tcp");
        if let Err(e) = fs::create_dir_all(&host_dir) {
            eprintln!("Failed to create directory {:?}: {}", host_dir, e);
        } 
        let path = host_dir.join("host.txt");
        if path.exists() {
            let host_and_port = fs::read_to_string(&path)?.trim().to_string();
            let parts = host_and_port.split(':').collect::<Vec<&str>>();
            if parts.len() == 2 {
                let host = parts[0].to_string();
                let port = parts[1].to_string();
                Ok((host, port))
            } else {
                eprintln!("Invalid host and port format in {:?}: {}", path, host_and_port);
                Err(anyhow::anyhow!("Invalid host and port format in {:?}: {}", path, host_and_port))
            }
        } else {
            Ok((DEFAULT_SERVER_HOST.to_string(), DEFAULT_SERVER_PORT.to_string()))
        }
    }
    
    pub async fn get_or_create_uuid() -> Result<String> {
        let uuid_dir = Path::new("data/gmod_tcp");
        
        if let Err(e) = fs::create_dir_all(&uuid_dir) {
            eprintln!("Failed to create directory {:?}: {}", uuid_dir, e);
        }
        
        let path = uuid_dir.join("uuid.txt");
        if path.exists() {
            let uuid = fs::read_to_string(&path)?
                .trim()
                .to_string();
            if uuid.is_empty() {
                eprintln!("uuid.txt is empty, generating new UUID");
                let new_uuid = Uuid::new_v4().to_string();
                fs::write(&path, &new_uuid)?;
                println!("Generated new UUID: {}", new_uuid);
                Ok(new_uuid)
            } else {
                println!("Using existing client identifier: {} (from {:?})", uuid, path);
                Ok(uuid)
            }
        } else {
            let new_uuid = Uuid::new_v4().to_string();
            fs::write(&path, &new_uuid)?;
            println!("Generated new UUID: {} (saved to {:?})", new_uuid, path);
            Ok(new_uuid)
        }
    }
    pub async fn connect(&self) -> Result<TcpStream> {
        let addr = format!("{}:{}", self.server_host, self.server_port);
        println!("Connecting to server at {}", addr);
        let connect_future = TcpStream::connect(&addr);
        match tokio::time::timeout(Duration::from_secs(10), connect_future).await {
            Ok(Ok(stream)) => {
                println!("Successfully connected to {}", addr);
                Ok(stream)
            }
            Ok(Err(e)) => {
                let os_error = e.raw_os_error();
                Err(anyhow::anyhow!("Connection failed to {}: {} (os error: {:?})", addr, e, os_error))
            }
            Err(_) => {
                Err(anyhow::anyhow!("Connection timeout after 10 seconds to {}", addr))
            }
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
    pub async fn register(&self) -> Result<()> {
        println!("Connecting to server for registration");
        let mut stream = self.connect().await?;
        let req = ClientRequest {
            action: "register".to_string(),
            uuid: self.client_uuid.clone(),
        };
        let req_json = serde_json::to_vec(&req)?;
        Self::write_message(&mut stream, &req_json).await?;
        let response_data = Self::read_message(&mut stream).await?;
        let response: ServerResponse = serde_json::from_slice(&response_data)?;
        if response.status != "ok" {
            eprintln!("Registration failed: {}", response.message.as_ref().unwrap_or(&serde_json::Value::Null));
            return Err(anyhow::anyhow!("Failed to register: {}", response.message.unwrap()));
        }
        Ok(())
    }
    pub async fn listen(self: &Arc<Self>, message_queue: &'static Mutex<Vec<Message>>) -> Result<()> {
        let clone_self = Arc::clone(&self);
        tokio::spawn(async move {
            loop {
                match clone_self.find_messages().await {
                    Ok(messages) => {
                        if !messages.is_empty() {
                            let mut queue = message_queue.lock().unwrap();
                            queue.extend(messages);
                            println!("Added {} message(s) to queue", queue.len());
                        }
                    }
                    Err(e) => {
                        eprintln!("Error finding messages: {}", e);
                    }
                }
                println!("Waiting 10 minutes until next poll");
                tokio::time::sleep(Duration::from_secs(600)).await;
            }
        });
        Ok(())
    }
    
    pub async fn find_messages(&self) -> Result<Vec<Message>> {
        println!("Polling server for new messages");
        let mut stream = self.connect().await?;
        let req = ClientRequest {
            action: "pool".to_string(),
            uuid: self.client_uuid.clone(),
        };
        let req_json = serde_json::to_vec(&req)?;
        Self::write_message(&mut stream, &req_json).await?;
        let response_data = Self::read_message(&mut stream).await?;
        let response: ServerResponse = serde_json::from_slice(&response_data)?;
        if response.status != "ok" {
            eprintln!("Polling failed: {}", response.message.as_ref().unwrap_or(&serde_json::Value::Null));
            return Err(anyhow::anyhow!("Failed to find messages: {}", response.message.unwrap()));
        }
        let messages: Vec<Message> = serde_json::from_value(response.message.unwrap())?;
        if messages.is_empty() {
            println!("No new messages");
        } else {
            println!("Received {} new message(s)", messages.len());
            for message in &messages {
                println!("Message: {:?}", message);
            }
        }
        Ok(messages)
    }
}