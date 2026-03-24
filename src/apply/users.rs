use crate::client::KeycloakClient;
use crate::models::{KeycloakResource, UserRepresentation};
use crate::utils::secrets::substitute_secrets;
use anyhow::{Context, Result};
use console::style;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs as async_fs;
use tokio::task::JoinSet;

use super::{SUCCESS_CREATE, SUCCESS_UPDATE};

pub async fn apply_users(
    client: &KeycloakClient,
    workspace_dir: &std::path::Path,
    env_vars: Arc<HashMap<String, String>>,
    planned_files: Arc<Option<HashSet<PathBuf>>>,
) -> Result<()> {
    // 7. Apply Users
    let users_dir = workspace_dir.join("users");
    if async_fs::try_exists(&users_dir).await? {
        let existing_users = client.get_users().await?;
        let existing_users_map: HashMap<String, UserRepresentation> = existing_users
            .into_iter()
            .filter_map(|u| u.get_identity().map(|id| (id, u)))
            .collect();
        let existing_users_map = Arc::new(existing_users_map);

        let mut entries = async_fs::read_dir(&users_dir).await?;
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
                let existing_users_map = Arc::clone(&existing_users_map);
                let env_vars = Arc::clone(&env_vars);
                set.spawn(async move {
                    let content = async_fs::read_to_string(&path).await?;
                    let mut val: serde_json::Value = serde_yaml::from_str(&content)
                        .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                    substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
                    let mut user_rep: UserRepresentation = serde_json::from_value(val)?;

                    let identity = user_rep
                        .get_identity()
                        .context(format!("Failed to get identity for user in {:?}", path))?;

                    if let Some(existing) = existing_users_map.get(&identity) {
                        if let Some(id) = &existing.id {
                            user_rep.id = Some(id.clone());
                            client.update_user(id, &user_rep).await.context(format!(
                                "Failed to update user {}",
                                user_rep.get_name()
                            ))?;
                            println!(
                                "  {} {}",
                                SUCCESS_UPDATE,
                                style(format!("Updated user {}", user_rep.get_name())).cyan()
                            );
                        }
                    } else {
                        user_rep.id = None;
                        client
                            .create_user(&user_rep)
                            .await
                            .context(format!("Failed to create user {}", user_rep.get_name()))?;
                        println!(
                            "  {} {}",
                            SUCCESS_CREATE,
                            style(format!("Created user {}", user_rep.get_name())).green()
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
                "/admin/realms/test/users",
                get(|| async {
                    Json(vec![UserRepresentation {
                        id: Some("existing-id".to_string()),
                        username: Some("existing-user".to_string()),
                        enabled: Some(true),
                        first_name: None,
                        last_name: None,
                        email: None,
                        email_verified: None,
                        credentials: None,
                        extra: Default::default(),
                    }])
                }),
            )
            .route(
                "/admin/realms/test/users/existing-id",
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
                "/admin/realms/test/users",
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
    async fn test_apply_users_error_paths() {
        let (server_url, call_count) = start_mock_server().await;
        let mut client = KeycloakClient::new(server_url);
        client.set_target_realm("test".to_string());
        client.set_token("mock_token".to_string());

        let temp = tempdir().unwrap();
        let users_dir = temp.path().join("users");
        fs::create_dir(&users_dir).unwrap();

        // 1. Test update failure
        call_count.store(0, std::sync::atomic::Ordering::SeqCst);
        let user_existing = users_dir.join("existing.yaml");
        fs::write(user_existing, "username: existing-user\nid: existing-id").unwrap();

        let res = apply_users(
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
                .contains("Failed to update user")
        );

        fs::remove_file(users_dir.join("existing.yaml")).unwrap();

        // 2. Test create failure
        call_count.store(0, std::sync::atomic::Ordering::SeqCst);
        let user_new = users_dir.join("new.yaml");
        fs::write(user_new, "username: new-user").unwrap();

        let res = apply_users(
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
                .contains("Failed to create user")
        );
    }
}
