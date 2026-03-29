use crate::models::IdentityProviderRepresentation;
use crate::utils::ui::SUCCESS_CREATE;
use anyhow::{Context, Result};
use console::style;
use dialoguer::{Input, theme::ColorfulTheme};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

pub async fn create_idp_interactive(workspace_dir: &Path) -> Result<()> {
    let theme = ColorfulTheme::default();

    let realm: String = Input::with_theme(&theme)
        .with_prompt("Target Realm")
        .interact_text()?;

    let alias: String = Input::with_theme(&theme)
        .with_prompt("IDP Alias (e.g. google, github)")
        .interact_text()?;

    let provider_id: String = Input::with_theme(&theme)
        .with_prompt("Provider ID (e.g. oidc, saml, google)")
        .default(alias.clone())
        .interact_text()?;

    create_idp_yaml(workspace_dir, &realm, &alias, &provider_id).await?;

    println!(
        "{} {}",
        SUCCESS_CREATE,
        style(format!(
            "Successfully generated YAML for Identity Provider '{}' in realm '{}'.",
            alias, realm
        ))
        .green()
    );
    Ok(())
}

pub async fn create_idp_yaml(
    workspace_dir: &Path,
    realm: &str,
    alias: &str,
    provider_id: &str,
) -> Result<()> {
    let idp = IdentityProviderRepresentation {
        internal_id: None,
        alias: Some(alias.to_string()),
        provider_id: Some(provider_id.to_string()),
        enabled: Some(true),
        update_profile_first_login_mode: Some("on".to_string()),
        trust_email: Some(false),
        store_token: Some(false),
        add_read_token_role_on_create: Some(false),
        authenticate_by_default: Some(false),
        link_only: Some(false),
        first_broker_login_flow_alias: Some("first broker login".to_string()),
        post_broker_login_flow_alias: None,
        display_name: Some(alias.to_string()),
        config: Some(HashMap::new()),
        extra: HashMap::new(),
    };

    let realm_dir = workspace_dir.join(realm).join("identity-providers");
    fs::create_dir_all(&realm_dir)
        .await
        .context("Failed to create identity-providers directory")?;

    let file_path = realm_dir.join(format!("{}.yaml", alias));
    let yaml = serde_yaml::to_string(&idp).context("Failed to serialize IDP to YAML")?;

    fs::write(&file_path, yaml)
        .await
        .context("Failed to write IDP YAML file")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_create_idp_yaml() {
        let dir = tempdir().unwrap();
        let workspace_dir = dir.path();

        create_idp_yaml(workspace_dir, "master", "google", "google")
            .await
            .unwrap();

        let file_path = workspace_dir
            .join("master")
            .join("identity-providers")
            .join("google.yaml");
        assert!(file_path.exists());

        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        let idp: IdentityProviderRepresentation = serde_yaml::from_str(&content).unwrap();
        assert_eq!(idp.alias.as_deref(), Some("google"));
    }
}
