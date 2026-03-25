use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

pub trait KeycloakResource {
    fn get_identity(&self) -> Option<String>;
    fn get_name(&self) -> String;
    fn api_path() -> &'static str;
    fn dir_name() -> &'static str;
    fn object_path(id: &str) -> String {
        format!("{}/{}", Self::api_path(), id)
    }
    fn get_filename(&self) -> String {
        self.get_name()
    }
}

pub trait ResourceMeta {
    fn label() -> &'static str;
    fn secret_prefix() -> &'static str;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RealmRepresentation {
    pub realm: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl KeycloakResource for RealmRepresentation {
    fn get_identity(&self) -> Option<String> {
        Some(self.realm.clone())
    }
    fn get_name(&self) -> String {
        self.realm.clone()
    }
    fn api_path() -> &'static str {
        "realms"
    }
    fn dir_name() -> &'static str {
        "realms"
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IdentityProviderRepresentation {
    #[serde(rename = "internalId", skip_serializing_if = "Option::is_none")]
    pub internal_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(rename = "providerId", skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(
        rename = "updateProfileFirstLoginMode",
        skip_serializing_if = "Option::is_none"
    )]
    pub update_profile_first_login_mode: Option<String>,
    #[serde(rename = "trustEmail", skip_serializing_if = "Option::is_none")]
    pub trust_email: Option<bool>,
    #[serde(rename = "storeToken", skip_serializing_if = "Option::is_none")]
    pub store_token: Option<bool>,
    #[serde(
        rename = "addReadTokenRoleOnCreate",
        skip_serializing_if = "Option::is_none"
    )]
    pub add_read_token_role_on_create: Option<bool>,
    #[serde(
        rename = "authenticateByDefault",
        skip_serializing_if = "Option::is_none"
    )]
    pub authenticate_by_default: Option<bool>,
    #[serde(rename = "linkOnly", skip_serializing_if = "Option::is_none")]
    pub link_only: Option<bool>,
    #[serde(
        rename = "firstBrokerLoginFlowAlias",
        skip_serializing_if = "Option::is_none"
    )]
    pub first_broker_login_flow_alias: Option<String>,
    #[serde(
        rename = "postBrokerLoginFlowAlias",
        skip_serializing_if = "Option::is_none"
    )]
    pub post_broker_login_flow_alias: Option<String>,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<HashMap<String, String>>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl KeycloakResource for IdentityProviderRepresentation {
    fn get_identity(&self) -> Option<String> {
        self.alias.clone().or_else(|| self.internal_id.clone())
    }
    fn get_name(&self) -> String {
        self.alias.clone().unwrap_or_else(|| "unknown".to_string())
    }
    fn api_path() -> &'static str {
        "identity-provider/instances"
    }
    fn dir_name() -> &'static str {
        "identity-providers"
    }
}

impl ResourceMeta for IdentityProviderRepresentation {
    fn label() -> &'static str {
        "identity providers"
    }
    fn secret_prefix() -> &'static str {
        "idp"
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClientRepresentation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "clientId", skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    #[serde(rename = "redirectUris", skip_serializing_if = "Option::is_none")]
    pub redirect_uris: Option<Vec<String>>,
    #[serde(rename = "webOrigins", skip_serializing_if = "Option::is_none")]
    pub web_origins: Option<Vec<String>>,
    #[serde(rename = "publicClient", skip_serializing_if = "Option::is_none")]
    pub public_client: Option<bool>,
    #[serde(rename = "bearerOnly", skip_serializing_if = "Option::is_none")]
    pub bearer_only: Option<bool>,
    #[serde(
        rename = "serviceAccountsEnabled",
        skip_serializing_if = "Option::is_none"
    )]
    pub service_accounts_enabled: Option<bool>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl KeycloakResource for ClientRepresentation {
    fn get_identity(&self) -> Option<String> {
        self.client_id.clone().or_else(|| self.id.clone())
    }
    fn get_name(&self) -> String {
        self.client_id
            .clone()
            .or_else(|| self.name.clone())
            .unwrap_or_else(|| "unknown".to_string())
    }
    fn api_path() -> &'static str {
        "clients"
    }
    fn dir_name() -> &'static str {
        "clients"
    }
}

