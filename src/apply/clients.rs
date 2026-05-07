use crate::client::KeycloakClient;
use crate::models::{ClientRepresentation, KeycloakResource};
use crate::utils::secrets::{SecretResolver, substitute_secrets};
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs as async_fs;
use tokio::task::JoinSet;

pub async fn apply_clients(
    client: &KeycloakClient,
    workspace_dir: &std::path::Path,
    resolver: Arc<dyn SecretResolver>,
    planned_files: Arc<Option<HashSet<PathBuf>>>,
    realm_name: &str,
) -> Result<()> {
    // 3. Apply Clients
    let clients_dir = workspace_dir.join("clients");
    if async_fs::try_exists(&clients_dir).await? {
        let existing_clients = client
            .get_clients()
            .await
            .with_context(|| format!("Failed to get clients for realm '{}'", realm_name))?;
        let existing_clients_map: HashMap<String, ClientRepresentation> = existing_clients
            .into_iter()
            .filter_map(|c| c.get_identity().map(|id| (id, c)))
            .collect();
        let existing_clients_map = std::sync::Arc::new(existing_clients_map);

        let mut entries = async_fs::read_dir(&clients_dir).await?;
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
                let existing_clients_map = existing_clients_map.clone();
                let resolver = Arc::clone(&resolver);
                let realm_name = realm_name.to_string();
                set.spawn(async move {
                    let content = async_fs::read_to_string(&path).await?;
                    let mut val: serde_json::Value = serde_yaml::from_str(&content)
                        .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                    substitute_secrets(&mut val, Arc::clone(&resolver)).await?;
                    let mut client_rep: ClientRepresentation = serde_json::from_value(val)?;

                    let identity = client_rep
                        .get_identity()
                        .context(format!("Failed to get identity for client in {:?}", path))?;

                    crate::handle_upsert! {
                        client: client,
                        realm: realm_name,
                        rep: client_rep,
                        id_opt: existing_clients_map.get(&identity).and_then(|e| e.id.as_ref()),
                        id_field: id,
                        resource_name: "client",
                        update_call: |id, rep| client.update_client(id, rep),
                        create_call: |rep| client.create_client(rep)
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
    async fn test_apply_clients_error_paths() -> Result<()> {
        let (server_url, call_count) = start_mock_server().await?;
        let mut client = KeycloakClient::new(server_url);
        client.set_target_realm("test".to_string());
        client.set_token("mock_token".to_string());

        let temp = tempdir()?;
        let clients_dir = temp.path().join("clients");
        fs::create_dir(&clients_dir)?;
        let resolver = Arc::new(EnvResolver::new(HashMap::new()));

        // 1. Test update failure
        call_count.store(0, std::sync::atomic::Ordering::SeqCst);
        let client_existing = clients_dir.join("existing.yaml");
        fs::write(
            client_existing,
            "clientId: existing-client\nname: Existing Client",
        )?;

        let res = apply_clients(
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
                .contains("Failed to update client")
        );

        fs::remove_file(clients_dir.join("existing.yaml"))?;

        // 2. Test create failure
        call_count.store(0, std::sync::atomic::Ordering::SeqCst);
        let client_new = clients_dir.join("new.yaml");
        fs::write(client_new, "clientId: new-client\nname: New Client")?;

        let res = apply_clients(
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
                .contains("Failed to create client")
        );

        // 3. Test invalid YAML
        let client_invalid = clients_dir.join("invalid.yaml");
        fs::write(client_invalid, "invalid: yaml: :")?;
        let res = apply_clients(
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
                .contains("Failed to parse YAML file")
        );

        Ok(())
    }
}
