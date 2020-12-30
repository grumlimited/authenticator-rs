use gtk::prelude::*;
use gtk::Builder;

#[derive(Clone, Debug)]
pub struct NoAccountsWindow {
    pub container: gtk::Box,
}

impl NoAccountsWindow {
    pub fn new(builder: Builder) -> NoAccountsWindow {
        NoAccountsWindow {
            container: builder.get_object("no_accounts").unwrap(),
        }
    }
}
