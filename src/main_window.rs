use crate::state::State;
use gtk::prelude::*;
use std::collections::HashMap;
pub struct MainWindow {
    window: gtk::Window,
    result: gtk::Label,
    popover: gtk::Popover,
    error_label: gtk::Label,
    user_spec_entry: gtk::Entry,
    buttons: HashMap<String, gtk::Button>,
}

impl MainWindow {
    pub fn new() -> MainWindow {
        // Initialize the UI from the Glade XML.
        let glade_src = include_str!("mainwindow.glade");
        let builder = gtk::Builder::new_from_string(glade_src);

        // Get handles for the various controls we need to use.
        let window: gtk::Window = builder.get_object("mainWindow").unwrap();
        let result: gtk::Label = builder.get_object("resultLabel").unwrap();
        let popover: gtk::Popover = builder.get_object("errorPopover").unwrap();
        let error_label: gtk::Label = builder.get_object("errorLabel").unwrap();
        let user_spec_entry: gtk::Entry = builder.get_object("userSpecEntry").unwrap();

        
        

        // Get handles for all the buttons.
        let mut buttons: HashMap<String, gtk::Button> = HashMap::new();
        for name in &[
            "rollD4",
            "rollD6",
            "rollD8",
            "rollD10",
            "rollD12",
            "rollD20",
            "rollD100",
            "clearResult",
            "halveUpResult",
            "halveDownResult",
            "rollUser",
        ] {
            buttons.insert(
                name.to_string(),
                builder
                    .get_object(name)
                    .expect(&format!("Could not get button {}", name)),
            );
        }

        MainWindow {
            window,
            result,
            popover,
            error_label,
            user_spec_entry,
            buttons,
        }
    }

    // Set up naming for the window and show it to the user.
    pub fn start(&self) {
        glib::set_application_name("gDiceRoller");
        self.window.set_wmclass("Dice Roller", "Dice Roller");
        self.window.connect_delete_event(|_, _| {
            gtk::main_quit();
            Inhibit(false)
        });
        self.window.show_all();
    }

    pub fn update_from(&self, state: &State) {
        if let Some(ref err) = state.error {
            self.error_label.set_text(&format!(
                "The dice expression entered is not valid:\n{}",
                err
            ));
            self.popover.show_all();
        } else {
            // The popover will hide itself anyway when the user clicks
            // outside of it, but we shouldn't leave an error indicator in it.
            self.error_label.set_text("");
        }

        self.result.set_text(&format!("{}", state.value));
    }

    pub fn button(&self, name: &str) -> &gtk::Button {
        self.buttons.get(name).expect("Could not get button.")
    }

    pub fn user_spec_entry(&self) -> &gtk::Entry {
        &self.user_spec_entry
    }
}
