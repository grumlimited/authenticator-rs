use crate::exporting::Exporting;
use crate::main_window::{Display, MainWindow};
use crate::ui::AccountsWindow;
use crate::{NAMESPACE, NAMESPACE_PREFIX};
use gio::prelude::*;
use gtk::prelude::*;
use gtk::{Button, MenuButton};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub trait Menus {
    fn build_menus(&mut self, connection: Arc<Mutex<Connection>>);

    fn build_search_button(&mut self) -> gtk::Button;

    fn build_system_menu(&mut self, connection: Arc<Mutex<Connection>>) -> gtk::MenuButton;

    fn build_action_menu(&mut self, connection: Arc<Mutex<Connection>>) -> gtk::MenuButton;
}

impl Menus for MainWindow {
    fn build_menus(&mut self, connection: Arc<Mutex<Connection>>) {
        let titlebar = gtk::HeaderBarBuilder::new().show_close_button(true).build();

        titlebar.pack_start(&self.build_action_menu(connection.clone()));

        titlebar.pack_start(&self.build_search_button());

        titlebar.pack_end(&self.build_system_menu(connection));
        self.window.set_titlebar(Some(&titlebar));

        titlebar.show_all();
    }

    fn build_search_button(&mut self) -> Button {
        let builder = gtk::Builder::from_resource(format!("{}/{}", NAMESPACE_PREFIX, "system_menu.ui").as_str());
        let search_button: gtk::Button = builder.get_object("search_button").unwrap();

        let filter = self.accounts_window.filter.clone();
        search_button.connect_clicked(move |_| {
            if filter.is_visible() {
                filter.hide()
            } else {
                filter.show();
                filter.grab_focus()
            }

            gio::Settings::new(NAMESPACE)
                .set_boolean("search-visible", filter.is_visible())
                .expect("Could not find setting search-visible");
        });

        if gio::Settings::new(NAMESPACE).get_boolean("search-visible") {
            let filter = self.accounts_window.filter.clone();
            filter.show()
        }

        search_button
    }

    fn build_system_menu(&mut self, connection: Arc<Mutex<Connection>>) -> MenuButton {
        let builder = gtk::Builder::from_resource(format!("{}/{}", NAMESPACE_PREFIX, "system_menu.ui").as_str());

        let popover: gtk::PopoverMenu = builder.get_object("popover").unwrap();

        let about_button: gtk::Button = builder.get_object("about_button").unwrap();

        let export_button: gtk::Button = builder.get_object("export_button").unwrap();

        let dark_mode_slider: gtk::Switch = {
            let switch: gtk::Switch = builder.get_object("dark_mode_slider").unwrap();
            let g_settings = gio::Settings::new(NAMESPACE);
            switch.set_state(g_settings.get_boolean("dark-theme"));
            switch
        };

        {
            let gui = self.clone();
            let connection = connection.clone();
            dark_mode_slider.connect_state_set(move |_, state| {
                let g_settings = gio::Settings::new(NAMESPACE);
                g_settings.set_boolean("dark-theme", state).expect("Could not find setting dark-theme");

                // switch first then redraw - to take into account state change
                gui.switch_to(Display::DisplayAccounts);

                AccountsWindow::refresh_accounts(&gui, connection.clone());

                Inhibit(false)
            });
        }

        export_button.connect_clicked(self.export_accounts(popover.clone(), connection.clone()));

        let import_button: gtk::Button = builder.get_object("import_button").unwrap();

        import_button.connect_clicked(self.import_accounts(popover.clone(), connection));

        let system_menu: gtk::MenuButton = builder.get_object("system_menu").unwrap();

        {
            let popover = popover.clone();
            system_menu.connect_clicked(move |_| {
                popover.show_all();
            });
        }

        let titlebar = gtk::HeaderBarBuilder::new().decoration_layout(":").title("About").build();

        self.about_popup.set_titlebar(Some(&titlebar));
        {
            let popup = self.about_popup.clone();
            about_button.connect_clicked(move |_| {
                popover.set_visible(false);
                popup.set_visible(true);
                popup.show_all();
            });
        };

        system_menu
    }

    fn build_action_menu(&mut self, connection: Arc<Mutex<Connection>>) -> MenuButton {
        let builder = gtk::Builder::from_resource(format!("{}/{}", NAMESPACE_PREFIX, "action_menu.ui").as_str());
        let popover: gtk::PopoverMenu = builder.get_object("popover").unwrap();
        let add_account_button: gtk::Button = builder.get_object("add_account_button").unwrap();
        let add_group_button: gtk::Button = builder.get_object("add_group_button").unwrap();

        {
            let popover = popover.clone();
            let add_group = self.add_group.clone();
            let gui = self.clone();

            add_group_button.connect_clicked(move |_| {
                popover.hide();
                add_group.reset();

                gui.switch_to(Display::DisplayAddGroup);
            });
        }

        let action_menu: gtk::MenuButton = builder.get_object("action_menu").unwrap();

        {
            let action_menu = action_menu.clone();
            let widgets = self.accounts_window.widgets.clone();
            let add_account_button = add_account_button.clone();
            let popover = popover.clone();
            let state = self.state.clone();

            action_menu.connect_clicked(move |_| {
                let widgets = widgets.lock().unwrap();

                /*
                 * Both add group and account buttons are available only if on
                 * main accounts display. This is to avoid having to clean temp files
                 * (ie. group icons) if switching half-way editing/adding a group.
                 *
                 * Todo: consider hiding the action menu altogether.
                 */

                let state = state.borrow();
                let display = state.display.clone();
                // can't add account if no groups
                add_account_button.set_sensitive(!widgets.is_empty() && display == Display::DisplayAccounts);

                add_group_button.set_sensitive(display == Display::DisplayAccounts || display == Display::DisplayNoAccounts);

                popover.show_all();
            });
        }

        {
            // creates a shortcut on the "+" image to action menu when no account page is displayed
            let action_menu = action_menu.clone();
            self.no_accounts.no_accounts_plus_sign.connect_button_press_event(move |_, _| {
                action_menu.clicked();
                Inhibit(true)
            });
        }

        add_account_button.connect_clicked(AccountsWindow::display_add_account_form(
            connection,
            popover,
            self.clone(),
            None,
        ));

        action_menu
    }
}
