pub mod secrets;
pub mod ui;
use anyhow::Context;
use serde::Serialize;
use std::path::Path;
use tokio::fs;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use tokio::os::unix::fs::OpenOptionsExt;

pub async fn write_secure(path: &Path, content: &str) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use tokio::io::AsyncWriteExt;

        // If file exists, ensure permissions are 0o600
        if fs::try_exists(path).await.unwrap_or(false) {
            fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
                .await
                .context(format!("Failed to set permissions for {:?}", path))?;
        }

        let mut options = fs::OpenOptions::new();
        options
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600);

        let mut file = options
            .open(path)
            .await
            .context(format!("Failed to open {:?}", path))?;
        file.write_all(content.as_bytes())
            .await
            .context(format!("Failed to write to {:?}", path))?;
        file.flush()
            .await
            .context(format!("Failed to flush {:?}", path))?;
    }
    #[cfg(not(unix))]
    {
        fs::write(path, content)
            .await
            .context(format!("Failed to write {:?}", path))?;
    }
    Ok(())
}

pub fn recursive_sort(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for (_, v) in map.iter_mut() {
                recursive_sort(v);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr.iter_mut() {
                recursive_sort(v);
            }
            if arr.is_empty() {
                return;
            }

            // Sort arrays of simple values (Strings, Numbers, Bools)
            if arr
                .iter()
                .all(|v| v.is_string() || v.is_number() || v.is_boolean())
            {
                arr.sort_by(|a, b| {
                    let s_a = a.to_string();
                    let s_b = b.to_string();
                    s_a.cmp(&s_b)
                });
            } else if arr.iter().all(|v| v.is_object()) {
                // Try to find a common sorting key: id, alias, or name
                let keys = ["id", "alias", "name"];
                for key in keys {
                    if arr.iter().all(|v| v.get(key).is_some()) {
                        arr.sort_by(|a, b| {
                            let v_a = a.get(key).map_or(String::new(), |v| v.to_string());
                            let v_b = b.get(key).map_or(String::new(), |v| v.to_string());
                            v_a.cmp(&v_b)
                        });
                        break;
                    }
                }
            }
        }
        _ => {}
    }
}

pub fn to_sorted_yaml_with_secrets<T: Serialize>(
    value: &T,
    prefix: &str,
    secrets: &mut std::collections::HashMap<String, String>,
) -> anyhow::Result<String> {
    let mut json_value =
        serde_json::to_value(value).context("Failed to serialize to JSON value")?;
    crate::utils::secrets::extract_secrets(&mut json_value, prefix, secrets);
    recursive_sort(&mut json_value);
    serde_yaml::to_string(&json_value).context("Failed to serialize to sorted YAML")
}

