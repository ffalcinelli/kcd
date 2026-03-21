mod common;
use app::client::KeycloakClient;
use app::models::{ClientRepresentation, RealmRepresentation, RoleRepresentation};
use app::plan;
use common::start_mock_server;
use std::fs;
use tempfile::tempdir;

#[tokio::test]
async fn test_plan() {
    let mock_url = start_mock_server().await;
    let mut client = KeycloakClient::new(mock_url);
    client.set_target_realm("test-realm".to_string());
    client
        .login("admin-cli", Some("secret"), None, None)
        .await
        .expect("Login failed");

    let dir = tempdir().unwrap();
    let workspace_dir = dir.path().to_path_buf();
    let realm_dir = workspace_dir.join("test-realm");
    std::fs::create_dir_all(&realm_dir).unwrap();

    // Create realm.yaml
    let realm = RealmRepresentation {
        realm: "test-realm".to_string(),
        enabled: Some(true),
        display_name: Some("Updated Realm".to_string()),
        extra: std::collections::HashMap::new(),
    };
    fs::write(
        realm_dir.join("realm.yaml"),
        serde_yaml::to_string(&realm).unwrap(),
    )
    .unwrap();

    // Create roles
    let roles_dir = realm_dir.join("roles");
    fs::create_dir(&roles_dir).unwrap();
    let role = RoleRepresentation {
        id: None,
        name: "new-role".to_string(),
        description: Some("New Role".to_string()),
        container_id: None,
        composite: false,
        client_role: false,
        extra: std::collections::HashMap::new(),
    };
    fs::write(
        roles_dir.join("new-role.yaml"),
        serde_yaml::to_string(&role).unwrap(),
    )
    .unwrap();

    let existing_role = RoleRepresentation {
        id: None,
        name: "role-1".to_string(), // Matches mock server response
        description: Some("Updated Role 1".to_string()),
        container_id: None,
        composite: false,
        client_role: false,
        extra: std::collections::HashMap::new(),
    };
    fs::write(
        roles_dir.join("role-1.yaml"),
        serde_yaml::to_string(&existing_role).unwrap(),
    )
    .unwrap();

    // Create clients
    let clients_dir = realm_dir.join("clients");
    fs::create_dir(&clients_dir).unwrap();
    let client_rep = ClientRepresentation {
        id: None,
        client_id: Some("new-client".to_string()),
        name: Some("New Client".to_string()),
        description: None,
        enabled: Some(true),
        protocol: None,
        redirect_uris: None,
        web_origins: None,
        public_client: None,
        bearer_only: None,
        service_accounts_enabled: None,
        extra: std::collections::HashMap::new(),
    };
    fs::write(
        clients_dir.join("new-client.yaml"),
        serde_yaml::to_string(&client_rep).unwrap(),
    )
    .unwrap();

    // Run plan
    plan::run(
        &client,
        workspace_dir.clone(),
        false, // changes_only
        false, // interactive
        &["test-realm".to_string()],
    )
    .await
    .expect("Plan failed");
}
