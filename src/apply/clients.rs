use crate::client::KeycloakClient;
use crate::models::{ClientRepresentation, KeycloakResource};
use crate::utils::secrets::substitute_secrets;
use crate::utils::ui::{SUCCESS_CREATE, SUCCESS_UPDATE};
use anyhow::{Context, Result};
use console::style;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs as async_fs;
use tokio::task::JoinSet;

pub async fn apply_clients(
    client: &KeycloakClient,
    workspace_dir: &std::path::Path,
    env_vars: Arc<HashMap<String, String>>,
    planned_files: Arc<Option<HashSet<PathBuf>>>,
    realm_name: &str,
) -> Result<()> {
    // 3. Apply Clients
    let clients_dir = workspace_dir.join("clients");
    if async_fs::try_exists(&clients_dir).await? {
        let existing_clients = client
            .get_clients()
            .await
            .with_context(|| format!("Failed to get clients for realm '{}'", realm_name))?;
        let existing_clients_map: HashMap<String, ClientRepresentation> = existing_clients
            .into_iter()
            .filter_map(|c| c.get_identity().map(|id| (id, c)))
            .collect();
        let existing_clients_map = std::sync::Arc::new(existing_clients_map);

        let mut entries = async_fs::read_dir(&clients_dir).await?;
        let mut set = JoinSet::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(plan) = &*planned_files
                && !plan.contains(&path)
            {
                continue;
            }
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let client = client.clone();
                let existing_clients_map = existing_clients_map.clone();
                let env_vars = Arc::clone(&env_vars);
                let realm_name = realm_name.to_string();
                set.spawn(async move {
                    let content = async_fs::read_to_string(&path).await?;
                    let mut val: serde_json::Value = serde_yaml::from_str(&content)
                        .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                    substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
                    let mut client_rep: ClientRepresentation = serde_json::from_value(val)?;

                    let identity = client_rep
                        .get_identity()
                        .context(format!("Failed to get identity for client in {:?}", path))?;

                    if let Some(existing) = existing_clients_map.get(&identity) {
                        if let Some(id) = &existing.id {
                            client_rep.id = Some(id.clone()); // Use remote ID
                            client
                                .update_client(id, &client_rep)
                                .await
                                .with_context(|| {
                                    format!(
                                        "Failed to update client '{}' in realm '{}'",
                                        client_rep.get_name(),
                                        realm_name
                                    )
                                })?;
                            println!(
                                "  {} {}",
                                SUCCESS_UPDATE,
                                style(format!("Updated client {}", client_rep.get_name())).cyan()
                            );
                        }
                    } else {
                        client_rep.id = None; // Don't send ID on create
                        client.create_client(&client_rep).await.with_context(|| {
                            format!(
                                "Failed to create client '{}' in realm '{}'",
                                client_rep.get_name(),
                                realm_name
                            )
                        })?;
                        println!(
                            "  {} {}",
                            SUCCESS_CREATE,
                            style(format!("Created client {}", client_rep.get_name())).green()
                        );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::KeycloakClient;
    use axum::{
        Json, Router,
        http::StatusCode,
        routing::{get, put},
    };
    use std::fs;
    use std::sync::Arc;
    use tempfile::tempdir;
    use tokio::net::TcpListener;

    async fn start_mock_server() -> Result<(String, Arc<std::sync::atomic::AtomicUsize>)> {
        let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count_clone = Arc::clone(&call_count);

        let app = Router::new()
            .route(
                "/admin/realms/test/clients",
                get(|| async {
                    Json(vec![ClientRepresentation {
                        id: Some("existing-id".to_string()),
                        client_id: Some("existing-client".to_string()),
                        name: Some("Existing Client".to_string()),
                        description: None,
                        enabled: Some(true),
                        protocol: None,
                        redirect_uris: None,
                        web_origins: None,
                        public_client: None,
                        bearer_only: None,
                        service_accounts_enabled: None,
                        extra: Default::default(),
                    }])
                })
                .post({
                    let count = Arc::clone(&count_clone);
                    move || {
                        let c = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        async move {
                            if c == 0 {
                                StatusCode::INTERNAL_SERVER_ERROR
                            } else {
                                StatusCode::CREATED
                            }
                        }
                    }
                }),
            )
            .route(
                "/admin/realms/test/clients/existing-id",
                put({
                    let count = Arc::clone(&count_clone);
                    move || {
                        let c = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        async move {
                            if c == 0 {
                                StatusCode::INTERNAL_SERVER_ERROR
                            } else {
                                StatusCode::OK
                            }
                        }
                    }
                }),
            );

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        Ok((format!("http://{}", addr), call_count))
    }

    #[tokio::test]
    async fn test_apply_clients_error_paths() -> Result<()> {
        let (server_url, call_count) = start_mock_server().await?;
        let mut client = KeycloakClient::new(server_url);
        client.set_target_realm("test".to_string());
        client.set_token("mock_token".to_string());

        let temp = tempdir()?;
        let clients_dir = temp.path().join("clients");
        fs::create_dir(&clients_dir)?;

        // 1. Test update failure
        call_count.store(0, std::sync::atomic::Ordering::SeqCst);
        let client_existing = clients_dir.join("existing.yaml");
        fs::write(
            client_existing,
            "clientId: existing-client\nname: Existing Client",
        )?;

        let res = apply_clients(
            &client,
            temp.path(),
            Arc::new(HashMap::new()),
            Arc::new(None),
            "test",
        )
        .await;
        assert!(res.is_err());
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("Failed to update client")
        );

        fs::remove_file(clients_dir.join("existing.yaml"))?;

        // 2. Test create failure
        call_count.store(0, std::sync::atomic::Ordering::SeqCst);
        let client_new = clients_dir.join("new.yaml");
        fs::write(client_new, "clientId: new-client\nname: New Client")?;

        let res = apply_clients(
            &client,
            temp.path(),
            Arc::new(HashMap::new()),
            Arc::new(None),
            "test",
        )
        .await;
        assert!(res.is_err());
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("Failed to create client")
        );

        // 3. Test invalid YAML
        let client_invalid = clients_dir.join("invalid.yaml");
        fs::write(client_invalid, "invalid: yaml: :")?;
        let res = apply_clients(
            &client,
            temp.path(),
            Arc::new(HashMap::new()),
            Arc::new(None),
            "test",
        )
        .await;
        assert!(res.is_err());
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("Failed to parse YAML file")
        );

        Ok(())
    }
}
