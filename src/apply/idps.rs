use crate::client::KeycloakClient;
use crate::models::{IdentityProviderRepresentation, KeycloakResource};
use crate::utils::secrets::{SecretResolver, substitute_secrets};
use crate::utils::ui::{SUCCESS_CREATE, SUCCESS_UPDATE};
use anyhow::{Context, Result};
use console::style;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs as async_fs;
use tokio::task::JoinSet;

pub async fn apply_identity_providers(
    client: &KeycloakClient,
    workspace_dir: &std::path::Path,
    resolver: Arc<dyn SecretResolver>,
    planned_files: Arc<Option<HashSet<PathBuf>>>,
    realm_name: &str,
) -> Result<()> {
    // 4. Apply Identity Providers
    let idps_dir = workspace_dir.join("identity-providers");
    if async_fs::try_exists(&idps_dir).await? {
        let existing_idps = client.get_identity_providers().await.with_context(|| {
            format!(
                "Failed to get identity providers for realm '{}'",
                realm_name
            )
        })?;
        let existing_idps_map: HashMap<String, IdentityProviderRepresentation> = existing_idps
            .into_iter()
            .filter_map(|i| i.get_identity().map(|id| (id, i)))
            .collect();
        let existing_idps_map = std::sync::Arc::new(existing_idps_map);

        let mut entries = async_fs::read_dir(&idps_dir).await?;
        let mut set = JoinSet::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(plan) = &*planned_files
                && !plan.contains(&path)
            {
                continue;
            }
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let client = client.clone();
                let existing_idps_map = existing_idps_map.clone();
                let resolver = Arc::clone(&resolver);
                let realm_name = realm_name.to_string();
                set.spawn(async move {
                    let content = async_fs::read_to_string(&path).await?;
                    let mut val: serde_json::Value = serde_yaml::from_str(&content)
                        .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                    substitute_secrets(&mut val, Arc::clone(&resolver)).await?;
                    let mut idp_rep: IdentityProviderRepresentation = serde_json::from_value(val)?;

                    let identity = idp_rep
                        .get_identity()
                        .context(format!("Failed to get identity for IDP in {:?}", path))?;

                    if let Some(existing) = existing_idps_map.get(&identity) {
                        if let Some(internal_id) = &existing.internal_id {
                            idp_rep.internal_id = Some(internal_id.clone());
                            client
                                .update_identity_provider(&identity, &idp_rep)
                                .await
                                .with_context(|| {
                                    format!(
                                        "Failed to update identity provider '{}' in realm '{}'",
                                        idp_rep.get_name(),
                                        realm_name
                                    )
                                })?;
                            println!(
                                "  {} {}",
                                SUCCESS_UPDATE,
                                style(format!("Updated identity provider {}", idp_rep.get_name()))
                                    .cyan()
                            );
                        }
                    } else {
                        idp_rep.internal_id = None;
                        client
                            .create_identity_provider(&idp_rep)
                            .await
                            .with_context(|| {
                                format!(
                                    "Failed to create identity provider '{}' in realm '{}'",
                                    idp_rep.get_name(),
                                    realm_name
                                )
                            })?;
                        println!(
                            "  {} {}",
                            SUCCESS_CREATE,
                            style(format!("Created identity provider {}", idp_rep.get_name()))
                                .green()
                        );
                    }
                    Ok::<(), anyhow::Error>(())
                });
            }
        }
        while let Some(res) = set.join_next().await {
            res??;
        }
    }
    Ok(())
}
