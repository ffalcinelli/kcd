use kcd::utils::ui::{DialoguerUi, Ui};

#[test]
fn test_dialoguer_ui_prints() {
    let ui = DialoguerUi;
    ui.print_info("info");
    ui.print_success("success");
    ui.print_error("error");
    ui.print_warn("warn");
}
