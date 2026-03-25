use crate::client::KeycloakClient;
use crate::models::{KeycloakResource, RequiredActionProviderRepresentation};
use crate::utils::secrets::substitute_secrets;
use crate::utils::ui::SPARKLE;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs as async_fs;

use super::print_diff;

pub async fn plan_required_actions(
    client: &KeycloakClient,
    workspace_dir: &Path,
    changes_only: bool,
    interactive: bool,
    env_vars: Arc<HashMap<String, String>>,
    changed_files: &mut Vec<PathBuf>,
    realm_name: &str,
) -> Result<()> {
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

        let mut set = tokio::task::JoinSet::new();
        let mut entries = async_fs::read_dir(&actions_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let env_vars = env_vars.clone();
                let existing_actions_map = existing_actions_map.clone();
                let realm_name = realm_name.to_string();

                set.spawn(async move {
                    let content = async_fs::read_to_string(&path).await?;
                    let mut val: serde_json::Value =
                        serde_yaml::from_str(&content).with_context(|| {
                            format!(
                                "Failed to parse YAML file {:?} in realm '{}'",
                                path, realm_name
                            )
                        })?;
                    substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
                    let local_action: RequiredActionProviderRepresentation =
                        serde_json::from_value(val).with_context(|| {
                            format!(
                                "Failed to deserialize YAML file {:?} in realm '{}'",
                                path, realm_name
                            )
                        })?;

                    let identity = local_action.get_identity().with_context(|| {
                        format!(
                            "Failed to get identity for action in {:?} in realm '{}'",
                            path, realm_name
                        )
                    })?;
                    let remote = existing_actions_map.get(&identity).cloned();

                    Ok::<
                        (
                            RequiredActionProviderRepresentation,
                            PathBuf,
                            Option<RequiredActionProviderRepresentation>,
                        ),
                        anyhow::Error,
                    >((local_action, path, remote))
                });
            }
        }

        while let Some(res) = set.join_next().await {
            let (local_action, path, remote) = res??;

            let changed = if let Some(remote) = remote {
                let remote_clone = remote.clone();
                print_diff(
                    &format!("RequiredAction {}", local_action.get_name()),
                    Some(&remote_clone),
                    &local_action,
                    changes_only,
                    "action",
                )?
            } else {
                println!(
                    "\n{} Will create RequiredAction: {}",
                    SPARKLE,
                    local_action.get_name()
                );
                print_diff(
                    &format!("RequiredAction {}", local_action.get_name()),
                    None::<&RequiredActionProviderRepresentation>,
                    &local_action,
                    changes_only,
                    "action",
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
