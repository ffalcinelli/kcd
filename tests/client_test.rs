mod common;
use app::client::KeycloakClient;
use app::models::ClientRepresentation;
use common::start_mock_server;
use tokio;

#[tokio::test]
async fn test_login_password_grant() {
    let mock_url = start_mock_server().await;
    let mut client = KeycloakClient::new(mock_url, "test-realm".to_string());

    let result = client
        .login("admin-cli", None, Some("admin"), Some("admin"))
        .await;
    assert!(result.is_ok(), "Login failed: {:?}", result.err());
    assert_eq!(client.get_token().unwrap(), "mock_token");
}

#[tokio::test]
async fn test_login_client_credentials_grant() {
    let mock_url = start_mock_server().await;
    let mut client = KeycloakClient::new(mock_url, "test-realm".to_string());

    let result = client.login("admin-cli", Some("secret"), None, None).await;
    assert!(result.is_ok(), "Login failed: {:?}", result.err());
    assert_eq!(client.get_token().unwrap(), "mock_token");
}

#[tokio::test]
async fn test_login_fail() {
    let mock_url = start_mock_server().await;
    let mut client = KeycloakClient::new(mock_url, "test-realm".to_string());

    let result = client
        .login("admin-cli", None, Some("admin"), Some("wrong"))
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_realm() {
    let mock_url = start_mock_server().await;
    let mut client = KeycloakClient::new(mock_url, "test-realm".to_string());
    client
        .login("admin-cli", Some("secret"), None, None)
        .await
        .expect("Login failed");

    let realm = client.get_realm().await.expect("Failed to get realm");
    assert_eq!(realm.realm, "test-realm");
    assert_eq!(realm.display_name, Some("Test Realm".to_string()));
}

#[tokio::test]
async fn test_get_clients() {
    let mock_url = start_mock_server().await;
    let mut client = KeycloakClient::new(mock_url, "test-realm".to_string());
    client
        .login("admin-cli", Some("secret"), None, None)
        .await
        .expect("Login failed");

    let clients = client.get_clients().await.expect("Failed to get clients");
    assert_eq!(clients.len(), 2);
    assert_eq!(clients[0].client_id, Some("client-1".to_string()));
}

#[tokio::test]
async fn test_get_roles() {
    let mock_url = start_mock_server().await;
    let mut client = KeycloakClient::new(mock_url, "test-realm".to_string());
    client
        .login("admin-cli", Some("secret"), None, None)
        .await
        .expect("Login failed");

    let roles = client.get_roles().await.expect("Failed to get roles");
    assert_eq!(roles.len(), 2);
    assert_eq!(roles[0].name, "role-1");
}

#[tokio::test]
async fn test_create_client() {
    let mock_url = start_mock_server().await;
    let mut client = KeycloakClient::new(mock_url, "test-realm".to_string());
    client
        .login("admin-cli", Some("secret"), None, None)
        .await
        .expect("Login failed");

    let client_rep = ClientRepresentation {
        id: None,
        client_id: Some("new-client".to_string()),
        name: None,
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

    let result = client.create_client(&client_rep).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_update_client() {
    let mock_url = start_mock_server().await;
    let mut client = KeycloakClient::new(mock_url, "test-realm".to_string());
    client
        .login("admin-cli", Some("secret"), None, None)
        .await
        .expect("Login failed");

    let client_rep = ClientRepresentation {
        id: Some("1".to_string()),
        client_id: Some("client-1".to_string()),
        name: Some("Updated Name".to_string()),
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

    let result = client.update_client("1", &client_rep).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_delete_client() {
    let mock_url = start_mock_server().await;
    let mut client = KeycloakClient::new(mock_url, "test-realm".to_string());
    client
        .login("admin-cli", Some("secret"), None, None)
        .await
        .expect("Login failed");

    let result = client.delete_client("1").await;
    assert!(result.is_ok());
}
