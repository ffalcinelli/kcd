use crate::client::KeycloakClient;
use crate::models::{ClientRepresentation, KeycloakResource};
use crate::utils::secrets::substitute_secrets;
use crate::utils::ui::SPARKLE;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs as async_fs;

use super::print_diff;

pub async fn plan_clients(
    client: &KeycloakClient,
    workspace_dir: &Path,
    changes_only: bool,
    interactive: bool,
    env_vars: Arc<HashMap<String, String>>,
    changed_files: &mut Vec<PathBuf>,
    realm_name: &str,
) -> Result<()> {
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
        let existing_clients_map = Arc::new(existing_clients_map);

        let mut set = tokio::task::JoinSet::new();
        let mut entries = async_fs::read_dir(&clients_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let env_vars = env_vars.clone();
                let existing_clients_map = existing_clients_map.clone();
                let realm_name = realm_name.to_string();

                set.spawn(async move {
                    let content = async_fs::read_to_string(&path).await?;
                    let mut val: serde_json::Value = serde_yaml::from_str(&content)
                        .with_context(|| format!("Failed to parse YAML file {:?} in realm '{}'", path, realm_name))?;
                    substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
                    let local_client: ClientRepresentation = serde_json::from_value(val)
                        .with_context(|| format!("Failed to deserialize YAML file {:?} in realm '{}'", path, realm_name))?;

                    let identity = local_client
                        .get_identity()
                        .with_context(|| format!("Failed to get identity for client in {:?} in realm '{}'", path, realm_name))?;
                    let remote = existing_clients_map.get(&identity).cloned();

                    Ok::<(ClientRepresentation, PathBuf, Option<ClientRepresentation>), anyhow::Error>((
                        local_client,
                        path,
                        remote,
                    ))
                });
            }
        }

        while let Some(res) = set.join_next().await {
            let (local_client, path, remote) = res??;

            let changed = if let Some(remote) = remote {
                let mut remote_clone = remote.clone();
                if local_client.id.is_none() {
                    remote_clone.id = None;
                }
                print_diff(
                    &format!("Client {}", local_client.get_name()),
                    Some(&remote_clone),
                    &local_client,
                    changes_only,
                    "client",
                )?
            } else {
                println!(
                    "\n{} Will create Client: {}",
                    SPARKLE,
                    local_client.get_name()
                );
                print_diff(
                    &format!("Client {}", local_client.get_name()),
                    None::<&ClientRepresentation>,
                    &local_client,
                    changes_only,
                    "client",
                )?
            };

            if changed {
                let mut include = true;
                if interactive {
                    include =
                        dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
                            .with_prompt("Include this change in the plan?")
                            .default(true)
                            .interact()?;
                }
                if include {
                    changed_files.push(path);
                }
            }
        }
    }
    Ok(())
}
