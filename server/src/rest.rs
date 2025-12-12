use axum:: {
    Router, http::{Method, HeaderValue, StatusCode, Request}, routing::{get, post, delete, put}, middleware::Next, response::Response
};
use anyhow::{Result, Context};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use std::collections::HashSet;
use std::sync::OnceLock;
use tokio::net::TcpListener;

use crate::rest_handlers;
use crate::tcp::TcpServer;
use tracing::{info, warn};

static PASSWORDS_CACHE: OnceLock<HashSet<String>> = OnceLock::new();

pub struct RestServer {
}

impl RestServer {
    
    async fn auth_middleware(
        req: Request<axum::body::Body>,
        next: Next,
    ) -> Result<Response, StatusCode> {
        let method = req.method().clone();
        let uri = req.uri().clone();
        info!("Incoming request: {} {}", method, uri);
        
        let passwords = Self::get_passwords();
        if passwords.is_empty() {
            info!("Request {} {} processed (no password protection)", method, uri);
            return Ok(next.run(req).await);
        }

        let api_key = req.headers()
            .get("X-API-Key")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| {
                warn!("Request {} {} rejected: missing X-API-Key header", method, uri);
                StatusCode::UNAUTHORIZED
            })?;

        if passwords.contains(api_key) {
            info!("Request {} {} authenticated successfully", method, uri);
            Ok(next.run(req).await)
        } else {
            warn!("Request {} {} rejected: invalid API key", method, uri);
            Err(StatusCode::UNAUTHORIZED)
        }
    }

    fn get_passwords() -> &'static HashSet<String> {
        PASSWORDS_CACHE.get_or_init(|| {
            dotenvy::dotenv().ok();
            std::env::var("API_PASSWORDS")
                .unwrap_or_default()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
    }

    fn load_passwords() -> HashSet<String> {
        Self::get_passwords().clone()
    }

    pub async fn new(tcp_server: Arc<TcpServer>) -> Result<Self> {
        dotenvy::dotenv().ok();
        let host = std::env::var("API_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = std::env::var("API_PORT").unwrap_or_else(|_| "9060".to_string());
        let allowed_origins = std::env::var("ALLOWED_ORIGINS").unwrap_or_else(|_| "*".to_string());
        let cors = CorsLayer::new()
            .allow_origin(allowed_origins.parse::<HeaderValue>().unwrap())
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
            .allow_headers([axum::http::header::HeaderName::from_static("x-api-key")])
            .allow_credentials(false);

        let passwords = Self::load_passwords();
        if !passwords.is_empty() {
            info!("API password protection enabled with {} password(s)", passwords.len());
        } else {
            warn!("API_PASSWORDS not set - API is unprotected!");
        }

        let router = Router::new()
            .route("/", get(|| async { (StatusCode::NOT_FOUND, "Not Found") }))
            .route("/ping", get(|| async { "pong" }))
            .route("/api/clients", get(rest_handlers::get_clients))
            .route("/api/messages/{client_uuid}", get(rest_handlers::get_messages))
            .route("/api/donates", get(rest_handlers::get_donates))
            .route("/api/donates", post(rest_handlers::create_donate))
            .route("/api/donates/{donate_id}", delete(rest_handlers::delete_donate))
            .route("/api/donates/{donate_id}", put(rest_handlers::update_donate))
            .fallback(|| async { (StatusCode::NOT_FOUND, "Not Found") })
            .layer( 
                TraceLayer::new_for_http()
                    .on_request(|request: &axum::http::Request<_>, _span: &tracing::Span| {
                        info!("TraceLayer: received request {} {}", request.method(), request.uri());
                    })
                    .on_response(|response: &axum::http::Response<_>, latency: std::time::Duration, _span: &tracing::Span| {
                        info!("TraceLayer: sending response {} latency: {:?}", response.status(), latency);
                    })
            )
            .layer(cors)
            .layer(axum::middleware::from_fn(Self::auth_middleware))
            .with_state(tcp_server);

        let addr = format!("{}:{}", host, port).parse::<SocketAddr>()?;
        info!("Starting HTTP API server on {}", addr);
        let listener = TcpListener::bind(&addr).await
            .with_context(|| format!("Failed to bind HTTP server to {}", addr))?;
        let _server = axum::serve(listener, router.into_make_service())
            .await?;
        info!("HTTP server bound to {}, starting to serve...", addr);
        Ok(Self {  })
    }
}