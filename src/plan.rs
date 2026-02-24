use crate::client::KeycloakClient;
use crate::models::{RealmRepresentation, ClientRepresentation, RoleRepresentation, IdentityProviderRepresentation};
use anyhow::Result;
use std::path::PathBuf;
use std::fs;
use std::collections::HashMap;
use similar::{ChangeTag, TextDiff};
use console::{Style, Emoji};
use serde::Serialize;

pub async fn run(client: &KeycloakClient, input_dir: PathBuf) -> Result<()> {
    println!("{} Planning changes for realm: {}", Emoji("🔮", ""), client.target_realm);

    // 1. Plan Realm
    plan_realm(client, &input_dir).await?;

    // 2. Plan Roles
    plan_roles(client, &input_dir).await?;

    // 3. Plan Clients
    plan_clients(client, &input_dir).await?;

    // 4. Plan Identity Providers
    plan_identity_providers(client, &input_dir).await?;

    Ok(())
}

fn print_diff<T: Serialize>(name: &str, old: Option<&T>, new: &T) -> Result<()> {
    let old_yaml = if let Some(o) = old {
        serde_yaml::to_string(o)?
    } else {
        String::new()
    };
    let new_yaml = serde_yaml::to_string(new)?;

    let diff = TextDiff::from_lines(&old_yaml, &new_yaml);

    if diff.ratio() < 1.0 {
        println!("\n{} Changes for {}:", Emoji("📝", ""), name);
        for change in diff.iter_all_changes() {
            let (sign, style) = match change.tag() {
                ChangeTag::Delete => ("-", Style::new().red()),
                ChangeTag::Insert => ("+", Style::new().green()),
                ChangeTag::Equal => (" ", Style::new().dim()),
            };
            print!("{}{}", style.apply_to(sign).bold(), style.apply_to(change));
        }
    } else {
         println!("{} No changes for {}", Emoji("✅", ""), name);
    }
    Ok(())
}

async fn plan_realm(client: &KeycloakClient, input_dir: &PathBuf) -> Result<()> {
    let realm_path = input_dir.join("realm.yaml");
    if realm_path.exists() {
        let content = fs::read_to_string(&realm_path)?;
        let local_realm: RealmRepresentation = serde_yaml::from_str(&content)?;

        // We handle the case where remote realm fetch might fail (e.g. if we are creating it)
        // by treating it as None (creation). However, usually plan is run against existing realm.
        // If get_realm fails, it might be an error or not exist.
        // For plan, we assume if it fails, it might not exist or we can't access it.
        // Let's try to fetch it.
        let remote_realm = client.get_realm().await.ok();

        print_diff("Realm", remote_realm.as_ref(), &local_realm)?;
    }
    Ok(())
}

async fn plan_roles(client: &KeycloakClient, input_dir: &PathBuf) -> Result<()> {
    let roles_dir = input_dir.join("roles");
    if roles_dir.exists() {
        let existing_roles = client.get_roles().await.unwrap_or_default();
        let existing_roles_map: HashMap<String, RoleRepresentation> = existing_roles
            .into_iter()
            .map(|r| (r.name.clone(), r))
            .collect();

        for entry in fs::read_dir(&roles_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "yaml") {
                let content = fs::read_to_string(&path)?;
                let local_role: RoleRepresentation = serde_yaml::from_str(&content)?;

                let remote_role = existing_roles_map.get(&local_role.name);

                if let Some(remote) = remote_role {
                     let mut remote_clone = remote.clone();
                     // Ignore ID differences if local doesn't specify it
                     if local_role.id.is_none() {
                         remote_clone.id = None;
                         remote_clone.container_id = None;
                     }
                     print_diff(&format!("Role {}", local_role.name), Some(&remote_clone), &local_role)?;
                } else {
                    println!("\n{} Will create Role: {}", Emoji("✨", ""), local_role.name);
                    print_diff(&format!("Role {}", local_role.name), None::<&RoleRepresentation>, &local_role)?;
                }
            }
        }
    }
    Ok(())
}

async fn plan_clients(client: &KeycloakClient, input_dir: &PathBuf) -> Result<()> {
    let clients_dir = input_dir.join("clients");
    if clients_dir.exists() {
        let existing_clients = client.get_clients().await.unwrap_or_default();
        let existing_clients_map: HashMap<String, ClientRepresentation> = existing_clients
            .into_iter()
            .filter_map(|c| c.client_id.clone().map(|id| (id, c)))
            .collect();

        for entry in fs::read_dir(&clients_dir)? {
             let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "yaml") {
                let content = fs::read_to_string(&path)?;
                let local_client: ClientRepresentation = serde_yaml::from_str(&content)?;
                let client_id = local_client.client_id.as_deref().unwrap_or("");

                if client_id.is_empty() { continue; }

                if let Some(remote) = existing_clients_map.get(client_id) {
                     let mut remote_clone = remote.clone();
                     if local_client.id.is_none() {
                         remote_clone.id = None;
                     }
                     print_diff(&format!("Client {}", client_id), Some(&remote_clone), &local_client)?;
                } else {
                     println!("\n{} Will create Client: {}", Emoji("✨", ""), client_id);
                     print_diff(&format!("Client {}", client_id), None::<&ClientRepresentation>, &local_client)?;
                }
            }
        }
    }
    Ok(())
}

async fn plan_identity_providers(client: &KeycloakClient, input_dir: &PathBuf) -> Result<()> {
    let idps_dir = input_dir.join("identity-providers");
    if idps_dir.exists() {
        let existing_idps = client.get_identity_providers().await.unwrap_or_default();
        let existing_idps_map: HashMap<String, IdentityProviderRepresentation> = existing_idps
            .into_iter()
            .filter_map(|i| i.alias.clone().map(|alias| (alias, i)))
            .collect();

        for entry in fs::read_dir(&idps_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "yaml") {
                 let content = fs::read_to_string(&path)?;
                 let local_idp: IdentityProviderRepresentation = serde_yaml::from_str(&content)?;
                 let alias = local_idp.alias.as_deref().unwrap_or("");

                 if alias.is_empty() { continue; }

                 if let Some(remote) = existing_idps_map.get(alias) {
                      let mut remote_clone = remote.clone();
                      if local_idp.internal_id.is_none() {
                          remote_clone.internal_id = None;
                      }
                      print_diff(&format!("IdentityProvider {}", alias), Some(&remote_clone), &local_idp)?;
                 } else {
                      println!("\n{} Will create IdentityProvider: {}", Emoji("✨", ""), alias);
                      print_diff(&format!("IdentityProvider {}", alias), None::<&IdentityProviderRepresentation>, &local_idp)?;
                 }
            }
        }
    }
    Ok(())
}
