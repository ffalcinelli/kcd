use crate::client::KeycloakClient;
use crate::models::{
    AuthenticationFlowRepresentation, ClientRepresentation, ClientScopeRepresentation,
    ComponentRepresentation, GroupRepresentation, IdentityProviderRepresentation,
    RealmRepresentation, RequiredActionProviderRepresentation, RoleRepresentation,
    UserRepresentation,
};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs as async_fs;
use tokio::task::JoinSet;

pub async fn run(client: &KeycloakClient, input_dir: PathBuf) -> Result<()> {
    // 1. Apply Realm
    let realm_path = input_dir.join("realm.yaml");
    if async_fs::try_exists(&realm_path).await? {
        let content = async_fs::read_to_string(&realm_path).await?;
        let realm_rep: RealmRepresentation = serde_yaml::from_str(&content)?;
        client
            .update_realm(&realm_rep)
            .await
            .context("Failed to update realm")?;
        println!("Updated realm configuration");
    }

    // 2. Apply Roles
    let roles_dir = input_dir.join("roles");
    if async_fs::try_exists(&roles_dir).await? {
        let existing_roles = client.get_roles().await?;
        let existing_roles_map: HashMap<String, RoleRepresentation> = existing_roles
            .into_iter()
            .map(|r| (r.name.clone(), r))
            .collect();
        let existing_roles_map = std::sync::Arc::new(existing_roles_map);

        let mut entries = async_fs::read_dir(&roles_dir).await?;
        let mut set = JoinSet::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "yaml") {
                let client = client.clone();
                let existing_roles_map = existing_roles_map.clone();
                set.spawn(async move {
                    let content = async_fs::read_to_string(&path).await?;
                    let mut role_rep: RoleRepresentation = serde_yaml::from_str(&content)?;

                    if let Some(existing) = existing_roles_map.get(&role_rep.name) {
                        if let Some(id) = &existing.id {
                            role_rep.id = Some(id.clone()); // Use remote ID
                            client
                                .update_role(id, &role_rep)
                                .await
                                .context(format!("Failed to update role {}", role_rep.name))?;
                            println!("Updated role {}", role_rep.name);
                        }
                    } else {
                        role_rep.id = None; // Don't send ID on create
                        client
                            .create_role(&role_rep)
                            .await
                            .context(format!("Failed to create role {}", role_rep.name))?;
                        println!("Created role {}", role_rep.name);
                    }
                    Ok::<(), anyhow::Error>(())
                });
            }
        }
        while let Some(res) = set.join_next().await {
            res??;
        }
    }

    // 4. Apply Identity Providers
    let idps_dir = input_dir.join("identity-providers");
    if async_fs::try_exists(&idps_dir).await? {
        let existing_idps = client.get_identity_providers().await?;
        let existing_idps_map: HashMap<String, IdentityProviderRepresentation> = existing_idps
            .into_iter()
            .filter_map(|i| i.alias.clone().map(|alias| (alias, i)))
            .collect();
        let existing_idps_map = std::sync::Arc::new(existing_idps_map);

        let mut entries = async_fs::read_dir(&idps_dir).await?;
        let mut set = JoinSet::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "yaml") {
                let client = client.clone();
                let existing_idps_map = existing_idps_map.clone();
                set.spawn(async move {
                    let content = async_fs::read_to_string(&path).await?;
                    let mut idp_rep: IdentityProviderRepresentation =
                        serde_yaml::from_str(&content)?;
                    let alias = idp_rep.alias.clone().unwrap_or_default();

                    if alias.is_empty() {
                        println!("Skipping IDP file {:?} due to missing alias", path);
                        return Ok::<(), anyhow::Error>(());
                    }

                    if let Some(existing) = existing_idps_map.get(&alias) {
                        if let Some(internal_id) = &existing.internal_id {
                            idp_rep.internal_id = Some(internal_id.clone());
                            client
                                .update_identity_provider(&alias, &idp_rep)
                                .await
                                .context(format!("Failed to update identity provider {}", alias))?;
                            println!("Updated identity provider {}", alias);
                        }
                    } else {
                        idp_rep.internal_id = None;
                        client
                            .create_identity_provider(&idp_rep)
                            .await
                            .context(format!("Failed to create identity provider {}", alias))?;
                        println!("Created identity provider {}", alias);
                    }
                    Ok::<(), anyhow::Error>(())
                });
            }
        }
        while let Some(res) = set.join_next().await {
            res??;
        }
    }

    // 3. Apply Clients
    let clients_dir = input_dir.join("clients");
    if async_fs::try_exists(&clients_dir).await? {
        let existing_clients = client.get_clients().await?;
        let existing_clients_map: HashMap<String, ClientRepresentation> = existing_clients
            .into_iter()
            .filter_map(|c| c.client_id.clone().map(|id| (id, c)))
            .collect();
        let existing_clients_map = std::sync::Arc::new(existing_clients_map);

        let mut entries = async_fs::read_dir(&clients_dir).await?;
        let mut set = JoinSet::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "yaml") {
                let client = client.clone();
                let existing_clients_map = existing_clients_map.clone();
                set.spawn(async move {
                    let content = async_fs::read_to_string(&path).await?;
                    let mut client_rep: ClientRepresentation = serde_yaml::from_str(&content)?;
                    let client_id = client_rep.client_id.clone().unwrap_or_default();

                    if client_id.is_empty() {
                        println!("Skipping client file {:?} due to missing clientId", path);
                        return Ok::<(), anyhow::Error>(());
                    }

                    if let Some(existing) = existing_clients_map.get(&client_id) {
                        if let Some(id) = &existing.id {
                            client_rep.id = Some(id.clone()); // Use remote ID
                            client
                                .update_client(id, &client_rep)
                                .await
                                .context(format!("Failed to update client {}", client_id))?;
                            println!("Updated client {}", client_id);
                        }
                    } else {
                        client_rep.id = None; // Don't send ID on create
                        client
                            .create_client(&client_rep)
                            .await
                            .context(format!("Failed to create client {}", client_id))?;
                        println!("Created client {}", client_id);
                    }
                    Ok::<(), anyhow::Error>(())
                });
            }
        }
        while let Some(res) = set.join_next().await {
            res??;
        }
    }

    // 5. Apply Client Scopes
    let scopes_dir = input_dir.join("client-scopes");
    if async_fs::try_exists(&scopes_dir).await? {
        let existing_scopes = client.get_client_scopes().await?;
        let existing_scopes_map: HashMap<String, ClientScopeRepresentation> = existing_scopes
            .into_iter()
            .filter_map(|s| s.name.clone().map(|n| (n, s)))
            .collect();

        let mut entries = async_fs::read_dir(&scopes_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "yaml") {
                let content = async_fs::read_to_string(&path).await?;
                let mut scope_rep: ClientScopeRepresentation = serde_yaml::from_str(&content)?;
                let name = scope_rep.name.as_deref().unwrap_or("");

                if name.is_empty() {
                    continue;
                }

                if let Some(existing) = existing_scopes_map.get(name) {
                    if let Some(id) = &existing.id {
                        scope_rep.id = Some(id.clone());
                        client
                            .update_client_scope(id, &scope_rep)
                            .await
                            .context(format!("Failed to update client scope {}", name))?;
                        println!("Updated client scope {}", name);
                    }
                } else {
                    scope_rep.id = None;
                    client
                        .create_client_scope(&scope_rep)
                        .await
                        .context(format!("Failed to create client scope {}", name))?;
                    println!("Created client scope {}", name);
                }
            }
        }
    }

    // 6. Apply Groups
    let groups_dir = input_dir.join("groups");
    if async_fs::try_exists(&groups_dir).await? {
        let existing_groups = client.get_groups().await?;
        let existing_groups_map: HashMap<String, GroupRepresentation> = existing_groups
            .into_iter()
            .filter_map(|g| g.name.clone().map(|n| (n, g)))
            .collect();

        let mut entries = async_fs::read_dir(&groups_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "yaml") {
                let content = async_fs::read_to_string(&path).await?;
                let mut group_rep: GroupRepresentation = serde_yaml::from_str(&content)?;
                let name = group_rep.name.as_deref().unwrap_or("");

                if name.is_empty() {
                    continue;
                }

                if let Some(existing) = existing_groups_map.get(name) {
                    if let Some(id) = &existing.id {
                        group_rep.id = Some(id.clone());
                        client
                            .update_group(id, &group_rep)
                            .await
                            .context(format!("Failed to update group {}", name))?;
                        println!("Updated group {}", name);
                    }
                } else {
                    group_rep.id = None;
                    client
                        .create_group(&group_rep)
                        .await
                        .context(format!("Failed to create group {}", name))?;
                    println!("Created group {}", name);
                }
            }
        }
    }

    // 7. Apply Users
    let users_dir = input_dir.join("users");
    if async_fs::try_exists(&users_dir).await? {
        let existing_users = client.get_users().await?;
        let existing_users_map: HashMap<String, UserRepresentation> = existing_users
            .into_iter()
            .filter_map(|u| u.username.clone().map(|n| (n, u)))
            .collect();

        let mut entries = async_fs::read_dir(&users_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "yaml") {
                let content = async_fs::read_to_string(&path).await?;
                let mut user_rep: UserRepresentation = serde_yaml::from_str(&content)?;
                let username = user_rep.username.as_deref().unwrap_or("");

                if username.is_empty() {
                    continue;
                }

                if let Some(existing) = existing_users_map.get(username) {
                    if let Some(id) = &existing.id {
                        user_rep.id = Some(id.clone());
                        client
                            .update_user(id, &user_rep)
                            .await
                            .context(format!("Failed to update user {}", username))?;
                        println!("Updated user {}", username);
                    }
                } else {
                    user_rep.id = None;
                    client
                        .create_user(&user_rep)
                        .await
                        .context(format!("Failed to create user {}", username))?;
                    println!("Created user {}", username);
                }
            }
        }
    }

    // 8. Apply Authentication Flows
    let flows_dir = input_dir.join("authentication-flows");
    if async_fs::try_exists(&flows_dir).await? {
        let existing_flows = client.get_authentication_flows().await?;
        let existing_flows_map: HashMap<String, AuthenticationFlowRepresentation> = existing_flows
            .into_iter()
            .filter_map(|f| f.alias.clone().map(|a| (a, f)))
            .collect();

        let mut entries = async_fs::read_dir(&flows_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "yaml") {
                let content = async_fs::read_to_string(&path).await?;
                let mut flow_rep: AuthenticationFlowRepresentation =
                    serde_yaml::from_str(&content)?;
                let alias = flow_rep.alias.as_deref().unwrap_or("");

                if alias.is_empty() {
                    continue;
                }

                if let Some(existing) = existing_flows_map.get(alias) {
                    if let Some(id) = &existing.id {
                        flow_rep.id = Some(id.clone());
                        client
                            .update_authentication_flow(id, &flow_rep)
                            .await
                            .context(format!("Failed to update authentication flow {}", alias))?;
                        println!("Updated authentication flow {}", alias);
                    }
                } else {
                    flow_rep.id = None;
                    client
                        .create_authentication_flow(&flow_rep)
                        .await
                        .context(format!("Failed to create authentication flow {}", alias))?;
                    println!("Created authentication flow {}", alias);
                }
            }
        }
    }

    // 9. Apply Required Actions
    let actions_dir = input_dir.join("required-actions");
    if async_fs::try_exists(&actions_dir).await? {
        let existing_actions = client.get_required_actions().await?;
        let existing_actions_map: HashMap<String, RequiredActionProviderRepresentation> =
            existing_actions
                .into_iter()
                .filter_map(|a| a.alias.clone().map(|n| (n, a)))
                .collect();

        let mut entries = async_fs::read_dir(&actions_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "yaml") {
                let content = async_fs::read_to_string(&path).await?;
                let action_rep: RequiredActionProviderRepresentation =
                    serde_yaml::from_str(&content)?;
                let alias = action_rep.alias.as_deref().unwrap_or("");

                if alias.is_empty() {
                    continue;
                }

                if existing_actions_map.contains_key(alias) {
                    client
                        .update_required_action(alias, &action_rep)
                        .await
                        .context(format!("Failed to update required action {}", alias))?;
                    println!("Updated required action {}", alias);
                } else {
                    // Register
                    client
                        .register_required_action(&action_rep)
                        .await
                        .context(format!("Failed to register required action {}", alias))?;
                    client
                        .update_required_action(alias, &action_rep)
                        .await
                        .context(format!(
                            "Failed to configure registered required action {}",
                            alias
                        ))?;
                    println!("Registered required action {}", alias);
                }
            }
        }
    }

    // 10. Apply Components
    let components_dir = input_dir.join("components");
    if async_fs::try_exists(&components_dir).await? {
        let existing_components = client.get_components().await?;
        let existing_components_map: HashMap<String, ComponentRepresentation> = existing_components
            .into_iter()
            .filter_map(|c| c.name.clone().map(|n| (n, c)))
            .collect();

        let mut entries = async_fs::read_dir(&components_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "yaml") {
                let content = async_fs::read_to_string(&path).await?;
                let mut component_rep: ComponentRepresentation = serde_yaml::from_str(&content)?;
                let name = component_rep.name.as_deref().unwrap_or("");

                if name.is_empty() {
                    continue;
                }

                if let Some(existing) = existing_components_map.get(name) {
                    if let Some(id) = &existing.id {
                        component_rep.id = Some(id.clone());
                        client
                            .update_component(id, &component_rep)
                            .await
                            .context(format!("Failed to update component {}", name))?;
                        println!("Updated component {}", name);
                    }
                } else {
                    component_rep.id = None;
                    client
                        .create_component(&component_rep)
                        .await
                        .context(format!("Failed to create component {}", name))?;
                    println!("Created component {}", name);
                }
            }
        }
    }

    Ok(())
}
