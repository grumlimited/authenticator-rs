use gtk::prelude::*;
use gtk::Builder;

#[derive(Clone, Debug)]
pub struct NoAccountsWindow {
    pub container: gtk::Box,
    pub no_accounts_plus_sign: gtk::EventBox,
}

impl NoAccountsWindow {
    pub fn new(builder: Builder) -> NoAccountsWindow {
        NoAccountsWindow {
            container: builder.object("no_accounts").unwrap(),
            no_accounts_plus_sign: builder.object("no_accounts_plus_sign").unwrap(),
        }
    }
}
