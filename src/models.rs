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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClientScopeRepresentation {
    pub id: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub protocol: Option<String>,
    pub attributes: Option<HashMap<String, String>>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupRepresentation {
    pub id: Option<String>,
    pub name: Option<String>,
    pub path: Option<String>,
    #[serde(rename = "subGroups")]
    pub sub_groups: Option<Vec<GroupRepresentation>>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CredentialRepresentation {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub value: Option<String>,
    pub temporary: Option<bool>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserRepresentation {
    pub id: Option<String>,
    pub username: Option<String>,
    pub enabled: Option<bool>,
    #[serde(rename = "firstName")]
    pub first_name: Option<String>,
    #[serde(rename = "lastName")]
    pub last_name: Option<String>,
    pub email: Option<String>,
    #[serde(rename = "emailVerified")]
    pub email_verified: Option<bool>,
    pub credentials: Option<Vec<CredentialRepresentation>>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthenticationExecutionExportRepresentation {
    pub authenticator: Option<String>,
    #[serde(rename = "authenticatorConfig")]
    pub authenticator_config: Option<String>,
    pub requirement: Option<String>,
    pub priority: Option<i32>,
    #[serde(rename = "authenticatorFlow")]
    pub authenticator_flow: Option<bool>,
    #[serde(rename = "flowAlias")]
    pub flow_alias: Option<String>,
    #[serde(rename = "userSetupAllowed")]
    pub user_setup_allowed: Option<bool>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthenticationFlowRepresentation {
    pub id: Option<String>,
    pub alias: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "providerId")]
    pub provider_id: Option<String>,
    #[serde(rename = "topLevel")]
    pub top_level: Option<bool>,
    #[serde(rename = "builtIn")]
    pub built_in: Option<bool>,
    #[serde(rename = "authenticationExecutions")]
    pub authentication_executions: Option<Vec<AuthenticationExecutionExportRepresentation>>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RequiredActionProviderRepresentation {
    pub alias: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "providerId")]
    pub provider_id: Option<String>,
    pub enabled: Option<bool>,
    #[serde(rename = "defaultAction")]
    pub default_action: Option<bool>,
    pub priority: Option<i32>,
    pub config: Option<HashMap<String, String>>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComponentRepresentation {
    pub id: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "providerId")]
    pub provider_id: Option<String>,
    #[serde(rename = "providerType")]
    pub provider_type: Option<String>,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    #[serde(rename = "subType")]
    pub sub_type: Option<String>,
    pub config: Option<HashMap<String, Value>>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}
