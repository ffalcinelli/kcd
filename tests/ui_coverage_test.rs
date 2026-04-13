use kcd::utils::ui::{DialoguerUi, Ui, MockUi};
use std::sync::Mutex;

#[test]
fn test_dialoguer_ui_prints() {
    let ui = DialoguerUi;
    ui.print_info("info");
    ui.print_success("success");
    ui.print_error("error");
    ui.print_warn("warn");
}

#[test]
fn test_mock_ui_password() {
    let ui = MockUi {
        inputs: Mutex::new(vec![]),
        confirms: Mutex::new(vec![]),
        selects: Mutex::new(vec![]),
        passwords: Mutex::new(vec!["secret123".to_string()]),
    };
    let pass = ui.password("Prompt", None).unwrap();
    assert_eq!(pass, "secret123");
}

#[test]
fn test_mock_ui_errors() {
    let ui = MockUi {
        inputs: Mutex::new(vec![]),
        confirms: Mutex::new(vec![]),
        selects: Mutex::new(vec![]),
        passwords: Mutex::new(vec![]),
    };
    assert!(ui.input("p", None, false).is_err());
    assert!(ui.confirm("p", true).is_err());
    assert!(ui.select("p", &["a"], 0).is_err());
    assert!(ui.password("p", None).is_err());
}
