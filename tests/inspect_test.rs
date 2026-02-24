mod common;
use common::start_mock_server;
use app::client::KeycloakClient;
use app::inspect;
use std::fs;
use tempfile::tempdir;

#[tokio::test]
async fn test_inspect() {
    let mock_url = start_mock_server().await;
    let mut client = KeycloakClient::new(mock_url, "test-realm".to_string());
    client.login("admin-cli", Some("secret"), None, None).await.expect("Login failed");

    let dir = tempdir().unwrap();
    let output_dir = dir.path().to_path_buf();

    inspect::run(&client, output_dir.clone()).await.expect("Inspect failed");

    assert!(output_dir.join("realm.yaml").exists(), "realm.yaml missing");
    assert!(output_dir.join("clients/client-1.yaml").exists(), "client-1.yaml missing");
    assert!(output_dir.join("roles/role-1.yaml").exists(), "role-1.yaml missing");

    // Check content
    let realm_content = fs::read_to_string(output_dir.join("realm.yaml")).unwrap();
    assert!(realm_content.contains("test-realm"));
}
