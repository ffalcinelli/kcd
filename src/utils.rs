pub mod secrets;
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
            // Sort arrays of simple values (Strings, Numbers, Bools)
            if !arr.is_empty() && arr.iter().all(|v| v.is_string() || v.is_number() || v.is_boolean()) {
                arr.sort_by(|a, b| {
                    let s_a = a.to_string();
                    let s_b = b.to_string();
                    s_a.cmp(&s_b)
                });
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
    let mut json_value = serde_json::to_value(value).context("Failed to serialize to JSON value")?;
    recursive_sort(&mut json_value);
    serde_yaml::to_string(&json_value).context("Failed to serialize to sorted YAML")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use serde_json::Value;
    use std::collections::HashMap;

    #[derive(Serialize, Deserialize, Debug)]
    struct TestStruct {
        b: String,
        a: String,
        #[serde(flatten)]
        extra: HashMap<String, Value>,
        list: Vec<InnerStruct>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    struct InnerStruct {
        z: String,
        y: String,
    }

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
}
