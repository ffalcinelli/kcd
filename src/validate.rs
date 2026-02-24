use crate::models::{RealmRepresentation, ClientRepresentation, RoleRepresentation, IdentityProviderRepresentation};
use anyhow::{Result, Context};
use std::path::PathBuf;
use std::fs;
use std::collections::HashSet;

pub fn run(input_dir: PathBuf) -> Result<()> {
    // 1. Validate Realm
    let realm_path = input_dir.join("realm.yaml");
    if !realm_path.exists() {
         anyhow::bail!("realm.yaml not found in {:?}", input_dir);
    }
    let realm_content = fs::read_to_string(&realm_path).context("Failed to read realm.yaml")?;
    let realm: RealmRepresentation = serde_yaml::from_str(&realm_content).context("Failed to parse realm.yaml")?;

    if realm.realm.is_empty() {
        anyhow::bail!("Realm name is empty in realm.yaml");
    }
    println!("Realm configuration is valid: {}", realm.realm);

    // 2. Validate Roles
    let roles_dir = input_dir.join("roles");
    let mut role_names = HashSet::new();
    if roles_dir.exists() {
        for entry in fs::read_dir(&roles_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "yaml") {
                let content = fs::read_to_string(&path).context(format!("Failed to read role file {:?}", path))?;
                let role: RoleRepresentation = serde_yaml::from_str(&content).context(format!("Failed to parse role file {:?}", path))?;

                if role.name.is_empty() {
                    anyhow::bail!("Role name is empty in {:?}", path);
                }
                if role_names.contains(&role.name) {
                     anyhow::bail!("Duplicate role name: {}", role.name);
                }
                role_names.insert(role.name.clone());
            }
        }
    }
    println!("Validated {} roles", role_names.len());

    // 3. Validate Clients
    let clients_dir = input_dir.join("clients");
    if clients_dir.exists() {
        for entry in fs::read_dir(&clients_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "yaml") {
                let content = fs::read_to_string(&path).context(format!("Failed to read client file {:?}", path))?;
                let client: ClientRepresentation = serde_yaml::from_str(&content).context(format!("Failed to parse client file {:?}", path))?;

                if client.client_id.is_none() || client.client_id.as_deref().unwrap_or("").is_empty() {
                     anyhow::bail!("Client ID is missing or empty in {:?}", path);
                }
            }
        }
    }
    println!("Validated clients");

    // 4. Validate Identity Providers
    let idps_dir = input_dir.join("identity-providers");
    if idps_dir.exists() {
        for entry in fs::read_dir(&idps_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "yaml") {
                let content = fs::read_to_string(&path).context(format!("Failed to read idp file {:?}", path))?;
                let idp: IdentityProviderRepresentation = serde_yaml::from_str(&content).context(format!("Failed to parse idp file {:?}", path))?;

                if idp.alias.is_none() || idp.alias.as_deref().unwrap_or("").is_empty() {
                     anyhow::bail!("Identity Provider alias is missing or empty in {:?}", path);
                }
                if idp.provider_id.is_none() || idp.provider_id.as_deref().unwrap_or("").is_empty() {
                     anyhow::bail!("Identity Provider providerId is missing or empty in {:?}", path);
                }
            }
        }
    }
    println!("Validated Identity Providers");

    Ok(())
}
