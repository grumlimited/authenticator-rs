use crate::main_window::MainWindow;
use chrono::prelude::*;
use chrono::Local;
use gtk::prelude::*;
use gtk::Builder;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

pub struct AccountsWindow {
    pub main_box: gtk::Box,
    pub edit_account: gtk::Box,
    pub stack: gtk::Stack,
    pub accounts_container: gtk::Box,
    pub progress_bar: Arc<Mutex<RefCell<gtk::ProgressBar>>>,
}

impl AccountsWindow {
    pub fn new(builder: Builder) -> AccountsWindow {
        let progress_bar: gtk::ProgressBar = builder.get_object("progress_bar").unwrap();
        let main_box: gtk::Box = builder.get_object("main_box").unwrap();
        let edit_account: gtk::Box = builder.get_object("edit_account").unwrap();
        let stack: gtk::Stack = builder.get_object("stack").unwrap();
        let accounts_container: gtk::Box = builder.get_object("accounts_container").unwrap();

        progress_bar.set_fraction(Self::progress_bar_fraction());

        AccountsWindow {
            main_box,
            edit_account,
            stack,
            accounts_container,
            progress_bar: Arc::new(Mutex::new(RefCell::new(progress_bar))),
        }
    }

    pub fn edit_buttons_actions(gui: &mut MainWindow) {
        for group_widgets in &mut gui.widgets {
            for account_widgets in &mut group_widgets.account_widgets {
                let id = account_widgets.id.clone();
                let popover = account_widgets.popover.clone();

                let main_box = gui.accounts_window.main_box.clone();
                let edit_account = gui.accounts_window.edit_account.clone();

                account_widgets.edit_button.connect_clicked(move |x| {
                    popover.hide();
                    main_box.set_visible(false);
                    edit_account.set_visible(true);
                });
            }
        }
    }

    pub fn progress_bar_fraction() -> f64 {
        Self::progress_bar_fraction_for(Local::now().second())
    }

    fn progress_bar_fraction_for(second: u32) -> f64 {
        (1_f64 - ((second % 30) as f64 / 30_f64)) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_bar_fraction() {
        assert_eq!(
            0.5333333333333333_f64,
            AccountsWindow::progress_bar_fraction_for(14)
        );
    }
}
