mod common;
use common::start_mock_server;
use kcd::client::KeycloakClient;
use kcd::plan;
use std::fs;
use tempfile::tempdir;

#[tokio::test]
async fn test_plan_non_existent_workspace() {
    let mock_url = start_mock_server().await;
    let client = KeycloakClient::new(mock_url);
    let res = plan::run(
        &client,
        std::path::PathBuf::from("non-existent-123"),
        false,
        false,
        &[],
    )
    .await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_plan_empty_workspace() {
    let mock_url = start_mock_server().await;
    let client = KeycloakClient::new(mock_url);
    let dir = tempdir().unwrap();
    let res = plan::run(&client, dir.path().to_path_buf(), false, false, &[]).await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn test_plan_with_secrets_file() {
    let mock_url = start_mock_server().await;
    let mut client = KeycloakClient::new(mock_url);
    client.set_target_realm("test-realm".to_string());
    client
        .login("admin-cli", Some("secret"), None, None)
        .await
        .unwrap();

    let dir = tempdir().unwrap();
    let workspace_dir = dir.path().to_path_buf();
    fs::write(workspace_dir.join(".secrets"), "MY_SECRET=value").unwrap();

    let realm_dir = workspace_dir.join("test-realm");
    fs::create_dir_all(&realm_dir).unwrap();

    let res = plan::run(
        &client,
        workspace_dir,
        false,
        false,
        &["test-realm".to_string()],
    )
    .await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn test_plan_cleanup_old_plan_file() {
    let mock_url = start_mock_server().await;
    let mut client = KeycloakClient::new(mock_url);
    client.set_target_realm("test-realm".to_string());
    client
        .login("admin-cli", Some("secret"), None, None)
        .await
        .unwrap();

    let dir = tempdir().unwrap();
    let workspace_dir = dir.path().to_path_buf();
    let plan_file = workspace_dir.join(".kcdplan");
    fs::write(&plan_file, "[]").unwrap();

    let realm_dir = workspace_dir.join("test-realm");
    fs::create_dir_all(&realm_dir).unwrap();

    // No changes, so it should remove .kcdplan
    let res = plan::run(
        &client,
        workspace_dir.clone(),
        false,
        false,
        &["test-realm".to_string()],
    )
    .await;
    assert!(res.is_ok());
    assert!(!plan_file.exists());
}

#[tokio::test]
async fn test_plan_realm_not_found_remote() {
    let mock_url = start_mock_server().await;
    let mut client = KeycloakClient::new(mock_url);
    client.set_target_realm("new-realm".to_string());
    client
        .login("admin-cli", Some("secret"), None, None)
        .await
        .unwrap();

    let dir = tempdir().unwrap();
    let workspace_dir = dir.path().to_path_buf();
    let realm_dir = workspace_dir.join("new-realm");
    fs::create_dir_all(&realm_dir).unwrap();

    let realm = kcd::models::RealmRepresentation {
        realm: "new-realm".to_string(),
        enabled: Some(true),
        display_name: Some("New Realm".to_string()),
        extra: std::collections::HashMap::new(),
    };
    fs::write(
        realm_dir.join("realm.yaml"),
        serde_yaml::to_string(&realm).unwrap(),
    )
    .unwrap();

    // "new-realm" will return 404 from mock server
    let res = plan::run(
        &client,
        workspace_dir,
        false,
        false,
        &["new-realm".to_string()],
    )
    .await;
    assert!(res.is_ok());
}
