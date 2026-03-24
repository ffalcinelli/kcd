use crate::client::KeycloakClient;
use crate::models::{ComponentRepresentation, KeycloakResource};
use crate::utils::secrets::substitute_secrets;
use anyhow::{Context, Result};
use console::style;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs as async_fs;
use tokio::task::JoinSet;

use super::{SUCCESS_CREATE, SUCCESS_UPDATE};

pub async fn apply_components_or_keys(
    client: &KeycloakClient,
    workspace_dir: &std::path::Path,
    dir_name: &str,
    env_vars: Arc<HashMap<String, String>>,
    planned_files: Arc<Option<HashSet<PathBuf>>>,
) -> Result<()> {
    let components_dir = workspace_dir.join(dir_name);
    if async_fs::try_exists(&components_dir).await? {
        let existing_components = client.get_components().await?;
        let mut by_identity: HashMap<String, ComponentRepresentation> = HashMap::new();
        type ComponentKey = (
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        );
        let mut by_details: HashMap<ComponentKey, ComponentRepresentation> = HashMap::new();

        for c in existing_components {
            if let Some(id) = c.get_identity() {
                by_identity.insert(id, c.clone());
            }
            let key = (
                c.name.clone(),
                c.sub_type.clone(),
                c.provider_id.clone(),
                c.parent_id.clone(),
            );
            by_details.insert(key, c);
        }
        let by_identity = Arc::new(by_identity);
        let by_details = Arc::new(by_details);

        let mut entries = async_fs::read_dir(&components_dir).await?;
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
                let by_identity = Arc::clone(&by_identity);
                let by_details = Arc::clone(&by_details);
                let env_vars = Arc::clone(&env_vars);
                set.spawn(async move {
                    let content = async_fs::read_to_string(&path).await?;
                    let mut val: serde_json::Value = serde_yaml::from_str(&content)
                        .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                    substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
                    let mut component_rep: ComponentRepresentation = serde_json::from_value(val)?;

                    let existing = if let Some(identity) = component_rep.get_identity() {
                        by_identity.get(&identity).or_else(|| {
                            let key = (
                                component_rep.name.clone(),
                                component_rep.sub_type.clone(),
                                component_rep.provider_id.clone(),
                                component_rep.parent_id.clone(),
                            );
                            by_details.get(&key)
                        })
                    } else {
                        let key = (
                            component_rep.name.clone(),
                            component_rep.sub_type.clone(),
                            component_rep.provider_id.clone(),
                            component_rep.parent_id.clone(),
                        );
                        by_details.get(&key)
                    };

                    if let Some(existing) = existing {
                        if let Some(id) = &existing.id {
                            component_rep.id = Some(id.clone());
                            client
                                .update_component(id, &component_rep)
                                .await
                                .context(format!(
                                    "Failed to update component {}",
                                    component_rep.get_name()
                                ))?;
                            println!(
                                "  {} {}",
                                SUCCESS_UPDATE,
                                style(format!("Updated component {}", component_rep.get_name()))
                                    .cyan()
                            );
                        }
                    } else {
                        component_rep.id = None;
                        client
                            .create_component(&component_rep)
                            .await
                            .context(format!(
                                "Failed to create component {}",
                                component_rep.get_name()
                            ))?;
                        println!(
                            "  {} {}",
                            SUCCESS_CREATE,
                            style(format!("Created component {}", component_rep.get_name()))
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
                "/admin/realms/test/components",
                get(|| async {
                    Json(vec![ComponentRepresentation {
                        id: Some("existing-id".to_string()),
                        name: Some("Existing Component".to_string()),
                        provider_id: Some("existing-provider".to_string()),
                        provider_type: Some("existing-type".to_string()),
                        parent_id: Some("test".to_string()),
                        sub_type: None,
                        config: None,
                        extra: Default::default(),
                    }])
                }),
            )
            .route(
                "/admin/realms/test/components/existing-id",
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
                "/admin/realms/test/components",
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
    async fn test_apply_components_error_paths() {
        let (server_url, call_count) = start_mock_server().await;
        let mut client = KeycloakClient::new(server_url);
        client.set_target_realm("test".to_string());
        client.set_token("mock_token".to_string());

        let temp = tempdir().unwrap();
        let components_dir = temp.path().join("components");
        fs::create_dir(&components_dir).unwrap();

        // 1. Test update failure
        call_count.store(0, std::sync::atomic::Ordering::SeqCst);
        let comp_existing = components_dir.join("existing.yaml");
        fs::write(comp_existing, "name: Existing Component\nid: existing-id").unwrap();

        let res = apply_components_or_keys(
            &client,
            temp.path(),
            "components",
            Arc::new(HashMap::new()),
            Arc::new(None),
        )
        .await;
        assert!(res.is_err());
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("Failed to update component")
        );

        fs::remove_file(components_dir.join("existing.yaml")).unwrap();

        // 2. Test create failure
        call_count.store(0, std::sync::atomic::Ordering::SeqCst);
        let comp_new = components_dir.join("new.yaml");
        fs::write(comp_new, "name: New Component\nproviderId: new-provider").unwrap();

        let res = apply_components_or_keys(
            &client,
            temp.path(),
            "components",
            Arc::new(HashMap::new()),
            Arc::new(None),
        )
        .await;
        assert!(res.is_err());
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("Failed to create component")
        );
    }
}
