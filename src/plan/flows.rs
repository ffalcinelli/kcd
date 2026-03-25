use crate::client::KeycloakClient;
use crate::models::{AuthenticationFlowRepresentation, KeycloakResource};
use crate::utils::secrets::substitute_secrets;
use crate::utils::ui::SPARKLE;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs as async_fs;

use super::print_diff;

pub async fn plan_authentication_flows(
    client: &KeycloakClient,
    workspace_dir: &Path,
    changes_only: bool,
    interactive: bool,
    env_vars: Arc<HashMap<String, String>>,
    changed_files: &mut Vec<PathBuf>,
    realm_name: &str,
) -> Result<()> {
    let flows_dir = workspace_dir.join("authentication-flows");
    if async_fs::try_exists(&flows_dir).await? {
        let existing_flows = client.get_authentication_flows().await.with_context(|| {
            format!(
                "Failed to get authentication flows for realm '{}'",
                realm_name
            )
        })?;
        let existing_flows_map: HashMap<String, AuthenticationFlowRepresentation> = existing_flows
            .into_iter()
            .filter_map(|f| f.get_identity().map(|id| (id, f)))
            .collect();
        let existing_flows_map = Arc::new(existing_flows_map);

        let mut set = tokio::task::JoinSet::new();
        let mut entries = async_fs::read_dir(&flows_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let env_vars = env_vars.clone();
                let existing_flows_map = existing_flows_map.clone();

                set.spawn(async move {
                    let content = async_fs::read_to_string(&path).await?;
                    let mut val: serde_json::Value = serde_yaml::from_str(&content)
                        .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                    substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
                    let local_flow: AuthenticationFlowRepresentation = serde_json::from_value(val)
                        .with_context(|| format!("Failed to deserialize YAML file: {:?}", path))?;

                    let identity = local_flow
                        .get_identity()
                        .context(format!("Failed to get identity for flow in {:?}", path))?;
                    let remote = existing_flows_map.get(&identity).cloned();

                    Ok::<
                        (
                            AuthenticationFlowRepresentation,
                            PathBuf,
                            Option<AuthenticationFlowRepresentation>,
                        ),
                        anyhow::Error,
                    >((local_flow, path, remote))
                });
            }
        }

        while let Some(res) = set.join_next().await {
            let (local_flow, path, remote) = res??;

            let changed = if let Some(remote) = remote {
                let mut remote_clone = remote.clone();
                if local_flow.id.is_none() {
                    remote_clone.id = None;
                }
                print_diff(
                    &format!("AuthenticationFlow {}", local_flow.get_name()),
                    Some(&remote_clone),
                    &local_flow,
                    changes_only,
                    "flow",
                )?
            } else {
                println!(
                    "\n{} Will create AuthenticationFlow: {}",
                    SPARKLE,
                    local_flow.get_name()
                );
                print_diff(
                    &format!("AuthenticationFlow {}", local_flow.get_name()),
                    None::<&AuthenticationFlowRepresentation>,
                    &local_flow,
                    changes_only,
                    "flow",
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
