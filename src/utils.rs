pub mod secrets;
pub mod ui;
use anyhow::Context;
use serde::Serialize;

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
                            let v_a = a.get(key).unwrap().to_string();
                            let v_b = b.get(key).unwrap().to_string();
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
}
