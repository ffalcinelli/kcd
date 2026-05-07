use crate::client::KeycloakClient;
use crate::models::{KeycloakResource, UserRepresentation};
use crate::utils::secrets::{SecretResolver, substitute_secrets};
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs as async_fs;
use tokio::task::JoinSet;

pub async fn apply_users(
    client: &KeycloakClient,
    workspace_dir: &std::path::Path,
    resolver: Arc<dyn SecretResolver>,
    planned_files: Arc<Option<HashSet<PathBuf>>>,
    realm_name: &str,
) -> Result<()> {
    // 7. Apply Users
    let users_dir = workspace_dir.join("users");
    if async_fs::try_exists(&users_dir).await? {
        let existing_users = client
            .get_users()
            .await
            .with_context(|| format!("Failed to get users for realm '{}'", realm_name))?;
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
                let resolver = Arc::clone(&resolver);
                let realm_name = realm_name.to_string();
                set.spawn(async move {
                    let content = async_fs::read_to_string(&path).await?;
                    let mut val: serde_json::Value = serde_yaml::from_str(&content)
                        .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                    substitute_secrets(&mut val, resolver).await?;
                    let mut user_rep: UserRepresentation = serde_json::from_value(val)?;

                    let identity = user_rep
                        .get_identity()
                        .context(format!("Failed to get identity for user in {:?}", path))?;

                    crate::handle_upsert! {
                        client: client,
                        realm: realm_name,
                        rep: user_rep,
                        id_opt: existing_users_map.get(&identity).and_then(|e| e.id.as_ref()),
                        id_field: id,
                        resource_name: "user",
                        update_call: |id, rep| client.update_user(id, rep),
                        create_call: |rep| client.create_user(rep)
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
    async fn test_apply_users_error_paths() -> Result<()> {
        let (server_url, call_count) = start_mock_server().await?;
        let mut client = KeycloakClient::new(server_url);
        client.set_target_realm("test".to_string());
        client.set_token("mock_token".to_string());

        let temp = tempdir()?;
        let users_dir = temp.path().join("users");
        fs::create_dir(&users_dir)?;

        let resolver = Arc::new(EnvResolver::new(HashMap::new()));

        // 1. Test update failure
        call_count.store(0, std::sync::atomic::Ordering::SeqCst);
        let user_existing = users_dir.join("existing.yaml");
        fs::write(user_existing, "username: existing-user\nid: existing-id")?;

        let res = apply_users(
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
                .contains("Failed to update user")
        );

        fs::remove_file(users_dir.join("existing.yaml"))?;

        // 2. Test create failure
        call_count.store(0, std::sync::atomic::Ordering::SeqCst);
        let user_new = users_dir.join("new.yaml");
        fs::write(user_new, "username: new-user")?;

        let res = apply_users(
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
                .contains("Failed to create user")
        );

        Ok(())
    }
}
