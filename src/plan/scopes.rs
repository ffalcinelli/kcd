use crate::client::KeycloakClient;
use crate::models::{ClientScopeRepresentation, KeycloakResource};
use crate::utils::secrets::substitute_secrets;
use crate::utils::ui::SPARKLE;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs as async_fs;

use super::print_diff;

pub async fn plan_client_scopes(
    client: &KeycloakClient,
    workspace_dir: &Path,
    changes_only: bool,
    interactive: bool,
    env_vars: Arc<HashMap<String, String>>,
    changed_files: &mut Vec<PathBuf>,
    realm_name: &str,
) -> Result<()> {
    let scopes_dir = workspace_dir.join("client-scopes");
    if async_fs::try_exists(&scopes_dir).await? {
        let existing_scopes = client
            .get_client_scopes()
            .await
            .with_context(|| format!("Failed to get client scopes for realm '{}'", realm_name))?;
        let existing_scopes_map: HashMap<String, ClientScopeRepresentation> = existing_scopes
            .into_iter()
            .filter_map(|s| s.get_identity().map(|id| (id, s)))
            .collect();
        let existing_scopes_map = Arc::new(existing_scopes_map);

        let mut set = tokio::task::JoinSet::new();
        let mut entries = async_fs::read_dir(&scopes_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let env_vars = env_vars.clone();
                let existing_scopes_map = existing_scopes_map.clone();

                set.spawn(async move {
                    let content = async_fs::read_to_string(&path).await?;
                    let mut val: serde_json::Value = serde_yaml::from_str(&content)
                        .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                    substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
                    let local_scope: ClientScopeRepresentation = serde_json::from_value(val)
                        .with_context(|| format!("Failed to deserialize YAML file: {:?}", path))?;

                    let identity = local_scope
                        .get_identity()
                        .context(format!("Failed to get identity for scope in {:?}", path))?;
                    let remote = existing_scopes_map.get(&identity).cloned();

                    Ok::<
                        (
                            ClientScopeRepresentation,
                            PathBuf,
                            Option<ClientScopeRepresentation>,
                        ),
                        anyhow::Error,
                    >((local_scope, path, remote))
                });
            }
        }

        while let Some(res) = set.join_next().await {
            let (local_scope, path, remote) = res??;

            let changed = if let Some(remote) = remote {
                let mut remote_clone = remote.clone();
                if local_scope.id.is_none() {
                    remote_clone.id = None;
                }
                print_diff(
                    &format!("ClientScope {}", local_scope.get_name()),
                    Some(&remote_clone),
                    &local_scope,
                    changes_only,
                    "client_scope",
                )?
            } else {
                println!(
                    "\n{} Will create ClientScope: {}",
                    SPARKLE,
                    local_scope.get_name()
                );
                print_diff(
                    &format!("ClientScope {}", local_scope.get_name()),
                    None::<&ClientScopeRepresentation>,
                    &local_scope,
                    changes_only,
                    "client_scope",
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
