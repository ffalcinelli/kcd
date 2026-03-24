use crate::client::KeycloakClient;
use crate::models::{IdentityProviderRepresentation, KeycloakResource};
use crate::utils::secrets::substitute_secrets;
use anyhow::{Context, Result};
use console::Emoji;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs as async_fs;

use super::print_diff;

pub async fn plan_identity_providers(
    client: &KeycloakClient,
    workspace_dir: &Path,
    changes_only: bool,
    interactive: bool,
    env_vars: Arc<HashMap<String, String>>,
    changed_files: &mut Vec<PathBuf>,
) -> Result<()> {
    let idps_dir = workspace_dir.join("identity-providers");
    if async_fs::try_exists(&idps_dir).await? {
        let existing_idps = client.get_identity_providers().await?;
        let existing_idps_map: HashMap<String, IdentityProviderRepresentation> = existing_idps
            .into_iter()
            .filter_map(|i| i.get_identity().map(|id| (id, i)))
            .collect();
        let existing_idps_map = Arc::new(existing_idps_map);

        let mut set = tokio::task::JoinSet::new();
        let mut entries = async_fs::read_dir(&idps_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let env_vars = env_vars.clone();
                let existing_idps_map = existing_idps_map.clone();

                set.spawn(async move {
                    let content = async_fs::read_to_string(&path).await?;
                    let mut val: serde_json::Value = serde_yaml::from_str(&content)
                        .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                    substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
                    let local_idp: IdentityProviderRepresentation = serde_json::from_value(val)
                        .with_context(|| format!("Failed to deserialize YAML file: {:?}", path))?;

                    let identity = local_idp
                        .get_identity()
                        .context(format!("Failed to get identity for IDP in {:?}", path))?;
                    let remote = existing_idps_map.get(&identity).cloned();

                    Ok::<
                        (
                            IdentityProviderRepresentation,
                            PathBuf,
                            Option<IdentityProviderRepresentation>,
                        ),
                        anyhow::Error,
                    >((local_idp, path, remote))
                });
            }
        }

        while let Some(res) = set.join_next().await {
            let (local_idp, path, remote) = res??;

            let changed = if let Some(remote) = remote {
                let mut remote_clone = remote.clone();
                if local_idp.internal_id.is_none() {
                    remote_clone.internal_id = None;
                }
                print_diff(
                    &format!("IdentityProvider {}", local_idp.get_name()),
                    Some(&remote_clone),
                    &local_idp,
                    changes_only,
                    "idp",
                )?
            } else {
                println!(
                    "\n{} Will create IdentityProvider: {}",
                    Emoji("✨", ""),
                    local_idp.get_name()
                );
                print_diff(
                    &format!("IdentityProvider {}", local_idp.get_name()),
                    None::<&IdentityProviderRepresentation>,
                    &local_idp,
                    changes_only,
                    "idp",
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
