use crate::helpers::ConfigManager;
use crate::main_window::MainWindow;
use crate::model::AccountGroupWidgets;
use chrono::prelude::*;
use chrono::Local;
use gtk::prelude::*;
use gtk::Builder;
use rusqlite::Connection;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct AccountsWindow {
    pub main_box: gtk::Box,
    pub edit_account: gtk::Box,
    pub stack: gtk::Stack,
    pub accounts_container: gtk::Box,
    pub progress_bar: Arc<Mutex<RefCell<gtk::ProgressBar>>>,
    pub widgets: Rc<RefCell<Vec<AccountGroupWidgets>>>,
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
            widgets: Rc::new(RefCell::new(vec![])),
        }
    }

    pub fn replace_accounts_and_widgets(gui: MainWindow, connection: Arc<Mutex<Connection>>) {
        let mut gui = gui;
        let accounts_container = gui.accounts_window.accounts_container.clone();

        let children = accounts_container.get_children();
        children.iter().for_each(|e| accounts_container.remove(e));

        let connection_clone = connection.clone();
        let mut groups = MainWindow::fetch_accounts(connection_clone);

        let widgets: Vec<AccountGroupWidgets> = groups
            .iter_mut()
            .map(|account_group| account_group.widget())
            .collect();

        widgets
            .iter()
            .for_each(|w| accounts_container.add(&w.container));

        gui.accounts_window.widgets.replace(widgets);
        gui.accounts_window.accounts_container = accounts_container;
        gui.accounts_window.accounts_container.show_all();

        let gui2 = gui.clone();
        let connection2 = connection.clone();
        AccountsWindow::edit_buttons_actions(gui, connection);
        AccountsWindow::delete_buttons_actions(gui2, connection2);
    }

    pub fn edit_buttons_actions(gui: MainWindow, connection: Arc<Mutex<Connection>>) {
        let o = gui.accounts_window.widgets;
        let mut oo = o.borrow_mut();

        for group_widgets in oo.iter_mut() {
            for account_widgets in &group_widgets.account_widgets {
                let id = account_widgets.account_id;
                let popover = account_widgets.popover.clone();

                let main_box = gui.accounts_window.main_box.clone();
                let edit_account = gui.accounts_window.edit_account.clone();

                let account = {
                    let connection = connection.clone();
                    ConfigManager::get_account(connection, id)
                }
                .unwrap();

                let connection = connection.clone();
                let input_group = gui.edit_account_window.input_group.clone();
                let input_name = gui.edit_account_window.input_name.clone();
                let input_secret = gui.edit_account_window.input_secret.clone();
                let input_account_id = gui.edit_account_window.input_account_id.clone();

                account_widgets.edit_button.connect_clicked(move |_| {
                    let connection = connection.lock().unwrap();
                    let groups = ConfigManager::load_account_groups(&connection).unwrap();

                    groups.iter().for_each(|group| {
                        let string = format!("{}", group.id);
                        let entry_id = Some(string.as_str());
                        input_group.append(entry_id, group.name.as_str());
                        if group.id == account.group_id {
                            input_group.set_active_id(entry_id);
                        }
                    });

                    let account_id = format!("{}", account.id);
                    input_account_id.set_text(account_id.as_str());
                    input_name.set_text(account.label.as_str());
                    input_secret.set_text(account.secret.as_str());

                    popover.hide();
                    main_box.set_visible(false);
                    edit_account.set_visible(true);
                });
            }
        }
    }

    pub fn delete_buttons_actions(gui: MainWindow, connection: Arc<Mutex<Connection>>) {
        let gui_clone = gui.clone();

        let o = gui.accounts_window.widgets;
        let mut oo = o.borrow_mut();

        for group_widgets in oo.iter_mut() {
            let group_widgets_outer = group_widgets.clone();
            let gui_outer = gui_clone.clone();

            for account_widgets in &group_widgets.account_widgets {
                let account_id = account_widgets.account_id;
                let group_id = account_widgets.group_id;
                // let popover = account_widgets.popover.clone();
                //
                // let connection = connection.clone();

                let gui_inner = gui_outer.clone();
                // let m = gui_inner.accounts_window.widgets.clone();
                // let group_widgets_inner = group_widgets_outer.clone();

                account_widgets.delete_button.connect_clicked(move |_| {
                    // let connection = connection.clone();
                    // let _ = ConfigManager::delete_account(connection, id).unwrap();

                    let gui = gui_inner.clone();
                    let m = gui.accounts_window.widgets;
                    let mut mm = m.borrow_mut();


                    println!("group_id {}", group_id);
                    println!("account_id to delete {}", account_id);

                    // let gui2 = gui_inner.clone();
                    // let arc = gui2.accounts_window.widgets;
                    // let ref_mut = arc.borrow_mut();
                    println!("before {:?}", mm);

                    mm.clear();

        //
        //             let mut gui2 = gui_inner.clone();
        //
        //             let r =
        //                 gui2.accounts_window.widgets.iter_mut().find(|x| x.id == group_id );
        //
        //             if let Some(a) = r {
        //                 let mut p = &mut a.account_widgets;
        //
        //                 if let Some(pos) = p.iter().position(|x| {
        //                     x.account_id == account_id
        //                 }) {
        //                     println!("pos {}", pos);
        //                     p.remove(pos);
        //                 }
        //             }
        //
        //             println!("after {:?}", gui.accounts_window.widgets);
        //
        //             // let mut group_widgets_inner = group_widgets_inner.clone();
        //             // let mut account_widgets = group_widgets_inner.account_widgets;
        //             // println!("before {}", account_widgets.len());
        //             // println!("before2 {}", gui_inner.accounts_window.widgets.len());
        //             // if let Some(pos) = account_widgets.iter().position(|x| {
        //             //     println!("account_id {}", x.account_id);
        //             //     x.account_id == account_id
        //             // }) {
        //             //     println!("pos {}", pos);
        //             //     account_widgets.remove(pos);
        //             // }
        //             //
        //             // println!("after {}", account_widgets.len());
        //             //
        //             // group_widgets_inner.account_widgets = account_widgets;
        //
        //             popover.hide();
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
