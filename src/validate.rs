use crate::models::{RealmRepresentation, ClientRepresentation, RoleRepresentation, IdentityProviderRepresentation};
use anyhow::{Result, Context};
use serde::de::DeserializeOwned;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

fn read_yaml_files<T: DeserializeOwned>(dir: &Path, file_type: &str) -> Result<Vec<(PathBuf, T)>> {
    let mut results = Vec::new();
    if dir.exists() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "yaml") {
                let content = fs::read_to_string(&path)
                    .context(format!("Failed to read {} file {:?}", file_type, path))?;
                let item: T = serde_yaml::from_str(&content)
                    .context(format!("Failed to parse {} file {:?}", file_type, path))?;
                results.push((path, item));
            }
        }
    }
    Ok(results)
}

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
    let roles: Vec<(PathBuf, RoleRepresentation)> = read_yaml_files(&roles_dir, "role")?;

    for (path, role) in roles {
        if role.name.is_empty() {
            anyhow::bail!("Role name is empty in {:?}", path);
        }
        if role_names.contains(&role.name) {
            anyhow::bail!("Duplicate role name: {}", role.name);
        }
        role_names.insert(role.name.clone());
    }
    println!("Validated {} roles", role_names.len());

    // 3. Validate Clients
    let clients_dir = input_dir.join("clients");
    let clients: Vec<(PathBuf, ClientRepresentation)> = read_yaml_files(&clients_dir, "client")?;

    for (path, client) in clients {
        if client.client_id.is_none()
            || client
                .client_id
                .as_deref()
                .unwrap_or("")
                .is_empty()
        {
            anyhow::bail!("Client ID is missing or empty in {:?}", path);
        }
    }
    println!("Validated clients");

    // 4. Validate Identity Providers
    let idps_dir = input_dir.join("identity-providers");
    let idps: Vec<(PathBuf, IdentityProviderRepresentation)> =
        read_yaml_files(&idps_dir, "idp")?;

    for (path, idp) in idps {
        if idp.alias.is_none() || idp.alias.as_deref().unwrap_or("").is_empty() {
            anyhow::bail!(
                "Identity Provider alias is missing or empty in {:?}",
                path
            );
        }
        if idp.provider_id.is_none()
            || idp
                .provider_id
                .as_deref()
                .unwrap_or("")
                .is_empty()
        {
            anyhow::bail!(
                "Identity Provider providerId is missing or empty in {:?}",
                path
            );
        }
    }
    println!("Validated Identity Providers");

    Ok(())
}
