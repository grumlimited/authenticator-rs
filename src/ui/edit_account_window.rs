use crate::main_window::MainWindow;
use gtk::prelude::*;
use gtk::Builder;

pub struct EditAccountWindow {
    pub edit_account: gtk::Box,
    pub container: gtk::Box,
    pub input_group: gtk::Entry,
    pub input_name: gtk::Entry,
    pub input_secret: gtk::Entry,
    pub cancel_button: gtk::Button,
    pub save_button: gtk::Button,
}

impl EditAccountWindow {
    pub fn new(builder: Builder) -> EditAccountWindow {
        EditAccountWindow {
            edit_account: builder.get_object("edit_account").unwrap(),
            container: builder.get_object("add_accounts_container").unwrap(),
            input_group: builder.get_object("edit_account_input_group").unwrap(),
            input_name: builder.get_object("edit_account_input_name").unwrap(),
            input_secret: builder.get_object("edit_account_input_secret").unwrap(),
            cancel_button: builder.get_object("edit_account_cancel").unwrap(),
            save_button: builder.get_object("edit_account_save").unwrap(),
        }
    }

    pub fn edit_account_buttons_actions(gui: &mut MainWindow) {
        fn with_action<F>(gui: &mut MainWindow, button: gtk::Button, button_closure: F)
        where
            F: 'static
                + Fn(
                    gtk::Entry,
                    gtk::Entry,
                    gtk::Entry,
                    gtk::Box,
                    gtk::Box,
                ) -> Box<dyn Fn(&gtk::Button)>,
        {
            let main_box = gui.accounts_window.main_box.clone();
            let edit_account = gui.accounts_window.edit_account.clone();

            let group = gui.edit_account_window.input_group.clone();
            let name = gui.edit_account_window.input_name.clone();
            let secret = gui.edit_account_window.input_secret.clone();

            let button_closure = button_closure(group, name, secret, main_box, edit_account);

            button.connect_clicked(button_closure);
        }

        let edit_account_cancel = gui.edit_account_window.cancel_button.clone();
        with_action(
            gui,
            edit_account_cancel,
            |group, name, secret, main_box, edit_account| {
                Box::new(move |_| {
                    group.set_text("");
                    name.set_text("");
                    secret.set_text("");

                    main_box.set_visible(true);
                    edit_account.set_visible(false);
                })
            },
        );

        let edit_account_save = gui.edit_account_window.save_button.clone();
        with_action(
            gui,
            edit_account_save,
            |group, name, secret, main_box, edit_account| {
                Box::new(move |_| {
                    let entry = group.get_buffer().get_text();
                    println!("{:?}", entry);
                })
            },
        );
    }
}
