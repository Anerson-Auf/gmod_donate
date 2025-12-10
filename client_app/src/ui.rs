use gmod_tcp_shared::types::{CreateRequest, Donate, Player, ClientConnection};
use egui::{Color32, CornerRadius, Stroke, Vec2};
use egui::RichText as rich;
use reqwest::{Client, Response};
use anyhow::Result;
use tracing::{info, error};
use chrono::{FixedOffset, Utc};
use serde::{Serialize, Deserialize};

use crate::app::{App, Tab};

fn moscow_timezone() -> FixedOffset {
    FixedOffset::east_opt(3 * 3600).unwrap()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub status: u32,
}

impl App {
    fn create_client_with_password(password: &str) -> Client {
        let mut headers = reqwest::header::HeaderMap::new();
        if !password.is_empty() {
            headers.insert("X-API-Key", password.parse().unwrap());
        }
        Client::builder()
            .default_headers(headers)
            .build()
            .unwrap_or_else(|_| Client::new())
    }

    pub fn draw(&mut self, ctx: &egui::Context) {
        self.setup_style(ctx);
        
        egui::TopBottomPanel::top("top_panel")
            .exact_height(60.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(10.0);
                        ui.heading(rich::new("Yufu Donate Manager").size(24.0).color(Color32::from_rgb(255, 0, 255)));
                        ui.add_space(5.0);
                    });
                });
            });

        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(250.0)
            .show(ctx, |ui| {
                if !self.logged {
                    return;
                }
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    ui.heading(rich::new("Navigation").size(18.0).color(Color32::from_rgb(255, 0, 255)));
                });
                ui.separator();
                ui.add_space(10.0);
                
                if ui.button(rich::new("📝 Create Donate").size(16.0)).clicked() {
                    self.selected_tab = Tab::Create;
                }
                ui.add_space(5.0);
                if ui.button(rich::new("👥 Clients").size(16.0)).clicked() {
                    self.selected_tab = Tab::Clients;
                    if let Err(e) = self.request_clients() {
                        error!("Failed to request clients: {}", e);
                    };
                }
                ui.add_space(5.0);
                if ui.button(rich::new("📋 History").size(16.0)).clicked() {
                    self.selected_tab = Tab::History;
                    if let Err(e) = self.request_donates() {
                        error!("Failed to request donates: {}", e);
                    };
                }
            });

        egui::CentralPanel::default()
            .show(ctx, |ui| {
                ui.add_space(20.0);
                if !self.logged {
                    self.draw_login(ui);
                    return;
                }
                match self.selected_tab {
                    Tab::Create => self.draw_create_donate(ui),
                    Tab::Clients => self.draw_clients(ui),
                    Tab::History => self.draw_history(ui),
                }
            });

        if let Some(mut editing_donate) = self.editing_donate.take() {
            let mut should_save = false;
            let mut should_cancel = false;
            
            egui::Window::new("Edit Donate")
                .collapsible(false)
                .resizable(true)
                .default_size([500.0, 600.0])
                .show(ctx, |ui| {
                    Self::draw_edit_donate_modal_ui(ui, &mut editing_donate, &mut should_save, &mut should_cancel);
                });
            
            if should_save {
                if let Err(e) = self.update_donate(editing_donate.clone()) {
                    error!("Failed to update donate: {}", e);
                }
                self.editing_donate = None;
            } else if should_cancel {
                self.editing_donate = None;
            } else {
                self.editing_donate = Some(editing_donate);
            }
        }
    }

    fn setup_style(&self, ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();
        
        style.visuals.dark_mode = true;
        style.visuals.panel_fill = Color32::from_rgb(20, 20, 25);
        style.visuals.window_fill = Color32::from_rgb(18, 18, 23);
        style.visuals.window_stroke = Stroke::new(1.0, Color32::from_rgb(180, 0, 180));
        style.visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(25, 25, 30);
        style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(30, 30, 35);
        style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(200, 0, 200);
        style.visuals.widgets.active.bg_fill = Color32::from_rgb(255, 0, 255);
        style.visuals.selection.bg_fill = Color32::from_rgb(180, 0, 180);
        style.visuals.weak_text_color = Some(Color32::from_rgb(150, 150, 160));
        style.visuals.extreme_bg_color = Color32::from_rgb(15, 15, 20);
        style.visuals.faint_bg_color = Color32::from_rgb(22, 22, 28);
        style.visuals.override_text_color = Some(Color32::from_rgb(220, 220, 230));
        
        style.spacing.button_padding = Vec2::new(15.0, 8.0);
        style.spacing.item_spacing = Vec2::new(10.0, 8.0);
        ctx.set_style(style);
    }

    fn draw_create_donate(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.heading(rich::new("Create New Donate").size(22.0).color(Color32::from_rgb(255, 0, 255)));
        });
        ui.add_space(30.0);

        ui.vertical_centered(|ui| {
            egui::Frame::group(ui.style())
                .fill(Color32::from_rgb(25, 25, 30))
                .stroke(Stroke::new(1.0, Color32::from_rgb(180, 0, 180)))
                .inner_margin(20.0)
                .show(ui, |ui| {
                    ui.set_max_width(600.0);
                    ui.vertical_centered(|ui| {
                        ui.add_space(10.0);
                        
                        ui.label(rich::new("Client UUID").size(14.0).color(Color32::from_rgb(200, 200, 210)));
                        egui::ComboBox::from_id_salt("client_uuid")
                            .width(ui.available_width())
                            .selected_text(if self.form.client_uuid.is_empty() {
                                "Select client..."
                            } else {
                                &self.form.client_uuid
                            })
                            .show_ui(ui, |ui| {
                                for client in &self.clients {
                                    if ui.selectable_label(
                                        self.form.client_uuid == client.uuid,
                                        format!("{} ({})", client.server_name, client.uuid)
                                    ).clicked() {
                                        self.form.client_uuid = client.uuid.clone();
                                    }
                                }
                            });
                        ui.add_space(15.0);

                        ui.label(rich::new("Account Name").size(14.0).color(Color32::from_rgb(200, 200, 210)));
                        ui.text_edit_singleline(&mut self.form.account_name);
                        ui.add_space(10.0);

                        ui.label(rich::new("Account Steam ID").size(14.0).color(Color32::from_rgb(200, 200, 210)));
                        ui.text_edit_singleline(&mut self.form.account_steam_id);
                        ui.add_space(15.0);

                        ui.label(rich::new("Who Name").size(14.0).color(Color32::from_rgb(200, 200, 210)));
                        ui.text_edit_singleline(&mut self.form.who_name);
                        ui.add_space(10.0);

                        ui.label(rich::new("Who Steam ID").size(14.0).color(Color32::from_rgb(200, 200, 210)));
                        ui.text_edit_singleline(&mut self.form.who_steam_id);
                        ui.add_space(15.0);

                        ui.label(rich::new("Donate Type").size(14.0).color(Color32::from_rgb(200, 200, 210)));
                        egui::ComboBox::from_id_salt("donate_type")
                            .width(ui.available_width())
                            .selected_text(if self.form.donate_type.is_empty() {
                                "Select type..."
                            } else {
                                &self.form.donate_type
                            })
                            .show_ui(ui, |ui| {
                                for donate_type in ["weapon", "money", "item"] {
                                    if ui.selectable_label(
                                        self.form.donate_type == donate_type,
                                        donate_type
                                    ).clicked() {
                                        self.form.donate_type = donate_type.to_string();
                                    }
                                }
                            });
                        ui.add_space(10.0);

                        ui.label(rich::new("Value").size(14.0).color(Color32::from_rgb(200, 200, 210)));
                        ui.text_edit_singleline(&mut self.form.value);
                        ui.add_space(10.0);

                        ui.label(rich::new("Faction").size(14.0).color(Color32::from_rgb(200, 200, 210)));
                        egui::ComboBox::from_id_salt("faction")
                            .width(ui.available_width())
                            .selected_text(if self.form.faction.is_empty() {
                                "Select faction..."
                            } else {
                                &self.form.faction
                            })
                            .show_ui(ui, |ui| {
                                for faction in ["all", "police", "mafia"] {
                                    if ui.selectable_label(
                                        self.form.faction == faction,
                                        faction
                                    ).clicked() {
                                        self.form.faction = faction.to_string();
                                    }
                                }
                            });
                        ui.add_space(20.0);

                        if ui.button(rich::new("✨ Create Donate").size(16.0).color(Color32::WHITE)).clicked() {
                            if let Err(e) = self.create_donate() {
                                error!("Failed to create donate: {}", e);
                            }
                        }
                    });
                });
        });
    }

    fn draw_clients(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.heading(rich::new("Registered Clients").size(22.0).color(Color32::from_rgb(255, 0, 255)));
        });
        ui.add_space(20.0);

        egui::ScrollArea::vertical()
            .max_height(500.0)
            .show(ui, |ui| {
                for client in &self.clients {
                    egui::Frame::group(ui.style())
                        .fill(Color32::from_rgb(25, 25, 30))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(180, 0, 180)))
                        .corner_radius(CornerRadius::same(10))
                        .inner_margin(15.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(rich::new(&client.server_name).size(16.0).color(Color32::from_rgb(255, 0, 255)));
                                    ui.label(rich::new(format!("UUID: {}", client.uuid)).size(12.0).color(Color32::from_rgb(180, 180, 190)));
                                });
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let moscow_time = client.last_seen.with_timezone(&moscow_timezone());
                                    ui.label(rich::new(format!("Last seen: {}", moscow_time.format("%Y-%m-%d %H:%M"))).size(11.0).color(Color32::from_rgb(150, 150, 160)));
                                });
                            });
                        });
                    ui.add_space(10.0);
                }
            });
    }

    fn draw_history(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.heading(rich::new("Donate History").size(22.0).color(Color32::from_rgb(255, 0, 255)));
        });
        ui.add_space(20.0);

        egui::Frame::group(ui.style())
            .fill(Color32::from_rgb(25, 25, 30))
            .stroke(Stroke::new(1.0, Color32::from_rgb(180, 0, 180)))
            .inner_margin(15.0)
            .show(ui, |ui| {
                ui.heading(rich::new("🔍 Filters").size(16.0).color(Color32::from_rgb(255, 0, 255)));
                ui.add_space(10.0);
                
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(rich::new("Steam ID").size(12.0).color(Color32::from_rgb(200, 200, 210)));
                        ui.text_edit_singleline(&mut self.history_filter_steam_id);
                    });
                    ui.add_space(10.0);
                    ui.vertical(|ui| {
                        ui.label(rich::new("Name").size(12.0).color(Color32::from_rgb(200, 200, 210)));
                        ui.text_edit_singleline(&mut self.history_filter_name);
                    });
                    ui.add_space(10.0);
                    ui.vertical(|ui| {
                        ui.label(rich::new("Type").size(12.0).color(Color32::from_rgb(200, 200, 210)));
                        ui.text_edit_singleline(&mut self.history_filter_type);
                    });
                });
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(rich::new("ID").size(12.0).color(Color32::from_rgb(200, 200, 210)));
                        ui.text_edit_singleline(&mut self.history_filter_id);
                    });
                    ui.add_space(10.0);
                    ui.vertical(|ui| {
                        ui.label(rich::new("Value").size(12.0).color(Color32::from_rgb(200, 200, 210)));
                        ui.text_edit_singleline(&mut self.history_filter_value);
                    });
                    ui.add_space(10.0);
                    ui.vertical(|ui| {
                        ui.add_space(20.0);
                        if ui.button(rich::new("Clear Filters").size(14.0)).clicked() {
                            self.history_filter_steam_id.clear();
                            self.history_filter_name.clear();
                            self.history_filter_type.clear();
                            self.history_filter_id.clear();
                            self.history_filter_value.clear();
                        }
                    });
                });
            });
        ui.add_space(15.0);

        let filtered_donates: Vec<&Donate> = self.donates.iter()
            .filter(|donate| {
                let steam_id_match = self.history_filter_steam_id.is_empty() ||
                    donate.account.steam_id.to_lowercase().contains(&self.history_filter_steam_id.to_lowercase()) ||
                    donate.who.steam_id.to_lowercase().contains(&self.history_filter_steam_id.to_lowercase());
                
                let name_match = self.history_filter_name.is_empty() ||
                    donate.account.name.to_lowercase().contains(&self.history_filter_name.to_lowercase()) ||
                    donate.who.name.to_lowercase().contains(&self.history_filter_name.to_lowercase());
                
                let type_match = self.history_filter_type.is_empty() ||
                    donate.donate_type.to_lowercase().contains(&self.history_filter_type.to_lowercase());
                
                let id_match = self.history_filter_id.is_empty() ||
                    donate.id.map(|id| id.to_string()).unwrap_or_default().contains(&self.history_filter_id);
                
                let value_match = self.history_filter_value.is_empty() ||
                    donate.value.to_lowercase().contains(&self.history_filter_value.to_lowercase());
                
                steam_id_match && name_match && type_match && id_match && value_match
            })
            .collect();

        let mut delete_id = None;
        let mut edit_donate = None;
        
        ui.label(rich::new(format!("Showing {} of {} donates", filtered_donates.len(), self.donates.len())).size(12.0).color(Color32::from_rgb(150, 150, 160)));
        ui.add_space(10.0);
        
        egui::ScrollArea::vertical()
            .max_height(500.0)
            .show(ui, |ui| {
                let num_columns = 4;
                let spacing = 1.0;
                let column_width = (ui.available_width() - (num_columns - 1) as f32 * spacing) / num_columns as f32;
                
                ui.horizontal(|ui| {
                    for col in 0..num_columns {
                        ui.vertical(|ui| {
                            ui.set_width(column_width);
                            for (idx, donate) in filtered_donates.iter().enumerate() {
                                if idx % num_columns == col {
                                    let donate_clone = (*donate).clone();
                                    let donate_id = donate.id;
                                    ui.set_max_width(column_width);
                                    egui::Frame::group(ui.style())
                                        .fill(Color32::from_rgb(25, 25, 30))
                                        .stroke(Stroke::new(1.0, Color32::from_rgb(180, 0, 180)))
                                        .corner_radius(CornerRadius::same(10))
                                        .inner_margin(15.0)
                                        .show(ui, |ui| {
                                            ui.vertical(|ui| {
                                                ui.set_max_width(150.0);
                                                ui.add(
                                                    egui::Label::new(
                                                        rich::new(&donate_clone.who.name).size(16.0).color(Color32::from_rgb(255, 0, 255))
                                                    ).wrap()
                                                );
                                                ui.add(
                                                    egui::Label::new(
                                                        rich::new(format!("{} → {}", donate_clone.account.name, donate_clone.value)).size(13.0).color(Color32::from_rgb(200, 200, 210))
                                                    ).wrap()
                                                );
                                                ui.add(
                                                    egui::Label::new(
                                                        rich::new(format!("Type: {} | Faction: {}", donate_clone.donate_type, donate_clone.faction)).size(11.0).color(Color32::from_rgb(150, 150, 160))
                                                    ).wrap()
                                                );
                                                if let Some(ref client_uuid) = donate_clone.client_uuid {
                                                    ui.add(
                                                        egui::Label::new(
                                                            rich::new(format!("Client: {}", client_uuid)).size(10.0).color(Color32::from_rgb(120, 120, 130))
                                                        ).wrap()
                                                    );
                                                }
                                                ui.add_space(5.0);
                                                ui.horizontal(|ui| {
                                                    if ui.button(rich::new("✏️").size(12.0)).clicked() {
                                                        edit_donate = Some(donate_clone.clone());
                                                    }
                                                    if let Some(id) = donate_id {
                                                        if ui.button(rich::new("🗑️").size(12.0)).clicked() {
                                                            delete_id = Some(id);
                                                        }
                                                    }
                                                });
                                            });
                                        });
                                    ui.add_space(1.0);
                                }
                            }
                        });
                        if col < num_columns - 1 {
                            ui.add_space(spacing);
                        }
                    }
                });
            });
        
        if let Some(id) = delete_id {
            if let Err(e) = self.delete_donate(id) {
                error!("Failed to delete donate: {}", e);
            }
        }
        
        if let Some(donate) = edit_donate {
            self.editing_donate = Some(donate);
        }
    }

    fn draw_login(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.heading(rich::new("Login").size(22.0).color(Color32::from_rgb(255, 0, 255)));
        });
        ui.add_space(20.0);
        ui.horizontal_centered(|ui| {
            egui::Frame::group(ui.style())
                .fill(Color32::from_rgb(25, 25, 30))
                .stroke(Stroke::new(1.0, Color32::from_rgb(180, 0, 180)))
                .inner_margin(20.0)
                .show(ui, |ui| {
                    ui.set_max_width(600.0);
                    ui.vertical_centered(|ui| {
                        ui.add_space(10.0);
                        ui.label(rich::new("API URL").size(14.0).color(Color32::from_rgb(200, 200, 210)));
                        ui.text_edit_singleline(&mut self.api_url);
                        ui.label(rich::new("API Password").size(14.0).color(Color32::from_rgb(200, 200, 210)));
                        ui.text_edit_singleline(&mut self.api_password);
                        ui.add_space(20.0);
                        if ui.button(rich::new("Login").size(16.0).color(Color32::WHITE)).clicked() {
                            let _ = self.request_clients();
                        }
                    });
                });
        });
    }

    fn create_donate(&mut self) -> Result<()> {
        
        let form = self.form.clone();
        let api_url = self.api_url.clone();
        info!("Creating donate for client {}", form.client_uuid);

        let donate = Donate {
            id: None,
            client_uuid: Some(form.client_uuid.clone()),
            account: Player {
                name: form.account_name.clone(),
                steam_id: form.account_steam_id.clone(),
            },
            date: Utc::now(),
            faction: form.faction.clone(),
            time: Utc::now(),
            donate_type: form.donate_type.clone(),
            value: form.value.clone(),
            who: Player {
                name: form.who_name.clone(),
                steam_id: form.who_steam_id.clone(),
            },
        };
        
        let request = CreateRequest {
            client_uuid: form.client_uuid.clone(),
            donate,
        };
        
        let api_password = self.api_password.clone();
        self.async_runtime.as_ref().unwrap().spawn(async move {
            let client = Self::create_client_with_password(&api_password);
            match client
                .post(format!("{}/api/donates", api_url))
                .json(&request)
                .send().await
            {
                Ok(resp) => {
                    if resp.status().is_success() {
                        info!("Donate created successfully");
                    } else {
                        error!("Failed to create donate: HTTP {}", resp.status());
                    }
                },
                Err(e) => error!("Failed to create donate: {}", e),
            }
        });
        Ok(())
    }
    
    fn delete_donate(&mut self, donate_id: u64) -> Result<()> {
        let api_url = self.api_url.clone();
        let donates_tx = self.donates_tx.clone();
        let api_password = self.api_password.clone();
        self.async_runtime.as_ref().unwrap().spawn(async move {
            let client = Self::create_client_with_password(&api_password);
            match client
                .delete(format!("{}/api/donates/{}", api_url, donate_id))
                .send().await
            {
                Ok(resp) => {
                    if resp.status().is_success() {
                        info!("Donate {} deleted successfully", donate_id);

                        if let Ok(donates_resp) = client
                            .get(format!("{}/api/donates", api_url))
                            .send().await
                        {
                            if let Ok(donates) = donates_resp.json::<Vec<Donate>>().await {
                                let _ = donates_tx.send(donates);
                            }
                        }
                    } else {
                        error!("Failed to delete donate: HTTP {}", resp.status());
                    }
                },
                Err(e) => error!("Failed to delete donate: {}", e),
            }
        });
        Ok(())
    }
    
    fn update_donate(&mut self, donate: Donate) -> Result<()> {
        let donate_id = donate.id.ok_or_else(|| anyhow::anyhow!("Donate ID is missing"))?;
        let api_url = self.api_url.clone();
        let donates_tx = self.donates_tx.clone();
        let donate_clone = donate.clone();
        let api_password = self.api_password.clone();
        self.async_runtime.as_ref().unwrap().spawn(async move {
            let client = Self::create_client_with_password(&api_password);
            match client
                .put(format!("{}/api/donates/{}", api_url, donate_id))
                .json(&donate_clone)
                .send().await
            {
                Ok(resp) => {
                    if resp.status().is_success() {
                        info!("Donate {} updated successfully", donate_id);

                        if let Ok(donates_resp) = client
                            .get(format!("{}/api/donates", api_url))
                            .send().await
                        {
                            if let Ok(donates) = donates_resp.json::<Vec<Donate>>().await {
                                let _ = donates_tx.send(donates);
                            }
                        }
                    } else {
                        error!("Failed to update donate: HTTP {}", resp.status());
                    }
                },
                Err(e) => error!("Failed to update donate: {}", e),
            }
        });
        Ok(())
    }
    
    fn draw_edit_donate_modal_ui(ui: &mut egui::Ui, donate: &mut Donate, should_save: &mut bool, should_cancel: &mut bool) {
        ui.vertical(|ui| {
            ui.heading(rich::new("Edit Donate").size(20.0).color(Color32::from_rgb(255, 0, 255)));
            ui.add_space(15.0);
            
            ui.label(rich::new("Account Name").size(14.0).color(Color32::from_rgb(200, 200, 210)));
            ui.text_edit_singleline(&mut donate.account.name);
            ui.add_space(10.0);
            
            ui.label(rich::new("Account Steam ID").size(14.0).color(Color32::from_rgb(200, 200, 210)));
            ui.text_edit_singleline(&mut donate.account.steam_id);
            ui.add_space(10.0);
            
            ui.label(rich::new("Who Name").size(14.0).color(Color32::from_rgb(200, 200, 210)));
            ui.text_edit_singleline(&mut donate.who.name);
            ui.add_space(10.0);
            
            ui.label(rich::new("Who Steam ID").size(14.0).color(Color32::from_rgb(200, 200, 210)));
            ui.text_edit_singleline(&mut donate.who.steam_id);
            ui.add_space(10.0);
            
            ui.label(rich::new("Donate Type").size(14.0).color(Color32::from_rgb(200, 200, 210)));
            ui.text_edit_singleline(&mut donate.donate_type);
            ui.add_space(10.0);
            
            ui.label(rich::new("Value").size(14.0).color(Color32::from_rgb(200, 200, 210)));
            ui.text_edit_singleline(&mut donate.value);
            ui.add_space(10.0);
            
            ui.label(rich::new("Faction").size(14.0).color(Color32::from_rgb(200, 200, 210)));
            egui::ComboBox::from_id_salt("edit_faction")
                .width(ui.available_width())
                .selected_text(&donate.faction)
                .show_ui(ui, |ui| {
                    for faction in ["all", "police", "mafia"] {
                        if ui.selectable_label(
                            donate.faction == faction,
                            faction
                        ).clicked() {
                            donate.faction = faction.to_string();
                        }
                    }
                });
            ui.add_space(20.0);
            
            ui.horizontal(|ui| {
                if ui.button(rich::new("Save").size(16.0).color(Color32::WHITE)).clicked() {
                    *should_save = true;
                }
                if ui.button(rich::new("Cancel").size(16.0)).clicked() {
                    *should_cancel = true;
                }
            });
        });
    }
    
    
   pub fn request_clients(&self) -> Result<()> {
        let api_url = self.api_url.clone();
        let clients_tx = self.clients_tx.clone();
        let login_status_tx = self.login_status_tx.clone();
        let api_password = self.api_password.clone();
        info!("Fetching clients from {}", api_url);
        self.async_runtime.as_ref().unwrap().spawn(async move {
            let client = Self::create_client_with_password(&api_password);
            match client
                .get(format!("{}/api/clients", api_url))
                .send().await
            {
                Ok(response) => {
                    let status_code = response.headers().clone().get("content-length").unwrap().to_str().unwrap().parse::<u32>().unwrap();
                    if let Ok(clients) = response.json::<Vec<ClientConnection>>().await {
                        info!("Loaded {} clients", clients.len());
                        if let Err(e) = clients_tx.send(clients) {
                            error!("Error sending clients in crossbeam channel: {}", e);
                        }
                    } else {
                        error!("Failed to parse clients response");
                    }
                    println!("status_code: {:?}", status_code);
                    if let Err(e) = login_status_tx.send(status_code == 150) {
                        error!("Error sending login status: {}", e);
                    }
                },
                Err(e) => {
                    error!("Failed to fetch clients: {}", e);
                    if let Err(e) = login_status_tx.send(false) {
                        error!("Error sending login status: {}", e);
                    }
                }
            }
        });
        Ok(())
    }
    #[allow(dead_code)]
    pub fn request_donates(&self) -> Result<()> {
        let api_url = self.api_url.clone();
        let tx = self.donates_tx.clone();
        let api_password = self.api_password.clone();
        self.async_runtime.as_ref().unwrap().spawn(async move {
            let client = Self::create_client_with_password(&api_password);
            let response = client
                .get(format!("{}/api/donates", api_url))
                .send().await;
            if let Ok(response) = response {
                let donates: Vec<Donate> = response.json().await.unwrap();
                if let Err(e) = tx.send(donates) {
                    error!("Error sending donates in crossbeam channel: {}", e);
                };
            }
        });
        Ok(())
    }
}
