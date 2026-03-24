use crate::client::KeycloakClient;
use crate::models::RealmRepresentation;
use crate::utils::secrets::substitute_secrets;
use anyhow::{Context, Result};
use console::style;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs as async_fs;

use super::SUCCESS_UPDATE;

pub async fn apply_realm(
    client: &KeycloakClient,
    workspace_dir: &std::path::Path,
    env_vars: Arc<HashMap<String, String>>,
    planned_files: Arc<Option<HashSet<PathBuf>>>,
) -> Result<()> {
    // 1. Apply Realm
    let realm_path = workspace_dir.join("realm.yaml");
    if let Some(plan) = &*planned_files
        && !plan.contains(&realm_path)
    {
        return Ok(());
    }
    if async_fs::try_exists(&realm_path).await? {
        let content = async_fs::read_to_string(&realm_path).await?;
        let mut val: serde_json::Value = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse YAML file: {:?}", realm_path))?;
        substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
        let realm_rep: RealmRepresentation = serde_json::from_value(val)?;
        client
            .update_realm(&realm_rep)
            .await
            .context("Failed to update realm")?;
        println!(
            "  {} {}",
            SUCCESS_UPDATE,
            style("Updated realm configuration").cyan()
        );
    }
    Ok(())
}
