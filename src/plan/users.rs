use crate::client::KeycloakClient;
use crate::models::{KeycloakResource, UserRepresentation};
use crate::utils::secrets::substitute_secrets;
use crate::utils::ui::SPARKLE;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs as async_fs;

use super::print_diff;

pub async fn plan_users(
    client: &KeycloakClient,
    workspace_dir: &Path,
    changes_only: bool,
    interactive: bool,
    env_vars: Arc<HashMap<String, String>>,
    changed_files: &mut Vec<PathBuf>,
    realm_name: &str,
) -> Result<()> {
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

        let mut set = tokio::task::JoinSet::new();
        let mut entries = async_fs::read_dir(&users_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let env_vars = env_vars.clone();
                let existing_users_map = existing_users_map.clone();

                set.spawn(async move {
                    let content = async_fs::read_to_string(&path).await?;
                    let mut val: serde_json::Value = serde_yaml::from_str(&content)
                        .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                    substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
                    let local_user: UserRepresentation = serde_json::from_value(val)
                        .with_context(|| format!("Failed to deserialize YAML file: {:?}", path))?;

                    let identity = local_user
                        .get_identity()
                        .context(format!("Failed to get identity for user in {:?}", path))?;
                    let remote = existing_users_map.get(&identity).cloned();

                    Ok::<(UserRepresentation, PathBuf, Option<UserRepresentation>), anyhow::Error>(
                        (local_user, path, remote),
                    )
                });
            }
        }

        while let Some(res) = set.join_next().await {
            let (local_user, path, remote) = res??;

            let changed = if let Some(remote) = remote {
                let mut remote_clone = remote.clone();
                if local_user.id.is_none() {
                    remote_clone.id = None;
                }
                print_diff(
                    &format!("User {}", local_user.get_name()),
                    Some(&remote_clone),
                    &local_user,
                    changes_only,
                    "user",
                )?
            } else {
                println!("\n{} Will create User: {}", SPARKLE, local_user.get_name());
                print_diff(
                    &format!("User {}", local_user.get_name()),
                    None::<&UserRepresentation>,
                    &local_user,
                    changes_only,
                    "user",
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
