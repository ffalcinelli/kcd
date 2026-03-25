use crate::client::KeycloakClient;
use crate::models::{KeycloakResource, RoleRepresentation};
use crate::utils::secrets::substitute_secrets;
use crate::utils::ui::SPARKLE;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs as async_fs;

use super::print_diff;

pub async fn plan_roles(
    client: &KeycloakClient,
    workspace_dir: &Path,
    changes_only: bool,
    interactive: bool,
    env_vars: Arc<HashMap<String, String>>,
    changed_files: &mut Vec<PathBuf>,
    realm_name: &str,
) -> Result<()> {
    let roles_dir = workspace_dir.join("roles");
    if async_fs::try_exists(&roles_dir).await? {
        let existing_roles = client
            .get_roles()
            .await
            .with_context(|| format!("Failed to get roles for realm '{}'", realm_name))?;
        let existing_roles_map: HashMap<String, RoleRepresentation> = existing_roles
            .into_iter()
            .filter_map(|r| r.get_identity().map(|id| (id, r)))
            .collect();
        let existing_roles_map = Arc::new(existing_roles_map);

        let mut set = tokio::task::JoinSet::new();
        let mut entries = async_fs::read_dir(&roles_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let env_vars = env_vars.clone();
                let existing_roles_map = existing_roles_map.clone();

                set.spawn(async move {
                    let content = async_fs::read_to_string(&path).await?;
                    let mut val: serde_json::Value = serde_yaml::from_str(&content)
                        .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                    substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
                    let local_role: RoleRepresentation = serde_json::from_value(val)
                        .with_context(|| format!("Failed to deserialize YAML file: {:?}", path))?;

                    let identity = local_role
                        .get_identity()
                        .context(format!("Failed to get identity for role in {:?}", path))?;
                    let remote = existing_roles_map.get(&identity).cloned();

                    Ok::<(RoleRepresentation, PathBuf, Option<RoleRepresentation>), anyhow::Error>(
                        (local_role, path, remote),
                    )
                });
            }
        }

        while let Some(res) = set.join_next().await {
            let (local_role, path, remote) = res??;

            let changed = if let Some(remote) = remote {
                let mut remote_clone = remote.clone();
                // Ignore ID differences if local doesn't specify it
                if local_role.id.is_none() {
                    remote_clone.id = None;
                    remote_clone.container_id = None;
                }
                print_diff(
                    &format!("Role {}", local_role.get_name()),
                    Some(&remote_clone),
                    &local_role,
                    changes_only,
                    "role",
                )?
            } else {
                println!("\n{} Will create Role: {}", SPARKLE, local_role.get_name());
                print_diff(
                    &format!("Role {}", local_role.get_name()),
                    None::<&RoleRepresentation>,
                    &local_role,
                    changes_only,
                    "role",
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
