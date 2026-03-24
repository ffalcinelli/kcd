use crate::client::KeycloakClient;
use crate::models::{KeycloakResource, RequiredActionProviderRepresentation};
use crate::utils::secrets::substitute_secrets;
use anyhow::{Context, Result};
use console::style;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs as async_fs;
use tokio::task::JoinSet;

use super::{SUCCESS_CREATE, SUCCESS_UPDATE};

pub async fn apply_required_actions(
    client: &KeycloakClient,
    workspace_dir: &std::path::Path,
    env_vars: Arc<HashMap<String, String>>,
    planned_files: Arc<Option<HashSet<PathBuf>>>,
) -> Result<()> {
    // 9. Apply Required Actions
    let actions_dir = workspace_dir.join("required-actions");
    if async_fs::try_exists(&actions_dir).await? {
        let existing_actions = client.get_required_actions().await?;
        let existing_actions_map: HashMap<String, RequiredActionProviderRepresentation> =
            existing_actions
                .into_iter()
                .filter_map(|a| a.get_identity().map(|id| (id, a)))
                .collect();
        let existing_actions_map = Arc::new(existing_actions_map);

        let mut entries = async_fs::read_dir(&actions_dir).await?;
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
                let existing_actions_map = Arc::clone(&existing_actions_map);
                let env_vars = Arc::clone(&env_vars);
                set.spawn(async move {
                    let content = async_fs::read_to_string(&path).await?;
                    let mut val: serde_json::Value = serde_yaml::from_str(&content)
                        .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                    substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
                    let action_rep: RequiredActionProviderRepresentation =
                        serde_json::from_value(val)?;

                    let identity = action_rep.get_identity().context(format!(
                        "Failed to get identity for required action in {:?}",
                        path
                    ))?;

                    if existing_actions_map.contains_key(&identity) {
                        client
                            .update_required_action(&identity, &action_rep)
                            .await
                            .context(format!(
                                "Failed to update required action {}",
                                action_rep.get_name()
                            ))?;
                        println!(
                            "  {} {}",
                            SUCCESS_UPDATE,
                            style(format!("Updated required action {}", action_rep.get_name()))
                                .cyan()
                        );
                    } else {
                        // Register
                        client
                            .register_required_action(&action_rep)
                            .await
                            .context(format!(
                                "Failed to register required action {}",
                                action_rep.get_name()
                            ))?;
                        client
                            .update_required_action(&identity, &action_rep)
                            .await
                            .context(format!(
                                "Failed to configure registered required action {}",
                                action_rep.get_name()
                            ))?;
                        println!(
                            "  {} {}",
                            SUCCESS_CREATE,
                            style(format!(
                                "Registered required action {}",
                                action_rep.get_name()
                            ))
                            .green()
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
                "/admin/realms/test/authentication/required-actions",
                get(|| async {
                    Json(vec![RequiredActionProviderRepresentation {
                        alias: Some("existing-action".to_string()),
                        name: Some("Existing Action".to_string()),
                        provider_id: Some("existing-provider".to_string()),
                        enabled: Some(true),
                        default_action: Some(false),
                        priority: Some(0),
                        config: None,
                        extra: Default::default(),
                    }])
                }),
            )
            .route(
                "/admin/realms/test/authentication/required-actions/existing-action",
                put({
                    let count = Arc::clone(&count_clone);
                    move || {
                        count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        async { StatusCode::INTERNAL_SERVER_ERROR }
                    }
                }),
            )
            .route(
                "/admin/realms/test/authentication/register-required-action",
                post({
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
                "/admin/realms/test/authentication/required-actions/new-action",
                put({
                    let count = Arc::clone(&count_clone);
                    move || {
                        count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        async { StatusCode::INTERNAL_SERVER_ERROR }
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
    async fn test_apply_required_actions_error_paths() {
        let (server_url, call_count) = start_mock_server().await;
        let mut client = KeycloakClient::new(server_url);
        client.set_target_realm("test".to_string());
        client.set_token("mock_token".to_string());

        let temp = tempdir().unwrap();
        let actions_dir = temp.path().join("required-actions");
        fs::create_dir(&actions_dir).unwrap();

        // 1. Test missing identity (alias missing)
        let action_no_alias = actions_dir.join("no_alias.yaml");
        fs::write(action_no_alias, "name: No Alias\nproviderId: some-provider").unwrap();

        let res = apply_required_actions(
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
                .contains("Failed to get identity")
        );

        fs::remove_file(actions_dir.join("no_alias.yaml")).unwrap();

        // 2. Test update failure
        call_count.store(0, std::sync::atomic::Ordering::SeqCst);
        let action_existing = actions_dir.join("existing.yaml");
        fs::write(
            action_existing,
            "alias: existing-action\nname: Existing Action\nproviderId: existing-provider",
        )
        .unwrap();

        let res = apply_required_actions(
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
                .contains("Failed to update required action")
        );

        fs::remove_file(actions_dir.join("existing.yaml")).unwrap();

        // 3. Test register failure
        call_count.store(0, std::sync::atomic::Ordering::SeqCst);
        let action_new = actions_dir.join("new.yaml");
        fs::write(
            action_new,
            "alias: new-action\nname: New Action\nproviderId: new-provider",
        )
        .unwrap();

        let res = apply_required_actions(
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
                .contains("Failed to register required action")
        );

        // 4. Test update after register failure
        // The mock server is set up to succeed on second register call but fail on update
        // We just called it once, so next call to register-required-action will succeed (c will be 1)
        let res = apply_required_actions(
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
                .contains("Failed to configure registered required action")
        );
    }
}
