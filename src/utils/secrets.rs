use serde_json::Value;
use std::collections::HashMap;

/// Heuristics to identify a secret key based on its name.
fn is_secret_key(key: &str) -> bool {
    let lower_key = key.to_lowercase();
    lower_key.contains("secret") || lower_key.contains("password") || lower_key == "value"
}

/// Recursively extract secrets and replace them with ${ENV_VAR}
pub fn extract_secrets(value: &mut Value, prefix: &str, secrets: &mut HashMap<String, String>) {
    match value {
        Value::Object(map) => {
            let mut keys_to_update = Vec::new();

            for (k, v) in map.iter_mut() {
                if v.is_string() && is_secret_key(k) {
                    keys_to_update.push(k.clone());
                } else if v.is_object() || v.is_array() {
                    let new_prefix = if prefix.is_empty() {
                        k.clone()
                    } else {
                        format!("{}_{}", prefix, k)
                    };
                    extract_secrets(v, &new_prefix, secrets);
                }
            }

            for k in keys_to_update {
                if let Some(Value::String(s)) = map.get_mut(&k) {
                    let mut env_var_name = if prefix.is_empty() {
                        format!("KEYCLOAK_{}", k)
                    } else {
                        format!("KEYCLOAK_{}_{}", prefix, k)
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
pub fn obfuscate_secrets(value: &mut Value) {
    match value {
        Value::Object(map) => {
            let mut keys_to_obfuscate = Vec::new();

            for (k, v) in map.iter_mut() {
                if v.is_string() && is_secret_key(k) {
                    keys_to_obfuscate.push(k.clone());
                } else if v.is_object() || v.is_array() {
                    obfuscate_secrets(v);
                }
            }

            for k in keys_to_obfuscate {
                if let Some(Value::String(s)) = map.get_mut(&k) {
                    *s = obfuscate_string(s);
                }
            }
        }
        Value::Array(arr) => {
            for v in arr.iter_mut() {
                obfuscate_secrets(v);
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
            "clientSecret": "my_super_secret",
            "name": "my_client"
        });
        let mut secrets = HashMap::new();
        extract_secrets(&mut val, "client", &mut secrets);

        assert_eq!(val["clientSecret"], "${KEYCLOAK_CLIENT_CLIENTSECRET}");
        assert_eq!(
            secrets.get("KEYCLOAK_CLIENT_CLIENTSECRET"),
            Some(&"my_super_secret".to_string())
        );
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
            "clientSecret": "s1"
        });
        let mut secrets = HashMap::new();
        extract_secrets(&mut val, "", &mut secrets);

        assert_eq!(val["clientSecret"], "${KEYCLOAK_CLIENTSECRET}");
        assert_eq!(
            secrets.get("KEYCLOAK_CLIENTSECRET"),
            Some(&"s1".to_string())
        );
    }

    #[test]
    fn test_extract_secrets_sanitization() {
        let mut val = json!({
            "client-secret": "s1",
            "db.password": "p1"
        });
        let mut secrets = HashMap::new();
        extract_secrets(&mut val, "app", &mut secrets);

        assert_eq!(val["client-secret"], "${KEYCLOAK_APP_CLIENT_SECRET}");
        assert_eq!(val["db.password"], "${KEYCLOAK_APP_DB_PASSWORD}");

        assert_eq!(
            secrets.get("KEYCLOAK_APP_CLIENT_SECRET"),
            Some(&"s1".to_string())
        );
        assert_eq!(
            secrets.get("KEYCLOAK_APP_DB_PASSWORD"),
            Some(&"p1".to_string())
        );
    }

    #[test]
    fn test_extract_secrets_nested() {
        let mut val = json!({
            "level1": {
                "password": "p1",
                "level2": [
                    { "secret": "s1" },
                    { "other": "v1" }
                ]
            }
        });
        let mut secrets = HashMap::new();
        extract_secrets(&mut val, "root", &mut secrets);

        assert_eq!(
            val["level1"]["password"],
            "${KEYCLOAK_ROOT_LEVEL1_PASSWORD}"
        );
        assert_eq!(
            val["level1"]["level2"][0]["secret"],
            "${KEYCLOAK_ROOT_LEVEL1_LEVEL2_0_SECRET}"
        );
        assert_eq!(val["level1"]["level2"][1]["other"], "v1");

        assert_eq!(
            secrets.get("KEYCLOAK_ROOT_LEVEL1_PASSWORD"),
            Some(&"p1".to_string())
        );
        assert_eq!(
            secrets.get("KEYCLOAK_ROOT_LEVEL1_LEVEL2_0_SECRET"),
            Some(&"s1".to_string())
        );
    }

    #[test]
    fn test_substitute_secrets() {
        let mut env_vars = HashMap::new();
        env_vars.insert("MY_TEST_SECRET".to_string(), "actual_value".to_string());

        let mut val = json!({
            "clientSecret": "${MY_TEST_SECRET}"
        });
        substitute_secrets(&mut val, &env_vars).unwrap();
        assert_eq!(val["clientSecret"], "actual_value");
    }

    #[test]
    fn test_substitute_secrets_missing() {
        let env_vars = HashMap::new();
        let mut val = json!({
            "clientSecret": "${MISSING_VAR}"
        });
        let res = substitute_secrets(&mut val, &env_vars);
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            "Missing required environment variable: MISSING_VAR"
        );
    }

    #[test]
    fn test_obfuscate_secrets() {
        let mut val = json!({
            "clientSecret": "supersecret"
        });
        obfuscate_secrets(&mut val);
        assert_eq!(val["clientSecret"], "s***t");
    }

    #[test]
    fn test_is_secret_key() {
        assert!(is_secret_key("secret"));
        assert!(is_secret_key("password"));
        assert!(is_secret_key("value"));
        assert!(is_secret_key("SECRET"));
        assert!(is_secret_key("Password"));
        assert!(is_secret_key("VALUE"));
        assert!(is_secret_key("clientSecret"));
        assert!(is_secret_key("db_password"));

        assert!(!is_secret_key(""));
        assert!(!is_secret_key("username"));
        assert!(!is_secret_key("id"));
    }

    #[test]
    fn test_obfuscate_string() {
        assert_eq!(obfuscate_string(""), "");
        assert_eq!(obfuscate_string("a"), "***");
        assert_eq!(obfuscate_string("ab"), "***");
        assert_eq!(obfuscate_string("abc"), "***");
        assert_eq!(obfuscate_string("abcd"), "a***d");
        assert_eq!(obfuscate_string("secret"), "s***t");
    }
}
