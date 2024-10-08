use gettextrs::gettext;
use std::sync::{Arc, Mutex};

use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use gtk::{Builder, Button, MenuButton, PopoverMenu};
use gtk_macros::get_widget;
use rusqlite::Connection;

use crate::exporting::{Exporting, ImportType};
use crate::main_window::{Display, MainWindow};
use crate::ui::{AccountsWindow, AddGroupWindow};
use crate::{NAMESPACE, NAMESPACE_PREFIX};

pub trait Menus {
    fn build_menus(&self, connection: Arc<Mutex<Connection>>);

    fn build_search_button(&self, connection: Arc<Mutex<Connection>>) -> Button;

    fn build_system_menu(&self, connection: Arc<Mutex<Connection>>) -> MenuButton;

    fn build_action_menu(&self, connection: Arc<Mutex<Connection>>) -> MenuButton;
}

impl Menus for MainWindow {
    fn build_menus(&self, connection: Arc<Mutex<Connection>>) {
        let title_bar = gtk::HeaderBar::builder().show_close_button(true).build();

        title_bar.pack_start(&self.build_action_menu(connection.clone()));

        title_bar.pack_start(&self.build_search_button(connection.clone()));

        title_bar.pack_end(&self.build_system_menu(connection));
        self.window.set_titlebar(Some(&title_bar));

        title_bar.show_all();
    }

    fn build_search_button(&self, connection: Arc<Mutex<Connection>>) -> Button {
        let builder = Builder::from_resource(format!("{}/{}", NAMESPACE_PREFIX, "system_menu.ui").as_str());
        get_widget!(builder, Button, search_button);

        search_button.connect_clicked(clone!(
            #[strong(rename_to = gui)]
            self,
            move |_| {
                if WidgetExt::is_visible(&gui.accounts_window.filter) {
                    gui.accounts_window.filter.hide();
                    gui.accounts_window.filter.set_text("");

                    glib::spawn_future_local(clone!(
                        #[strong]
                        connection,
                        #[strong]
                        gui,
                        async move {
                            let results = AccountsWindow::load_account_groups(connection.clone(), None).await;
                            gui.accounts_window.replace_accounts_and_widgets(results, gui.clone(), connection).await;
                        }
                    ));
                } else {
                    gui.accounts_window.filter.show();
                    gui.accounts_window.filter.grab_focus()
                }

                gio::Settings::new(NAMESPACE)
                    .set_boolean("search-visible", WidgetExt::is_visible(&gui.accounts_window.filter))
                    .expect("Could not find setting search-visible");
            }
        ));

        if gio::Settings::new(NAMESPACE).boolean("search-visible") {
            self.accounts_window.filter.show()
        }

        search_button
    }

    fn build_system_menu(&self, connection: Arc<Mutex<Connection>>) -> MenuButton {
        let builder = Builder::from_resource(format!("{}/{}", NAMESPACE_PREFIX, "system_menu.ui").as_str());

        get_widget!(builder, PopoverMenu, popover);
        get_widget!(builder, Button, about_button);
        get_widget!(builder, Button, export_button);
        get_widget!(builder, Button, import_button_yaml);
        get_widget!(builder, Button, import_button_ga);
        get_widget!(builder, MenuButton, system_menu);

        let dark_mode_slider: gtk::Switch = {
            let switch: gtk::Switch = builder.object("dark_mode_slider").unwrap();
            let g_settings = gio::Settings::new(NAMESPACE);
            switch.set_state(g_settings.boolean("dark-theme"));
            switch
        };

        dark_mode_slider.connect_state_set(clone!(
            #[strong(rename_to = gui)]
            self,
            move |_, state| {
                let g_settings = gio::Settings::new(NAMESPACE);
                g_settings.set_boolean("dark-theme", state).expect("Could not find setting dark-theme");

                // switch first then redraw - to take into account state change
                gui.switch_to(Display::Accounts);
                gui.accounts_window.refresh_accounts(&gui);

                gtk::glib::Propagation::Proceed
            }
        ));

        export_button.connect_clicked(self.export_accounts(popover.clone(), connection.clone()));

        import_button_yaml.connect_clicked(self.import_accounts(ImportType::Internal, popover.clone(), connection.clone()));
        import_button_ga.connect_clicked(self.import_accounts(ImportType::GoogleAuthenticator, popover.clone(), connection));

        system_menu.connect_clicked(clone!(
            #[strong]
            popover,
            move |_| {
                popover.show_all();
            }
        ));

        let title_bar = gtk::HeaderBar::builder().decoration_layout(":").title(gettext("About")).build();

        self.about_popup.set_titlebar(Some(&title_bar));

        about_button.connect_clicked(clone!(
            #[strong(rename_to = popup)]
            self.about_popup,
            move |_| {
                popover.set_visible(false);
                popup.set_visible(true);
                popup.show_all();
            }
        ));

        system_menu
    }

    fn build_action_menu(&self, connection: Arc<Mutex<Connection>>) -> MenuButton {
        let builder = Builder::from_resource(format!("{}/{}", NAMESPACE_PREFIX, "action_menu.ui").as_str());
        get_widget!(builder, PopoverMenu, popover);
        get_widget!(builder, Button, add_account_button);
        get_widget!(builder, Button, add_group_button);
        get_widget!(builder, MenuButton, action_menu);

        add_group_button.connect_clicked(clone!(
            #[strong]
            popover,
            #[strong(rename_to = gui)]
            self,
            #[strong]
            connection,
            move |_| {
                let builder = Builder::from_resource(format!("{}/{}", NAMESPACE_PREFIX, "main.ui").as_str());

                let add_group = AddGroupWindow::new(&builder);
                add_group.edit_group_buttons_actions(&gui, connection.clone());

                gui.add_group.replace_with(&add_group);

                popover.hide();
                add_group.reset();

                gui.switch_to(Display::AddGroup);
            }
        ));

        action_menu.connect_clicked(clone!(
            #[strong]
            popover,
            #[strong(rename_to = state)]
            self.state,
            #[strong]
            add_account_button,
            #[strong(rename_to = widgets)]
            self.accounts_window.widgets,
            move |_| {
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
            }
        ));

        // creates a shortcut on the "+" image to action menu when no account page is displayed
        self.no_accounts.no_accounts_plus_sign.connect_button_press_event(clone!(
            #[strong]
            action_menu,
            move |_, _| {
                action_menu.clicked();
                gtk::glib::Propagation::Stop
            }
        ));

        add_account_button.connect_clicked(self.accounts_window.display_add_account_form(connection, &popover, self, None));

        action_menu
    }
}
