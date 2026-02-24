use app::validate;
use std::fs;
use tempfile::tempdir;
use app::models::RealmRepresentation;

#[test]
fn test_validate() {
    let dir = tempdir().unwrap();
    let input_dir = dir.path().to_path_buf();

    // Create valid realm.yaml
    let realm = RealmRepresentation {
        realm: "test-realm".to_string(),
        enabled: Some(true),
        display_name: None,
        extra: std::collections::HashMap::new(),
    };
    fs::write(input_dir.join("realm.yaml"), serde_yaml::to_string(&realm).unwrap()).unwrap();

    let result = validate::run(input_dir.clone());
    assert!(result.is_ok());
}

#[test]
fn test_validate_missing_realm() {
    let dir = tempdir().unwrap();
    let input_dir = dir.path().to_path_buf();

    let result = validate::run(input_dir.clone());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("realm.yaml not found"));
}
