use gmod_tcp_shared::types::{ClientConnection, Donate};
use axum::{Json, extract::{Path, State}};
use crate::tcp::TcpServer;
use gmod_tcp_shared::types::{Message, CreateRequest, CreateResponse};
use tracing::{info, error};
use std::sync::Arc;
use chrono::Utc;
use axum::http::StatusCode;

pub async fn get_clients(State(server): State<Arc<TcpServer>>) -> Json<Vec<ClientConnection>> {
    info!("get_clients handler called");
    let clients = match server.get_clients().await {
        Ok(clients) => {
            info!("GET /api/clients: {} clients found", clients.len());
            clients
        },
        Err(e) => {
            error!("Error getting clients: {}", e);
            return Json(vec![]);
        }
    };
    info!("get_clients returning response");
    Json(clients)
}

pub async fn get_messages(Path(client_uuid): Path<String>, State(server): State<Arc<TcpServer>>) -> Json<Vec<Message>> {
    let messages = match server.get_pending_messages(client_uuid).await {
        Ok(messages) => messages,
        Err(e) => {
            error!("Error getting messages: {}", e);
            return Json(vec![]);
        }
    };
    Json(messages)
}

pub async fn get_donates(State(server): State<Arc<TcpServer>>) -> Json<Vec<Donate>> {
    let donates = match server.get_donates().await {
        Ok(donates) => {
            info!("GET /api/donates: {} donates found", donates.len());
            donates
        },
        Err(e) => {
            error!("Error getting donates: {}", e);
            return Json(vec![]);
        }
    };
    Json(donates)
}

pub async fn create_donate(State(server): State<Arc<TcpServer>>, Json(request): Json<CreateRequest>) -> Json<CreateResponse> {
    info!("POST /api/donates: Received request for client {}", request.client_uuid);
    match server.create_message(Message{
        id: 0,
        client_uuid: request.client_uuid.clone(),
        message_type: "donate".to_string(),
        message_data: serde_json::to_value(request.donate.clone()).unwrap_or_default(),
        created_at: Utc::now(),
        delivered_at: None,
        status: "pending".to_string(),
    }).await {
        Ok(message_id) => {
            info!("POST /api/donates: Created donate for client {} (message_id: {})", request.client_uuid, message_id);
            Json(CreateResponse{
                status: "ok".to_string(),
                message: format!("Donate created successfully with message_id: {}", message_id),
            })
        },
        Err(e) => {
            error!("Error creating donate for client {}: {}", request.client_uuid, e);
            Json(CreateResponse{
                status: "error".to_string(),
                message: format!("Error creating donate: {}", e),
            })
        }
    }
}

pub async fn delete_donate(Path(donate_id): Path<u64>, State(server): State<Arc<TcpServer>>) -> Result<Json<CreateResponse>, StatusCode> {
    match server.delete_donate(donate_id).await {
        Ok(Some((donate, client_uuid))) => {
            let message_data = serde_json::json!({
                "donate_id": donate_id,
                "donate": donate
            });
            if let Err(e) = server.create_message(Message {
                id: 0,
                client_uuid: client_uuid.clone(),
                message_type: "donate_deleted".to_string(),
                message_data,
                created_at: Utc::now(),
                delivered_at: None,
                status: "pending".to_string(),
            }).await {
                error!("Error creating delete message for client {}: {}", client_uuid, e);
            }
            info!("DELETE /api/donates/{}: Donate deleted successfully, message sent to client {}", donate_id, client_uuid);
            Ok(Json(CreateResponse {
                status: "ok".to_string(),
                message: format!("Donate {} deleted successfully", donate_id),
            }))
        },
        Ok(None) => {
            error!("Donate {} not found", donate_id);
            Err(StatusCode::NOT_FOUND)
        },
        Err(e) => {
            error!("Error deleting donate {}: {}", donate_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn update_donate(Path(donate_id): Path<u64>, State(server): State<Arc<TcpServer>>, Json(donate): Json<Donate>) -> Result<Json<CreateResponse>, StatusCode> {
    match server.update_donate(donate_id, donate.clone()).await {
        Ok(Some(client_uuid)) => {
            let mut updated_donate = donate.clone();
            updated_donate.id = Some(donate_id);
            let message_data = serde_json::json!({
                "donate_id": donate_id,
                "donate": updated_donate
            });
            if let Err(e) = server.create_message(Message {
                id: 0,
                client_uuid: client_uuid.clone(),
                message_type: "donate_updated".to_string(),
                message_data,
                created_at: Utc::now(),
                delivered_at: None,
                status: "pending".to_string(),
            }).await {
                error!("Error creating update message for client {}: {}", client_uuid, e);
            }
            info!("PUT /api/donates/{}: Donate updated successfully, message sent to client {}", donate_id, client_uuid);
            Ok(Json(CreateResponse {
                status: "ok".to_string(),
                message: format!("Donate {} updated successfully", donate_id),
            }))
        },
        Ok(None) => {
            error!("Donate {} not found or has no client_uuid", donate_id);
            Err(StatusCode::NOT_FOUND)
        },
        Err(e) => {
            error!("Error updating donate {}: {}", donate_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}