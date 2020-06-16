use std::sync::{Arc, Mutex};
use std::cell::RefCell;
use gtk::prelude::*;
use gtk::Builder;
use chrono::prelude::*;
use chrono::Local;

pub struct AccountsWindow {
    pub progress_bar: Arc<Mutex<RefCell<gtk::ProgressBar>>>,
}

impl AccountsWindow {

    pub fn new(builder: Builder) -> AccountsWindow {
        let progress_bar: gtk::ProgressBar = builder.get_object("progress_bar").unwrap();

        progress_bar.set_fraction(Self::progress_bar_fraction());

        AccountsWindow {
            progress_bar: Arc::new(Mutex::new(RefCell::new(progress_bar))),
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
        assert_eq!(0.5333333333333333_f64, AccountsWindow::progress_bar_fraction_for(14));
    }
}
