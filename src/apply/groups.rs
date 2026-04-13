use crate::client::KeycloakClient;
use crate::models::{GroupRepresentation, KeycloakResource};
use crate::utils::secrets::{SecretResolver, substitute_secrets};
use crate::utils::ui::{SUCCESS_CREATE, SUCCESS_UPDATE};
use anyhow::{Context, Result};
use console::style;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs as async_fs;
use tokio::task::JoinSet;

pub async fn apply_groups(
    client: &KeycloakClient,
    workspace_dir: &std::path::Path,
    resolver: Arc<dyn SecretResolver>,
    planned_files: Arc<Option<HashSet<PathBuf>>>,
    realm_name: &str,
) -> Result<()> {
    // 6. Apply Groups
    let groups_dir = workspace_dir.join("groups");
    if async_fs::try_exists(&groups_dir).await? {
        let existing_groups = client
            .get_groups()
            .await
            .with_context(|| format!("Failed to get groups for realm '{}'", realm_name))?;
        let existing_groups_map: HashMap<String, GroupRepresentation> = existing_groups
            .into_iter()
            .filter_map(|g| g.get_identity().map(|id| (id, g)))
            .collect();
        let existing_groups_map = Arc::new(existing_groups_map);

        let mut entries = async_fs::read_dir(&groups_dir).await?;
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
                let existing_groups_map = Arc::clone(&existing_groups_map);
                let resolver = Arc::clone(&resolver);
                let realm_name = realm_name.to_string();
                set.spawn(async move {
                    let content = async_fs::read_to_string(&path).await?;
                    let mut val: serde_json::Value = serde_yaml::from_str(&content)
                        .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                    substitute_secrets(&mut val, Arc::clone(&resolver)).await?;
                    let mut group_rep: GroupRepresentation = serde_json::from_value(val)?;

                    let identity = group_rep
                        .get_identity()
                        .context(format!("Failed to get identity for group in {:?}", path))?;

                    if let Some(existing) = existing_groups_map.get(&identity) {
                        if let Some(id) = &existing.id {
                            group_rep.id = Some(id.clone());
                            client.update_group(id, &group_rep).await.with_context(|| {
                                format!(
                                    "Failed to update group '{}' in realm '{}'",
                                    group_rep.get_name(),
                                    realm_name
                                )
                            })?;
                            println!(
                                "  {} {}",
                                SUCCESS_UPDATE,
                                style(format!("Updated group {}", group_rep.get_name())).cyan()
                            );
                        }
                    } else {
                        group_rep.id = None;
                        client.create_group(&group_rep).await.with_context(|| {
                            format!(
                                "Failed to create group '{}' in realm '{}'",
                                group_rep.get_name(),
                                realm_name
                            )
                        })?;
                        println!(
                            "  {} {}",
                            SUCCESS_CREATE,
                            style(format!("Created group {}", group_rep.get_name())).green()
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
    use crate::utils::secrets::EnvResolver;
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
                "/admin/realms/test/groups",
                get(|| async {
                    Json(vec![GroupRepresentation {
                        id: Some("existing-id".to_string()),
                        name: Some("Existing Group".to_string()),
                        path: Some("/existing-group".to_string()),
                        sub_groups: None,
                        extra: Default::default(),
                    }])
                }),
            )
            .route(
                "/admin/realms/test/groups/existing-id",
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
                "/admin/realms/test/groups",
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
    async fn test_apply_groups_error_paths() {
        let (server_url, call_count) = start_mock_server().await;
        let mut client = KeycloakClient::new(server_url);
        client.set_target_realm("test".to_string());
        client.set_token("mock_token".to_string());

        let temp = tempdir().unwrap();
        let groups_dir = temp.path().join("groups");
        fs::create_dir(&groups_dir).unwrap();
        let resolver = Arc::new(EnvResolver::new(HashMap::new()));

        // 1. Test update failure
        call_count.store(0, std::sync::atomic::Ordering::SeqCst);
        let group_existing = groups_dir.join("existing.yaml");
        fs::write(
            group_existing,
            "name: Existing Group\nid: existing-id\npath: /existing-group",
        )
        .unwrap();

        let res = apply_groups(
            &client,
            temp.path(),
            Arc::clone(&resolver) as Arc<dyn SecretResolver>,
            Arc::new(None),
            "test",
        )
        .await;
        assert!(res.is_err());
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("Failed to update group")
        );

        fs::remove_file(groups_dir.join("existing.yaml")).unwrap();

        // 2. Test create failure
        call_count.store(0, std::sync::atomic::Ordering::SeqCst);
        let group_new = groups_dir.join("new.yaml");
        fs::write(group_new, "name: New Group").unwrap();

        let res = apply_groups(
            &client,
            temp.path(),
            Arc::clone(&resolver) as Arc<dyn SecretResolver>,
            Arc::new(None),
            "test",
        )
        .await;
        assert!(res.is_err());
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("Failed to create group")
        );
    }
}
