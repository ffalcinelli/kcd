use serde_json::Value;
use std::collections::HashMap;
use std::env;

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
pub fn substitute_secrets(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (_, v) in map.iter_mut() {
                substitute_secrets(v);
            }
        }
        Value::Array(arr) => {
            for v in arr.iter_mut() {
                substitute_secrets(v);
            }
        }
        Value::String(s) => {
            if s.starts_with("${") && s.ends_with("}") {
                let var_name = &s[2..s.len() - 1];
                if let Ok(env_val) = env::var(var_name) {
                    *s = env_val;
                }
            }
        }
        _ => {}
    }
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
    fn test_substitute_secrets() {
        unsafe {
            std::env::set_var("MY_TEST_SECRET", "actual_value");
        }
        let mut val = json!({
            "clientSecret": "${MY_TEST_SECRET}"
        });
        substitute_secrets(&mut val);
        assert_eq!(val["clientSecret"], "actual_value");
    }

    #[test]
    fn test_obfuscate_secrets() {
        let mut val = json!({
            "clientSecret": "supersecret"
        });
        obfuscate_secrets(&mut val);
        assert_eq!(val["clientSecret"], "s***t");
    }
}
