mod common;
use app::client::KeycloakClient;
use app::models::{ComponentRepresentation, RealmRepresentation};
use app::plan;
use common::start_mock_server;
use std::fs;
use tempfile::tempdir;

#[tokio::test]
async fn test_plan_keys_and_extended() {
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
    fs::create_dir_all(&realm_dir).unwrap();

    // 1. Create realm.yaml
    let realm = RealmRepresentation {
        realm: "test-realm".to_string(),
        enabled: Some(true),
        display_name: Some("Test Realm".to_string()),
        extra: std::collections::HashMap::new(),
    };
    fs::write(
        realm_dir.join("realm.yaml"),
        serde_yaml::to_string(&realm).unwrap(),
    )
    .unwrap();

    // 2. Create keys directory and a key component
    let keys_dir = realm_dir.join("keys");
    fs::create_dir(&keys_dir).unwrap();

    // This matches NOTHING in the mock server (mock server returns kid "key-1" in /keys, but /components returns component-1)
    // Actually mock server /components returns:
    // { "id": "c1", "name": "component-1", "providerId": "ldap", "providerType": "org.keycloak.storage.UserStorageProvider" }

    let key_component = ComponentRepresentation {
        id: None,
        name: Some("new-key".to_string()),
        provider_id: Some("rsa-generated".to_string()),
        provider_type: Some("org.keycloak.keys.KeyProvider".to_string()),
        sub_type: None,
        parent_id: None,
        config: None,
        extra: std::collections::HashMap::new(),
    };
    fs::write(
        keys_dir.join("new-key.yaml"),
        serde_yaml::to_string(&key_component).unwrap(),
    )
    .unwrap();

    // 3. Create a component with ID already set (to hit local_component.id.is_some() branch)
    let components_dir = realm_dir.join("components");
    fs::create_dir(&components_dir).unwrap();
    let existing_comp = ComponentRepresentation {
        id: Some("c1".to_string()),
        name: Some("component-1".to_string()),
        provider_id: Some("ldap".to_string()),
        provider_type: Some("org.keycloak.storage.UserStorageProvider".to_string()),
        sub_type: None,
        parent_id: None,
        config: None,
        extra: std::collections::HashMap::new(),
    };
    fs::write(
        components_dir.join("component-1.yaml"),
        serde_yaml::to_string(&existing_comp).unwrap(),
    )
    .unwrap();

    // Run plan with changes_only=true to trigger check_keys_drift
    plan::run(
        &client,
        workspace_dir.clone(),
        true,
        false,
        &["test-realm".to_string()],
    )
    .await
    .expect("Plan failed");

    // Run plan with changes_only=false
    plan::run(
        &client,
        workspace_dir.clone(),
        false,
        false,
        &["test-realm".to_string()],
    )
    .await
    .expect("Plan failed");
}

#[tokio::test]
async fn test_plan_substitute_secrets_error() {
    let mock_url = start_mock_server().await;
    let mut client = KeycloakClient::new(mock_url);
    client.set_target_realm("test-realm".to_string());
    client.set_token("mock".to_string());

    let dir = tempdir().unwrap();
    let workspace_dir = dir.path().to_path_buf();
    let realm_dir = workspace_dir.join("test-realm");
    fs::create_dir_all(&realm_dir).unwrap();

    // Create a client with a missing environment variable
    let clients_dir = realm_dir.join("clients");
    fs::create_dir(&clients_dir).unwrap();
    fs::write(
        clients_dir.join("error-client.yaml"),
        "clientId: error-client\nsecret: '${KEYCLOAK_MISSING_VAR}'\n",
    )
    .unwrap();

    let res = plan::run(
        &client,
        workspace_dir,
        false,
        false,
        &["test-realm".to_string()],
    )
    .await;

    // Should fail due to missing environment variable
    assert!(res.is_err());
    assert!(
        res.unwrap_err()
            .to_string()
            .contains("Missing required environment variable")
    );
}
