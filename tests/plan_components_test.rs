use app::client::KeycloakClient;
use app::plan::components::{check_keys_drift, plan_components_or_keys};
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::fs;

#[tokio::test]
async fn test_plan_components_no_dir() {
    let client = KeycloakClient::new("http://localhost:8080".to_string());
    let dir = tempdir().unwrap();
    let workspace_dir = dir.path();
    let mut changed_files = Vec::new();
    let env_vars = Arc::new(HashMap::new());

    // Should not fail if directory doesn't exist
    let res = plan_components_or_keys(
        &client,
        workspace_dir,
        false,
        false,
        "non-existent",
        env_vars,
        &mut changed_files,
    )
    .await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn test_check_keys_drift_fail() {
    // Client that will fail to connect
    let client = KeycloakClient::new("http://localhost:1".to_string());
    let res = check_keys_drift(&client, true).await;
    // check_keys_drift ignores error if not available
    assert!(res.is_ok());
}

#[tokio::test]
async fn test_plan_components_with_invalid_yaml() {
    let client = KeycloakClient::new("http://localhost:8080".to_string());
    let dir = tempdir().unwrap();
    let workspace_dir = dir.path();
    let components_dir = workspace_dir.join("components");
    fs::create_dir_all(&components_dir).await.unwrap();
    fs::write(components_dir.join("bad.yaml"), "invalid: [ :")
        .await
        .unwrap();

    let mut changed_files = Vec::new();
    let env_vars = Arc::new(HashMap::new());

    let res = plan_components_or_keys(
        &client,
        workspace_dir,
        false,
        false,
        "components",
        env_vars,
        &mut changed_files,
    )
    .await;
    assert!(res.is_err());
}
