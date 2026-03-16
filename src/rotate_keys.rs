use crate::client::KeycloakClient;
use anyhow::{Context, Result};
use std::io::{self, Write};
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn run(client: &KeycloakClient, yes: bool, realms_to_consider: &[String]) -> Result<()> {
    let realms = if realms_to_consider.is_empty() {
        let all_realms = client
            .get_realms()
            .await
            .context("Failed to fetch realms")?;
        all_realms.into_iter().map(|r| r.realm).collect()
    } else {
        realms_to_consider.to_vec()
    };

    for realm_name in realms {
        let mut realm_client = client.clone();
        realm_client.set_target_realm(realm_name.clone());
        println!("Rotating keys for realm: {}", realm_name);
        rotate_keys_for_realm(&realm_client, yes).await?;
    }
    Ok(())
}

async fn rotate_keys_for_realm(client: &KeycloakClient, yes: bool) -> Result<()> {
    let keys_metadata = match client.get_keys().await {
        Ok(km) => km,
        Err(e) => {
            println!("Failed to fetch keys metadata (skipping): {}", e);
            return Ok(());
        }
    };

    let components = match client.get_components().await {
        Ok(c) => c,
        Err(e) => {
            println!("Failed to fetch components (skipping): {}", e);
            return Ok(());
        }
    };

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let thirty_days = 30 * 24 * 60 * 60 * 1000;

    let keys = keys_metadata.keys.unwrap_or_default();

    // Rotate keys that are currently ACTIVE
    for key in &keys {
        if key.status.as_deref() == Some("ACTIVE") {
            let provider_id = key.provider_id.as_deref().unwrap_or("unknown");

            // Check if it's near expiration
            let mut should_rotate = false;
            if let Some(valid_to) = key.valid_to {
                #[allow(clippy::collapsible_if)]
                if valid_to > 0 && valid_to - now < thirty_days {
                    should_rotate = true;
                }
            }

            if should_rotate {
                println!(
                    "Active key (providerId: {}) is near expiration. Rotating...",
                    provider_id
                );

                // Find corresponding component
                if let Some(component) = components
                    .iter()
                    .find(|c| c.id.as_deref() == Some(provider_id))
                {
                    let mut new_component = component.clone();
                    new_component.id = None;
                    let old_name = new_component
                        .name
                        .clone()
                        .unwrap_or_else(|| "key".to_string());
                    let timestamp = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    new_component.name = Some(format!("{}-rotated-{}", old_name, timestamp));

                    // Priority is normally an attribute in config
                    #[allow(clippy::collapsible_if)]
                    if let Some(config) = &mut new_component.config {
                        if let Some(priority_vals) = config.get_mut("priority") {
                            // Bump priority to make it the new active key
                            if let Some(arr) = priority_vals.as_array_mut() {
                                if let Some(first) = arr.first_mut() {
                                    if let Some(p_str) = first.as_str() {
                                        if let Ok(p_num) = p_str.parse::<i64>() {
                                            *first =
                                                serde_json::Value::String((p_num + 10).to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if let Err(e) = client.create_component(&new_component).await {
                        println!("Failed to create rotated key component: {}", e);
                    } else {
                        println!(
                            "Successfully created rotated key component: {}",
                            new_component.name.as_ref().unwrap()
                        );
                    }
                } else {
                    println!(
                        "Could not find corresponding component for providerId: {}",
                        provider_id
                    );
                }
            }
        }
    }

    // Handle old keys
    for key in &keys {
        let is_expired = key.valid_to.is_some_and(|vt| vt > 0 && vt < now);
        let is_disabled = key.status.as_deref() == Some("DISABLED");

        if is_expired || is_disabled {
            let provider_id = key.provider_id.as_deref().unwrap_or("unknown");
            println!(
                "Found old key (providerId: {}, status: {}, expired: {})",
                provider_id,
                key.status.as_deref().unwrap_or("unknown"),
                is_expired
            );

            let should_delete = if yes {
                true
            } else {
                print!("Delete this key? [y/N]: ");
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                input.trim().eq_ignore_ascii_case("y")
            };

            if should_delete {
                match client.delete_component(provider_id).await {
                    Ok(_) => println!("Deleted component {}", provider_id),
                    Err(e) => println!("Failed to delete component {}: {}", provider_id, e),
                }
            } else {
                println!("Kept component {}", provider_id);
            }
        }
    }

    Ok(())
}
