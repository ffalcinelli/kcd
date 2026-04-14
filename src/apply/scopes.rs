use crate::client::KeycloakClient;
use crate::models::{ClientScopeRepresentation, KeycloakResource};
use crate::utils::secrets::substitute_secrets;
use crate::utils::ui::{SUCCESS_CREATE, SUCCESS_UPDATE};
use anyhow::{Context, Result};
use console::style;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs as async_fs;
use tokio::task::JoinSet;

pub async fn apply_client_scopes(
    client: &KeycloakClient,
    workspace_dir: &std::path::Path,
    env_vars: Arc<HashMap<String, String>>,
    planned_files: Arc<Option<HashSet<PathBuf>>>,
    realm_name: &str,
) -> Result<()> {
    // 5. Apply Client Scopes
    let scopes_dir = workspace_dir.join("client-scopes");
    if async_fs::try_exists(&scopes_dir).await? {
        let existing_scopes = client
            .get_client_scopes()
            .await
            .with_context(|| format!("Failed to get client scopes for realm '{}'", realm_name))?;
        let existing_scopes_map: HashMap<String, ClientScopeRepresentation> = existing_scopes
            .into_iter()
            .filter_map(|s| s.get_identity().map(|id| (id, s)))
            .collect();
        let existing_scopes_map = Arc::new(existing_scopes_map);

        let mut entries = async_fs::read_dir(&scopes_dir).await?;
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
                let existing_scopes_map = Arc::clone(&existing_scopes_map);
                let env_vars = Arc::clone(&env_vars);
                let realm_name = realm_name.to_string();
                set.spawn(async move {
                    let content = async_fs::read_to_string(&path).await?;
                    let mut val: serde_json::Value = serde_yaml::from_str(&content)
                        .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                    substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
                    let mut scope_rep: ClientScopeRepresentation = serde_json::from_value(val)?;

                    let identity = scope_rep.get_identity().context(format!(
                        "Failed to get identity for client scope in {:?}",
                        path
                    ))?;

                    if let Some(existing) = existing_scopes_map.get(&identity) {
                        if let Some(id) = &existing.id {
                            scope_rep.id = Some(id.clone());
                            client
                                .update_client_scope(id, &scope_rep)
                                .await
                                .with_context(|| {
                                    format!(
                                        "Failed to update client scope '{}' in realm '{}'",
                                        scope_rep.get_name(),
                                        realm_name
                                    )
                                })?;
                            println!(
                                "  {} {}",
                                SUCCESS_UPDATE,
                                style(format!("Updated client scope {}", scope_rep.get_name()))
                                    .cyan()
                            );
                        }
                    } else {
                        scope_rep.id = None;
                        client
                            .create_client_scope(&scope_rep)
                            .await
                            .with_context(|| {
                                format!(
                                    "Failed to create client scope '{}' in realm '{}'",
                                    scope_rep.get_name(),
                                    realm_name
                                )
                            })?;
                        println!(
                            "  {} {}",
                            SUCCESS_CREATE,
                            style(format!("Created client scope {}", scope_rep.get_name())).green()
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
        routing::{get, post, put},
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
                "/admin/realms/test/client-scopes",
                get(|| async {
                    Json(vec![ClientScopeRepresentation {
                        id: Some("existing-id".to_string()),
                        name: Some("existing-scope".to_string()),
                        description: None,
                        protocol: None,
                        attributes: None,
                        extra: Default::default(),
                    }])
                }),
            )
            .route(
                "/admin/realms/test/client-scopes/existing-id",
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
            )
            .route(
                "/admin/realms/test/client-scopes",
                post({
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
            );

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        Ok((format!("http://{}", addr), call_count))
    }

    #[tokio::test]
    async fn test_apply_client_scopes_error_paths() -> Result<()> {
        let (server_url, call_count) = start_mock_server().await?;
        let mut client = KeycloakClient::new(server_url);
        client.set_target_realm("test".to_string());
        client.set_token("mock_token".to_string());

        let temp = tempdir()?;
        let scopes_dir = temp.path().join("client-scopes");
        fs::create_dir(&scopes_dir)?;

        // 1. Test update failure
        call_count.store(0, std::sync::atomic::Ordering::SeqCst);
        let scope_existing = scopes_dir.join("existing.yaml");
        fs::write(scope_existing, "name: existing-scope\nid: existing-id")?;

        let res = apply_client_scopes(
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
                .contains("Failed to update client scope")
        );

        fs::remove_file(scopes_dir.join("existing.yaml"))?;

        // 2. Test create failure
        call_count.store(0, std::sync::atomic::Ordering::SeqCst);
        let scope_new = scopes_dir.join("new.yaml");
        fs::write(scope_new, "name: new-scope")?;

        let res = apply_client_scopes(
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
                .contains("Failed to create client scope")
        );

        Ok(())
    }
}
