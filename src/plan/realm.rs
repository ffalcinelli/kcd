use crate::utils::secrets::substitute_secrets;
use anyhow::{Context, Result};
use std::path::PathBuf;
use tokio::fs as async_fs;

use super::{PlanContext, print_diff};

pub async fn plan_realm(
    ctx: &PlanContext<'_>,
    changed_files: &mut Vec<PathBuf>,
) -> Result<()> {
    let realm_path = ctx.workspace_dir.join("realm.yaml");
    if async_fs::try_exists(&realm_path).await? {
        let content = async_fs::read_to_string(&realm_path).await?;
        let mut val: serde_json::Value = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse YAML file: {:?}", realm_path))?;
        substitute_secrets(&mut val, &ctx.env_vars).map_err(|e| anyhow::anyhow!(e))?;
        let local_realm: crate::models::RealmRepresentation = serde_json::from_value(val)
            .with_context(|| format!("Failed to deserialize YAML file: {:?}", realm_path))?;

        // We handle the case where remote realm fetch might fail (e.g. if we are creating it)
        // by treating it as None (creation). However, usually plan is run against existing realm.
        let remote_realm = match ctx.client.get_realm().await {
            Ok(r) => Some(r),
            Err(e) => {
                // Check if it's a 404 (Not Found)
                if e.to_string().contains("404") {
                    None
                } else {
                    return Err(e).with_context(|| {
                        format!("Failed to get realm '{}' from Keycloak", ctx.realm_name)
                    });
                }
            }
        };

        if print_diff(
            "Realm",
            remote_realm.as_ref(),
            &local_realm,
            ctx.options.changes_only,
            "realm",
        )? {
            let mut include = true;
            if ctx.options.interactive {
                include = ctx.ui.confirm("Include this change in the plan?", true)?;
            }
            if include {
                changed_files.push(realm_path);
            }
        }
    }
    Ok(())
}
