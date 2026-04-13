use std::sync::Arc;
mod common;
use common::start_mock_server;
use kcd::client::KeycloakClient;
use kcd::{apply, plan};
use kcd::utils::ui::DialoguerUi;
use std::fs;
use tempfile::tempdir;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_ultimate_coverage() {
    let mock_url = start_mock_server().await;
    let mut client = KeycloakClient::new(mock_url);
    client.set_target_realm("test-realm".to_string());
    client.set_token("mock_token".to_string());

    let dir = tempdir().unwrap();
    let workspace_dir = dir.path().to_path_buf();
    let realm_dir = workspace_dir.join("test-realm");
    fs::create_dir_all(&realm_dir).unwrap();

    // 1. Realm update
    fs::write(
        realm_dir.join("realm.yaml"),
        "realm: test-realm\nenabled: true\ndisplayName: 'New Name'\n",
    )
    .unwrap();

    // 2. Clients (Create, Update, Delete)
    let clients_dir = realm_dir.join("clients");
    fs::create_dir(&clients_dir).unwrap();
    // new-client (Create)
    fs::write(
        clients_dir.join("new-client.yaml"),
        "clientId: new-client\nenabled: true\n",
    )
    .unwrap();
    // client-1 (Update)
    fs::write(
        clients_dir.join("client-1.yaml"),
        "clientId: client-1\nname: 'Updated Client 1'\nenabled: true\n",
    )
    .unwrap();
    // client-2 is on mock but not here (Delete)

    // 3. Roles
    let roles_dir = realm_dir.join("roles");
    fs::create_dir(&roles_dir).unwrap();
    fs::write(
        roles_dir.join("role-1.yaml"),
        "name: role-1\ndescription: 'Updated Role 1'\n",
    )
    .unwrap();

    // 4. Groups
    let groups_dir = realm_dir.join("groups");
    fs::create_dir(&groups_dir).unwrap();
    fs::write(
        groups_dir.join("group-1.yaml"),
        "name: group-1\npath: /group-1\n",
    )
    .unwrap();

    // 5. Users
    let users_dir = realm_dir.join("users");
    fs::create_dir(&users_dir).unwrap();
    fs::write(
        users_dir.join("user-1.yaml"),
        "username: user-1\nenabled: true\nemail: 'user1@example.com'\n",
    )
    .unwrap();

    // 6. IDPs
    let idps_dir = realm_dir.join("identity-providers");
    fs::create_dir(&idps_dir).unwrap();
    fs::write(
        idps_dir.join("google.yaml"),
        "alias: google\nproviderId: google\nenabled: true\nconfig:\n  clientId: 'abc'\n",
    )
    .unwrap();

    // 7. Client Scopes
    let scopes_dir = realm_dir.join("client-scopes");
    fs::create_dir(&scopes_dir).unwrap();
    fs::write(
        scopes_dir.join("scope-1.yaml"),
        "name: scope-1\nprotocol: openid-connect\n",
    )
    .unwrap();

    // 8. Auth Flows
    let flows_dir = realm_dir.join("authentication-flows");
    fs::create_dir(&flows_dir).unwrap();
    fs::write(
        flows_dir.join("flow-1.yaml"),
        "alias: flow-1\nproviderId: basic-flow\ntopLevel: true\n",
    )
    .unwrap();

    // 9. Required Actions
    let actions_dir = realm_dir.join("required-actions");
    fs::create_dir(&actions_dir).unwrap();
    fs::write(
        actions_dir.join("action-1.yaml"),
        "alias: action-1\nname: 'Updated Action 1'\nproviderId: action-provider\nenabled: true\n",
    )
    .unwrap();

    // 10. Components
    let components_dir = realm_dir.join("components");
    fs::create_dir(&components_dir).unwrap();
    fs::write(components_dir.join("component-1.yaml"), "name: component-1\nproviderId: ldap\nproviderType: org.keycloak.storage.UserStorageProvider\nconfig:\n  priority: ['1']\n").unwrap();

    // Run Plan
    plan::run(
        &client,
        workspace_dir.clone(),
        false,
        false,
        &["test-realm".to_string()],
        Arc::new(DialoguerUi),
    )
    .await
    .unwrap();

    // Run Apply
    apply::run(
        &client,
        workspace_dir.clone(),
        &["test-realm".to_string()],
        true,
    )
    .await
    .unwrap();

    // Run Drift
    plan::run(
        &client,
        workspace_dir.clone(),
        true,
        false,
        &["test-realm".to_string()],
        Arc::new(DialoguerUi),
    )
    .await
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_plan_all_realms() {
    let mock_url = start_mock_server().await;
    let mut client = KeycloakClient::new(mock_url);
    client.set_token("mock_token".to_string());

    let dir = tempdir().unwrap();
    let workspace_dir = dir.path().to_path_buf();
    fs::create_dir(workspace_dir.join("master")).unwrap();
    fs::create_dir(workspace_dir.join("test-realm")).unwrap();

    // Test auto-discovery of realms in plan
    plan::run(
        &client,
        workspace_dir.clone(),
        false,
        false,
        &[],
        Arc::new(DialoguerUi),
    )
    .await
    .unwrap();
}