impl ResourceMeta for ClientRepresentation {
    fn label() -> &'static str {
        "clients"
    }
    fn secret_prefix() -> &'static str {
        "client"
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RoleRepresentation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "containerId", skip_serializing_if = "Option::is_none")]
    pub container_id: Option<String>,
    #[serde(default)]
    pub composite: bool,
    #[serde(rename = "clientRole", default)]
    pub client_role: bool,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl KeycloakResource for RoleRepresentation {
    fn get_identity(&self) -> Option<String> {
        Some(self.name.clone()).or_else(|| self.id.clone())
    }
    fn get_name(&self) -> String {
        self.name.clone()
    }
    fn api_path() -> &'static str {
        "roles"
    }
    fn dir_name() -> &'static str {
        "roles"
    }
    fn object_path(id: &str) -> String {
        format!("roles-by-id/{}", id)
    }
}

impl ResourceMeta for RoleRepresentation {
    fn label() -> &'static str {
        "roles"
    }
    fn secret_prefix() -> &'static str {
        "role"
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClientScopeRepresentation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<HashMap<String, String>>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl KeycloakResource for ClientScopeRepresentation {
    fn get_identity(&self) -> Option<String> {
        self.name.clone().or_else(|| self.id.clone())
    }
    fn get_name(&self) -> String {
        self.name.clone().unwrap_or_else(|| "unknown".to_string())
    }
    fn api_path() -> &'static str {
        "client-scopes"
    }
    fn dir_name() -> &'static str {
        "client-scopes"
    }
}

impl ResourceMeta for ClientScopeRepresentation {
    fn label() -> &'static str {
        "client scopes"
    }
    fn secret_prefix() -> &'static str {
        "client_scope"
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupRepresentation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(rename = "subGroups", skip_serializing_if = "Option::is_none")]
    pub sub_groups: Option<Vec<GroupRepresentation>>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl KeycloakResource for GroupRepresentation {
    fn get_identity(&self) -> Option<String> {
        self.path
            .clone()
            .or_else(|| self.id.clone())
            .or_else(|| self.name.clone())
    }
    fn get_name(&self) -> String {
        self.name.clone().unwrap_or_else(|| "unknown".to_string())
    }
    fn api_path() -> &'static str {
        "groups"
    }
    fn dir_name() -> &'static str {
        "groups"
    }
    fn get_filename(&self) -> String {
        format!(
            "{}-{}",
            self.get_name(),
            self.id.as_deref().unwrap_or("unknown")
        )
    }
}

impl ResourceMeta for GroupRepresentation {
    fn label() -> &'static str {
        "groups"
    }
    fn secret_prefix() -> &'static str {
        "group"
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CredentialRepresentation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temporary: Option<bool>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserRepresentation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(rename = "firstName", skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    #[serde(rename = "lastName", skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(rename = "emailVerified", skip_serializing_if = "Option::is_none")]
    pub email_verified: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials: Option<Vec<CredentialRepresentation>>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl KeycloakResource for UserRepresentation {
    fn get_identity(&self) -> Option<String> {
        self.username.clone().or_else(|| self.id.clone())
    }
    fn get_name(&self) -> String {
        self.username
            .clone()
            .unwrap_or_else(|| "unknown".to_string())
    }
    fn api_path() -> &'static str {
        "users"
    }
    fn dir_name() -> &'static str {
        "users"
    }
}

impl ResourceMeta for UserRepresentation {
    fn label() -> &'static str {
        "users"
    }
    fn secret_prefix() -> &'static str {
        "user"
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthenticationExecutionExportRepresentation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authenticator: Option<String>,
    #[serde(
        rename = "authenticatorConfig",
        skip_serializing_if = "Option::is_none"
    )]
    pub authenticator_config: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirement: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(rename = "authenticatorFlow", skip_serializing_if = "Option::is_none")]
    pub authenticator_flow: Option<bool>,
    #[serde(rename = "flowAlias", skip_serializing_if = "Option::is_none")]
    pub flow_alias: Option<String>,
    #[serde(rename = "userSetupAllowed", skip_serializing_if = "Option::is_none")]
    pub user_setup_allowed: Option<bool>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthenticationFlowRepresentation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "providerId", skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    #[serde(rename = "topLevel", skip_serializing_if = "Option::is_none")]
    pub top_level: Option<bool>,
    #[serde(rename = "builtIn", skip_serializing_if = "Option::is_none")]
    pub built_in: Option<bool>,
    #[serde(
        rename = "authenticationExecutions",
        skip_serializing_if = "Option::is_none"
    )]
    pub authentication_executions: Option<Vec<AuthenticationExecutionExportRepresentation>>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl KeycloakResource for AuthenticationFlowRepresentation {
    fn get_identity(&self) -> Option<String> {
        self.alias.clone().or_else(|| self.id.clone())
    }
    fn get_name(&self) -> String {
        self.alias.clone().unwrap_or_else(|| "unknown".to_string())
    }
    fn api_path() -> &'static str {
        "authentication/flows"
    }
    fn dir_name() -> &'static str {
        "authentication-flows"
    }
}

