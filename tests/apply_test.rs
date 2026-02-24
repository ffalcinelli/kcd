mod common;
use app::apply;
use app::client::KeycloakClient;
use app::models::{ClientRepresentation, RealmRepresentation, RoleRepresentation};
use common::start_mock_server;
use std::fs;
use tempfile::tempdir;

#[tokio::test]
async fn test_apply() {
    let mock_url = start_mock_server().await;
    let mut client = KeycloakClient::new(mock_url, "test-realm".to_string());
    client
        .login("admin-cli", Some("secret"), None, None)
        .await
        .expect("Login failed");

    let dir = tempdir().unwrap();
    let input_dir = dir.path().to_path_buf();

    // Create realm.yaml
    let realm = RealmRepresentation {
        realm: "test-realm".to_string(),
        enabled: Some(true),
        display_name: Some("Updated Realm".to_string()),
        extra: std::collections::HashMap::new(),
    };
    fs::write(
        input_dir.join("realm.yaml"),
        serde_yaml::to_string(&realm).unwrap(),
    )
    .unwrap();

    // Create roles
    let roles_dir = input_dir.join("roles");
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

    // Create clients
    let clients_dir = input_dir.join("clients");
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

    // Run apply
    apply::run(&client, input_dir.clone())
        .await
        .expect("Apply failed");
}
