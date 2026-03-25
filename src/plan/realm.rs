use crate::client::KeycloakClient;
use crate::models::RealmRepresentation;
use crate::utils::secrets::substitute_secrets;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs as async_fs;

use super::print_diff;

pub async fn plan_realm(
    client: &KeycloakClient,
    workspace_dir: &Path,
    changes_only: bool,
    interactive: bool,
    env_vars: Arc<HashMap<String, String>>,
    changed_files: &mut Vec<PathBuf>,
    realm_name: &str,
) -> Result<()> {
    let realm_path = workspace_dir.join("realm.yaml");
    if async_fs::try_exists(&realm_path).await? {
        let content = async_fs::read_to_string(&realm_path).await?;
        let mut val: serde_json::Value = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse YAML file: {:?}", realm_path))?;
        substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
        let local_realm: RealmRepresentation = serde_json::from_value(val)
            .with_context(|| format!("Failed to deserialize YAML file: {:?}", realm_path))?;

        // We handle the case where remote realm fetch might fail (e.g. if we are creating it)
        // by treating it as None (creation). However, usually plan is run against existing realm.
        let remote_realm = match client.get_realm().await {
            Ok(r) => Some(r),
            Err(e) => {
                // Check if it's a 404 (Not Found)
                if e.to_string().contains("404") {
                    None
                } else {
                    return Err(e).with_context(|| {
                        format!("Failed to get realm '{}' from Keycloak", realm_name)
                    });
                }
            }
        };

        if print_diff(
            "Realm",
            remote_realm.as_ref(),
            &local_realm,
            changes_only,
            "realm",
        )? {
            let mut include = true;
            if interactive {
                include =
                    dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
                        .with_prompt("Include this change in the plan?")
                        .default(true)
                        .interact()?;
            }
            if include {
                changed_files.push(realm_path);
            }
        }
    }
    Ok(())
}
