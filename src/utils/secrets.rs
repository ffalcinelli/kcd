use serde_json::Value;
use std::collections::HashMap;

/// Heuristics to identify a secret key based on its name.
fn is_secret_key(key: &str, prefix: &str) -> bool {
    let lower_key = key.to_lowercase();

    // Blacklist common false positives in Keycloak configuration
    if lower_key.contains("policy")
        || lower_key.contains("passwordless")
        || lower_key.contains("creation")
        || lower_key.contains("delivery")
    {
        return false;
    }

    if lower_key.contains("secret") || lower_key.contains("password") || lower_key.contains("token")
    {
        return true;
    }

    if lower_key == "value" {
        let lower_prefix = prefix.to_lowercase();
        return lower_prefix.contains("credential")
            || lower_prefix.contains("secret")
            || lower_prefix.contains("password")
            || lower_prefix.contains("token");
    }

    false
}

/// Heuristics to identify if a string looks like a boolean or simple toggle.
fn is_boolean_string(s: &str) -> bool {
    let lower = s.to_lowercase();
    lower == "true" || lower == "false" || lower == "on" || lower == "off"
}

/// Recursively extract secrets and replace them with ${ENV_VAR}
pub fn extract_secrets(value: &mut Value, prefix: &str, secrets: &mut HashMap<String, String>) {
    match value {
        Value::Object(map) => {
            let mut keys_to_update = Vec::new();

            // Try to find an identifier for this object to make secret names better
            let id = map
                .get("clientId")
                .or_else(|| map.get("username"))
                .or_else(|| map.get("alias"))
                .or_else(|| map.get("name"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let current_prefix = if let Some(id_str) = id {
                if prefix.is_empty() {
                    id_str
                } else {
                    format!("{}_{}", prefix, id_str)
                }
            } else {
                prefix.to_string()
            };

            for (k, v) in map.iter_mut() {
                if let Value::String(s) = v {
                    if is_secret_key(k, &current_prefix) && !is_boolean_string(s) {
                        keys_to_update.push(k.clone());
                    }
                } else if v.is_object() || v.is_array() {
                    let new_prefix = if current_prefix.is_empty() {
                        k.clone()
                    } else {
                        format!("{}_{}", current_prefix, k)
                    };
                    extract_secrets(v, &new_prefix, secrets);
                }
            }

            for k in keys_to_update {
                if let Some(Value::String(s)) = map.get_mut(&k) {
                    let mut env_var_name = if current_prefix.is_empty() {
                        format!("KEYCLOAK_{}", k)
                    } else {
                        format!("KEYCLOAK_{}_{}", current_prefix, k)
                    };
                    env_var_name = env_var_name
                        .chars()
                        .map(|c| {
                            if c.is_alphanumeric() {
                                c.to_ascii_uppercase()
                            } else {
                                '_'
                            }
                        })
                        .collect();
                    secrets.insert(env_var_name.clone(), s.clone());
                    *s = format!("${{{}}}", env_var_name);
                }
            }
        }
        Value::Array(arr) => {
            for (i, v) in arr.iter_mut().enumerate() {
                let new_prefix = format!("{}_{}", prefix, i);
                extract_secrets(v, &new_prefix, secrets);
            }
        }
        _ => {}
    }
}

/// Recursively substitute ${ENV_VAR} with actual values
pub fn substitute_secrets(
    value: &mut Value,
    env_vars: &HashMap<String, String>,
) -> Result<(), String> {
    match value {
        Value::Object(map) => {
            for (_, v) in map.iter_mut() {
                substitute_secrets(v, env_vars)?;
            }
        }
        Value::Array(arr) => {
            for v in arr.iter_mut() {
                substitute_secrets(v, env_vars)?;
            }
        }
        Value::String(s) => {
            if s.starts_with("${") && s.ends_with("}") {
                let var_name = &s[2..s.len() - 1];
                if var_name.starts_with("KEYCLOAK_") {
                    if let Some(env_val) = env_vars.get(var_name) {
                        *s = env_val.clone();
                    } else {
                        return Err(format!(
                            "Missing required environment variable: {}",
                            var_name
                        ));
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

/// Helper to obfuscate a single string
fn obfuscate_string(s: &str) -> String {
    if s.is_empty() {
        return s.to_string();
    }

    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= 3 {
        return "***".to_string();
    }
    let first = chars[0];
    let last = chars[chars.len() - 1];
    format!("{}***{}", first, last)
}

/// Recursively obfuscate known secret fields
pub fn obfuscate_secrets(value: &mut Value, prefix: &str) {
    match value {
        Value::Object(map) => {
            let mut keys_to_obfuscate = Vec::new();

            // Try to find an identifier for this object to make secret identification better
            let id = map
                .get("clientId")
                .or_else(|| map.get("username"))
                .or_else(|| map.get("alias"))
                .or_else(|| map.get("name"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let current_prefix = if let Some(id_str) = id {
                if prefix.is_empty() {
                    id_str
                } else {
                    format!("{}_{}", prefix, id_str)
                }
            } else {
                prefix.to_string()
            };

            for (k, v) in map.iter_mut() {
                if v.is_string() && is_secret_key(k, &current_prefix) {
                    keys_to_obfuscate.push(k.clone());
                } else if v.is_object() || v.is_array() {
                    let new_prefix = if current_prefix.is_empty() {
                        k.clone()
                    } else {
                        format!("{}_{}", current_prefix, k)
                    };
                    obfuscate_secrets(v, &new_prefix);
                }
            }

            for k in keys_to_obfuscate {
                if let Some(Value::String(s)) = map.get_mut(&k) {
                    *s = obfuscate_string(s);
                }
            }
        }
        Value::Array(arr) => {
            for (i, v) in arr.iter_mut().enumerate() {
                let new_prefix = format!("{}_{}", prefix, i);
                obfuscate_secrets(v, &new_prefix);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_secrets() {
        let mut val = json!({
            "clientId": "my_client",
            "clientSecret": "my_super_secret",
            "storeToken": "true"
        });
        let mut secrets = HashMap::new();
        extract_secrets(&mut val, "client", &mut secrets);

        assert_eq!(
            val["clientSecret"],
            "${KEYCLOAK_CLIENT_MY_CLIENT_CLIENTSECRET}"
        );
        assert_eq!(val["storeToken"], "true");
        assert_eq!(
            secrets.get("KEYCLOAK_CLIENT_MY_CLIENT_CLIENTSECRET"),
            Some(&"my_super_secret".to_string())
        );
        assert!(!secrets.contains_key("KEYCLOAK_CLIENT_STORETOKEN"));
    }

    #[test]
    fn test_obfuscate_secrets_nested() {
        let mut val = json!({
            "name": "root",
            "level1": {
                "name": "l1",
                "password": "superpassword",
                "level2": [
                    { "name": "idx0", "secret": "verysecret" },
                    { "other": "v1" }
                ]
            }
        });
        obfuscate_secrets(&mut val, "");

        assert_eq!(val["level1"]["password"], "s***d");
        assert_eq!(val["level1"]["level2"][0]["secret"], "v***t");
        assert_eq!(val["level1"]["level2"][1]["other"], "v1");
    }

    #[test]
    fn test_obfuscate_secrets_array() {
        let mut val = json!([
            { "clientSecret": "secret1" },
            { "password": "password1" },
            "not_a_secret"
        ]);
        obfuscate_secrets(&mut val, "prefix");

        assert_eq!(val[0]["clientSecret"], "s***1");
        assert_eq!(val[1]["password"], "p***1");
        assert_eq!(val[2], "not_a_secret");
    }

    #[test]
    fn test_obfuscate_secrets_non_string() {
        let mut val = json!({
            "secret": 123,
            "password": true,
            "token": null
        });
        obfuscate_secrets(&mut val, "prefix");

        assert_eq!(val["secret"], 123);
        assert_eq!(val["password"], true);
        assert_eq!(val["token"], json!(null));
    }

    #[test]
    fn test_obfuscate_secrets_value_field() {
        let mut val = json!({
            "value": "mycredential"
        });
        obfuscate_secrets(&mut val, "credential");
        assert_eq!(val["value"], "m***l");

        let mut val_not_secret = json!({
            "value": "some_value"
        });
        obfuscate_secrets(&mut val_not_secret, "name");
        assert_eq!(val_not_secret["value"], "some_value");
    }

    #[test]
    fn test_extract_secrets_non_string() {
        let mut val = json!({
            "secret": 123
        });
        let mut secrets = HashMap::new();
        extract_secrets(&mut val, "prefix", &mut secrets);

        assert_eq!(val["secret"], 123);
        assert!(secrets.is_empty());
    }

    #[test]
    fn test_extract_secrets_no_prefix() {
        let mut val = json!({
            "clientId": "web",
            "clientSecret": "s1"
        });
        let mut secrets = HashMap::new();
        extract_secrets(&mut val, "", &mut secrets);

        assert_eq!(val["clientSecret"], "${KEYCLOAK_WEB_CLIENTSECRET}");
        assert_eq!(
            secrets.get("KEYCLOAK_WEB_CLIENTSECRET"),
            Some(&"s1".to_string())
        );
    }

    #[test]
    fn test_extract_secrets_sanitization() {
        let mut val = json!({
            "name": "my-app",
            "client-secret": "s1",
            "db.password": "p1"
        });
        let mut secrets = HashMap::new();
        extract_secrets(&mut val, "app", &mut secrets);

        assert_eq!(val["client-secret"], "${KEYCLOAK_APP_MY_APP_CLIENT_SECRET}");
        assert_eq!(val["db.password"], "${KEYCLOAK_APP_MY_APP_DB_PASSWORD}");

        assert_eq!(
            secrets.get("KEYCLOAK_APP_MY_APP_CLIENT_SECRET"),
            Some(&"s1".to_string())
        );
        assert_eq!(
            secrets.get("KEYCLOAK_APP_MY_APP_DB_PASSWORD"),
            Some(&"p1".to_string())
        );
    }

    #[test]
    fn test_extract_secrets_nested() {
        let mut val = json!({
            "name": "root",
            "level1": {
                "name": "l1",
                "password": "p1",
                "level2": [
                    { "name": "idx0", "secret": "s1" },
                    { "other": "v1" }
                ]
            }
        });
        let mut secrets = HashMap::new();
        extract_secrets(&mut val, "", &mut secrets);

        assert_eq!(
            val["level1"]["password"],
            "${KEYCLOAK_ROOT_LEVEL1_L1_PASSWORD}"
        );
        assert_eq!(
            val["level1"]["level2"][0]["secret"],
            "${KEYCLOAK_ROOT_LEVEL1_L1_LEVEL2_0_IDX0_SECRET}"
        );
        assert_eq!(val["level1"]["level2"][1]["other"], "v1");

        assert_eq!(
            secrets.get("KEYCLOAK_ROOT_LEVEL1_L1_PASSWORD"),
            Some(&"p1".to_string())
        );
        assert_eq!(
            secrets.get("KEYCLOAK_ROOT_LEVEL1_L1_LEVEL2_0_IDX0_SECRET"),
            Some(&"s1".to_string())
        );
    }

    #[test]
    fn test_substitute_secrets() {
        let mut env_vars = HashMap::new();
        env_vars.insert(
            "KEYCLOAK_TEST_SECRET".to_string(),
            "actual_value".to_string(),
        );

        let mut val = json!({
            "clientSecret": "${KEYCLOAK_TEST_SECRET}",
            "other": "${NOT_A_SECRET}"
        });
        substitute_secrets(&mut val, &env_vars).unwrap();
        assert_eq!(val["clientSecret"], "actual_value");
        assert_eq!(val["other"], "${NOT_A_SECRET}");
    }

    #[test]
    fn test_substitute_secrets_missing() {
        let env_vars = HashMap::new();
        let mut val = json!({
            "clientSecret": "${KEYCLOAK_MISSING_VAR}"
        });
        let res = substitute_secrets(&mut val, &env_vars);
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            "Missing required environment variable: KEYCLOAK_MISSING_VAR"
        );
    }

    #[test]
    fn test_substitute_secrets_complex() {
        let mut env_vars = HashMap::new();
        env_vars.insert("KEYCLOAK_VAR1".to_string(), "val1".to_string());
        env_vars.insert("KEYCLOAK_VAR2".to_string(), "val2".to_string());

        let mut val = json!({
            "arr": ["${KEYCLOAK_VAR1}", "normal", "${KEYCLOAK_VAR2}"],
            "nested": {
                "deep_arr": [
                    { "secret": "${KEYCLOAK_VAR1}" }
                ]
            }
        });

        substitute_secrets(&mut val, &env_vars).unwrap();

        assert_eq!(val["arr"][0], "val1");
        assert_eq!(val["arr"][1], "normal");
        assert_eq!(val["arr"][2], "val2");
        assert_eq!(val["nested"]["deep_arr"][0]["secret"], "val1");
    }

    #[test]
    fn test_substitute_secrets_edge_cases() {
        let mut env_vars = HashMap::new();
        env_vars.insert("KEYCLOAK_VAR".to_string(), "val".to_string());

        let mut val = json!({
            "no_braces": "KEYCLOAK_VAR",
            "not_at_start": "prefix ${KEYCLOAK_VAR}",
            "not_at_end": "${KEYCLOAK_VAR} suffix",
            "empty_braces": "${}",
            "no_closing": "${KEYCLOAK_VAR",
            "not_keycloak": "${OTHER_VAR}"
        });

        let original = val.clone();
        substitute_secrets(&mut val, &env_vars).unwrap();

        // None of these should have been substituted based on the current logic
        assert_eq!(val, original);

        // Test "${KEYCLOAK_}" explicitly as it should trigger a missing env var error
        let mut val_fail = json!({ "only_prefix": "${KEYCLOAK_}" });
        let res = substitute_secrets(&mut val_fail, &env_vars);
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            "Missing required environment variable: KEYCLOAK_"
        );
    }

    #[test]
    fn test_substitute_secrets_nested_missing() {
        let env_vars = HashMap::new();
        let mut val = json!({
            "nested": {
                "arr": [
                    "normal",
                    { "secret": "${KEYCLOAK_MISSING}" }
                ]
            }
        });

        let res = substitute_secrets(&mut val, &env_vars);
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            "Missing required environment variable: KEYCLOAK_MISSING"
        );
    }

    #[test]
    fn test_obfuscate_secrets() {
        let mut val = json!({
            "clientSecret": "supersecret"
        });
        obfuscate_secrets(&mut val, "client");
        assert_eq!(val["clientSecret"], "s***t");
    }

    #[test]
    fn test_is_secret_key() {
        assert!(is_secret_key("secret", ""));
        assert!(is_secret_key("password", ""));
        assert!(is_secret_key("token", ""));
        assert!(is_secret_key("value", "credential"));
        assert!(is_secret_key("value", "secret"));
        assert!(is_secret_key("SECRET", ""));
        assert!(is_secret_key("Password", ""));
        assert!(is_secret_key("TOKEN", ""));
        assert!(is_secret_key("clientSecret", ""));
        assert!(is_secret_key("db_password", ""));

        assert!(!is_secret_key("passwordPolicy", ""));
        assert!(!is_secret_key("webAuthnPolicyPasswordless", ""));
        assert!(!is_secret_key("createdAt", ""));
        assert!(!is_secret_key("deliveryMode", ""));
        assert!(!is_secret_key("value", "name"));
        assert!(!is_secret_key("", ""));
        assert!(!is_secret_key("username", ""));
        assert!(!is_secret_key("id", ""));
    }

    #[test]
    fn test_obfuscate_string() {
        assert_eq!(obfuscate_string(""), "");
        assert_eq!(obfuscate_string("a"), "***");
        assert_eq!(obfuscate_string("ab"), "***");
        assert_eq!(obfuscate_string("abc"), "***");
        assert_eq!(obfuscate_string("abcd"), "a***d");
        assert_eq!(obfuscate_string("secret"), "s***t");
        assert_eq!(obfuscate_string("🦀"), "***");
        assert_eq!(obfuscate_string("🦀🦀"), "***");
        assert_eq!(obfuscate_string("🦀🦀🦀"), "***");
        assert_eq!(obfuscate_string("🦀🦀🦀🦀"), "🦀***🦀");
        assert_eq!(obfuscate_string("a🦀"), "***");
        assert_eq!(obfuscate_string("a🦀b"), "***");
        assert_eq!(obfuscate_string("a🦀b🦀"), "a***🦀");
        assert_eq!(obfuscate_string("   "), "***");
        assert_eq!(obfuscate_string("    "), " *** ");
    }

    #[test]
    fn test_is_boolean_string() {
        assert!(is_boolean_string("true"));
        assert!(is_boolean_string("false"));
        assert!(is_boolean_string("on"));
        assert!(is_boolean_string("off"));
        assert!(is_boolean_string("TRUE"));
        assert!(is_boolean_string("FALSE"));
        assert!(is_boolean_string("ON"));
        assert!(is_boolean_string("OFF"));
        assert!(!is_boolean_string("yes"));
        assert!(!is_boolean_string("no"));
        assert!(!is_boolean_string("1"));
        assert!(!is_boolean_string("0"));
    }

    #[test]
    fn test_extract_secrets_array() {
        let mut val = json!([
            { "clientSecret": "secret1" },
            { "password": "password1" },
            "not_a_secret"
        ]);
        let mut secrets = HashMap::new();
        extract_secrets(&mut val, "prefix", &mut secrets);

        assert_eq!(val[0]["clientSecret"], "${KEYCLOAK_PREFIX_0_CLIENTSECRET}");
        assert_eq!(val[1]["password"], "${KEYCLOAK_PREFIX_1_PASSWORD}");
        assert_eq!(val[2], "not_a_secret");

        assert_eq!(
            secrets.get("KEYCLOAK_PREFIX_0_CLIENTSECRET"),
            Some(&"secret1".to_string())
        );
        assert_eq!(
            secrets.get("KEYCLOAK_PREFIX_1_PASSWORD"),
            Some(&"password1".to_string())
        );
    }

    #[test]
    fn test_extract_secrets_primitive() {
        let mut val = json!(true);
        let mut secrets = HashMap::new();
        extract_secrets(&mut val, "prefix", &mut secrets);

        assert_eq!(val, json!(true));
        assert!(secrets.is_empty());

        let mut val_null = json!(null);
        extract_secrets(&mut val_null, "prefix", &mut secrets);

        assert_eq!(val_null, json!(null));
        assert!(secrets.is_empty());
    }
}
