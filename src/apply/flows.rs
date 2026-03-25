use crate::client::KeycloakClient;
use crate::models::{AuthenticationFlowRepresentation, KeycloakResource};
use crate::utils::secrets::substitute_secrets;
use anyhow::{Context, Result};
use console::style;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs as async_fs;
use tokio::task::JoinSet;

use super::{SUCCESS_CREATE, SUCCESS_UPDATE};

pub async fn apply_authentication_flows(
    client: &KeycloakClient,
    workspace_dir: &std::path::Path,
    env_vars: Arc<HashMap<String, String>>,
    planned_files: Arc<Option<HashSet<PathBuf>>>,
    realm_name: &str,
) -> Result<()> {
    // 8. Apply Authentication Flows
    let flows_dir = workspace_dir.join("authentication-flows");
    if async_fs::try_exists(&flows_dir).await? {
        let existing_flows = client.get_authentication_flows().await.with_context(|| {
            format!(
                "Failed to get authentication flows for realm '{}'",
                realm_name
            )
        })?;
        let existing_flows_map: HashMap<String, AuthenticationFlowRepresentation> = existing_flows
            .into_iter()
            .filter_map(|f| f.get_identity().map(|id| (id, f)))
            .collect();
        let existing_flows_map = Arc::new(existing_flows_map);

        let mut entries = async_fs::read_dir(&flows_dir).await?;
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
                let existing_flows_map = Arc::clone(&existing_flows_map);
                let env_vars = Arc::clone(&env_vars);
                let realm_name = realm_name.to_string();
                set.spawn(async move {
                    let content = async_fs::read_to_string(&path).await?;
                    let mut val: serde_json::Value = serde_yaml::from_str(&content)
                        .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                    substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
                    let mut flow_rep: AuthenticationFlowRepresentation =
                        serde_json::from_value(val)?;

                    let identity = flow_rep
                        .get_identity()
                        .context(format!("Failed to get identity for flow in {:?}", path))?;

                    if let Some(existing) = existing_flows_map.get(&identity) {
                        if let Some(id) = &existing.id {
                            flow_rep.id = Some(id.clone());
                            client
                                .update_authentication_flow(id, &flow_rep)
                                .await
                                .with_context(|| {
                                    format!(
                                        "Failed to update authentication flow '{}' in realm '{}'",
                                        flow_rep.get_name(),
                                        realm_name
                                    )
                                })?;
                            println!(
                                "  {} {}",
                                SUCCESS_UPDATE,
                                style(format!(
                                    "Updated authentication flow {}",
                                    flow_rep.get_name()
                                ))
                                .cyan()
                            );
                        }
                    } else {
                        flow_rep.id = None;
                        client
                            .create_authentication_flow(&flow_rep)
                            .await
                            .with_context(|| {
                                format!(
                                    "Failed to create authentication flow '{}' in realm '{}'",
                                    flow_rep.get_name(),
                                    realm_name
                                )
                            })?;
                        println!(
                            "  {} {}",
                            SUCCESS_CREATE,
                            style(format!(
                                "Created authentication flow {}",
                                flow_rep.get_name()
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
                "/admin/realms/test/authentication/flows",
                get(|| async {
                    Json(vec![AuthenticationFlowRepresentation {
                        id: Some("existing-id".to_string()),
                        alias: Some("existing-flow".to_string()),
                        description: Some("Existing Flow".to_string()),
                        provider_id: None,
                        top_level: Some(true),
                        built_in: Some(false),
                        authentication_executions: None,
                        extra: Default::default(),
                    }])
                }),
            )
            .route(
                "/admin/realms/test/authentication/flows/existing-id",
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
                "/admin/realms/test/authentication/flows",
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
    async fn test_apply_authentication_flows_error_paths() {
        let (server_url, call_count) = start_mock_server().await;
        let mut client = KeycloakClient::new(server_url);
        client.set_target_realm("test".to_string());
        client.set_token("mock_token".to_string());

        let temp = tempdir().unwrap();
        let flows_dir = temp.path().join("authentication-flows");
        fs::create_dir(&flows_dir).unwrap();

        // 1. Test update failure
        call_count.store(0, std::sync::atomic::Ordering::SeqCst);
        let flow_existing = flows_dir.join("existing.yaml");
        fs::write(flow_existing, "alias: existing-flow\nid: existing-id").unwrap();

        let res = apply_authentication_flows(
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
                .contains("Failed to update authentication flow")
        );

        fs::remove_file(flows_dir.join("existing.yaml")).unwrap();

        // 2. Test create failure
        call_count.store(0, std::sync::atomic::Ordering::SeqCst);
        let flow_new = flows_dir.join("new.yaml");
        fs::write(flow_new, "alias: new-flow\nproviderId: basic-flow").unwrap();

        let res = apply_authentication_flows(
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
                .contains("Failed to create authentication flow")
        );
    }
}
