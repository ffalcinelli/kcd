use anyhow::Context;
use serde::Serialize;

pub fn to_sorted_yaml<T: Serialize>(value: &T) -> anyhow::Result<String> {
    let json_value = serde_json::to_value(value).context("Failed to serialize to JSON value")?;
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
    fn test_recursive_sorting() {
        let mut extra = HashMap::new();
        extra.insert("x".to_string(), Value::String("val_x".to_string()));

        let inner = InnerStruct {
            z: "val_z".to_string(),
            y: "val_y".to_string(),
        };

        let s = TestStruct {
            b: "val_b".to_string(),
            a: "val_a".to_string(),
            extra,
            list: vec![inner],
        };

        let yaml = to_sorted_yaml(&s).unwrap();

        let lines: Vec<&str> = yaml.lines().collect();

        // Expected Output:
        // a: val_a
        // b: val_b
        // list:
        // - y: val_y
        //   z: val_z
        // x: val_x

        assert_eq!(lines[0], "a: val_a");
        assert_eq!(lines[1], "b: val_b");
        assert_eq!(lines[2], "list:");
        assert_eq!(lines[3], "- y: val_y");
        assert_eq!(lines[4], "  z: val_z");
        assert_eq!(lines[5], "x: val_x");
    }
}
