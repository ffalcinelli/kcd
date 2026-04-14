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

pub type ComponentKey = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

pub fn build_component_indices(
    existing_components: impl IntoIterator<Item = ComponentRepresentation>,
) -> (
    HashMap<String, ComponentRepresentation>,
    HashMap<ComponentKey, ComponentRepresentation>,
) {
    let mut by_identity: HashMap<String, ComponentRepresentation> = HashMap::new();
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
    (by_identity, by_details)
}

pub async fn process_component_file(
    path: PathBuf,
    client: KeycloakClient,
    by_identity: Arc<HashMap<String, ComponentRepresentation>>,
    by_details: Arc<HashMap<ComponentKey, ComponentRepresentation>>,
    env_vars: Arc<HashMap<String, String>>,
    realm_name: String,
) -> Result<()> {
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
                .with_context(|| {
                    format!(
                        "Failed to update component '{}' in realm '{}'",
                        component_rep.get_name(),
                        realm_name
                    )
                })?;
            println!(
                "  {} {}",
                SUCCESS_UPDATE,
                style(format!("Updated component {}", component_rep.get_name())).cyan()
            );
        }
    } else {
        component_rep.id = None;
        client
            .create_component(&component_rep)
            .await
            .with_context(|| {
                format!(
                    "Failed to create component '{}' in realm '{}'",
                    component_rep.get_name(),
                    realm_name
                )
            })?;
        println!(
            "  {} {}",
            SUCCESS_CREATE,
            style(format!("Created component {}", component_rep.get_name())).green()
        );
    }
    Ok(())
}

pub async fn apply_components_or_keys(
    client: &KeycloakClient,
    workspace_dir: &std::path::Path,
    dir_name: &str,
    env_vars: Arc<HashMap<String, String>>,
    planned_files: Arc<Option<HashSet<PathBuf>>>,
    realm_name: &str,
) -> Result<()> {
    let components_dir = workspace_dir.join(dir_name);
    if async_fs::try_exists(&components_dir).await? {
        let existing_components = client
            .get_components()
            .await
            .with_context(|| format!("Failed to get components/keys for realm '{}'", realm_name))?;

        let (by_identity, by_details) = build_component_indices(existing_components);
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
                let realm_name = realm_name.to_string();
                set.spawn(async move {
                    process_component_file(
                        path,
                        client,
                        by_identity,
                        by_details,
                        env_vars,
                        realm_name,
                    )
                    .await
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

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        Ok((format!("http://{}", addr), call_count))
    }

    #[test]
    fn test_build_component_indices() {
        let comps = vec![
            ComponentRepresentation {
                id: Some("id1".to_string()),
                name: Some("comp1".to_string()),
                provider_id: Some("prov1".to_string()),
                provider_type: Some("type1".to_string()),
                sub_type: Some("sub1".to_string()),
                parent_id: Some("parent1".to_string()),
                config: None,
                extra: Default::default(),
            },
            ComponentRepresentation {
                id: None,
                name: Some("comp2".to_string()),
                provider_id: Some("prov2".to_string()),
                provider_type: Some("type2".to_string()),
                sub_type: None,
                parent_id: Some("parent2".to_string()),
                config: None,
                extra: Default::default(),
            },
        ];

        let (by_identity, by_details) = build_component_indices(comps);

        // get_identity for ComponentRepresentation returns `id.or_else(|| name)`.
        // First component has id "id1", so it uses "id1".
        // Second component has no id, so it uses "comp2".
        assert_eq!(by_identity.len(), 2);
        assert!(by_identity.contains_key("id1"));
        assert!(by_identity.contains_key("comp2"));

        assert_eq!(by_details.len(), 2);
        let key1 = (
            Some("comp1".to_string()),
            Some("sub1".to_string()),
            Some("prov1".to_string()),
            Some("parent1".to_string()),
        );
        let key2 = (
            Some("comp2".to_string()),
            None,
            Some("prov2".to_string()),
            Some("parent2".to_string()),
        );
        assert!(by_details.contains_key(&key1));
        assert!(by_details.contains_key(&key2));
    }

    #[tokio::test]
    async fn test_apply_components_error_paths() -> Result<()> {
        let (server_url, call_count) = start_mock_server().await?;
        let mut client = KeycloakClient::new(server_url);
        client.set_target_realm("test".to_string());
        client.set_token("mock_token".to_string());

        let temp = tempdir()?;
        let components_dir = temp.path().join("components");
        fs::create_dir(&components_dir)?;

        // 1. Test update failure
        call_count.store(0, std::sync::atomic::Ordering::SeqCst);
        let comp_existing = components_dir.join("existing.yaml");
        fs::write(comp_existing, "name: Existing Component\nid: existing-id")?;

        let res = apply_components_or_keys(
            &client,
            temp.path(),
            "components",
            Arc::new(HashMap::new()),
            Arc::new(None),
            "test",
        )
        .await;
        assert!(res.is_err());
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("Failed to update component")
        );

        fs::remove_file(components_dir.join("existing.yaml"))?;

        // 2. Test create failure
        call_count.store(0, std::sync::atomic::Ordering::SeqCst);
        let comp_new = components_dir.join("new.yaml");
        fs::write(comp_new, "name: New Component\nproviderId: new-provider")?;

        let res = apply_components_or_keys(
            &client,
            temp.path(),
            "components",
            Arc::new(HashMap::new()),
            Arc::new(None),
            "test",
        )
        .await;
        assert!(res.is_err());
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("Failed to create component")
        );

        Ok(())
    }
}
