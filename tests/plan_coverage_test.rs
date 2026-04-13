mod common;
use common::start_mock_server;
use kcd::client::KeycloakClient;
use kcd::plan;
use std::fs;
use std::sync::Arc;
use tempfile::tempdir;

use kcd::utils::ui::DialoguerUi;

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
        Arc::new(DialoguerUi),
    )
    .await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_plan_empty_workspace() {
    let mock_url = start_mock_server().await;
    let client = KeycloakClient::new(mock_url);
    let dir = tempdir().unwrap();
    let res = plan::run(
        &client,
        dir.path().to_path_buf(),
        false,
        false,
        &[],
        Arc::new(DialoguerUi),
    )
    .await;
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
        Arc::new(DialoguerUi),
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
        Arc::new(DialoguerUi),
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
        Arc::new(DialoguerUi),
    )
    .await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn test_plan_resources_creation() {
    let mock_url = start_mock_server().await;
    let mut client = KeycloakClient::new(mock_url);
    client.set_target_realm("test-realm".to_string());
    client
        .login("admin-cli", Some("secret"), None, None)
        .await
        .unwrap();

    let dir = tempdir().unwrap();
    let workspace_dir = dir.path().to_path_buf();
    let realm_dir = workspace_dir.join("test-realm");
    fs::create_dir_all(&realm_dir).unwrap();

    let roles_dir = realm_dir.join("roles");
    fs::create_dir_all(&roles_dir).unwrap();
    let role = kcd::models::RoleRepresentation {
        id: None,
        name: "new-role".to_string(),
        description: Some("New role".to_string()),
        container_id: None,
        composite: false,
        client_role: false,
        extra: std::collections::HashMap::new(),
    };
    let role_path = roles_dir.join("new-role.yaml");
    fs::write(&role_path, serde_yaml::to_string(&role).unwrap()).unwrap();

    let res = plan::run(
        &client,
        workspace_dir.clone(),
        false,
        false,
        &["test-realm".to_string()],
        Arc::new(DialoguerUi),
    )
    .await;
    assert!(res.is_ok());

    let plan_file = workspace_dir.join(".kcdplan");
    assert!(plan_file.exists());
    let plan_content = fs::read_to_string(plan_file).unwrap();
    assert!(plan_content.contains("test-realm/roles/new-role.yaml"));
}

#[tokio::test]
async fn test_plan_resources_with_secrets() {
    let mock_url = start_mock_server().await;
    let mut client = KeycloakClient::new(mock_url);
    client.set_target_realm("test-realm".to_string());
    client
        .login("admin-cli", Some("secret"), None, None)
        .await
        .unwrap();

    let dir = tempdir().unwrap();
    let workspace_dir = dir.path().to_path_buf();
    let realm_dir = workspace_dir.join("test-realm");
    fs::create_dir_all(&realm_dir).unwrap();

    // Use environment variable for secret
    unsafe {
        std::env::set_var("ROLE_DESC", "Secret Description");
    }

    let roles_dir = realm_dir.join("roles");
    fs::create_dir_all(&roles_dir).unwrap();
    let role_yaml = "
name: secret-role
description: ${ROLE_DESC}
";
    let role_path = roles_dir.join("secret-role.yaml");
    fs::write(&role_path, role_yaml).unwrap();

    let res = plan::run(
        &client,
        workspace_dir.clone(),
        false,
        false,
        &["test-realm".to_string()],
        Arc::new(DialoguerUi),
    )
    .await;
    assert!(res.is_ok());

    let plan_file = workspace_dir.join(".kcdplan");
    assert!(plan_file.exists());
    let plan_content = fs::read_to_string(plan_file).unwrap();
    assert!(plan_content.contains("test-realm/roles/secret-role.yaml"));
}

