use crate::client::KeycloakClient;
use crate::models::{KeycloakResource, RoleRepresentation};
use crate::utils::secrets::substitute_secrets;
use anyhow::{Context, Result};
use console::style;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs as async_fs;
use tokio::task::JoinSet;

use super::{SUCCESS_CREATE, SUCCESS_UPDATE};

pub async fn apply_roles(
    client: &KeycloakClient,
    workspace_dir: &std::path::Path,
    env_vars: Arc<HashMap<String, String>>,
    planned_files: Arc<Option<HashSet<PathBuf>>>,
) -> Result<()> {
    // 2. Apply Roles
    let roles_dir = workspace_dir.join("roles");
    if async_fs::try_exists(&roles_dir).await? {
        let existing_roles = client.get_roles().await?;
        let existing_roles_map: HashMap<String, String> = existing_roles
            .into_iter()
            .filter_map(|r| {
                let identity = r.get_identity();
                let id = r.id.clone();
                match (identity, id) {
                    (Some(identity), Some(id)) => Some((identity, id)),
                    _ => None,
                }
            })
            .collect();
        let existing_roles_map = std::sync::Arc::new(existing_roles_map);

        let mut entries = async_fs::read_dir(&roles_dir).await?;
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
                let existing_roles_map = existing_roles_map.clone();
                let env_vars = Arc::clone(&env_vars);
                set.spawn(async move {
                    let content = async_fs::read_to_string(&path).await?;
                    let mut val: serde_json::Value = serde_yaml::from_str(&content)
                        .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                    substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
                    let mut role_rep: RoleRepresentation = serde_json::from_value(val)?;

                    let identity = role_rep
                        .get_identity()
                        .context(format!("Failed to get identity for role in {:?}", path))?;

                    if let Some(id) = existing_roles_map.get(&identity) {
                        role_rep.id = Some(id.clone()); // Use remote ID
                        client
                            .update_role(id, &role_rep)
                            .await
                            .context(format!("Failed to update role {}", role_rep.get_name()))?;
                        println!(
                            "  {} {}",
                            SUCCESS_UPDATE,
                            style(format!("Updated role {}", role_rep.get_name())).cyan()
                        );
                    } else {
                        role_rep.id = None; // Don't send ID on create
                        client
                            .create_role(&role_rep)
                            .await
                            .context(format!("Failed to create role {}", role_rep.get_name()))?;
                        println!(
                            "  {} {}",
                            SUCCESS_CREATE,
                            style(format!("Created role {}", role_rep.get_name())).green()
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

    async fn start_mock_server() -> (String, Arc<std::sync::atomic::AtomicUsize>) {
        let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count_clone = Arc::clone(&call_count);

        let app = Router::new()
            .route(
                "/admin/realms/test/roles",
                get(|| async {
                    Json(vec![RoleRepresentation {
                        id: Some("existing-id".to_string()),
                        name: "existing-role".to_string(),
                        description: None,
                        container_id: None,
                        composite: false,
                        client_role: false,
                        extra: Default::default(),
                    }])
                }),
            )
            .route(
                "/admin/realms/test/roles-by-id/existing-id",
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
                "/admin/realms/test/roles",
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

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (format!("http://{}", addr), call_count)
    }

    #[tokio::test]
    async fn test_apply_roles_error_paths() {
        let (server_url, call_count) = start_mock_server().await;
        let mut client = KeycloakClient::new(server_url);
        client.set_target_realm("test".to_string());
        client.set_token("mock_token".to_string());

        let temp = tempdir().unwrap();
        let roles_dir = temp.path().join("roles");
        fs::create_dir(&roles_dir).unwrap();

        // 1. Test update failure
        call_count.store(0, std::sync::atomic::Ordering::SeqCst);
        let role_existing = roles_dir.join("existing.yaml");
        fs::write(role_existing, "name: existing-role\nid: existing-id").unwrap();

        let res = apply_roles(
            &client,
            temp.path(),
            Arc::new(HashMap::new()),
            Arc::new(None),
        )
        .await;
        assert!(res.is_err());
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("Failed to update role")
        );

        fs::remove_file(roles_dir.join("existing.yaml")).unwrap();

        // 2. Test create failure
        call_count.store(0, std::sync::atomic::Ordering::SeqCst);
        let role_new = roles_dir.join("new.yaml");
        fs::write(role_new, "name: new-role").unwrap();

        let res = apply_roles(
            &client,
            temp.path(),
            Arc::new(HashMap::new()),
            Arc::new(None),
        )
        .await;
        assert!(res.is_err());
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("Failed to create role")
        );
    }
}
