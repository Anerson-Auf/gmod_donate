use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub id: u64,
    pub client_uuid: String,
    pub message_type: String,
    pub message_data: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub status: String
}

/*
    ["account"]	=	STEAM_0:1:156722227
    ["date"]	=	1783587600
    ["faction"]	=	all
    ["time"]	=	Fri Nov 28 19:10:26 2025
    ["type"]	=	weapon
    ["value"]	=	yufu_spear
    ["who"]	=	Anerson Darling
*/

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Player {
    pub name: String,
    pub steam_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Donate {
    pub id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_uuid: Option<String>,
    pub account: Player,
    pub date: DateTime<Utc>,
    pub faction: String,
    pub time: DateTime<Utc>,
    pub donate_type: String,
    pub value: String,
    pub who: Player,
} 

pub struct DonateList {
    pub list: Vec<Donate>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClientRequest {
    pub action: String,
    pub uuid: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServerResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ClientConnection {
    pub uuid: String,
    pub server_name: String,
    pub registered_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateRequest {
    pub client_uuid: String,
    pub donate: Donate,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateResponse {
    pub status: String,
    pub message: String,
}