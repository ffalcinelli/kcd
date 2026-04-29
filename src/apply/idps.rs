use crate::client::KeycloakClient;
use crate::models::{IdentityProviderRepresentation, KeycloakResource};
use crate::utils::secrets::{SecretResolver, substitute_secrets};
use anyhow::{Context, Result};
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

                    crate::handle_upsert! {
                        client: client,
                        realm: realm_name,
                        rep: idp_rep,
                        id_opt: existing_idps_map.get(&identity).and_then(|e| e.internal_id.as_ref()),
                        id_field: internal_id,
                        resource_name: "identity provider",
                        update_call: |id, rep| client.update_identity_provider(&identity, rep),
                        create_call: |rep| client.create_identity_provider(rep)
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
