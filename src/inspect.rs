use crate::client::KeycloakClient;
use anyhow::{Context, Result};
use sanitize_filename::sanitize;
use std::fs;
use std::path::PathBuf;

pub async fn run(client: &KeycloakClient, output_dir: PathBuf) -> Result<()> {
    if !output_dir.exists() {
        fs::create_dir_all(&output_dir).context("Failed to create output directory")?;
    }

    // Fetch realm
    let realm = client.get_realm().await.context("Failed to fetch realm")?;
    let realm_yaml = serde_yaml::to_string(&realm).context("Failed to serialize realm")?;
    fs::write(output_dir.join("realm.yaml"), realm_yaml).context("Failed to write realm.yaml")?;
    println!("Exported realm configuration to realm.yaml");

    // Fetch clients
    let clients = client
        .get_clients()
        .await
        .context("Failed to fetch clients")?;
    let clients_dir = output_dir.join("clients");
    if !clients_dir.exists() {
        fs::create_dir_all(&clients_dir).context("Failed to create clients directory")?;
    }
    for client_rep in clients {
        let name = client_rep.client_id.as_deref().unwrap_or("unknown");
        let filename = format!("{}.yaml", sanitize(name));
        let path = clients_dir.join(filename);
        let yaml = serde_yaml::to_string(&client_rep).context("Failed to serialize client")?;
        fs::write(&path, yaml).context(format!("Failed to write client {}", name))?;
    }
    println!("Exported clients to clients/");

    // Fetch roles
    let roles = client.get_roles().await.context("Failed to fetch roles")?;
    let roles_dir = output_dir.join("roles");
    if !roles_dir.exists() {
        fs::create_dir_all(&roles_dir).context("Failed to create roles directory")?;
    }
    for role in roles {
        let name = &role.name;
        let filename = format!("{}.yaml", sanitize(name));
        let path = roles_dir.join(filename);
        let yaml = serde_yaml::to_string(&role).context("Failed to serialize role")?;
        fs::write(&path, yaml).context(format!("Failed to write role {}", name))?;
    }
    println!("Exported roles to roles/");

    Ok(())
}