#[tokio::test]
async fn test_plan_resources_invalid_yaml() {
    let mock_url = start_mock_server().await;
    let mut client = KeycloakClient::new(mock_url);
    client.set_target_realm("test-realm".to_string());
    client
        .login("admin-cli", Some("secret"), None, None)
        .await
        .unwrap();

    let dir = tempdir().unwrap();
    let workspace_dir = dir.path().to_path_buf();
    let realm_dir = workspace_dir.join("test-realm");
    fs::create_dir_all(&realm_dir).unwrap();

    let roles_dir = realm_dir.join("roles");
    fs::create_dir_all(&roles_dir).unwrap();
    fs::write(roles_dir.join("bad.yaml"), "not : [ : valid").unwrap();

    let res = plan::run(
        &client,
        workspace_dir,
        false,
        false,
        &["test-realm".to_string()],
        Arc::new(DialoguerUi),
    )
    .await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_plan_resources_missing_identity() {
    let mock_url = start_mock_server().await;
    let mut client = KeycloakClient::new(mock_url);
    client.set_target_realm("test-realm".to_string());
    client
        .login("admin-cli", Some("secret"), None, None)
        .await
        .unwrap();

    let dir = tempdir().unwrap();
    let workspace_dir = dir.path().to_path_buf();
    let realm_dir = workspace_dir.join("test-realm");
    fs::create_dir_all(&realm_dir).unwrap();

    let roles_dir = realm_dir.join("roles");
    fs::create_dir_all(&roles_dir).unwrap();
    // Missing 'name' for role
    fs::write(roles_dir.join("no-id.yaml"), "description: missing name").unwrap();

    let res = plan::run(
        &client,
        workspace_dir,
        false,
        false,
        &["test-realm".to_string()],
        Arc::new(DialoguerUi),
    )
    .await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_plan_resources_update() {
    let mock_url = start_mock_server().await;
    let mut client = KeycloakClient::new(mock_url);
    client.set_target_realm("test-realm".to_string());
    client
        .login("admin-cli", Some("secret"), None, None)
        .await
        .unwrap();

    let dir = tempdir().unwrap();
    let workspace_dir = dir.path().to_path_buf();
    let realm_dir = workspace_dir.join("test-realm");
    fs::create_dir_all(&realm_dir).unwrap();

    let roles_dir = realm_dir.join("roles");
    fs::create_dir_all(&roles_dir).unwrap();
    // 'role-1' exists in mock server
    let role = kcd::models::RoleRepresentation {
        id: Some("r1".to_string()),
        name: "role-1".to_string(),
        description: Some("Updated description".to_string()),
        container_id: None,
        composite: false,
        client_role: false,
        extra: std::collections::HashMap::new(),
    };
    let role_path = roles_dir.join("role-1.yaml");
    fs::write(&role_path, serde_yaml::to_string(&role).unwrap()).unwrap();

    let res = plan::run(
        &client,
        workspace_dir.clone(),
        false,
        false,
        &["test-realm".to_string()],
        Arc::new(DialoguerUi),
    )
    .await;
    assert!(res.is_ok());

    let plan_file = workspace_dir.join(".kcdplan");
    assert!(plan_file.exists());
}

#[tokio::test]
async fn test_plan_resources_changes_only() {
    let mock_url = start_mock_server().await;
    let mut client = KeycloakClient::new(mock_url);
    client.set_target_realm("test-realm".to_string());
    client
        .login("admin-cli", Some("secret"), None, None)
        .await
        .unwrap();

    let dir = tempdir().unwrap();
    let workspace_dir = dir.path().to_path_buf();
    let realm_dir = workspace_dir.join("test-realm");
    fs::create_dir_all(&realm_dir).unwrap();

    let roles_dir = realm_dir.join("roles");
    fs::create_dir_all(&roles_dir).unwrap();
    // 'role-1' exists in mock server, same content
    let role = kcd::models::RoleRepresentation {
        id: Some("r1".to_string()),
        name: "role-1".to_string(),
        description: Some("Role 1".to_string()),
        container_id: None,
        composite: false,
        client_role: false,
        extra: std::collections::HashMap::new(),
    };
    let role_path = roles_dir.join("role-1.yaml");
    fs::write(&role_path, serde_yaml::to_string(&role).unwrap()).unwrap();

    let res = plan::run(
        &client,
        workspace_dir.clone(),
        true, // changes_only
        false,
        &["test-realm".to_string()],
        Arc::new(DialoguerUi),
    )
    .await;
    assert!(res.is_ok());

    let plan_file = workspace_dir.join(".kcdplan");
    assert!(!plan_file.exists());
}

use kcd::utils::ui::MockUi;
use std::sync::Mutex;

#[tokio::test]
async fn test_plan_interactive_include() {
    let mock_url = start_mock_server().await;
    let mut client = KeycloakClient::new(mock_url);
    client.set_target_realm("test-realm".to_string());
    client
        .login("admin-cli", Some("secret"), None, None)
        .await
        .unwrap();

    let dir = tempdir().unwrap();
    let workspace_dir = dir.path().to_path_buf();
    let realm_dir = workspace_dir.join("test-realm");
    fs::create_dir_all(&realm_dir).unwrap();

    let roles_dir = realm_dir.join("roles");
    fs::create_dir_all(&roles_dir).unwrap();
    let role = kcd::models::RoleRepresentation {
        id: None,
        name: "interactive-role".to_string(),
        description: Some("Interactive role".to_string()),
        container_id: None,
        composite: false,
        client_role: false,
        extra: std::collections::HashMap::new(),
    };
    let role_path = roles_dir.join("interactive-role.yaml");
    fs::write(&role_path, serde_yaml::to_string(&role).unwrap()).unwrap();

    let ui = Arc::new(MockUi {
        inputs: Mutex::new(vec![]),
        confirms: Mutex::new(vec![true]), // Confirm inclusion
        selects: Mutex::new(vec![]),
        passwords: Mutex::new(vec![]),
    });

    let res = plan::run(
        &client,
        workspace_dir.clone(),
        false,
        true, // interactive
        &["test-realm".to_string()],
        ui,
    )
    .await;
    assert!(res.is_ok());

    let plan_file = workspace_dir.join(".kcdplan");
    assert!(plan_file.exists());
    let plan_content = fs::read_to_string(plan_file).unwrap();
    assert!(plan_content.contains("test-realm/roles/interactive-role.yaml"));
}

#[tokio::test]
async fn test_plan_interactive_exclude() {
    let mock_url = start_mock_server().await;
    let mut client = KeycloakClient::new(mock_url);
    client.set_target_realm("test-realm".to_string());
    client
        .login("admin-cli", Some("secret"), None, None)
        .await
        .unwrap();

    let dir = tempdir().unwrap();
    let workspace_dir = dir.path().to_path_buf();
    let realm_dir = workspace_dir.join("test-realm");
    fs::create_dir_all(&realm_dir).unwrap();

    let roles_dir = realm_dir.join("roles");
    fs::create_dir_all(&roles_dir).unwrap();
    let role = kcd::models::RoleRepresentation {
        id: None,
        name: "excluded-role".to_string(),
        description: Some("Excluded role".to_string()),
        container_id: None,
        composite: false,
        client_role: false,
        extra: std::collections::HashMap::new(),
    };
    let role_path = roles_dir.join("excluded-role.yaml");
    fs::write(&role_path, serde_yaml::to_string(&role).unwrap()).unwrap();

    let ui = Arc::new(MockUi {
        inputs: Mutex::new(vec![]),
        confirms: Mutex::new(vec![false]), // Reject inclusion
        selects: Mutex::new(vec![]),
        passwords: Mutex::new(vec![]),
    });

    let res = plan::run(
        &client,
        workspace_dir.clone(),
        false,
        true, // interactive
        &["test-realm".to_string()],
        ui,
    )
    .await;
    assert!(res.is_ok());

    let plan_file = workspace_dir.join(".kcdplan");
    assert!(!plan_file.exists());
}

#[tokio::test]
async fn test_plan_error_paths() {
    let mock_url = start_mock_server().await;
    let mut client = KeycloakClient::new(mock_url);
    client.set_target_realm("error-realm".to_string());
    client.set_token("mock".to_string());

    let dir = tempdir().unwrap();
    let workspace_dir = dir.path().to_path_buf();
    let realm_dir = workspace_dir.join("error-realm");
    fs::create_dir_all(&realm_dir).unwrap();

    // 1. Realm fetch error (non-404)
    fs::write(
        realm_dir.join("realm.yaml"),
        "realm: error-realm\nenabled: true\n",
    )
    .unwrap();

    let res = plan::run(
        &client,
        workspace_dir.clone(),
        false,
        false,
        &["error-realm".to_string()],
        Arc::new(DialoguerUi),
    )
    .await;
    assert!(res.is_err());
    assert!(res.unwrap_err().to_string().contains("Failed to get realm"));

    // 2. Resource fetch error
    // Remove realm.yaml to avoid realm fetch error, but add roles directory
    fs::remove_file(realm_dir.join("realm.yaml")).unwrap();
    let roles_dir = realm_dir.join("roles");
    fs::create_dir_all(&roles_dir).unwrap();
    fs::write(roles_dir.join("role.yaml"), "name: role\n").unwrap();

    let res = plan::run(
        &client,
        workspace_dir.clone(),
        false,
        false,
        &["error-realm".to_string()],
        Arc::new(DialoguerUi),
    )
    .await;
    assert!(res.is_err());
    assert!(res.unwrap_err().to_string().contains("Failed to get roles"));

    // 3. Components fetch error
    fs::remove_dir_all(&roles_dir).unwrap();
    let comp_dir = realm_dir.join("components");
    fs::create_dir_all(&comp_dir).unwrap();
    fs::write(comp_dir.join("comp.yaml"), "name: comp\n").unwrap();

    let res = plan::run(
        &client,
        workspace_dir.clone(),
        false,
        false,
        &["error-realm".to_string()],
        Arc::new(DialoguerUi),
    )
    .await;
    assert!(res.is_err());
    assert!(res.unwrap_err().to_string().contains("Failed to get components"));
}

#[tokio::test]
async fn test_plan_empty_realms_list() {
    let mock_url = start_mock_server().await;
    let client = KeycloakClient::new(mock_url);
    let dir = tempdir().unwrap();
    let workspace_dir = dir.path().to_path_buf();
    
    // Create an empty directory (already empty)
    let res = plan::run(
        &client,
        workspace_dir,
        false,
        false,
        &[],
        Arc::new(DialoguerUi),
    )
    .await;
    assert!(res.is_ok());
}

#[test]
fn test_print_diff_delete() {
    use kcd::models::RoleRepresentation;
    use kcd::plan::print_diff;
    use std::collections::HashMap;

    let old = RoleRepresentation {
        id: Some("1".to_string()),
        name: "role".to_string(),
        description: Some("old".to_string()),
        container_id: None,
        composite: false,
        client_role: false,
        extra: HashMap::new(),
    };
    let new = RoleRepresentation {
        id: Some("1".to_string()),
        name: "role".to_string(),
        description: None, // Deleted field
        container_id: None,
        composite: false,
        client_role: false,
        extra: HashMap::new(),
    };

    let res = print_diff("test", Some(&old), &new, false, "role").unwrap();
    assert!(res);
}

#[tokio::test]
async fn test_plan_auto_discovery_no_realm_yaml() {
    let mock_url = start_mock_server().await;
    let client = KeycloakClient::new(mock_url);
    let dir = tempdir().unwrap();
    let workspace_dir = dir.path().to_path_buf();
    
    let realm_dir = workspace_dir.join("test-realm");
    fs::create_dir_all(&realm_dir).unwrap();
    // No realm.yaml, but maybe roles
    let roles_dir = realm_dir.join("roles");
    fs::create_dir_all(&roles_dir).unwrap();
    
    let res = plan::run(
        &client,
        workspace_dir,
        false,
        false,
        &[],
        Arc::new(DialoguerUi),
    )
    .await;
    assert!(res.is_ok());
}
