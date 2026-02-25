use app::models::{RealmRepresentation, RoleRepresentation};
use app::validate;
use std::fs;
use tempfile::tempdir;

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
    fs::write(
        input_dir.join("realm.yaml"),
        serde_yaml::to_string(&realm).unwrap(),
    )
    .unwrap();

    let result = validate::run(input_dir.clone());
    assert!(result.is_ok());
}

#[test]
fn test_validate_empty_role_name() {
    let dir = tempdir().unwrap();
    let input_dir = dir.path().to_path_buf();

    // Create valid realm.yaml
    let realm = RealmRepresentation {
        realm: "test-realm".to_string(),
        enabled: Some(true),
        display_name: None,
        extra: std::collections::HashMap::new(),
    };
    fs::write(
        input_dir.join("realm.yaml"),
        serde_yaml::to_string(&realm).unwrap(),
    )
    .unwrap();

    // Create roles directory
    let roles_dir = input_dir.join("roles");
    fs::create_dir(&roles_dir).unwrap();

    // Create role with empty name
    let role = RoleRepresentation {
        id: None,
        name: "".to_string(),
        description: None,
        container_id: None,
        composite: false,
        client_role: false,
        extra: std::collections::HashMap::new(),
    };
    fs::write(
        roles_dir.join("role.yaml"),
        serde_yaml::to_string(&role).unwrap(),
    )
    .unwrap();

    let result = validate::run(input_dir.clone());
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Role name is empty")
    );
}

#[test]
fn test_validate_duplicate_role_name() {
    let dir = tempdir().unwrap();
    let input_dir = dir.path().to_path_buf();

    // Create valid realm.yaml
    let realm = RealmRepresentation {
        realm: "test-realm".to_string(),
        enabled: Some(true),
        display_name: None,
        extra: std::collections::HashMap::new(),
    };
    fs::write(
        input_dir.join("realm.yaml"),
        serde_yaml::to_string(&realm).unwrap(),
    )
    .unwrap();

    // Create roles directory
    let roles_dir = input_dir.join("roles");
    fs::create_dir(&roles_dir).unwrap();

    // Create first role
    let role1 = RoleRepresentation {
        id: None,
        name: "admin".to_string(),
        description: None,
        container_id: None,
        composite: false,
        client_role: false,
        extra: std::collections::HashMap::new(),
    };
    fs::write(
        roles_dir.join("role1.yaml"),
        serde_yaml::to_string(&role1).unwrap(),
    )
    .unwrap();

    // Create second role with same name
    let role2 = RoleRepresentation {
        id: None,
        name: "admin".to_string(),
        description: None,
        container_id: None,
        composite: false,
        client_role: false,
        extra: std::collections::HashMap::new(),
    };
    fs::write(
        roles_dir.join("role2.yaml"),
        serde_yaml::to_string(&role2).unwrap(),
    )
    .unwrap();

    let result = validate::run(input_dir.clone());
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Duplicate role name: admin")
    );
}

#[test]
fn test_validate_missing_realm() {
    let dir = tempdir().unwrap();
    let input_dir = dir.path().to_path_buf();

    let result = validate::run(input_dir.clone());
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("realm.yaml not found")
    );
}

#[test]
fn test_validate_empty_realm_name() {
    let dir = tempdir().unwrap();
    let input_dir = dir.path().to_path_buf();

    // Create realm.yaml with empty name
    let realm = RealmRepresentation {
        realm: "".to_string(),
        enabled: Some(true),
        display_name: None,
        extra: std::collections::HashMap::new(),
    };
    fs::write(
        input_dir.join("realm.yaml"),
        serde_yaml::to_string(&realm).unwrap(),
    )
    .unwrap();

    let result = validate::run(input_dir.clone());
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Realm name is empty in realm.yaml")
    );
}