impl ResourceMeta for AuthenticationFlowRepresentation {
    fn label() -> &'static str {
        "authentication flows"
    }
    fn secret_prefix() -> &'static str {
        "flow"
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RequiredActionProviderRepresentation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "providerId", skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(rename = "defaultAction", skip_serializing_if = "Option::is_none")]
    pub default_action: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<HashMap<String, String>>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl KeycloakResource for RequiredActionProviderRepresentation {
    fn get_identity(&self) -> Option<String> {
        self.alias.clone()
    }
    fn get_name(&self) -> String {
        self.alias.clone().unwrap_or_else(|| "unknown".to_string())
    }
    fn api_path() -> &'static str {
        "authentication/required-actions"
    }
    fn dir_name() -> &'static str {
        "required-actions"
    }
}

impl ResourceMeta for RequiredActionProviderRepresentation {
    fn label() -> &'static str {
        "required actions"
    }
    fn secret_prefix() -> &'static str {
        "action"
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComponentRepresentation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "providerId", skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    #[serde(rename = "providerType", skip_serializing_if = "Option::is_none")]
    pub provider_type: Option<String>,
    #[serde(rename = "parentId", skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(rename = "subType", skip_serializing_if = "Option::is_none")]
    pub sub_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<HashMap<String, Value>>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl KeycloakResource for ComponentRepresentation {
    fn get_identity(&self) -> Option<String> {
        self.id.clone().or_else(|| self.name.clone())
    }
    fn get_name(&self) -> String {
        self.name.clone().unwrap_or_else(|| "unknown".to_string())
    }
    fn api_path() -> &'static str {
        "components"
    }
    fn dir_name() -> &'static str {
        "components"
    }
    fn get_filename(&self) -> String {
        format!(
            "{}-{}",
            self.get_name(),
            self.id.as_deref().unwrap_or("unknown")
        )
    }
}

