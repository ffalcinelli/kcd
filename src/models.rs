use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RealmRepresentation {
    pub realm: String,
    pub enabled: Option<bool>,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClientRepresentation {
    pub id: Option<String>,
    #[serde(rename = "clientId")]
    pub client_id: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub protocol: Option<String>,
    #[serde(rename = "redirectUris")]
    pub redirect_uris: Option<Vec<String>>,
    #[serde(rename = "webOrigins")]
    pub web_origins: Option<Vec<String>>,
    #[serde(rename = "publicClient")]
    pub public_client: Option<bool>,
    #[serde(rename = "bearerOnly")]
    pub bearer_only: Option<bool>,
    #[serde(rename = "serviceAccountsEnabled")]
    pub service_accounts_enabled: Option<bool>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RoleRepresentation {
    pub id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "containerId")]
    pub container_id: Option<String>,
    #[serde(default)]
    pub composite: bool,
    #[serde(rename = "clientRole", default)]
    pub client_role: bool,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}
