use crate::models::RoleRepresentation;
use crate::utils::ui::SUCCESS_CREATE;
use anyhow::{Context, Result};
use console::style;
use dialoguer::{Confirm, Input, theme::ColorfulTheme};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

pub async fn create_role_interactive(workspace_dir: &Path) -> Result<()> {
    let theme = ColorfulTheme::default();

    let realm: String = Input::with_theme(&theme)
        .with_prompt("Target Realm")
        .interact_text()?;

    let name: String = Input::with_theme(&theme)
        .with_prompt("Role Name")
        .interact_text()?;

    let description: String = Input::with_theme(&theme)
        .with_prompt("Description")
        .allow_empty(true)
        .interact_text()?;

    let is_client_role = Confirm::with_theme(&theme)
        .with_prompt("Is this a client role?")
        .default(false)
        .interact()?;

    let client_id = if is_client_role {
        let id: String = Input::with_theme(&theme)
            .with_prompt("Client ID")
            .interact_text()?;
        Some(id)
    } else {
        None
    };

    let description_opt = if description.is_empty() {
        None
    } else {
        Some(description)
    };

    create_role_yaml(workspace_dir, &realm, &name, description_opt, client_id).await?;

    println!(
        "{} {}",
        SUCCESS_CREATE,
        style(format!(
            "Successfully generated YAML for role '{}' in realm '{}'.",
            name, realm
        ))
        .green()
    );
    Ok(())
}

pub async fn create_role_yaml(
    workspace_dir: &Path,
    realm: &str,
    name: &str,
    description: Option<String>,
    client_id: Option<String>,
) -> Result<()> {
    let role = RoleRepresentation {
        id: None,
        name: name.to_string(),
        description,
        container_id: None,
        composite: false,
        client_role: client_id.is_some(),
        extra: HashMap::new(),
    };

    let realm_dir = workspace_dir.join(realm);
    let roles_dir = if let Some(cid) = &client_id {
        realm_dir.join("clients").join(cid).join("roles")
    } else {
        realm_dir.join("roles")
    };

    fs::create_dir_all(&roles_dir)
        .await
        .context("Failed to create roles directory")?;

    let file_path = roles_dir.join(format!("{}.yaml", name));
    let yaml = serde_yaml::to_string(&role).context("Failed to serialize role to YAML")?;

    fs::write(&file_path, yaml)
        .await
        .context("Failed to write role YAML file")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_create_role_yaml() {
        let dir = tempdir().unwrap();
        let workspace_dir = dir.path();

        // Realm role
        create_role_yaml(
            workspace_dir,
            "master",
            "admin",
            Some("desc".to_string()),
            None,
        )
        .await
        .unwrap();
        let realm_role_path = workspace_dir
            .join("master")
            .join("roles")
            .join("admin.yaml");
        assert!(realm_role_path.exists());
        let content = tokio::fs::read_to_string(&realm_role_path).await.unwrap();
        let role: RoleRepresentation = serde_yaml::from_str(&content).unwrap();
        assert_eq!(role.name, "admin");
        assert!(!role.client_role);

        // Client role
        create_role_yaml(
            workspace_dir,
            "master",
            "editor",
            None,
            Some("my-client".to_string()),
        )
        .await
        .unwrap();
        let client_role_path = workspace_dir
            .join("master")
            .join("clients")
            .join("my-client")
            .join("roles")
            .join("editor.yaml");
        assert!(client_role_path.exists());
    }
}
