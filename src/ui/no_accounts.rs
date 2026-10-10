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
            container: builder.object("no_accounts").expect("Missing 'no_accounts' container in UI file"),
            no_accounts_plus_sign: builder
                .object("no_accounts_plus_sign")
                .expect("Missing 'no_accounts_plus_sign' event box in UI file"),
        }
    }
}
