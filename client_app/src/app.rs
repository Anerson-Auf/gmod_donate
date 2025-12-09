use anyhow::Result;
use gmod_tcp_shared::types::{ClientConnection, Donate};
use tokio::runtime::Runtime as runtime;
use tokio::sync::broadcast;
use tracing::{info, error};
use reqwest::Client;
// use std::cell::RefCell;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct App {
    pub selected_tab: Tab,
    pub clients: Vec<ClientConnection>,
    pub donates: Vec<Donate>,
    pub form: DonateForm,
    pub api_url: String,
    #[serde(default)]
    pub api_password: String,
    #[serde(skip)]
    pub async_runtime: Option<runtime>,
    #[serde(skip)]
    pub clients_tx: crossbeam_channel::Sender<Vec<ClientConnection>>,
    #[serde(skip)]
    pub clients_rx: crossbeam_channel::Receiver<Vec<ClientConnection>>,
    #[serde(skip)]
    pub donates_tx: crossbeam_channel::Sender<Vec<Donate>>,
    #[serde(skip)]
    pub donates_rx: crossbeam_channel::Receiver<Vec<Donate>>,
    #[serde(skip)]
    pub shutdown_tx: Option<broadcast::Sender<()>>,
    #[serde(skip)]
    pub editing_donate: Option<Donate>,
    #[serde(skip)]
    pub history_filter_steam_id: String,
    #[serde(skip)]
    pub history_filter_name: String,
    #[serde(skip)]
    pub history_filter_type: String,
    #[serde(skip)]
    pub history_filter_id: String,
    #[serde(skip)]
    pub history_filter_value: String,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq)]
pub enum Tab {
    Create,
    Clients,
    History,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Default)]
pub struct DonateForm {
    pub client_uuid: String,
    pub account_name: String,
    pub account_steam_id: String,
    pub who_name: String,
    pub who_steam_id: String,
    pub donate_type: String,
    pub value: String,
    pub faction: String,
}

impl Default for App {
    fn default() -> Self {
        dotenvy::dotenv().ok();
        let api_url = std::env::var("API_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string());
        let api_password = std::env::var("API_PASSWORD")
            .unwrap_or_default();
        let (clients_tx, clients_rx) = crossbeam_channel::bounded(100);
        let (donates_tx, donates_rx) = crossbeam_channel::bounded(100);
        Self {
            selected_tab: Tab::Create,
            clients: Vec::new(),
            donates: Vec::new(),
            form: DonateForm::default(),
            api_url,
            api_password,
            async_runtime: None,
            clients_tx,
            clients_rx,
            donates_tx,
            donates_rx,
            shutdown_tx: None,
            editing_donate: None,
            history_filter_steam_id: String::new(),
            history_filter_name: String::new(),
            history_filter_type: String::new(),
            history_filter_id: String::new(),
            history_filter_value: String::new(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Ok(clients) = self.clients_rx.try_recv() {
            self.clients = clients;
        }
        if let Ok(donates) = self.donates_rx.try_recv() {
            self.donates = donates;
        }
        self.draw(ctx);
    }
    
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(shutdown_tx) = &self.shutdown_tx {
            let _ = shutdown_tx.send(());
        }
    }
}

impl App {
    pub fn new() -> Self {
        let app = Self::default();
        app
    }
    pub fn run_native(async_runtime: runtime) -> Result<()> {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 800.0]).with_resizable(false),
            ..Default::default()
        };
        let mut app = Self::new();
        app.async_runtime = Some(async_runtime);
        
        let (shutdown_tx, mut _shutdown_rx) = broadcast::channel(1);
        app.shutdown_tx = Some(shutdown_tx.clone());
        
        let api_url = app.api_url.clone();
        let api_password = app.api_password.clone();
        let tx = app.donates_tx.clone();
        let mut shutdown_rx_task = shutdown_tx.subscribe();
        app.async_runtime.as_ref().unwrap().spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_rx_task.recv() => {
                        info!("Shutdown signal received, stopping donates fetch loop");
                        break;
                    }
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(60)) => {
                        info!("Fetching donates from {}", api_url);
                        let mut headers = reqwest::header::HeaderMap::new();
                        if !api_password.is_empty() {
                            headers.insert("X-API-Key", api_password.parse().unwrap());
                        }
                        let client = Client::builder()
                            .default_headers(headers)
                            .build()
                            .unwrap_or_else(|_| Client::new());
                        let response = client
                            .get(format!("{}/api/donates", api_url))
                            .send().await;
                        match response {
                            Ok(resp) => {
                                if let Ok(donates) = resp.json::<Vec<Donate>>().await {
                                    info!("Loaded {} donates", donates.len());
                                    if let Err(e) = tx.send(donates) {
                                        error!("Error sending donates in crossbeam channel: {}", e);
                                        break;
                                    }
                                } else {
                                    error!("Failed to parse donates response");
                                }
                            },
                            Err(e) => {
                                error!("Failed to fetch donates: {}", e);
                            }
                        }
                    }
                }
            }
        });
        if let Err(e) = app.request_clients() {
            error!("Error requesting clients: {}", e);
        };
        
        let result = eframe::run_native(
            "GMod TCP App",
            options,
            Box::new(|_cc| Ok(Box::new(app) as Box<dyn eframe::App>)),
        );

        let _ = shutdown_tx.send(());

        std::thread::sleep(std::time::Duration::from_millis(100));
        
        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("eframe error: {}", e)),
        }
    }
}