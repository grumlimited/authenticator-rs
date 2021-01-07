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
            container: builder.get_object("errors").unwrap(),
            error_display_message: builder.get_object("error_display_message").unwrap(),
        }
    }
}
