use crate::client::KeycloakClient;
use crate::models::{ComponentRepresentation, KeycloakResource};
use crate::utils::secrets::substitute_secrets;
use crate::utils::ui::{SPARKLE, WARN, Ui};
use anyhow::{Context, Result};
use console::style;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs as async_fs;

use super::{PlanOptions, print_diff};

pub async fn plan_components_or_keys(
    client: &KeycloakClient,
    workspace_dir: &Path,
    options: PlanOptions,
    dir_name: &str,
    env_vars: Arc<HashMap<String, String>>,
    changed_files: &mut Vec<PathBuf>,
    realm_name: &str,
    ui: &dyn Ui,
) -> Result<()> {
    let components_dir = workspace_dir.join(dir_name);
    if async_fs::try_exists(&components_dir).await? {
        let existing_components = client
            .get_components()
            .await
            .with_context(|| format!("Failed to get components for realm '{}'", realm_name))?;
        let mut by_identity: HashMap<String, ComponentRepresentation> = HashMap::new();
        type ComponentKey = (
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        );
        let mut by_details: HashMap<ComponentKey, ComponentRepresentation> = HashMap::new();

        for c in existing_components {
            if let Some(id) = c.get_identity() {
                by_identity.insert(id, c.clone());
            }
            let key = (
                c.name.clone(),
                c.sub_type.clone(),
                c.provider_id.clone(),
                c.parent_id.clone(),
            );
            by_details.insert(key, c);
        }

        let by_identity = Arc::new(by_identity);
        let by_details = Arc::new(by_details);

        let mut set = tokio::task::JoinSet::new();
        let mut entries = async_fs::read_dir(&components_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let env_vars = env_vars.clone();
                let by_identity = by_identity.clone();
                let by_details = by_details.clone();
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
                    let local_component: ComponentRepresentation = serde_json::from_value(val)
                        .with_context(|| {
                            format!(
                                "Failed to deserialize YAML file {:?} in realm '{}'",
                                path, realm_name
                            )
                        })?;

                    let remote = if let Some(identity) = local_component.get_identity() {
                        by_identity
                            .get(&identity)
                            .or_else(|| {
                                let key = (
                                    local_component.name.clone(),
                                    local_component.sub_type.clone(),
                                    local_component.provider_id.clone(),
                                    local_component.parent_id.clone(),
                                );
                                by_details.get(&key)
                            })
                            .cloned()
                    } else {
                        let key = (
                            local_component.name.clone(),
                            local_component.sub_type.clone(),
                            local_component.provider_id.clone(),
                            local_component.parent_id.clone(),
                        );
                        by_details.get(&key).cloned()
                    };

                    Ok::<
                        (
                            ComponentRepresentation,
                            PathBuf,
                            Option<ComponentRepresentation>,
                        ),
                        anyhow::Error,
                    >((local_component, path, remote))
                });
            }
        }

        while let Some(res) = set.join_next().await {
            let (local_component, path, remote) = res??;

            let changed = if let Some(remote) = remote {
                let mut remote_clone = remote.clone();
                if local_component.id.is_none() {
                    remote_clone.id = None;
                }
                let prefix = if dir_name == "keys" {
                    "key"
                } else {
                    "component"
                };
                print_diff(
                    &format!("Component {}", local_component.get_name()),
                    Some(&remote_clone),
                    &local_component,
                    options.changes_only,
                    prefix,
                )?
            } else {
                println!(
                    "\n{} Will create Component: {}",
                    SPARKLE,
                    local_component.get_name()
                );
                let prefix = if dir_name == "keys" {
                    "key"
                } else {
                    "component"
                };
                print_diff(
                    &format!("Component {}", local_component.get_name()),
                    None::<&ComponentRepresentation>,
                    &local_component,
                    options.changes_only,
                    prefix,
                )?
            };

            if changed {
                let mut include = true;
                if options.interactive {
                    include = ui.confirm("Include this change in the plan?", true)?;
                }
                if include {
                    changed_files.push(path);
                }
            }
        }
    }
    Ok(())
}

pub async fn check_keys_drift(
    client: &KeycloakClient,
    options: PlanOptions,
    realm_name: &str,
) -> Result<()> {
    if !options.changes_only {
        return Ok(());
    }

    let keys_metadata = match client.get_keys().await {
        Ok(km) => km,
        Err(_) => return Ok(()), // Ignore if not available
    };

    if let Some(keys) = keys_metadata.keys {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("System clock is before UNIX EPOCH")?
            .as_millis() as i64;
        let thirty_days = 30 * 24 * 60 * 60 * 1000; // 30 days in ms

        for key in keys {
            #[allow(clippy::collapsible_if)]
            if key.status.as_deref() == Some("ACTIVE") {
                if let Some(valid_to) = key.valid_to {
                    #[allow(clippy::collapsible_if)]
                    if valid_to > 0 && valid_to - now < thirty_days {
                        let provider_id = key.provider_id.as_deref().unwrap_or("unknown");
                        println!(
                            "{} Warning: Active key (providerId: {}) in realm '{}' is near expiration or expired! Consider rotating keys.",
                            WARN,
                            style(provider_id).yellow(),
                            realm_name
                        );
                    }
                }
            }
        }
    }

    Ok(())
}
