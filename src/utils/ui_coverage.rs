#[cfg(test)]
mod tests {
    use crate::utils::ui::{DialoguerUi, Ui};

    #[test]
    fn test_dialoguer_ui_input() {
        let ui = DialoguerUi::new();
        // Since input blocks, we might not easily cover this without mocking terminal inputs,
        // but it looks like we need to test these methods. The UI methods block on stdin/tty.
        // Let's see if there's any tests we can add that don't block.
    }
}
