use crate::client::KeycloakClient;
use crate::models::{RealmRepresentation, ClientRepresentation, RoleRepresentation, IdentityProviderRepresentation};
use anyhow::{Result, Context};
use std::path::PathBuf;
use tokio::fs as async_fs;
use tokio::task::JoinSet;
use std::collections::HashMap;

pub async fn run(client: &KeycloakClient, input_dir: PathBuf) -> Result<()> {
    // 1. Apply Realm
    let realm_path = input_dir.join("realm.yaml");
    if async_fs::try_exists(&realm_path).await? {
        let content = async_fs::read_to_string(&realm_path).await?;
        let realm_rep: RealmRepresentation = serde_yaml::from_str(&content)?;
        client.update_realm(&realm_rep).await.context("Failed to update realm")?;
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
                            client.update_role(id, &role_rep).await.context(format!("Failed to update role {}", role_rep.name))?;
                            println!("Updated role {}", role_rep.name);
                        }
                    } else {
                        role_rep.id = None; // Don't send ID on create
                        client.create_role(&role_rep).await.context(format!("Failed to create role {}", role_rep.name))?;
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
                    let mut idp_rep: IdentityProviderRepresentation = serde_yaml::from_str(&content)?;
                    let alias = idp_rep.alias.clone().unwrap_or_default();

                    if alias.is_empty() {
                        println!("Skipping IDP file {:?} due to missing alias", path);
                        return Ok::<(), anyhow::Error>(());
                    }

                    if let Some(existing) = existing_idps_map.get(&alias) {
                        if let Some(internal_id) = &existing.internal_id {
                            idp_rep.internal_id = Some(internal_id.clone());
                            client.update_identity_provider(&alias, &idp_rep).await.context(format!("Failed to update identity provider {}", alias))?;
                            println!("Updated identity provider {}", alias);
                        }
                    } else {
                        idp_rep.internal_id = None;
                        client.create_identity_provider(&idp_rep).await.context(format!("Failed to create identity provider {}", alias))?;
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
                            client.update_client(id, &client_rep).await.context(format!("Failed to update client {}", client_id))?;
                            println!("Updated client {}", client_id);
                        }
                    } else {
                        client_rep.id = None; // Don't send ID on create
                        client.create_client(&client_rep).await.context(format!("Failed to create client {}", client_id))?;
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

    Ok(())
}
