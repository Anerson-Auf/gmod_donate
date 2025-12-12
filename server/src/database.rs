use crate::tcp::TcpServer;

use anyhow::Result;
use rusqlite::{Connection, params};
use chrono::{DateTime, Utc};
use gmod_tcp_shared::types::{Message, Donate, Player, ClientConnection};
use serde_json;
use tracing::info;

impl TcpServer {
    pub async fn init_database(&self) -> Result<()> {
        let db = Connection::open("server.db")?;
        db.execute("
            CREATE TABLE IF NOT EXISTS clients (
            uuid TEXT PRIMARY KEY,
            server_name TEXT NOT NULL,
            registered_at TEXT NOT NULL,
            last_seen TEXT NOT NULL
        );
        ", [])?;
        db.execute("
            CREATE TABLE IF NOT EXISTS messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            client_uuid TEXT NOT NULL,
            message_type TEXT NOT NULL,
            message_data TEXT NOT NULL,
            created_at TEXT NOT NULL,
            delivered_at TEXT,
            status TEXT NOT NULL DEFAULT 'pending',
            FOREIGN KEY (client_uuid) REFERENCES clients(uuid)
        );
        ", [])?;
        db.execute("
            CREATE TABLE IF NOT EXISTS donates (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            message_id INTEGER,
            client_uuid TEXT NOT NULL,
            account_name TEXT DEFAULT NULL,
            account_steam_id TEXT NOT NULL,
            who_name TEXT NOT NULL,
            who_steam_id TEXT DEFAULT NULL,
            donate_type TEXT NOT NULL,
            value TEXT NOT NULL,
            faction TEXT NOT NULL,
            date TEXT NOT NULL,
            time TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY (client_uuid) REFERENCES clients(uuid),
            FOREIGN KEY (message_id) REFERENCES messages(id)
        );
        ", [])?;
        Ok(())
    }
    pub async fn register_client(&self, client_uuid: String) -> Result<()> {
        let client_uuid_clone = client_uuid.clone();
        let result = tokio::task::spawn_blocking(move || -> Result<()> {
            let db = Connection::open("server.db")?;
            db.execute("
                INSERT INTO clients (uuid, server_name, registered_at, last_seen) VALUES (?, ?, ?, ?);
            ", params![&client_uuid_clone, &client_uuid_clone, Utc::now().to_rfc3339(), Utc::now().to_rfc3339()])?;
            Ok(())
        }).await?;
        if result.is_err() {
            info!("Failed to register client. Maybe he already registered?");
        }
        Ok(())
    }
    pub async fn proof_client(&self, client_uuid: String) -> Result<()> {
        let client_uuid_clone = client_uuid.clone();
        let result = tokio::task::spawn_blocking(move || -> Result<i32> {
            let db = Connection::open("server.db")?;
            let count: i32 = db.query_row("SELECT COUNT(*) FROM clients WHERE uuid = ?", [client_uuid_clone], |row| row.get(0))?;
            Ok(count)
        }).await??;
        if result == 0 {
            return Err(anyhow::anyhow!("Client not found"));
        }
        Ok(())
    }

    pub async fn update_last_seen(&self, client_uuid: String) -> Result<()> {
        let client_uuid_clone = client_uuid.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let db = Connection::open("server.db")?;
            db.execute("UPDATE clients SET last_seen = ? WHERE uuid = ?", params![Utc::now().to_rfc3339(), client_uuid_clone])?;
            Ok(())
        }).await??;
        Ok(())
    }

    pub async fn create_message(&self, message: Message) -> Result<u64> {
        let message_clone: Message = message.clone();
        let message_type_clone = message.message_type.clone();
        let client_uuid_clone = message.client_uuid.clone();
        let message_data_clone = message.message_data.clone();
        let message_id = tokio::task::spawn_blocking(move || -> Result<u64> {
            let db = Connection::open("server.db")?;
            db.execute("INSERT INTO messages (client_uuid, message_type, message_data, created_at, status) VALUES (?, ?, ?, ?, ?);", params![message_clone.client_uuid, message_clone.message_type, serde_json::to_string(&message_clone.message_data)?, message_clone.created_at.to_rfc3339(), message_clone.status])?;
            let id = db.last_insert_rowid() as u64;
            Ok(id)
        }).await??;
        if message_type_clone == "donate" {
            let donate: Donate = serde_json::from_value(message_data_clone.clone())?;
            let donate_id = self.create_donate(donate, message_id, client_uuid_clone.clone()).await?;
            
            let mut updated_message_data = message_data_clone;
            if let Some(obj) = updated_message_data.as_object_mut() {
                obj.insert("id".to_string(), serde_json::json!(donate_id));
            }
            
            let message_id_clone = message_id;
            tokio::task::spawn_blocking(move || -> Result<()> {
                let db = Connection::open("server.db")?;
                db.execute(
                    "UPDATE messages SET message_data = ? WHERE id = ?",
                    params![serde_json::to_string(&updated_message_data)?, message_id_clone]
                )?;
                Ok(())
            }).await??;
        }
        Ok(message_id)
    }
    pub async fn create_donate(&self, donate: Donate, message_id: u64, client_uuid: String) -> Result<u64> {
        let donate_clone = donate.clone();
        let client_uuid_clone = client_uuid.clone();
        let donate_id = tokio::task::spawn_blocking(move || -> Result<u64> {
            let db = Connection::open("server.db")?;
            db.execute("INSERT INTO donates (message_id, client_uuid, account_name, account_steam_id, who_name, who_steam_id, donate_type, value, faction, date, time, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);", params![message_id, client_uuid_clone, donate_clone.account.name, donate_clone.account.steam_id, donate_clone.who.name, donate_clone.who.steam_id, donate_clone.donate_type, donate_clone.value, donate_clone.faction, donate_clone.date.to_rfc3339(), donate_clone.time.to_rfc3339(), Utc::now().to_rfc3339()])?;
            let id = db.last_insert_rowid() as u64;
            Ok(id)
        }).await??;
        Ok(donate_id)
    }
    pub async fn get_pending_messages(&self, client_uuid: String) -> Result<Vec<Message>> {
        let client_uuid_clone = client_uuid.clone();
        let messages = tokio::task::spawn_blocking(move || -> Result<Vec<Message>> {
            let db = Connection::open("server.db")?;
            let mut stmt = db.prepare("SELECT id, client_uuid, message_type, message_data, created_at, delivered_at, status FROM messages WHERE client_uuid = ? AND status = 'pending'")?;
            let messages: Result<Vec<Message>, _> = stmt.query_map([client_uuid_clone], |row| {
                let created_at_str: String = row.get(4)?;
                let delivered_at_str: Option<String> = row.get(5)?;
                Ok(Message {
                    id: row.get(0)?,
                    client_uuid: row.get(1)?,
                    message_type: row.get(2)?,
                    message_data: serde_json::from_str(&row.get::<_, String>(3)?).map_err(|_| rusqlite::Error::InvalidColumnType(0, "message_data".to_string(), rusqlite::types::Type::Text))?,
                    created_at: DateTime::parse_from_rfc3339(&created_at_str).map_err(|_| rusqlite::Error::InvalidColumnType(4, "created_at".to_string(), rusqlite::types::Type::Text))?.with_timezone(&Utc),
                    delivered_at: delivered_at_str.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))),
                    status: row.get(6)?,
                })
            })?.collect();
            messages.map_err(|e| anyhow::anyhow!("Database error: {}", e))
        }).await??;
        Ok(messages)
    }
    pub async fn mark_messages_delivered(&self, ids: Vec<u64>) -> Result<()> {
        let ids_clone = ids.clone();
        let now = Utc::now().to_rfc3339();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let db = Connection::open("server.db")?;
            if ids_clone.is_empty() {
                return Ok(());
            }
            for id in ids_clone {
                db.execute("UPDATE messages SET status = 'delivered', delivered_at = ? WHERE id = ?", params![now, id])?;
            }
            Ok(())
        }).await??;
        Ok(())
    }
    pub async fn get_donates(&self) -> Result<Vec<Donate>> {
        let donates = tokio::task::spawn_blocking(move || -> Result<Vec<Donate>> {
            let db = Connection::open("server.db")?;
            let mut stmt = db.prepare("SELECT id, client_uuid, account_name, account_steam_id, date, faction, time, donate_type, value, who_name, who_steam_id FROM donates")?;
            let donates: Result<Vec<Donate>, _> = stmt.query_map([], |row| {
                let date_str: String = row.get(4)?;
                let time_str: String = row.get(6)?;
                Ok(Donate {
                    id: Some(row.get(0)?),
                    client_uuid: Some(row.get(1)?),
                    account: Player {
                        name: row.get(2)?,
                        steam_id: row.get(3)?,
                    },
                    date: DateTime::parse_from_rfc3339(&date_str).map_err(|_| rusqlite::Error::InvalidColumnType(4, "date".to_string(), rusqlite::types::Type::Text))?.with_timezone(&Utc),
                    faction: row.get(5)?,
                    time: DateTime::parse_from_rfc3339(&time_str).map_err(|_| rusqlite::Error::InvalidColumnType(6, "time".to_string(), rusqlite::types::Type::Text))?.with_timezone(&Utc),
                    donate_type: row.get(7)?,
                    value: row.get(8)?,
                    who: Player {
                        name: row.get(9)?,
                        steam_id: row.get(10)?,
                    },
                })
            })?.collect();
            donates.map_err(|e| anyhow::anyhow!("Database error: {}", e))
        }).await??;
        Ok(donates)
    }
    
    pub async fn get_donate_by_id(&self, donate_id: u64) -> Result<Option<(Donate, String)>> {
        let result = tokio::task::spawn_blocking(move || -> Result<Option<(Donate, String)>> {
            let db = Connection::open("server.db")?;
            let mut stmt = db.prepare("SELECT id, account_name, account_steam_id, date, faction, time, donate_type, value, who_name, who_steam_id, client_uuid FROM donates WHERE id = ?")?;
            match stmt.query_row(params![donate_id], |row| {
                let date_str: String = row.get(3)?;
                let time_str: String = row.get(5)?;
                let client_uuid: String = row.get(10)?;
                Ok((
                    Donate {
                        id: Some(row.get(0)?),
                        client_uuid: Some(client_uuid.clone()),
                        account: Player {
                            name: row.get(1)?,
                            steam_id: row.get(2)?,
                        },
                        date: DateTime::parse_from_rfc3339(&date_str).map_err(|_| rusqlite::Error::InvalidColumnType(3, "date".to_string(), rusqlite::types::Type::Text))?.with_timezone(&Utc),
                        faction: row.get(4)?,
                        time: DateTime::parse_from_rfc3339(&time_str).map_err(|_| rusqlite::Error::InvalidColumnType(5, "time".to_string(), rusqlite::types::Type::Text))?.with_timezone(&Utc),
                        donate_type: row.get(6)?,
                        value: row.get(7)?,
                        who: Player {
                            name: row.get(8)?,
                            steam_id: row.get(9)?,
                        },
                    },
                    client_uuid,
                ))
            }) {
                Ok(val) => Ok(Some(val)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(anyhow::anyhow!("Database error: {}", e)),
            }
        }).await??;
        Ok(result)
    }
    
    pub async fn delete_donate(&self, donate_id: u64) -> Result<Option<(Donate, String)>> {
        let donate_info = self.get_donate_by_id(donate_id).await?;
        
        if let Some((_, _)) = &donate_info {
            tokio::task::spawn_blocking(move || -> Result<()> {
                let db = Connection::open("server.db")?;
                db.execute("DELETE FROM donates WHERE id = ?", params![donate_id])?;
                Ok(())
            }).await??;
        }
        
        Ok(donate_info)
    }
    
    pub async fn update_donate(&self, donate_id: u64, donate: Donate) -> Result<Option<String>> {
        let old_donate_info = self.get_donate_by_id(donate_id).await?;
        let client_uuid = old_donate_info.as_ref().map(|(_, uuid)| uuid.clone());
        
        let donate_clone = donate.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let db = Connection::open("server.db")?;
            db.execute(
                "UPDATE donates SET account_name = ?, account_steam_id = ?, who_name = ?, who_steam_id = ?, donate_type = ?, value = ?, faction = ?, date = ?, time = ? WHERE id = ?",
                params![
                    donate_clone.account.name,
                    donate_clone.account.steam_id,
                    donate_clone.who.name,
                    donate_clone.who.steam_id,
                    donate_clone.donate_type,
                    donate_clone.value,
                    donate_clone.faction,
                    donate_clone.date.to_rfc3339(),
                    donate_clone.time.to_rfc3339(),
                    donate_id
                ]
            )?;
            Ok(())
        }).await??;
        Ok(client_uuid)
    }
    
    pub async fn get_clients(&self) -> Result<Vec<ClientConnection>> {
        let clients = tokio::task::spawn_blocking(move || -> Result<Vec<ClientConnection>> {
            let db = Connection::open("server.db")?;
            let mut stmt = db.prepare("SELECT uuid, server_name, registered_at, last_seen FROM clients")?;
            let clients: Result<Vec<ClientConnection>, _> = stmt.query_map([], |row| {
                let registered_at_str: String = row.get(2)?;
                let last_seen_str: String = row.get(3)?;
                Ok(ClientConnection {
                    uuid: row.get(0)?,
                    server_name: row.get(1)?,
                    registered_at: DateTime::parse_from_rfc3339(&registered_at_str).map_err(|_| rusqlite::Error::InvalidColumnType(2, "registered_at".to_string(), rusqlite::types::Type::Text))?.with_timezone(&Utc),
                    last_seen: DateTime::parse_from_rfc3339(&last_seen_str).map_err(|_| rusqlite::Error::InvalidColumnType(3, "last_seen".to_string(), rusqlite::types::Type::Text))?.with_timezone(&Utc),
                })
            })?.collect();
            clients.map_err(|e| anyhow::anyhow!("Database error: {}", e))
        }).await??;
        Ok(clients)
    }
    pub async fn clear_delivered_messages(&self) -> Result<()> {
        tokio::task::spawn_blocking(move || -> Result<()> {
            let db = Connection::open("server.db")?;
            let cutoff_time = Utc::now().to_rfc3339();

            db.execute(
                "DELETE FROM donates WHERE message_id IN (
                    SELECT id FROM messages WHERE status = 'delivered' AND delivered_at < ?
                )",
                params![cutoff_time]
            )?;

            db.execute("DELETE FROM messages WHERE status = 'delivered' AND delivered_at < ?", params![cutoff_time])?;
            Ok(())
        }).await??;
        Ok(())
    }
}