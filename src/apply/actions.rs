use crate::client::KeycloakClient;
use crate::models::{KeycloakResource, RequiredActionProviderRepresentation};
use crate::utils::secrets::{SecretResolver, substitute_secrets};
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
    resolver: Arc<dyn SecretResolver>,
    planned_files: Arc<Option<HashSet<PathBuf>>>,
    realm_name: &str,
) -> Result<()> {
    // 9. Apply Required Actions
    let actions_dir = workspace_dir.join("required-actions");
    if async_fs::try_exists(&actions_dir).await? {
        let existing_actions = client.get_required_actions().await.with_context(|| {
            format!("Failed to get required actions for realm '{}'", realm_name)
        })?;
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
                let resolver = Arc::clone(&resolver);
                let realm_name = realm_name.to_string();
                set.spawn(async move {
                    let content = async_fs::read_to_string(&path).await?;
                    let mut val: serde_json::Value = serde_yaml::from_str(&content)
                        .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                    substitute_secrets(&mut val, Arc::clone(&resolver)).await?;
                    #[allow(unused_mut)]
                    let mut action_rep: RequiredActionProviderRepresentation =
                        serde_json::from_value(val)?;

                    let identity = action_rep.get_identity().context(format!(
                        "Failed to get identity for required action in {:?}",
                        path
                    ))?;

                    if existing_actions_map.contains_key(&identity) {
                        client
                            .update_required_action(&identity, &action_rep)
                            .await
                            .with_context(|| {
                                format!(
                                    "Failed to update required action '{}' in realm '{}'",
                                    action_rep.get_name(),
                                    realm_name
                                )
                            })?;
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
                            .with_context(|| {
                                format!(
                                    "Failed to register required action '{}' in realm '{}'",
                                    action_rep.get_name(),
                                    realm_name
                                )
                            })?;
                        client
                            .update_required_action(&identity, &action_rep)
                            .await
                            .with_context(|| {
                                format!(
                                    "Failed to configure registered required action '{}' in realm '{}'",
                                    action_rep.get_name(),
                                    realm_name
                                )
                            })?;
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
    use crate::apply::test_utils::start_mock_server;

    use super::*;
    use crate::client::KeycloakClient;
    use crate::utils::secrets::EnvResolver;

    use std::fs;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_apply_required_actions_error_paths() -> Result<()> {
        let (server_url, call_count) = start_mock_server().await?;
        let mut client = KeycloakClient::new(server_url);
        client.set_target_realm("test".to_string());
        client.set_token("mock_token".to_string());

        let temp = tempdir()?;
        let actions_dir = temp.path().join("required-actions");
        fs::create_dir(&actions_dir)?;
        let resolver = Arc::new(EnvResolver::new(HashMap::new()));

        // 1. Test missing identity (alias missing)
        let action_no_alias = actions_dir.join("no_alias.yaml");
        fs::write(action_no_alias, "name: No Alias\nproviderId: some-provider")?;

        let res = apply_required_actions(
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
                .contains("Failed to get identity")
        );

        fs::remove_file(actions_dir.join("no_alias.yaml"))?;

        // 2. Test update failure
        call_count.store(0, std::sync::atomic::Ordering::SeqCst);
        let action_existing = actions_dir.join("existing.yaml");
        fs::write(
            action_existing,
            "alias: existing-action\nname: Existing Action\nproviderId: existing-provider",
        )?;

        let res = apply_required_actions(
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
                .contains("Failed to update required action")
        );

        fs::remove_file(actions_dir.join("existing.yaml"))?;

        // 3. Test register failure
        call_count.store(0, std::sync::atomic::Ordering::SeqCst);
        let action_new = actions_dir.join("new.yaml");
        fs::write(
            action_new,
            "alias: new-action\nname: New Action\nproviderId: new-provider",
        )?;

        let res = apply_required_actions(
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
                .contains("Failed to register required action")
        );

        // 4. Test update after register failure
        // The mock server is set up to succeed on second register call but fail on update
        // We just called it once, so next call to register-required-action will succeed (c will be 1)
        let res = apply_required_actions(
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
                .contains("Failed to configure registered required action")
        );

        Ok(())
    }
}
