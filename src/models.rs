use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

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
pub struct IdentityProviderRepresentation {
    #[serde(rename = "internalId")]
    pub internal_id: Option<String>,
    pub alias: Option<String>,
    #[serde(rename = "providerId")]
    pub provider_id: Option<String>,
    pub enabled: Option<bool>,
    #[serde(rename = "updateProfileFirstLoginMode")]
    pub update_profile_first_login_mode: Option<String>,
    #[serde(rename = "trustEmail")]
    pub trust_email: Option<bool>,
    #[serde(rename = "storeToken")]
    pub store_token: Option<bool>,
    #[serde(rename = "addReadTokenRoleOnCreate")]
    pub add_read_token_role_on_create: Option<bool>,
    #[serde(rename = "authenticateByDefault")]
    pub authenticate_by_default: Option<bool>,
    #[serde(rename = "linkOnly")]
    pub link_only: Option<bool>,
    #[serde(rename = "firstBrokerLoginFlowAlias")]
    pub first_broker_login_flow_alias: Option<String>,
    #[serde(rename = "postBrokerLoginFlowAlias")]
    pub post_broker_login_flow_alias: Option<String>,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    pub config: Option<HashMap<String, String>>,
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
