use std::sync::{Arc, Mutex};

use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use gtk::{Button, MenuButton};
use gtk_macros::get_widget;
use rusqlite::Connection;

use crate::exporting::Exporting;
use crate::main_window::{Display, MainWindow};
use crate::ui::{AccountsWindow, AddGroupWindow};
use crate::{NAMESPACE, NAMESPACE_PREFIX};

pub trait Menus {
    fn build_menus(&mut self, connection: Arc<Mutex<Connection>>);

    fn build_search_button(&mut self, connection: Arc<Mutex<Connection>>) -> gtk::Button;

    fn build_system_menu(&mut self, connection: Arc<Mutex<Connection>>) -> gtk::MenuButton;

    fn build_action_menu(&mut self, connection: Arc<Mutex<Connection>>) -> gtk::MenuButton;
}

impl Menus for MainWindow {
    fn build_menus(&mut self, connection: Arc<Mutex<Connection>>) {
        let titlebar = gtk::HeaderBar::builder().show_close_button(true).build();

        titlebar.pack_start(&self.build_action_menu(connection.clone()));

        titlebar.pack_start(&self.build_search_button(connection.clone()));

        titlebar.pack_end(&self.build_system_menu(connection));
        self.window.set_titlebar(Some(&titlebar));

        titlebar.show_all();
    }

    fn build_search_button(&mut self, connection: Arc<Mutex<Connection>>) -> Button {
        let builder = gtk::Builder::from_resource(format!("{}/{}", NAMESPACE_PREFIX, "system_menu.ui").as_str());
        get_widget!(builder, gtk::Button, search_button);

        search_button.connect_clicked(clone!(@strong self as gui, @strong self.accounts_window.filter as filter => move |_| {
            if WidgetExt::is_visible(&filter) {
                filter.hide();
                filter.set_text("");

                let (tx, rx) = async_channel::bounded(1);

                glib::spawn_future_local(clone!(@strong gui, @strong connection => async move {
                    let _ = rx.recv().await.unwrap();
                    gui.accounts_window.replace_accounts_and_widgets(gui.clone(), connection.clone())
                }));

                gui.pool.spawn_ok(AccountsWindow::load_account_groups(tx, connection.clone(), None));

            } else {
                filter.show();
                filter.grab_focus()
            }

            gio::Settings::new(NAMESPACE)
                .set_boolean("search-visible", WidgetExt::is_visible(&filter))
                .expect("Could not find setting search-visible");
        }));

        if gio::Settings::new(NAMESPACE).boolean("search-visible") {
            let filter = self.accounts_window.filter.clone();
            filter.show()
        }

        search_button
    }

    fn build_system_menu(&mut self, connection: Arc<Mutex<Connection>>) -> MenuButton {
        let builder = gtk::Builder::from_resource(format!("{}/{}", NAMESPACE_PREFIX, "system_menu.ui").as_str());

        get_widget!(builder, gtk::PopoverMenu, popover);
        get_widget!(builder, gtk::Button, about_button);
        get_widget!(builder, gtk::Button, export_button);

        let dark_mode_slider: gtk::Switch = {
            let switch: gtk::Switch = builder.object("dark_mode_slider").unwrap();
            let g_settings = gio::Settings::new(NAMESPACE);
            switch.set_state(g_settings.boolean("dark-theme"));
            switch
        };

        dark_mode_slider.connect_state_set(clone!(@strong connection, @strong self as gui => move |_, state| {
            let g_settings = gio::Settings::new(NAMESPACE);
            g_settings.set_boolean("dark-theme", state).expect("Could not find setting dark-theme");

            // switch first then redraw - to take into account state change
            gui.switch_to(Display::Accounts);

            gui.accounts_window.refresh_accounts(&gui, connection.clone());

            gtk::glib::Propagation::Proceed
        }));

        export_button.connect_clicked(self.export_accounts(popover.clone(), connection.clone()));

        let import_button: gtk::Button = builder.object("import_button").unwrap();

        import_button.connect_clicked(self.import_accounts(popover.clone(), connection));

        let system_menu: gtk::MenuButton = builder.object("system_menu").unwrap();

        system_menu.connect_clicked(clone!(@strong popover => move |_| {
            popover.show_all();
        }));

        let titlebar = gtk::HeaderBar::builder().decoration_layout(":").title("About").build();

        self.about_popup.set_titlebar(Some(&titlebar));

        about_button.connect_clicked(clone!(@strong self.about_popup as popup => move |_| {
            popover.set_visible(false);
            popup.set_visible(true);
            popup.show_all();
        }));

        system_menu
    }

    fn build_action_menu(&mut self, connection: Arc<Mutex<Connection>>) -> MenuButton {
        let builder = gtk::Builder::from_resource(format!("{}/{}", NAMESPACE_PREFIX, "action_menu.ui").as_str());
        get_widget!(builder, gtk::PopoverMenu, popover);
        get_widget!(builder, gtk::Button, add_account_button);
        get_widget!(builder, gtk::Button, add_group_button);
        get_widget!(builder, gtk::MenuButton, action_menu);

        let gui = self.clone();
        let widgets = self.accounts_window.widgets.clone();
        let state = self.state.clone();

        add_group_button.connect_clicked(clone!(@strong popover, @strong gui, @strong connection => move |_| {
            let builder = gtk::Builder::from_resource(format!("{}/{}", NAMESPACE_PREFIX, "main.ui").as_str());

            let add_group = AddGroupWindow::new(&builder);
            add_group.add_group_container_add.set_visible(true);
            add_group.add_group_container_edit.set_visible(false);
            add_group.edit_account_buttons_actions(&gui, connection.clone());

            gui.add_group.replace_with(&add_group);

            popover.hide();
            add_group.reset();

            gui.switch_to(Display::AddGroup);
        }));

        action_menu.connect_clicked(clone!(@strong popover, @strong state, @strong add_account_button, @strong widgets => move |_| {
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
            add_account_button.set_sensitive(!widgets.is_empty() && display == Display::Accounts);

            add_group_button.set_sensitive(display == Display::Accounts || display == Display::NoAccounts);

            popover.show_all();
        }));

        // creates a shortcut on the "+" image to action menu when no account page is displayed
        self.no_accounts
            .no_accounts_plus_sign
            .connect_button_press_event(clone!(@strong action_menu => move |_, _| {
                action_menu.clicked();
                gtk::glib::Propagation::Stop
            }));

        add_account_button.connect_clicked(self.accounts_window.display_add_account_form(connection, &popover, self, None));

        action_menu
    }
}
