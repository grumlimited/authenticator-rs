use gtk::prelude::*;
use gtk::Builder;

#[derive(Clone, Debug)]
pub struct ErrorsWindow {
    pub container: gtk::Box,
    pub error_display_message: gtk::TextBuffer,
}

impl ErrorsWindow {
    pub fn new(builder: Builder) -> ErrorsWindow {
        ErrorsWindow {
            container: builder.object("errors").unwrap(),
            error_display_message: builder.object("error_display_message").unwrap(),
        }
    }
}
