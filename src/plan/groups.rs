use crate::client::KeycloakClient;
use crate::models::{GroupRepresentation, KeycloakResource};
use crate::utils::secrets::substitute_secrets;
use crate::utils::ui::SPARKLE;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs as async_fs;

use super::print_diff;

pub async fn plan_groups(
    client: &KeycloakClient,
    workspace_dir: &Path,
    changes_only: bool,
    interactive: bool,
    env_vars: Arc<HashMap<String, String>>,
    changed_files: &mut Vec<PathBuf>,
    realm_name: &str,
) -> Result<()> {
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

        let mut set = tokio::task::JoinSet::new();
        let mut entries = async_fs::read_dir(&groups_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let env_vars = env_vars.clone();
                let existing_groups_map = existing_groups_map.clone();

                set.spawn(async move {
                    let content = async_fs::read_to_string(&path).await?;
                    let mut val: serde_json::Value = serde_yaml::from_str(&content)
                        .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                    substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
                    let local_group: GroupRepresentation = serde_json::from_value(val)
                        .with_context(|| format!("Failed to deserialize YAML file: {:?}", path))?;

                    let identity = local_group
                        .get_identity()
                        .context(format!("Failed to get identity for group in {:?}", path))?;
                    let remote = existing_groups_map.get(&identity).cloned();

                    Ok::<(GroupRepresentation, PathBuf, Option<GroupRepresentation>), anyhow::Error>((
                        local_group,
                        path,
                        remote,
                    ))
                });
            }
        }

        while let Some(res) = set.join_next().await {
            let (local_group, path, remote) = res??;

            let changed = if let Some(remote) = remote {
                let mut remote_clone = remote.clone();
                if local_group.id.is_none() {
                    remote_clone.id = None;
                }
                print_diff(
                    &format!("Group {}", local_group.get_name()),
                    Some(&remote_clone),
                    &local_group,
                    changes_only,
                    "group",
                )?
            } else {
                println!(
                    "\n{} Will create Group: {}",
                    SPARKLE,
                    local_group.get_name()
                );
                print_diff(
                    &format!("Group {}", local_group.get_name()),
                    None::<&GroupRepresentation>,
                    &local_group,
                    changes_only,
                    "group",
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
