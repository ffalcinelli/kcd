use crate::models::{ClientRepresentation, ClientScopeRepresentation};
use crate::utils::ui::SUCCESS_CREATE;
use anyhow::{Context, Result};
use console::style;
use dialoguer::{Confirm, Input, theme::ColorfulTheme};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

pub async fn create_client_interactive(workspace_dir: &Path) -> Result<()> {
    let theme = ColorfulTheme::default();

    let realm: String = Input::with_theme(&theme)
        .with_prompt("Target Realm")
        .interact_text()?;

    let client_id: String = Input::with_theme(&theme)
        .with_prompt("Client ID")
        .interact_text()?;

    let is_public = Confirm::with_theme(&theme)
        .with_prompt("Is this a public client? (No for confidential)")
        .default(true)
        .interact()?;

    create_client_yaml(workspace_dir, &realm, &client_id, is_public).await?;

    println!(
        "{} {}",
        SUCCESS_CREATE,
        style(format!(
            "Successfully generated YAML for client '{}' in realm '{}'.",
            client_id, realm
        ))
        .green()
    );
    Ok(())
}

pub async fn create_client_yaml(
    workspace_dir: &Path,
    realm: &str,
    client_id: &str,
    is_public: bool,
) -> Result<()> {
    let client = ClientRepresentation {
        id: None,
        client_id: Some(client_id.to_string()),
        name: None,
        description: None,
        enabled: Some(true),
        protocol: Some("openid-connect".to_string()),
        redirect_uris: Some(vec!["/*".to_string()]),
        web_origins: Some(vec!["+".to_string()]),
        public_client: Some(is_public),
        bearer_only: Some(false),
        service_accounts_enabled: Some(!is_public),
        extra: HashMap::new(),
    };

    let realm_dir = workspace_dir.join(realm).join("clients");
    fs::create_dir_all(&realm_dir)
        .await
        .context("Failed to create clients directory")?;

    let file_path = realm_dir.join(format!("{}.yaml", client_id));
    let yaml = serde_yaml::to_string(&client).context("Failed to serialize client to YAML")?;

    fs::write(&file_path, yaml)
        .await
        .context("Failed to write client YAML file")?;

    Ok(())
}

pub async fn create_client_scope_interactive(workspace_dir: &Path) -> Result<()> {
    let theme = ColorfulTheme::default();

    let realm: String = Input::with_theme(&theme)
        .with_prompt("Target Realm")
        .interact_text()?;

    let name: String = Input::with_theme(&theme)
        .with_prompt("Scope Name")
        .interact_text()?;

    let protocol: String = Input::with_theme(&theme)
        .with_prompt("Protocol")
        .default("openid-connect".to_string())
        .interact_text()?;

    create_client_scope_yaml(workspace_dir, &realm, &name, &protocol).await?;

    println!(
        "{} {}",
        SUCCESS_CREATE,
        style(format!(
            "Successfully generated YAML for client scope '{}' in realm '{}'.",
            name, realm
        ))
        .green()
    );
    Ok(())
}

pub async fn create_client_scope_yaml(
    workspace_dir: &Path,
    realm: &str,
    name: &str,
    protocol: &str,
) -> Result<()> {
    let scope = ClientScopeRepresentation {
        id: None,
        name: Some(name.to_string()),
        description: None,
        protocol: Some(protocol.to_string()),
        attributes: Some(HashMap::new()),
        extra: HashMap::new(),
    };

    let scopes_dir = workspace_dir.join(realm).join("client-scopes");
    fs::create_dir_all(&scopes_dir)
        .await
        .context("Failed to create client-scopes directory")?;

    let file_path = scopes_dir.join(format!("{}.yaml", name));
    let yaml = serde_yaml::to_string(&scope).context("Failed to serialize client scope to YAML")?;

    fs::write(&file_path, yaml)
        .await
        .context("Failed to write client scope YAML file")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_create_client_yaml() {
        let dir = tempdir().unwrap();
        let workspace_dir = dir.path();

        create_client_yaml(workspace_dir, "master", "testclient", true)
            .await
            .unwrap();

        let file_path = workspace_dir
            .join("master")
            .join("clients")
            .join("testclient.yaml");
        assert!(file_path.exists());

        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        let client: ClientRepresentation = serde_yaml::from_str(&content).unwrap();

        assert_eq!(client.client_id.as_deref(), Some("testclient"));
        assert_eq!(client.public_client, Some(true));
        assert_eq!(client.service_accounts_enabled, Some(false));
    }

    #[tokio::test]
    async fn test_create_client_scope_yaml() {
        let dir = tempdir().unwrap();
        let workspace_dir = dir.path();

        create_client_scope_yaml(workspace_dir, "master", "my-scope", "openid-connect")
            .await
            .unwrap();
        let file_path = workspace_dir
            .join("master")
            .join("client-scopes")
            .join("my-scope.yaml");
        assert!(file_path.exists());
    }
}
