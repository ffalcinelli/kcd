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

        let json_str = serde_json::to_string(&realm).unwrap();
        let json_val: Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(json_val["realm"], "myrealm");
        assert_eq!(json_val["displayName"], "My Realm");
        assert_eq!(json_val["someExtraField"], "someValue");

        let deserialized: RealmRepresentation = serde_json::from_str(&json_str).unwrap();
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

        let json_str = serde_json::to_string(&idp).unwrap();
        let json_val: Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(json_val["providerId"], "google");
        assert_eq!(json_val["updateProfileFirstLoginMode"], "on");

        let deserialized: IdentityProviderRepresentation = serde_json::from_str(&json_str).unwrap();
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

        let json_str = serde_json::to_string(&client).unwrap();
        let json_val: Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(json_val["clientId"], "my-client");
        assert_eq!(json_val["publicClient"], true);
        assert_eq!(json_val["redirectUris"][0], "http://localhost/*");

        let deserialized: ClientRepresentation = serde_json::from_str(&json_str).unwrap();
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

        let json_str = serde_json::to_string(&role).unwrap();
        let json_val: Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(json_val["containerId"], "realm-id");
        assert_eq!(json_val["clientRole"], true);

        let deserialized: RoleRepresentation = serde_json::from_str(&json_str).unwrap();
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

        let json_str = serde_json::to_string(&group).unwrap();
        let json_val: Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(json_val["subGroups"][0]["name"], "subgroup");

        let deserialized: GroupRepresentation = serde_json::from_str(&json_str).unwrap();
        assert_eq!(
            deserialized.sub_groups.unwrap()[0].name,
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

        let json_str = serde_json::to_string(&user).unwrap();
        let json_val: Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(json_val["firstName"], "John");
        assert_eq!(json_val["lastName"], "Doe");
        assert_eq!(json_val["emailVerified"], true);

        let deserialized: UserRepresentation = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.first_name, Some("John".to_string()));
    }
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
