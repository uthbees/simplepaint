use rfd::{MessageDialog, MessageLevel};

pub mod dimensions;

pub struct Utils {}

impl Utils {
    pub fn show_error_dialog(title: &str, description: &str) {
        MessageDialog::new()
            .set_level(MessageLevel::Error)
            .set_title(title)
            .set_description(description)
            .show();
    }
}