impl ResourceMeta for ComponentRepresentation {
    fn label() -> &'static str {
        "components"
    }
    fn secret_prefix() -> &'static str {
        "component"
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KeyMetadataRepresentation {
    #[serde(rename = "providerId")]
    pub provider_id: Option<String>,
    #[serde(rename = "providerPriority")]
    pub provider_priority: Option<i64>,
    pub kid: Option<String>,
    pub status: Option<String>,
    #[serde(rename = "type")]
    pub key_type: Option<String>,
    pub algorithm: Option<String>,
    #[serde(rename = "publicKey")]
    pub public_key: Option<String>,
    pub certificate: Option<String>,
    pub use_: Option<String>,
    #[serde(rename = "validTo")]
    pub valid_to: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KeysMetadataRepresentation {
    pub active: Option<HashMap<String, String>>,
    pub keys: Option<Vec<KeyMetadataRepresentation>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_realm_serialization() {
        let mut extra = HashMap::new();
        extra.insert("someExtraField".to_string(), json!("someValue"));

        let realm = RealmRepresentation {
            realm: "myrealm".to_string(),
            enabled: Some(true),
            display_name: Some("My Realm".to_string()),
            extra,
        };

        let json_str = serde_json::to_string(&realm).expect("Failed to serialize realm");
        let json_val: Value = serde_json::from_str(&json_str).expect("Failed to parse json");

        assert_eq!(json_val["realm"], "myrealm");
        assert_eq!(json_val["displayName"], "My Realm");
        assert_eq!(json_val["someExtraField"], "someValue");

        let deserialized: RealmRepresentation =
            serde_json::from_str(&json_str).expect("Failed to deserialize realm");
        assert_eq!(deserialized.realm, "myrealm");
        assert_eq!(deserialized.display_name, Some("My Realm".to_string()));
        assert_eq!(
            deserialized.extra.get("someExtraField"),
            Some(&json!("someValue"))
        );
    }

    #[test]
    fn test_identity_provider_serialization() {
        let idp = IdentityProviderRepresentation {
            internal_id: None,
            alias: Some("google".to_string()),
            provider_id: Some("google".to_string()),
            enabled: Some(true),
            update_profile_first_login_mode: Some("on".to_string()),
            trust_email: None,
            store_token: None,
            add_read_token_role_on_create: None,
            authenticate_by_default: None,
            link_only: None,
            first_broker_login_flow_alias: None,
            post_broker_login_flow_alias: None,
            display_name: None,
            config: None,
            extra: HashMap::new(),
        };

        let json_str = serde_json::to_string(&idp).expect("Failed to serialize idp");
        let json_val: Value = serde_json::from_str(&json_str).expect("Failed to parse json");

        assert_eq!(json_val["providerId"], "google");
        assert_eq!(json_val["updateProfileFirstLoginMode"], "on");

        let deserialized: IdentityProviderRepresentation =
            serde_json::from_str(&json_str).expect("Failed to deserialize idp");
        assert_eq!(
            deserialized.update_profile_first_login_mode,
            Some("on".to_string())
        );
    }

    #[test]
    fn test_client_serialization() {
        let client = ClientRepresentation {
            id: None,
            client_id: Some("my-client".to_string()),
            name: None,
            description: None,
            enabled: None,
            protocol: None,
            redirect_uris: Some(vec!["http://localhost/*".to_string()]),
            web_origins: None,
            public_client: Some(true),
            bearer_only: None,
            service_accounts_enabled: None,
            extra: HashMap::new(),
        };

        let json_str = serde_json::to_string(&client).expect("Failed to serialize client");
        let json_val: Value = serde_json::from_str(&json_str).expect("Failed to parse json");

        assert_eq!(json_val["clientId"], "my-client");
        assert_eq!(json_val["publicClient"], true);
        assert_eq!(json_val["redirectUris"][0], "http://localhost/*");

        let deserialized: ClientRepresentation =
            serde_json::from_str(&json_str).expect("Failed to deserialize client");
        assert_eq!(deserialized.client_id, Some("my-client".to_string()));
        assert_eq!(
            deserialized.redirect_uris,
            Some(vec!["http://localhost/*".to_string()])
        );
    }

    #[test]
    fn test_role_serialization() {
        let role = RoleRepresentation {
            id: None,
            name: "admin".to_string(),
            description: None,
            container_id: Some("realm-id".to_string()),
            composite: false,
            client_role: true,
            extra: HashMap::new(),
        };

        let json_str = serde_json::to_string(&role).expect("Failed to serialize role");
        let json_val: Value = serde_json::from_str(&json_str).expect("Failed to parse json");

        assert_eq!(json_val["containerId"], "realm-id");
        assert_eq!(json_val["clientRole"], true);

        let deserialized: RoleRepresentation =
            serde_json::from_str(&json_str).expect("Failed to deserialize role");
        assert_eq!(deserialized.container_id, Some("realm-id".to_string()));
    }

    #[test]
    fn test_group_serialization() {
        let sub_group = GroupRepresentation {
            id: None,
            name: Some("subgroup".to_string()),
            path: None,
            sub_groups: None,
            extra: HashMap::new(),
        };

        let group = GroupRepresentation {
            id: None,
            name: Some("group".to_string()),
            path: None,
            sub_groups: Some(vec![sub_group]),
            extra: HashMap::new(),
        };

        let json_str = serde_json::to_string(&group).expect("Failed to serialize group");
        let json_val: Value = serde_json::from_str(&json_str).expect("Failed to parse json");

        assert_eq!(json_val["subGroups"][0]["name"], "subgroup");

        let deserialized: GroupRepresentation =
            serde_json::from_str(&json_str).expect("Failed to deserialize group");
        assert_eq!(
            deserialized.sub_groups.expect("Failed to get sub_groups")[0].name,
            Some("subgroup".to_string())
        );
    }

    #[test]
    fn test_user_serialization() {
        let user = UserRepresentation {
            id: None,
            username: Some("jdoe".to_string()),
            enabled: None,
            first_name: Some("John".to_string()),
            last_name: Some("Doe".to_string()),
            email: None,
            email_verified: Some(true),
            credentials: None,
            extra: HashMap::new(),
        };

        let json_str = serde_json::to_string(&user).expect("Failed to serialize user");
        let json_val: Value = serde_json::from_str(&json_str).expect("Failed to parse json");

        assert_eq!(json_val["firstName"], "John");
        assert_eq!(json_val["lastName"], "Doe");
        assert_eq!(json_val["emailVerified"], true);

        let deserialized: UserRepresentation =
            serde_json::from_str(&json_str).expect("Failed to deserialize user");
        assert_eq!(deserialized.first_name, Some("John".to_string()));
    }
}
