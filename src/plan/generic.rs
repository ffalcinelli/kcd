use crate::models::{KeycloakResource, ResourceMeta};
use crate::utils::secrets::substitute_secrets;
use crate::utils::ui::SPARKLE;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs as async_fs;

use super::{PlanContext, print_diff};

pub async fn plan_resources<T>(
    ctx: &PlanContext<'_>,
    changed_files: &mut Vec<PathBuf>,
) -> Result<()>
where
    T: KeycloakResource
        + ResourceMeta
        + serde::Serialize
        + for<'de> serde::Deserialize<'de>
        + Send
        + Sync
        + Clone
        + 'static,
{
    let dir_name = T::dir_name();
    let resources_dir = ctx.workspace_dir.join(dir_name);
    if !async_fs::try_exists(&resources_dir).await? {
        return Ok(());
    }

    let existing_resources = ctx.client.get_resources::<T>().await.with_context(|| {
        format!(
            "Failed to get {} for realm '{}'",
            T::label(),
            ctx.realm_name
        )
    })?;

    let existing_map: HashMap<String, T> = existing_resources
        .into_iter()
        .filter_map(|r| r.get_identity().map(|id| (id, r)))
        .collect();
    let existing_map = Arc::new(existing_map);

    let mut set = tokio::task::JoinSet::new();
    let mut entries = async_fs::read_dir(&resources_dir).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "yaml") {
            let resolver = Arc::clone(&ctx.resolver);
            let existing_map = Arc::clone(&existing_map);
            let realm_name = ctx.realm_name.to_string();

            set.spawn(async move {
                let content = async_fs::read_to_string(&path).await?;
                let mut val: serde_json::Value =
                    serde_yaml::from_str(&content).with_context(|| {
                        format!(
                            "Failed to parse YAML file {:?} in realm '{}'",
                            path, realm_name
                        )
                    })?;
                substitute_secrets(&mut val, resolver).await?;
                let local: T = serde_json::from_value(val).with_context(|| {
                    format!(
                        "Failed to deserialize YAML file {:?} in realm '{}'",
                        path, realm_name
                    )
                })?;

                let identity = local.get_identity().with_context(|| {
                    format!(
                        "Failed to get identity for {} in {:?} in realm '{}'",
                        T::label(),
                        path,
                        realm_name
                    )
                })?;
                let remote = existing_map.get(&identity).cloned();

                Ok::<(T, PathBuf, Option<T>), anyhow::Error>((local, path, remote))
            });
        }
    }

    while let Some(res) = set.join_next().await {
        let (local, path, remote) = res??;

        let changed = if let Some(remote) = remote {
            let mut remote_clone = remote.clone();
            // If local doesn't have an ID, clear it from remote clone for diffing
            if !local.has_id() {
                remote_clone.clear_metadata();
            }
            print_diff(
                &format!("{} {}", T::label(), local.get_name()),
                Some(&remote_clone),
                &local,
                ctx.options.changes_only,
                T::secret_prefix(),
            )?
        } else {
            println!("\n{} Will create {}", SPARKLE, T::label());
            print_diff(
                &format!("{} {}", T::label(), local.get_name()),
                None::<&T>,
                &local,
                ctx.options.changes_only,
                T::secret_prefix(),
            )?
        };

        if changed {
            let mut include = true;
            if ctx.options.interactive {
                include = ctx.ui.confirm("Include this change in the plan?", true)?;
            }
            if include {
                changed_files.push(path);
            }
        }
    }
    Ok(())
}
