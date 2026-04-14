use crate::models::GroupRepresentation;
use crate::utils::ui::Ui;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

pub async fn create_group_interactive(workspace_dir: &Path, ui: &dyn Ui) -> Result<()> {
    let realm = ui.input("Target Realm", None, false)?;
    let name = ui.input("Group Name", None, false)?;

    create_group_yaml(workspace_dir, &realm, &name).await?;

    ui.print_success(&format!(
        "Successfully generated YAML for group '{}' in realm '{}'.",
        name, realm
    ));
    Ok(())
}

pub async fn create_group_yaml(workspace_dir: &Path, realm: &str, name: &str) -> Result<()> {
    let group = GroupRepresentation {
        id: None,
        name: Some(name.to_string()),
        path: None,
        sub_groups: None,
        extra: HashMap::new(),
    };

    let realm_dir = workspace_dir.join(realm).join("groups");
    fs::create_dir_all(&realm_dir)
        .await
        .context("Failed to create groups directory")?;

    let file_path = realm_dir.join(format!("{}.yaml", name));
    let yaml = serde_yaml::to_string(&group).context("Failed to serialize group to YAML")?;

    fs::write(&file_path, yaml)
        .await
        .context("Failed to write group YAML file")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_create_group_yaml() {
        let dir = tempdir().unwrap();
        let workspace_dir = dir.path();

        create_group_yaml(workspace_dir, "master", "my-group")
            .await
            .unwrap();

        let file_path = workspace_dir
            .join("master")
            .join("groups")
            .join("my-group.yaml");
        assert!(file_path.exists());

        let content = fs::read_to_string(&file_path).await.unwrap();
        let group: GroupRepresentation = serde_yaml::from_str(&content).unwrap();
        assert_eq!(group.name.as_deref(), Some("my-group"));
    }
}