pub fn to_sorted_yaml<T: Serialize>(value: &T) -> anyhow::Result<String> {
    let mut json_value =
        serde_json::to_value(value).context("Failed to serialize to JSON value")?;
    recursive_sort(&mut json_value);
    serde_yaml::to_string(&json_value).context("Failed to serialize to sorted YAML")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sorting_with_value() {
        let val = serde_json::json!({
            "z": "val_z",
            "a": "val_a",
            "m": ["item3", "item1", "item2"]
        });

        let yaml = to_sorted_yaml(&val).expect("Failed to serialize yaml");
        println!("Generated YAML:\n{}", yaml);

        let lines: Vec<&str> = yaml.lines().collect();
        assert_eq!(lines[0], "a: val_a");
        assert_eq!(lines[1], "m:");
        assert_eq!(lines[2], "- item1");
        assert_eq!(lines[3], "- item2");
        assert_eq!(lines[4], "- item3");
        assert_eq!(lines[5], "z: val_z");
    }

    #[test]
    fn test_sorting_arrays_of_objects() {
        let val = serde_json::json!({
            "list": [
                { "name": "c", "v": 3 },
                { "name": "a", "v": 1 },
                { "name": "b", "v": 2 }
            ],
            "aliases": [
                { "alias": "z" },
                { "alias": "x" }
            ],
            "ids": [
                { "id": "2" },
                { "id": "1" }
            ]
        });

        let yaml = to_sorted_yaml(&val).expect("Failed to serialize yaml");
        println!("Generated YAML:\n{}", yaml);

        let lines: Vec<&str> = yaml.lines().collect();
        // aliases sorted by alias
        assert_eq!(lines[0], "aliases:");
        assert_eq!(lines[1], "- alias: x");
        assert_eq!(lines[2], "- alias: z");
        // ids sorted by id
        assert_eq!(lines[3], "ids:");
        assert_eq!(lines[4], "- id: '1'");
        assert_eq!(lines[5], "- id: '2'");
        // list sorted by name
        assert_eq!(lines[6], "list:");
        assert_eq!(lines[7], "- name: a");
        assert_eq!(lines[8], "  v: 1");
        assert_eq!(lines[9], "- name: b");
        assert_eq!(lines[10], "  v: 2");
        assert_eq!(lines[11], "- name: c");
        assert_eq!(lines[12], "  v: 3");
    }

    #[test]
    fn test_recursive_sort_empty_array() {
        let mut val = serde_json::json!([]);
        recursive_sort(&mut val);
        assert_eq!(val, serde_json::json!([]));

        let mut val_obj = serde_json::json!({ "empty_arr": [] });
        recursive_sort(&mut val_obj);
        assert_eq!(val_obj, serde_json::json!({ "empty_arr": [] }));
    }

    #[test]
    fn test_recursive_sort_simple_arrays() {
        let mut val = serde_json::json!([3, 1, 2]);
        recursive_sort(&mut val);
        assert_eq!(val, serde_json::json!([1, 2, 3]));

        let mut val_bool = serde_json::json!([true, false, true]);
        recursive_sort(&mut val_bool);
        assert_eq!(val_bool, serde_json::json!([false, true, true]));

        let mut val_mixed = serde_json::json!(["b", 1, true, false, "a"]);
        recursive_sort(&mut val_mixed);
        // string representation: "b", "1", "true", "false", "a"
        // sorted by exact output of .to_string() for json Values:
        // "b" -> "\"b\"", 1 -> "1", true -> "true", false -> "false", "a" -> "\"a\""
        // sorted: "\"a\"", "\"b\"", "1", "false", "true"
        // meaning String("a"), String("b"), Number(1), Bool(false), Bool(true)
        // Note: Number strings ("1") come after String strings starting with double quotes ("\"a\"") in lexicographical order.
        assert_eq!(val_mixed, serde_json::json!(["a", "b", 1, false, true]));
    }

    #[test]
    fn test_recursive_sort_mixed_and_no_keys() {
        let mut val_mixed = serde_json::json!([{"a": 1}, 1, "string"]);
        recursive_sort(&mut val_mixed);
        assert_eq!(val_mixed, serde_json::json!([{"a": 1}, 1, "string"]));

        let mut val_no_keys = serde_json::json!([
            {"other": 2},
            {"other": 1}
        ]);
        recursive_sort(&mut val_no_keys);
        assert_eq!(
            val_no_keys,
            serde_json::json!([
                {"other": 2},
                {"other": 1}
            ])
        );
    }

    #[test]
    fn test_recursive_sort_nested_arrays() {
        let mut val = serde_json::json!({
            "nested": [[2, 1], [4, 3]]
        });
        recursive_sort(&mut val);
        // Elements in array might not be sortable (they are arrays), but inner should be
        assert_eq!(
            val,
            serde_json::json!({
                "nested": [[1, 2], [3, 4]]
            })
        );
    }

    #[test]
    fn test_recursive_sort_primitive() {
        let mut val_str = serde_json::json!("test");
        recursive_sort(&mut val_str);
        assert_eq!(val_str, serde_json::json!("test"));

        let mut val_num = serde_json::json!(42);
        recursive_sort(&mut val_num);
        assert_eq!(val_num, serde_json::json!(42));

        let mut val_null = serde_json::Value::Null;
        recursive_sort(&mut val_null);
        assert_eq!(val_null, serde_json::Value::Null);
    }

    #[test]
    fn test_recursive_sort_mixed_identify_keys() {
        // Scenario 1: Both "id" and "name" are present in all elements. Should sort by "id".
        let mut val1 = serde_json::json!([
            { "id": "2", "name": "a" },
            { "id": "1", "name": "b" }
        ]);
        recursive_sort(&mut val1);
        assert_eq!(
            val1,
            serde_json::json!([
                { "id": "1", "name": "b" },
                { "id": "2", "name": "a" }
            ])
        );

        // Scenario 2: "id" is only present in some elements, but "name" is present in all. Should sort by "name".
        let mut val2 = serde_json::json!([
            { "id": "1", "name": "b" },
            { "name": "a" }
        ]);
        recursive_sort(&mut val2);
        assert_eq!(
            val2,
            serde_json::json!([
                { "name": "a" },
                { "id": "1", "name": "b" }
            ])
        );

        // Scenario 3: "alias" is present in all, but "id" and "name" are missing or partially present. Should sort by "alias".
        let mut val3 = serde_json::json!([
            { "alias": "z", "name": "a" },
            { "alias": "x", "id": "1" }
        ]);
        recursive_sort(&mut val3);
        assert_eq!(
            val3,
            serde_json::json!([
                { "alias": "x", "id": "1" },
                { "alias": "z", "name": "a" }
            ])
        );
    }

    #[test]
    fn test_to_sorted_yaml_with_secrets() {
        let mut secrets = std::collections::HashMap::new();
        let val = serde_json::json!({
            "clientId": "myclient",
            "secret": "very-secret",
            "nested": {
                "password": "pass"
            }
        });

        let yaml = to_sorted_yaml_with_secrets(&val, "CLIENT", &mut secrets).unwrap();
        // current_prefix should be "CLIENT_myclient"
        // secret env var should be "KEYCLOAK_CLIENT_MYCLIENT_SECRET"
        // nested password env var should be "KEYCLOAK_CLIENT_MYCLIENT_NESTED_PASSWORD"
        assert!(yaml.contains("secret: ${KEYCLOAK_CLIENT_MYCLIENT_SECRET}"));
        assert!(yaml.contains("password: ${KEYCLOAK_CLIENT_MYCLIENT_NESTED_PASSWORD}"));
        assert_eq!(
            secrets.get("KEYCLOAK_CLIENT_MYCLIENT_SECRET"),
            Some(&"very-secret".to_string())
        );
        assert_eq!(
            secrets.get("KEYCLOAK_CLIENT_MYCLIENT_NESTED_PASSWORD"),
            Some(&"pass".to_string())
        );
    }

    #[test]
    fn test_to_sorted_yaml_simple() {
        let val = serde_json::json!({ "b": 2, "a": 1 });
        let yaml = to_sorted_yaml(&val).unwrap();
        assert_eq!(yaml.trim(), "a: 1\nb: 2");
    }

    #[tokio::test]
    async fn test_write_secure_permissions() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("secure.txt");
        let content = "sensitive data";

        // Test creating a new file
        write_secure(&file_path, content).await.unwrap();
        let read_content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(read_content, content);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(&file_path).unwrap();
            let mode = metadata.permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }

        // Test updating an existing file with insecure permissions
        let existing_path = temp_dir.path().join("existing.txt");
        std::fs::write(&existing_path, "old content").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&existing_path, std::fs::Permissions::from_mode(0o644)).unwrap();
            let metadata = std::fs::metadata(&existing_path).unwrap();
            assert_eq!(metadata.permissions().mode() & 0o777, 0o644);
        }

        write_secure(&existing_path, "new content").await.unwrap();
        let read_content = std::fs::read_to_string(&existing_path).unwrap();
        assert_eq!(read_content, "new content");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(&existing_path).unwrap();
            let mode = metadata.permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }
}
